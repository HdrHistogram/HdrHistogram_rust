use num;
use Histogram;

pub mod percentile;
pub mod linear;
pub mod log;
pub mod recorded;
pub mod all;

pub trait PickyIterator<T: num::Num> {
    /// should an item be yielded for the given index?
    fn pick(&mut self, usize, i64) -> bool;
    /// should we keep iterating even though all future indices are zeros?
    fn more(&mut self, usize) -> bool;
}

pub struct HistogramIterator<'a, T: 'a + num::Num, P: PickyIterator<T>> {
    hist: &'a Histogram<T>,
    totalCountToIndex: i64,
    prevTotalCount: i64,
    currentIndex: usize,
    fresh: bool,
    ended: bool,
    picker: P,
}

impl<'a, T: num::Num + Copy, P: PickyIterator<T>> HistogramIterator<'a, T, P> {
    pub fn new(h: &'a Histogram<T>, picker: P) -> HistogramIterator<'a, T, P> {
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
    fn current(&self) -> (i64, f64, T, i64) {
        let value = self.hist.highest_equivalent(self.hist.value_from_index(self.currentIndex));
        let perc = 100.0 * self.totalCountToIndex as f64 / self.hist.total() as f64;
        let count = self.hist[self.currentIndex];
        (value, perc, count, self.totalCountToIndex - self.prevTotalCount)
    }
}

impl<'a, T: 'a, P> Iterator for HistogramIterator<'a, T, P>
    where T: num::Num + num::ToPrimitive + Copy,
          P: PickyIterator<T>
{
    type Item = (i64, f64, T, i64);
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
            let total = self.hist.total();
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
                    let count = self.hist[self.currentIndex].to_i64().unwrap();

                    // if we've seen all counts, no other counts should be non-zero
                    if self.totalCountToIndex == total {
                        assert_eq!(count, 0);
                    }

                    // maintain total count so we can yield percentiles
                    self.totalCountToIndex += count;

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
