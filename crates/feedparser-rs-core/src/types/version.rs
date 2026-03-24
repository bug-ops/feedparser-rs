use std::fmt;

/// Feed format version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedVersion {
    /// RSS 0.90
    Rss090,
    /// RSS 0.91 Netscape
    Rss091Netscape,
    /// RSS 0.91 Userland
    Rss091Userland,
    /// RSS 0.92
    Rss092,
    /// RSS 1.0 (RDF)
    Rss10,
    /// RSS 2.0
    Rss20,
    /// Atom 0.3
    Atom03,
    /// Atom 1.0
    Atom10,
    /// JSON Feed 1.0
    JsonFeed10,
    /// JSON Feed 1.1
    JsonFeed11,
    /// Unknown format
    #[default]
    Unknown,
}

impl FeedVersion {
    /// Returns feedparser-compatible version string
    ///
    /// # Examples
    ///
    /// ```
    /// use feedparser_rs::FeedVersion;
    ///
    /// assert_eq!(FeedVersion::Rss20.as_str(), "rss20");
    /// assert_eq!(FeedVersion::Atom10.as_str(), "atom10");
    /// ```
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Rss090 => "rss090",
            Self::Rss091Netscape => "rss091n",
            Self::Rss091Userland => "rss091u",
            Self::Rss092 => "rss092",
            Self::Rss10 => "rss10",
            Self::Rss20 => "rss20",
            Self::Atom03 => "atom03",
            Self::Atom10 => "atom10",
            Self::JsonFeed10 => "json10",
            Self::JsonFeed11 => "json11",
            Self::Unknown => "",
        }
    }
}

impl fmt::Display for FeedVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_as_str() {
        assert_eq!(FeedVersion::Rss20.as_str(), "rss20");
        assert_eq!(FeedVersion::Atom10.as_str(), "atom10");
        assert_eq!(FeedVersion::Unknown.as_str(), "");
    }

    #[test]
    fn test_version_display() {
        assert_eq!(format!("{}", FeedVersion::Rss20), "rss20");
        assert_eq!(format!("{}", FeedVersion::Atom10), "atom10");
    }

    #[test]
    fn test_version_default() {
        let v: FeedVersion = FeedVersion::default();
        assert_eq!(v, FeedVersion::Unknown);
    }
}
