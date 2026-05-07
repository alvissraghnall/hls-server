struct MediaSegment<'q> {
    uri: String,
    byte_range: &'q [u8], // not entirely sure about this just yet
    duration: i16,        // trying not to use floats
}
