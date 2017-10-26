use super::super::super::*;
use super::super::*;
use super::*;

#[test]
fn write_header_comment() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    {
        let mut header_writer = IntervalLogHeaderWriter::new(&mut buf, &mut serializer);

        header_writer.write_comment("foo").unwrap();
    }

    assert_eq!(&b"#foo\n"[..], &buf[..]);
}

#[test]
fn write_header_then_interval_comment() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    {
        let mut header_writer = IntervalLogHeaderWriter::new(&mut buf, &mut serializer);
        header_writer.write_comment("foo").unwrap();
        let mut log_writer = header_writer.into_log_writer();

        log_writer.write_comment("bar").unwrap();
    }

    assert_eq!("#foo\n#bar\n", str::from_utf8(&buf[..]).unwrap());
}

#[test]
fn write_interval_histo_no_tag() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    {
        let header_writer = IntervalLogHeaderWriter::new(&mut buf, &mut serializer);
        let mut log_writer = header_writer.into_log_writer();

        log_writer.write_histogram(&h, 1.234, 5.678, None).unwrap();
    }

    assert_eq!(
        "1.234,5.678,0,HISTEwAAAAEAAAAAAAAAAwAAAAAAAAAB//////////8/8AAAAAAAAAA=\n",
        str::from_utf8(&buf[..]).unwrap()
    );
}

#[test]
fn write_interval_histo_with_tag() {
    let mut buf = Vec::new();
    let mut serializer = V2Serializer::new();

    let h = Histogram::<u64>::new_with_bounds(1, u64::max_value(), 3).unwrap();

    {
        let header_writer = IntervalLogHeaderWriter::new(&mut buf, &mut serializer);
        let mut log_writer = header_writer.into_log_writer();

        log_writer
            .write_histogram(&h, 1.234, 5.678, Tag::new("t"))
            .unwrap();
    }

    assert_eq!(
        "Tag=t,1.234,5.678,0,HISTEwAAAAEAAAAAAAAAAwAAAAAAAAAB//////////8/8AAAAAAAAAA=\n",
        str::from_utf8(&buf[..]).unwrap()
    );
}

#[test]
fn parse_start_time() {
    let (rest, e) = start_time(
        b"#[StartTime: 1441812279.474 (seconds since epoch), Wed Sep 09 08:24:39 PDT 2015]\nfoo",
    ).unwrap();

    let expected = LogEntry::StartTime(1441812279.474);

    assert_eq!(expected, e);
    assert_eq!(b"foo", rest);
}

#[test]
fn parse_base_time() {
    let (rest, e) = base_time(
        b"#[BaseTime: 1441812279.474 (seconds since epoch), Wed Sep 09 08:24:39 PDT 2015]\nfoo",
    ).unwrap();

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
        max_value: 2.769,
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
        max_value: 2.769,
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
        max_value: 2.769,
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
        max_value: 2.769,
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
        max_value: 2.769,
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
