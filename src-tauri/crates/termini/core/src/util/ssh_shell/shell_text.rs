pub fn decode_shell_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    #[cfg(windows)]
    {
        let (text, _, had_errors) = encoding_rs::GBK.decode(bytes);
        if !had_errors {
            return text.into_owned();
        }
        let (text, _, _) = encoding_rs::GB18030.decode(bytes);
        return text.into_owned();
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}
