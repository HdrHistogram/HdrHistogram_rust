use num;
use Histogram;
use iterators::{HistogramIterator, PickyIterator};

pub struct Iter<'a, T: 'a + num::Num>(&'a Histogram<T>);

impl<'a, T: 'a + num::Num + Copy> Iter<'a, T> {
    pub fn new(hist: &'a Histogram<T>) -> HistogramIterator<'a, T, Iter<'a, T>> {
        HistogramIterator::new(hist, Iter(hist))
    }
}

impl<'a, T: 'a + num::Num + Copy> PickyIterator<T> for Iter<'a, T> {
    fn pick(&mut self, index: usize, _: i64) -> bool {
        self.0[index] != T::zero()
    }

    fn last(&mut self) -> bool {
        false
    }
}
