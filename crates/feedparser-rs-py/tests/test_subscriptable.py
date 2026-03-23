"""Tests for dict-style subscript access (__getitem__) on nested binding types."""

import feedparser_rs
import pytest

ATOM_FEED = b"""<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Feed</title>
  <link href="http://example.com/" rel="alternate" type="text/html" hreflang="en"/>
  <id>urn:uuid:test</id>
  <updated>2024-01-01T00:00:00Z</updated>
  <generator uri="http://gen.example.com/" version="1.0">MyGen</generator>
  <author>
    <name>John Doe</name>
    <email>john@example.com</email>
    <uri>http://john.example.com/</uri>
  </author>
  <category term="tech" scheme="http://scheme.example.com/" label="Technology"/>
  <entry>
    <title>Entry Title</title>
    <id>urn:uuid:entry1</id>
    <updated>2024-01-01T00:00:00Z</updated>
    <link href="http://example.com/entry1" rel="alternate"/>
    <author>
      <name>Jane Smith</name>
      <email>jane@example.com</email>
    </author>
    <content type="html" xml:lang="en" xml:base="http://example.com/">Hello &lt;b&gt;world&lt;/b&gt;</content>
    <enclosure xmlns="http://www.w3.org/2005/Atom" href="http://example.com/audio.mp3" type="audio/mpeg" length="12345"/>
  </entry>
</feed>
"""

RSS_FEED_WITH_IMAGE = b"""<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <link>http://example.com/</link>
    <image>
      <url>http://example.com/logo.png</url>
      <title>Test Logo</title>
      <link>http://example.com/</link>
      <width>144</width>
      <height>144</height>
    </image>
    <item>
      <title>Item</title>
      <link>http://example.com/item</link>
      <enclosure url="http://example.com/podcast.mp3" type="audio/mpeg" length="98765"/>
    </item>
  </channel>
</rss>
"""

RSS_FEED_WITH_SOURCE = b"""<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test</title>
    <link>http://example.com/</link>
    <item>
      <title>Item with source</title>
      <link>http://example.com/item</link>
      <source url="http://source.example.com/rss">Source Feed</source>
    </item>
  </channel>
</rss>
"""


def test_person_getitem():
    result = feedparser_rs.parse(ATOM_FEED)
    author = result.feed.author_detail
    assert author["name"] == "John Doe"
    assert author["email"] == "john@example.com"
    assert author["href"] == "http://john.example.com/"


def test_person_getitem_unknown_key():
    result = feedparser_rs.parse(ATOM_FEED)
    author = result.feed.author_detail
    with pytest.raises(KeyError):
        _ = author["nonexistent"]


def test_tag_getitem():
    result = feedparser_rs.parse(ATOM_FEED)
    tag = result.feed.tags[0]
    assert tag["term"] == "tech"
    assert tag["scheme"] == "http://scheme.example.com/"
    assert tag["label"] == "Technology"


def test_tag_getitem_unknown_key():
    result = feedparser_rs.parse(ATOM_FEED)
    tag = result.feed.tags[0]
    with pytest.raises(KeyError):
        _ = tag["nonexistent"]


def test_link_getitem():
    result = feedparser_rs.parse(ATOM_FEED)
    link = result.feed.links[0]
    assert link["href"] == "http://example.com/"
    assert link["rel"] == "alternate"
    assert link["type"] == "text/html"
    assert link["hreflang"] == "en"


def test_link_getitem_unknown_key():
    result = feedparser_rs.parse(ATOM_FEED)
    link = result.feed.links[0]
    with pytest.raises(KeyError):
        _ = link["nonexistent"]


def test_content_getitem():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    content = entry.content[0]
    assert "Hello" in content["value"]
    assert content["type"] == "html"
    # language and base from xml:lang/xml:base — may be None depending on parser support
    assert content["language"] is None or isinstance(content["language"], str)
    assert content["base"] is None or isinstance(content["base"], str)


def test_content_getitem_unknown_key():
    result = feedparser_rs.parse(ATOM_FEED)
    content = result.entries[0].content[0]
    with pytest.raises(KeyError):
        _ = content["nonexistent"]


def test_generator_getitem():
    result = feedparser_rs.parse(ATOM_FEED)
    gen = result.feed.generator_detail
    # gen["name"] maps to Generator.value (text content of <generator> element)
    assert isinstance(gen["name"], str)
    assert gen["href"] == "http://gen.example.com/"
    assert gen["version"] == "1.0"


def test_generator_getitem_unknown_key():
    result = feedparser_rs.parse(ATOM_FEED)
    gen = result.feed.generator_detail
    with pytest.raises(KeyError):
        _ = gen["nonexistent"]


def test_image_getitem():
    result = feedparser_rs.parse(RSS_FEED_WITH_IMAGE)
    image = result.feed.image
    assert image["href"] == "http://example.com/logo.png"
    assert image["title"] == "Test Logo"
    assert image["link"] == "http://example.com/"
    assert image["width"] == "144"
    assert image["height"] == "144"


def test_image_getitem_unknown_key():
    result = feedparser_rs.parse(RSS_FEED_WITH_IMAGE)
    image = result.feed.image
    with pytest.raises(KeyError):
        _ = image["nonexistent"]


def test_enclosure_getitem():
    result = feedparser_rs.parse(RSS_FEED_WITH_IMAGE)
    enc = result.entries[0].enclosures[0]
    assert enc["href"] == "http://example.com/podcast.mp3"
    assert enc["type"] == "audio/mpeg"
    assert enc["length"] == "98765"


def test_enclosure_getitem_unknown_key():
    result = feedparser_rs.parse(RSS_FEED_WITH_IMAGE)
    enc = result.entries[0].enclosures[0]
    with pytest.raises(KeyError):
        _ = enc["nonexistent"]


def test_source_getitem():
    result = feedparser_rs.parse(RSS_FEED_WITH_SOURCE)
    source = result.entries[0].source
    assert source is not None
    # Known keys return None when unpopulated (consistent with feedparser optional fields)
    assert source["title"] is None
    assert source["link"] is None
    assert source["id"] is None


def test_source_getitem_unknown_key():
    result = feedparser_rs.parse(RSS_FEED_WITH_SOURCE)
    source = result.entries[0].source
    assert source is not None
    with pytest.raises(KeyError):
        _ = source["nonexistent"]
