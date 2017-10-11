use core::counter::Counter;
use Histogram;

/// An iterator that iterates over histogram quantiles.
pub mod quantile;

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
    /// `index` is a valid index in the relevant histogram.
    fn pick(&mut self, index: usize, total_count_to_index: u64) -> bool;
    /// should we keep iterating even though all future indices are zeros?
    fn more(&mut self, index: usize) -> bool;
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
    total_count_to_index: u64,
    prev_total_count: u64,
    current_index: usize,
    fresh: bool,
    ended: bool,
    picker: P,
}

/// The value emitted at each step when iterating over a `Histogram`.
#[derive(Debug, PartialEq)]
pub struct IterationValue<T: Counter> {
    value: u64,
    quantile: f64,
    count_at_value: T,
    count_since_last_iteration: u64
}

impl<T: Counter> IterationValue<T> {
    /// Create a new IterationValue.
    pub fn new(value: u64, quantile: f64, count_at_value: T, count_since_last_iteration: u64)
            -> IterationValue<T> {
        IterationValue {
            value,
            quantile,
            count_at_value,
            count_since_last_iteration
        }
    }

    /// the lowest value stored in the current histogram bin
    pub fn value(&self) -> u64 {
        self.value
    }

    /// percent of recorded values that are equivalent to or below `value`.
    /// This is simply the quantile multiplied by 100.0, so if you care about maintaining the best
    /// floating-point precision, use `quantile()` instead.
    pub fn percentile(&self) -> f64 {
        self.quantile * 100.0
    }

    /// quantile of recorded values that are equivalent to or below `value`
    pub fn quantile(&self) -> f64 { self.quantile }

    /// recorded count for values equivalent to `value`
    pub fn count_at_value(&self) -> T {
        self.count_at_value
    }

    /// number of values traversed since the last iteration step
    pub fn count_since_last_iteration(&self) -> u64 {
        self.count_since_last_iteration
    }
}

impl<'a, T: Counter, P: PickyIterator<T>> HistogramIterator<'a, T, P> {
    fn new(h: &'a Histogram<T>, picker: P) -> HistogramIterator<'a, T, P> {
        HistogramIterator {
            hist: h,
            total_count_to_index: 0,
            prev_total_count: 0,
            current_index: 0,
            picker,
            fresh: true,
            ended: false,
        }
    }

    fn current(&self) -> IterationValue<T> {
        IterationValue {
            value: self.hist.highest_equivalent(self.hist.value_for(self.current_index)),
            quantile: self.total_count_to_index as f64 / self.hist.count() as f64,
            count_at_value: self.hist.count_at_index(self.current_index)
                .expect("current index cannot exceed counts length"),
            count_since_last_iteration: self.total_count_to_index - self.prev_total_count
        }
    }
}

impl<'a, T: 'a, P> Iterator for HistogramIterator<'a, T, P>
    where T: Counter,
          P: PickyIterator<T>
{
    type Item = IterationValue<T>;
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
            if self.current_index == self.hist.len() {
                self.ended = true;
                return None;
            }

            // have we yielded all non-zeros in the histogram?
            let total = self.hist.count();
            if self.prev_total_count == total {
                // is the picker done?
                if !self.picker.more(self.current_index) {
                    self.ended = true;
                    return None;
                }

            // nope -- alright, let's keep iterating
            } else {
                assert!(self.current_index < self.hist.len());
                assert!(self.prev_total_count < total);

                if self.fresh {
                    let count = self.hist.count_at_index(self.current_index)
                        .expect("Already checked that current_index is < counts len");

                    // if we've seen all counts, no other counts should be non-zero
                    if self.total_count_to_index == total {
                        // TODO this can fail when total count overflows
                        assert!(count == T::zero());
                    }

                    // TODO overflow
                    self.total_count_to_index = self.total_count_to_index + count.as_u64();

                    // make sure we don't add this index again
                    self.fresh = false;
                }
            }

            // figure out if picker thinks we should yield this value
            if self.picker.pick(self.current_index, self.total_count_to_index) {
                let val = self.current();

                // note that we *don't* increment self.current_index here. the picker will be
                // exposed to the same value again after yielding. not sure why this is the
                // behavior we want, but it's what the original Java implementation dictates.

                self.prev_total_count = self.total_count_to_index;
                return Some(val);
            }

            // check the next entry
            self.current_index += 1;
            self.fresh = true;
        }
        None
    }
}
