use super::Histogram;

#[path = "helpers.rs"]
mod helpers;
#[path = "init.rs"]
mod init;
#[path = "index_calculation.rs"]
mod index_calculation;

#[test]
fn new_err_high_not_double_low() {
    let res = Histogram::<u64>::new_with_bounds(10, 15, 0);
    assert_eq!("highest trackable value must be >= 2 * lowest discernible value", res.unwrap_err());
}
