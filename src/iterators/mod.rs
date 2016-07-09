use num;
use Histogram;

pub mod percentile;
pub mod linear;
pub mod log;
pub mod recorded;
pub mod all;

pub trait PickyIterator<T: num::Num> {
    fn pick(&mut self, usize, i64) -> bool;
    fn last(&mut self) -> bool;
}

pub struct HistogramIterator<'a, T: 'a + num::Num, P: PickyIterator<T>> {
    hist: &'a Histogram<T>,
    totalCountToIndex: i64,
    prevTotalCount: i64,
    currentIndex: usize,
    ended: bool,
    picker: P,
}

impl<'a, T: num::Num + Copy, P: PickyIterator<T>> HistogramIterator<'a, T, P> {
    pub fn new(h: &'a Histogram<T>, picker: P) -> HistogramIterator<'a, T, P> {
        HistogramIterator {
            hist: h,
            totalCountToIndex: 0,
            prevTotalCount: 0,
            currentIndex: 0,
            picker: picker,
            ended: false,
        }
    }

    // (value, percentile, count-for-value, count-for-step)
    fn current(&self) -> (i64, f64, T, i64) {
        let value = self.hist.highest_equivalent(self.hist.value_from_index(self.currentIndex));
        let perc = self.totalCountToIndex as f64 / self.hist.total() as f64;
        let count = self.hist[self.currentIndex];
        (value, perc, count, self.totalCountToIndex - self.prevTotalCount)
    }
}

impl<'a, T: 'a, P> Iterator for HistogramIterator<'a, T, P>
    where T: num::Num + num::ToPrimitive + Copy,
          P: PickyIterator<T>
{
    type Item = (i64, f64, T, i64);
    fn next(&mut self) -> Option<Self::Item> {
        // rust doesn't support tail call optimization, so we'd run out of stack if we simply
        // called self.next() again at the bottom. instead, we loop when we would have yielded None
        // unless we have ended.
        while !self.ended {
            // have we exhausted the histogram?
            if self.currentIndex == self.hist.len() {
                self.ended = true;

                // does the picker want to yield the last value after all?
                return if self.picker.last() {
                    self.currentIndex -= 1;
                    Some(self.current())
                } else {
                    None
                };
            }

            // maintain total count so we can yield percentiles
            self.totalCountToIndex += self.hist[self.currentIndex].to_i64().unwrap();

            // figure out if picker thinks we should yield this value
            if self.picker.pick(self.currentIndex, self.totalCountToIndex) {
                let val = self.current();

                // make sure next() will keep yielding later entries
                self.currentIndex += 1;

                // make sure we don't yield the last value twice when picker.last() == true
                if self.currentIndex == self.hist.len() {
                    self.ended = true;
                }

                self.prevTotalCount = self.totalCountToIndex;
                return Some(val);
            }

            // check the next entry
            self.currentIndex += 1;
        }
        None
    }
}
