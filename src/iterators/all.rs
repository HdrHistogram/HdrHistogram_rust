use Counter;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

/// An iterator that will yield every bin.
pub struct Iter(Option<usize>);

impl Iter {
    /// Construct a new full iterator. See `Histogram::iter_all` for details.
    pub fn new<'a, T: Counter>(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter> {
        HistogramIterator::new(hist, Iter(None))
    }
}

impl<T: Counter> PickyIterator<T> for Iter {
    fn pick(&mut self, index: usize, _: u64) -> bool {
        // have we visited before?
        if self.0.is_none() || self.0.unwrap() != index {
            self.0 = Some(index);
            true
        } else {
            false
        }
    }

    fn more(&mut self, _: usize) -> bool {
        true
    }
}
