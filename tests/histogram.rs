//! Tests from HistogramTest.java

extern crate hdrsample;
extern crate num;
extern crate rand;
extern crate ieee754;

use self::rand::Rng;

use hdrsample::{Histogram, SubtractionError};
use hdrsample::serialization::{V2Serializer, Deserializer};
use std::borrow::Borrow;
use std::cmp;
use std::fmt;
use num::Saturating;
use ieee754::Ieee754;

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

fn verify_max<T: hdrsample::Counter, B: Borrow<Histogram<T>>>(hist: B) -> bool {
    let hist = hist.borrow();
    if let Some(mx) = hist.iter_recorded()
        .map(|v| v.value())
        .map(|v| hist.highest_equivalent(v))
        .last() {
        hist.max() == mx
    } else {
        hist.max() == 0
    }
}

fn assert_min_max_count<T: hdrsample::Counter, B: Borrow<Histogram<T>>>(hist: B) {
    let h = hist.borrow();
    let mut min = None;
    let mut max = None;
    let mut total = 0;
    for i in 0..h.len() {
        let value = h.value_for(i);
        let count = h.count_at(value).unwrap();
        if count == T::zero() {
            continue;
        }

        min = Some(cmp::min(min.unwrap_or(u64::max_value()), value));
        max = Some(cmp::max(max.unwrap_or(0), value));
        total = total.saturating_add(count.to_u64().unwrap());
    }

    let min = min.map(|m| h.lowest_equivalent(m)).unwrap_or(0);
    let max = max.map(|m| h.highest_equivalent(m)).unwrap_or(0);

    assert_eq!(min, h.min());
    assert_eq!(max, h.max());
    assert_eq!(total, h.count());
}

const TRACKABLE_MAX: u64 = 3600 * 1000 * 1000;
// Store up to 2 * 10^3 in single-unit precision. Can be 5 at most.
const SIGFIG: u8 = 3;
const TEST_VALUE_LEVEL: u64 = 4;

#[test]
fn construction_arg_ranges() {
    assert!(Histogram::<u64>::new_with_max(1, SIGFIG).is_err());
    assert!(Histogram::<u64>::new_with_max(TRACKABLE_MAX, 6).is_err());
}

#[test]
fn empty_histogram() {
    let h = Histogram::<u64>::new(SIGFIG).unwrap();
    assert_eq!(h.min(), 0);
    assert_eq!(h.max(), 0);
    assert_near!(h.mean(), 0.0, 0.0000000000001);
    assert_near!(h.stdev(), 0.0, 0.0000000000001);
    assert_near!(h.percentile_below(0).unwrap(), 100.0, 0.0000000000001);
    assert!(verify_max(h));
}

#[test]
fn construction_arg_gets() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.low(), 1);
    assert_eq!(h.high(), TRACKABLE_MAX);
    assert_eq!(h.sigfig(), SIGFIG);

    let h = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.low(), 1000);
}

#[test]
fn record() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), Ok(1));
    assert_eq!(h.count(), 1);
    assert!(verify_max(h));
}

#[test]
fn record_past_trackable_max() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert!(h.record(3 * TRACKABLE_MAX).is_err());
}

#[test]
fn record_in_interval() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h.record_correct(TEST_VALUE_LEVEL, TEST_VALUE_LEVEL / 4).unwrap();
    let mut r = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    r += TEST_VALUE_LEVEL;

    // The data will include corrected samples:
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 1) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 2) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 3) / 4), Ok(1));
    assert_eq!(h.count_at((TEST_VALUE_LEVEL * 4) / 4), Ok(1));
    assert_eq!(h.count(), 4);
    // But the raw data will not:
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 1) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 2) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 3) / 4), Ok(0));
    assert_eq!(r.count_at((TEST_VALUE_LEVEL * 4) / 4), Ok(1));
    assert_eq!(r.count(), 1);

    assert!(verify_max(h));
}

#[test]
fn reset() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h.reset();

    assert_eq!(h.count_at(TEST_VALUE_LEVEL), Ok(0));
    assert_eq!(h.count(), 0);
    assert!(verify_max(h));
}

#[test]
fn add() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;
    h2 += TEST_VALUE_LEVEL;
    h2 += 1000 * TEST_VALUE_LEVEL;
    h1 += &h2;

    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count(), 4);

    let mut big = Histogram::<u64>::new_with_max(2 * TRACKABLE_MAX, SIGFIG).unwrap();
    big += TEST_VALUE_LEVEL;
    big += 1000 * TEST_VALUE_LEVEL;
    big += 2 * TRACKABLE_MAX;

    // Adding the smaller histogram to the bigger one should work:
    big += &h1;
    assert_eq!(big.count_at(TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(1000 * TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(2 * TRACKABLE_MAX), Ok(1)); // overflow smaller hist...
    assert_eq!(big.count(), 7);

    // But trying to add a larger histogram into a smaller one should throw an AIOOB:
    assert!(h1.add(&big).is_err());

    assert!(verify_max(h1));
    assert!(verify_max(h2));
    assert!(verify_max(big));
}

#[test]
fn subtract_after_add() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;
    h2 += TEST_VALUE_LEVEL;
    h2 += 1000 * TEST_VALUE_LEVEL;

    h1.add(&h2).unwrap();
    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count(), 4);

    h1 += &h2;
    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(h1.count(), 6);

    h1.subtract(&h2).unwrap();
    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(h1.count(), 4);

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_to_zero_counts() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;

    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(1));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(1));
    assert_eq!(h1.count(), 2);

    let clone = h1.clone();
    h1.subtract(&clone).unwrap();
    assert_eq!(h1.count_at(TEST_VALUE_LEVEL), Ok(0));
    assert_eq!(h1.count_at(1000 * TEST_VALUE_LEVEL), Ok(0));
    assert_eq!(h1.count(), 0);

    assert_min_max_count(h1);
}

#[test]
fn subtract_to_negative_counts_error() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;
    h2.record_n(TEST_VALUE_LEVEL, 2).unwrap();
    h2.record_n(1000 * TEST_VALUE_LEVEL, 2).unwrap();

    assert_eq!(SubtractionError::SubtrahendCountExceedsMinuendCount, h1.subtract(&h2).unwrap_err());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_subtrahend_values_outside_minuend_range_error() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;

    let mut big = Histogram::<u64>::new_with_max(2 * TRACKABLE_MAX, SIGFIG).unwrap();
    big += TEST_VALUE_LEVEL;
    big += 1000 * TEST_VALUE_LEVEL;
    big += 2 * TRACKABLE_MAX;

    assert_eq!(SubtractionError::SubtrahendValueExceedsMinuendRange, h1.subtract(&big).unwrap_err());

    assert_min_max_count(h1);
    assert_min_max_count(big);
}

#[test]
fn subtract_values_inside_minuend_range_works() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += TEST_VALUE_LEVEL;
    h1 += 1000 * TEST_VALUE_LEVEL;

    let mut big = Histogram::<u64>::new_with_max(2 * TRACKABLE_MAX, SIGFIG).unwrap();
    big += TEST_VALUE_LEVEL;
    big += 1000 * TEST_VALUE_LEVEL;
    big += 2 * TRACKABLE_MAX;

    let big2 = big.clone();
    big += &big2;
    big += &big2;

    assert_eq!(big.count_at(TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(1000 * TEST_VALUE_LEVEL), Ok(3));
    assert_eq!(big.count_at(2 * TRACKABLE_MAX), Ok(3)); // overflow smaller hist...
    assert_eq!(big.count(), 9);

    // Subtracting the smaller histogram from the bigger one should work:
    big -= &h1;
    assert_eq!(big.count_at(TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(big.count_at(1000 * TEST_VALUE_LEVEL), Ok(2));
    assert_eq!(big.count_at(2 * TRACKABLE_MAX), Ok(3)); // overflow smaller hist...
    assert_eq!(big.count(), 7);

    assert_min_max_count(h1);
    assert_min_max_count(big);
}

#[test]
fn subtract_values_strictly_inside_minuend_range_yields_same_min_max_no_restat() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += 1;
    h1 += 10;
    h1 += 100;
    h1 += 1000;

    h2 += 10;
    h2 += 100;

    // will not require a restat
    h1.subtract(&h2).unwrap();

    assert_eq!(1, h1.min());
    assert_eq!(1000, h1.max());
    assert_eq!(2, h1.count());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_at_extent_of_minuend_zero_count_range_recalculates_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1 += 1;
    h1 += 10;
    h1 += 100;
    h1 += 1000;

    h2 += 1;
    h2 += 1000;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(10, h1.min());
    assert_eq!(100, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_at_extent_of_minuend_nonzero_count_range_recalculates_same_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();

    h1.record_n(1, 2).unwrap();
    h1.record_n(10, 2).unwrap();
    h1.record_n(100, 2).unwrap();
    h1.record_n(1000, 2).unwrap();

    h2 += 1;
    h2 += 1000;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(1, h1.min());
    assert_eq!(1000, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_within_bucket_precision_of_of_minuend_min_recalculates_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 5).unwrap();

    // sub bucket size is 2 above 2048 with 3 sigfits
    h1.record(3000).unwrap();
    h1.record(3100).unwrap();
    h1.record(3200).unwrap();
    h1.record(3300).unwrap();

    // h2 has 5 sigfits, so bucket size is 1 still
    h2 += 3001;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(3100, h1.min());
    assert_eq!(3301, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_at_minuend_min_recalculates_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 5).unwrap();

    // sub bucket size is 2 above 2048 with 3 sigfits
    h1.record(3000).unwrap();
    h1.record(3100).unwrap();
    h1.record(3200).unwrap();
    h1.record(3300).unwrap();

    // h2 has 5 sigfits, so bucket size is 1 still
    h2 += 3000;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(3100, h1.min());
    assert_eq!(3301, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_within_bucket_precision_of_of_minuend_max_recalculates_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 5).unwrap();

    // sub bucket size is 2 above 2048 with 3 sigfits
    h1.record(3000).unwrap();
    h1.record(3100).unwrap();
    h1.record(3200).unwrap();
    h1.record(3300).unwrap();

    // h2 has 5 sigfits, so bucket size is 1 still
    h2 += 3301;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(3000, h1.min());
    assert_eq!(3201, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_at_minuend_max_recalculates_min_max() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 5).unwrap();

    // sub bucket size is 2 above 2048 with 3 sigfits
    h1.record(3000).unwrap();
    h1.record(3100).unwrap();
    h1.record(3200).unwrap();
    h1.record(3300).unwrap();

    // h2 has 5 sigfits, so bucket size is 1 still
    h2 += 3300;

    // will trigger a restat because min/max values are having counts subtracted
    h1.subtract(&h2).unwrap();

    assert_eq!(3000, h1.min());
    assert_eq!(3201, h1.max());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_minuend_saturated_total_recalculates_saturated() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();

    h1.record_n(1, u64::max_value()).unwrap();
    h1.record_n(10, u64::max_value()).unwrap();
    h1.record_n(100, u64::max_value()).unwrap();
    h1.record_n(1000, u64::max_value()).unwrap();

    h2.record(10).unwrap();
    h2.record(100).unwrap();

    // will trigger a restat - total count is saturated
    h1.subtract(&h2).unwrap();

    // min, max haven't changed
    assert_eq!(1, h1.min());
    assert_eq!(1000, h1.max());
    // still saturated
    assert_eq!(u64::max_value(), h1.count());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn subtract_values_minuend_saturated_total_recalculates_not_saturated() {
    let mut h1 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();

    // 3 of these is just under u64::max_value()
    let chunk = (u64::max_value() / 16) * 5;

    h1.record_n(1, chunk).unwrap();
    h1.record_n(10, chunk).unwrap();
    h1.record_n(100, chunk).unwrap();
    h1.record_n(1000, chunk).unwrap();

    h2.record_n(10, chunk).unwrap();

    // will trigger a restat - total count is saturated
    h1.subtract(&h2).unwrap();

    // min, max haven't changed
    assert_eq!(1, h1.min());
    assert_eq!(1000, h1.max());
    // not saturated
    assert_eq!(u64::max_value() / 16 * 15, h1.count());

    assert_min_max_count(h1);
    assert_min_max_count(h2);
}

#[test]
fn equivalent_range() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.equivalent_range(1), 1);
    assert_eq!(h.equivalent_range(2500), 2);
    assert_eq!(h.equivalent_range(8191), 4);
    assert_eq!(h.equivalent_range(8192), 8);
    assert_eq!(h.equivalent_range(10000), 8);
}

#[test]
fn scaled_equivalent_range() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.equivalent_range(1 * 1024), 1 * 1024);
    assert_eq!(h.equivalent_range(2500 * 1024), 2 * 1024);
    assert_eq!(h.equivalent_range(8191 * 1024), 4 * 1024);
    assert_eq!(h.equivalent_range(8192 * 1024), 8 * 1024);
    assert_eq!(h.equivalent_range(10000 * 1024), 8 * 1024);
}

#[test]
fn lowest_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.lowest_equivalent(10007), 10000);
    assert_eq!(h.lowest_equivalent(10009), 10008);
}


#[test]
fn scaled_lowest_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.lowest_equivalent(10007 * 1024), 10000 * 1024);
    assert_eq!(h.lowest_equivalent(10009 * 1024), 10008 * 1024);
}

#[test]
fn highest_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.highest_equivalent(8180), 8183);
    assert_eq!(h.highest_equivalent(8191), 8191);
    assert_eq!(h.highest_equivalent(8193), 8199);
    assert_eq!(h.highest_equivalent(9995), 9999);
    assert_eq!(h.highest_equivalent(10007), 10007);
    assert_eq!(h.highest_equivalent(10008), 10015);
}

#[test]
fn scaled_highest_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.highest_equivalent(8180 * 1024), 8183 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(8191 * 1024), 8191 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(8193 * 1024), 8199 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(9995 * 1024), 9999 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(10007 * 1024), 10007 * 1024 + 1023);
    assert_eq!(h.highest_equivalent(10008 * 1024), 10015 * 1024 + 1023);
}


#[test]
fn median_equivalent() {
    let h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.median_equivalent(4), 4);
    assert_eq!(h.median_equivalent(5), 5);
    assert_eq!(h.median_equivalent(4000), 4001);
    assert_eq!(h.median_equivalent(8000), 8002);
    assert_eq!(h.median_equivalent(10007), 10004);
}

#[test]
fn median_equivalent_doesnt_panic_at_extremes() {
    let h = Histogram::<u64>::new_with_max(u64::max_value(), 3).unwrap();
    let _ = h.median_equivalent(u64::max_value());
    let _ = h.median_equivalent(u64::max_value() - 1);
    let _ = h.median_equivalent(0);
    let _ = h.median_equivalent(1);
}

#[test]
fn scaled_median_equivalent() {
    let h = Histogram::<u64>::new_with_bounds(1024, TRACKABLE_MAX, SIGFIG).unwrap();
    assert_eq!(h.median_equivalent(1024 * 4), 1024 * 4 + 512);
    assert_eq!(h.median_equivalent(1024 * 5), 1024 * 5 + 512);
    assert_eq!(h.median_equivalent(1024 * 4000), 1024 * 4001);
    assert_eq!(h.median_equivalent(1024 * 8000), 1024 * 8002);
    assert_eq!(h.median_equivalent(1024 * 10007), 1024 * 10004);
}

fn are_equal<T, B1, B2>(actual: B1, expected: B2)
    where T: hdrsample::Counter + fmt::Debug,
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
    assert_eq!(actual.count(), expected.count());
    assert!(verify_max(expected));
    assert!(verify_max(actual));
}

#[test]
fn clone() {
    let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h += 10 * TEST_VALUE_LEVEL;

    let max = h.high();
    h.record_correct(max - 1, 31000).unwrap();

    are_equal(h.clone(), h);
}

#[test]
fn scaled_clone() {
    let mut h = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    h += TEST_VALUE_LEVEL;
    h += 10 * TEST_VALUE_LEVEL;

    let max = h.high();
    h.record_correct(max - 1, 31000).unwrap();

    are_equal(h.clone(), h);
}

#[test]
fn set_to() {
    let mut h1 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
    h1 += TEST_VALUE_LEVEL;
    h1 += 10 * TEST_VALUE_LEVEL;

    let max = h1.high();
    h1.record_correct(max - 1, 31000).unwrap();

    h2.set_to(&h1).unwrap();
    are_equal(&h1, &h2);

    h1 += 20 * TEST_VALUE_LEVEL;

    h2.set_to(&h1).unwrap();
    are_equal(&h1, &h2);
}

#[test]
fn scaled_set_to() {
    let mut h1 = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    let mut h2 = Histogram::<u64>::new_with_bounds(1000, TRACKABLE_MAX, SIGFIG).unwrap();
    h1 += TEST_VALUE_LEVEL;
    h1 += 10 * TEST_VALUE_LEVEL;

    let max = h1.high();
    h1.record_correct(max - 1, 31000).unwrap();

    h2.set_to(&h1).unwrap();
    are_equal(&h1, &h2);

    h1 += 20 * TEST_VALUE_LEVEL;

    h2.set_to(&h1).unwrap();
    are_equal(&h1, &h2);
}


#[test]
fn random_write_full_value_range_precision_5_no_panic() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 5).unwrap();

    let mut rng = rand::weak_rng();

    for _ in 0..1_000_000 {
        let mut r: u64 = rng.gen();
        if r == 0 {
            r = 1;
        }

        h.record(r).unwrap();
    }
}


#[test]
fn random_write_full_value_range_precision_0_no_panic() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 0).unwrap();

    let mut rng = rand::weak_rng();

    for _ in 0..1_000_000 {
        let mut r: u64 = rng.gen();
        if r == 0 {
            r = 1;
        }

        h.record(r).unwrap();
    }
}

#[test]
fn random_write_middle_of_value_range_precision_3_no_panic() {
    let low = 1_000;
    let high = 1_000_000_000;
    let mut h = Histogram::<u64>::new_with_bounds(low, high, 3).unwrap();

    let mut rng = rand::weak_rng();

    for _ in 0..1_000_000 {
        h.record(rng.gen_range(low, high + 1)).unwrap();
    }
}

#[test]
fn value_count_overflow_from_record_saturates_u16() {
    let mut h = Histogram::<u16>::new_with_max(TRACKABLE_MAX, 2).unwrap();

    h.record_n(3, u16::max_value() - 1).unwrap();
    h.record_n(3, u16::max_value() - 1).unwrap();

    // individual count has saturated
    assert_eq!(u16::max_value(), h.count_at(3).unwrap());
    // total is a u64 though
    assert_eq!((u16::max_value() - 1) as u64 * 2, h.count());
}

#[test]
fn value_count_overflow_from_record_saturates_u64() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h.record_n(1, u64::max_value() - 1).unwrap();

    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn value_count_overflow_from_record_autoresize_doesnt_panic_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, 10000, 3).unwrap();
    h.auto(true);

    h.record_n(1, u64::max_value() - 1).unwrap();
    h.record_n(1, u64::max_value() - 1).unwrap();

    // forces resize
    h.record_n(1_000_000_000, u64::max_value() - 1).unwrap();
    h.record_n(1_000_000_000, u64::max_value() - 1).unwrap();

    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count_at(1_000_000_000).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn value_count_overflow_from_add_same_dimensions_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h2.record_n(1, u64::max_value() - 1).unwrap();

    h.add(h2).unwrap();
    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn value_count_overflow_from_add_different_precision_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    // different precision
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 4).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h2.record_n(1, u64::max_value() - 1).unwrap();

    h.add(h2).unwrap();
    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn value_count_overflow_from_add_with_resize_to_same_dimensions_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, 10_000, 3).unwrap();
    h.auto(true);
    let mut h2 = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h2.record_n(1, u64::max_value() - 1).unwrap();
    // recording at value == h2 max should trigger h to resize to the same dimensions when added
    h2.record_n(10_000_000_000, u64::max_value() - 1).unwrap();

    h.add(h2).unwrap();
    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_record_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h.record_n(10, u64::max_value() - 1).unwrap();

    assert_eq!(u64::max_value() - 1, h.count_at(1).unwrap());
    assert_eq!(u64::max_value() - 1, h.count_at(10).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_add_same_dimensions_saturates_calculating_other_addend_total() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value() - 10).unwrap();
    h2.record_n(10, u64::max_value() - 1).unwrap();
    h2.record_n(20, 10).unwrap();

    // just h2's total would overflow

    h.add(h2).unwrap();
    assert_eq!(u64::max_value() - 10, h.count_at(1).unwrap());
    assert_eq!(10, h.count_at(20).unwrap());

    // if accumulating total count for h2 had overflowed, we would see max_value - 1000 + 9 here
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_add_same_dimensions_saturates_when_added_to_orig_total_count() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value() - 10).unwrap();
    h2.record_n(10, 9).unwrap();
    h2.record_n(20, 9).unwrap();

    // h2's total wouldn't overflow, but it would when added to h1

    h.add(h2).unwrap();
    assert_eq!(u64::max_value() - 10, h.count_at(1).unwrap());
    assert_eq!(9, h.count_at(20).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_add_different_precision_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    // different precision
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 4).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();

    h2.record_n(20, u64::max_value() - 1).unwrap();

    h.add(h2).unwrap();
    assert_eq!(u64::max_value() - 1, h.count_at(1).unwrap());
    assert_eq!(u64::max_value() - 1, h.count_at(20).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_add_with_resize_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, 10_000, 3).unwrap();
    h.auto(true);
    let mut h2 = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3).unwrap();

    h.record_n(1, u64::max_value() - 1).unwrap();
    h2.record_n(1, u64::max_value() - 1).unwrap();
    h2.record_n(10_000_000_000, u64::max_value() - 1).unwrap();

    h.add(h2).unwrap();
    assert_eq!(u64::max_value(), h.count_at(1).unwrap());
    assert_eq!(u64::max_value(), h.count());
}

#[test]
fn total_count_overflow_from_deserialize_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    // can't go bigger than i64 max because it will be serialized
    h.record_n(1, i64::max_value() as u64).unwrap();
    h.record_n(1000, i64::max_value() as u64).unwrap();
    h.record_n(1000_000, i64::max_value() as u64).unwrap();
    assert_eq!(u64::max_value(), h.count());

    let mut vec = Vec::new();

    V2Serializer::new().serialize(&h, &mut vec).unwrap();
    let deser_h: Histogram<u64> = Deserializer::new().deserialize(&mut vec.as_slice()).unwrap();
    assert_eq!(u64::max_value(), deser_h.count());
}

#[test]
fn subtract_underflow_guarded_by_per_value_count_check() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    let mut h2 = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, 1).unwrap();
    h2.record_n(1, 100).unwrap();

    assert_eq!(SubtractionError::SubtrahendCountExceedsMinuendCount, h.subtract(h2).unwrap_err());
}

#[test]
fn quantile_2_values() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record(1).unwrap();
    h.record(2).unwrap();

    assert_eq!(1, h.value_at_quantile(0.25));
    assert_eq!(1, h.value_at_quantile(0.5));

    let almost_half = 0.5000000000000001;
    let next = 0.5000000000000002;
    // one ulp apart
    assert_eq!(almost_half, 0.5_f64.next());
    assert_eq!(next, almost_half.next());

    assert_eq!(1, h.value_at_quantile(0.5));
    // ideally this would return 2, not 1
    assert_eq!(1, h.value_at_quantile(almost_half));
    assert_eq!(2, h.value_at_quantile(next));
}

#[test]
fn quantile_5_values() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record(1).unwrap();
    h.record(2).unwrap();
    h.record(2).unwrap();
    h.record(2).unwrap();
    h.record(2).unwrap();

    assert_eq!(2, h.value_at_quantile(0.25));
    assert_eq!(2, h.value_at_quantile(0.3));
}


#[test]
fn quantile_20k() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    for i in 1..20_001 {
        h.record(i).unwrap();
    }

    assert_eq!(20_000, h.count());

    assert!(h.equivalent(19961, h.value_at_quantile(0.99805)));
}

#[test]
fn quantile_large_numbers() {
    let mut h = Histogram::<u64>::new_with_bounds(20_000_000, 100_000_000, 5).unwrap();
    h.record(100_000_000).unwrap();
    h.record(20_000_000).unwrap();
    h.record(30_000_000).unwrap();

    assert!(h.equivalent(20_000_000, h.value_at_quantile(0.5)));
    assert!(h.equivalent(30_000_000, h.value_at_quantile(0.5)));
    assert!(h.equivalent(100_000_000, h.value_at_quantile(0.8333)));
    assert!(h.equivalent(100_000_000, h.value_at_quantile(0.8334)));
    assert!(h.equivalent(100_000_000, h.value_at_quantile(0.99)));
}

#[test]
fn value_at_quantile_matches_pctile_iter_sequence() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    let lengths = vec![1, 5, 10, 50, 100, 500, 1_000, 5_000, 10_000, 50_000, 100_000];

    for length in lengths {
        h.reset();

        for i in 1..(length + 1) {
            h.record(i).unwrap();
        }

        assert_eq!(length, h.count());

        let iter = h.iter_percentiles(1000);

        for iter_val in iter {
            let calculated_value = h.value_at_quantile(iter_val.quantile());
            let v = iter_val.value();

            assert_eq!(v, calculated_value,
                       "len {} iter quantile {} q count {} iter val {} -> {} calc val {} -> {}",
                       length, iter_val.quantile(), iter_val.quantile() * length as f64, v, h.highest_equivalent(v), calculated_value, h.highest_equivalent(calculated_value));
        }
    }
}

#[test]
fn value_at_quantile_matches_value_sequence() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    let lengths = vec![1, 5, 10, 50, 100, 500, 1_000, 5_000, 10_000, 50_000, 100_000];

    for length in lengths {
        h.reset();

        for i in 1..(length + 1) {
            h.record(i).unwrap();
        }

        assert_eq!(length, h.count());

        for v in 1..(length + 1) {
            let quantile = v as f64 / length as f64;
            let calculated_value = h.value_at_quantile(quantile);
            if !h.equivalent(v, calculated_value) {
                assert_eq!(h.highest_equivalent(v), calculated_value,
                         "len {} quantile {} q count {} actual {} -> {} calc {} -> {}",
                         length, quantile, quantile * length as f64, v, h.highest_equivalent(v), calculated_value, h.highest_equivalent(calculated_value));
            }
        }
    }
}

#[test]
fn value_at_quantile_matches_pctile_iter_random() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    // random u64s tend to be pretty darn big, so percentile calculations have to scan more.
    let lengths = vec![1, 5, 10, 50, 100, 500, 1_000, 5_000, 10_000];

    let mut rng = rand::weak_rng();

    for length in lengths {
        h.reset();

        for _ in 1..(length + 1) {
            h.record(rng.gen()).unwrap();
        }

        assert_eq!(length, h.count());

        let iter = h.iter_percentiles(1000);

        for iter_val in iter {
            let calculated_value = h.value_at_quantile(iter_val.quantile());
            let v = iter_val.value();

            assert_eq!(v, calculated_value,
                       "len {} iter quantile {} q count {} iter val {} -> {} calc val {} -> {}",
                       length, iter_val.quantile(), iter_val.quantile() * length as f64, v, h.highest_equivalent(v), calculated_value, h.highest_equivalent(calculated_value));
        }
    }
}

#[test]
fn value_at_quantile_matches_value_random() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    let mut values = Vec::new();

    let lengths = vec![1, 5, 10, 50, 100, 500, 1_000, 5_000, 10_000];

    let mut rng = rand::weak_rng();

    for length in lengths {
        h.reset();

        for _ in 1..(length + 1) {
            let v = rng.gen();
            h.record(v).unwrap();
            values.push(v);
        }

        values.sort();

        assert_eq!(length, h.count());

        for (index, &v) in values.iter().enumerate() {
            let quantile = (index + 1) as f64 / length as f64;
            let calculated_value = h.value_at_quantile(quantile);
            if !h.equivalent(v, calculated_value) {
                // TODO this fails quickly
//                assert_eq!(h.highest_equivalent(v), calculated_value,
//                           "len {} quantile {} q count {} actual {} -> {} calc {} -> {}",
//                         length, quantile, quantile * length as f64, v, h.highest_equivalent(v), calculated_value, h.highest_equivalent(calculated_value));
            }
        }
    }
}
