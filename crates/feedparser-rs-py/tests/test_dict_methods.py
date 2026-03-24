"""Tests for dict-compatible methods: .get(), .keys(), .values(), .items() on FeedMeta and Entry."""

import feedparser_rs

ATOM_FEED = b"""<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Test Feed</title>
  <link href="http://example.com/" rel="alternate"/>
  <id>urn:uuid:test</id>
  <updated>2024-01-01T00:00:00Z</updated>
  <entry>
    <title>Entry One</title>
    <id>urn:uuid:entry1</id>
    <updated>2024-01-01T00:00:00Z</updated>
    <link href="http://example.com/entry1" rel="alternate"/>
    <summary>Entry summary text</summary>
  </entry>
</feed>
"""


# --- FeedMeta.get() ---


def test_feed_get_existing_key():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.feed.get("title") == "Test Feed"


def test_feed_get_missing_key_returns_none():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.feed.get("nonexistent_field") is None


def test_feed_get_missing_key_with_default():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.feed.get("nonexistent_field", "fallback") == "fallback"


def test_feed_get_absent_optional_field_returns_none():
    result = feedparser_rs.parse(ATOM_FEED)
    # subtitle is not set in this feed
    assert result.feed.get("subtitle") is None


def test_feed_get_absent_optional_field_with_default():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.feed.get("subtitle", "no subtitle") == "no subtitle"


# --- FeedMeta.keys() / values() / items() ---


def test_feed_keys_contains_populated_fields():
    result = feedparser_rs.parse(ATOM_FEED)
    keys = result.feed.keys()
    assert "title" in keys
    assert "link" in keys
    assert "id" in keys


def test_feed_keys_excludes_none_fields():
    result = feedparser_rs.parse(ATOM_FEED)
    keys = result.feed.keys()
    # subtitle is not set in this feed
    assert "subtitle" not in keys


def test_feed_values_returns_list():
    result = feedparser_rs.parse(ATOM_FEED)
    values = result.feed.values()
    assert isinstance(values, list)
    assert "Test Feed" in values


def test_feed_items_returns_key_value_pairs():
    result = feedparser_rs.parse(ATOM_FEED)
    items = result.feed.items()
    items_dict = dict(items)
    assert items_dict["title"] == "Test Feed"
    assert "subtitle" not in items_dict


def test_feed_keys_values_items_consistency():
    result = feedparser_rs.parse(ATOM_FEED)
    keys = result.feed.keys()
    values = result.feed.values()
    items = result.feed.items()
    assert len(keys) == len(values)
    assert len(keys) == len(items)
    for i, (k, v) in enumerate(items):
        assert k == keys[i]
        assert v == values[i]


# --- Entry.get() ---


def test_entry_get_existing_key():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    assert entry.get("title") == "Entry One"


def test_entry_get_missing_key_returns_none():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    assert entry.get("nonexistent_field") is None


def test_entry_get_missing_key_with_default():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    assert entry.get("nonexistent_field", "x") == "x"


def test_entry_get_absent_optional_field_returns_none():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    # author not set in this entry
    assert entry.get("author") is None


def test_entry_get_with_default_for_absent_field():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    assert entry.get("author", "anonymous") == "anonymous"


# --- Entry.keys() / values() / items() ---


def test_entry_keys_contains_populated_fields():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    keys = entry.keys()
    assert "title" in keys
    assert "id" in keys
    assert "summary" in keys


def test_entry_keys_excludes_none_fields():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    keys = entry.keys()
    assert "author" not in keys


def test_entry_values_returns_list():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    values = entry.values()
    assert isinstance(values, list)
    assert "Entry One" in values


def test_entry_items_returns_key_value_pairs():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    items_dict = dict(entry.items())
    assert items_dict["title"] == "Entry One"
    assert "author" not in items_dict


def test_entry_keys_values_items_consistency():
    result = feedparser_rs.parse(ATOM_FEED)
    entry = result.entries[0]
    keys = entry.keys()
    values = entry.values()
    items = entry.items()
    assert len(keys) == len(values)
    assert len(keys) == len(items)
    for i, (k, v) in enumerate(items):
        assert k == keys[i]
        assert v == values[i]


# --- FeedParserDict.get() ---


def test_parsed_feed_get_existing_key():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.get("version") is not None


def test_parsed_feed_get_missing_key_returns_none():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.get("nonexistent") is None


def test_parsed_feed_get_missing_key_with_default():
    result = feedparser_rs.parse(ATOM_FEED)
    assert result.get("nonexistent", "fallback") == "fallback"


def test_parsed_feed_keys():
    result = feedparser_rs.parse(ATOM_FEED)
    keys = result.keys()
    assert "feed" in keys
    assert "entries" in keys
    assert "version" in keys


def test_parsed_feed_items():
    result = feedparser_rs.parse(ATOM_FEED)
    items_dict = dict(result.items())
    assert "feed" in items_dict
    assert "entries" in items_dict
