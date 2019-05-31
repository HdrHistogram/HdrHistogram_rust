use crate::core::counter::Counter;
use crate::iterators::{HistogramIterator, PickMetadata, PickyIterator};
use crate::Histogram;

/// An iterator that will yield only bins with at least one sample.
pub struct Iter {
    visited: Option<usize>,
}

impl Iter {
    /// Construct a new sampled iterator. See `Histogram::iter_recorded` for details.
    pub fn new<T: Counter>(hist: &Histogram<T>) -> HistogramIterator<T, Iter> {
        HistogramIterator::new(hist, Iter { visited: None })
    }
}

impl<T: Counter> PickyIterator<T> for Iter {
    fn pick(&mut self, index: usize, _: u64, count_at_index: T) -> Option<PickMetadata> {
        if count_at_index != T::zero() && self.visited.map(|i| i != index).unwrap_or(true) {
            self.visited = Some(index);
            return Some(PickMetadata::new(None, None));
        }
        None
    }

    fn more(&mut self, _: usize) -> bool {
        false
    }
}
