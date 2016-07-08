extern crate hdrsample;

use hdrsample::Histogram;

#[allow(dead_code)]
struct Loaded {
    raw: Histogram<u64>,
    cor: Histogram<u64>,
    scaled_raw: Histogram<u64>,
    scaled_cor: Histogram<u64>,
}

fn load_histograms() -> Loaded {
    let highest_trackable = 3600 * 1000 * 1000;
    let sigfig = 3;
    let interval = 10000;
    let scale = 512;
    let scaled_interval = interval * scale;

    let mut raw = Histogram::new_with_max(highest_trackable, sigfig).unwrap();
    let mut cor = Histogram::new_with_max(highest_trackable, sigfig).unwrap();
    let mut scaled_raw = Histogram::new_with_bounds(1000, highest_trackable * scale, sigfig)
        .unwrap();
    let mut scaled_cor = Histogram::new_with_bounds(1000, highest_trackable * scale, sigfig)
        .unwrap();

    for _ in 0..10000 {
        raw += 1000;
        cor.recordInInterval(1000, interval).unwrap();

        scaled_raw += 1000 * scale;
        scaled_cor.recordInInterval(1000 * scale, scaled_interval).unwrap();
    }

    raw += 100000000;
    cor.recordInInterval(100000000, interval).unwrap();

    scaled_raw += 100000000 * scale;
    scaled_cor.recordInInterval(100000000 * scale, scaled_interval).unwrap();

    Loaded {
        raw: raw,
        cor: cor,
        scaled_raw: scaled_raw,
        scaled_cor: scaled_cor,
    }
}

pub fn compare_double(a: f64, b: f64, delta: f64) -> bool {
    (a - b).abs() < delta
}
pub fn compare_values(a: f64, b: f64, variation: f64) -> bool {
    compare_double(a, b, b * variation)
}
pub fn compare_percentile(a: i64, b: f64, variation: f64) -> bool {
    compare_values(a as f64, b, variation)
}

#[test]
fn test_create() {
    let h = Histogram::<u64>::new_with_max(3600000000, 3).unwrap();

    assert_eq!(h.len(), 23552);
}

#[test]
fn test_invalid_init() {
    assert!(Histogram::<u64>::new_with_bounds(0, 6481024, 2).is_err());
    assert!(Histogram::<u64>::new_with_bounds(80, 110, 5).is_err());
}

#[test]
fn test_invalid_sigfig() {
    assert!(Histogram::<u64>::new_with_max(3600000000, 6).is_err());
}


#[test]
fn test_total_count() {
    let Loaded { raw, cor, .. } = load_histograms();

    assert_eq!(raw.total(), 10001);
    assert_eq!(cor.total(), 20000);
}

#[test]
fn test_get_max_value() {
    let Loaded { raw, cor, .. } = load_histograms();

    assert!(raw.equivalent(raw.max(), 100000000));
    assert!(cor.equivalent(cor.max(), 100000000));
}

#[test]
fn test_get_min_value() {
    let Loaded { raw, cor, .. } = load_histograms();

    assert_eq!(raw.min(), 1000);
    assert_eq!(cor.min(), 1000);
}

#[test]
fn test_percentiles() {
    let Loaded { raw, cor, .. } = load_histograms();

    assert!(compare_percentile(raw.value_at_percentile(30.0), 1000.0, 0.001));
    assert!(compare_percentile(raw.value_at_percentile(99.0), 1000.0, 0.001));
    assert!(compare_percentile(raw.value_at_percentile(99.99), 1000.0, 0.001));
    assert!(compare_percentile(raw.value_at_percentile(99.999), 100000000.0, 0.001));
    assert!(compare_percentile(raw.value_at_percentile(100.0), 100000000.0, 0.001));

    assert!(compare_percentile(cor.value_at_percentile(30.0), 1000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(50.0), 1000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(75.0), 50000000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(90.0), 80000000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(99.0), 98000000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(99.999), 100000000.0, 0.001));
    assert!(compare_percentile(cor.value_at_percentile(100.0), 100000000.0, 0.001));
}

#[test]
fn test_recorded_values() {
    let Loaded { raw, cor, .. } = load_histograms();

    let mut last = 0;
    for (idx, item) in raw.iter_recorded().enumerate() {
        let count_added = item.3;

        last = idx;

        if idx == 0 {
            assert_eq!(count_added, 10000)
        } else {
            assert_eq!(count_added, 1)
        }
    }
    assert_eq!(last + 1, 2);

    let mut total_added_count = 0;
    for (idx, item) in cor.iter_recorded().enumerate() {
        let count_added = item.3;

        if idx == 0 {
            assert_eq!(count_added, 10000)
        }
        assert!(count_added != 0);
        total_added_count += count_added;
    }
    assert_eq!(total_added_count, 20000);
}

#[test]
fn test_linear_values() {
    let Loaded { raw, cor, .. } = load_histograms();
    let mut last = 0;

    for (idx, item) in raw.iter_linear(100000).enumerate() {
        let count_added = item.3;

        last = idx;

        if idx == 0 {
            assert_eq!(count_added, 10000)
        } else if idx == 999 {
            assert_eq!(count_added, 1)
        } else {
            assert_eq!(count_added, 0)
        }
    }
    assert_eq!(last + 1, 1000);

    let mut total_added_count = 0;
    let mut last = 0;
    for (idx, item) in cor.iter_linear(10000).enumerate() {
        let count_added = item.3;

        if idx == 0 {
            assert_eq!(count_added, 10001)
        }

        total_added_count += count_added;
        last = idx;
    }
    assert_eq!(last + 1, 10000);
    assert_eq!(total_added_count, 20000);
}

#[test]
fn test_logarithmic_values() {
    let Loaded { raw, cor, .. } = load_histograms();

    let mut last = 0;
    for (idx, item) in raw.iter_log(10000, 2.0).enumerate() {
        let count_added = item.3;

        if idx == 0 {
            assert_eq!(count_added, 10000)
        } else if idx == 14 {
            assert_eq!(count_added, 1)
        } else {
            assert_eq!(count_added, 0)
        }

        last = idx;
    }
    assert_eq!(last, 14);

    let mut last = 0;
    let mut total_added_count = 0;
    for (idx, item) in cor.iter_log(10000, 2.0).enumerate() {
        let count_added = item.3;

        if idx == 0 {
            assert_eq!(count_added, 10001)
        }

        total_added_count += count_added;
        last = idx;
    }
    assert_eq!(last, 14);
    assert_eq!(total_added_count, 20000);
}

#[test]
fn test_reset() {
    let Loaded { mut raw, mut cor, .. } = load_histograms();

    assert!(raw.value_at_percentile(99.0) != 0);
    assert!(cor.value_at_percentile(99.0) != 0);

    raw.reset();
    cor.reset();

    assert_eq!(raw.total(), 0);
    assert_eq!(cor.total(), 0);
    assert_eq!(raw.value_at_percentile(99.0), 0);
    assert_eq!(cor.value_at_percentile(99.0), 0);
}

#[test]
fn test_scaling_equivalence() {
    let Loaded { cor, scaled_cor, .. } = load_histograms();

    assert!(compare_values(cor.mean() * 512.0, scaled_cor.mean(), 0.000001));
    assert_eq!(cor.total(), scaled_cor.total());

    let expected_99th = cor.value_at_percentile(99.0) * 512;
    let scaled_99th = scaled_cor.value_at_percentile(99.0);

    assert_eq!(cor.lowest_equivalent(expected_99th),
               scaled_cor.lowest_equivalent(scaled_99th));
}

#[test]
fn test_out_of_range_values() {
    let mut h = Histogram::<u64>::new_with_max(1000, 4).unwrap();

    assert!(h.record(32767).is_ok());
    assert!(h.record(32768).is_err());
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
fn test_clone() {
    let mut h = Histogram::<u64>::new_with_bounds(20000000, 100000000, 5).unwrap();

    h += 100000000;
    h += 20000000;
    h += 30000000;

    assert_eq!(h.total(), 3);

    let b = h.clone();

    assert_eq!(h.total(), b.total());
    assert_eq!(h.count_at(100000000), b.count_at(100000000));
}
