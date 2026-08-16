"""Tests for JSON Feed extension objects (underscore-prefixed custom fields)"""

import feedparser_rs


def test_feed_level_extension_captured():
    json = b"""{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "Podcast Feed",
        "_cast": {"subcategory": "Tech News"},
        "items": []
    }"""

    result = feedparser_rs.parse(json)

    assert result.feed.json_extensions["_cast"]["subcategory"] == "Tech News"


def test_item_level_extension_captured():
    json = b"""{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "Podcast Feed",
        "items": [{"id": "1", "_explicit": true}]
    }"""

    result = feedparser_rs.parse(json)

    assert result.entries[0].json_extensions["_explicit"] is True


def test_extensions_absent_are_empty_dict():
    json = b"""{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "Plain Feed",
        "items": [{"id": "1"}]
    }"""

    result = feedparser_rs.parse(json)

    assert result.feed.json_extensions == {}
    assert result.entries[0].json_extensions == {}


def test_json_extensions_in_keys():
    json = b"""{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "Podcast Feed",
        "_cast": {"subcategory": "Tech News"},
        "items": []
    }"""

    result = feedparser_rs.parse(json)

    feed_keys = result.feed.keys()

    assert "json_extensions" in feed_keys
