pub trait StreamOutput: Sized {
    fn from_stream_text(raw: &str) -> Self;
}

impl StreamOutput for String {
    fn from_stream_text(raw: &str) -> Self {
        raw.to_string()
    }
}
