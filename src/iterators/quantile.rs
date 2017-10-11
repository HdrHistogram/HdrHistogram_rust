use core::counter::Counter;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield at quantile steps through the histogram's value range.
pub struct Iter<'a, T: 'a + Counter> {
    hist: &'a Histogram<T>,

    ticks_per_half_distance: u32,
    quantile_to_iterate_to: f64,
    reached_last_recorded_value: bool,
}

impl<'a, T: 'a + Counter> Iter<'a, T> {
    /// Construct a new iterator. See `Histogram::iter_quantiles` for details.
    pub fn new(hist: &'a Histogram<T>, ticks_per_half_distance: u32)
               -> HistogramIterator<'a, T, Iter<'a, T>> {
        assert!(ticks_per_half_distance > 0, "Ticks per half distance must be > 0");

        HistogramIterator::new(hist,
                               Iter {
                                   hist,
                                   ticks_per_half_distance,
                                   quantile_to_iterate_to: 0.0,
                                   reached_last_recorded_value: false,
                               })
    }
}

impl<'a, T: 'a + Counter> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, running_total: u64) -> bool {
        let count = &self.hist.count_at_index(index)
            .expect("index must be valid by PickyIterator contract");
        if *count == T::zero() {
            return false;
        }

        // This calculation, combined with the `quantile * count` in `value_at_quantile`, tends
        // to produce a count_at_quantile that is 1 ulp wrong. That's just the way IEEE754 works.
        let current_quantile = running_total as f64 / self.hist.count() as f64;
        if current_quantile < self.quantile_to_iterate_to {
            return false;
        }

        // The choice to maintain fixed-sized "ticks" in each half-distance to 100% [starting from
        // 0%], as opposed to a "tick" size that varies with each interval, was made to make the
        // steps easily comprehensible and readable to humans. The resulting quantile steps are
        // much easier to browse through in a quantile distribution output, for example.
        //
        // We calculate the number of equal-sized "ticks" that the 0-1 range will be divided by
        // at the current scale. The scale is determined by the quantile level we are iterating
        // to. The following math determines the tick size for the current scale, and maintain a
        // fixed tick size for the remaining "half the distance to 100%" [from either 0% or from
        // the previous half-distance]. When that half-distance is crossed, the scale changes and
        // the tick size is effectively cut in half.
        //
        // Calculate the number of times we've halved the distance to 100%, This is 1 at 50%, 2 at
        // 75%, 3 at 87.5%, etc. 2 ^ num_halvings is the number of slices that will fit into 100%.
        // At 50%, num_halvings would be 1, so 2 ^ 1 would yield 2 slices, etc. At any given number
        // of slices, the last slice is what we're going to traverse the first half of. With 1 total
        // slice, traverse half to get to 50%. Then traverse half of the last (second) slice to get
        // to 75%, etc.
        // Minimum of 0 (1.0/1.0 = 1, log 2 of which is 0) so unsigned cast is safe.
        let num_halvings = (1.0 / (1.0 - self.quantile_to_iterate_to)).log2() as u32;
        // Calculate the total number of ticks in 0-1 given that half of each slice is tick'd.
        // The number of slices is 2 ^ num_halvings, and each slice has two "half distances" to
        // tick, so we add an extra power of two to get ticks per whole distance.
        // Use u64 math so that there's less risk of overflow with large numbers of ticks and data
        // that ends up needing large numbers of halvings.
        // TODO calculate the worst case total_ticks and make sure we can't ever overflow here
        let total_ticks = (self.ticks_per_half_distance as u64)
            .checked_mul(1_u64.checked_shl(num_halvings + 1).expect("too many halvings"))
            .expect("too many total ticks");
        let increment_size = 1.0 / total_ticks as f64;
        self.quantile_to_iterate_to += increment_size;
        true
    }

    fn more(&mut self, _: usize) -> bool {
        // We want one additional last step to 100%
        if !self.reached_last_recorded_value && self.hist.count() != 0 {
            self.quantile_to_iterate_to = 1.0;
            self.reached_last_recorded_value = true;
            true
        } else {
            false
        }
    }
}
