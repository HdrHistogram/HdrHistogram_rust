use core::counter::Counter;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield at fixed-size steps through the histogram's value range.
pub struct Iter<'a, T: 'a + Counter> {
    hist: &'a Histogram<T>,

    // > 0
    value_units_per_bucket: u64,
    current_step_highest_value_reporting_level: u64,
    current_step_lowest_value_reporting_level: u64,
}

impl<'a, T: 'a + Counter> Iter<'a, T> {
    /// Construct a new linear iterator. See `Histogram::iter_linear` for details.
    pub fn new(hist: &'a Histogram<T>,
               value_units_per_bucket: u64)
               -> HistogramIterator<'a, T, Iter<'a, T>> {
        assert!(value_units_per_bucket > 0, "value_units_per_bucket must be > 0");
        HistogramIterator::new(hist,
                               Iter {
                                   hist: hist,
                                   value_units_per_bucket: value_units_per_bucket,
                                   // won't underflow because value_units_per_bucket > 0
                                   current_step_highest_value_reporting_level: value_units_per_bucket - 1,
                                   current_step_lowest_value_reporting_level:
                                       hist.lowest_equivalent(value_units_per_bucket - 1),
                               })
    }
}

impl<'a, T: 'a + Counter> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: u64) -> bool {
        let val = self.hist.value_for(index);
        if val >= self.current_step_lowest_value_reporting_level || index == self.hist.last_index() {
            self.current_step_highest_value_reporting_level += self.value_units_per_bucket;
            self.current_step_lowest_value_reporting_level = self.hist
                .lowest_equivalent(self.current_step_highest_value_reporting_level);
            true
        } else {
            false
        }
    }

    fn more(&mut self, index: usize) -> bool {
        // If the next iterate will not move to the next sub bucket index (which is empty if
        // if we reached this point), then we are not yet done iterating (we want to iterate
        // until we are no longer on a value that has a count, rather than util we first reach
        // the last value that has a count. The difference is subtle but important)...
        // TODO index + 1 could overflow 16-bit usize
        self.current_step_highest_value_reporting_level + 1 < self.hist.value_for(index + 1)
    }
}
