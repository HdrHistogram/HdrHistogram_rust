#![cfg_attr(benchmark, feature(plugin))]

#[cfg(feature = "benchmark")]
extern crate criterion;

#[cfg(feature = "benchmark")]
extern crate hdrsample;

#[cfg(feature = "benchmark")]
extern crate num;

#[cfg(feature = "benchmark")]
mod inner {
    use criterion::Criterion;
    use hdrsample::Histogram;
    use std::time::Duration;

    const TRACKABLE_MAX: i64 = 3600 * 1000 * 1000; // e.g. for 1 hr in usec units
    const SIGFIG: u32 = 3;
    const TEST_VALUE_LEVEL: i64 = 12340;

    pub fn main() {
        let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
        Criterion::default()
            .warm_up_time(Duration::new(2, 0))
            .sample_size(7000)
            .bench_function("raw_record_with_interval_u64", |b| {
                let mut i = 0;
                b.iter(|| {
                    h.record_correct(TEST_VALUE_LEVEL + (i & 0x8000), 1000000000).unwrap();
                    i += 1;
                });
            });

        let mut h = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG).unwrap();
        Criterion::default()
            .warm_up_time(Duration::new(1, 0))
            .sample_size(7000)
            .bench_function("raw_record_u64", |b| {
                let mut i = 0;
                b.iter(|| {
                    h += TEST_VALUE_LEVEL + (i & 0x8000);
                    i += 1;
                });
            });
    }
}

#[cfg(not(feature = "benchmark"))]
mod inner {
    pub fn main() {
        println!("compile with --features benchmark on nightly to run the benchmarks");
    }
}

fn main() {
    inner::main();
}
