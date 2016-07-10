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
//! in recording data values. **Author's note: I have not verified this to be the case for the
//! Rust port. Benchmarks welcome!**
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
//! hist.recordInInterval(54321, 10).expect("value 54321 should be in range");
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
//! println!("# of samples: {}", hist.total());
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
//! for (value, percentile, _, count) in hist.iter_recorded() {
//!     println!("{}'th percentile of data is {} with {} samples", percentile, value, count);
//! }
//! ```
//!
//! # Limitations and Caveats
//!
//! As with all the other HdrHistogram ports, the latest features and bug fixes from the upstream
//! HdrHistogram implementations may not be available in this port. A number of features have also
//! not (yet) been implemented:
//!
//!  - `CopyInto`-like methods.
//!  - Concurrency support (`AtomicHistogram`, `ConcurrentHistogram`, …).
//!  - `DoubleHistogram`. You can use `f64` as the counter type, but none of the "special"
//!    `DoubleHistogram` features are supported.
//!  - The `Recorder` feature of HdrHistogram.
//!  - Histogram subtraction.
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

#![allow(non_snake_case)]

extern crate num;

use std::ops::Index;
use std::ops::IndexMut;
use std::ops::AddAssign;
use std::borrow::Borrow;

pub struct Histogram<T: num::Num> {
    autoResize: bool,

    highestTrackableValue: i64,
    lowestDiscernibleValue: i64,
    significantValueDigits: u32,

    bucketCount: usize,
    subBucketCount: usize,

    leadingZeroCountBase: isize,
    subBucketHalfCountMagnitude: isize,

    unitMagnitude: isize,
    subBucketHalfCount: usize,

    subBucketMask: i64,

    unitMagnitudeMask: i64,

    // TODO: use options for these?
    maxValue: i64, // 0
    minNonZeroValue: i64, // MAX

    // atomics for maxValue/minNonZeroValue ?
    totalCount: i64,
    counts: Vec<T>,
}

// accessors

impl<T: num::Num> Histogram<T> {
    /**
     * Get the current number of bins
     */
    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /**
     * Get the configured lowest discernible value.
     */
    pub fn getLowestDiscernibleValue(&self) -> i64 {
        self.lowestDiscernibleValue
    }

    /**
     * Get the highest trackable value.
     */
    pub fn getHighestTrackableValue(&self) -> i64 {
        self.highestTrackableValue
    }

    /**
     * Get the configured number of significant value digits
     */
    pub fn getNumberOfSignificantValueDigits(&self) -> u32 {
        self.significantValueDigits
    }

    // TODO: -> count
    pub fn total(&self) -> i64 {
        self.totalCount
    }
}

// lookups

impl<T: num::Num> Histogram<T> {
    fn indexOf(&self, value: i64) -> isize {
        if value < 0 {
            panic!("Histogram recorded value cannot be negative.");
        }

        let bucketIndex = self.bucketIndexOf(value);
        let subBucketIndex = self.subBucketIndexOf(value, bucketIndex);

        assert!(subBucketIndex < self.subBucketCount as isize);
        assert!(bucketIndex == 0 || (subBucketIndex >= self.subBucketHalfCount as isize));

        // Calculate the index for the first entry that will be used in the bucket (halfway through
        // subBucketCount). For bucketIndex 0, all subBucketCount entries may be used, but
        // bucketBaseIndex is still set in the middle.
        let bucketBaseIndex = (bucketIndex + 1) << self.subBucketHalfCountMagnitude;

        // Calculate the offset in the bucket. This subtraction will result in a positive value in
        // all buckets except the 0th bucket (since a value in that bucket may be less than half
        // the bucket's 0 to subBucketCount range). However, this works out since we give bucket 0
        // twice as much space.
        let offsetInBucket = subBucketIndex - self.subBucketHalfCount as isize;

        // The following is the equivalent of
        // ((subBucketIndex  - subBucketHalfCount) + bucketBaseIndex;
        (bucketBaseIndex + offsetInBucket) as isize
    }

    fn get_at(&self, value: i64) -> Result<&T, ()> {
        let i = self.indexOf(value);
        if i < 0 || i >= self.len() as isize {
            Err(())
        } else {
            Ok(&self.counts[i as usize])
        }
    }

    fn mut_at(&mut self, value: i64) -> Result<&mut T, ()> {
        let i = self.indexOf(value);
        if i < 0 || i >= self.len() as isize {
            Err(())
        } else {
            Ok(&mut self.counts[i as usize])
        }
    }
}

// using an index
impl<T: num::Num> Index<usize> for Histogram<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        &self.counts[index]
    }
}

impl<T: num::Num> IndexMut<usize> for Histogram<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.counts[index]
    }
}

// using a value
impl<T: num::Num> Index<i64> for Histogram<T> {
    type Output = T;
    fn index(&self, value: i64) -> &Self::Output {
        self.get_at(value).unwrap()
    }
}

impl<T: num::Num> IndexMut<i64> for Histogram<T> {
    fn index_mut(&mut self, value: i64) -> &mut Self::Output {
        self.mut_at(value).unwrap()
    }
}

// various cloning functions

impl<T: num::Num + num::ToPrimitive + Copy> Clone for Histogram<T> {
    fn clone(&self) -> Self {
        let mut h = Histogram::new_from(self);
        h += self;
        h
    }
}

impl<T: num::Num + num::ToPrimitive + Copy> Histogram<T> {
    /// Add the contents of another histogram to this one.
    ///
    /// As part of adding the contents, the start/end timestamp range of this histogram will be
    /// extended to include the start/end timestamp range of the other histogram.
    ///
    /// May panic if values in the other histogram are higher than `highestTrackableValue`, and
    /// auto-resize is disabled.
    ///
    pub fn add<B: Borrow<Histogram<T>>>(&mut self, source: B) -> Result<(), &'static str> {
        let source = source.borrow();

        // make sure we can take the values in source
        let top = self.highest_equivalent(self.value_from_index(self.lastIndex()));
        if top < source.max() {
            if !self.autoResize {
                return Err("The other histogram includes values that do not fit in this \
                            histogram's range.");
            }
            self.resize(source.max());
        }

        if self.bucketCount == source.bucketCount && self.subBucketCount == source.subBucketCount &&
           self.unitMagnitude == source.unitMagnitude {
            // Counts arrays are of the same length and meaning,
            // so we can just iterate and add directly:
            let mut observedOtherTotalCount = 0i64;
            for i in 0..source.len() {
                let otherCount = source[i];
                if otherCount != T::zero() {
                    self[i] = self[i] + otherCount;
                    observedOtherTotalCount += otherCount.to_i64().unwrap();
                }
            }

            self.totalCount += observedOtherTotalCount;
            let mx = source.max();
            if mx > self.max() {
                self.updateMaxValue(mx);
            }
            let mn = source.min_nz();
            if mn < self.min_nz() {
                self.updateMinNonZeroValue(mn);
            }
        } else {
            // Arrays are not a direct match (or the other could change on the fly in some valid
            // way), so we can't just stream through and add them. Instead, go through the array
            // and add each non-zero value found at it's proper value:

            // Do max value first, to avoid max value updates on each iteration:
            let otherMaxIndex = source.indexOf(source.max()) as usize;
            let otherCount = source[otherMaxIndex];
            self.recordCountAtValue(otherCount, source.value_from_index(otherMaxIndex)).unwrap();

            // Record the remaining values, up to but not including the max value:
            for i in 0..otherMaxIndex {
                let otherCount = source[i];
                if otherCount != T::zero() {
                    self.recordCountAtValue(otherCount, source.value_from_index(i)).unwrap();
                }
            }
        }

        // TODO:
        // if source.startTime < self.startTime {
        //     self.startTime = source.startTime;
        // }
        // if source.endTime > self.endTime {
        //     self.endTime = source.endTime;
        // }
        Ok(())
    }
}

impl<'a, T: num::Num + num::ToPrimitive + Copy> AddAssign<&'a Histogram<T>> for Histogram<T> {
    fn add_assign(&mut self, source: &'a Histogram<T>) {
        self.add(source).unwrap();
    }
}


impl<T: num::Num + num::ToPrimitive + Copy> Histogram<T> {
    /**
     * Get a copy of this histogram, corrected for coordinated omission.
     *
     * To compensate for the loss of sampled values when a recorded value is larger than the
     * expected interval between value samples, the new histogram will include an auto-generated
     * additional series of decreasingly-smaller (down to the `expectedInterval`) value records for
     * each count found in the current histogram that is larger than the `expectedInterval`.
     *
     * Note: This is a post-correction method, as opposed to the at-recording correction method
     * provided by `recordValueWithExpectedInterval`. The two methods are mutually exclusive, and
     * only one of the two should be be used on a given data set to correct for the same
     * coordinated omission issue.
     *
     * See notes in the description of the Histogram calls for an illustration of why this
     * corrective behavior is important.
     *
     * If `expectedInterval` is larger than 0, add auto-generated value records as appropriate if
     * value is larger than `expectedInterval`.
     */
    pub fn correctedClone(&self, expectedInterval: i64) -> Histogram<T> {
        let mut h = Histogram::new_from(self);
        for (value, _, count, _) in self.iter_recorded() {
            h.recordCountWithInterval(value, count, expectedInterval).unwrap();
        }
        h
    }
}

// administrative (resize, clear, copy, etc.)

impl<T: num::Num> Histogram<T> {
    /**
     * Reset the contents of this histogram.
     */
    pub fn clear(&mut self) {
        for c in self.counts.iter_mut() {
            *c = T::zero();
        }
        self.totalCount = 0;
    }

    /**
     * Reset the contents and stats of this histogram.
     */
    pub fn reset(&mut self) {
        self.clear();
        self.resetMaxValue(0);
        self.resetMinNonZeroValue(i64::max_value());
        // self.normalizingIndexOffset = 0;
        // self.startTime = time::Instant::now();
        // self.endTime = time::Instant::now();
        // self.tag = String::new();
    }

    fn resetMaxValue(&mut self, maxValue: i64) {
        self.maxValue = maxValue | self.unitMagnitudeMask; // Max unit-equivalent value
    }

    fn resetMinNonZeroValue(&mut self, minNonZeroValue: i64) {
        let internalValue = minNonZeroValue & !self.unitMagnitudeMask; // Min unit-equivalent value
        self.minNonZeroValue = if minNonZeroValue == i64::max_value() {
            minNonZeroValue
        } else {
            internalValue
        };
    }

    /**
     * Control whether or not the histogram can auto-resize and auto-adjust it's
     * `highestTrackableValue`.
     */
    pub fn setAutoResize(&mut self, enabled: bool) {
        self.autoResize = enabled;
    }

    fn bucketsNeededToCover(&self, value: i64) -> usize {
        // the k'th bucket can express from 0 * 2^k to subBucketCount * 2^k in units of 2^k
        let mut smallestUntrackableValue = (self.subBucketCount as i64) << self.unitMagnitude;

        // always have at least 1 bucket
        let mut bucketsNeeded = 1;
        while smallestUntrackableValue <= value {
            if smallestUntrackableValue > i64::max_value() / 2 {
                // next shift will overflow, meaning that bucket could represent values up to ones
                // greater than i64::max_value, so it's the last bucket
                return bucketsNeeded + 1;
            }
            smallestUntrackableValue <<= 1;
            bucketsNeeded += 1;
        }
        bucketsNeeded
    }

    /**
     * If we have N such that subBucketCount * 2^N > max value, we need storage for N+1 buckets,
     * each with enough slots to hold the top half of the subBucketCount (the lower half is covered
     * by previous buckets), and the +1 being used for the lower half of the 0'th bucket. Or,
     * equivalently, we need 1 more bucket to capture the max value if we consider the sub-bucket
     * length to be halved.
     */
    fn lengthForNumberOfBuckets(&self, numberOfBuckets: usize) -> usize {
        (numberOfBuckets + 1) * (self.subBucketCount / 2)
    }

    /**
     * The buckets (each of which has subBucketCount sub-buckets, here assumed to be 2048 as an
     * example) overlap:
     *
     * <pre>
     * The 0'th bucket covers 0...2047 in multiples of 1, using all 2048 sub-buckets
     * The 1'th bucket covers 2048..4097 in multiples of 2, using only the top 1024 sub-buckets
     * The 2'th bucket covers 4096..8191 in multiple of 4, using only the top 1024 sub-buckets
     * ...
     * </pre>
     *
     * Bucket 0 is "special" here. It is the only one that has 2048 entries. All the rest have 1024
     * entries (because their bottom half overlaps with and is already covered by the all of the
     * previous buckets put together). In other words, the k'th bucket could represent 0 * 2^k to
     * 2048 * 2^k in 2048 buckets with 2^k precision, but the midpoint of 1024 * 2^k = 2048 *
     * 2^(k-1) = the k-1'th bucket's end, so we would use the previous bucket for those lower
     * values as it has better precision.
     */
    fn establishSize(&mut self, newHighestTrackableValue: i64) -> Result<usize, &'static str> {
        if newHighestTrackableValue < 2i64 * self.lowestDiscernibleValue {
            return Err("highestTrackableValue cannot be < (2 * lowestDiscernibleValue)");
        }

        // establish counts array length:
        let bucketsNeeded = self.bucketsNeededToCover(newHighestTrackableValue);
        let countsArrayLength = self.lengthForNumberOfBuckets(bucketsNeeded);

        // establish exponent range needed to support the trackable value with no overflow:
        self.bucketCount = bucketsNeeded;

        // establish the new highest trackable value:
        self.highestTrackableValue = newHighestTrackableValue;

        Ok(countsArrayLength)
    }
}

// Construction

impl<T: num::Num + Copy> Histogram<T> {
    /**
     * Construct an auto-resizing histogram with a lowest discernible value of 1 and an
     * auto-adjusting highestTrackableValue. Can auto-resize up to track values up to
     * `(i64::max_value() / 2)`.
     *
     * `significantValueDigits` specifies the precision to use. This is the number of significant
     * decimal digits to which the histogram will maintain value resolution and separation. Must be
     * a non-negative integer between 0 and 5.
     */
    pub fn new(significantValueDigits: u32) -> Result<Histogram<T>, &'static str> {
        let mut h = Self::new_with_bounds(1, 2, significantValueDigits);
        if let Ok(ref mut h) = h {
            h.autoResize = true;
        }
        h
    }

    /**
     * Construct a Histogram given the Highest value to be tracked and a number of significant
     * decimal digits. The histogram will be constructed to implicitly track (distinguish from 0)
     * values as low as 1.
     *
     * `highestTrackableValue` is the highest value to be tracked by the histogram. Must be a
     * positive integer that is >= 2. `significantValueDigits` specifies the precision to use. This
     * is the number of significant decimal digits to which the histogram will maintain value
     * resolution and separation. Must be a non-negative integer between 0 and 5.
     */
    pub fn new_with_max(highestTrackableValue: i64,
                        significantValueDigits: u32)
                        -> Result<Histogram<T>, &'static str> {
        Self::new_with_bounds(1, highestTrackableValue, significantValueDigits)
    }

    /**
     * Construct a Histogram given the Lowest and Highest values to be tracked and a number of
     * significant decimal digits. Providing a `lowestDiscernibleValue` is useful is situations
     * where the units used for the histogram's values are much smaller that the minimal accuracy
     * required. E.g. when tracking time values stated in nanosecond units, where the minimal
     * accuracy required is a microsecond, the proper value for `lowestDiscernibleValue` would be
     * 1000.
     *
     * `lowestDiscernibleValue` is the lowest value that can be discerned (distinguished from 0) by
     * the histogram. Must be a positive integer that is >= 1. May be internally rounded down to
     * nearest power of 2. `highestTrackableValue` is the highest value to be tracked by the
     * histogram. Must be a positive integer that is >= (2 * `lowestDiscernibleValue`).
     * `significantValueDigits` Specifies the precision to use. This is the number of significant
     * decimal digits to which the histogram will maintain value resolution and separation. Must be
     * a non-negative integer between 0 and 5.
     */
    pub fn new_with_bounds(lowestDiscernibleValue: i64,
                           highestTrackableValue: i64,
                           significantValueDigits: u32)
                           -> Result<Histogram<T>, &'static str> {
        // Verify argument validity
        if lowestDiscernibleValue < 1 {
            return Err("lowestDiscernibleValue must be >= 1");
        }
        if highestTrackableValue < 2i64 * lowestDiscernibleValue {
            return Err("highestTrackableValue must be >= 2 * lowestDiscernibleValue");
        }
        if significantValueDigits > 5 {
            return Err("numberOfSignificantValueDigits must be between 0 and 5");
        }

        // Given a 3 decimal point accuracy, the expectation is obviously for "+/- 1 unit at 1000".
        // It also means that it's "ok to be +/- 2 units at 2000". The "tricky" thing is that it is
        // NOT ok to be +/- 2 units at 1999. Only starting at 2000. So internally, we need to
        // maintain single unit resolution to 2x 10^decimalPoints.
        //

        // largest value with single unit resolution
        let largest = 2 * 10i64.pow(significantValueDigits as u32);

        let unitMagnitude = ((lowestDiscernibleValue as f64).log2() / 2f64.log2()).floor() as isize;
        let unitMagnitudeMask = (1 << unitMagnitude) - 1;

        // We need to maintain power-of-two subBucketCount (for clean direct indexing) that is
        // large enough to provide unit resolution to at least
        // largestValueWithSingleUnitResolution. So figure out
        // largestValueWithSingleUnitResolution's nearest power-of-two (rounded up), and use that:
        let subBucketCountMagnitude = ((largest as f64).log2() / 2f64.log2()).ceil() as isize;
        let subBucketHalfCountMagnitude = if subBucketCountMagnitude > 1 {
            subBucketCountMagnitude
        } else {
            1
        } - 1;
        let subBucketCount = 2usize.pow(subBucketHalfCountMagnitude as u32 + 1);
        let subBucketHalfCount = subBucketCount / 2;
        let subBucketMask = (subBucketCount as i64 - 1) << unitMagnitude;

        let mut h = Histogram {
            autoResize: false,

            highestTrackableValue: highestTrackableValue,
            lowestDiscernibleValue: lowestDiscernibleValue,
            significantValueDigits: significantValueDigits,

            bucketCount: 0, // set by establishSize below
            subBucketCount: subBucketCount,

            leadingZeroCountBase: 0, // set below, needs establishSize
            subBucketHalfCountMagnitude: subBucketHalfCountMagnitude,

            unitMagnitude: unitMagnitude,
            subBucketHalfCount: subBucketHalfCount,

            subBucketMask: subBucketMask,

            unitMagnitudeMask: unitMagnitudeMask,
            maxValue: 0,
            minNonZeroValue: i64::max_value(),

            // TODO: atomics for maxValue/minNonZeroValue ?
            totalCount: 0,
            counts: Vec::new(), // set by alloc() below
        };

        // determine exponent range needed to support the trackable value with no overflow:
        let len = try!(h.establishSize(highestTrackableValue));

        // Establish leadingZeroCountBase, used in bucketIndexOf() fast path:
        // subtract the bits that would be used by the largest value in bucket 0.
        h.leadingZeroCountBase = 64 - h.unitMagnitude - h.subBucketHalfCountMagnitude - 1;

        // TODO:
        // percentileIterator = new PercentileIterator(this, 1);
        // recordedValuesIterator = new RecordedValuesIterator(this);

        h.alloc(len);
        Ok(h)
    }

    /**
     * Construct a histogram with the same range settings as a given source histogram, duplicating
     * the source's start/end timestamps (but NOT its contents).
     */
    pub fn new_from<F: num::Num>(source: &Histogram<F>) -> Histogram<T> {
        let mut h = Self::new_with_bounds(source.lowestDiscernibleValue,
                                          source.highestTrackableValue,
                                          source.significantValueDigits)
            .unwrap();

        // h.startTime = source.startTime;
        // h.endTime = source.endTime;
        h.autoResize = source.autoResize;
        h.alloc(source.len());
        h
    }

    fn alloc(&mut self, len: usize) {
        use std::iter;
        self.counts = iter::repeat(T::zero()).take(len).collect();
    }

    fn resize(&mut self, newHighestTrackableValue: i64) {
        // figure out how large the sample tracker now needs to be
        let len = self.establishSize(newHighestTrackableValue).unwrap();

        // expand counts to also hold the new counts
        self.counts.resize(len, T::zero());
    }
}

// recording

impl<T: num::Num + num::ToPrimitive + Copy> AddAssign<i64> for Histogram<T> {
    fn add_assign(&mut self, value: i64) {
        self.record(value).unwrap();
    }
}

impl<T: num::Num + num::ToPrimitive + Copy> Histogram<T> {
    /**
     * Record `value` in the histogram.
     *
     * Returns an error if `value` exceeds `highestTrackableValue` and aut-resize is disabled.
     */
    pub fn record(&mut self, value: i64) -> Result<(), ()> {
        self.recordCountAtValue(T::one(), value)
    }

    /**
     * Record a value in the histogram, adding to the value's current count.
     *
     * `count` is the number of occurrences of this value to record. Returns an error if `value`
     * exceeds `highestTrackableValue` and auto-resize is disabled.
     */
    pub fn record_n(&mut self, value: i64, count: T) -> Result<(), ()> {
        self.recordCountAtValue(count, value)
    }

    /**
     * Record a value in the histogram.
     *
     * To compensate for the loss of sampled values when a recorded value is larger than the
     * expected interval between value samples, Histogram will auto-generate an additional series
     * of decreasingly-smaller (down to the `expectedIntervalBetweenValueSamples`) value records.
     *
     * Note: This is a at-recording correction method, as opposed to the post-recording correction
     * method provided by `copyCorrectedForCoordinatedOmission`. The two methods are mutually
     * exclusive, and only one of the two should be be used on a given data set to correct for the
     * same coordinated omission issue.
     *
     * See notes in the description of the Histogram calls for an illustration of why this
     * corrective behavior is important.
     *
     * If `expectedIntervalBetweenValueSamples` is larger than 0, add auto-generated value records
     * as appropriate if value is larger than `expectedIntervalBetweenValueSamples`.
     * @throws ArrayIndexOutOfBoundsException (may throw) if value is exceeds highestTrackableValue
     *
     * Returns an error if `value` exceeds `highestTrackableValue` and auto-resize is disabled.
     */
    pub fn recordInInterval(&mut self,
                            value: i64,
                            expectedIntervalBetweenValueSamples: i64)
                            -> Result<(), ()> {
        self.recordCountWithInterval(value, T::one(), expectedIntervalBetweenValueSamples)
    }

    fn recordCountAtValue(&mut self, count: T, value: i64) -> Result<(), ()> {
        let success = if let Ok(c) = self.mut_at(value) {
            *c = *c + count;
            true
        } else {
            false
        };

        if !success {
            if !self.autoResize {
                return Err(());
            }
            self.handleRecordException(count, value);
        }

        self.updateMinAndMax(value);
        self.totalCount += count.to_i64().unwrap();
        Ok(())
    }

    fn handleRecordException(&mut self, count: T, value: i64) {
        self.resize(value);
        {
            let v = self.mut_at(value).expect("value should fit after resize");
            *v = *v + count;
        }

        self.highestTrackableValue =
            self.highest_equivalent(self.value_from_index(self.lastIndex()));
    }

    fn recordCountWithInterval(&mut self,
                               value: i64,
                               count: T,
                               expectedInterval: i64)
                               -> Result<(), ()> {

        try!(self.recordCountAtValue(count, value));
        if expectedInterval <= 0 {
            return Ok(());
        }

        let mut missingValue = value - expectedInterval;
        while missingValue >= expectedInterval {
            try!(self.recordCountAtValue(count, missingValue));
            missingValue -= expectedInterval;
        }
        Ok(())
    }

    /**
     * Set internally tracked maxValue to new value if new value is greater than current one.
     */
    fn updateMaxValue(&mut self, value: i64) {
        let internalValue = value | self.unitMagnitudeMask; // Max unit-equivalent value
        if internalValue > self.maxValue {
            self.maxValue = internalValue;
        }
    }

    /**
     * Set internally tracked minNonZeroValue to new value if new value is smaller than current
     * one.
     */
    fn updateMinNonZeroValue(&mut self, value: i64) {
        if value <= self.unitMagnitudeMask {
            return; // Unit-equivalent to 0.
        }

        let internalValue = value & !self.unitMagnitudeMask; // Min unit-equivalent value
        if internalValue < self.minNonZeroValue {
            self.minNonZeroValue = internalValue;
        }
    }

    fn updateMinAndMax(&mut self, value: i64) {
        if value > self.maxValue {
            self.updateMaxValue(value);
        }
        if value < self.minNonZeroValue && value != 0 {
            self.updateMinNonZeroValue(value);
        }
    }
}

// comparison

impl<T: num::Num + num::ToPrimitive, F: num::Num + num::ToPrimitive> PartialEq<Histogram<F>> for Histogram<T> {
    fn eq(&self, other: &Histogram<F>) -> bool {
        if self.lowestDiscernibleValue != other.lowestDiscernibleValue ||
           self.significantValueDigits != other.significantValueDigits {
            return false;
        }
        if self.totalCount != other.totalCount {
            return false;
        }
        if self.max() != other.max() {
            return false;
        }
        if self.min_nz() != other.min_nz() {
            return false;
        }
        (0..self.len()).all(|i| self[i].to_i64() == other[i].to_i64())
    }
}

// iterators

pub mod iterators;
impl<T: num::Num + Copy> Histogram<T> {
    /**
     * Provide a means of iterating through histogram values according to percentile levels. The
     * iteration is performed in steps that start at 0% and reduce their distance to 100% according
     * to the `percentileTicksPerHalfDistance` parameter, ultimately reaching 100% when all
     * recorded histogram values are exhausted.
     */
    pub fn iter_percentiles<'a>
        (&'a self,
         percentileTicksPerHalfDistance: isize)
         -> iterators::HistogramIterator<'a, T, iterators::percentile::Iter<'a, T>> {
        iterators::percentile::Iter::new(self, percentileTicksPerHalfDistance)
    }

    /**
     * Provide a means of iterating through histogram values using linear steps. The iteration is
     * performed in steps of `valueUnitsPerBucket` in size, terminating when all recorded histogram
     * values are exhausted.
     */
    pub fn iter_linear<'a>
        (&'a self,
         valueUnitsPerBucket: i64)
         -> iterators::HistogramIterator<'a, T, iterators::linear::Iter<'a, T>> {
        iterators::linear::Iter::new(self, valueUnitsPerBucket)
    }

    /**
     * Provide a means of iterating through histogram values at logarithmically increasing levels.
     * The iteration is performed in steps that start at `valueUnitsInFirstBucket` and increase
     * exponentially according to `logBase`, terminating when all recorded histogram values are
     * exhausted.
     */
    pub fn iter_log<'a>(&'a self,
                        valueUnitsInFirstBucket: i64,
                        logBase: f64)
                        -> iterators::HistogramIterator<'a, T, iterators::log::Iter<'a, T>> {
        iterators::log::Iter::new(self, valueUnitsInFirstBucket, logBase)
    }

    /**
     * Provide a means of iterating through all recorded histogram values using the finest
     * granularity steps supported by the underlying representation. The iteration steps through
     * all non-zero recorded value counts, and terminates when all recorded histogram values are
     * exhausted.
     */
    pub fn iter_recorded<'a>
        (&'a self)
         -> iterators::HistogramIterator<'a, T, iterators::recorded::Iter<'a, T>> {
        iterators::recorded::Iter::new(self)
    }

    /**
     * Provide a means of iterating through all histogram values using the finest granularity steps
     * supported by the underlying representation. The iteration steps through all possible unit
     * value levels, regardless of whether or not there were recorded values for that value level,
     * and terminates when all recorded histogram values are exhausted.
     */
    pub fn iter_all<'a>(&'a self) -> iterators::HistogramIterator<'a, T, iterators::all::Iter> {
        iterators::all::Iter::new(self)
    }
}


// minor data statistics

impl<T: num::Num> Histogram<T> {
    /**
     * Get the lowest value that is equivalent to the given value within the histogram's
     * resolution. Where "equivalent" means that value samples recorded for any two equivalent
     * values are counted in a common total count.
     */
    pub fn lowest_equivalent(&self, value: i64) -> i64 {
        let bucketIndex = self.bucketIndexOf(value);
        let subBucketIndex = self.subBucketIndexOf(value, bucketIndex);
        self.valueFromLocation(bucketIndex, subBucketIndex)
    }

    /**
     * Get the highest value that is equivalent to the given value within the histogram's
     * resolution. Where "equivalent" means that value samples recorded for any two equivalent
     * values are counted in a common total count.
     */
    pub fn highest_equivalent(&self, value: i64) -> i64 {
        self.next_non_equivalent(value) - 1
    }

    /**
     * Get a value that lies in the middle (rounded up) of the range of values equivalent the given
     * value. Where "equivalent" means that value samples recorded for any two equivalent values
     * are counted in a common total count.
     */
    pub fn median_equivalent(&self, value: i64) -> i64 {
        match self.lowest_equivalent(value).overflowing_add(self.equivalent_range_len(value) >> 1) {
            (_, of) if of => i64::max_value(),
            (v, _) => v,
        }
    }

    /**
     * Get the next value that is not equivalent to the given value within the histogram's
     * resolution. Where "equivalent" means that value samples recorded for any two equivalent
     * values are counted in a common total count.
     */
    pub fn next_non_equivalent(&self, value: i64) -> i64 {
        match self.lowest_equivalent(value).overflowing_add(self.equivalent_range_len(value)) {
            (_, of) if of => i64::max_value(),
            (v, _) => v,
        }
    }

    /**
     * Get the lowest recorded value level in the histogram. If the histogram has no recorded
     * values, the value returned is undefined.
     */
    pub fn min(&self) -> i64 {
        if self.totalCount == 0 || self[0usize] != T::zero() {
            0
        } else {
            self.min_nz()
        }
    }

    /**
     * Get the highest recorded value level in the histogram. If the histogram has no recorded
     * values, the value returned is undefined.
     */
    pub fn max(&self) -> i64 {
        if self.maxValue == 0 {
            0
        } else {
            self.highest_equivalent(self.maxValue)
        }
    }

    /**
     * Get the lowest recorded non-zero value level in the histogram. If the histogram has no
     * recorded values, the value returned is undefined.
     */
    pub fn min_nz(&self) -> i64 {
        if self.minNonZeroValue == i64::max_value() {
            i64::max_value()
        } else {
            self.lowest_equivalent(self.minNonZeroValue)
        }
    }

    /**
     * Determine if two values are equivalent with the histogram's resolution.
     * Where "equivalent" means that value samples recorded for any two
     * equivalent values are counted in a common total count.
     *
     * @param value1 first value to compare
     * @param value2 second value to compare
     * @return True if values are equivalent with the histogram's resolution.
     */
    pub fn equivalent(&self, value1: i64, value2: i64) -> bool {
        self.lowest_equivalent(value1) == self.lowest_equivalent(value2)
    }
}

// major data statistics

impl<T: num::Num + num::ToPrimitive + Copy> Histogram<T> {
    /**
     * Get the computed mean value of all recorded values in the histogram.
     */
    pub fn mean(&self) -> f64 {
        if self.totalCount == 0 {
            return 0.0;
        }

        self.iter_recorded().fold(0.0f64, |total, (v, _, c, _)| {
            total +
            self.median_equivalent(v) as f64 * c.to_i64().unwrap() as f64 / self.totalCount as f64
        })
    }

    /**
     * Get the computed standard deviation of all recorded values in the histogram
     */
    pub fn stdev(&self) -> f64 {
        if self.totalCount == 0 {
            return 0.0;
        }

        let mean = self.mean();
        let geom_dev_tot = self.iter_recorded().fold(0.0f64, |gdt, (v, _, _, sc)| {
            let dev = self.median_equivalent(v) as f64 - mean;
            gdt + (dev * dev) * sc as f64
        });

        (geom_dev_tot / self.totalCount as f64).sqrt()
    }

    /**
     * Get the value at a given percentile.
     *
     * When the given percentile is > 0.0, the value returned is the value that the given
     * percentage of the overall recorded value entries in the histogram are either smaller than or
     * equivalent to. When the given percentile is 0.0, the value returned is the value that all
     * value entries in the histogram are either larger than or equivalent to.
     *
     * Note that two values are "equivalent" in this statement if `self.equivalent` would return
     * true.
     */
    pub fn value_at_percentile(&self, percentile: f64) -> i64 {
        use std::cmp;

        // Truncate down to 100%
        let percentile = if percentile > 100.0 {
            100.0
        } else {
            percentile
        };

        // round to nearest
        let countAtPercentile = (((percentile / 100.0) * self.totalCount as f64) + 0.5) as i64;

        // Make sure we at least reach the first recorded entry
        let countAtPercentile = cmp::max(countAtPercentile, 1);

        let mut totalToCurrentIndex = 0i64;
        for i in 0..self.len() {
            totalToCurrentIndex += self[i].to_i64().unwrap();
            if totalToCurrentIndex >= countAtPercentile {
                let valueAtIndex = self.value_from_index(i);
                return if percentile == 0.0 {
                    self.lowest_equivalent(valueAtIndex)
                } else {
                    self.highest_equivalent(valueAtIndex)
                };
            }
        }

        0
    }

    /**
     * Get the percentile at and below a given value.
     *
     * The percentile returned is the percentile of values recorded in the histogram that are
     * smaller than or equivalent to the given value.
     *
     * Note that two values are "equivalent" in this statement if `self.equivalent` would return
     * true.
     */
    pub fn percentile_below(&self, value: i64) -> f64 {
        use std::cmp;

        if self.totalCount == 0 {
            return 100.0;
        }

        let targetIndex = cmp::min(self.indexOf(value), self.lastIndex() as isize) as usize;
        let totalToCurrentIndex: i64 =
            (0..(targetIndex + 1)).map(|i| self[i].to_i64().unwrap()).fold(0, |t, v| t + v);
        (100 * totalToCurrentIndex) as f64 / self.totalCount as f64
    }

    /**
     * Get the count of recorded values within a range of value levels (inclusive to within the
     * histogram's resolution).
     *
     * `lowValue` gives the lower value bound on the range for which to provide the recorded count.
     * Will be rounded down with `lowest_equivalent`. Similarly, `highValue` gives the higher value
     * bound on the range, and will be rounded up with `highest_equivalent`.
     *
     * Returns the total count of values recorded in the histogram within the value range that is
     * `>= lowest_equivalent(lowValue)` and `<= highest_equivalent(highValue)`. May fail if the
     * given values are out of bounds.
     */
    pub fn count_between(&self, lowValue: i64, highValue: i64) -> Result<i64, ()> {
        use std::cmp;
        let lowIndex = cmp::max(0, self.indexOf(lowValue)) as usize;
        let highIndex = cmp::min(self.indexOf(highValue), self.lastIndex() as isize) as usize;
        Ok((lowIndex..(highIndex + 1)).map(|i| self[i].to_i64().unwrap()).fold(0, |t, v| t + v))
    }

    /**
     * Get the count of recorded values at a specific value (to within the histogram resolution at
     * the value level).
     *
     * The count is cumputed across values recorded in the histogram that are within the value
     * range that is `>= lowest_equivalent(value)` and `<= highest_equivalent(value)`. May fail
     * if the given value is out of bounds.
     */
    pub fn count_at(&self, value: i64) -> Result<T, ()> {
        use std::cmp;
        Ok(self[cmp::min(cmp::max(0, self.indexOf(value)), self.lastIndex() as isize) as usize])
    }
}

// helpers

impl<T: num::Num> Histogram<T> {
    /**
     * Return the lowest (and therefore highest precision) bucket index that can represent the
     * value.
     */
    fn bucketIndexOf(&self, value: i64) -> isize {
        // Calculates the number of powers of two by which the value is greater than the biggest
        // value that fits in bucket 0. This is the bucket index since each successive bucket can
        // hold a value 2x greater. The mask maps small values to bucket 0.
        self.leadingZeroCountBase - (value | self.subBucketMask).leading_zeros() as isize
    }

    fn subBucketIndexOf(&self, value: i64, bucketIndex: isize) -> isize {
        // For bucketIndex 0, this is just value, so it may be anywhere in 0 to subBucketCount. For
        // other bucketIndex, this will always end up in the top half of subBucketCount: assume
        // that for some bucket k > 0, this calculation will yield a value in the bottom half of 0
        // to subBucketCount. Then, because of how buckets overlap, it would have also been in the
        // top half of bucket k-1, and therefore would have returned k-1 in bucketIndexOf(). Since
        // we would then shift it one fewer bits here, it would be twice as big, and therefore in
        // the top half of subBucketCount.
        // TODO: >>> ?
        (value >> (bucketIndex + self.unitMagnitude)) as isize
    }

    #[inline]
    fn valueFromLocation(&self, bucketIndex: isize, subBucketIndex: isize) -> i64 {
        (subBucketIndex as i64) << (bucketIndex + self.unitMagnitude)
    }

    pub fn bucketCount(&self) -> usize {
        self.bucketCount
    }

    pub fn value_from_index(&self, index: usize) -> i64 {
        let mut bucketIndex = (index >> self.subBucketHalfCountMagnitude) as isize - 1;
        let mut subBucketIndex =
            ((index & (self.subBucketHalfCount - 1)) + self.subBucketHalfCount) as isize;
        if bucketIndex < 0 {
            subBucketIndex -= self.subBucketHalfCount as isize;
            bucketIndex = 0;
        }
        self.valueFromLocation(bucketIndex, subBucketIndex)
    }

    /**
     * Get the size (in value units) of the range of values that are equivalent to the given value
     * within the histogram's resolution. Where "equivalent" means that value samples recorded for
     * any two equivalent values are counted in a common total count.
     */
    pub fn equivalent_range_len(&self, value: i64) -> i64 {
        let bucketIndex = self.bucketIndexOf(value);
        let subBucketIndex = self.subBucketIndexOf(value, bucketIndex);
        // calculate distance to next value
        1i64 <<
        (self.unitMagnitude +
         if subBucketIndex >= self.subBucketCount as isize {
            bucketIndex + 1
        } else {
            bucketIndex
        })
    }

    pub fn lastIndex(&self) -> usize {
        self.len() - 1
    }
}

// /**
//  * Indicate whether or not the histogram is capable of supporting auto-resize functionality.
//  * Note that this is an indication that enabling auto-resize by calling setAutoResize() is
//  * allowed, and NOT that the histogram will actually auto-resize. Use isAutoResize() to
//  * determine if the histogram is in auto-resize mode.
//  * @return autoResize setting
//  */
// public boolean supportsAutoResize() { return true; }

// TODO: copy methods
// TODO: subtract
// TODO: shift
// TODO: hash
// TODO: serialization
// TODO: encoding/decoding
// TODO: timestamps and tags
// TODO: textual output
