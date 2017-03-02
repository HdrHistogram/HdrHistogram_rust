//! HdrSample is a port of Gil Tene's HdrHistogram to native Rust. It provides recording and
//! analyzing of sampled data value counts across a large, configurable value range with
//! configurable precision within the range. The resulting "HDR" histogram allows for fast and
//! accurate analysis of the extreme ranges of data with non-normal distributions, like latency.
//!
//! # HdrHistogram
//!
//! What follows is a description from [the HdrHistogram
//! website](https://hdrhistogram.github.io/HdrHistogram/). Users are encourages to read the
//! documentation from the original [Java
//! implementation](https://github.com/HdrHistogram/HdrHistogram), as most of the concepts
//! translate directly to the Rust port.
//!
//! HdrHistogram supports the recording and analyzing of sampled data value counts across a
//! configurable integer value range with configurable value precision within the range. Value
//! precision is expressed as the number of significant digits in the value recording, and provides
//! control over value quantization behavior across the value range and the subsequent value
//! resolution at any given level.
//!
//! For example, a Histogram could be configured to track the counts of observed integer values
//! between 0 and 3,600,000,000 while maintaining a value precision of 3 significant digits across
//! that range. Value quantization within the range will thus be no larger than 1/1,000th (or 0.1%)
//! of any value. This example Histogram could be used to track and analyze the counts of observed
//! response times ranging between 1 microsecond and 1 hour in magnitude, while maintaining a value
//! resolution of 1 microsecond up to 1 millisecond, a resolution of 1 millisecond (or better) up
//! to one second, and a resolution of 1 second (or better) up to 1,000 seconds. At it's maximum
//! tracked value (1 hour), it would still maintain a resolution of 3.6 seconds (or better).
//!
//! HDR Histogram is designed for recoding histograms of value measurements in latency and
//! performance sensitive applications. Measurements show value recording times as low as 3-6
//! nanoseconds on modern (circa 2014) Intel CPUs. The HDR Histogram maintains a fixed cost in both
//! space and time. A Histogram's memory footprint is constant, with no allocation operations
//! involved in recording data values or in iterating through them. The memory footprint is fixed
//! regardless of the number of data value samples recorded, and depends solely on the dynamic
//! range and precision chosen. The amount of work involved in recording a sample is constant, and
//! directly computes storage index locations such that no iteration or searching is ever involved
//! in recording data values.
//!
//! # Interacting with the library
//!
//! HdrSample's API follows that of the original HdrHistogram Java implementation, with some
//! modifications to make its use more idiomatic in Rust. The description in this section has been
//! adapted from that given by the [Python port](https://github.com/HdrHistogram/HdrHistogram_py),
//! as it gives a nicer first-time introduction to the use of HdrHistogram than the Java docs do.
//!
//! HdrSample is generally used in one of two modes: recording samples, or querying for analytics.
//! In distributed deployments, the recording may be performed remotely (and possibly in multiple
//! locations), to then be aggregated later in a central location for analysis.
//!
//! ## Recording samples
//!
//! A histogram instance is created using the `::new` methods on the `Histogram` struct. These come
//! in three variants: `new`, `new_with_max`, and `new_with_bounds`. The first of these only sets
//! the required precision of the sampled data, but leaves the value range open such that any value
//! may be recorded. A `Histogram` created this way (or one where auto-resize has been explicitly
//! enabled) will automatically resize itself if a value that is too large to fit in the current
//! dataset is encountered. `new_with_max` sets an upper bound on the values to be recorded, and
//! disables auto-resizing, thus preventing any re-allocation during recording. If the application
//! attempts to record a larger value than this maximum bound, the record call will fail. Finally,
//! `new_with_bounds` restricts the lowest representible value of the dataset, such that a smaller
//! range needs to be covered (thus reducing the overall allocation size).
//!
//! For example the example below shows how to create a `Histogram` that can count values in the
//! `[1..3600000]` range with 1% precision, which could be used to track latencies in the range `[1
//! msec..1 hour]`).
//!
//! ```
//! use hdrsample::Histogram;
//! let mut hist = Histogram::<u64>::new_with_bounds(1, 60 * 60 * 1000, 2).unwrap();
//!
//! // samples can be recorded using .record, which will error if the value is too small or large
//! hist.record(54321).expect("value 54321 should be in range");
//!
//! // for ergonomics, samples can also be recorded with +=
//! // this call will panic if the value is out of range!
//! hist += 54321;
//!
//! // if the code that generates the values is subject to Coordinated Omission,
//! // the self-correcting record method should be used instead.
//! // for example, if the expected sampling interval is 10 msec:
//! hist.record_correct(54321, 10).expect("value 54321 should be in range");
//! ```
//!
//! Note the `u64` annotation. This type can be changed to reduce the storage overhead for all the
//! histogram bins, at the cost of a risk of overflowing if a large number of samples end up in the
//! same bin.
//!
//! ## Querying samples
//!
//! At any time, the histogram can be queried to return interesting statistical measurements, such
//! as the total number of recorded samples, or the value at a given percentile:
//!
//! ```
//! use hdrsample::Histogram;
//! let hist = Histogram::<u64>::new(2).unwrap();
//! // ...
//! println!("# of samples: {}", hist.count());
//! println!("99.9'th percentile: {}", hist.value_at_percentile(99.9));
//! ```
//!
//! Several useful iterators are also provided for quickly getting an overview of the dataset. The
//! simplest one is `iter_recorded()`, which yields one item for every non-empty sample bin. All
//! the HdrHistogram iterators are supported in HdrSample, so look for the `*Iterator` classes in
//! the [Java documentation](https://hdrhistogram.github.io/HdrHistogram/JavaDoc/).
//!
//! ```
//! use hdrsample::Histogram;
//! let hist = Histogram::<u64>::new(2).unwrap();
//! // ...
//! for v in hist.iter_recorded() {
//!     println!("{}'th percentile of data is {} with {} samples",
//!         v.percentile(), v.value(), v.count_at_value());
//! }
//! ```
//!
//! # Limitations and Caveats
//!
//! As with all the other HdrHistogram ports, the latest features and bug fixes from the upstream
//! HdrHistogram implementations may not be available in this port. A number of features have also
//! not (yet) been implemented:
//!
//!  - Concurrency support (`AtomicHistogram`, `ConcurrentHistogram`, …).
//!  - `DoubleHistogram`. You can use `f64` as the counter type, but none of the "special"
//!    `DoubleHistogram` features are supported.
//!  - The `Recorder` feature of HdrHistogram.
//!  - Value shifting ("normalization").
//!  - Histogram serialization and encoding/decoding.
//!  - Timestamps and tags.
//!  - Textual output methods. These seem almost orthogonal to HdrSample, though it might be
//!    convenient if we implemented some relevant traits (CSV, JSON, and possibly simple
//!    `fmt::Display`).
//!
//! Most of these should be fairly straightforward to add, as the code aligns pretty well with the
//! original Java/C# code. If you do decide to implement one and send a PR, please make sure you
//! also port the [test
//! cases](https://github.com/HdrHistogram/HdrHistogram/tree/master/src/test/java/org/HdrHistogram),
//! and try to make sure you implement appropriate traits to make the use of the feature as
//! ergonomic as possible.

#![deny(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_results,
    variant_size_differences,
    warnings
)]

extern crate num;

#[cfg(feature = "rustc-serialize")]
extern crate rustc_serialize;

#[cfg(feature = "serde-serialize")]
#[macro_use]
extern crate serde_derive;

use std::borrow::Borrow;
use std::cmp;
use std::ops::{Index, IndexMut, AddAssign, SubAssign};

use iterators::HistogramIterator;

/// This auto-implemented marker trait represents the operations a histogram must be able to
/// perform on the underlying counter type. The `ToPrimitive` trait is needed to perform floating
/// point operations on the counts (usually for percentiles). The `FromPrimitive` to convert back
/// into an integer count. Partial ordering is used for threshholding, also usually in the context
/// of percentiles.
pub trait Counter
    : num::Num + num::ToPrimitive + num::FromPrimitive + Copy + PartialOrd<Self> {
}

// auto-implement marker trait
impl<T> Counter for T
    where T: num::Num + num::ToPrimitive + num::FromPrimitive + Copy + PartialOrd<T>
{
}

/// `Histogram` is the core data structure in HdrSample. It records values, and performs analytics.
///
/// At its heart, it keeps the count for recorded samples in "buckets" of values. The resolution
/// and distribution of these buckets is tuned based on the desired highest trackable value, as
/// well as the user-specified number of significant decimal digits to preserve. The values for the
/// buckets are kept in a way that resembles floats and doubles: there is a mantissa and an
/// exponent, and each bucket represents a different exponent. The "sub-buckets" within a bucket
/// represent different values for the mantissa.
///
/// To a first approximation, the sub-buckets of the first
/// bucket would hold the values `0`, `1`, `2`, `3`, …, the sub-buckets of the second bucket would
/// hold `0`, `2`, `4`, `6`, …, the third would hold `0`, `4`, `8`, and so on. However, the low
/// half of each bucket (except bucket 0) is unnecessary, since those values are already covered by
/// the sub-buckets of all the preceeding buckets. Thus, `Histogram` keeps the top half of every
/// such bucket.
///
/// For the purposes of explanation, consider a `Histogram` with 2048 sub-buckets for every bucket,
/// and a lowest discernible value of 1:
///
/// <pre>
/// The 0th bucket covers 0...2047 in multiples of 1, using all 2048 sub-buckets
/// The 1st bucket covers 2048..4097 in multiples of 2, using only the top 1024 sub-buckets
/// The 2nd bucket covers 4096..8191 in multiple of 4, using only the top 1024 sub-buckets
/// ...
/// </pre>
///
/// Bucket 0 is "special" here. It is the only one that has 2048 entries. All the rest have
/// 1024 entries (because their bottom half overlaps with and is already covered by the all of
/// the previous buckets put together). In other words, the `k`'th bucket could represent `0 *
/// 2^k` to `2048 * 2^k` in 2048 buckets with `2^k` precision, but the midpoint of `1024 * 2^k
/// = 2048 * 2^(k-1)`, which is the k-1'th bucket's end. So, we would use the previous bucket
/// for those lower values as it has better precision.
///
#[cfg_attr(feature = "rustc-serialize",  derive(RustcEncodable, RustcDecodable))]
#[cfg_attr(feature = "serde-serialize",  derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct Histogram<T: Counter> {
    auto_resize: bool,

    // >= 2 * lowest_discernible_value
    highest_trackable_value: u64,
    // >= 1
    lowest_discernible_value: u64,
    // in [0, 5]
    significant_value_digits: u8,

    // in [1, 64]
    bucket_count: u8,
    // 2^(sub_bucket_half_count_magnitude + 1) = [2, 2^18]
    sub_bucket_count: u32,
    // sub_bucket_count / 2 = [1, 2^17]
    sub_bucket_half_count: u32,
    // In [0, 17]
    sub_bucket_half_count_magnitude: u8,
    sub_bucket_mask: u64,

    // in [1, 63]
    leading_zero_count_base: u8,

    // Largest exponent of 2 that's smaller than the lowest discernible value. In [0, 62].
    unit_magnitude: u8,
    // low unit_magnitude bits set
    unit_magnitude_mask: u64,

    max_value: u64,
    min_non_zero_value: u64,

    total_count: u64,
    counts: Vec<T>,
}

/// Errors that can occur when creating a histogram.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum CreationError {
    /// Lowest discernible value must be >= 1.
    LowIsZero,
    /// Lowest discernible value must be <= `u64::max_value() / 2` because the highest value is
    /// a `u64` and the lowest value must be no bigger than half the highest.
    LowExceedsMax,
    /// Highest trackable value must be >= 2 * lowest discernible value for some internal
    /// calculations to work out. In practice, high is typically much higher than 2 * low.
    HighLessThanTwiceLow,
    /// Number of significant digits must be in the range `[0, 5]`. It is capped at 5 because 5
    /// significant digits is already more than almost anyone needs, and memory usage scales
    /// exponentially as this increases.
    SigFigExceedsMax,
    /// Cannot represent sigfig worth of values beyond the lowest discernible value. Decrease the
    /// significant figures, lowest discernible value, or both.
    ///
    /// This could happen if low is very large (like 2^60) and sigfigs is 5, which requires 18
    /// additional bits, which would then require more bits than will fit in a u64. Specifically,
    /// the exponent of the largest power of two that is smaller than the lowest value and the bits
    /// needed to represent the requested significant figures must sum to 63 or less.
    CannotRepresentSigFigBeyondLow
}

/// Errors that can occur when adding another histogram.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum AdditionError {
    /// The other histogram includes values that do not fit in this histogram's range.
    /// Only possible when auto resize is disabled.
    OtherAddendValuesExceedRange
}

/// Errors that can occur when subtracting another histogram.
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum SubtractionError {
    /// The other histogram includes values that do not fit in this histogram's range.
    /// Only possible when auto resize is disabled.
    SubtrahendValuesExceedMinuendRange,
    /// The other histogram includes counts that are higher than the current count for a value, and
    /// counts cannot go negative. The subtraction may have been partially applied to some counts as
    /// this error is returned when the first impossible subtraction is detected.
    SubtrahendCountExceedsMinuendCount
}


/// Module containing the implementations of all `Histogram` iterators.
pub mod iterators;

impl<T: Counter> Histogram<T> {
    // ********************************************************************************************
    // Histogram administrative read-outs
    // ********************************************************************************************

    /// Get the current number of distinct counted values in the histogram.
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /// Get the lowest discernible value for the histogram in its current configuration.
    pub fn low(&self) -> u64 {
        self.lowest_discernible_value
    }

    /// Get the highest trackable value for the histogram in its current configuration.
    pub fn high(&self) -> u64 {
        self.highest_trackable_value
    }

    /// Get the number of significant value digits kept by this histogram.
    pub fn sigfig(&self) -> u8 {
        self.significant_value_digits
    }

    /// Get the total number of samples recorded.
    pub fn count(&self) -> u64 {
        self.total_count
    }

    /// Get the index of the last histogram bin.
    pub fn last(&self) -> usize {
        self.len() - 1
    }

    /// Get the number of buckets used by the histogram to cover the highest trackable value.
    ///
    /// This method differs from `.len()` in that it does not count the sub buckets within each
    /// bucket.
    ///
    /// This method is probably only useful for testing purposes.
    pub fn buckets(&self) -> u8 {
        self.bucket_count
    }

    // ********************************************************************************************
    // Methods for looking up the count for a given value/index
    // ********************************************************************************************

    /// Find the bucket the given value should be placed in.
    fn index_for(&self, value: u64) -> usize {
        let bucket_index = self.bucket_for(value);
        let sub_bucket_index = self.sub_bucket_for(value, bucket_index);

        debug_assert!(sub_bucket_index < self.sub_bucket_count);
        debug_assert!(bucket_index == 0 || (sub_bucket_index >= self.sub_bucket_half_count));

        // Calculate the index for the first entry that will be used in the bucket (halfway through
        // sub_bucket_count). For bucket_index 0, all sub_bucket_count entries may be used, but
        // bucket_base_index is still set in the middle.
        let bucket_base_index = (bucket_index as usize + 1) << self.sub_bucket_half_count_magnitude;

        // Calculate the offset in the bucket. This subtraction will result in a positive value in
        // all buckets except the 0th bucket (since a value in that bucket may be less than half
        // the bucket's 0 to sub_bucket_count range). However, this works out since we give bucket 0
        // twice as much space.
        let offset_in_bucket = sub_bucket_index as isize - self.sub_bucket_half_count as isize;

        let index = bucket_base_index as isize + offset_in_bucket;
        // This is always non-negative because offset_in_bucket is only negative (and only then by
        // sub_bucket_half_count at most) for bucket 0, and bucket_base_index will be halfway into
        // bucket 0's sub buckets in that case.
        debug_assert!(index >= 0);
        return index as usize;
    }

    /// Get a mutable reference to the count bucket for the given value, if it is in range.
    fn mut_at(&mut self, value: u64) -> Option<&mut T> {
        let i = self.index_for(value);
        self.counts.get_mut(i)
    }

    // ********************************************************************************************
    // Histograms should be cloneable.
    // ********************************************************************************************

    /// Get a copy of this histogram, corrected for coordinated omission.
    ///
    /// To compensate for the loss of sampled values when a recorded value is larger than the
    /// expected interval between value samples, the new histogram will include an auto-generated
    /// additional series of decreasingly-smaller (down to the `interval`) value records for each
    /// count found in the current histogram that is larger than the `interval`.
    ///
    /// Note: This is a post-correction method, as opposed to the at-recording correction method
    /// provided by `record_correct`. The two methods are mutually exclusive, and only one of the
    /// two should be be used on a given data set to correct for the same coordinated omission
    /// issue.
    ///
    /// See notes in the description of the Histogram calls for an illustration of why this
    /// corrective behavior is important.
    ///
    /// If `interval` is larger than 0, add auto-generated value records as appropriate if value is
    /// larger than `interval`.
    pub fn clone_correct(&self, interval: u64) -> Histogram<T> {
        let mut h = Histogram::new_from(self);
        for v in self.iter_recorded() {
            h.record_n_correct(v.value(), v.count_at_value(), interval).unwrap();
        }
        h
    }

    /// Overwrite this histogram with the given histogram. All data and statistics in this
    /// histogram will be overwritten.
    pub fn set_to<B: Borrow<Histogram<T>>>(&mut self, source: B) -> Result<(), AdditionError> {
        self.reset();
        self.add(source.borrow())
    }

    /// Overwrite this histogram with the given histogram while correcting for coordinated
    /// omission. All data and statistics in this histogram will be overwritten. See
    /// `clone_correct` for more detailed explanation about how correction is applied
    pub fn set_to_corrected<B: Borrow<Histogram<T>>>(&mut self,
                                                     source: B,
                                                     interval: u64)
                                                     -> Result<(), ()> {
        self.reset();
        self.add_correct(source, interval)
    }

    // ********************************************************************************************
    // Add and subtract methods for, well, adding or subtracting two histograms
    // ********************************************************************************************

    /// Add the contents of another histogram to this one.
    ///
    /// May fail if values in the other histogram are higher than `.high()`, and auto-resize is
    /// disabled.
    pub fn add<B: Borrow<Histogram<T>>>(&mut self, source: B) -> Result<(), AdditionError> {
        let source = source.borrow();

        // make sure we can take the values in source
        let top = self.highest_equivalent(self.value_for(self.last()));
        if top < source.max() {
            if !self.auto_resize {
                return Err(AdditionError::OtherAddendValuesExceedRange);
            }
            self.resize(source.max());
        }

        if self.bucket_count == source.bucket_count && self.sub_bucket_count == source.sub_bucket_count &&
           self.unit_magnitude == source.unit_magnitude {
            // Counts arrays are of the same length and meaning,
            // so we can just iterate and add directly:
            let mut observed_other_total_count: u64 = 0;
            for i in 0..source.len() {
                let other_count = source[i];
                if other_count != T::zero() {
                    self[i] = self[i] + other_count;
                    observed_other_total_count = observed_other_total_count + other_count.to_u64().unwrap();
                }
            }

            self.total_count = self.total_count + observed_other_total_count;
            let mx = source.max();
            if mx > self.max() {
                self.update_max(mx);
            }
            let mn = source.min_nz();
            if mn < self.min_nz() {
                self.update_min(mn);
            }
        } else {
            // Arrays are not a direct match (or the other could change on the fly in some valid
            // way), so we can't just stream through and add them. Instead, go through the array
            // and add each non-zero value found at it's proper value:

            // Do max value first, to avoid max value updates on each iteration:
            let other_max_index = source.index_for(source.max());
            let other_count = source[other_max_index];
            self.record_n(source.value_for(other_max_index), other_count).unwrap();

            // Record the remaining values, up to but not including the max value:
            for i in 0..other_max_index {
                let other_count = source[i];
                if other_count != T::zero() {
                    self.record_n(source.value_for(i), other_count).unwrap();
                }
            }
        }

        // TODO:
        // if source.start_time < self.start_time {
        //     self.start_time = source.start_time;
        // }
        // if source.end_time > self.end_time {
        //     self.end_time = source.end_time;
        // }
        Ok(())
    }

    /// Add the contents of another histogram to this one, while correcting for coordinated
    /// omission.
    ///
    /// To compensate for the loss of sampled values when a recorded value is larger than the
    /// expected interval between value samples, the values added will include an auto-generated
    /// additional series of decreasingly-smaller (down to the given `interval`) value records for
    /// each count found in the current histogram that is larger than `interval`.
    ///
    /// Note: This is a post-recording correction method, as opposed to the at-recording correction
    /// method provided by `record_correct`. The two methods are mutually exclusive, and only one
    /// of the two should be be used on a given data set to correct for the same coordinated
    /// omission issue.
    ///
    /// See notes in the description of the `Histogram` calls for an illustration of why this
    /// corrective behavior is important.
    ///
    /// May fail if values in the other histogram are higher than `.high()`, and auto-resize is
    /// disabled.
    pub fn add_correct<B: Borrow<Histogram<T>>>(&mut self,
                                                source: B,
                                                interval: u64)
                                                -> Result<(), ()> {
        let source = source.borrow();

        for v in source.iter_recorded() {
            try!(self.record_n_correct(v.value(), v.count_at_value(), interval));
        }
        Ok(())
    }

    /// Subtract the contents of another histogram from this one.
    ///
    /// May fail if values in the other histogram are higher than `.high()`, and auto-resize is
    /// disabled. Or, if the count for a given value in the other histogram is higher than that of
    /// this histogram. In the latter case, some of the counts may still have been updated, which
    /// may cause data corruption.
    pub fn subtract<B: Borrow<Histogram<T>>>(&mut self, other: B) -> Result<(), SubtractionError> {
        let other = other.borrow();

        // make sure we can take the values in source
        let top = self.highest_equivalent(self.value_for(self.last()));
        if top < other.max() {
            if !self.auto_resize {
                return Err(SubtractionError::SubtrahendValuesExceedMinuendRange);
            }
            self.resize(other.max());
        }

        for i in 0..other.len() {
            let other_count = other[i];
            if other_count != T::zero() {
                let other_value = other.value_for(i);
                if self.count_at(other_value).unwrap() < other_count {
                    return Err(SubtractionError::SubtrahendCountExceedsMinuendCount);
                }
                self.alter_n(other_value, other_count, false).expect("value should fit by now");
            }
        }

        // With subtraction, the max and min_non_zero values could have changed:
        if self.count_at(self.max()).unwrap() == T::zero() ||
           self.count_at(self.min_nz()).unwrap() == T::zero() {
            let l = self.len();
            self.restat(l);
        }

        Ok(())
    }

    // ********************************************************************************************
    // Setters and resetters.
    // ********************************************************************************************

    /// Clear the contents of this histogram while preserving its statistics and configuration.
    pub fn clear(&mut self) {
        for c in self.counts.iter_mut() {
            *c = T::zero();
        }
        self.total_count = 0;
    }

    /// Reset the contents and statistics of this histogram, preserving only its configuration.
    pub fn reset(&mut self) {
        self.clear();

        self.reset_max(0);
        self.reset_min(u64::max_value());
        // self.normalizing_index_offset = 0;
        // self.start_time = time::Instant::now();
        // self.end_time = time::Instant::now();
        // self.tag = String::new();
    }

    /// Control whether or not the histogram can auto-resize and auto-adjust it's highest trackable
    /// value as high-valued samples are recorded.
    pub fn auto(&mut self, enabled: bool) {
        self.auto_resize = enabled;
    }

    // ********************************************************************************************
    // Construction.
    // ********************************************************************************************

    /// Construct an auto-resizing `Histogram` with a lowest discernible value of 1 and an
    /// auto-adjusting highest trackable value. Can auto-resize up to track values up to
    /// `(i64::max_value() / 2)`.
    ///
    /// See [`new_with_bounds`] for info on `sigfig`.
    ///
    /// [`new_with_bounds`]: #method.new_with_bounds
    pub fn new(sigfig: u8) -> Result<Histogram<T>, CreationError> {
        let mut h = Self::new_with_bounds(1, 2, sigfig);
        if let Ok(ref mut h) = h {
            h.auto_resize = true;
        }
        h
    }

    /// Construct a `Histogram` given a known maximum value to be tracked, and a number of
    /// significant decimal digits. The histogram will be constructed to implicitly track
    /// (distinguish from 0) values as low as 1. Auto-resizing will be disabled.
    ///
    /// See [`new_with_bounds`] for info on `high` and `sigfig`.
    ///
    /// [`new_with_bounds`]: #method.new_with_bounds
    pub fn new_with_max(high: u64, sigfig: u8) -> Result<Histogram<T>, CreationError> {
        Self::new_with_bounds(1, high, sigfig)
    }

    /// Construct a `Histogram` with known upper and lower bounds for recorded sample values.
    ///
    /// `low` is the lowest value that can be discerned (distinguished from 0) by the histogram,
    /// and must be a positive integer that is >= 1. It may be internally rounded down to nearest
    /// power of 2. Providing a lowest discernible value (`low`) is useful is situations where the
    /// units used for the histogram's values are much smaller that the minimal accuracy required.
    /// E.g. when tracking time values stated in nanosecond units, where the minimal accuracy
    /// required is a microsecond, the proper value for `low` would be 1000. If you're not sure,
    /// use 1.
    ///
    /// `high` is the highest value to be tracked by the histogram, and must be a
    /// positive integer that is `>= (2 * low)`. If you're not sure, use `u64::max_value()`.
    ///
    /// `sigfig` Specifies the number of significant figures to maintain. This is the number of
    /// significant decimal digits to which the histogram will maintain value resolution and
    /// separation. Must be in the range [0, 5]. If you're not sure, use 3. As `sigfig` increases,
    /// memory usage grows exponentially, so choose carefully if there will be many histograms in
    /// memory at once or if storage is otherwise a concern.
    pub fn new_with_bounds(low: u64, high: u64, sigfig: u8) -> Result<Histogram<T>, CreationError> {
        // Verify argument validity
        if low < 1 {
            return Err(CreationError::LowIsZero);
        }
        if low > u64::max_value() / 2 {
            // avoid overflow in 2 * low
            return Err(CreationError::LowExceedsMax)
        }
        if high < 2 * low {
            return Err(CreationError::HighLessThanTwiceLow);
        }
        if sigfig > 5 {
            return Err(CreationError::SigFigExceedsMax);
        }

        // Given a 3 decimal point accuracy, the expectation is obviously for "+/- 1 unit at 1000".
        // It also means that it's "ok to be +/- 2 units at 2000". The "tricky" thing is that it is
        // NOT ok to be +/- 2 units at 1999. Only starting at 2000. So internally, we need to
        // maintain single unit resolution to 2x 10^decimal_points.

        // largest value with single unit resolution, in [2, 200_000].
        let largest = 2 * 10_u32.pow(sigfig as u32);

        let unit_magnitude = (low as f64).log2().floor() as u8;
        let unit_magnitude_mask = (1 << unit_magnitude) - 1;

        // We need to maintain power-of-two sub_bucket_count (for clean direct indexing) that is
        // large enough to provide unit resolution to at least
        // largest_value_with_single_unit_resolution. So figure out
        // largest_value_with_single_unit_resolution's nearest power-of-two (rounded up), and use
        // that.
        // In [1, 18]. 2^18 > 2 * 10^5 (the largest possible
        // largest_value_with_single_unit_resolution)
        let sub_bucket_count_magnitude = (largest as f64).log2().ceil() as u8;
        let sub_bucket_half_count_magnitude = sub_bucket_count_magnitude - 1;
        let sub_bucket_count = 1_u32 << (sub_bucket_count_magnitude as u32);

        if unit_magnitude + sub_bucket_count_magnitude > 63 {
            // sub_bucket_count entries can't be represented, with unit_magnitude applied, in a
            // u64. Technically it still sort of works if their sum is 64: you can represent all
            // but the last number in the shifted sub_bucket_count. However, the utility of such a
            // histogram vs ones whose magnitude here fits in 63 bits is debatable, and it makes
            // it harder to work through the logic. Sums larger than 64 are totally broken as
            // leading_zero_count_base would go negative.
            return Err(CreationError::CannotRepresentSigFigBeyondLow);
        };

        let sub_bucket_half_count = sub_bucket_count / 2;
        // sub_bucket_count is always at least 2, so subtraction won't underflow
        let sub_bucket_mask = (sub_bucket_count as u64 - 1) << unit_magnitude;

        let mut h = Histogram {
            auto_resize: false,

            highest_trackable_value: high,
            lowest_discernible_value: low,
            significant_value_digits: sigfig,

            // set by cover() below
            bucket_count: 0,
            sub_bucket_count: sub_bucket_count,


            // Establish leading_zero_count_base, used in bucket_index_of() fast path:
            // subtract the bits that would be used by the largest value in bucket 0.
            leading_zero_count_base: 64 - unit_magnitude - sub_bucket_count_magnitude,
            sub_bucket_half_count_magnitude: sub_bucket_half_count_magnitude,

            unit_magnitude: unit_magnitude,
            sub_bucket_half_count: sub_bucket_half_count,

            sub_bucket_mask: sub_bucket_mask,

            unit_magnitude_mask: unit_magnitude_mask,
            max_value: 0,
            min_non_zero_value: u64::max_value(),

            total_count: 0,
            // set by alloc() below
            counts: Vec::new(),
        };

        // determine exponent range needed to support the trackable value with no overflow:
        let len = h.cover(high);

        h.alloc(len as usize);
        Ok(h)
    }

    /// Construct a `Histogram` with the same range settings as a given source histogram,
    /// duplicating the source's start/end timestamps (but NOT its contents).
    pub fn new_from<F: Counter>(source: &Histogram<F>) -> Histogram<T> {
        let mut h = Self::new_with_bounds(source.lowest_discernible_value,
                                          source.highest_trackable_value,
                                          source.significant_value_digits)
            .unwrap();

        // h.start_time = source.start_time;
        // h.end_time = source.end_time;
        h.auto_resize = source.auto_resize;
        h.alloc(source.len());
        h
    }

    /// Allocate a counts array of the given size.
    fn alloc(&mut self, len: usize) {
        self.counts = std::iter::repeat(T::zero()).take(len).collect();
    }

    // ********************************************************************************************
    // Recording samples.
    // ********************************************************************************************

    /// Record `value` in the histogram.
    ///
    /// Returns an error if `value` exceeds the highest trackable value and auto-resize is
    /// disabled.
    pub fn record(&mut self, value: u64) -> Result<(), ()> {
        self.record_n(value, T::one())
    }

    /// Record multiple samples for a value in the histogram, adding to the value's current count.
    ///
    /// `count` is the number of occurrences of this value to record. Returns an error if `value`
    /// exceeds the highest trackable value and auto-resize is disabled.
    pub fn record_n(&mut self, value: u64, count: T) -> Result<(), ()> {
        self.alter_n(value, count, true)
    }

    fn alter_n(&mut self, value: u64, count: T, add: bool) -> Result<(), ()> {
        let success = if let Some(c) = self.mut_at(value) {
            if add {
                *c = *c + count;
            } else {
                *c = *c - count;
            }
            true
        } else {
            false
        };

        if !success {
            if !self.auto_resize {
                return Err(());
            }

            self.resize(value);

            {
                let c = self.mut_at(value).expect("value should fit after resize");
                if add {
                    *c = *c + count;
                } else {
                    *c = *c - count;
                }
            }

            self.highest_trackable_value = self.highest_equivalent(self.value_for(self.last()));
        }

        self.update_min_max(value);
        if add {
            self.total_count = self.total_count + count.to_u64().unwrap();
        } else {
            self.total_count = self.total_count - count.to_u64().unwrap();
        }
        Ok(())
    }

    /// Record a value in the histogram while correcting for coordinated omission.
    ///
    /// See `record_n_correct` for further documentation.
    pub fn record_correct(&mut self, value: u64, interval: u64) -> Result<(), ()> {
        self.record_n_correct(value, T::one(), interval)
    }

    /// Record multiple values in the histogram while correcting for coordinated omission.
    ///
    /// To compensate for the loss of sampled values when a recorded value is larger than the
    /// expected interval between value samples, this method will auto-generate and record an
    /// additional series of decreasingly-smaller (down to `interval`) value records.
    ///
    /// Note: This is a at-recording correction method, as opposed to the post-recording correction
    /// method provided by `correct_clone`. The two methods are mutually exclusive, and only one of
    /// the two should be be used on a given data set to correct for the same coordinated omission
    /// issue.
    ///
    /// Returns an error if `value` exceeds the highest trackable value and auto-resize is
    /// disabled.
    pub fn record_n_correct(&mut self, value: u64, count: T, interval: u64) -> Result<(), ()> {
        try!(self.record_n(value, count));
        if interval == 0 {
            return Ok(());
        }

        if value > interval {
            // only enter loop when calculations will stay non-negative
            let mut missing_value = value - interval;
            while missing_value >= interval {
                try!(self.record_n(missing_value, count));
                missing_value -= interval;
            }
        }

        Ok(())
    }

    // ********************************************************************************************
    // Iterators
    // ********************************************************************************************

    /// Iterate through histogram values by percentile levels.
    ///
    /// The iteration mechanic for this iterator may appear somewhat confusing, but it yields
    /// fairly pleasing output. The iterator starts with a *percentile step size* of
    /// `100/halving_period`. For every iteration, it yields a value whose percentile is that much
    /// greater than the previously emitted percentile (i.e., initially 0, 10, 20, etc.). Once
    /// `halving_period` values have been emitted, the percentile step size is halved, and the
    /// iteration continues.
    ///
    /// The iterator yields an `iterators::IterationValue` struct.
    ///
    /// ```
    /// use hdrsample::Histogram;
    /// use hdrsample::iterators::IterationValue;
    /// let mut hist = Histogram::<u64>::new_with_max(10000, 4).unwrap();
    /// for i in 0..10000 {
    ///     hist += i;
    /// }
    ///
    /// let mut perc = hist.iter_percentiles(1);
    ///
    /// println!("{:?}", hist.iter_percentiles(1).collect::<Vec<_>>());
    ///
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(0.01), 0.01, 1, 1)));
    /// // step size = 50
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(50.0), 50.0, 1, 5000 - 1)));
    /// // step size = 25
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(75.0), 75.0, 1, 2500)));
    /// // step size = 12.5
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(87.5), 87.5, 1, 1250)));
    /// // step size = 6.25
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(93.75), 93.75, 1, 625)));
    /// // step size = 3.125
    /// assert_eq!(perc.next(), Some(IterationValue::new(hist.value_at_percentile(96.88), 96.88, 1, 313)));
    /// // etc...
    /// ```
    pub fn iter_percentiles<'a>(&'a self, percentile_ticks_per_half_distance: isize)
            -> HistogramIterator<'a, T, iterators::percentile::Iter<'a, T>> {
        iterators::percentile::Iter::new(self, percentile_ticks_per_half_distance)
    }

    /// Iterates through histogram values using linear value steps. The iteration is performed in
    /// steps of size `step`, each one yielding the count for all values in the preceeding value
    /// range of size `step`. The iterator terminates when all recorded histogram values are
    /// exhausted.
    ///
    /// The iterator yields an `iterators::IterationValue` struct.
    ///
    /// ```
    /// use hdrsample::Histogram;
    /// use hdrsample::iterators::IterationValue;
    /// let mut hist = Histogram::<u64>::new_with_max(1000, 3).unwrap();
    /// hist += 100;
    /// hist += 500;
    /// hist += 800;
    /// hist += 850;
    ///
    /// let mut perc = hist.iter_linear(100);
    /// assert_eq!(perc.next(), Some(IterationValue::new(99, hist.percentile_below(99), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(199, hist.percentile_below(199), 0, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(299, hist.percentile_below(299), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(399, hist.percentile_below(399), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(499, hist.percentile_below(499), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(599, hist.percentile_below(599), 0, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(699, hist.percentile_below(699), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(799, hist.percentile_below(799), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(899, hist.percentile_below(899), 0, 2)));
    /// assert_eq!(perc.next(), None);
    /// ```
    pub fn iter_linear<'a>(&'a self, step: u64)
            -> HistogramIterator<'a, T, iterators::linear::Iter<'a, T>> {
        iterators::linear::Iter::new(self, step)
    }

    /// Iterates through histogram values at logarithmically increasing levels. The iteration is
    /// performed in steps that start at `start` and increase exponentially according to `exp`. The
    /// iterator terminates when all recorded histogram values are exhausted.
    ///
    /// The iterator yields an `iterators::IterationValue` struct.
    ///
    /// ```
    /// use hdrsample::Histogram;
    /// use hdrsample::iterators::IterationValue;
    /// let mut hist = Histogram::<u64>::new_with_max(1000, 3).unwrap();
    /// hist += 100;
    /// hist += 500;
    /// hist += 800;
    /// hist += 850;
    ///
    /// let mut perc = hist.iter_log(1, 10.0);
    /// assert_eq!(perc.next(), Some(IterationValue::new(0, hist.percentile_below(0), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(9, hist.percentile_below(9), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(99, hist.percentile_below(99), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(999, hist.percentile_below(999), 0, 4)));
    /// assert_eq!(perc.next(), None);
    /// ```
    pub fn iter_log<'a>(&'a self, start: u64, exp: f64)
            -> HistogramIterator<'a, T, iterators::log::Iter<'a, T>> {
        iterators::log::Iter::new(self, start, exp)
    }

    /// Iterates through all recorded histogram values using the finest granularity steps supported
    /// by the underlying representation. The iteration steps through all non-zero recorded value
    /// counts, and terminates when all recorded histogram values are exhausted.
    ///
    /// The iterator yields an `iterators::IterationValue` struct.
    ///
    /// ```
    /// use hdrsample::Histogram;
    /// use hdrsample::iterators::IterationValue;
    /// let mut hist = Histogram::<u64>::new_with_max(1000, 3).unwrap();
    /// hist += 100;
    /// hist += 500;
    /// hist += 800;
    /// hist += 850;
    ///
    /// let mut perc = hist.iter_recorded();
    /// assert_eq!(perc.next(), Some(IterationValue::new(100, hist.percentile_below(100), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(500, hist.percentile_below(500), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(800, hist.percentile_below(800), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(850, hist.percentile_below(850), 1, 1)));
    /// assert_eq!(perc.next(), None);
    /// ```
    pub fn iter_recorded<'a>(&'a self)
            -> HistogramIterator<'a, T, iterators::recorded::Iter<'a, T>> {
        iterators::recorded::Iter::new(self)
    }

    /// Iterates through all histogram values using the finest granularity steps supported by the
    /// underlying representation. The iteration steps through all possible unit value levels,
    /// regardless of whether or not there were recorded values for that value level, and
    /// terminates when all recorded histogram values are exhausted.
    ///
    /// The iterator yields an `iterators::IterationValue` struct.
    ///
    /// ```
    /// use hdrsample::Histogram;
    /// use hdrsample::iterators::IterationValue;
    /// let mut hist = Histogram::<u64>::new_with_max(10, 1).unwrap();
    /// hist += 1;
    /// hist += 5;
    /// hist += 8;
    ///
    /// let mut perc = hist.iter_all();
    /// assert_eq!(perc.next(), Some(IterationValue::new(0, 0.0, 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(1, hist.percentile_below(1), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(2, hist.percentile_below(2), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(3, hist.percentile_below(3), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(4, hist.percentile_below(4), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(5, hist.percentile_below(5), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(6, hist.percentile_below(6), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(7, hist.percentile_below(7), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(8, hist.percentile_below(8), 1, 1)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(9, hist.percentile_below(9), 0, 0)));
    /// assert_eq!(perc.next(), Some(IterationValue::new(10, 100.0, 0, 0)));
    /// ```
    pub fn iter_all<'a>(&'a self) -> HistogramIterator<'a, T, iterators::all::Iter> {
        iterators::all::Iter::new(self)
    }

    // ********************************************************************************************
    // Data statistics
    // ********************************************************************************************

    /// Get the lowest recorded value level in the histogram.
    /// If the histogram has no recorded values, the value returned will be 0.
    pub fn min(&self) -> u64 {
        if self.total_count == 0 || self[0_usize] != T::zero() {
            0
        } else {
            self.min_nz()
        }
    }

    /// Get the highest recorded value level in the histogram.
    /// If the histogram has no recorded values, the value returned is undefined.
    pub fn max(&self) -> u64 {
        if self.max_value == 0 {
            0
        } else {
            self.highest_equivalent(self.max_value)
        }
    }

    /// Get the lowest recorded non-zero value level in the histogram.
    /// If the histogram has no recorded values, the value returned is `u64::max_value()`.
    pub fn min_nz(&self) -> u64 {
        if self.min_non_zero_value == u64::max_value() {
            u64::max_value()
        } else {
            self.lowest_equivalent(self.min_non_zero_value)
        }
    }

    /// Determine if two values are equivalent with the histogram's resolution. Equivalent here
    /// means that value samples recorded for any two equivalent values are counted in a common
    /// total count.
    pub fn equivalent(&self, value1: u64, value2: u64) -> bool {
        self.lowest_equivalent(value1) == self.lowest_equivalent(value2)
    }

    /// Get the computed mean value of all recorded values in the histogram.
    pub fn mean(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        self.iter_recorded().fold(0.0_f64, |total, v| {
            total +
                self.median_equivalent(v.value()) as f64 * v.count_at_value().to_f64().unwrap()
                    / self.total_count as f64
        })
    }

    /// Get the computed standard deviation of all recorded values in the histogram
    pub fn stdev(&self) -> f64 {
        if self.total_count == 0 {
            return 0.0;
        }

        let mean = self.mean();
        let geom_dev_tot = self.iter_recorded().fold(0.0_f64, |gdt, v| {
            let dev = self.median_equivalent(v.value()) as f64 - mean;
            gdt + (dev * dev) * v.count_since_last_iteration() as f64
        });

        (geom_dev_tot / self.total_count as f64).sqrt()
    }

    /// Get the value at a given percentile.
    ///
    /// When the given percentile is > 0.0, the value returned is the value that the given
    /// percentage of the overall recorded value entries in the histogram are either smaller than
    /// or equivalent to. When the given percentile is 0.0, the value returned is the value that
    /// all value entries in the histogram are either larger than or equivalent to.
    ///
    /// Two values are considered "equivalent" if `self.equivalent` would return true.
    pub fn value_at_percentile(&self, percentile: f64) -> u64 {
        // Truncate down to 100%
        let percentile = if percentile > 100.0 {
            100.0
        } else {
            percentile
        };

        // round to nearest
        let mut count_at_percentile = (((percentile / 100.0) * self.total_count as f64) + 0.5) as u64;

        // Make sure we at least reach the first recorded entry
        if count_at_percentile == 0 {
            count_at_percentile = 1;
        }

        let mut total_to_current_index: u64 = 0;
        for i in 0..self.len() {
            total_to_current_index = total_to_current_index + self[i].to_u64().unwrap();
            if total_to_current_index >= count_at_percentile {
                let value_at_index = self.value_for(i);
                return if percentile == 0.0 {
                    self.lowest_equivalent(value_at_index)
                } else {
                    self.highest_equivalent(value_at_index)
                };
            }
        }

        0
    }

    /// Get the percentile of samples at and below a given value.
    ///
    /// The percentile returned is the percentile of values recorded in the histogram that are
    /// smaller than or equivalent to the given value.
    ///
    /// Two values are considered "equivalent" if `self.equivalent` would return true.
    pub fn percentile_below(&self, value: u64) -> f64 {
        if self.total_count == 0 {
            return 100.0;
        }

        let target_index = cmp::min(self.index_for(value), self.last());
        let total_to_current_index =
            (0..(target_index + 1)).map(|i| self[i]).fold(T::zero(), |t, v| t + v);
        100.0 * total_to_current_index.to_f64().unwrap() / self.total_count as f64
    }

    /// Get the count of recorded values within a range of value levels (inclusive to within the
    /// histogram's resolution).
    ///
    /// `low` gives the lower value bound on the range for which to provide the recorded count.
    /// Will be rounded down with `lowest_equivalent`. Similarly, `high` gives the higher value
    /// bound on the range, and will be rounded up with `highest_equivalent`. The function returns
    /// the total count of values recorded in the histogram within the value range that is `>=
    /// lowest_equivalent(low)` and `<= highest_equivalent(high)`.
    ///
    /// May fail if the given values are out of bounds.
    pub fn count_between(&self, low: u64, high: u64) -> Result<T, ()> {
        let low_index = self.index_for(low);
        let high_index = cmp::min(self.index_for(high), self.last());
        Ok((low_index..(high_index + 1)).map(|i| self[i]).fold(T::zero(), |t, v| t + v))
    }

    /// Get the count of recorded values at a specific value (to within the histogram resolution at
    /// the value level).
    ///
    /// The count is computed across values recorded in the histogram that are within the value
    /// range that is `>= lowest_equivalent(value)` and `<= highest_equivalent(value)`.
    ///
    /// May fail if the given value is out of bounds.
    pub fn count_at(&self, value: u64) -> Result<T, ()> {
        Ok(self[cmp::min(self.index_for(value), self.last())])
    }

    // ********************************************************************************************
    // Public helpers
    // ********************************************************************************************

    /// Computes the matching histogram value for the given histogram bin.
    pub fn value_for(&self, index: usize) -> u64 {
        // TODO what to do about indexes that are beyond what can be represented by this histogram?
        // Pretty easy to end up shifting off the high end of a u64, e.g. asking for a big index
        // when sigfigs is high.

        // Dividing by sub bucket half count will yield 1 in top half of first bucket, 2 in
        // in the top half (i.e., the only half that's used) of the 2nd bucket, etc, so subtract 1
        // to get 0-indexed bucket indexes. This will be -1 for the bottom half of the first bucket.
        let mut bucket_index = (index >> self.sub_bucket_half_count_magnitude) as isize - 1;
        // Calculate the remainder of dividing by sub_bucket_half_count, shifted into the top half
        // of the corresponding bucket. This will (temporarily) map indexes in the lower half of
        // first bucket into the top half.
        // The subtraction won't underflow because half count is always at least 1.
        // TODO precalculate sub_bucket_half_count mask if benchmarks show improvement
        let mut sub_bucket_index =
            ((index & (self.sub_bucket_half_count as usize - 1))
                + (self.sub_bucket_half_count as usize)) as u32;
        if bucket_index < 0 {
            // lower half of first bucket case; move sub bucket index back
            sub_bucket_index -= self.sub_bucket_half_count;
            bucket_index = 0;
        }
        self.value_from_loc(bucket_index as u8, sub_bucket_index)
    }

    /// Get the lowest value that is equivalent to the given value within the histogram's
    /// resolution. Equivalent here means that value samples recorded for any two equivalent values
    /// are counted in a common total count.
    pub fn lowest_equivalent(&self, value: u64) -> u64 {
        let bucket_index = self.bucket_for(value);
        let sub_bucket_index = self.sub_bucket_for(value, bucket_index);
        self.value_from_loc(bucket_index, sub_bucket_index)
    }

    /// Get the highest value that is equivalent to the given value within the histogram's
    /// resolution. Equivalent here means that value samples recorded for any two equivalent values
    /// are counted in a common total count.
    ///
    /// Note that the return value is capped at `u64::max_value()`.
    pub fn highest_equivalent(&self, value: u64) -> u64 {
        if value == u64::max_value() {
            u64::max_value()
        } else {
            self.next_non_equivalent(value) - 1
        }
    }

    /// Get a value that lies in the middle (rounded up) of the range of values equivalent the
    /// given value. Equivalent here means that value samples recorded for any two equivalent
    /// values are counted in a common total count.
    ///
    /// Note that the return value is capped at `u64::max_value()`.
    pub fn median_equivalent(&self, value: u64) -> u64 {
        match self.lowest_equivalent(value).overflowing_add(self.equivalent_range(value) >> 1) {
            (_, of) if of => u64::max_value(),
            (v, _) => v,
        }
    }

    /// Get the next value that is *not* equivalent to the given value within the histogram's
    /// resolution. Equivalent means that value samples recorded for any two equivalent values are
    /// counted in a common total count.
    ///
    /// Note that the return value is capped at `u64::max_value()`.
    pub fn next_non_equivalent(&self, value: u64) -> u64 {
        self.lowest_equivalent(value).saturating_add(self.equivalent_range(value))
    }

    /// Get the size (in value units) of the range of values that are equivalent to the given value
    /// within the histogram's resolution. Equivalent here means that value samples recorded for
    /// any two equivalent values are counted in a common total count.
    pub fn equivalent_range(&self, value: u64) -> u64 {
        let bucket_index = self.bucket_for(value);
        1_u64 << self.unit_magnitude + bucket_index
    }

    // ********************************************************************************************
    // Internal helpers
    // ********************************************************************************************

    /// Compute the lowest (and therefore highest precision) bucket index whose sub-buckets can
    /// represent the value.
    #[inline]
    fn bucket_for(&self, value: u64) -> u8 {
        // Calculates the number of powers of two by which the value is greater than the biggest
        // value that fits in bucket 0. This is the bucket index since each successive bucket can
        // hold a value 2x greater. The mask maps small values to bucket 0.
        self.leading_zero_count_base - (value | self.sub_bucket_mask).leading_zeros() as u8
    }

    #[inline]
    /// Compute the position inside a bucket at which the given value should be recorded, indexed
    /// from position 0 in the bucket (in the first half, which is not used past the first bucket).
    /// For bucket_index > 0, the result will be in the top half of the bucket.
    fn sub_bucket_for(&self, value: u64, bucket_index: u8) -> u32 {
        // Since bucket_index is simply how many powers of 2 greater value is than what will fit in
        // bucket 0 (that is, what will fit in [0, sub_bucket_count)), we shift off that many
        // powers of two, and end up with a number in [0, sub_bucket_count).
        // For bucket_index 0, this is just value. For bucket index k > 0, we know value won't fit
        // in bucket (k - 1) by definition, so this calculation won't end up in the lower half of
        // [0, sub_bucket_count) because that would mean it would also fit in bucket (k - 1).
        (value >> (bucket_index + self.unit_magnitude)) as u32
    }

    #[inline]
    fn value_from_loc(&self, bucket_index: u8, sub_bucket_index: u32) -> u64 {
        // Sum won't overflow; bucket_index and unit_magnitude are both <= 64.
        // However, the resulting shift may overflow given bogus input, e.g. if unit magnitude is
        // large and the input sub_bucket_index is for an entry in the counts index that shouldn't
        // be used (because this calculation will overflow).
        // TODO probably audit uses to make sure none will cause overflow
        (sub_bucket_index as u64) << (bucket_index + self.unit_magnitude)
    }

    /// Find the number of buckets needed such that `value` is representable.
    fn buckets_to_cover(&self, value: u64) -> u8 {
        // Shift won't overflow because sub_bucket_magnitude + unit_magnitude <= 63.
        // the k'th bucket can express from 0 * 2^k to sub_bucket_count * 2^k in units of 2^k
        let mut smallest_untrackable_value = (self.sub_bucket_count as u64) << self.unit_magnitude;

        // always have at least 1 bucket
        let mut buckets_needed = 1;
        while smallest_untrackable_value <= value {
            if smallest_untrackable_value > u64::max_value() / 2 {
                // next shift will overflow, meaning that bucket could represent values up to ones
                // greater than i64::max_value, so it's the last bucket
                return buckets_needed + 1;
            }
            smallest_untrackable_value <<= 1;
            buckets_needed += 1;
        }
        buckets_needed
    }

    /// Compute the actual number of bins to use for the given bucket count (that is, including the
    /// sub-buckets within each top-level bucket).
    ///
    /// If we have `N` such that `sub_bucket_count * 2^N > high`, we need storage for `N+1` buckets,
    /// each with enough slots to hold the top half of the `sub_bucket_count` (the lower half is
    /// covered by previous buckets), and the +1 being used for the lower half of the 0'th bucket.
    /// Or, equivalently, we need 1 more bucket to capture the max value if we consider the
    /// sub-bucket length to be halved.
    fn num_bins(&self, number_of_buckets: u8) -> u32 {
        (number_of_buckets as u32 + 1) * (self.sub_bucket_half_count)
    }

    /// Compute the number of buckets needed to cover the given value, as well as the total number
    /// of bins needed to cover that range.
    ///
    /// May fail if `high` is not at least twice the lowest discernible value.
    fn cover(&mut self, high: u64) -> u32 {
        // TODO validate before we get to here so we don't need an assert
        assert!(high >= 2 * self.lowest_discernible_value,
            "highest trackable value must be >= (2 * lowest discernible value)");

        // establish counts array length:
        let buckets_needed = self.buckets_to_cover(high);
        let counts_array_length = self.num_bins(buckets_needed);

        // establish exponent range needed to support the trackable value with no overflow:
        self.bucket_count = buckets_needed;

        // establish the new highest trackable value:
        self.highest_trackable_value = high;

        counts_array_length
    }

    /// Resize the underlying counts array such that it can cover the given `high` value.
    fn resize(&mut self, high: u64) {
        // figure out how large the sample tracker now needs to be
        let len = self.cover(high);

        // expand counts to also hold the new counts
        self.counts.resize(len as usize, T::zero());
    }

    /// Set internally tracked max_value to new value if new value is greater than current one.
    fn update_max(&mut self, value: u64) {
        let internal_value = value | self.unit_magnitude_mask; // Max unit-equivalent value
        if internal_value > self.max_value {
            self.max_value = internal_value;
        }
    }

    /// Set internally tracked min_non_zero_value to new value if new value is smaller than current
    /// one.
    fn update_min(&mut self, value: u64) {
        if value <= self.unit_magnitude_mask {
            return; // Unit-equivalent to 0.
        }

        let internal_value = value & !self.unit_magnitude_mask; // Min unit-equivalent value
        if internal_value < self.min_non_zero_value {
            self.min_non_zero_value = internal_value;
        }
    }

    fn update_min_max(&mut self, value: u64) {
        if value > self.max_value {
            self.update_max(value);
        }
        if value < self.min_non_zero_value && value != 0 {
            self.update_min(value);
        }
    }

    fn reset_max(&mut self, max: u64) {
        self.max_value = max | self.unit_magnitude_mask; // Max unit-equivalent value
    }

    fn reset_min(&mut self, min: u64) {
        let internal_value = min & !self.unit_magnitude_mask; // Min unit-equivalent value
        self.min_non_zero_value = if min == u64::max_value() {
            min
        } else {
            internal_value
        };
    }

    fn restat(&mut self, until: usize) {
        self.reset_max(0);
        self.reset_min(u64::max_value());

        let mut max_i = None;
        let mut min_i = None;
        let mut total_count: u64 = 0;
        for i in 0..until {
            let count = self[i];
            if count != T::zero() {
                total_count = total_count + count.to_u64().unwrap();
                max_i = Some(i);
                if min_i.is_none() && i != 0 {
                    min_i = Some(i);
                }
            }
        }

        if let Some(max_i) = max_i {
            let max = self.highest_equivalent(self.value_for(max_i));
            self.update_max(max);
        }
        if let Some(min_i) = min_i {
            let min = self.value_for(min_i);
            self.update_min(min);
        }

        self.total_count = total_count;
    }
}

// ********************************************************************************************
// Trait implementations
// ********************************************************************************************

impl<T: Counter> Index<usize> for Histogram<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.counts[index]
    }
}

impl<T: Counter> IndexMut<usize> for Histogram<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.counts[index]
    }
}

impl<T: Counter> Clone for Histogram<T> {
    fn clone(&self) -> Self {
        let mut h = Histogram::new_from(self);
        h += self;
        h
    }
}

// make it more ergonomic to add and subtract histograms
impl<'a, T: Counter> AddAssign<&'a Histogram<T>> for Histogram<T> {
    fn add_assign(&mut self, source: &'a Histogram<T>) {
        self.add(source).unwrap();
    }
}

impl<'a, T: Counter> SubAssign<&'a Histogram<T>> for Histogram<T> {
    fn sub_assign(&mut self, other: &'a Histogram<T>) {
        self.subtract(other).unwrap();
    }
}

// make it more ergonomic to record samples
impl<T: Counter> AddAssign<u64> for Histogram<T> {
    fn add_assign(&mut self, value: u64) {
        self.record(value).unwrap();
    }
}

// allow comparing histograms
impl<T: Counter, F: Counter> PartialEq<Histogram<F>> for Histogram<T>
    where T: PartialEq<F>
{
    fn eq(&self, other: &Histogram<F>) -> bool {
        if self.lowest_discernible_value != other.lowest_discernible_value ||
           self.significant_value_digits != other.significant_value_digits {
            return false;
        }
        if self.total_count != other.total_count {
            return false;
        }
        if self.max() != other.max() {
            return false;
        }
        if self.min_nz() != other.min_nz() {
            return false;
        }
        (0..self.len()).all(|i| self[i] == other[i])
    }
}

// /**
//  * Indicate whether or not the histogram is capable of supporting auto-resize functionality.
//  * Note that this is an indication that enabling auto-resize by calling set_auto_resize() is
//  * allowed, and NOT that the histogram will actually auto-resize. Use is_auto_resize() to
//  * determine if the histogram is in auto-resize mode.
//  * @return auto_resize setting
//  */
// public boolean supports_auto_resize() { return true; }

// TODO: shift
// TODO: hash
// TODO: serialization
// TODO: encoding/decoding
// TODO: timestamps and tags
// TODO: textual output

#[path = "tests/tests.rs"]
#[cfg(test)]
mod tests;
