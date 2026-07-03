//! Proves the chunked skip-scan in `value_at_quantile` returns byte-identical
//! results to a plain linear prefix-sum scan. Exercises the skip, crossing-chunk,
//! and tail (`remainder`) paths — including a case whose crossing element is pinned
//! into the tail — and covers every `Counter` width (u8/u16/u32/u64), since the scan
//! is generic over `T: Counter`.

use crate::{Counter, Histogram};

/// The pre-optimization linear scan, kept here as the correctness oracle. Generic
/// over `T` so it mirrors the real (generic) scan for every counter width.
fn linear_reference<T: Counter>(h: &Histogram<T>, quantile: f64) -> u64 {
    let quantile = if quantile > 1.0 { 1.0 } else { quantile };
    let mut count_at_quantile = (quantile * h.total_count as f64).ceil() as u64;
    if count_at_quantile == 0 {
        count_at_quantile = 1;
    }
    let mut total: u64 = 0;
    for (i, count) in h.counts.iter().enumerate() {
        total += count.as_u64();
        if total >= count_at_quantile {
            let value_at_index = h.value_for(i);
            return if quantile == 0.0 {
                h.lowest_equivalent(value_at_index)
            } else {
                h.highest_equivalent(value_at_index)
            };
        }
    }
    0
}

/// Deterministic xorshift so the test needs no external rand dependency.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn assert_parity<T: Counter>(h: &Histogram<T>) {
    // Fine sweep including both edges; the interior points land at assorted
    // cumulative-count boundaries, exercising skip and crossing-chunk paths.
    let mut q = 0.0;
    while q <= 1.0 {
        assert_eq!(
            h.value_at_quantile(q),
            linear_reference(h, q),
            "chunked != linear at quantile {}",
            q
        );
        q += 0.0013;
    }
    for &q in &[0.0, 1.0] {
        assert_eq!(h.value_at_quantile(q), linear_reference(h, q));
    }
}

#[test]
fn chunked_scan_matches_linear_all_counter_widths() {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let configs = [
        (1u64, 100_000u64, 0u8), // sig=0 → sub_bucket_half_count 1 → len not %8: exercises the tail
        (1, 100, 1),             // tiny counts[]
        (1, 1000, 2),            // small
        (1, 3_600_000_000, 3),   // production-sized
        (1000, 100_000_000, 3),  // offset low bound, wide range
    ];

    for &(low, high, sig) in &configs {
        // Every counter width — the scan is generic over T: Counter.
        let mut h8 = Histogram::<u8>::new_with_bounds(low, high, sig).unwrap();
        let mut h16 = Histogram::<u16>::new_with_bounds(low, high, sig).unwrap();
        let mut h32 = Histogram::<u32>::new_with_bounds(low, high, sig).unwrap();
        let mut h64 = Histogram::<u64>::new_with_bounds(low, high, sig).unwrap();
        for _ in 0..4000 {
            let v = low + xorshift(&mut state) % (high - low + 1);
            // saturating_record keeps small counter types from overflowing while
            // still populating clusters/gaps across counts[].
            h8.saturating_record(v);
            h16.saturating_record(v);
            h32.saturating_record(v);
            h64.saturating_record(v);
        }
        assert_parity(&h8);
        assert_parity(&h16);
        assert_parity(&h32);
        assert_parity(&h64);
    }
}

#[test]
fn chunked_scan_crossing_in_tail() {
    // The chunk loop consumes counts[] in blocks of 8; the remainder() tail runs only
    // when counts.len() is not a multiple of 8. For sig >= 1 that never happens
    // (sub_bucket_half_count is a power of two >= 16, so counts.len() is always a
    // multiple of 8 and the tail is unreachable). A sig=0 histogram has
    // sub_bucket_half_count == 1, giving a non-multiple length — the only way to
    // force a crossing element into the tail loop.
    let high = 100_000u64;
    let mut h = Histogram::<u64>::new_with_bounds(1, high, 0).unwrap();
    let n = h.counts.len();
    assert_ne!(n % 8, 0, "sig=0 geometry should leave a non-empty tail");
    h.record(1).unwrap();
    h.record(high).unwrap();
    // The top value's index lands in the tail region [ (n/8)*8, n ).
    assert!(
        h.index_for(high).unwrap() >= (n / 8) * 8,
        "crossing element is not in the tail"
    );

    for &q in &[0.0, 0.5, 1.0] {
        assert_eq!(h.value_at_quantile(q), linear_reference(&h, q));
    }
}
