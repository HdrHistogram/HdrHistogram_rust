#[cfg(all(feature = "serialization", test))]
mod tests {
    extern crate hdrsample;

    use self::hdrsample::Histogram;
    use self::hdrsample::serialization::{Deserializer, V2Serializer};

    use std::io::{BufRead, BufReader, Read};
    use std::fs::File;
    use std::path::Path;

    #[test]
    fn serialize_no_compression_matches_java_impl() {
        let h = load_histogram_from_num_per_line(Path::new("tests/data/seq-nums.txt"));

        let mut serialized = Vec::new();
        V2Serializer::new().serialize(&h, &mut serialized).unwrap();

        let mut java_serialized = Vec::new();
        File::open("tests/data/seq-nums.hist").unwrap()
                .read_to_end(&mut java_serialized).unwrap();

        assert_eq!(java_serialized, serialized);
    }

    // zlib compression is not identical between the rust and java versions, so can't compare compressed
    // versions byte for byte

    #[test]
    fn deserialize_no_compression_matches_java() {
        let h = load_histogram_from_num_per_line(Path::new("tests/data/seq-nums.txt"));

        let deser_h: Histogram<u64> = Deserializer::new().deserialize(
            &mut File::open("tests/data/seq-nums.hist").unwrap()).unwrap();

        assert_eq!(h, deser_h);
    }

    #[test]
    fn deserialize_compression_matches_java() {
        let h = load_histogram_from_num_per_line(Path::new("tests/data/seq-nums.txt"));

        let deser_h: Histogram<u64> = Deserializer::new().deserialize(
            &mut File::open("tests/data/seq-nums.histz").unwrap()).unwrap();

        assert_eq!(h, deser_h);
    }

    fn load_histogram_from_num_per_line(path: &Path) -> Histogram<u64> {
        // max is Java's Long.MAX_VALUE
        let mut h: Histogram<u64> = Histogram::new_with_bounds(1, u64::max_value() >> 1, 3).unwrap();
        for num in BufReader::new(File::open(path).unwrap())
                .lines()
                .map(|l| l.unwrap().parse().unwrap()) {
            h.record(num).unwrap();
        }

        h
    }
}
