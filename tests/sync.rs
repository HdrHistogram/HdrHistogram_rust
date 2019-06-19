//! Tests from RecorderTest.java

use hdrhistogram::{sync::SyncHistogram, Histogram};
use std::sync::Arc;
use std::{thread, time};

const TRACKABLE_MAX: u64 = 3600 * 1000 * 1000;
// Store up to 2 * 10^3 in single-unit precision. Can be 5 at most.
const SIGFIG: u8 = 3;
const TEST_VALUE_LEVEL: u64 = 4;

#[test]
fn record_through() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();
    h.record(TEST_VALUE_LEVEL).unwrap();
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), 1);
    assert_eq!(h.len(), 1);
}

#[test]
fn recorder_drop() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();
    let mut r = h.recorder();
    let jh = thread::spawn(move || {
        r += TEST_VALUE_LEVEL;
    });
    h.phase();
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), 1);
    assert_eq!(h.len(), 1);
    jh.join().unwrap();
}

#[test]
fn record_nodrop() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();
    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut r = h.recorder();
    let b = Arc::clone(&barrier);
    let jh = thread::spawn(move || {
        r += TEST_VALUE_LEVEL;
        b.wait();
    });
    h.phase();
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), 1);
    assert_eq!(h.len(), 1);
    barrier.wait();
    jh.join().unwrap();
}

#[test]
fn phase_timeout() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();
    h.record(TEST_VALUE_LEVEL).unwrap();
    let mut r = h.recorder();
    r += TEST_VALUE_LEVEL;
    h.phase_timeout(time::Duration::from_millis(100));

    // second TEST_VALUE_LEVEL should not be visible
    // since no record happened after phase()
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), 1);
    assert_eq!(h.len(), 1);
}

#[test]
fn recorder_synchronize() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();

    let barrier = Arc::new(std::sync::Barrier::new(2));
    let mut r = h.recorder();
    let b = Arc::clone(&barrier);
    let jh = thread::spawn(move || {
        let n = 10_000;
        for _ in 0..n {
            r += TEST_VALUE_LEVEL;
        }
        // one of the writes above will unblock the reader's first phase
        // the 1st barrier below ensures that the reader's second phase isn't passed by a write too
        // the 2nd barrier below ensures that there is at least one write to synchronize,
        // and that that write doesn't wake up the 2nd phase
        b.wait();
        r += TEST_VALUE_LEVEL;
        b.wait();
        r.synchronize();
        n + 1
    });
    h.phase(); // this should be unblocked by one of the writes
    barrier.wait();
    barrier.wait();
    h.phase(); // this will be unblocked by, and will unblock, the synchronize
    let n = jh.join().unwrap();
    h.phase(); // no recorders, so we should be fine

    assert_eq!(h.count_at(TEST_VALUE_LEVEL), n);
    assert_eq!(h.len(), n);
}

#[test]
fn phase_no_wait_after_drop() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();

    {
        let _ = h.recorder();
    }
    h.phase(); // this shouldn't block since the recorder went away

    assert_eq!(h.len(), 0);
}

#[test]
fn concurrent_writes() {
    let mut h: SyncHistogram<_> = Histogram::<u64>::new_with_max(TRACKABLE_MAX, SIGFIG)
        .unwrap()
        .into();
    h.record(TEST_VALUE_LEVEL).unwrap();
    let mut r = h.recorder();
    r += TEST_VALUE_LEVEL;
    h.phase_timeout(time::Duration::from_millis(100));

    // second TEST_VALUE_LEVEL should not be visible
    // since no record happened after phase()
    assert_eq!(h.count_at(TEST_VALUE_LEVEL), 1);
    assert_eq!(h.len(), 1);
}
