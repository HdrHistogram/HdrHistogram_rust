extern crate hdrsample;
use hdrsample::Histogram;

const TRACKABLE_MAX: i64 = 3600 * 1000 * 1000; // e.g. for 1 hr in usec units
const SIGFIG: u32 = 3;
const TEST_VALUE_LEVEL: i64 = 12340;

fn main() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut i = 0;
    loop {
        h += TEST_VALUE_LEVEL + (i & 0x8000);
        i += 1;
    }
}
