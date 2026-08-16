use super::common::{MimeType, Url};

/// iTunes podcast metadata for feeds
///
/// Contains podcast-level iTunes namespace metadata from the `itunes:` prefix.
/// Namespace URI: `http://www.itunes.com/dtds/podcast-1.0.dtd`
///
/// # Examples
///
/// ```
/// use feedparser_rs::ItunesFeedMeta;
///
/// let mut itunes = ItunesFeedMeta::default();
/// itunes.author = Some("John Doe".to_string());
/// itunes.explicit = Some(false);
/// itunes.podcast_type = Some("episodic".to_string());
///
/// assert_eq!(itunes.author.as_deref(), Some("John Doe"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ItunesFeedMeta {
    /// Podcast author (itunes:author)
    pub author: Option<String>,
    /// Podcast owner contact information (itunes:owner)
    pub owner: Option<ItunesOwner>,
    /// Podcast categories with optional subcategories
    pub categories: Vec<ItunesCategory>,
    /// Explicit content flag (itunes:explicit)
    pub explicit: Option<bool>,
    /// Podcast artwork URL (itunes:image href attribute)
    pub image: Option<Url>,
    /// Search keywords (itunes:keywords)
    pub keywords: Vec<String>,
    /// Podcast type: "episodic" or "serial"
    pub podcast_type: Option<String>,
    /// Podcast completion status (itunes:complete)
    ///
    /// Raw XML text value from the feed (e.g., "Yes", "No").
    pub complete: Option<String>,
    /// New feed URL for migrated podcasts (itunes:new-feed-url)
    ///
    /// Indicates the podcast has moved to a new feed location.
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub new_feed_url: Option<Url>,
    /// Podcast subtitle (itunes:subtitle)
    pub subtitle: Option<String>,
    /// Podcast summary (itunes:summary)
    pub summary: Option<String>,
    /// Block flag: 1 = blocked ("yes"), 0 = not blocked ("no" or absent)
    ///
    /// Normalized from itunes:block: "yes" → 1, any other value → 0.
    pub block: Option<u8>,
}

/// iTunes podcast metadata for episodes
///
/// Contains episode-level iTunes namespace metadata from the `itunes:` prefix.
///
/// # Examples
///
/// ```
/// use feedparser_rs::ItunesEntryMeta;
///
/// let mut episode = ItunesEntryMeta::default();
/// episode.duration = Some("1:00:00".to_string());
/// episode.episode = Some("42".to_string());
/// episode.season = Some("3".to_string());
/// episode.episode_type = Some("full".to_string());
///
/// assert_eq!(episode.duration.as_deref(), Some("1:00:00"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ItunesEntryMeta {
    /// Episode title override (itunes:title)
    pub title: Option<String>,
    /// Episode author (itunes:author)
    pub author: Option<String>,
    /// Episode duration as raw string (itunes:duration)
    ///
    /// Preserved verbatim from the feed: "3600", "60:00", "1:00:00", "1:23:45", etc.
    pub duration: Option<String>,
    /// Explicit content flag for this episode
    pub explicit: Option<bool>,
    /// Episode-specific artwork URL (itunes:image href)
    pub image: Option<Url>,
    /// Episode number as raw string (itunes:episode)
    pub episode: Option<String>,
    /// Season number as raw string (itunes:season)
    pub season: Option<String>,
    /// Episode type: "full", "trailer", or "bonus"
    pub episode_type: Option<String>,
    /// Episode subtitle (itunes:subtitle)
    pub subtitle: Option<String>,
    /// Episode summary (itunes:summary)
    pub summary: Option<String>,
}

/// iTunes podcast owner information
///
/// Contact information for the podcast owner (itunes:owner).
///
/// # Examples
///
/// ```
/// use feedparser_rs::ItunesOwner;
///
/// let owner = ItunesOwner {
///     name: Some("Jane Doe".to_string()),
///     email: Some("jane@example.com".to_string()),
/// };
///
/// assert_eq!(owner.name.as_deref(), Some("Jane Doe"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ItunesOwner {
    /// Owner's name (itunes:name)
    pub name: Option<String>,
    /// Owner's email address (itunes:email)
    pub email: Option<String>,
}

/// iTunes category with optional subcategory
///
/// Categories follow Apple's podcast category taxonomy.
///
/// # Examples
///
/// ```
/// use feedparser_rs::ItunesCategory;
///
/// let category = ItunesCategory {
///     text: "Technology".to_string(),
///     subcategory: Some("Software How-To".to_string()),
/// };
///
/// assert_eq!(category.text, "Technology");
/// ```
#[derive(Debug, Clone)]
pub struct ItunesCategory {
    /// Category name (text attribute)
    pub text: String,
    /// Optional subcategory (nested itunes:category text attribute)
    pub subcategory: Option<String>,
}

/// Podcast 2.0 metadata
///
/// Modern podcast namespace extensions from `https://podcastindex.org/namespace/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastMeta;
///
/// let mut podcast = PodcastMeta::default();
/// podcast.guid = Some("9b024349-ccf0-5f69-a609-6b82873eab3c".to_string());
///
/// assert!(podcast.guid.is_some());
/// ```
#[derive(Debug, Clone, Default)]
pub struct PodcastMeta {
    /// Transcript URLs (podcast:transcript)
    pub transcripts: Vec<PodcastTranscript>,
    /// Funding/donation links (podcast:funding)
    pub funding: Vec<PodcastFunding>,
    /// People associated with podcast (podcast:person)
    pub persons: Vec<PodcastPerson>,
    /// Permanent podcast GUID (podcast:guid)
    pub guid: Option<String>,
    /// Value-for-value payment information (podcast:value)
    pub value: Option<PodcastValue>,
    /// Content medium type (podcast:medium)
    pub medium: Option<String>,
    /// Ownership transfer lock (podcast:locked text content: "yes" or "no")
    pub locked: Option<String>,
    /// Email of the lock owner (podcast:locked owner attribute)
    pub locked_owner: Option<String>,
    /// Geographic location (podcast:location)
    pub location: Option<PodcastLocation>,
    /// Related feed references (podcast:podroll)
    pub podroll: Vec<PodcastRemoteItem>,
    /// Text records (podcast:txt)
    pub txt: Vec<PodcastTxt>,
    /// Update frequency schedule (podcast:updateFrequency)
    pub update_frequency: Option<PodcastUpdateFrequency>,
    /// Follow links (podcast:follow)
    pub follow: Vec<PodcastFollow>,
    /// Chat room references (podcast:chat)
    pub chat: Vec<PodcastChat>,
    /// Whether the podcast uses Podping for update notifications
    /// (podcast:podping `usesPodping` attribute)
    pub podping_uses_podping: Option<bool>,
}

/// Podcast 2.0 value element for monetization
///
/// Implements value-for-value payment model using cryptocurrency and streaming payments.
/// Used for podcast monetization via Lightning Network, Hive, and other payment methods.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::{PodcastValue, PodcastValueRecipient};
///
/// let value = PodcastValue {
///     type_: "lightning".to_string(),
///     method: "keysend".to_string(),
///     suggested: Some("0.00000005000".to_string()),
///     recipients: vec![
///         PodcastValueRecipient {
///             name: Some("Host".to_string()),
///             type_: "node".to_string(),
///             address: "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a".to_string(),
///             split: 90,
///             fee: Some(false),
///         },
///         PodcastValueRecipient {
///             name: Some("Producer".to_string()),
///             type_: "node".to_string(),
///             address: "02d5c1bf8b940dc9cadca86d1b0a3c37fbe39cee4c7e839e33bef9174531d27f52".to_string(),
///             split: 10,
///             fee: Some(false),
///         },
///     ],
///     time_splits: vec![],
/// };
///
/// assert_eq!(value.type_, "lightning");
/// assert_eq!(value.recipients.len(), 2);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PodcastValue {
    /// Payment type (type attribute): "lightning", "hive", etc.
    pub type_: String,
    /// Payment method (method attribute): "keysend" for Lightning Network
    pub method: String,
    /// Suggested payment amount (suggested attribute)
    ///
    /// Format depends on payment type. For Lightning, this is typically satoshis.
    pub suggested: Option<String>,
    /// List of payment recipients with split percentages
    pub recipients: Vec<PodcastValueRecipient>,
    /// Time-bounded payment splits for pre-recorded remote content
    /// (podcast:valueTimeSplit)
    pub time_splits: Vec<PodcastValueTimeSplit>,
}

/// Value recipient for payment splitting
///
/// Defines a single recipient in the value-for-value payment model.
/// Each recipient receives a percentage (split) of the total payment.
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastValueRecipient;
///
/// let recipient = PodcastValueRecipient {
///     name: Some("Podcast Host".to_string()),
///     type_: "node".to_string(),
///     address: "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a".to_string(),
///     split: 95,
///     fee: Some(false),
/// };
///
/// assert_eq!(recipient.split, 95);
/// assert_eq!(recipient.fee, Some(false));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastValueRecipient {
    /// Recipient's name (name attribute)
    pub name: Option<String>,
    /// Recipient type (type attribute): "node" for Lightning Network nodes
    pub type_: String,
    /// Payment address (address attribute)
    ///
    /// For Lightning: node public key (hex-encoded)
    /// For other types: appropriate address format
    ///
    /// # Security Warning
    ///
    /// This address comes from untrusted feed input. Applications MUST validate
    /// addresses before sending payments to prevent sending funds to wrong recipients.
    pub address: String,
    /// Payment split percentage (split attribute)
    ///
    /// Can be absolute percentage (1-100) or relative value that's normalized.
    /// Total of all splits should equal 100 for percentage-based splits.
    pub split: u32,
    /// Whether this is a fee recipient (fee attribute)
    ///
    /// Fee recipients are paid before regular splits are calculated.
    pub fee: Option<bool>,
}

/// Podcast 2.0 value time split for pre-recorded remote content
///
/// Represents a `<podcast:valueTimeSplit>` element, which routes
/// value-for-value payments for a specific time range of an episode to its
/// own set of recipients and/or to a remote item (e.g. a licensed music
/// track).
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastValueTimeSplit;
///
/// let split = PodcastValueTimeSplit {
///     start_time: 60.0,
///     duration: 30.0,
///     ..Default::default()
/// };
///
/// assert_eq!(split.start_time, 60.0);
/// assert_eq!(split.remote_percentage, 100.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PodcastValueTimeSplit {
    /// Start time in seconds within the episode (startTime attribute)
    pub start_time: f64,
    /// Duration in seconds of this split (duration attribute)
    pub duration: f64,
    /// Start time within the remote item, in seconds (remoteStartTime attribute)
    ///
    /// Defaults to `0.0` when absent or unparseable.
    pub remote_start_time: f64,
    /// Percentage of the payment routed to this split (remotePercentage attribute)
    ///
    /// Defaults to `100.0` per the podcast namespace spec when absent or
    /// unparseable, and is clamped to the 0.0-100.0 range.
    pub remote_percentage: f64,
    /// Payment recipients for this split (podcast:valueRecipient children)
    pub recipients: Vec<PodcastValueRecipient>,
    /// Remote item this split routes payment to, if any (podcast:remoteItem child)
    ///
    /// Only the first `podcast:remoteItem` encountered in the split is kept.
    pub remote_item: Option<PodcastRemoteItem>,
}

impl Default for PodcastValueTimeSplit {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            duration: 0.0,
            remote_start_time: 0.0,
            remote_percentage: 100.0,
            recipients: Vec::new(),
            remote_item: None,
        }
    }
}

/// Podcast 2.0 transcript
///
/// Links to transcript files in various formats.
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastTranscript;
///
/// let transcript = PodcastTranscript {
///     url: "https://example.com/transcript.txt".into(),
///     transcript_type: Some("text/plain".into()),
///     language: Some("en".to_string()),
///     rel: None,
/// };
///
/// assert_eq!(transcript.url, "https://example.com/transcript.txt");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodcastTranscript {
    /// Transcript URL (url attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: Url,
    /// MIME type (type attribute): "text/plain", "text/html", "application/json", etc.
    pub transcript_type: Option<MimeType>,
    /// Language code (language attribute): "en", "es", etc.
    pub language: Option<String>,
    /// Relationship (rel attribute): "captions" or empty
    pub rel: Option<String>,
}

/// Podcast 2.0 funding information
///
/// Links for supporting the podcast financially.
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastFunding;
///
/// let funding = PodcastFunding {
///     url: "https://example.com/donate".into(),
///     message: Some("Support our show!".to_string()),
/// };
///
/// assert_eq!(funding.url, "https://example.com/donate");
/// ```
#[derive(Debug, Clone)]
pub struct PodcastFunding {
    /// Funding URL (url attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: Url,
    /// Optional message/call-to-action (text content)
    pub message: Option<String>,
}

/// Podcast 2.0 person
///
/// Information about hosts, guests, or other people associated with the podcast.
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastPerson;
///
/// let host = PodcastPerson {
///     name: "John Doe".to_string(),
///     role: Some("host".to_string()),
///     group: None,
///     img: Some("https://example.com/john.jpg".into()),
///     href: Some("https://example.com/john".into()),
/// };
///
/// assert_eq!(host.name, "John Doe");
/// assert_eq!(host.role.as_deref(), Some("host"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PodcastPerson {
    /// Person's name (text content)
    pub name: String,
    /// Role: "host", "guest", "editor", etc. (role attribute)
    pub role: Option<String>,
    /// Group name (group attribute)
    pub group: Option<String>,
    /// Image URL (img attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub img: Option<Url>,
    /// Personal URL/homepage (href attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub href: Option<Url>,
}

/// Podcast 2.0 chapters information
///
/// Links to chapter markers for time-based navigation within an episode.
/// Namespace: `https://podcastindex.org/namespace/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastChapters;
///
/// let chapters = PodcastChapters {
///     url: "https://example.com/chapters.json".into(),
///     type_: "application/json+chapters".into(),
/// };
///
/// assert_eq!(chapters.url, "https://example.com/chapters.json");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastChapters {
    /// Chapters file URL (url attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub url: Url,
    /// MIME type (type attribute): "application/json+chapters" or "application/xml+chapters"
    pub type_: MimeType,
}

/// Podcast 2.0 soundbite (shareable clip)
///
/// Marks a portion of the audio for social sharing or highlights.
/// Namespace: `https://podcastindex.org/namespace/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastSoundbite;
///
/// let soundbite = PodcastSoundbite {
///     start_time: 120.5,
///     duration: 30.0,
///     title: Some("Great quote".to_string()),
/// };
///
/// assert_eq!(soundbite.start_time, 120.5);
/// assert_eq!(soundbite.duration, 30.0);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PodcastSoundbite {
    /// Start time in seconds (startTime attribute)
    pub start_time: f64,
    /// Duration in seconds (duration attribute)
    pub duration: f64,
    /// Optional title/description (text content)
    pub title: Option<String>,
}

/// Podcast 2.0 alternate enclosure source
///
/// A single source URI within a `podcast:alternateEnclosure` element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastAlternateEnclosureSource {
    /// Source URI (uri attribute, required)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    /// Applications MUST validate URLs before fetching to prevent SSRF attacks.
    pub uri: Url,
    /// Optional MIME type override (contentType attribute)
    pub content_type: Option<MimeType>,
}

/// Podcast 2.0 integrity verification for alternate enclosures
///
/// Cryptographic integrity check for enclosure sources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastIntegrity {
    /// Integrity type (type attribute): "sri" or "pgp-signature"
    pub type_: String,
    /// Integrity value (text content)
    pub value: String,
}

/// Podcast 2.0 alternate enclosure
///
/// An alternate version of the main episode audio/video in a different format or quality.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub struct PodcastAlternateEnclosure {
    /// MIME type (type attribute, required)
    pub type_: MimeType,
    /// File size in bytes (length attribute)
    pub length: Option<u64>,
    /// Bitrate in kbps (bitrate attribute)
    pub bitrate: Option<f64>,
    /// Video height in pixels (height attribute)
    pub height: Option<u32>,
    /// Language code (lang attribute)
    pub lang: Option<String>,
    /// Title (title attribute)
    pub title: Option<String>,
    /// Relationship (rel attribute): "default", "alternate", etc.
    pub rel: Option<String>,
    /// Codecs string (codecs attribute)
    pub codecs: Option<String>,
    /// Whether this is the default enclosure (default attribute)
    pub default: Option<bool>,
    /// Source URIs for this enclosure
    pub sources: Vec<PodcastAlternateEnclosureSource>,
    /// Integrity verification
    pub integrity: Option<PodcastIntegrity>,
}

/// Podcast 2.0 geographic location
///
/// Location information for a podcast or episode.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastLocation {
    /// Human-readable location name (text content)
    pub name: String,
    /// Geographic coordinates (geo attribute): "geo:37.786971,-122.399677"
    pub geo: Option<String>,
    /// OpenStreetMap reference (osm attribute): "R113314"
    pub osm: Option<String>,
}

/// Podcast 2.0 remote item reference
///
/// A reference to a remote podcast feed or episode, used within `podcast:podroll`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastRemoteItem {
    /// Feed GUID (feedGuid attribute)
    pub feed_guid: Option<String>,
    /// Feed URL (feedUrl attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    pub feed_url: Option<Url>,
    /// Item GUID (itemGuid attribute)
    pub item_guid: Option<String>,
    /// Content medium type (medium attribute)
    pub medium: Option<String>,
    /// Display title (title attribute)
    pub title: Option<String>,
}

/// Podcast 2.0 social interaction
///
/// Links a podcast episode to a social media thread.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastSocialInteract {
    /// Social thread URI (uri attribute, required)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    pub uri: Url,
    /// Social protocol (protocol attribute): "activitypub", "twitter", etc.
    pub protocol: Option<String>,
    /// Account identifier (accountId attribute)
    pub account_id: Option<String>,
    /// Account URL (accountUrl attribute)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    pub account_url: Option<Url>,
    /// Priority (priority attribute, lower = higher priority)
    pub priority: Option<u32>,
}

/// Podcast 2.0 text record
///
/// Arbitrary text metadata with an optional purpose tag.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastTxt {
    /// Purpose of the text (purpose attribute)
    pub purpose: Option<String>,
    /// Text content
    pub value: String,
}

/// Podcast 2.0 update frequency
///
/// Indicates how often a podcast publishes new episodes.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastUpdateFrequency {
    /// iCalendar RRULE string (rrule attribute)
    pub rrule: Option<String>,
    /// Whether the podcast is complete (complete attribute)
    pub complete: Option<bool>,
    /// Start date in ISO 8601 (dtstart attribute)
    pub dtstart: Option<String>,
    /// Human-readable label (text content)
    pub label: Option<String>,
}

/// Podcast 2.0 follow link
///
/// A URL and optional platform for following the podcast.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastFollow {
    /// Follow URL (url attribute, required)
    ///
    /// # Security Warning
    ///
    /// This URL comes from untrusted feed input and has NOT been validated for SSRF.
    pub url: Url,
    /// Platform name (platform attribute)
    pub platform: Option<String>,
}

/// Podcast 2.0 chat room reference
///
/// Points to a chat server/room associated with the podcast or episode
/// (podcast:chat), e.g. Matrix or XMPP.
///
/// Namespace: `https://podcastindex.org/namespace/1.0`
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastChat;
///
/// let chat = PodcastChat {
///     server: "matrix.example.com".to_string(),
///     protocol: "matrix".to_string(),
///     account_id: Some("@podcast:example.com".to_string()),
///     space: None,
/// };
///
/// assert_eq!(chat.server, "matrix.example.com");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PodcastChat {
    /// Chat server address (server attribute, required)
    pub server: String,
    /// Chat protocol (protocol attribute, required): "matrix", "xmpp", etc.
    pub protocol: String,
    /// Account identifier on the chat server (accountId attribute)
    pub account_id: Option<String>,
    /// Space identifier, for protocols that group rooms (space attribute)
    pub space: Option<String>,
}

/// Podcast 2.0 metadata for episodes
///
/// Container for entry-level podcast metadata.
///
/// # Examples
///
/// ```
/// use feedparser_rs::PodcastEntryMeta;
///
/// let mut podcast = PodcastEntryMeta::default();
/// assert!(podcast.transcript.is_empty());
/// assert!(podcast.chapters.is_none());
/// assert!(podcast.soundbite.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PodcastEntryMeta {
    /// Transcript URLs (podcast:transcript)
    pub transcript: Vec<PodcastTranscript>,
    /// Chapter markers (podcast:chapters)
    pub chapters: Option<PodcastChapters>,
    /// Shareable soundbites (podcast:soundbite)
    pub soundbite: Vec<PodcastSoundbite>,
    /// People associated with this episode (podcast:person)
    pub persons: Vec<PodcastPerson>,
    /// Content medium type (podcast:medium)
    pub medium: Option<String>,
    /// Season number (podcast:season number attribute)
    pub season: Option<String>,
    /// Episode number (podcast:episode number attribute)
    pub episode: Option<String>,
    /// Alternate enclosures (podcast:alternateEnclosure)
    pub alternate_enclosures: Vec<PodcastAlternateEnclosure>,
    /// Value-for-value payment information (podcast:value)
    ///
    /// Item-level `<podcast:value>` is where time-bounded payment splits
    /// (`podcast:valueTimeSplit`) are expected to appear in practice, since
    /// they redistribute payment during playback of a specific episode.
    ///
    /// `None` both when `<podcast:value>` is absent and when it is present
    /// but self-closing (`<podcast:value/>`) — a self-closing `podcast:value`
    /// is skipped entirely (attributes included), matching how self-closing
    /// `podcast:valueTimeSplit` is handled. If multiple `<podcast:value>`
    /// elements appear in the same `<item>` (invalid per spec, which allows
    /// at most one), the last one parsed wins.
    pub value: Option<PodcastValue>,
    /// Geographic location (podcast:location)
    pub location: Option<PodcastLocation>,
    /// Social interaction threads (podcast:socialInteract)
    pub social_interact: Vec<PodcastSocialInteract>,
    /// Text records (podcast:txt)
    pub txt: Vec<PodcastTxt>,
    /// Follow links (podcast:follow)
    pub follow: Vec<PodcastFollow>,
    /// Chat room references (podcast:chat)
    pub chat: Vec<PodcastChat>,
}

/// Parse iTunes explicit flag from various string representations
///
/// Maps "yes"/"true"/"explicit" to `Some(true)`.
/// Maps "no"/"false"/"clean" and absent values to `None` (per Python feedparser compatibility).
///
/// Case-insensitive matching.
///
/// # Arguments
///
/// * `s` - Explicit flag string
///
/// # Examples
///
/// ```
/// use feedparser_rs::parse_explicit;
///
/// assert_eq!(parse_explicit("yes"), Some(true));
/// assert_eq!(parse_explicit("YES"), Some(true));
/// assert_eq!(parse_explicit("true"), Some(true));
/// assert_eq!(parse_explicit("explicit"), Some(true));
///
/// assert_eq!(parse_explicit("no"), None);
/// assert_eq!(parse_explicit("false"), None);
/// assert_eq!(parse_explicit("clean"), None);
///
/// assert_eq!(parse_explicit("unknown"), None);
/// ```
pub fn parse_explicit(s: &str) -> Option<bool> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("yes")
        || s.eq_ignore_ascii_case("true")
        || s.eq_ignore_ascii_case("explicit")
    {
        Some(true)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_explicit_true_variants() {
        assert_eq!(parse_explicit("yes"), Some(true));
        assert_eq!(parse_explicit("YES"), Some(true));
        assert_eq!(parse_explicit("Yes"), Some(true));
        assert_eq!(parse_explicit("true"), Some(true));
        assert_eq!(parse_explicit("TRUE"), Some(true));
        assert_eq!(parse_explicit("explicit"), Some(true));
        assert_eq!(parse_explicit("EXPLICIT"), Some(true));
    }

    #[test]
    fn test_parse_explicit_false_variants_return_none() {
        // "no"/"false"/"clean" → None (Python feedparser compat: only "yes" is truthy)
        assert_eq!(parse_explicit("no"), None);
        assert_eq!(parse_explicit("NO"), None);
        assert_eq!(parse_explicit("No"), None);
        assert_eq!(parse_explicit("false"), None);
        assert_eq!(parse_explicit("FALSE"), None);
        assert_eq!(parse_explicit("clean"), None);
        assert_eq!(parse_explicit("CLEAN"), None);
    }

    #[test]
    fn test_parse_explicit_whitespace() {
        assert_eq!(parse_explicit("  yes  "), Some(true));
        assert_eq!(parse_explicit("  no  "), None);
    }

    #[test]
    fn test_parse_explicit_unknown() {
        assert_eq!(parse_explicit("unknown"), None);
        assert_eq!(parse_explicit("maybe"), None);
        assert_eq!(parse_explicit(""), None);
        assert_eq!(parse_explicit("1"), None);
    }

    #[test]
    fn test_itunes_feed_meta_default() {
        let meta = ItunesFeedMeta::default();
        assert!(meta.author.is_none());
        assert!(meta.owner.is_none());
        assert!(meta.categories.is_empty());
        assert!(meta.explicit.is_none());
        assert!(meta.image.is_none());
        assert!(meta.keywords.is_empty());
        assert!(meta.podcast_type.is_none());
        assert!(meta.complete.is_none());
        assert!(meta.new_feed_url.is_none());
    }

    #[test]
    fn test_itunes_entry_meta_default() {
        let meta = ItunesEntryMeta::default();
        assert!(meta.title.is_none());
        assert!(meta.author.is_none());
        assert!(meta.duration.is_none());
        assert!(meta.explicit.is_none());
        assert!(meta.image.is_none());
        assert!(meta.episode.is_none());
        assert!(meta.season.is_none());
        assert!(meta.episode_type.is_none());
    }

    #[test]
    fn test_itunes_entry_meta_string_fields() {
        let meta = ItunesEntryMeta {
            duration: Some("1:23:45".to_string()),
            episode: Some("42".to_string()),
            season: Some("3".to_string()),
            ..Default::default()
        };
        assert_eq!(meta.duration.as_deref(), Some("1:23:45"));
        assert_eq!(meta.episode.as_deref(), Some("42"));
        assert_eq!(meta.season.as_deref(), Some("3"));
    }

    #[test]
    fn test_itunes_owner_default() {
        let owner = ItunesOwner::default();
        assert!(owner.name.is_none());
        assert!(owner.email.is_none());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_itunes_category_clone() {
        let category = ItunesCategory {
            text: "Technology".to_string(),
            subcategory: Some("Software".to_string()),
        };
        let cloned = category.clone();
        assert_eq!(cloned.text, "Technology");
        assert_eq!(cloned.subcategory.as_deref(), Some("Software"));
    }

    #[test]
    fn test_podcast_meta_default() {
        let meta = PodcastMeta::default();
        assert!(meta.transcripts.is_empty());
        assert!(meta.funding.is_empty());
        assert!(meta.persons.is_empty());
        assert!(meta.guid.is_none());
        assert!(meta.location.is_none());
        assert!(meta.podroll.is_empty());
        assert!(meta.txt.is_empty());
        assert!(meta.update_frequency.is_none());
        assert!(meta.follow.is_empty());
        assert!(meta.chat.is_empty());
        assert!(meta.podping_uses_podping.is_none());
    }

    #[test]
    fn test_podcast_entry_meta_new_fields_default() {
        let meta = PodcastEntryMeta::default();
        assert!(meta.alternate_enclosures.is_empty());
        assert!(meta.location.is_none());
        assert!(meta.social_interact.is_empty());
        assert!(meta.txt.is_empty());
        assert!(meta.follow.is_empty());
        assert!(meta.chat.is_empty());
    }

    #[test]
    fn test_podcast_chat_default() {
        let chat = PodcastChat::default();
        assert!(chat.server.is_empty());
        assert!(chat.protocol.is_empty());
        assert!(chat.account_id.is_none());
        assert!(chat.space.is_none());
    }

    #[test]
    fn test_podcast_value_time_split_default() {
        let split = PodcastValueTimeSplit::default();
        assert!((split.start_time - 0.0).abs() < f64::EPSILON);
        assert!((split.duration - 0.0).abs() < f64::EPSILON);
        assert!((split.remote_start_time - 0.0).abs() < f64::EPSILON);
        assert!((split.remote_percentage - 100.0).abs() < f64::EPSILON);
        assert!(split.recipients.is_empty());
        assert!(split.remote_item.is_none());
    }

    #[test]
    fn test_podcast_location_default() {
        let loc = PodcastLocation::default();
        assert!(loc.name.is_empty());
        assert!(loc.geo.is_none());
        assert!(loc.osm.is_none());
    }

    #[test]
    fn test_podcast_social_interact_default() {
        let si = PodcastSocialInteract::default();
        assert!(si.uri.is_empty());
        assert!(si.protocol.is_none());
        assert!(si.account_id.is_none());
        assert!(si.account_url.is_none());
        assert!(si.priority.is_none());
    }

    #[test]
    fn test_podcast_txt_default() {
        let txt = PodcastTxt::default();
        assert!(txt.purpose.is_none());
        assert!(txt.value.is_empty());
    }

    #[test]
    fn test_podcast_update_frequency_default() {
        let uf = PodcastUpdateFrequency::default();
        assert!(uf.rrule.is_none());
        assert!(uf.complete.is_none());
        assert!(uf.dtstart.is_none());
        assert!(uf.label.is_none());
    }

    #[test]
    fn test_podcast_follow_default() {
        let f = PodcastFollow::default();
        assert!(f.url.is_empty());
        assert!(f.platform.is_none());
    }

    #[test]
    fn test_podcast_remote_item_default() {
        let item = PodcastRemoteItem::default();
        assert!(item.feed_guid.is_none());
        assert!(item.feed_url.is_none());
        assert!(item.item_guid.is_none());
        assert!(item.medium.is_none());
        assert!(item.title.is_none());
    }

    #[test]
    fn test_podcast_alternate_enclosure_default() {
        let ae = PodcastAlternateEnclosure::default();
        assert!(ae.type_.is_empty());
        assert!(ae.length.is_none());
        assert!(ae.bitrate.is_none());
        assert!(ae.sources.is_empty());
        assert!(ae.integrity.is_none());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_transcript_clone() {
        let transcript = PodcastTranscript {
            url: "https://example.com/transcript.txt".to_string().into(),
            transcript_type: Some("text/plain".to_string().into()),
            language: Some("en".to_string()),
            rel: None,
        };
        let cloned = transcript.clone();
        assert_eq!(cloned.url, "https://example.com/transcript.txt");
        assert_eq!(cloned.transcript_type.as_deref(), Some("text/plain"));
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_funding_clone() {
        let funding = PodcastFunding {
            url: "https://example.com/donate".to_string().into(),
            message: Some("Support us!".to_string()),
        };
        let cloned = funding.clone();
        assert_eq!(cloned.url, "https://example.com/donate");
        assert_eq!(cloned.message.as_deref(), Some("Support us!"));
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_person_clone() {
        let person = PodcastPerson {
            name: "John Doe".to_string(),
            role: Some("host".to_string()),
            group: None,
            img: Some("https://example.com/john.jpg".to_string().into()),
            href: Some("https://example.com".to_string().into()),
        };
        let cloned = person.clone();
        assert_eq!(cloned.name, "John Doe");
        assert_eq!(cloned.role.as_deref(), Some("host"));
    }

    #[test]
    fn test_podcast_chapters_default() {
        let chapters = PodcastChapters::default();
        assert!(chapters.url.is_empty());
        assert!(chapters.type_.is_empty());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_chapters_clone() {
        let chapters = PodcastChapters {
            url: "https://example.com/chapters.json".to_string().into(),
            type_: "application/json+chapters".to_string().into(),
        };
        let cloned = chapters.clone();
        assert_eq!(cloned.url, "https://example.com/chapters.json");
        assert_eq!(cloned.type_, "application/json+chapters");
    }

    #[test]
    fn test_podcast_soundbite_default() {
        let soundbite = PodcastSoundbite::default();
        assert!((soundbite.start_time - 0.0).abs() < f64::EPSILON);
        assert!((soundbite.duration - 0.0).abs() < f64::EPSILON);
        assert!(soundbite.title.is_none());
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_soundbite_clone() {
        let soundbite = PodcastSoundbite {
            start_time: 120.5,
            duration: 30.0,
            title: Some("Great quote".to_string()),
        };
        let cloned = soundbite.clone();
        assert!((cloned.start_time - 120.5).abs() < f64::EPSILON);
        assert!((cloned.duration - 30.0).abs() < f64::EPSILON);
        assert_eq!(cloned.title.as_deref(), Some("Great quote"));
    }

    #[test]
    fn test_podcast_entry_meta_default() {
        let meta = PodcastEntryMeta::default();
        assert!(meta.transcript.is_empty());
        assert!(meta.chapters.is_none());
        assert!(meta.soundbite.is_empty());
        assert!(meta.persons.is_empty());
        assert!(meta.medium.is_none());
    }

    #[test]
    fn test_itunes_feed_meta_new_fields() {
        let meta = ItunesFeedMeta {
            complete: Some("Yes".to_string()),
            new_feed_url: Some("https://example.com/new-feed.xml".to_string().into()),
            ..Default::default()
        };

        assert_eq!(meta.complete.as_deref(), Some("Yes"));
        assert_eq!(
            meta.new_feed_url.as_deref(),
            Some("https://example.com/new-feed.xml")
        );
    }

    #[test]
    fn test_podcast_value_default() {
        let value = PodcastValue::default();
        assert!(value.type_.is_empty());
        assert!(value.method.is_empty());
        assert!(value.suggested.is_none());
        assert!(value.recipients.is_empty());
        assert!(value.time_splits.is_empty());
    }

    #[test]
    fn test_podcast_value_lightning() {
        let value = PodcastValue {
            type_: "lightning".to_string(),
            method: "keysend".to_string(),
            suggested: Some("0.00000005000".to_string()),
            recipients: vec![
                PodcastValueRecipient {
                    name: Some("Host".to_string()),
                    type_: "node".to_string(),
                    address: "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a"
                        .to_string(),
                    split: 90,
                    fee: Some(false),
                },
                PodcastValueRecipient {
                    name: Some("Producer".to_string()),
                    type_: "node".to_string(),
                    address: "02d5c1bf8b940dc9cadca86d1b0a3c37fbe39cee4c7e839e33bef9174531d27f52"
                        .to_string(),
                    split: 10,
                    fee: Some(false),
                },
            ],
            time_splits: vec![],
        };

        assert_eq!(value.type_, "lightning");
        assert_eq!(value.method, "keysend");
        assert_eq!(value.suggested.as_deref(), Some("0.00000005000"));
        assert_eq!(value.recipients.len(), 2);
        assert_eq!(value.recipients[0].split, 90);
        assert_eq!(value.recipients[1].split, 10);
    }

    #[test]
    fn test_podcast_value_recipient_default() {
        let recipient = PodcastValueRecipient::default();
        assert!(recipient.name.is_none());
        assert!(recipient.type_.is_empty());
        assert!(recipient.address.is_empty());
        assert_eq!(recipient.split, 0);
        assert!(recipient.fee.is_none());
    }

    #[test]
    fn test_podcast_value_recipient_with_fee() {
        let recipient = PodcastValueRecipient {
            name: Some("Hosting Provider".to_string()),
            type_: "node".to_string(),
            address: "02d5c1bf8b940dc9cadca86d1b0a3c37fbe39cee4c7e839e33bef9174531d27f52"
                .to_string(),
            split: 5,
            fee: Some(true),
        };

        assert_eq!(recipient.name.as_deref(), Some("Hosting Provider"));
        assert_eq!(recipient.split, 5);
        assert_eq!(recipient.fee, Some(true));
    }

    #[test]
    fn test_podcast_value_recipient_without_name() {
        let recipient = PodcastValueRecipient {
            name: None,
            type_: "node".to_string(),
            address: "03ae9f91a0cb8ff43840e3c322c4c61f019d8c1c3cea15a25cfc425ac605e61a4a"
                .to_string(),
            split: 100,
            fee: Some(false),
        };

        assert!(recipient.name.is_none());
        assert_eq!(recipient.split, 100);
    }

    #[test]
    fn test_podcast_value_multiple_recipients() {
        let mut value = PodcastValue {
            type_: "lightning".to_string(),
            method: "keysend".to_string(),
            suggested: None,
            recipients: Vec::new(),
            time_splits: vec![],
        };

        // Add multiple recipients
        for i in 1..=5 {
            value.recipients.push(PodcastValueRecipient {
                name: Some(format!("Recipient {i}")),
                type_: "node".to_string(),
                address: format!("address_{i}"),
                split: 20,
                fee: Some(false),
            });
        }

        assert_eq!(value.recipients.len(), 5);
        assert_eq!(value.recipients.iter().map(|r| r.split).sum::<u32>(), 100);
    }

    #[test]
    fn test_podcast_value_hive() {
        let value = PodcastValue {
            type_: "hive".to_string(),
            method: "direct".to_string(),
            suggested: Some("1.00000".to_string()),
            recipients: vec![PodcastValueRecipient {
                name: Some("@username".to_string()),
                type_: "account".to_string(),
                address: "username".to_string(),
                split: 100,
                fee: Some(false),
            }],
            time_splits: vec![],
        };

        assert_eq!(value.type_, "hive");
        assert_eq!(value.method, "direct");
    }

    #[test]
    fn test_podcast_meta_with_value() {
        let mut meta = PodcastMeta::default();
        assert!(meta.value.is_none());

        meta.value = Some(PodcastValue {
            type_: "lightning".to_string(),
            method: "keysend".to_string(),
            suggested: Some("0.00000005000".to_string()),
            recipients: vec![],
            time_splits: vec![],
        });

        assert!(meta.value.is_some());
        assert_eq!(meta.value.as_ref().unwrap().type_, "lightning");
    }

    #[test]
    #[allow(clippy::redundant_clone)]
    fn test_podcast_value_clone() {
        let value = PodcastValue {
            type_: "lightning".to_string(),
            method: "keysend".to_string(),
            suggested: Some("0.00000005000".to_string()),
            recipients: vec![PodcastValueRecipient {
                name: Some("Host".to_string()),
                type_: "node".to_string(),
                address: "abc123".to_string(),
                split: 100,
                fee: Some(false),
            }],
            time_splits: vec![],
        };

        let cloned = value.clone();
        assert_eq!(cloned.type_, "lightning");
        assert_eq!(cloned.recipients.len(), 1);
        assert_eq!(cloned.recipients[0].name.as_deref(), Some("Host"));
    }
}
