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

#[test]
fn iter_all_values_all_buckets() {
    let mut h = histo64(1, 8191, 3);

    h.record(1).unwrap();
    h.record(2).unwrap();
    // first in top half
    h.record(1024).unwrap();
    // first in 2nd bucket
    h.record(2048).unwrap();
    // first in 3rd
    h.record(4096).unwrap();
    // smallest value in last sub bucket of third
    h.record(8192 - 4).unwrap();

    let iter_values: Vec<(u64, u64)> = h.iter_all()
            .map(|v| (v.value_iterated_to(), v.count_at_value()))
            .collect();

    // 4096 distinct expressible values
    assert_eq!(2048 + 2 * 1024, iter_values.len());

    // value to expected count
    let expected = vec![
        (1, 1),
        (2, 1),
        (1024, 1),
        (2048 + 1, 1),
        (4096 + 3, 1),
        (8192 - 1, 1)];

    let nonzero_count = iter_values.iter()
            .filter(|v| v.1 != 0)
            .map(|&v| v)
            .collect::<Vec<(u64, u64)>>();

    assert_eq!(expected, nonzero_count);
}

#[test]
fn iter_all_values_all_buckets_unit_magnitude_2() {
    let mut h = histo64(4, 16384 - 1, 3);

    h.record(4).unwrap();
    // first in top half
    h.record(4096).unwrap();
    // first in second bucket
    h.record(8192).unwrap();
    // smallest value in last sub bucket of second
    h.record(16384 - 8).unwrap();

    let iter_values: Vec<(u64, u64)> = h.iter_all()
            .map(|v| (v.value_iterated_to(), v.count_at_value()))
            .collect();

    // magnitude 2 means 2nd bucket is scale of 8 = 2 * 2^2
    assert_eq!(2048 + 1024, iter_values.len());

    // value to expected count
    let expected = vec![
        (4 + 3, 1),
        (4096 + 3, 1),
        (8192 + 7, 1),
        (16384 - 1, 1)];

    let nonzero_count = iter_values.iter()
            .filter(|v| v.1 != 0)
            .map(|&v| v)
            .collect::<Vec<(u64, u64)>>();

    assert_eq!(expected, nonzero_count);
}


#[test]
fn iter_recorded_values_all_buckets() {
    let mut h = histo64(1, 8191, 3);

    h.record(1).unwrap();
    h.record(2).unwrap();
    // first in top half
    h.record(1024).unwrap();
    // first in 2nd bucket
    h.record(2048).unwrap();
    // first in 3rd
    h.record(4096).unwrap();
    // smallest value in last sub bucket of third
    h.record(8192 - 4).unwrap();

    let iter_values: Vec<(u64, u64)> = h.iter_recorded()
            .map(|v| (v.value_iterated_to(), v.count_at_value()))
            .collect();

    let expected = vec![
        (1, 1),
        (2, 1),
        (1024, 1),
        (2048 + 1, 1),
        (4096 + 3, 1),
        (8192 - 1, 1)];

    let nonzero_count = iter_values.iter()
            .filter(|v| v.1 != 0)
            .map(|&v| v)
            .collect::<Vec<(u64, u64)>>();

    assert_eq!(expected, nonzero_count);
}

#[test]
fn iter_recorded_values_all_buckets_unit_magnitude_2() {
    let mut h = histo64(4, 16384 - 1, 3);

    h.record(4).unwrap();
    // first in top half
    h.record(4096).unwrap();
    // first in second bucket
    h.record(8192).unwrap();
    // smallest value in last sub bucket of second
    h.record(16384 - 8).unwrap();

    let iter_values: Vec<(u64, u64)> = h.iter_recorded()
            .map(|v| (v.value_iterated_to(), v.count_at_value()))
            .collect();

    let expected = vec![
        (4 + 3, 1),
        (4096 + 3, 1),
        (8192 + 7, 1),
        (16384 - 1, 1)];

    let nonzero_count = iter_values.iter()
            .filter(|v| v.1 != 0)
            .map(|&v| v)
            .collect::<Vec<(u64, u64)>>();

    assert_eq!(expected, nonzero_count);
}

#[test]
fn iter_logarithmic_bucket_values_min_1_base_2_all_buckets() {
    let h = prepare_histo_for_logarithmic_iterator();

    let iter_values: Vec<(u64, u64, u64)> = h.iter_log(1, 2.0)
            .map(|v| (v.value_iterated_to(), v.count_since_last_iteration(), v.count_at_value()))
            .collect();

    let expected = vec![
        (0, 0, 0),
        (1, 1, 1),
        (3, 1, 0),
        (7, 0, 0),
        (15, 0, 0),
        (31, 3, 1),
        (63, 0, 0),
        (127, 0, 0),
        (255, 0, 0),
        (511, 0, 0),
        (1023, 0, 0),
        (2047, 3, 0),
        (4095, 1, 1)];

    assert_eq!(expected, iter_values);
}

#[test]
fn iter_logarithmic_bucket_values_min_4_base_2_all_buckets() {
    let h = prepare_histo_for_logarithmic_iterator();

    let iter_values: Vec<(u64, u64, u64)> = h.iter_log(4, 2.0)
            .map(|v| (v.value_iterated_to(), v.count_since_last_iteration(), v.count_at_value()))
            .collect();

    let expected = vec![
        (3, 2, 0),
        (7, 0, 0),
        (15, 0, 0),
        (31, 3, 1),
        (63, 0, 0),
        (127, 0, 0),
        (255, 0, 0),
        (511, 0, 0),
        (1023, 0, 0),
        (2047, 3, 0),
        (4095, 1, 1)];

    assert_eq!(expected, iter_values);
}

#[test]
fn iter_logarithmic_bucket_values_min_1_base_2_all_buckets_unit_magnitude_2() {
    // two buckets
    let mut h = histo64(4, 16383, 3);

    h.record(3).unwrap();
    h.record(4).unwrap();

    // inside [2^(4 + 2), 2^(5 + 2)
    h.record(70).unwrap();
    h.record(80).unwrap();
    h.record(90).unwrap();

    // in 2nd half
    h.record(5000).unwrap();
    h.record(5100).unwrap();
    h.record(5200).unwrap();

    // in last sub bucket of 2nd bucket
    h.record(16384 - 1).unwrap();

    let iter_values: Vec<(u64, u64, u64)> = h.iter_log(1, 2.0)
            .map(|v| (v.value_iterated_to(), v.count_since_last_iteration(), v.count_at_value()))
            .collect();

    // first 3 iterations are just getting up to 3, which is still the '0' sub bucket.
    // All at the same index, so count_at_value stays at 1 for the first 3
    let expected = vec![
        (0, 1, 1),
        (1, 0, 1),
        (3, 0, 1),
        (7, 1, 1),
        (15, 0, 0),
        (31, 0, 0),
        (63, 0, 0),
        (127, 3, 0),
        (255, 0, 0),
        (511, 0, 0),
        (1023, 0, 0),
        (2047, 0, 0),
        (4095, 0, 0),
        (8191, 3, 0),
        (16383, 1, 1)];

    assert_eq!(expected, iter_values);
}

#[test]
fn iter_logarithmic_bucket_values_min_1_base_10_all_buckets() {
    let h = prepare_histo_for_logarithmic_iterator();

    let iter_values: Vec<(u64, u64, u64)> = h.iter_log(1, 10.0)
            .map(|v| (v.value_iterated_to(), v.count_since_last_iteration(), v.count_at_value()))
            .collect();

    let expected = vec![
        (0, 0, 0),
        (9, 2, 0),
        (99, 3, 0),
        (999, 0, 0),
        (9999, 4, 1)];

    assert_eq!(expected, iter_values);
}

#[test]
fn iter_linear_bucket_values_size_8_all_buckets() {
    // two buckets: 32 sub-buckets with scale 1, 16 with scale 2
    let mut h = histo64(1, 63, 1);

    assert_eq!(48, h.len());

    h.record(3).unwrap();
    h.record(4).unwrap();
    h.record(7).unwrap();

    // top half of first bucket
    h.record(24).unwrap();
    h.record(25).unwrap();

    h.record(61).unwrap();
    // stored in same sub bucket as last value
    h.record(62).unwrap();
    // in last sub bucket of 2nd bucket
    h.record(63).unwrap();

    let iter_values: Vec<(u64, u64, u64)> = h.iter_linear(8)
            .map(|v| (v.value_iterated_to(), v.count_since_last_iteration(), v.count_at_value()))
            .collect();

    let expected = vec![
        (7, 3, 1),
        (15, 0, 0),
        (23, 0, 0),
        (31, 2, 0),
        (39, 0, 0),
        (47, 0, 0),
        (55, 0, 0),
        (63, 3, 2)];

    assert_eq!(expected, iter_values);
}

#[test]
fn iter_quantiles_smorgasboard() {
    let mut h = histo64(1, 4095, 3);

    // one of each value up to 2 buckets
    for i in 0..4096 {
        h.record(i).unwrap();
    }

    let iter_values: Vec<(u64, u64, u64, f64, f64)> = h.iter_quantiles(2)
            .map(|v| (v.value_iterated_to(),
                      v.count_since_last_iteration(),
                      v.count_at_value(),
                      v.quantile(),
                      v.quantile_iterated_to()))
            .collect();

    // penultimate percentile is 100.0 because 99.96% of 4095 is 4093.36, so it falls into last
    // sub bucket, thus gets to 100.0% of count.
    // this does look like it takes 2 steps and then decreases the step size
    let expected = vec![
        (0, 1, 1, 0.000244140625, 0.0),
        (1023, 1023, 1, 0.25, 0.25),
        (2047, 1024, 1, 0.5, 0.5),
        (2559, 512, 2, 0.625, 0.625),
        (3071, 512, 2, 0.75, 0.75),
        (3327, 256, 2, 0.8125, 0.8125),
        (3583, 256, 2, 0.875, 0.875),
        (3711, 128, 2, 0.90625, 0.90625),
        (3839, 128, 2, 0.9375, 0.9375),
        (3903, 64, 2, 0.953125, 0.953125),
        (3967, 64, 2, 0.96875, 0.96875),
        (3999, 32, 2, 0.9765625, 0.9765625),
        (4031, 32, 2, 0.984375, 0.984375),
        (4047, 16, 2, 0.98828125, 0.98828125),
        (4063, 16, 2, 0.9921875, 0.9921875),
        (4071, 8, 2, 0.994140625, 0.994140625),
        (4079, 8, 2, 0.99609375, 0.99609375),
        (4083, 4, 2, 0.9970703125, 0.9970703125),
        (4087, 4, 2, 0.998046875, 0.998046875),
        (4089, 2, 2, 0.99853515625, 0.99853515625),
        (4091, 2, 2, 0.9990234375, 0.9990234375),
        (4093, 2, 2, 0.99951171875, 0.999267578125),
        (4093, 0, 2, 0.99951171875, 0.99951171875),
        (4095, 2, 2, 1.0, 0.9996337890625),
        (4095, 0, 2, 1.0, 1.0)];

    assert_eq!(expected, iter_values);
}

fn prepare_histo_for_logarithmic_iterator() -> Histogram<u64> {
    // two buckets
    let mut h = histo64(1, 4095, 3);

    h.record(1).unwrap();
    h.record(2).unwrap();

    // inside [2^4, 2^5)
    h.record(20).unwrap();
    h.record(25).unwrap();
    h.record(31).unwrap();

    // in 2nd half
    h.record(1500).unwrap();
    h.record(1600).unwrap();
    h.record(1700).unwrap();

    // in last sub bucket of 2nd bucket
    h.record(4096 - 1).unwrap();

    h
}

fn histo64(lowest_discernible_value: u64, highest_trackable_value: u64, num_significant_digits: u8) -> Histogram<u64> {
    Histogram::<u64>::new_with_bounds(lowest_discernible_value, highest_trackable_value, num_significant_digits).unwrap()
}
