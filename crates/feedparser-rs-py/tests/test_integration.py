"""Integration tests for Phase 1: published_parsed and URL resolution"""

import time

import feedparser_rs


def test_feed_published_parsed_atom():
    """Test feed.published_parsed returns time.struct_time for Atom"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
        <published>2025-01-01T00:00:00Z</published>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.feed.published is not None
    assert result.feed.published_parsed is not None
    assert isinstance(result.feed.published_parsed, time.struct_time)
    assert result.feed.published_parsed.tm_year == 2025
    assert result.feed.published_parsed.tm_mon == 1
    assert result.feed.published_parsed.tm_mday == 1
    assert result.feed.published_parsed.tm_isdst == 0  # UTC


def test_feed_published_parsed_rss():
    """Test RSS channel pubDate maps to feed.published_parsed"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <title>Test Podcast</title>
            <pubDate>Wed, 18 Dec 2024 10:00:00 +0000</pubDate>
        </channel>
    </rss>
    """

    result = feedparser_rs.parse(xml)
    assert result.feed.published_parsed is not None
    assert result.feed.published_parsed.tm_year == 2024
    assert result.feed.published_parsed.tm_mon == 12
    assert result.feed.published_parsed.tm_mday == 18


def test_feed_updated_parsed_still_works():
    """Test feed.updated_parsed still works (backwards compatibility)"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
        <updated>2024-12-18T10:30:00Z</updated>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.feed.updated_parsed is not None
    assert isinstance(result.feed.updated_parsed, time.struct_time)
    assert result.feed.updated_parsed.tm_year == 2024
    assert result.feed.updated_parsed.tm_mon == 12
    assert result.feed.updated_parsed.tm_mday == 18
    assert result.feed.updated_parsed.tm_isdst == 0


def test_entry_all_parsed_dates():
    """Test entry has all *_parsed date fields"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test</title>
        <entry>
            <title>Entry 1</title>
            <published>2025-01-01T12:00:00Z</published>
            <updated>2025-01-02T12:00:00Z</updated>
        </entry>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    entry = result.entries[0]

    assert entry.published_parsed is not None
    assert entry.published_parsed.tm_year == 2025
    assert entry.published_parsed.tm_mon == 1

    assert entry.updated_parsed is not None
    assert entry.updated_parsed.tm_mday == 2


def test_missing_dates_return_none():
    """Test that missing dates return None, not crash"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom">
        <title>Test Feed</title>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.feed.updated_parsed is None
    assert result.feed.published_parsed is None


def test_atom_xml_base_resolution():
    """Test relative URLs resolved against xml:base"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/blog/">
        <title>Test Feed</title>
        <link href="index.html" rel="self" />
        <entry>
            <title>Post 1</title>
            <link href="posts/first.html" rel="alternate" />
        </entry>
    </feed>
    """

    result = feedparser_rs.parse(xml)

    # Feed link resolved
    assert result.feed.links[0].href == "http://example.org/blog/index.html"

    # Entry link resolved
    assert result.entries[0].link == "http://example.org/blog/posts/first.html"


def test_nested_xml_base():
    """Test nested xml:base combines with parent"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <entry xml:base="posts/">
            <link href="123.html" />
        </entry>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.entries[0].link == "http://example.org/posts/123.html"


def test_rss_relative_links():
    """Test RSS links resolved against channel link"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <link>http://example.org/</link>
            <item>
                <title>Episode 1</title>
                <link>episodes/ep1.html</link>
            </item>
        </channel>
    </rss>
    """

    result = feedparser_rs.parse(xml)
    assert result.entries[0].link == "http://example.org/episodes/ep1.html"


def test_rss_relative_enclosures():
    """Test RSS enclosures resolved against channel link"""
    xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
        <channel>
            <link>http://podcast.example.com/</link>
            <item>
                <enclosure url="episodes/ep1.mp3" type="audio/mpeg" length="12345" />
            </item>
        </channel>
    </rss>
    """

    result = feedparser_rs.parse(xml)
    assert result.entries[0].enclosures[0].url == "http://podcast.example.com/episodes/ep1.mp3"


def test_absolute_urls_not_modified():
    """Test absolute URLs remain unchanged"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <entry>
            <link href="http://absolute.com/page" />
        </entry>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.entries[0].link == "http://absolute.com/page"


def test_atom_feed_icons_and_logos_resolved():
    """Test Atom feed icon and logo URLs are resolved"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/blog/">
        <title>Test</title>
        <icon>icon.png</icon>
        <logo>logo.png</logo>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.feed.icon == "http://example.org/blog/icon.png"
    assert result.feed.logo == "http://example.org/blog/logo.png"


def test_special_schemes_preserved():
    """Test special URL schemes like mailto: are preserved"""
    xml = b"""<?xml version="1.0"?>
    <feed xmlns="http://www.w3.org/2005/Atom" xml:base="http://example.org/">
        <entry>
            <link href="mailto:test@example.com" />
        </entry>
    </feed>
    """

    result = feedparser_rs.parse(xml)
    assert result.entries[0].link == "mailto:test@example.com"
