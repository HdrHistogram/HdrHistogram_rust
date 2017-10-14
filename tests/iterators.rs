extern crate hdrsample;

use hdrsample::Histogram;

#[test]
fn iter_recorded_non_saturated_total_count() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record(1).unwrap();
    h.record(1_000).unwrap();
    h.record(1_000_000).unwrap();

    let expected = vec![1, 1_000, h.highest_equivalent(1_000_000)];
    assert_eq!(expected, h.iter_recorded()
        .map(|iv| iv.value())
        .collect::<Vec<u64>>());
}

#[test]
fn iter_recorded_saturated_total_count() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value()).unwrap();
    h.record_n(1_000, u64::max_value()).unwrap();
    h.record_n(1_000_000, u64::max_value()).unwrap();

    let expected = vec![1, 1_000, h.highest_equivalent(1_000_000)];
    assert_eq!(expected, h.iter_recorded()
        .map(|iv| iv.value())
        .collect::<Vec<u64>>());
}
