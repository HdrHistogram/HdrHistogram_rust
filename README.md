# hdrsample

[![Crates.io](https://img.shields.io/crates/v/hdrsample.svg)](https://crates.io/crates/hdrsample)
[![Documentation](https://docs.rs/hdrsample/badge.svg)](https://docs.rs/hdrsample/)
[![Build Status](https://travis-ci.org/jonhoo/hdrsample.svg?branch=master)](https://travis-ci.org/jonhoo/hdrsample)

HdrSample is a port of Gil Tene's HdrHistogram to native Rust. It provides recording and
analyzing of sampled data value counts across a large, configurable value range with
configurable precision within the range. The resulting "HDR" histogram allows for fast and
accurate analysis of the extreme ranges of data with non-normal distributions, like latency.

## HdrHistogram

What follows is a description from [the HdrHistogram
website](https://hdrhistogram.github.io/HdrHistogram/). Users are encourages to read the
documentation from the original [Java
implementation](https://github.com/HdrHistogram/HdrHistogram), as most of the concepts
translate directly to the Rust port.

HdrHistogram supports the recording and analyzing of sampled data value counts across a
configurable integer value range with configurable value precision within the range. Value
precision is expressed as the number of significant digits in the value recording, and provides
control over value quantization behavior across the value range and the subsequent value
resolution at any given level.

For example, a Histogram could be configured to track the counts of observed integer values
between 0 and 3,600,000,000 while maintaining a value precision of 3 significant digits across
that range. Value quantization within the range will thus be no larger than 1/1,000th (or 0.1%)
of any value. This example Histogram could be used to track and analyze the counts of observed
response times ranging between 1 microsecond and 1 hour in magnitude, while maintaining a value
resolution of 1 microsecond up to 1 millisecond, a resolution of 1 millisecond (or better) up
to one second, and a resolution of 1 second (or better) up to 1,000 seconds. At it's maximum
tracked value (1 hour), it would still maintain a resolution of 3.6 seconds (or better).

HDR Histogram is designed for recoding histograms of value measurements in latency and
performance sensitive applications. Measurements show value recording times as low as 3-6
nanoseconds on modern (circa 2014) Intel CPUs. The HDR Histogram maintains a fixed cost in both
space and time. A Histogram's memory footprint is constant, with no allocation operations
involved in recording data values or in iterating through them. The memory footprint is fixed
regardless of the number of data value samples recorded, and depends solely on the dynamic
range and precision chosen. The amount of work involved in recording a sample is constant, and
directly computes storage index locations such that no iteration or searching is ever involved
in recording data values.

## Interacting with the library

HdrSample's API follows that of the original HdrHistogram Java implementation, with some
modifications to make its use more idiomatic in Rust. The description in this section has been
adapted from that given by the [Python port](https://github.com/HdrHistogram/HdrHistogram_py),
as it gives a nicer first-time introduction to the use of HdrHistogram than the Java docs do.

HdrSample is generally used in one of two modes: recording samples, or querying for analytics.
In distributed deployments, the recording may be performed remotely (and possibly in multiple
locations), to then be aggregated later in a central location for analysis.

### Recording samples

A histogram instance is created using the `::new` methods on the `Histogram` struct. These come
in three variants: `new`, `new_with_max`, and `new_with_bounds`. The first of these only sets
the required precision of the sampled data, but leaves the value range open such that any value
may be recorded. A `Histogram` created this way (or one where auto-resize has been explicitly
enabled) will automatically resize itself if a value that is too large to fit in the current
dataset is encountered. `new_with_max` sets an upper bound on the values to be recorded, and
disables auto-resizing, thus preventing any re-allocation during recording. If the application
attempts to record a larger value than this maximum bound, the record call will fail. Finally,
`new_with_bounds` restricts the lowest representible value of the dataset, such that a smaller
range needs to be covered (thus reducing the overall allocation size).

For example the example below shows how to create a `Histogram` that can count values in the
`[1..3600000]` range with 1% precision, which could be used to track latencies in the range `[1
msec..1 hour]`).

```rust
use hdrsample::Histogram;
let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 60 * 1000, 2).unwrap();

// samples can be recorded using .record, which will error if the value is too small or large
hist.record(54321).expect("value 54321 should be in range");

// for ergonomics, samples can also be recorded with +=
// this call will panic if the value is out of range!
hist += 54321;

// if the code that generates the values is subject to Coordinated Omission,
// the self-correcting record method should be used instead.
// for example, if the expected sampling interval is 10 msec:
hist.record_correct(54321, 10).expect("value 54321 should be in range");
```

Note the `u64` annotation. This type can be changed to reduce the storage overhead for all the
histogram bins, at the cost of a risk of overflowing if a large number of samples end up in the
same bin.

### Querying samples

At any time, the histogram can be queried to return interesting statistical measurements, such
as the total number of recorded samples, or the value at a given percentile:

```rust
use hdrsample::Histogram;
let hist = Histogram::<u64>::new(2).unwrap();
// ...
println!("# of samples: {}", hist.count());
println!("99.9'th percentile: {}", hist.value_at_percentile(99.9));
```

Several useful iterators are also provided for quickly getting an overview of the dataset. The
simplest one is `iter_recorded()`, which yields one item for every non-empty sample bin. All
the HdrHistogram iterators are supported in HdrSample, so look for the `*Iterator` classes in
the [Java documentation](https://hdrhistogram.github.io/HdrHistogram/JavaDoc/).

```rust
use hdrsample::Histogram;
let hist = Histogram::<u64>::new(2).unwrap();
// ...
for v in hist.iter_recorded() {
    println!("{}'th percentile of data is {} with {} samples",
        v.percentile(), v.value(), v.count_at_value());
}
```

## Limitations and Caveats

As with all the other HdrHistogram ports, the latest features and bug fixes from the upstream
HdrHistogram implementations may not be available in this port. A number of features have also
not (yet) been implemented:

 - Concurrency support (`AtomicHistogram`, `ConcurrentHistogram`, …).
 - `DoubleHistogram`. You can use `f64` as the counter type, but none of the "special"
   `DoubleHistogram` features are supported.
 - The `Recorder` feature of HdrHistogram.
 - Value shifting ("normalization").
 - Histogram serialization and encoding/decoding.
 - Timestamps and tags.
 - Textual output methods. These seem almost orthogonal to HdrSample, though it might be
   convenient if we implemented some relevant traits (CSV, JSON, and possibly simple
   `fmt::Display`).

Most of these should be fairly straightforward to add, as the code aligns pretty well with the
original Java/C# code. If you do decide to implement one and send a PR, please make sure you
also port the [test
cases](https://github.com/HdrHistogram/HdrHistogram/tree/master/src/test/java/org/HdrHistogram),
and try to make sure you implement appropriate traits to make the use of the feature as
ergonomic as possible.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
hdrsample = "3.0"
```

and this to your crate root:

```rust
extern crate hdrsample;
```

## License

Dual-licensed to be compatible with the Rust project.

Licensed under the Apache License, Version 2.0
http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
http://opensource.org/licenses/MIT, at your option. This file may not be
copied, modified, or distributed except according to those terms.
