use std::str;

use nom::{double, line_ending, not_line_ending, IResult};

#[derive(PartialEq, Debug)]
pub struct IntervalLogHistogram<'a> {
    tag: Option<&'a str>,
    start_timestamp: f64,
    duration: f64,
    max_value: f64,
    encoded_histogram: &'a str,
}

impl<'a> IntervalLogHistogram<'a> {
    /// Tag, if any is present.
    pub fn tag(&self) -> Option<&'a str> {
        self.tag
    }

    /// Timestamp of the start of the interval in seconds.
    ///
    /// The timestamp may be absolute vs the epoch, or there may be a StartTime or BaseTime for the
    /// log.
    pub fn start_timestamp(&self) -> f64 {
        self.start_timestamp
    }

    /// Duration of the interval in seconds.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// Max value in the encoded histogram
    ///
    /// This max value is the max of the histogram divided by some scaling factor (which may be
    /// 1.0).
    pub fn max_value(&self) -> f64 {
        self.max_value
    }

    /// Base64-encoded serialized histogram.
    ///
    /// If you need the deserialized histogram, use a `Deserializer.
    ///
    /// Histograms are left in their original encoding to make parsing each log entry very cheap.
    /// One usage pattern is to navigate to a certain point in the log and only deserialize a few
    /// interesting histograms, so it would be inefficient to deserialize them at log parse time..
    pub fn encoded_histogram(&self) -> &'a str {
        self.encoded_histogram
    }
}

#[derive(PartialEq, Debug)]
/// Represents one non-comment line in an interval log.
///
/// One thing to note is that the way your interval timestamps work can vary. If your log was
/// written with a StartTime or BaseTime, that metadata will appear in header comments, and that
/// will be represented by the iterator providing the corresponding variants here. The presence
/// of those timestamps will affect how you should interpret the timestamps for individual
/// intervals.
pub enum LogEntry<'a> {
    /// Logs may include a StartTime. If present, it represents seconds since the epoch.
    StartTime(f64),
    /// Logs may include a BaseTime. If present, it represents seconds since the epoch.
    BaseTime(f64),
    /// An individual interval histogram.
    Interval(IntervalLogHistogram<'a>),
}

#[derive(Debug, PartialEq)]
pub enum LogIteratorError {
    ParseError { offset: usize },
}

/// Parse interval logs.
///
/// Interval logs, as handled by the Java implementation's `HistogramLogWriter`,
/// `HistogramLogReader`, and `HistogramLogProcessor`, are a way to record a sequence of histograms
/// over time. Suppose you were running a load test for an hour: you might want to record a
/// histogram per second or minute so that you could correlate measurements with behavior you might
/// see in logs, etc.
///
/// An interval log contains some initial metadata, then a sequence of histograms, each with some
/// additional metadata (timestamps, etc). This iterator exposes each item (excluding comments and
/// other information-free lines). See `LogEntry`.
///
/// This parses from a slice representing the complete file because it made implementation easier
/// (and also supports mmap'd files for maximum parsing speed). If parsing from a `Read` is
/// important for your use case, open an issue about it.
///
/// # Examples
///
/// ```
/// use hdrsample::serialization;
/// // two newline-separated log lines: a comment, then an interval
/// let log = b"#I'm a comment\nTag=t,0.127,1.007,2.769,base64EncodedHisto\n";
///
/// let mut iter = serialization::IntervalLogIterator::new(&log[..]);
///
/// match iter.next().unwrap().unwrap() {
///     serialization::LogEntry::Interval(h) => {
///         assert_eq!(0.127, h.start_timestamp());
///     }
///     _ => panic!()
/// }
///
/// assert_eq!(None, iter.next());
/// ```
pub struct IntervalLogIterator<'a> {
    orig_len: usize,
    input: &'a [u8],
}

impl<'a> IntervalLogIterator<'a> {
    /// Create a new iterator from the bytes of an interval log.
    pub fn new(input: &'a [u8]) -> IntervalLogIterator<'a> {
        IntervalLogIterator {
            orig_len: input.len(),
            input,
        }
    }
}

impl<'a> Iterator for IntervalLogIterator<'a> {
    type Item = Result<LogEntry<'a>, LogIteratorError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Look for magic comments first otherwise they will get matched by the simple comment
        // parser
        loop {
            if self.input.is_empty() {
                return None;
            }

            if let IResult::Done(rest, e) = log_entry(self.input) {
                self.input = rest;
                return Some(Ok(e));
            }

            // it wasn't a log entry; try parsing a comment

            let ignored_line_result = ignored_line(self.input);
            match ignored_line_result {
                IResult::Done(rest, _) => {
                    self.input = rest;
                    continue;
                }
                _ => {
                    return Some(Err(LogIteratorError::ParseError {
                        offset: self.orig_len - self.input.len(),
                    }));
                }
            }
        }
    }
}

named!(start_time<&[u8], LogEntry>,
    do_parse!(
        tag!("#[StartTime: ") >>
        n: double >>
        char!(' ') >>
        not_line_ending >>
        line_ending >>
        (LogEntry::StartTime(n))
));

named!(base_time<&[u8], LogEntry>,
    do_parse!(
        tag!("#[BaseTime: ") >>
        n: double >>
        char!(' ') >>
        not_line_ending >>
        line_ending >>
        (LogEntry::BaseTime(n))
));

named!(interval_hist<&[u8], LogEntry>,
    do_parse!(
        tag: opt!(
            map_res!(map!(pair!(tag!("Tag="), take_until_and_consume!(",")), |p| p.1),
             str::from_utf8)) >>
        start_timestamp: double >>
        char!(',') >>
        duration: double >>
        char!(',') >>
        max_value: double >>
        char!(',') >>
        encoded_histogram: map_res!(not_line_ending, str::from_utf8) >>
        line_ending >>
        (LogEntry::Interval(IntervalLogHistogram {
            tag,
            start_timestamp,
            duration,
            max_value,
            encoded_histogram
        }))
    )
);

named!(log_entry<&[u8], LogEntry>, alt_complete!(start_time | base_time | interval_hist));

named!(comment_line<&[u8], ()>,
    do_parse!(tag!("#") >> not_line_ending >> line_ending >> (()))
);

named!(legend<&[u8], ()>,
    do_parse!(tag!("\"StartTimestamp\"") >> not_line_ending >> line_ending >> (()))
);

named!(ignored_line<&[u8], ()>, alt!(comment_line | legend));

#[path = "interval_log_tests.rs"]
#[cfg(test)]
mod interval_log_tests;
