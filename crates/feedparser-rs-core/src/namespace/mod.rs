/// Namespace handlers for extended feed formats
///
/// This module provides support for parsing various XML namespaces
/// commonly found in RSS and Atom feeds:
///
/// - **Dublin Core** (`dc:`) - Metadata elements
/// - **Content** (`content:`) - Full HTML content
/// - **Media RSS** (`media:`) - Multimedia content
/// - **GeoRSS** (`georss:`) - Geographic location data
/// - **Creative Commons** (`cc:`) - License information
///
/// # Usage
///
/// These handlers are called by the main parsers when encountering
/// namespaced elements. They update the feed metadata or entries
/// with information from the namespace elements.
///
/// # Example
///
/// ```
/// use feedparser_rs::namespace::dublin_core;
/// use feedparser_rs::FeedMeta;
///
/// let mut feed = FeedMeta::default();
/// dublin_core::handle_feed_element("creator", "John Doe", &mut feed);
/// assert_eq!(feed.author.as_deref(), Some("John Doe"));
/// ```
/// Creative Commons license information
pub mod cc;
/// Content Module for RSS 1.0
pub mod content;
/// Dublin Core Metadata Element Set
pub mod dublin_core;
/// GeoRSS geographic location data
pub mod georss;
/// Media RSS specification
pub mod media_rss;
/// Syndication Module for RSS 1.0
pub mod syndication;

/// Common namespace URIs used in feeds
pub mod namespaces {
    /// Dublin Core Metadata Element Set
    pub const DUBLIN_CORE: &str = "http://purl.org/dc/elements/1.1/";

    /// Content Module for RSS 1.0
    pub const CONTENT: &str = "http://purl.org/rss/1.0/modules/content/";

    /// Media RSS
    pub const MEDIA: &str = "http://search.yahoo.com/mrss/";

    /// Atom 1.0
    pub const ATOM: &str = "http://www.w3.org/2005/Atom";

    /// RSS 1.0 (RDF)
    pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    /// RSS 1.0
    pub const RSS_10: &str = "http://purl.org/rss/1.0/";

    /// Syndication Module for RSS 1.0
    pub const SYNDICATION: &str = "http://purl.org/rss/1.0/modules/syndication/";

    /// iTunes Podcast
    pub const ITUNES: &str = "http://www.itunes.com/dtds/podcast-1.0.dtd";

    /// Podcast 2.0
    pub const PODCAST: &str = "https://podcastindex.org/namespace/1.0";

    /// `GeoRSS`
    pub const GEORSS: &str = "http://www.georss.org/georss";

    /// Creative Commons (modern)
    pub const CC: &str = "http://creativecommons.org/ns#";

    /// Creative Commons (legacy Userland)
    pub const CREATIVE_COMMONS: &str = "http://backend.userland.com/creativeCommonsRssModule";
}

/// Get namespace URI for a common prefix
///
/// # Arguments
///
/// * `prefix` - Namespace prefix (e.g., "dc", "content", "media")
///
/// # Returns
///
/// Returns the namespace URI if the prefix is recognized
pub fn get_namespace_uri(prefix: &str) -> Option<&'static str> {
    match prefix {
        "dc" => Some(namespaces::DUBLIN_CORE),
        "content" => Some(namespaces::CONTENT),
        "media" => Some(namespaces::MEDIA),
        "atom" => Some(namespaces::ATOM),
        "rdf" => Some(namespaces::RDF),
        "syn" | "syndication" => Some(namespaces::SYNDICATION),
        "itunes" => Some(namespaces::ITUNES),
        "podcast" => Some(namespaces::PODCAST),
        "georss" => Some(namespaces::GEORSS),
        "cc" => Some(namespaces::CC),
        "creativeCommons" => Some(namespaces::CREATIVE_COMMONS),
        _ => None,
    }
}

/// Get common prefix for a namespace URI
///
/// # Arguments
///
/// * `uri` - Namespace URI
///
/// # Returns
///
/// Returns the common prefix if the URI is recognized
pub fn get_namespace_prefix(uri: &str) -> Option<&'static str> {
    match uri {
        namespaces::DUBLIN_CORE => Some("dc"),
        namespaces::CONTENT => Some("content"),
        namespaces::MEDIA => Some("media"),
        namespaces::ATOM => Some("atom"),
        namespaces::RDF => Some("rdf"),
        namespaces::SYNDICATION => Some("syn"),
        namespaces::ITUNES => Some("itunes"),
        namespaces::PODCAST => Some("podcast"),
        namespaces::GEORSS => Some("georss"),
        namespaces::CC => Some("cc"),
        namespaces::CREATIVE_COMMONS => Some("creativeCommons"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_namespace_uri() {
        assert_eq!(
            get_namespace_uri("dc"),
            Some("http://purl.org/dc/elements/1.1/")
        );
        assert_eq!(
            get_namespace_uri("content"),
            Some("http://purl.org/rss/1.0/modules/content/")
        );
        assert_eq!(
            get_namespace_uri("media"),
            Some("http://search.yahoo.com/mrss/")
        );
        assert_eq!(get_namespace_uri("unknown"), None);
    }

    #[test]
    fn test_get_namespace_prefix() {
        assert_eq!(
            get_namespace_prefix("http://purl.org/dc/elements/1.1/"),
            Some("dc")
        );
        assert_eq!(
            get_namespace_prefix("http://purl.org/rss/1.0/modules/content/"),
            Some("content")
        );
        assert_eq!(
            get_namespace_prefix("http://search.yahoo.com/mrss/"),
            Some("media")
        );
        assert_eq!(get_namespace_prefix("http://unknown.example.com/"), None);
    }
}
