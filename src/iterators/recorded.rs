use core::counter::Counter;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield only bins with at least one sample.
pub struct Iter<'a, T: 'a + Counter> {
    hist: &'a Histogram<T>,
    visited: Option<usize>,
}

impl<'a, T: 'a + Counter> Iter<'a, T> {
    /// Construct a new sampled iterator. See `Histogram::iter_recorded` for details.
    pub fn new(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter<'a, T>> {
        HistogramIterator::new(hist,
                               Iter {
                                   hist: hist,
                                   visited: None,
                               })
    }
}

impl<'a, T: 'a + Counter> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: u64) -> bool {
        // is the count non-zero?
        let count = self.hist.count_at_index(index)
            .expect("index must be valid by PickyIterator contract");
        if count != T::zero() {
            // have we visited before?
            if self.visited.map(|i| i != index).unwrap_or(true) {
                self.visited = Some(index);
                return true;
            }
        }
        false
    }

    fn more(&mut self, _: usize) -> bool {
        false
    }
}
