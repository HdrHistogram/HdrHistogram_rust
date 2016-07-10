use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield only bins with at least one sample.
pub struct Iter<'a, T: 'a + num::Num> {
    hist: &'a Histogram<T>,
    visited: Option<usize>,
}

impl<'a, T: 'a + num::Num + Copy> Iter<'a, T> {
    /// Construct a new sampled iterator. See `Histogram::iter_recorded` for details.
    pub fn new(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter<'a, T>> {
        HistogramIterator::new(hist,
                               Iter {
                                   hist: hist,
                                   visited: None,
                               })
    }
}

impl<'a, T: 'a + num::Num + Copy> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: i64) -> bool {
        // is the count non-zero?
        if self.hist[index] != T::zero() {
            // have we visited before?
            if self.visited.is_none() || self.visited.unwrap() != index {
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
