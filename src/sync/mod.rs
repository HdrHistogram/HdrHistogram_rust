//! Synchronized types that allow access to a `Histogram` from multiple threads.

use crate::errors::*;
use crate::{Counter, Histogram};
use std::borrow::Borrow;
use std::ops::{AddAssign, Deref, DerefMut};
use std::sync::{atomic, Arc, Condvar, Mutex};
use std::time;

/// A write-only handle to a [`SyncHistogram`].
///
/// This handle allows you to record samples from multiple threads, each with its own `Recorder`,
/// concurrently. Writes to a `Recorder` are wait-free and scalable except for when the
/// [`SyncHistogram`] initiates a _phase shift_. During a phase shift, the next write on each
/// associated `Recorder` merges its results into a shared [`Histogram`] that is then made
/// available to the [`SyncHistogram`] once the phase shift completes.
///
/// An idle `Recorder` will hold up a phase shift indefinitely, or until it times out (is using
/// [`SyncHistogram::refresh_timeout`]. If a `Recorder` will remain idle for extended periods of
/// time, it should call [`Recorder::idle`], which will tell the reader not to wait for this
/// particular writer.
///
/// If a `Recorder` is dropped, any samples not seen by the latest call to
/// [`SyncHistogram::refresh`] are lost. If you wish to ensure that _all_ samples are communicated
/// to the reader, use [`Recorder::synchronize`].
#[derive(Debug)]
pub struct Recorder<C: Counter> {
    local: Histogram<C>,
    shared: Arc<Shared<C>>,
    last_phase: usize,
}

// make it more ergonomic to record samples
impl<C: Counter> AddAssign<u64> for Recorder<C> {
    fn add_assign(&mut self, value: u64) {
        self.record(value).unwrap();
    }
}

impl<C: Counter> Clone for Recorder<C> {
    fn clone(&self) -> Self {
        // reader will have to wait for one more recorder
        {
            let mut truth = self.shared.truth.lock().unwrap();
            truth.recorders += 1;
        }

        // new recorder should start with an empty histogram
        let mut h = self.local.clone();
        h.clear();

        // new recorder starts at the same phase as we do
        Recorder {
            local: h,
            shared: self.shared.clone(),
            last_phase: self.last_phase,
        }
    }
}

impl<C: Counter> Drop for Recorder<C> {
    fn drop(&mut self) {
        // we'll need to decrement the # of recorders
        {
            let mut truth = self.shared.truth.lock().unwrap();
            truth.recorders -= 1;

            // we have to be careful; we _may_ already have incremented for the ongoing phase!
            if truth.phased != 0 {
                // note that we _have_ to do this load under the lock, otherwise a reader may
                // increment the phase after we read, and before we take the lock above!
                let phase = self.shared.phase.load(atomic::Ordering::Acquire);
                if phase == self.last_phase {
                    // we contributed to the current phased value -- undo that
                    truth.phased -= 1;
                }
            }

            // by decrementing, we _may_ also finish the phase
            if truth.recorders == truth.phased {
                self.shared.all_phased.notify_one();
            }
        }
    }
}

#[derive(Debug)]
struct Critical<C: Counter> {
    /// Will be Some whenever the Histogram is in the process of being phased.
    merged: Option<Histogram<C>>,
    recorders: usize,
    phased: usize,
}

#[derive(Debug)]
struct Shared<C: Counter> {
    truth: Mutex<Critical<C>>,
    all_phased: Condvar,
    phase_change: Condvar,
    phase: atomic::AtomicUsize,
}

/// This guard denotes that a [`Recorder`] is currently idle, and should not be waited on by a
/// [`SyncHistogram`] phase-shift.
pub struct IdleRecorderGuard<'a, C: Counter>(&'a mut Recorder<C>);

impl<'a, C: Counter> Drop for IdleRecorderGuard<'a, C> {
    fn drop(&mut self) {
        let mut phased = false;
        // the Recorder is no longer idle, so the reader has to wait for us again
        // this basically means re-incrementing .recorders
        {
            let mut crit = self.0.shared.truth.lock().unwrap();
            crit.recorders += 1;

            // if there's a phase shift ongoing, we need to take part in that
            if let Some(ref mut h) = crit.merged {
                if !self.0.local.is_empty() {
                    h.add(&self.0.local)
                        .expect("TODO: failed to merge histogram");
                    phased = true;
                }
                crit.phased += 1;
            }

            // we are now up-to-date with the current phase
            // NOTE: we have to load this during the lock so we don't read too early/late
            self.0.last_phase = self.0.shared.phase.load(atomic::Ordering::Acquire);

            // NOTE: we cannot have finished the phase, since it cannot have been waiting for us.
        }

        // if we phase shifted, clear the changes we just merged
        if phased {
            self.0.local.clear();
        }
    }
}

impl<C: Counter> Recorder<C> {
    fn with_hist<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Histogram<C>) -> R,
    {
        let r = f(&mut self.local);
        let phase = self.shared.phase.load(atomic::Ordering::Acquire);
        if phase != self.last_phase {
            {
                let mut crit = self.shared.truth.lock().unwrap();
                if !self.local.is_empty() {
                    // it could be that the reader timed out, in which case we'll have to wait
                    if let Some(ref mut m) = crit.merged {
                        m.add(&self.local).expect("TODO: failed to merge histogram");
                    }
                }
                crit.phased += 1;
                if crit.phased == crit.recorders {
                    self.shared.all_phased.notify_one();
                }
            }
            self.last_phase = phase;
            self.local.clear();
        }
        r
    }

    /// Call this method if the Recorder will be idle for a while.
    ///
    /// Until the returned guard is dropped, the associated [`SyncHistogram`] will not wait for
    /// this recorder on a phase shift.
    pub fn idle(&mut self) -> IdleRecorderGuard<C> {
        let phase;
        {
            let mut crit = self.shared.truth.lock().unwrap();
            phase = self.shared.phase.load(atomic::Ordering::Acquire);
            if phase != self.last_phase {
                // if we happen to be in a phase shift, sync our outstanding changes
                if !self.local.is_empty() {
                    if let Some(ref mut m) = crit.merged {
                        m.add(&self.local).expect("TODO: failed to merge histogram");
                    }
                }
            // NOTE; we do _not_ increment .phased, since we're about to decrement .recorders
            } else if crit.phased != 0 {
                // we must have already performed this phase shift, and so .phased includes our +1
                // since we're about to decrement .recorders, we need to also undo that
                crit.phased -= 1;
            }

            // make the reader no longer wait for us
            crit.recorders -= 1;

            // we may have just finished the phase!
            if crit.phased == crit.recorders {
                self.shared.all_phased.notify_one();
            }
        }

        // if we phase shifted, clear the changes we just merged
        if phase != self.last_phase {
            self.last_phase = phase;
            self.local.clear();
        }

        IdleRecorderGuard(self)
    }

    /// This blocking call will return only once any locally-held samples have been made visible to
    /// the reader (the associated [`SyncHistogram`]).
    ///
    /// You may wish to call this before a `Recorder` goes out of scope to ensure that none of its
    /// samples are lost.
    pub fn synchronize(&mut self) {
        if self.local.is_empty() {
            return;
        }

        {
            let mut crit = self.shared.truth.lock().unwrap();
            loop {
                if let Some(ref mut m) = crit.merged {
                    m.add(&self.local).expect("TODO: failed to merge histogram");

                    // NOTE: we _may_ still be in a phase we've already synchronized;
                    // we should only bump .phased if we haven't already!
                    let phase = self.shared.phase.load(atomic::Ordering::Acquire);
                    if phase != self.last_phase {
                        crit.phased += 1;
                        self.last_phase = phase;
                        if crit.phased == crit.recorders {
                            self.shared.all_phased.notify_one();
                        }
                    }
                    break;
                }
                crit = self.shared.phase_change.wait(crit).unwrap();
            }
        }
        self.local.clear();
    }

    /// See [`Histogram::add`].
    pub fn add<B: Borrow<Histogram<C>>>(&mut self, source: B) -> Result<(), AdditionError> {
        self.with_hist(move |h| h.add(source))
    }

    /// See [`Histogram::add_correct`].
    pub fn add_correct<B: Borrow<Histogram<C>>>(
        &mut self,
        source: B,
        interval: u64,
    ) -> Result<(), RecordError> {
        self.with_hist(move |h| h.add_correct(source, interval))
    }

    /// See [`Histogram::subtract`].
    pub fn subtract<B: Borrow<Histogram<C>>>(
        &mut self,
        subtrahend: B,
    ) -> Result<(), SubtractionError> {
        self.with_hist(move |h| h.subtract(subtrahend))
    }

    /// See [`Histogram::record`].
    pub fn record(&mut self, value: u64) -> Result<(), RecordError> {
        self.with_hist(move |h| h.record(value))
    }

    /// See [`Histogram::saturating_record`].
    pub fn saturating_record(&mut self, value: u64) {
        self.with_hist(move |h| h.saturating_record(value))
    }

    /// See [`Histogram::record_n`].
    pub fn record_n(&mut self, value: u64, count: C) -> Result<(), RecordError> {
        self.with_hist(move |h| h.record_n(value, count))
    }

    /// See [`Histogram::saturating_record_n`].
    pub fn saturating_record_n(&mut self, value: u64, count: C) {
        self.with_hist(move |h| h.saturating_record_n(value, count))
    }

    /// See [`Histogram::record_correct`].
    pub fn record_correct(&mut self, value: u64, interval: u64) -> Result<(), RecordError> {
        self.with_hist(move |h| h.record_correct(value, interval))
    }

    /// See [`Histogram::record_n_correct`].
    pub fn record_n_correct(
        &mut self,
        value: u64,
        count: C,
        interval: u64,
    ) -> Result<(), RecordError> {
        self.with_hist(move |h| h.record_n_correct(value, count, interval))
    }
}

/// A `Histogram` that can be written to by multiple threads concurrently.
///
/// Each writer thread should have a [`Recorder`], which allows it to record new samples without
/// synchronization. New recorded samples are made available through this histogram by calling
/// [`SyncHistogram::refresh`], which blocks until it has synchronized with every recorder.
pub struct SyncHistogram<C: Counter> {
    /// Will be None during a phase shift.
    merged: Option<Histogram<C>>,
    shared: Arc<Shared<C>>,
}

impl<C: Counter> SyncHistogram<C> {
    fn refresh_inner(&mut self, timeout: Option<time::Duration>) {
        let end = timeout.map(|dur| time::Instant::now() + dur);

        // time to start a phase change
        let mut truth = self.shared.truth.lock().unwrap();

        // provide histogram for writers to merge into
        truth.merged = self.merged.take();
        assert_eq!(truth.phased, 0);

        // tell writers to phase
        let _ = self.shared.phase.fetch_add(1, atomic::Ordering::AcqRel);

        // wait for writers to all have phased
        while truth.phased != truth.recorders {
            if let Some(end) = end {
                let now = time::Instant::now();
                if now > end {
                    truth = self.shared.truth.lock().unwrap();
                    break;
                }

                let (t, wtr) = self
                    .shared
                    .all_phased
                    .wait_timeout(truth, end - now)
                    .unwrap();
                truth = t;
                if wtr.timed_out() {
                    break;
                }
            } else {
                self.shared.phase_change.notify_all();
                truth = self.shared.all_phased.wait(truth).unwrap();
            }
        }

        // take the merged histogram back out
        self.merged = truth.merged.take();

        // reset for next phase
        truth.phased = 0;

        self.shared.phase_change.notify_all();
    }

    /// Block until writes from all [`Recorder`] instances for this histogram have been
    /// incorporated.
    pub fn refresh(&mut self) {
        self.refresh_inner(None)
    }

    /// Block until writes from all [`Recorder`] instances for this histogram have been
    /// incorporated, or until the given amount of time has passed.
    pub fn refresh_timeout(&mut self, timeout: time::Duration) {
        self.refresh_inner(Some(timeout))
    }

    /// Obtain another multi-threaded writer for this histogram.
    ///
    /// Note that writes made to the `Recorder` will not be visible until the next call to
    /// [`SyncHistogram::refresh`].
    pub fn recorder(&self) -> Recorder<C> {
        // we will have to wait for one more recorder
        {
            let mut truth = self.shared.truth.lock().unwrap();
            truth.recorders += 1;
        }

        // new recorder should start with an empty histogram
        let mut h = self
            .merged
            .as_ref()
            .expect("local histogram None outside phase shift")
            .clone();
        h.clear();

        // new recorder starts at the current phase
        Recorder {
            local: h,
            shared: self.shared.clone(),
            last_phase: self.shared.phase.load(atomic::Ordering::Acquire),
        }
    }
}

impl<C: Counter> From<Histogram<C>> for SyncHistogram<C> {
    fn from(h: Histogram<C>) -> Self {
        SyncHistogram {
            merged: Some(h),
            shared: Arc::new(Shared {
                truth: Mutex::new(Critical {
                    merged: None,
                    recorders: 0,
                    phased: 0,
                }),
                all_phased: Condvar::new(),
                phase_change: Condvar::new(),
                phase: atomic::AtomicUsize::new(0),
            }),
        }
    }
}

impl<C: Counter> Deref for SyncHistogram<C> {
    type Target = Histogram<C>;
    fn deref(&self) -> &Self::Target {
        self.merged
            .as_ref()
            .expect("local histogram None outside phase shift")
    }
}

impl<C: Counter> DerefMut for SyncHistogram<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.merged
            .as_mut()
            .expect("local histogram None outside phase shift")
    }
}
