use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

pub struct Iter;

impl Iter {
    pub fn new<'a, T: num::Num + Copy>(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter> {
        HistogramIterator::new(hist, Iter)
    }
}

impl<T: num::Num + Copy> PickyIterator<T> for Iter {
    fn pick(&mut self, _: usize, _: i64) -> bool {
        true
    }

    fn last(&mut self) -> bool {
        false
    }
}
