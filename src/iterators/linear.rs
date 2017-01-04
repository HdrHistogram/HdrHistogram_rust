use Counter;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield at fixed-size steps through the histogram's value range.
pub struct Iter<'a, T: 'a + Counter> {
    hist: &'a Histogram<T>,

    valueUnitsPerBucket: u64,
    currentStepHighestValueReportingLevel: u64,
    currentStepLowestValueReportingLevel: u64,
}

impl<'a, T: 'a + Counter> Iter<'a, T> {
    /// Construct a new linear iterator. See `Histogram::iter_linear` for details.
    pub fn new(hist: &'a Histogram<T>,
               valueUnitsPerBucket: u64)
               -> HistogramIterator<'a, T, Iter<'a, T>> {
        HistogramIterator::new(hist,
                               Iter {
                                   hist: hist,
                                   valueUnitsPerBucket: valueUnitsPerBucket,
                                   currentStepHighestValueReportingLevel: valueUnitsPerBucket - 1,
                                   currentStepLowestValueReportingLevel:
                                       hist.lowest_equivalent(valueUnitsPerBucket - 1),
                               })
    }
}

impl<'a, T: 'a + Counter> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: u64) -> bool {
        let val = self.hist.value_for(index);
        if val >= self.currentStepLowestValueReportingLevel || index == self.hist.last() {
            self.currentStepHighestValueReportingLevel += self.valueUnitsPerBucket;
            self.currentStepLowestValueReportingLevel = self.hist
                .lowest_equivalent(self.currentStepHighestValueReportingLevel);
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
        self.currentStepHighestValueReportingLevel + 1 < self.hist.value_for(index + 1)
    }
}
