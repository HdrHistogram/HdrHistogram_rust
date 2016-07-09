//! Tests from HistogramTest.java

#![allow(non_snake_case)]

extern crate hdrsample;
extern crate num;

use hdrsample::Histogram;
use std::borrow::Borrow;
use std::fmt;

macro_rules! assert_near {
    ($a: expr, $b: expr, $tolerance: expr) => {{
        let a = $a as f64;
        let b = $b as f64;
        let tol = $tolerance as f64;
        assert!((a - b).abs() <= b * tol,
            "assertion failed: `(left ~= right) (left: `{}`, right: `{}`, tolerance: `{:.5}%`)",
            a,
            b,
            100.0 * tol);
    }}
}

fn verify_max<T: num::Num + num::ToPrimitive + Copy, B: Borrow<Histogram<T>>>(hist: B) -> bool {
    let hist = hist.borrow();
    if let Some(mx) = hist.iter_recorded()
        .map(|(v, _, _, _)| v)
        .map(|v| hist.highest_equivalent(v))
        .last() {
        hist.max() == mx
    } else {
        hist.max() == 0
    }
}

const TRACKABLE_MAX: i64 = 3600 * 1000 * 1000;
const SIGFIG: u32 = 3;
const TEST_VALUE_LEVEL: i64 = 4;

#[test]
fn test_construction_arg_ranges() {
    assert!(Histogram::<u64>::new_with_max(1, SIGFIG).is_err());
    assert!(Histogram::<u64>::new_with_max(TRACKABLE_MAX, 6).is_err());
}

#[test]
fn test_empty_histogram() {
    let h = Histogram::<u64>::new(SIGFIG).unwrap();
    assert_eq!(h.min(), 0);
    assert_eq!(h.max(), 0);
    assert_near!(h.mean(), 0.0, 0.0000000000001);
    assert_near!(h.stdev(), 0.0, 0.0000000000001);
    assert_near!(h.percentile_below(0), 100.0, 0.0000000000001);
    assert!(verify_max(h));
}

#[test]
fn test_construction_arg_gets() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.getLowestDiscernibleValue(), 1);
    assert_eq!(h.getHighestTrackableValue(), TRACKABLE_MAX);
    assert_eq!(h.getNumberOfSignificantValueDigits(), SIGFIG);

    let h = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.getLowestDiscernibleValue(), 1000);
}

#[test]
fn test_record() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), Ok(1));
    assert_eq!(h.total(), 1);
    assert!(verify_max(h));
}

#[test]
fn test_record_overflow() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert!(h.record(3 * TRACKABLE_MAX).is_err());
}

#[test]
fn test_create_with_large_values() {
    let mut h = Histogram::<u64>::new_with_bounds(20000000, 100000000, 5).unwrap();

    h += 100000000;
    h += 20000000;
    h += 30000000;

    assert!(h.equivalent(20000000, h.value_at_percentile(50.0)));
    assert!(h.equivalent(30000000, h.value_at_percentile(83.33)));
    assert!(h.equivalent(100000000, h.value_at_percentile(83.34)));
    assert!(h.equivalent(100000000, h.value_at_percentile(99.0)));
}

#[test]
fn test_record_in_interval() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h.recordInInterval(TEST_VALUE_LEVEL, TEST_VALUE_LEVEL / 4).unwrap();
    let mut r = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    r += TEST_VALUE_LEVEL;

    // The data will include corrected samples:
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 1) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 2) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 3) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 4) / 4), Ok(1));
    assert_eq!(h.total(), 4);
    // But the raw data will not:
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 1) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 2) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 3) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 4) / 4), Ok(1));
    assert_eq!(r.total(), 1);

    assert!(verify_max(h));
}

#[test]
fn test_reset() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h.reset();

    assert_eq!(h.count_at(TEST_VALUE_LEVEL), Ok(0));
    assert_eq!(h.total(), 0);
    assert!(verify_max(h));
}

#[test]
fn test_add() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;
    h2 += TEST_VALUE_LEVEL;
    h2 += 1000 * TEST_VALUE_LEVEL;
    h1 += &h2;

    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.total(), 4);

    let mut big = Histogram::<u64>::new_with_max(2 * TRACKABLE_MAX, SIGFIG).unwrap();
    big += TEST_VALUE_LEVEL;
    big += 1000 * TEST_VALUE_LEVEL;
    big += 2 * TRACKABLE_MAX;

    // Adding the smaller histogram to the bigger one should work:
    big += &h1;
    assert_eq!(big.count_at(TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(1000 * TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(2 * TRACKABLE_MAX), Ok(1)); // overflow smaller hist...
    assert_eq!(big.total(), 7);

    // But trying to add a larger histogram into a smaller one should throw an AIOOB:
    assert!(h1.add(&big).is_err());

    assert!(verify_max(h1));
    assert!(verify_max(h2));
    assert!(verify_max(big));
}

#[test]
fn test_equivalent_range_len() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.equivalent_range_len(1), 1);
    assert_eq!(h.equivalent_range_len(2500), 2);
    assert_eq!(h.equivalent_range_len(8191), 4);
    assert_eq!(h.equivalent_range_len(8192), 8);
    assert_eq!(h.equivalent_range_len(10000), 8);
}

#[test]
fn test_scaled_equivalent_range_len() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.equivalent_range_len(1 * 1024), 1 * 1024);
    assert_eq!(h.equivalent_range_len(2500 * 1024), 2 * 1024);
    assert_eq!(h.equivalent_range_len(8191 * 1024), 4 * 1024);
    assert_eq!(h.equivalent_range_len(8192 * 1024), 8 * 1024);
    assert_eq!(h.equivalent_range_len(10000 * 1024), 8 * 1024);
}

#[test]
fn test_lowest_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.lowest_equivalent(10007), 10000);
    assert_eq!(h.lowest_equivalent(10009), 10008);
}


#[test]
fn test_scaled_lowest_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.lowest_equivalent(10007 * 1024), 10000 * 1024);
    assert_eq!(h.lowest_equivalent(10009 * 1024), 10008 * 1024);
}

#[test]
fn test_highest_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.highest_equivalent(8180), 8183);
    assert_eq!(h.highest_equivalent(8191), 8191);
    assert_eq!(h.highest_equivalent(8193), 8199);
    assert_eq!(h.highest_equivalent(9995), 9999);
    assert_eq!(h.highest_equivalent(10007), 10007);
    assert_eq!(h.highest_equivalent(10008), 10015);
}

#[test]
fn test_scaled_highest_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.highest_equivalent(8180 * 1024), 8183 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(8191 * 1024), 8191 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(8193 * 1024), 8199 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(9995 * 1024), 9999 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(10007 * 1024), 10007 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(10008 * 1024), 10015 * 1024 + 1023);
}


#[test]
fn test_median_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.median_equivalent(4), 4);
    assert_eq!(h.median_equivalent(5), 5);
    assert_eq!(h.median_equivalent(4000), 4001);
    assert_eq!(h.median_equivalent(8000), 8002);
    assert_eq!(h.median_equivalent(10007), 10004);
}

#[test]
fn test_scaled_median_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.median_equivalent(1024 * 4), 1024 * 4 + 512);
    assert_eq!(h.median_equivalent(1024 * 5), 1024 * 5 + 512);
    assert_eq!(h.median_equivalent(1024 * 4000), 1024 * 4001);
    assert_eq!(h.median_equivalent(1024 * 8000), 1024 * 8002);
    assert_eq!(h.median_equivalent(1024 * 10007), 1024 * 10004);
}

#[test]
#[should_panic]
fn test_overflow() {
    let mut h = Histogram::<i16>::new_with_max(TRACKABLE_MAX, 2).unwrap();
    h += TEST_VALUE_LEVEL;
    h += 10 * TEST_VALUE_LEVEL;
    // This should overflow a short:
    let max = h.getHighestTrackableValue();
    drop(h.recordInInterval(max - 1, 500));
}

fn are_equal<T, B1, B2>(actual: B1, expected: B2)
    where T: num::Num + num::ToPrimitive + Copy + fmt::Debug,
          B1: Borrow<Histogram<T>>,
          B2: Borrow<Histogram<T>>
{
    let actual = actual.borrow();
    let expected = expected.borrow();

    assert!(actual == expected);
    assert_eq!(actual.count_at(TEST_VALUE_LEVEL),
               expected.count_at(TEST_VALUE_LEVEL));
    assert_eq!(actual.count_at(10 * TEST_VALUE_LEVEL),
               expected.count_at(10 * TEST_VALUE_LEVEL));
    assert_eq!(actual.total(), expected.total());
    assert!(verify_max(expected));
    assert!(verify_max(actual));
}

#[test]
fn test_clone() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h += 10 * TEST_VALUE_LEVEL;

    let max = h.getHighestTrackableValue();
    h.recordInInterval(max - 1, 31000).unwrap();

    are_equal(h.clone(), h);
}

#[test]
fn test_scaled_clone() {
    let mut h = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h += 10 * TEST_VALUE_LEVEL;

    let max = h.getHighestTrackableValue();
    h.recordInInterval(max - 1, 31000).unwrap();

    are_equal(h.clone(), h);
}
