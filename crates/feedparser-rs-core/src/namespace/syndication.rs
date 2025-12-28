/// Syndication Module for RSS 1.0
///
/// Namespace: <http://purl.org/rss/1.0/modules/syndication/>
/// Prefix: syn
///
/// This module provides parsing support for the Syndication namespace,
/// used in RSS 1.0 feeds to indicate update schedules and frequencies.
///
/// Elements:
/// - `syn:updatePeriod` → Update period (hourly, daily, weekly, monthly, yearly)
/// - `syn:updateFrequency` → Number of times per period
/// - `syn:updateBase` → Base date for update schedule (ISO 8601)
use crate::types::FeedMeta;

/// Syndication namespace URI
pub const SYNDICATION_NAMESPACE: &str = "http://purl.org/rss/1.0/modules/syndication/";

/// Valid update period values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePeriod {
    /// Update hourly
    Hourly,
    /// Update daily
    Daily,
    /// Update weekly
    Weekly,
    /// Update monthly
    Monthly,
    /// Update yearly
    Yearly,
}

impl UpdatePeriod {
    /// Parse update period from string (case-insensitive)
    ///
    /// Returns `None` if the string doesn't match any valid period.
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hourly" => Some(Self::Hourly),
            "daily" => Some(Self::Daily),
            "weekly" => Some(Self::Weekly),
            "monthly" => Some(Self::Monthly),
            "yearly" => Some(Self::Yearly),
            _ => None,
        }
    }

    /// Convert to string representation
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
        }
    }
}

/// Syndication metadata
#[derive(Debug, Clone, Default)]
pub struct SyndicationMeta {
    /// Update period (hourly, daily, weekly, monthly, yearly)
    pub update_period: Option<UpdatePeriod>,
    /// Number of times updated per period
    pub update_frequency: Option<u32>,
    /// Base date for update schedule (ISO 8601)
    pub update_base: Option<String>,
}

/// Handle Syndication namespace element at feed level
///
/// # Arguments
///
/// * `element` - Local name of the element (without namespace prefix)
/// * `text` - Text content of the element
/// * `feed` - Feed metadata to update
pub fn handle_feed_element(element: &str, text: &str, feed: &mut FeedMeta) {
    match element {
        "updatePeriod" => {
            if let Some(period) = UpdatePeriod::parse(text) {
                if feed.syndication.is_none() {
                    feed.syndication = Some(Box::new(SyndicationMeta::default()));
                }
                if let Some(syn) = &mut feed.syndication {
                    syn.update_period = Some(period);
                }
            }
        }
        "updateFrequency" => {
            if let Ok(freq) = text.parse::<u32>() {
                if feed.syndication.is_none() {
                    feed.syndication = Some(Box::new(SyndicationMeta::default()));
                }
                if let Some(syn) = &mut feed.syndication {
                    syn.update_frequency = Some(freq);
                }
            }
        }
        "updateBase" => {
            if feed.syndication.is_none() {
                feed.syndication = Some(Box::new(SyndicationMeta::default()));
            }
            if let Some(syn) = &mut feed.syndication {
                syn.update_base = Some(text.to_string());
            }
        }
        _ => {
            // Ignore unknown syndication elements
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_period_parse() {
        assert_eq!(UpdatePeriod::parse("hourly"), Some(UpdatePeriod::Hourly));
        assert_eq!(UpdatePeriod::parse("daily"), Some(UpdatePeriod::Daily));
        assert_eq!(UpdatePeriod::parse("weekly"), Some(UpdatePeriod::Weekly));
        assert_eq!(UpdatePeriod::parse("monthly"), Some(UpdatePeriod::Monthly));
        assert_eq!(UpdatePeriod::parse("yearly"), Some(UpdatePeriod::Yearly));
        assert_eq!(UpdatePeriod::parse("invalid"), None);
    }

    #[test]
    fn test_update_period_case_insensitive() {
        assert_eq!(UpdatePeriod::parse("HOURLY"), Some(UpdatePeriod::Hourly));
        assert_eq!(UpdatePeriod::parse("Daily"), Some(UpdatePeriod::Daily));
        assert_eq!(UpdatePeriod::parse("WeeKLY"), Some(UpdatePeriod::Weekly));
    }

    #[test]
    fn test_update_period_as_str() {
        assert_eq!(UpdatePeriod::Hourly.as_str(), "hourly");
        assert_eq!(UpdatePeriod::Daily.as_str(), "daily");
        assert_eq!(UpdatePeriod::Weekly.as_str(), "weekly");
        assert_eq!(UpdatePeriod::Monthly.as_str(), "monthly");
        assert_eq!(UpdatePeriod::Yearly.as_str(), "yearly");
    }

    #[test]
    fn test_handle_update_period() {
        let mut feed = FeedMeta::default();

        handle_feed_element("updatePeriod", "daily", &mut feed);

        assert!(feed.syndication.is_some());
        let syn = feed.syndication.as_ref().unwrap();
        assert_eq!(syn.update_period, Some(UpdatePeriod::Daily));
    }

    #[test]
    fn test_handle_update_frequency() {
        let mut feed = FeedMeta::default();

        handle_feed_element("updateFrequency", "2", &mut feed);

        assert!(feed.syndication.is_some());
        let syn = feed.syndication.as_ref().unwrap();
        assert_eq!(syn.update_frequency, Some(2));
    }

    #[test]
    fn test_handle_update_base() {
        let mut feed = FeedMeta::default();

        handle_feed_element("updateBase", "2024-12-18T00:00:00Z", &mut feed);

        assert!(feed.syndication.is_some());
        let syn = feed.syndication.as_ref().unwrap();
        assert_eq!(syn.update_base.as_deref(), Some("2024-12-18T00:00:00Z"));
    }

    #[test]
    fn test_handle_multiple_elements() {
        let mut feed = FeedMeta::default();

        handle_feed_element("updatePeriod", "hourly", &mut feed);
        handle_feed_element("updateFrequency", "1", &mut feed);
        handle_feed_element("updateBase", "2024-01-01T00:00:00Z", &mut feed);

        let syn = feed.syndication.as_ref().unwrap();
        assert_eq!(syn.update_period, Some(UpdatePeriod::Hourly));
        assert_eq!(syn.update_frequency, Some(1));
        assert_eq!(syn.update_base.as_deref(), Some("2024-01-01T00:00:00Z"));
    }

    #[test]
    fn test_handle_invalid_frequency() {
        let mut feed = FeedMeta::default();

        handle_feed_element("updateFrequency", "not-a-number", &mut feed);

        // Should not create syndication metadata for invalid input
        assert!(feed.syndication.is_none());
    }

    #[test]
    fn test_handle_unknown_element() {
        let mut feed = FeedMeta::default();

        handle_feed_element("unknown", "value", &mut feed);

        assert!(feed.syndication.is_none());
    }
}
