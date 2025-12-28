/// Media RSS Specification
///
/// Namespace: <http://search.yahoo.com/mrss/>
/// Prefix: media
///
/// This module provides parsing support for Media RSS elements commonly
/// used in video/audio feeds and podcasts.
///
/// Common elements:
/// - `media:content` → enclosures
/// - `media:thumbnail` → (could add thumbnails field)
/// - `media:title` → title (fallback)
/// - `media:description` → summary (fallback)
/// - `media:keywords` → tags (comma-separated)
/// - `media:category` → tags
/// - `media:credit` → contributors
use crate::types::{Enclosure, Entry, Tag};

/// Media RSS namespace URI
pub const MEDIA_NAMESPACE: &str = "http://search.yahoo.com/mrss/";

/// Media RSS content element with full attribute support
///
/// Represents a media object embedded in the feed with detailed metadata.
/// Commonly used in video/audio feeds and podcasts.
///
/// # Security Warning
///
/// The `url` field comes from untrusted feed input and has NOT been validated for SSRF.
/// Applications MUST validate URLs before fetching to prevent SSRF attacks.
///
/// # Examples
///
/// ```
/// use feedparser_rs::namespace::media_rss::MediaContent;
///
/// let content = MediaContent {
///     url: "https://example.com/video.mp4".to_string(),
///     type_: Some("video/mp4".to_string()),
///     medium: Some("video".to_string()),
///     width: Some(1920),
///     height: Some(1080),
///     ..Default::default()
/// };
///
/// assert_eq!(content.url, "https://example.com/video.mp4");
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct MediaContent {
    /// URL of the media object (url attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: String,
    /// MIME type (type attribute): "video/mp4", "audio/mpeg", etc.
    pub type_: Option<String>,
    /// Medium type (medium attribute): "image", "video", "audio", "document", "executable"
    pub medium: Option<String>,
    /// File size in bytes (fileSize attribute)
    pub file_size: Option<u64>,
    /// Bitrate in kilobits per second (bitrate attribute)
    pub bitrate: Option<u32>,
    /// Frame rate in frames per second (framerate attribute)
    pub framerate: Option<f32>,
    /// Width in pixels (width attribute)
    pub width: Option<u32>,
    /// Height in pixels (height attribute)
    pub height: Option<u32>,
    /// Duration in seconds (duration attribute)
    pub duration: Option<u32>,
    /// Expression (expression attribute): "full", "sample", "nonstop"
    ///
    /// - "full": complete media object
    /// - "sample": preview/sample of media
    /// - "nonstop": continuous/streaming media
    pub expression: Option<String>,
    /// Whether this is the default media object (isDefault attribute)
    pub is_default: Option<bool>,
}

/// Media RSS thumbnail element
///
/// Represents a thumbnail image for a media object.
///
/// # Security Warning
///
/// The `url` field comes from untrusted feed input and has NOT been validated for SSRF.
/// Applications MUST validate URLs before fetching to prevent SSRF attacks.
///
/// # Examples
///
/// ```
/// use feedparser_rs::namespace::media_rss::MediaThumbnail;
///
/// let thumbnail = MediaThumbnail {
///     url: "https://example.com/thumb.jpg".to_string(),
///     width: Some(640),
///     height: Some(480),
///     time: None,
/// };
///
/// assert_eq!(thumbnail.url, "https://example.com/thumb.jpg");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MediaThumbnail {
    /// URL of the thumbnail image (url attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: String,
    /// Width in pixels (width attribute)
    pub width: Option<u32>,
    /// Height in pixels (height attribute)
    pub height: Option<u32>,
    /// Time offset in NTP format (time attribute)
    ///
    /// Indicates which frame of the media this thumbnail represents.
    pub time: Option<String>,
}

/// Handle Media RSS element at entry level
///
/// Note: This is a simplified implementation. Full Media RSS support
/// would require parsing element attributes (url, type, width, height, etc.)
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `entry` - Entry to update
pub fn handle_entry_element(element: &str, text: &str, entry: &mut Entry) {
    match element {
        "title" => {
            if entry.title.is_none() {
                entry.title = Some(text.to_string());
            }
        }
        "description" => {
            if entry.summary.is_none() {
                entry.summary = Some(text.to_string());
            }
        }
        "keywords" => {
            // Comma-separated keywords
            for keyword in text.split(',') {
                let keyword = keyword.trim();
                if !keyword.is_empty() {
                    entry.tags.push(Tag::new(keyword));
                }
            }
        }
        "category" => {
            if !text.is_empty() {
                entry.tags.push(Tag::new(text));
            }
        }
        _ => {
            // Other elements like media:content, media:thumbnail, media:credit
            // would require attribute parsing which needs integration with
            // the XML parser. For now, we skip these.
        }
    }
}

/// Convert `MediaContent` to `Enclosure` for backward compatibility
///
/// Extracts URL, type, and `file_size` to create a basic enclosure.
/// Used when adding `media:content` to `entry.enclosures`.
///
/// # Examples
///
/// ```
/// use feedparser_rs::namespace::media_rss::{MediaContent, media_content_to_enclosure};
///
/// let content = MediaContent {
///     url: "https://example.com/video.mp4".to_string(),
///     type_: Some("video/mp4".to_string()),
///     file_size: Some(1_024_000),
///     ..Default::default()
/// };
///
/// let enclosure = media_content_to_enclosure(&content);
/// assert_eq!(enclosure.url, "https://example.com/video.mp4");
/// assert_eq!(enclosure.enclosure_type.as_deref(), Some("video/mp4"));
/// assert_eq!(enclosure.length, Some(1_024_000));
/// ```
pub fn media_content_to_enclosure(content: &MediaContent) -> Enclosure {
    Enclosure {
        url: content.url.clone(),
        enclosure_type: content.type_.clone(),
        length: content.file_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_title() {
        let mut entry = Entry::default();
        handle_entry_element("title", "Video Title", &mut entry);

        assert_eq!(entry.title.as_deref(), Some("Video Title"));
    }

    #[test]
    fn test_media_description() {
        let mut entry = Entry::default();
        handle_entry_element("description", "Video description", &mut entry);

        assert_eq!(entry.summary.as_deref(), Some("Video description"));
    }

    #[test]
    fn test_media_keywords() {
        let mut entry = Entry::default();
        handle_entry_element("keywords", "tech, programming, rust", &mut entry);

        assert_eq!(entry.tags.len(), 3);
        assert_eq!(entry.tags[0].term, "tech");
        assert_eq!(entry.tags[1].term, "programming");
        assert_eq!(entry.tags[2].term, "rust");
    }

    #[test]
    fn test_media_keywords_with_spaces() {
        let mut entry = Entry::default();
        handle_entry_element("keywords", "  tech  ,  programming  ", &mut entry);

        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0].term, "tech");
        assert_eq!(entry.tags[1].term, "programming");
    }

    #[test]
    fn test_media_category() {
        let mut entry = Entry::default();
        handle_entry_element("category", "Technology", &mut entry);

        assert_eq!(entry.tags.len(), 1);
        assert_eq!(entry.tags[0].term, "Technology");
    }

    #[test]
    fn test_media_content_default() {
        let content = MediaContent::default();
        assert!(content.url.is_empty());
        assert!(content.type_.is_none());
        assert!(content.medium.is_none());
        assert!(content.file_size.is_none());
        assert!(content.bitrate.is_none());
        assert!(content.framerate.is_none());
        assert!(content.width.is_none());
        assert!(content.height.is_none());
        assert!(content.duration.is_none());
        assert!(content.expression.is_none());
        assert!(content.is_default.is_none());
    }

    #[test]
    fn test_media_content_full_attributes() {
        let content = MediaContent {
            url: "https://example.com/video.mp4".to_string(),
            type_: Some("video/mp4".to_string()),
            medium: Some("video".to_string()),
            file_size: Some(10_485_760), // 10 MB
            bitrate: Some(1500),         // 1500 kbps
            framerate: Some(30.0),
            width: Some(1920),
            height: Some(1080),
            duration: Some(600), // 10 minutes
            expression: Some("full".to_string()),
            is_default: Some(true),
        };

        assert_eq!(content.url, "https://example.com/video.mp4");
        assert_eq!(content.type_.as_deref(), Some("video/mp4"));
        assert_eq!(content.medium.as_deref(), Some("video"));
        assert_eq!(content.file_size, Some(10_485_760));
        assert_eq!(content.bitrate, Some(1500));
        assert_eq!(content.framerate, Some(30.0));
        assert_eq!(content.width, Some(1920));
        assert_eq!(content.height, Some(1080));
        assert_eq!(content.duration, Some(600));
        assert_eq!(content.expression.as_deref(), Some("full"));
        assert_eq!(content.is_default, Some(true));
    }

    #[test]
    fn test_media_content_audio() {
        let content = MediaContent {
            url: "https://example.com/audio.mp3".to_string(),
            type_: Some("audio/mpeg".to_string()),
            medium: Some("audio".to_string()),
            file_size: Some(5_242_880), // 5 MB
            bitrate: Some(128),         // 128 kbps
            duration: Some(180),        // 3 minutes
            ..Default::default()
        };

        assert_eq!(content.medium.as_deref(), Some("audio"));
        assert_eq!(content.bitrate, Some(128));
        assert!(content.width.is_none());
        assert!(content.height.is_none());
        assert!(content.framerate.is_none());
    }

    #[test]
    fn test_media_content_image() {
        let content = MediaContent {
            url: "https://example.com/image.jpg".to_string(),
            type_: Some("image/jpeg".to_string()),
            medium: Some("image".to_string()),
            width: Some(800),
            height: Some(600),
            ..Default::default()
        };

        assert_eq!(content.medium.as_deref(), Some("image"));
        assert_eq!(content.width, Some(800));
        assert_eq!(content.height, Some(600));
        assert!(content.duration.is_none());
    }

    #[test]
    fn test_media_content_expression_variants() {
        let full = MediaContent {
            expression: Some("full".to_string()),
            ..Default::default()
        };
        let sample = MediaContent {
            expression: Some("sample".to_string()),
            ..Default::default()
        };
        let nonstop = MediaContent {
            expression: Some("nonstop".to_string()),
            ..Default::default()
        };

        assert_eq!(full.expression.as_deref(), Some("full"));
        assert_eq!(sample.expression.as_deref(), Some("sample"));
        assert_eq!(nonstop.expression.as_deref(), Some("nonstop"));
    }

    #[test]
    fn test_media_thumbnail_default() {
        let thumbnail = MediaThumbnail::default();
        assert!(thumbnail.url.is_empty());
        assert!(thumbnail.width.is_none());
        assert!(thumbnail.height.is_none());
        assert!(thumbnail.time.is_none());
    }

    #[test]
    fn test_media_thumbnail_full_attributes() {
        let thumbnail = MediaThumbnail {
            url: "https://example.com/thumb.jpg".to_string(),
            width: Some(640),
            height: Some(480),
            time: Some("12:05:01.123".to_string()),
        };

        assert_eq!(thumbnail.url, "https://example.com/thumb.jpg");
        assert_eq!(thumbnail.width, Some(640));
        assert_eq!(thumbnail.height, Some(480));
        assert_eq!(thumbnail.time.as_deref(), Some("12:05:01.123"));
    }

    #[test]
    fn test_media_thumbnail_without_time() {
        let thumbnail = MediaThumbnail {
            url: "https://example.com/poster.jpg".to_string(),
            width: Some(1920),
            height: Some(1080),
            time: None,
        };

        assert_eq!(thumbnail.width, Some(1920));
        assert_eq!(thumbnail.height, Some(1080));
        assert!(thumbnail.time.is_none());
    }

    #[test]
    fn test_media_content_to_enclosure() {
        let content = MediaContent {
            url: "https://example.com/video.mp4".to_string(),
            type_: Some("video/mp4".to_string()),
            file_size: Some(1_024_000),
            width: Some(1920), // These fields are not in Enclosure
            height: Some(1080),
            ..Default::default()
        };

        let enclosure = media_content_to_enclosure(&content);

        assert_eq!(enclosure.url, "https://example.com/video.mp4");
        assert_eq!(enclosure.enclosure_type.as_deref(), Some("video/mp4"));
        assert_eq!(enclosure.length, Some(1_024_000));
    }

    #[test]
    fn test_media_content_to_enclosure_minimal() {
        let content = MediaContent {
            url: "https://example.com/file.bin".to_string(),
            ..Default::default()
        };

        let enclosure = media_content_to_enclosure(&content);

        assert_eq!(enclosure.url, "https://example.com/file.bin");
        assert!(enclosure.enclosure_type.is_none());
        assert!(enclosure.length.is_none());
    }

    #[test]
    fn test_empty_keywords() {
        let mut entry = Entry::default();
        handle_entry_element("keywords", "", &mut entry);

        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_keywords_with_empty_values() {
        let mut entry = Entry::default();
        handle_entry_element("keywords", "tech, , programming", &mut entry);

        assert_eq!(entry.tags.len(), 2);
        assert_eq!(entry.tags[0].term, "tech");
        assert_eq!(entry.tags[1].term, "programming");
    }
}
