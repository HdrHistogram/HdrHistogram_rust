use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

pub struct Iter<'a, T: 'a + num::Num> {
    hist: &'a Histogram<T>,

    percentileTicksPerHalfDistance: isize,
    percentileLevelToIterateTo: f64,
    reachedLastRecordedValue: bool,
}

impl<'a, T: 'a + num::Num + Copy> Iter<'a, T> {
    pub fn new(hist: &'a Histogram<T>,
               percentileTicksPerHalfDistance: isize)
               -> HistogramIterator<'a, T, Iter<'a, T>> {
        HistogramIterator::new(hist,
                               Iter {
                                   hist: hist,
                                   percentileTicksPerHalfDistance: percentileTicksPerHalfDistance,
                                   percentileLevelToIterateTo: 0.0,
                                   reachedLastRecordedValue: false,
                               })
    }
}

impl<'a, T: 'a + Copy + num::Num> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, running_total: i64) -> bool {
        let count = &self.hist[index];
        if *count == T::zero() {
            return false;
        }

        let currentPercentile = (100 * running_total) as f64 / self.hist.total() as f64;
        if currentPercentile < self.percentileLevelToIterateTo {
            return false;
        }

        // The choice to maintain fixed-sized "ticks" in each half-distance to 100% [starting from
        // 0%], as opposed to a "tick" size that varies with each interval, was made to make the
        // steps easily comprehensible and readable to humans. The resulting percentile steps are
        // much easier to browse through in a percentile distribution output, for example.
        //
        // We calculate the number of equal-sized "ticks" that the 0-100 range will be divided by
        // at the current scale. The scale is detemined by the percentile level we are iterating
        // to. The following math determines the tick size for the current scale, and maintain a
        // fixed tick size for the remaining "half the distance to 100%" [from either 0% or from
        // the previous half-distance]. When that half-distance is crossed, the scale changes and
        // the tick size is effectively cut in half.

        let percentileReportingTicks =
            self.percentileTicksPerHalfDistance *
            2f64.powi(((100.0 / (100.0 - self.percentileLevelToIterateTo)).ln() /
                       2f64.ln()) as i32 + 1) as isize;
        self.percentileLevelToIterateTo += 100.0 / percentileReportingTicks as f64;
        true
    }

    fn more(&mut self, _: usize) -> bool {
        // We want one additional last step to 100%
        if !self.reachedLastRecordedValue && self.hist.total() > 0 {
            self.percentileLevelToIterateTo = 100.0;
            self.reachedLastRecordedValue = true;
            true
        } else {
            false
        }
    }
}
