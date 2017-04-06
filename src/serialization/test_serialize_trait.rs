// Maybe someday there will be an obvious right answer for what serialization should look like, at
// least to the user, but for now we'll only take an easily reversible step towards that. There are
// still several ways the serializer interfaces could change to achieve better performance, so
// committing to anything right now would be premature.
trait TestOnlyHypotheticalSerializerInterface {
    type SerializeError: Debug;

    fn serialize<T: Counter, W: Write>(&mut self, h: &Histogram<T>, writer: &mut W)
                                       -> Result<usize, Self::SerializeError>;
}

impl TestOnlyHypotheticalSerializerInterface for V2Serializer {
    type SerializeError = V2SerializeError;

    fn serialize<T: Counter, W: Write>(&mut self, h: &Histogram<T>, writer: &mut W) -> Result<usize, Self::SerializeError> {
        self.serialize(h, writer)
    }
}

impl TestOnlyHypotheticalSerializerInterface for V2DeflateSerializer {
    type SerializeError = V2DeflateSerializeError;

    fn serialize<T: Counter, W: Write>(&mut self, h: &Histogram<T>, writer: &mut W) -> Result<usize, Self::SerializeError> {
        self.serialize(h, writer)
    }
}
