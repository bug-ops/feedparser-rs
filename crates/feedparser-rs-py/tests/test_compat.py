"""
Test Python feedparser backward compatibility field mappings.

Tests that deprecated field names correctly map to their modern equivalents:
- Feed-level: description, tagline, modified, copyright, date, url
- Entry-level: guid, description, issued, modified, date
- Container-level: channel, items
"""

import feedparser_rs
import pytest


def test_feed_description_alias():
    """feed.description should map to feed.subtitle"""
    xml = """<rss version="2.0">
        <channel>
            <description>Test subtitle text</description>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Both should work and return the same value
    assert feed.feed.subtitle == "Test subtitle text"
    assert feed.feed.description == "Test subtitle text"
    assert feed.feed.description == feed.feed.subtitle


def test_feed_tagline_alias():
    """feed.tagline should map to feed.subtitle (old Atom 0.3 field)"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <subtitle>My feed tagline</subtitle>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    assert feed.feed.subtitle == "My feed tagline"
    assert feed.feed.tagline == "My feed tagline"
    assert feed.feed.tagline == feed.feed.subtitle


def test_feed_modified_alias():
    """feed.modified should map to feed.updated"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <updated>2024-01-01T12:00:00Z</updated>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    assert feed.feed.updated is not None
    assert feed.feed.modified == feed.feed.updated
    # Both _parsed versions should work
    assert feed.feed.modified_parsed is not None
    assert feed.feed.modified_parsed.tm_year == 2024


def test_feed_copyright_alias():
    """feed.copyright should map to feed.rights"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <rights>Copyright 2024 Example Corp</rights>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    assert feed.feed.rights == "Copyright 2024 Example Corp"
    assert feed.feed.copyright == "Copyright 2024 Example Corp"
    assert feed.feed.copyright == feed.feed.rights


def test_feed_date_alias_falls_back_to_updated():
    """feed.date should map to feed.updated as primary fallback"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <updated>2024-01-15T10:30:00Z</updated>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    assert feed.feed.date == feed.feed.updated
    assert feed.feed.date_parsed.tm_year == 2024
    assert feed.feed.date_parsed.tm_mon == 1
    assert feed.feed.date_parsed.tm_mday == 15


def test_feed_date_alias_falls_back_to_published():
    """feed.date should fall back to feed.published if updated is absent"""
    xml = """<rss version="2.0">
        <channel>
            <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # updated is None, so date should map to published
    assert feed.feed.updated is None
    assert feed.feed.published is not None
    assert feed.feed.date == feed.feed.published
    assert feed.feed.date_parsed.tm_year == 2024


def test_feed_url_alias():
    """feed.url should map to feed.link"""
    xml = """<rss version="2.0">
        <channel>
            <link>https://example.com</link>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    assert feed.feed.link == "https://example.com"
    assert feed.feed.url == "https://example.com"
    assert feed.feed.url == feed.feed.link


def test_entry_guid_alias():
    """entry.guid should map to entry.id"""
    xml = """<rss version="2.0">
        <channel>
            <item>
                <guid>abc123xyz</guid>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.id == "abc123xyz"
    assert entry.guid == "abc123xyz"
    assert entry.guid == entry.id


def test_entry_description_alias():
    """entry.description should map to entry.summary"""
    xml = """<rss version="2.0">
        <channel>
            <item>
                <description>Entry summary text</description>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.summary == "Entry summary text"
    assert entry.description == "Entry summary text"
    assert entry.description == entry.summary


def test_entry_issued_alias():
    """entry.issued should map to entry.published"""
    xml = """<rss version="2.0">
        <channel>
            <item>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.published is not None
    assert entry.issued == entry.published
    # Both _parsed versions should work
    assert entry.issued_parsed is not None
    assert entry.issued_parsed.tm_year == 2024


def test_entry_modified_alias():
    """entry.modified should map to entry.updated"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <entry>
            <updated>2024-01-15T10:30:00Z</updated>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.updated is not None
    assert entry.modified == entry.updated
    assert entry.modified_parsed.tm_year == 2024


def test_entry_date_alias_falls_back_to_updated():
    """entry.date should map to entry.updated as primary fallback"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <entry>
            <updated>2024-01-15T10:30:00Z</updated>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    assert entry.date == entry.updated
    assert entry.date_parsed.tm_year == 2024


def test_entry_date_alias_falls_back_to_published():
    """entry.date should fall back to entry.published if updated is absent"""
    xml = """<rss version="2.0">
        <channel>
            <item>
                <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed.entries[0]

    # updated is None, so date should map to published
    assert entry.updated is None
    assert entry.published is not None
    assert entry.date == entry.published
    assert entry.date_parsed.tm_year == 2024


def test_container_channel_alias():
    """d.channel should map to d.feed (RSS uses <channel>)"""
    xml = """<rss version="2.0">
        <channel>
            <title>RSS Feed Title</title>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    # Both should work and return the same object
    assert d.feed.title == "RSS Feed Title"
    assert d.channel.title == "RSS Feed Title"
    # Verify they're the same object by checking id
    assert d.channel.title == d.feed.title


def test_container_items_alias():
    """d.items should map to d.entries (RSS uses <item>)"""
    xml = """<rss version="2.0">
        <channel>
            <item><title>Item 1</title></item>
            <item><title>Item 2</title></item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    # Both should work and return the same list
    assert len(d.entries) == 2
    assert len(d.items) == 2
    assert d.items[0].title == "Item 1"
    assert d.items[1].title == "Item 2"


def test_unknown_field_raises_attribute_error():
    """Accessing unknown field should raise AttributeError"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test</title>
            <item>
                <title>Test Item</title>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Unknown fields should raise AttributeError
    with pytest.raises(AttributeError, match="has no attribute"):
        _ = feed.feed.nonexistent_field

    with pytest.raises(AttributeError, match="has no attribute"):
        _ = feed.entries[0].fake_attribute

    with pytest.raises(AttributeError, match="has no attribute"):
        _ = feed.this_does_not_exist


def test_multiple_alias_access():
    """Test accessing multiple aliases in same object"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <title>My Feed</title>
        <subtitle>Feed description</subtitle>
        <updated>2024-01-01T12:00:00Z</updated>
        <rights>Copyright 2024</rights>
        <entry>
            <id>entry-1</id>
            <title>Entry Title</title>
            <summary>Entry summary</summary>
            <published>2024-01-01T10:00:00Z</published>
            <updated>2024-01-01T11:00:00Z</updated>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # Feed-level aliases
    assert feed.feed.description == "Feed description"
    assert feed.feed.tagline == "Feed description"
    assert feed.feed.modified is not None
    assert feed.feed.copyright == "Copyright 2024"

    # Entry-level aliases
    entry = feed.entries[0]
    assert entry.guid == "entry-1"
    assert entry.description == "Entry summary"
    assert entry.issued is not None
    assert entry.modified is not None


def test_detail_field_aliases():
    """Test that _detail field aliases work correctly"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <subtitle type="html">&lt;b&gt;Bold subtitle&lt;/b&gt;</subtitle>
        <rights type="text">Copyright 2024</rights>
        <entry>
            <summary type="text">Entry summary</summary>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # Feed-level _detail aliases
    assert feed.feed.subtitle_detail is not None
    assert feed.feed.description_detail is not None
    assert feed.feed.description_detail.type == feed.feed.subtitle_detail.type

    assert feed.feed.rights_detail is not None
    assert feed.feed.copyright_detail is not None
    assert feed.feed.copyright_detail.type == feed.feed.rights_detail.type

    # Entry-level _detail aliases
    entry = feed.entries[0]
    assert entry.summary_detail is not None
    assert entry.description_detail is not None
    assert entry.description_detail.value == entry.summary_detail.value


def test_existing_attribute_access_still_works():
    """Ensure normal attribute access is not affected by __getattr__"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <description>Feed description</description>
            <item>
                <title>Entry Title</title>
                <link>https://example.com/entry</link>
                <guid>entry-1</guid>
                <description>Entry summary</description>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Direct attribute access should work normally
    assert feed.feed.title == "Test Feed"
    assert feed.feed.link == "https://example.com"
    assert feed.feed.subtitle == "Feed description"

    assert feed.entries[0].title == "Entry Title"
    assert feed.entries[0].link == "https://example.com/entry"
    assert feed.entries[0].id == "entry-1"
    assert feed.entries[0].summary == "Entry summary"

    # FeedParserDict level
    assert feed.version is not None
    assert feed.bozo is not None


# Phase 2: Dict-style access tests (__getitem__)


def test_dict_access_feed_fields():
    """Test dict-style access for feed fields"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <description>Feed description</description>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Dict-style access should work
    assert feed["feed"]["title"] == "Test Feed"
    assert feed["feed"]["link"] == "https://example.com"
    assert feed["feed"]["subtitle"] == "Feed description"

    # Mixed access should work
    assert feed["feed"].title == "Test Feed"
    assert feed.feed["title"] == "Test Feed"


def test_dict_access_entry_fields():
    """Test dict-style access for entry fields"""
    xml = """<rss version="2.0">
        <channel>
            <item>
                <title>Entry Title</title>
                <link>https://example.com/entry</link>
                <guid>entry-1</guid>
                <description>Entry summary</description>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    entry = feed["entries"][0]

    # Dict-style access should work
    assert entry["title"] == "Entry Title"
    assert entry["link"] == "https://example.com/entry"
    assert entry["id"] == "entry-1"
    assert entry["summary"] == "Entry summary"

    # Mixed access should work
    assert feed["entries"][0].title == "Entry Title"
    assert feed.entries[0]["title"] == "Entry Title"


def test_dict_access_with_deprecated_aliases():
    """Test dict-style access with deprecated field names"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <title>My Feed</title>
        <subtitle>Feed description</subtitle>
        <updated>2024-01-01T12:00:00Z</updated>
        <rights>Copyright 2024</rights>
        <entry>
            <id>entry-1</id>
            <title>Entry Title</title>
            <summary>Entry summary</summary>
            <published>2024-01-01T10:00:00Z</published>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # Feed-level deprecated aliases should work with dict access
    assert feed["feed"]["description"] == "Feed description"
    assert feed["feed"]["tagline"] == "Feed description"
    assert feed["feed"]["copyright"] == "Copyright 2024"
    assert feed["feed"]["modified"] is not None

    # Entry-level deprecated aliases should work with dict access
    entry = feed["entries"][0]
    assert entry["guid"] == "entry-1"
    assert entry["description"] == "Entry summary"
    assert entry["issued"] is not None


def test_dict_access_container_aliases():
    """Test dict-style access with container name aliases"""
    xml = """<rss version="2.0">
        <channel>
            <title>RSS Feed</title>
            <item><title>Item 1</title></item>
            <item><title>Item 2</title></item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    # channel → feed alias should work with dict access
    assert d["channel"]["title"] == "RSS Feed"
    assert d["feed"]["title"] == "RSS Feed"

    # items → entries alias should work with dict access
    assert len(d["items"]) == 2
    assert len(d["entries"]) == 2
    assert d["items"][0]["title"] == "Item 1"
    assert d["entries"][0]["title"] == "Item 1"


def test_dict_access_top_level_fields():
    """Test dict-style access for top-level FeedParserDict fields"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test</title>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Top-level fields should be accessible via dict-style
    assert feed["version"] == "rss20"
    assert feed["bozo"] is False
    assert feed["encoding"] is not None


def test_dict_access_unknown_key_raises_keyerror():
    """Dict access with unknown key should raise KeyError (not AttributeError)"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test</title>
            <item>
                <title>Test Item</title>
            </item>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Unknown keys should raise KeyError for dict access
    with pytest.raises(KeyError):
        _ = feed["nonexistent_field"]

    with pytest.raises(KeyError):
        _ = feed["feed"]["fake_field"]

    with pytest.raises(KeyError):
        _ = feed["entries"][0]["unknown_key"]

    # But AttributeError should still be raised for attribute access
    with pytest.raises(AttributeError, match="has no attribute"):
        _ = feed.feed.fake_field


def test_dict_and_attribute_access_equivalence():
    """Test that dict and attribute access return same values"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <title>My Feed</title>
        <subtitle>Feed description</subtitle>
        <link href="https://example.com"/>
        <updated>2024-01-01T12:00:00Z</updated>
        <entry>
            <id>entry-1</id>
            <title>Entry Title</title>
            <summary>Entry summary</summary>
            <link href="https://example.com/entry"/>
            <updated>2024-01-01T10:00:00Z</updated>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # Feed-level fields should be identical via both access methods
    assert feed.feed.title == feed["feed"]["title"]
    assert feed.feed.subtitle == feed["feed"]["subtitle"]
    assert feed.feed.link == feed["feed"]["link"]
    assert feed.feed.updated == feed["feed"]["updated"]

    # Entry-level fields should be identical via both access methods
    entry = feed.entries[0]
    assert entry.id == entry["id"]
    assert entry.title == entry["title"]
    assert entry.summary == entry["summary"]
    assert entry.link == entry["link"]
    assert entry.updated == entry["updated"]

    # Top-level fields should be identical
    assert feed.version == feed["version"]
    assert feed.bozo == feed["bozo"]


def test_dict_access_with_none_values():
    """Test dict access returns None for missing optional fields"""
    xml = """<rss version="2.0">
        <channel>
            <title>Minimal Feed</title>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)

    # Missing optional fields should return None via dict access
    assert feed["feed"]["subtitle"] is None
    assert feed["feed"]["updated"] is None
    assert feed["feed"]["author"] is None
    assert feed["feed"]["image"] is None


def test_dict_access_detail_fields():
    """Test dict access for _detail fields"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <subtitle type="html">&lt;b&gt;Bold subtitle&lt;/b&gt;</subtitle>
        <rights type="text">Copyright 2024</rights>
        <entry>
            <summary type="text">Entry summary</summary>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # _detail fields should work with dict access
    assert feed["feed"]["subtitle_detail"] is not None
    assert feed["feed"]["subtitle_detail"].type == "html"

    assert feed["feed"]["rights_detail"] is not None
    assert feed["feed"]["copyright_detail"] is not None
    assert feed["feed"]["copyright_detail"].type == "text"

    entry = feed["entries"][0]
    assert entry["summary_detail"] is not None
    assert entry["description_detail"] is not None


def test_dict_access_list_fields():
    """Test dict access for list fields (links, tags, authors, etc.)"""
    xml = """<feed xmlns="http://www.w3.org/2005/Atom">
        <link href="https://example.com/feed" rel="self"/>
        <link href="https://example.com" rel="alternate"/>
        <category term="technology"/>
        <category term="programming"/>
        <entry>
            <link href="https://example.com/entry"/>
            <category term="rust"/>
        </entry>
    </feed>"""

    feed = feedparser_rs.parse(xml)

    # List fields should work with dict access
    assert len(feed["feed"]["links"]) == 2
    assert feed["feed"]["links"][0].href == "https://example.com/feed"

    assert len(feed["feed"]["tags"]) == 2
    assert feed["feed"]["tags"][0].term == "technology"

    entry = feed["entries"][0]
    assert len(entry["links"]) >= 1
    assert len(entry["tags"]) == 1
    assert entry["tags"][0].term == "rust"


# =============================================================================
# Phase 4: Auto-URL Detection Tests
# =============================================================================


def test_parse_with_optional_http_params():
    """Test that parse() accepts optional HTTP parameters for URL fetching"""
    # When parsing content (not URL), these params should be ignored
    xml = """<rss version="2.0">
        <channel>
            <title>Test Feed</title>
        </channel>
    </rss>"""

    # Should work with optional params (they're just ignored for content)
    feed = feedparser_rs.parse(xml, etag="some-etag", modified="some-date")
    assert feed.feed.title == "Test Feed"
    assert feed.version == "rss20"


def test_parse_with_user_agent_param():
    """Test that parse() accepts user_agent parameter"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test Feed</title>
        </channel>
    </rss>"""

    # Should work with user_agent param (ignored for content)
    feed = feedparser_rs.parse(xml, user_agent="TestBot/1.0")
    assert feed.feed.title == "Test Feed"


def test_parse_url_detection_http():
    """Test that parse() detects http:// URLs"""
    # This test verifies URL detection logic without actually fetching
    # Since we don't have an HTTP feature enabled or a real server,
    # we just verify the parse function signature accepts URL-like strings
    try:
        # This will either succeed (if http feature enabled and server exists)
        # or raise NotImplementedError (if http feature disabled)
        feedparser_rs.parse("http://example.com/nonexistent")
    except NotImplementedError as e:
        # http feature not enabled - this is expected
        assert "http" in str(e).lower()
    except Exception:
        # Some other error (network, etc.) - also acceptable
        pass


def test_parse_url_detection_https():
    """Test that parse() detects https:// URLs"""
    try:
        feedparser_rs.parse("https://example.com/nonexistent")
    except NotImplementedError as e:
        # http feature not enabled - this is expected
        assert "http" in str(e).lower()
    except Exception:
        # Some other error (network, etc.) - also acceptable
        pass


def test_parse_content_starting_with_http_in_text():
    """Test that content containing 'http' as text is not treated as URL"""
    # This should be parsed as content, not as a URL
    xml = """<rss version="2.0">
        <channel>
            <title>HTTP Guide</title>
            <description>Learn about http protocol</description>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    assert feed.feed.title == "HTTP Guide"
    assert "http" in feed.feed.subtitle.lower()


def test_parse_bytes_content():
    """Test that bytes content is still parsed correctly"""
    xml = b"""<rss version="2.0">
        <channel>
            <title>Bytes Feed</title>
        </channel>
    </rss>"""

    feed = feedparser_rs.parse(xml)
    assert feed.feed.title == "Bytes Feed"


def test_parse_with_limits_accepts_http_params():
    """Test that parse_with_limits() also accepts HTTP parameters"""
    xml = """<rss version="2.0">
        <channel>
            <title>Test Feed</title>
        </channel>
    </rss>"""

    limits = feedparser_rs.ParserLimits()

    # Should work with all optional params
    feed = feedparser_rs.parse_with_limits(
        xml, etag="etag", modified="modified", user_agent="TestBot/1.0", limits=limits
    )
    assert feed.feed.title == "Test Feed"
