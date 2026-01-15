"""Basic parsing tests for feedparser_rs"""

import sys
import time

import pytest

# Import the Rust extension directly for testing
sys.path.insert(0, "../python")
import feedparser_rs


def test_parse_rss20_basic():
    """Test parsing a basic RSS 2.0 feed"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <description>A test RSS feed</description>
            <item>
                <title>Test Item</title>
                <link>https://example.com/item1</link>
                <description>Test description</description>
                <pubDate>Mon, 15 Dec 2025 10:00:00 +0000</pubDate>
            </item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    assert d.version == "rss20"
    assert d.feed.title == "Test Feed"
    assert d.feed.link == "https://example.com"
    assert len(d.entries) == 1
    assert d.entries[0].title == "Test Item"
    assert d.entries[0].link == "https://example.com/item1"
    assert not d.bozo


def test_parse_atom10_basic():
    """Test parsing a basic Atom 1.0 feed"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
        <link href="https://example.com"/>
        <updated>2025-12-15T10:00:00Z</updated>
        <entry>
            <title>Test Entry</title>
            <link href="https://example.com/entry1"/>
            <published>2025-12-15T10:00:00Z</published>
            <summary>Test summary</summary>
        </entry>
    </feed>"""

    d = feedparser_rs.parse(xml)

    assert d.version == "atom10"
    assert d.feed.title == "Test Feed"
    assert len(d.entries) == 1
    assert d.entries[0].title == "Test Entry"


def test_parse_from_string():
    """Test parsing from string (not just bytes)"""
    xml = '<rss version="2.0"><channel><title>Test</title></channel></rss>'

    d = feedparser_rs.parse(xml)

    assert d.version == "rss20"
    assert d.feed.title == "Test"


def test_bozo_flag_malformed():
    """Test that malformed XML sets bozo flag"""
    xml = b"<rss><channel><title>Broken</title></rss>"  # Missing </channel>

    d = feedparser_rs.parse(xml)

    # Should still parse but set bozo flag
    assert d.bozo
    assert d.bozo_exception is not None


def test_datetime_struct_time():
    """Test that published_parsed returns time.struct_time"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <item>
                <pubDate>Mon, 15 Dec 2025 14:30:00 +0000</pubDate>
            </item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)
    parsed = d.entries[0].published_parsed

    # Must be time.struct_time
    assert isinstance(parsed, time.struct_time)
    assert parsed.tm_year == 2025
    assert parsed.tm_mon == 12
    assert parsed.tm_mday == 15
    assert parsed.tm_hour == 14
    assert parsed.tm_min == 30
    assert parsed.tm_sec == 0


def test_datetime_none():
    """Test that missing dates return None"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <item><title>No Date</title></item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)
    assert d.entries[0].published_parsed is None


def test_encoding():
    """Test encoding detection"""
    xml = b'<?xml version="1.0" encoding="utf-8"?><rss version="2.0"><channel><title>Test</title></channel></rss>'

    d = feedparser_rs.parse(xml)

    assert d.encoding == "utf-8"


def test_parse_with_limits():
    """Test parsing with custom limits"""
    xml = b'<rss version="2.0"><channel><title>Test</title></channel></rss>'

    limits = feedparser_rs.ParserLimits(
        max_feed_size_bytes=1000,
        max_entries=10,
    )

    d = feedparser_rs.parse_with_limits(xml, limits=limits)
    assert d.version == "rss20"


def test_parse_with_limits_exceeded():
    """Test that exceeding limits raises error"""
    xml = b'<rss version="2.0"><channel><title>Test</title></channel></rss>'

    limits = feedparser_rs.ParserLimits(
        max_feed_size_bytes=10,  # Too small
    )

    with pytest.raises(ValueError, match="exceeds maximum"):
        feedparser_rs.parse_with_limits(xml, limits=limits)


def test_detect_format_rss20():
    """Test format detection for RSS 2.0"""
    xml = b'<rss version="2.0"><channel></channel></rss>'
    assert feedparser_rs.detect_format(xml) == "rss20"


def test_detect_format_atom10():
    """Test format detection for Atom 1.0"""
    xml = b'<feed xmlns="http://www.w3.org/2005/Atom"></feed>'
    assert feedparser_rs.detect_format(xml) == "atom10"


def test_detect_format_json():
    """Test format detection for JSON Feed"""
    json_feed = b'{"version": "https://jsonfeed.org/version/1.1", "title": "Test"}'
    version = feedparser_rs.detect_format(json_feed)
    assert version in ["json10", "json11"]


def test_multiple_entries():
    """Test parsing feed with multiple entries"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test</title>
            <item><title>Entry 1</title></item>
            <item><title>Entry 2</title></item>
            <item><title>Entry 3</title></item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    assert len(d.entries) == 3
    assert d.entries[0].title == "Entry 1"
    assert d.entries[1].title == "Entry 2"
    assert d.entries[2].title == "Entry 3"


def test_podcast_itunes_metadata():
    """Test parsing iTunes podcast metadata"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
        <channel>
            <title>Test Podcast</title>
            <itunes:author>John Doe</itunes:author>
            <itunes:explicit>false</itunes:explicit>
            <item>
                <title>Episode 1</title>
                <itunes:duration>3600</itunes:duration>
                <itunes:episode>1</itunes:episode>
                <itunes:season>1</itunes:season>
            </item>
        </channel>
    </rss>"""

    d = feedparser_rs.parse(xml)

    # Feed-level iTunes metadata
    assert d.feed.itunes is not None
    assert d.feed.itunes.author == "John Doe"
    assert d.feed.itunes.explicit is False

    # Entry-level iTunes metadata
    assert d.entries[0].itunes is not None
    assert d.entries[0].itunes.duration == 3600
    assert d.entries[0].itunes.episode == 1
    assert d.entries[0].itunes.season == 1


def test_repr_methods():
    """Test __repr__ methods for debugging"""
    xml = b'<rss version="2.0"><channel><title>Test</title><item><title>Entry</title></item></channel></rss>'

    d = feedparser_rs.parse(xml)

    # Should have useful repr
    assert "FeedParserDict" in repr(d)
    assert "rss20" in repr(d)
    assert "FeedMeta" in repr(d.feed)
    assert "Entry" in repr(d.entries[0])


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
