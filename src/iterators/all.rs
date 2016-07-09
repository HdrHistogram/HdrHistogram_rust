use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

pub struct Iter(Option<usize>);

impl Iter {
    pub fn new<'a, T: num::Num + Copy>(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter> {
        HistogramIterator::new(hist, Iter(None))
    }
}

impl<T: num::Num + Copy> PickyIterator<T> for Iter {
    fn pick(&mut self, index: usize, _: i64) -> bool {
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
