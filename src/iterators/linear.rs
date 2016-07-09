use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

pub struct Iter<'a, T: 'a + num::Num> {
    hist: &'a Histogram<T>,

    valueUnitsPerBucket: i64,
    currentStepHighestValueReportingLevel: i64,
    currentStepLowestValueReportingLevel: i64,
}

impl<'a, T: 'a + num::Num + Copy> Iter<'a, T> {
    pub fn new(hist: &'a Histogram<T>,
               valueUnitsPerBucket: i64)
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

impl<'a, T: 'a + num::Num + Copy> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: i64) -> bool {
        let val = self.hist.value_from_index(index);
        if val >= self.currentStepLowestValueReportingLevel || index == self.hist.lastIndex() {
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
        self.currentStepHighestValueReportingLevel + 1 < self.hist.value_from_index(index + 1)
    }
}
