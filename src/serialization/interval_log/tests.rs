use std::time;
use std::ops::Add;

use super::super::super::*;
use super::super::*;
use super::*;

#[test]
fn write_header_comment() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let _ = IntervalLogWriterBuilder::new()
        .add_comment("foo")
        .begin_log_with(&mut buf, &mut serializer)
        .unwrap();

    assert_eq!(&b"#foo\n"[..], &buf[..]);
}

#[test]
fn write_header_then_interval_comment() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    {
        let mut log_writer = IntervalLogWriterBuilder::new()
            .add_comment("foo")
            .add_comment("bar")
            .begin_log_with(&mut buf, &mut serializer)
            .unwrap();
        log_writer.write_comment("baz").unwrap();
    }

    assert_eq!("#foo\n#bar\n#baz\n", str::from_utf8(&buf[..]).unwrap());
}

#[test]
fn write_headers_multiple_times_only_last_is_used() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    {
        let _ = IntervalLogWriterBuilder::new()
            .with_start_time(system_time_after_epoch(10, 0))
            .with_base_time(system_time_after_epoch(20, 0))
            .with_start_time(system_time_after_epoch(100, 0))
            .with_base_time(system_time_after_epoch(200, 0))
            .with_max_value_divisor(1_000.0)
            .with_max_value_divisor(1_000_000.0)
            .begin_log_with(&mut buf, &mut serializer)
            .unwrap();
    }

    let expected = "\
                    #[StartTime: 100.000 (seconds since epoch)]\n\
                    #[BaseTime: 200.000 (seconds since epoch)]\n\
                    #[MaxValueDivisor: 1000000.000]\n";

    assert_eq!(expected, str::from_utf8(&buf[..]).unwrap());
}

#[test]
fn write_interval_histo_no_tag() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let mut h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();
    h.record(1000).unwrap();

    {
        let mut log_writer = IntervalLogWriterBuilder::new()
            .with_max_value_divisor(10.0)
            .begin_log_with(&mut buf, &mut serializer)
            .unwrap();

        log_writer
            .write_histogram(
                &h,
                time::Duration::new(1, 234_567_890),
                time::Duration::new(5, 670_000_000),
                None,
            )
            .unwrap();
    }

    let expected = "\
                    #[MaxValueDivisor: 10.000]\n\
                    1.235,5.670,100.000,HISTEwAAAAMAAAAAAAAAAwAAAAAAAAAB//////////8/8AAAAAAAAM8PAg==\n";

    assert_eq!(expected, str::from_utf8(&buf[..]).unwrap());
}

#[test]
fn write_interval_histo_with_tag() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    {
        let mut log_writer = IntervalLogWriterBuilder::new()
            .begin_log_with(&mut buf, &mut serializer)
            .unwrap();

        log_writer
            .write_histogram(
                &h,
                time::Duration::new(1, 234_000_000),
                time::Duration::new(5, 678_000_000),
                Tag::new("t"),
            )
            .unwrap();
    }

    assert_eq!(
        "Tag=t,1.234,5.678,0.000,HISTEwAAAAEAAAAAAAAAAwAAAAAAAAAB//////////8/8AAAAAAAAAA=\n",
        str::from_utf8(&buf[..]).unwrap()
    );
}

#[test]
fn write_start_time() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let _ = IntervalLogWriterBuilder::new()
        .with_start_time(system_time_after_epoch(123, 456_789_012))
        .begin_log_with(&mut buf, &mut serializer)
        .unwrap();

    assert_eq!(
        "#[StartTime: 123.457 (seconds since epoch)]\n",
        str::from_utf8(&buf[..]).unwrap()
    );
}

#[test]
fn write_base_time() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    {
        let _ = IntervalLogWriterBuilder::new()
            .with_base_time(system_time_after_epoch(123, 456_789_012))
            .begin_log_with(&mut buf, &mut serializer)
            .unwrap();
    }

    assert_eq!(
        "#[BaseTime: 123.457 (seconds since epoch)]\n",
        str::from_utf8(&buf[..]).unwrap()
    );
}

#[test]
fn parse_start_time_with_human_date() {
    let (rest, e) = start_time(
        b"#[StartTime: 1441812279.474 (seconds since epoch), Wed Sep 09 08:24:39 PDT 2015]\nfoo",
    ).unwrap();

    let expected = LogEntry::StartTime(1441812279.474);

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn parse_start_time_without_human_date() {
    // Can't be bothered to format a timestamp for humans, so we don't write that data. It's just
    // another part that could be wrong -- what if it disagrees with the seconds since epoch?
    // Also, BaseTime doesn't have a human-formatted time.
    let (rest, e) = start_time(b"#[StartTime: 1441812279.474 (seconds since epoch)]\nfoo").unwrap();

    let expected = LogEntry::StartTime(1441812279.474);

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn parse_base_time() {
    let (rest, e) = base_time(b"#[BaseTime: 1441812279.474 (seconds since epoch)]\nfoo").unwrap();

    let expected = LogEntry::BaseTime(1441812279.474);

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn parse_legend() {
    let input = b"\"StartTimestamp\",\"Interval_Length\",\"Interval_Max\",\
    \"Interval_Compressed_Histogram\"\nfoo";
    let (rest, _) = legend(input).unwrap();

    assert_eq!(b"foo", rest);
}

#[test]
fn parse_comment() {
    let (rest, _) = comment_line(b"#SomeOtherComment\nfoo").unwrap();

    assert_eq!(b"foo", rest);
}

#[test]
fn parse_interval_hist_no_tag() {
    let (rest, e) = interval_hist(b"0.127,1.007,2.769,couldBeBase64\nfoo").unwrap();

    let expected = LogEntry::Interval(IntervalLogHistogram {
        tag: None,
        start_timestamp: 0.127,
        duration: 1.007,
        max: 2.769,
        encoded_histogram: "couldBeBase64",
    });

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn parse_interval_hist_with_tag() {
    let (rest, e) = interval_hist(b"Tag=t,0.127,1.007,2.769,couldBeBase64\nfoo").unwrap();

    let expected = LogEntry::Interval(IntervalLogHistogram {
        tag: Some(Tag("t")),
        start_timestamp: 0.127,
        duration: 1.007,
        max: 2.769,
        encoded_histogram: "couldBeBase64",
    });

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn iter_with_ignored_prefix() {
    let mut data = Vec::new();
    data.extend_from_slice(b"#I'm a comment\n");
    data.extend_from_slice(b"\"StartTimestamp\",etc\n");
    data.extend_from_slice(b"Tag=t,0.127,1.007,2.769,couldBeBase64\n");
    data.extend_from_slice(b"#[StartTime: 1441812279.474 ...\n");

    let entries: Vec<LogEntry> = IntervalLogIterator::new(&data)
        .map(|r| r.unwrap())
        .collect();

    let expected0 = LogEntry::Interval(IntervalLogHistogram {
        tag: Some(Tag("t")),
        start_timestamp: 0.127,
        duration: 1.007,
        max: 2.769,
        encoded_histogram: "couldBeBase64",
    });

    let expected1 = LogEntry::StartTime(1441812279.474);

    assert_eq!(vec![expected0, expected1], entries)
}

#[test]
fn iter_without_ignored_prefix() {
    let mut data = Vec::new();
    data.extend_from_slice(b"Tag=t,0.127,1.007,2.769,couldBeBase64\n");
    data.extend_from_slice(b"#[StartTime: 1441812279.474 ...\n");

    let entries: Vec<LogEntry> = IntervalLogIterator::new(&data)
        .map(|r| r.unwrap())
        .collect();

    let expected0 = LogEntry::Interval(IntervalLogHistogram {
        tag: Some(Tag("t")),
        start_timestamp: 0.127,
        duration: 1.007,
        max: 2.769,
        encoded_histogram: "couldBeBase64",
    });

    let expected1 = LogEntry::StartTime(1441812279.474);

    assert_eq!(vec![expected0, expected1], entries)
}

#[test]
fn iter_multiple_entrties_with_interleaved_ignored() {
    let mut data = Vec::new();
    data.extend_from_slice(b"#I'm a comment\n");
    data.extend_from_slice(b"\"StartTimestamp\",etc\n");
    data.extend_from_slice(b"Tag=t,0.127,1.007,2.769,couldBeBase64\n");
    data.extend_from_slice(b"#Another comment\n");
    data.extend_from_slice(b"#[StartTime: 1441812279.474 ...\n");
    data.extend_from_slice(b"#Yet another comment\n");
    data.extend_from_slice(b"#[BaseTime: 1441812279.474 ...\n");
    data.extend_from_slice(b"#Enough with the comments\n");

    let entries: Vec<LogEntry> = IntervalLogIterator::new(&data)
        .map(|r| r.unwrap())
        .collect();

    let expected0 = LogEntry::Interval(IntervalLogHistogram {
        tag: Some(Tag("t")),
        start_timestamp: 0.127,
        duration: 1.007,
        max: 2.769,
        encoded_histogram: "couldBeBase64",
    });

    let expected1 = LogEntry::StartTime(1441812279.474);
    let expected2 = LogEntry::BaseTime(1441812279.474);

    assert_eq!(vec![expected0, expected1, expected2], entries)
}

#[test]
fn iter_all_ignored_empty_iter() {
    let mut data = Vec::new();
    data.extend_from_slice(b"#I'm a comment\n");
    data.extend_from_slice(b"\"StartTimestamp\",etc\n");
    data.extend_from_slice(b"#Another comment\n");

    assert_eq!(0, IntervalLogIterator::new(&data).count());
}

fn system_time_after_epoch(secs: u64, nanos: u32) -> time::SystemTime {
    time::UNIX_EPOCH.add(time::Duration::new(secs, nanos))
}
