use Counter;
use Histogram;

/// An iterator that iterates over histogram percentiles.
pub mod percentile;

/// An iterator that iterates linearly over histogram values.
pub mod linear;

/// An iterator that iterates logarithmically over histogram values.
pub mod log;

/// An iterator that iterates over recorded histogram values.
pub mod recorded;

/// An iterator that iterates over histogram values.
pub mod all;

/// A trait for designing an subset iterator over values in a `Histogram`.
pub trait PickyIterator<T: Counter> {
    /// should an item be yielded for the given index?
    fn pick(&mut self, usize, u64) -> bool;
    /// should we keep iterating even though all future indices are zeros?
    fn more(&mut self, usize) -> bool;
}

/// `HistogramIterator` provides a base iterator for a `Histogram`.
///
/// It will iterate over all discrete values until there are no more recorded values (i.e., *not*
/// necessarily until all bins have been exhausted). To facilitate the development of more
/// sophisticated iterators, a *picker* is also provided, which is allowed to only select some bins
/// that should be yielded. The picker may also extend the iteration to include a suffix of empty
/// bins.
///
/// One peculiarity of this iterator is that, if the picker does choose to yield a particular bin,
/// that bin *is re-visited* before moving on to later bins. It is not clear why this is, but it is
/// how the iterators were implemented in the original HdrHistogram, so we preserve the behavior
/// here. This is the reason why iterators such as all and recorded need to keep track of which
/// indices they have already visited.
pub struct HistogramIterator<'a, T: 'a + Counter, P: PickyIterator<T>> {
    hist: &'a Histogram<T>,
    totalCountToIndex: u64,
    prevTotalCount: u64,
    currentIndex: usize,
    fresh: bool,
    ended: bool,
    picker: P,
}

impl<'a, T: Counter, P: PickyIterator<T>> HistogramIterator<'a, T, P> {
    fn new(h: &'a Histogram<T>, picker: P) -> HistogramIterator<'a, T, P> {
        HistogramIterator {
            hist: h,
            totalCountToIndex: 0,
            prevTotalCount: 0,
            currentIndex: 0,
            picker: picker,
            fresh: true,
            ended: false,
        }
    }

    // (value, percentile, count-for-value, count-for-step)
    fn current(&self) -> (i64, f64, T, u64) {
        let value = self.hist.highest_equivalent(self.hist.value_for(self.currentIndex));
        let perc = 100.0 * self.totalCountToIndex as f64 / self.hist.count() as f64;
        let count = self.hist[self.currentIndex];
        (value, perc, count, self.totalCountToIndex - self.prevTotalCount)
    }
}

impl<'a, T: 'a, P> Iterator for HistogramIterator<'a, T, P>
    where T: Counter,
          P: PickyIterator<T>
{
    type Item = (i64, f64, T, u64);
    fn next(&mut self) -> Option<Self::Item> {
        // here's the deal: we are iterating over all the indices in the histogram's .count array.
        // however, most of those values (especially towards the end) will be zeros, which the
        // original HdrHistogram implementation doesn't yield (probably with good reason -- there
        // could be a lot of them!). so, what we do instead is iterate over indicies until we reach
        // the total *count*. After that, we iterate only until .more() returns false, at which
        // point we stop completely.

        // rust doesn't support tail call optimization, so we'd run out of stack if we simply
        // called self.next() again at the bottom. instead, we loop when we would have yielded None
        // unless we have ended.
        while !self.ended {
            // have we reached the end?
            if self.currentIndex == self.hist.len() {
                self.ended = true;
                return None;
            }

            // have we yielded all non-zeros in the histogram?
            let total = self.hist.count();
            if self.prevTotalCount == total {
                // is the picker done?
                if !self.picker.more(self.currentIndex) {
                    self.ended = true;
                    return None;
                }

                // nope -- alright, let's keep iterating
            } else {
                assert!(self.currentIndex < self.hist.len());
                assert!(self.prevTotalCount < total);

                if self.fresh {
                    let count = self.hist[self.currentIndex];

                    // if we've seen all counts, no other counts should be non-zero
                    if self.totalCountToIndex == total {
                        assert!(count == T::zero());
                    }

                    // maintain total count so we can yield percentiles
                    self.totalCountToIndex = self.totalCountToIndex + count.to_u64().unwrap();

                    // make sure we don't add this index again
                    self.fresh = false;
                }
            }

            // figure out if picker thinks we should yield this value
            if self.picker.pick(self.currentIndex, self.totalCountToIndex) {
                let val = self.current();

                // note that we *don't* increment self.currentIndex here. the picker will be
                // exposed to the same value again after yielding. not sure why this is the
                // behavior we want, but it's what the original Java implementation dictates.

                self.prevTotalCount = self.totalCountToIndex;
                return Some(val);
            }

            // check the next entry
            self.currentIndex += 1;
            self.fresh = true;
        }
        None
    }
}
