use feedparser_rs::FeedError;
use napi::{Error, Status};

pub fn convert_feed_error(err: FeedError) -> Error {
    match err {
        FeedError::XmlError(msg) => {
            Error::new(Status::InvalidArg, format!("XML parse error: {msg}"))
        }
        FeedError::IoError(msg) => Error::new(Status::GenericFailure, format!("I/O error: {msg}")),
        FeedError::InvalidFormat(msg) => {
            Error::new(Status::InvalidArg, format!("Invalid feed format: {msg}"))
        }
        // `EncodingError` can originate from remote content fetched via `parse_url` (not just
        // caller-supplied bytes), so `InvalidArg` is not always fault-accurate. Kept for parity
        // with the Python binding's `convert_feed_error`, which maps it the same way.
        FeedError::EncodingError(msg) => {
            Error::new(Status::InvalidArg, format!("Encoding error: {msg}"))
        }
        FeedError::JsonError(msg) => {
            Error::new(Status::InvalidArg, format!("JSON parse error: {msg}"))
        }
        FeedError::Http { message } => {
            Error::new(Status::GenericFailure, format!("HTTP error: {message}"))
        }
        FeedError::UrlError(msg) => {
            Error::new(Status::InvalidArg, format!("URL parse error: {msg}"))
        }
        FeedError::Unknown(msg) => {
            Error::new(Status::GenericFailure, format!("Unknown error: {msg}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mismatched_end_tag_error() -> quick_xml::Error {
        let mut reader = quick_xml::Reader::from_str("<a></b>");
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => unreachable!("input must produce an error"),
                Ok(_) => {}
                Err(e) => return e,
            }
        }
    }

    #[test]
    fn maps_format_and_input_errors_to_invalid_arg() {
        let cases = [
            FeedError::XmlError(mismatched_end_tag_error()),
            FeedError::InvalidFormat("not a feed".to_string()),
            FeedError::EncodingError("bad charset".to_string()),
            FeedError::JsonError(serde_json::from_str::<u8>("x").expect_err("must fail")),
            FeedError::UrlError(url::ParseError::EmptyHost),
        ];
        for case in cases {
            assert_eq!(convert_feed_error(case).status, Status::InvalidArg);
        }
    }

    #[test]
    fn maps_io_and_unknown_errors_to_generic_failure() {
        let cases = [
            FeedError::IoError(std::io::Error::other("disk full")),
            FeedError::Http {
                message: "timeout".to_string(),
            },
            FeedError::Unknown("mystery".to_string()),
        ];
        for case in cases {
            assert_eq!(convert_feed_error(case).status, Status::GenericFailure);
        }
    }

    #[test]
    fn preserves_error_message() {
        // Capture the inner quick-xml error's own message dynamically rather
        // than hardcoding its Display wording, which is outside quick-xml's
        // semver guarantee.
        let inner_message = mismatched_end_tag_error().to_string();
        let err = convert_feed_error(FeedError::XmlError(mismatched_end_tag_error()));
        assert!(err.reason.starts_with("XML parse error: "));
        assert!(err.reason.contains(&inner_message));
    }
}
