//! Interval log parsing and writing.
//!
//! Interval logs, as handled by the Java implementation's `HistogramLogWriter`,
//! `HistogramLogReader`, and `HistogramLogProcessor`, are a way to record a sequence of histograms
//! over time. Suppose you were running a load test for an hour: you might want to record a
//! histogram per second or minute so that you could correlate measurements with behavior you might
//! see in logs, etc.
//!
//! An interval log contains some initial metadata, then a sequence of histograms, each with some
//! additional metadata (timestamps, etc).
//!
//! To parse a log, see `IntervalLogIterator`. To write a log, see `IntervalLogHeaderWriter`.

extern crate base64;

use std::{io, ops, str};
use std::fmt::Write;

use nom::{double, line_ending, not_line_ending, IResult};

use super::super::{Counter, Histogram};
use super::Serializer;

/// Start writing an interval log.
///
/// This type only allows writing comments and headers. Once you're done writing those things, use
/// `into_log_writer()` to convert this into an `IntervalLogWriter`.
pub struct IntervalLogHeaderWriter<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> {
    internal_writer: InternalLogWriter<'a, 'b, W, S>,
}

impl<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> IntervalLogHeaderWriter<'a, 'b, W, S> {
    /// Create a new log writer that writes to `writer` and serializes histograms with `serializer`.
    pub fn new(writer: &'a mut W, serializer: &'b mut S) -> IntervalLogHeaderWriter<'a, 'b, W, S> {
        IntervalLogHeaderWriter {
            internal_writer: InternalLogWriter {
                writer,
                serializer,
                text_buf: String::new(),
                serialize_buf: Vec::new(),
            },
        }
    }

    /// Add a comment line.
    pub fn write_comment(&mut self, s: &str) -> io::Result<()> {
        self.internal_writer.write_comment(s)
    }

    /// Once you're finished with headers, convert this into a log writer so you can write interval
    /// histograms.
    pub fn into_log_writer(self) -> IntervalLogWriter<'a, 'b, W, S> {
        IntervalLogWriter {
            internal_writer: self.internal_writer,
        }
    }
}

/// Writes interval histograms in an interval log.
///
/// This isn't created directly; start with an `IntervalLogHeaderWriter`.
pub struct IntervalLogWriter<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> {
    internal_writer: InternalLogWriter<'a, 'b, W, S>,
}

impl<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> IntervalLogWriter<'a, 'b, W, S> {
    /// Add a comment line.
    pub fn write_comment(&mut self, s: &str) -> io::Result<()> {
        self.internal_writer.write_comment(s)
    }

    /// Write an interval histogram.
    ///
    /// `start_timestamp` is the time since the epoch in seconds. If you're using a StartTime or
    /// BaseTime offset, you should instead use a delta since that time.
    /// `duration` is the duration of the interval in seconds.
    /// `tag` is an optional tag for this histogram.
    /// `max_value_divisor` is used to scale down the max value to something that may be more human
    /// readable. The max value in the log is only for human consumption, so you might prefer to
    /// divide by 10^9 to turn nanoseconds into fractional seconds, for instance.
    pub fn write_histogram<T: Counter>(
        &mut self,
        h: &Histogram<T>,
        start_timestamp: f64,
        duration: f64,
        tag: Option<Tag>,
        max_value_divisor: f64,
    ) -> Result<(), IntervalLogWriterError<S::SerializeError>> {
        self.internal_writer
            .write_histogram(h, start_timestamp, duration, tag, max_value_divisor)
    }
}

/// Errors that can occur while writing a log.
#[derive(Debug)]
pub enum IntervalLogWriterError<E> {
    /// Histogram serialization failed.
    SerializeError(E),
    /// An i/o error occurred.
    IoError(io::ErrorKind),
}

impl<E> From<io::Error> for IntervalLogWriterError<E> {
    fn from(e: io::Error) -> Self {
        IntervalLogWriterError::IoError(e.kind())
    }
}

/// Write interval logs.
struct InternalLogWriter<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> {
    writer: &'a mut W,
    serializer: &'b mut S,
    text_buf: String,
    serialize_buf: Vec<u8>,
}

impl<'a, 'b, W: 'a + io::Write, S: 'b + Serializer> InternalLogWriter<'a, 'b, W, S> {
    fn write_comment(&mut self, s: &str) -> io::Result<()> {
        write!(self.writer, "#{}\n", s)
    }

    fn write_histogram<T: Counter>(
        &mut self,
        h: &Histogram<T>,
        start_timestamp: f64,
        duration: f64,
        tag: Option<Tag>,
        max_value_divisor: f64,
    ) -> Result<(), IntervalLogWriterError<S::SerializeError>> {
        self.serialize_buf.clear();
        self.text_buf.clear();

        if let Some(Tag(s)) = tag {
            write!(self.text_buf, "Tag={},", &s).expect("Writes to a String can't fail");
        }

        write!(
            self.writer,
            "{}{:.3},{:.3},{:.3},",
            self.text_buf,
            start_timestamp,
            duration,
            h.max() as f64 / max_value_divisor // because the Java impl does it this way
        )?;

        self.text_buf.clear();
        let _len = self.serializer
            .serialize(h, &mut self.serialize_buf)
            .map_err(|e| IntervalLogWriterError::SerializeError(e))?;
        base64::encode_config_buf(&self.serialize_buf, base64::STANDARD, &mut self.text_buf);

        self.writer.write_all(self.text_buf.as_bytes())?;
        self.writer.write_all(b"\n")?;

        Ok(())
    }
}

/// A tag for an interval histogram.
///
/// Tags are just `str`s that do not contain a few disallowed characters: ',', '\r', '\n', and ' '.
///
/// To get the wrapped `str` back out, use `as_str()` or the `Deref<str>` implementation
/// (`&some_tag`).
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Tag<'a>(pub &'a str);

impl<'a> Tag<'a> {
    /// Create a new Tag.
    ///
    /// If a disallowed character is present, this will return `None`.
    pub fn new(s: &'a str) -> Option<Tag<'a>> {
        if s.chars()
            .any(|c| c == ',' || c == '\r' || c == '\n' || c == ' ')
        {
            None
        } else {
            Some(Tag(s))
        }
    }

    /// Returns the tag contents as a str.
    pub fn as_str(&self) -> &'a str {
        self.0
    }
}

impl<'a> ops::Deref for Tag<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

/// An individual interval histogram parsed from an interval log.
#[derive(PartialEq, Debug)]
pub struct IntervalLogHistogram<'a> {
    tag: Option<Tag<'a>>,
    start_timestamp: f64,
    duration: f64,
    max: f64,
    encoded_histogram: &'a str,
}

impl<'a> IntervalLogHistogram<'a> {
    /// Tag, if any is present.
    pub fn tag(&self) -> Option<Tag<'a>> {
        self.tag
    }

    /// Timestamp of the start of the interval in seconds.
    ///
    /// The timestamp may be absolute vs the epoch, or there may be a `StartTime` or `BaseTime` for
    /// the log, in which case you may wish to consider this number as a delta vs those timestamps..
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
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Base64-encoded serialized histogram.
    ///
    /// If you need the deserialized histogram, base64-decode and use a `Deserializer` on the
    /// resulting bytes.
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

/// Errors that occur when parsing an interval log.
#[derive(Debug, PartialEq)]
pub enum LogIteratorError {
    /// Parsing failed
    ParseError {
        /// Offset in the input where the failed parse started
        offset: usize,
    },
}

/// Parse interval logs.
///
/// This iterator exposes each item (excluding comments and other information-free lines). See
/// `LogEntry`.
///
/// Because histogram deserialization is deferred, parsing logs is fast. (See the `interval_log`
/// benchmark if you wish to see how it does on your hardware. As a baseline, parsing a log of 1000
/// random histograms of 10,000 values each takes 8ms total on an E5-1650v3.)
///
/// Deferring deserialization is handy because it allows you to cheaply navigate the log to find
/// the records you care about (e.g. ones in a certain time range, or with a certain tag) without
/// doing all the allocation, etc, of deserialization.
///
/// This parses from a slice representing the complete file because it made implementation easier
/// (and also supports mmap'd files for maximum parsing speed). If parsing from a `Read` is
/// important for your use case, open an issue about it.
///
/// # Examples
///
/// Parse a single interval from a log.
///
/// ```
/// use hdrsample::serialization::interval_log;
/// // two newline-separated log lines: a comment, then an interval
/// let log = b"#I'm a comment\nTag=t,0.127,1.007,2.769,base64EncodedHisto\n";
///
/// let mut iter = interval_log::IntervalLogIterator::new(&log[..]);
///
/// match iter.next().unwrap().unwrap() {
///     interval_log::LogEntry::Interval(h) => {
///         assert_eq!(0.127, h.start_timestamp());
///     }
///     _ => panic!()
/// }
///
/// assert_eq!(None, iter.next());
/// ```
///
/// Skip logs that started before 3 seconds.
///
/// ```
/// use hdrsample::serialization::interval_log;
/// let mut log = Vec::new();
/// log.extend_from_slice(b"#I'm a comment\n");
/// log.extend_from_slice(b"Tag=a,0.123,1.007,2.769,base64EncodedHisto\n");
/// log.extend_from_slice(b"1.456,1.007,2.769,base64EncodedHisto\n");
/// log.extend_from_slice(b"3.789,1.007,2.769,base64EncodedHisto\n");
/// log.extend_from_slice(b"Tag=b,4.123,1.007,2.769,base64EncodedHisto\n");
/// log.extend_from_slice(b"5.456,1.007,2.769,base64EncodedHisto\n");
/// log.extend_from_slice(b"#Another comment\n");
///
/// let iter = interval_log::IntervalLogIterator::new(&log);
///
/// let count = iter.map(|r| r.unwrap())
///     .filter_map(|e| match e {
///         interval_log::LogEntry::Interval(ilh) => Some(ilh),
///          _ => None
///     })
///     .filter(|ilh| ilh.start_timestamp() >= 3.0)
///     .count();
///
/// assert_eq!(3, count);
/// ```
pub struct IntervalLogIterator<'a> {
    orig_len: usize,
    input: &'a [u8],
}

impl<'a> IntervalLogIterator<'a> {
    /// Create a new iterator from the UTF-8 bytes of an interval log.
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
            map!(
                map_res!(
                    map!(pair!(tag!("Tag="), take_until_and_consume!(",")), |p| p.1),
                    str::from_utf8),
                |s| Tag(s))) >>
        start_timestamp: double >>
        char!(',') >>
        duration: double >>
        char!(',') >>
        max: double >>
        char!(',') >>
        encoded_histogram: map_res!(not_line_ending, str::from_utf8) >>
        line_ending >>
        (LogEntry::Interval(IntervalLogHistogram {
            tag,
            start_timestamp,
            duration,
            max,
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

#[cfg(test)]
mod tests;
