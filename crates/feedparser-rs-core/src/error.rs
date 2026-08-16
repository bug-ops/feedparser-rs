use thiserror::Error;

/// Feed parsing errors
#[derive(Error, Debug)]
pub enum FeedError {
    /// XML parsing error
    #[error("XML parsing error: {0}")]
    XmlError(#[from] quick_xml::Error),

    /// I/O error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid feed format
    #[error("Invalid feed format: {0}")]
    InvalidFormat(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    EncodingError(String),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// HTTP error
    #[error("HTTP error: {message}")]
    Http {
        /// Error message
        message: String,
    },

    /// URL parsing error
    #[error("URL parsing error: {0}")]
    UrlError(#[from] url::ParseError),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type for feed parsing operations
pub type Result<T> = std::result::Result<T, FeedError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[test]
    fn test_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let inner = io_err.to_string();
        let err = FeedError::from(quick_xml::Error::Io(std::sync::Arc::new(io_err)));
        assert!(err.to_string().starts_with("XML parsing error: "));
        assert!(err.to_string().contains(&inner));
        assert!(matches!(err, FeedError::XmlError(quick_xml::Error::Io(_))));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let feed_err = FeedError::from(io_err);
        assert!(matches!(feed_err, FeedError::IoError(_)));
    }

    #[test]
    #[allow(clippy::unnecessary_wraps)]
    fn test_result_type() {
        fn get_result() -> Result<i32> {
            Ok(42)
        }
        let result = get_result();
        assert!(result.is_ok());
        assert_eq!(result.expect("should be ok"), 42);

        let error: Result<i32> = Err(FeedError::Unknown("test".to_string()));
        assert!(error.is_err());
    }

    #[test]
    fn typed_variants_expose_downcastable_source() {
        let xml_err = FeedError::from(quick_xml::Error::Io(std::sync::Arc::new(
            std::io::Error::other("xml io failure"),
        )));
        let xml_source = StdError::source(&xml_err).expect("XmlError must carry a source");
        assert!(xml_source.downcast_ref::<quick_xml::Error>().is_some());

        let io_err = FeedError::from(std::io::Error::other("io failure"));
        let io_source = StdError::source(&io_err).expect("IoError must carry a source");
        assert!(io_source.downcast_ref::<std::io::Error>().is_some());

        let json_err =
            FeedError::from(serde_json::from_str::<u8>("not json").expect_err("must fail"));
        let json_source = StdError::source(&json_err).expect("JsonError must carry a source");
        assert!(json_source.downcast_ref::<serde_json::Error>().is_some());

        let url_err = FeedError::from(url::ParseError::EmptyHost);
        let url_source = StdError::source(&url_err).expect("UrlError must carry a source");
        assert!(url_source.downcast_ref::<url::ParseError>().is_some());
    }
}
