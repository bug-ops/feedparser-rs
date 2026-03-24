"""Tests for flat itunes_* key access via __getitem__ (Python feedparser compat, issue #232)."""

import feedparser_rs
import pytest

RSS_WITH_ITUNES = b"""<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"
  xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
  <channel>
    <title>My Podcast</title>
    <link>https://example.com/</link>
    <itunes:author>Show Author</itunes:author>
    <itunes:explicit>true</itunes:explicit>
    <itunes:image href="https://example.com/cover.jpg"/>
    <itunes:block>yes</itunes:block>
    <itunes:complete>yes</itunes:complete>
    <itunes:type>episodic</itunes:type>
    <itunes:new-feed-url>https://example.com/new.xml</itunes:new-feed-url>
    <item>
      <title>Episode 1</title>
      <link>https://example.com/ep1</link>
      <itunes:author>Jane Doe</itunes:author>
      <itunes:duration>1800</itunes:duration>
      <itunes:episode>1</itunes:episode>
      <itunes:season>2</itunes:season>
      <itunes:explicit>true</itunes:explicit>
      <itunes:episodeType>full</itunes:episodeType>
      <itunes:title>Episode Title</itunes:title>
    </item>
  </channel>
</rss>
"""


@pytest.fixture(scope="module")
def parsed():
    return feedparser_rs.parse(RSS_WITH_ITUNES)


def test_entry_itunes_duration_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_duration"] == "1800"


def test_entry_itunes_episode_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_episode"] == "1"


def test_entry_itunes_season_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_season"] == "2"


def test_entry_itunes_explicit_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_explicit"] is True


def test_entry_itunes_episodetype_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_episodetype"] == "full"


def test_entry_itunes_author_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_author"] == "Jane Doe"


def test_entry_itunes_title_flat(parsed):
    e = parsed.entries[0]
    assert e["itunes_title"] == "Episode Title"


def test_entry_nested_still_works(parsed):
    e = parsed.entries[0]
    assert e.itunes.duration == "1800"
    assert e.itunes.episode == "1"


def test_unknown_key_raises_keyerror(parsed):
    e = parsed.entries[0]
    with pytest.raises(KeyError):
        _ = e["itunes_nonexistent"]


def test_feed_itunes_author_flat(parsed):
    f = parsed.feed
    assert f["itunes_author"] == "Show Author"


def test_feed_itunes_explicit_flat(parsed):
    f = parsed.feed
    assert f["itunes_explicit"] is True


def test_feed_itunes_block_flat(parsed):
    f = parsed.feed
    assert f["itunes_block"] == 1


def test_feed_itunes_complete_flat(parsed):
    f = parsed.feed
    assert f["itunes_complete"] == "yes"


def test_feed_itunes_type_flat(parsed):
    f = parsed.feed
    assert f["itunes_type"] == "episodic"


def test_feed_itunes_new_feed_url_flat(parsed):
    f = parsed.feed
    assert f["itunes_new-feed-url"] == "https://example.com/new.xml"
