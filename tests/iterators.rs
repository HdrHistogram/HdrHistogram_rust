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
        .map(|iv| iv.value_iterated_to())
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
        .map(|iv| iv.value_iterated_to())
        .collect::<Vec<u64>>());
}

#[test]
fn iter_linear_count_since_last_iteration_saturates() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record_n(1, u64::max_value()).unwrap();
    h.record_n(4, u64::max_value() - 1).unwrap();
    h.record_n(5, u64::max_value() - 1).unwrap();
    h.record_n(6, 100).unwrap();
    h.record_n(7, 200).unwrap();
    h.record_n(10, 400).unwrap();

    let expected = vec![
        // 0-1 has 1's max value
        (1, u64::max_value()),
        // 2-3 has nothing
        (3, 0),
        // 4-5 has 2x (max - 1), should saturate
        (5, u64::max_value()),
        // 6-7 shouldn't be saturated from 4-5
        (7, 300),
        // 8-9 has nothing
        (9, 0),
        // 10-11 has just 10's count
        (11, 400)];

    // step in 2s to test count accumulation for each step
    assert_eq!(expected, h.iter_linear(2)
            .map(|iv| (iv.value_iterated_to(), iv.count_since_last_iteration()))
            .collect::<Vec<(u64, u64)>>());
}

#[test]
fn iter_linear_visits_buckets_wider_than_step_size_multiple_times() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record(1).unwrap();
    h.record(2047).unwrap();
    // bucket size 2
    h.record(2048).unwrap();
    h.record(2049).unwrap();
    h.record(4095).unwrap();
    // bucket size 4
    h.record(4096).unwrap();
    h.record(4097).unwrap();
    h.record(4098).unwrap();
    h.record(4099).unwrap();
    // 2nd bucket in size 4
    h.record(4100).unwrap();

    let iter_values = h.iter_linear(1)
            .map(|iv| (iv.value_iterated_to(), iv.count_since_last_iteration()))
            .collect::<Vec<(u64, u64)>>();

    // bucket size 1
    assert_eq!((0, 0), iter_values[0]);
    assert_eq!((1, 1), iter_values[1]);
    assert_eq!((2046, 0), iter_values[2046]);
    assert_eq!((2047, 1), iter_values[2047]);
    // bucket size 2
    assert_eq!((2048, 2), iter_values[2048]);
    assert_eq!((2049, 0), iter_values[2049]);
    assert_eq!((2050, 0), iter_values[2050]);
    assert_eq!((2051, 0), iter_values[2051]);
    assert_eq!((4094, 1), iter_values[4094]);
    assert_eq!((4095, 0), iter_values[4095]);
    // bucket size 4
    assert_eq!((4096, 4), iter_values[4096]);
    assert_eq!((4097, 0), iter_values[4097]);
    assert_eq!((4098, 0), iter_values[4098]);
    assert_eq!((4099, 0), iter_values[4099]);
    // also size 4, last bucket
    assert_eq!((4100, 1), iter_values[4100]);
    assert_eq!((4101, 0), iter_values[4101]);
    assert_eq!((4102, 0), iter_values[4102]);
    assert_eq!((4103, 0), iter_values[4103]);

    assert_eq!(4104, iter_values.len());
}

#[test]
fn iter_linear_visits_buckets_once_when_step_size_equals_bucket_size() {
    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    h.record(1).unwrap();
    h.record(2047).unwrap();
    // bucket size 2
    h.record(2048).unwrap();
    h.record(2049).unwrap();
    h.record(4095).unwrap();
    // bucket size 4
    h.record(4096).unwrap();
    h.record(4097).unwrap();
    h.record(4098).unwrap();
    h.record(4099).unwrap();
    // 2nd bucket in size 4
    h.record(4100).unwrap();

    let iter_values = h.iter_linear(4)
            .map(|iv| (iv.value_iterated_to(), iv.count_since_last_iteration()))
            .collect::<Vec<(u64, u64)>>();

    // bucket size 1
    assert_eq!((3, 1), iter_values[0]);
    assert_eq!((2047, 1), iter_values[511]);
    // bucket size 2
    assert_eq!((2051, 2), iter_values[512]);
    assert_eq!((4095, 1), iter_values[1023]);
    // bucket size 4
    assert_eq!((4099, 4), iter_values[1024]);
    // also size 4, last bucket
    assert_eq!((4103, 1), iter_values[1025]);

    assert_eq!(1026, iter_values.len());
}
