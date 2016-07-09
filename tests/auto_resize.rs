//! Tests from HistogramAutosizingTest.java

#![allow(non_snake_case)]

extern crate hdrsample;

use hdrsample::Histogram;

#[test]
#[ignore]
fn test_histogram_autosizing_edges() {
    let mut histogram = Histogram::<u64>::new(3).unwrap();
    histogram += (1i64 << 62) - 1;
    assert_eq!(histogram.bucketCount(), 52);
    assert_eq!(histogram.len(), 54272);
    histogram += i64::max_value();
    assert_eq!(histogram.bucketCount(), 53);
    assert_eq!(histogram.len(), 55296);
}

#[test]
#[ignore]
fn test_histogram_autosizing() {
    let mut histogram = Histogram::<u64>::new(3).unwrap();
    for i in 0..63 {
        histogram += 1i64 << i;
    }
    assert_eq!(histogram.bucketCount(), 53);
    assert_eq!(histogram.len(), 55296);
}

#[test]
fn test_autosizing_add() {
    let mut histogram1 = Histogram::<u64>::new(2).unwrap();
    let mut histogram2 = Histogram::<u64>::new(2).unwrap();

    histogram1 += 1000i64;
    histogram1 += 1000000000i64;

    histogram2 += &histogram1;
    assert!(histogram2.equivalent(histogram2.max(), 1000000000i64));
}

#[test]
fn test_autosizing_across_continuous_range() {
    let mut histogram = Histogram::<u64>::new(2).unwrap();

    for i in 0..10000000i64 {
        histogram += i;
    }
}
