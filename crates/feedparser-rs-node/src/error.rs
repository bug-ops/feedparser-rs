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

    #[test]
    fn maps_format_and_input_errors_to_invalid_arg() {
        let cases = [
            FeedError::XmlError("bad xml".to_string()),
            FeedError::InvalidFormat("not a feed".to_string()),
            FeedError::EncodingError("bad charset".to_string()),
            FeedError::JsonError("bad json".to_string()),
            FeedError::UrlError("bad url".to_string()),
        ];
        for case in cases {
            assert_eq!(convert_feed_error(case).status, Status::InvalidArg);
        }
    }

    #[test]
    fn maps_io_and_unknown_errors_to_generic_failure() {
        let cases = [
            FeedError::IoError("disk full".to_string()),
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
        let err = convert_feed_error(FeedError::XmlError("unexpected EOF".to_string()));
        assert!(err.reason.contains("unexpected EOF"));
    }
}
