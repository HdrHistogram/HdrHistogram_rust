use super::{CreationError, Histogram};

#[path = "helpers.rs"]
pub mod helpers;
#[path = "init.rs"]
mod init;
#[path = "index_calculation.rs"]
mod index_calculation;
#[path = "value_calculation.rs"]
mod value_calculation;
#[path = "subtract.rs"]
mod subtract;

#[test]
fn new_err_high_not_double_low() {
    let res = Histogram::<u64>::new_with_bounds(10, 15, 0);
    assert_eq!(CreationError::HighLessThanTwiceLow, res.unwrap_err());
}

#[test]
fn correct_original_min() {
    // until we get const fns, make sure workaround is correct
    assert_eq!(u64::max_value(), super::ORIGINAL_MIN);
}
