"""Acceptance tests for jiff AutoSi RFC 3339 formatting and Python datetime.fromisoformat()."""

from datetime import datetime, timezone

import feedparser_rs
import pytest


def _atom_with_dates(dc_date: str, thr_updated: str) -> bytes:
    return f"""<?xml version="1.0" encoding="utf-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:dc="http://purl.org/dc/elements/1.1/"
          xmlns:thr="http://purl.org/syndication/thread/1.0">
      <title>Test Feed</title>
      <id>urn:uuid:feed</id>
      <updated>2025-01-15T12:30:00Z</updated>
      <entry>
        <title>Test Entry</title>
        <id>urn:uuid:entry</id>
        <updated>{dc_date}</updated>
        <dc:date>{dc_date}</dc:date>
        <link rel="replies" href="https://example.com/replies" thr:updated="{thr_updated}"/>
      </entry>
    </feed>""".encode()


@pytest.mark.parametrize(
    ("raw_date", "expected_rfc3339"),
    [
        # 0 fractional digits
        ("2025-01-15T12:30:00Z", "2025-01-15T12:30:00+00:00"),
        # 1-digit fractional second pads to 3 digits (ms-aligned)
        ("2025-01-15T12:30:00.5Z", "2025-01-15T12:30:00.500+00:00"),
        # 3-digit fractional second stays 3 digits
        ("2025-01-15T14:30:00.123+02:00", "2025-01-15T12:30:00.123+00:00"),
        # 4-digit fractional second pads to 6 digits (us-aligned)
        ("2025-01-15T12:30:00.1234Z", "2025-01-15T12:30:00.123400+00:00"),
        # 6-digit fractional second stays 6 digits
        ("2025-01-15T07:30:00.123456-05:00", "2025-01-15T12:30:00.123456+00:00"),
        # 7-digit fractional second pads to 9 digits (ns)
        ("2025-01-15T12:30:00.1234567Z", "2025-01-15T12:30:00.123456700+00:00"),
        # 9-digit fractional second stays 9 digits
        ("2025-01-15T12:30:00.123456789Z", "2025-01-15T12:30:00.123456789+00:00"),
    ],
)
def test_autosi_rfc3339_precision_on_dc_date_and_thr_updated(
    raw_date: str, expected_rfc3339: str
) -> None:
    result = feedparser_rs.parse(_atom_with_dates(raw_date, raw_date))
    entry = result.entries[0]
    replies_link = next(link for link in entry.links if link.rel == "replies")

    assert entry.dc_date == expected_rfc3339
    assert entry["dc_date"] == expected_rfc3339
    assert replies_link.thr_updated == expected_rfc3339


@pytest.mark.parametrize(
    ("raw_date", "expected_dt"),
    [
        # 0, 3, and 6 fractional digits with +00:00 are accepted by Python 3.10+ fromisoformat()
        (
            "2025-01-15T12:30:00Z",
            datetime(2025, 1, 15, 12, 30, 0, 0, tzinfo=timezone.utc),
        ),
        (
            "2025-01-15T14:30:00.5+02:00",
            datetime(2025, 1, 15, 12, 30, 0, 500_000, tzinfo=timezone.utc),
        ),
        (
            "2025-01-15T12:30:00.123Z",
            datetime(2025, 1, 15, 12, 30, 0, 123_000, tzinfo=timezone.utc),
        ),
        (
            "2025-01-15T12:30:00.1234Z",
            datetime(2025, 1, 15, 12, 30, 0, 123_400, tzinfo=timezone.utc),
        ),
        (
            "2025-01-15T07:30:00.123456-05:00",
            datetime(2025, 1, 15, 12, 30, 0, 123_456, tzinfo=timezone.utc),
        ),
    ],
)
def test_autosi_0_3_6_digit_timestamps_compatible_with_fromisoformat(
    raw_date: str, expected_dt: datetime
) -> None:
    result = feedparser_rs.parse(_atom_with_dates(raw_date, raw_date))
    entry = result.entries[0]
    replies_link = next(link for link in entry.links if link.rel == "replies")

    assert entry.dc_date is not None
    assert replies_link.thr_updated is not None
    assert datetime.fromisoformat(entry.dc_date) == expected_dt
    assert datetime.fromisoformat(replies_link.thr_updated) == expected_dt


def test_atom03_created_dict_access_uses_autosi() -> None:
    atom03 = b"""<?xml version="1.0" encoding="utf-8"?>
    <feed version="0.3" xmlns="http://purl.org/atom/ns#">
      <title>Atom 0.3</title>
      <entry>
        <title>Entry 1</title>
        <id>tag:example.com,2025:1</id>
        <created>2025-01-15T14:30:00.123+02:00</created>
      </entry>
    </feed>"""

    result = feedparser_rs.parse(atom03)
    entry = result.entries[0]

    assert entry.created == "2025-01-15T14:30:00.123+02:00"
    assert entry["created"] == "2025-01-15T12:30:00.123+00:00"
    assert datetime.fromisoformat(entry["created"]) == datetime(
        2025, 1, 15, 12, 30, 0, 123_000, tzinfo=timezone.utc
    )


def test_date_formats_offset_preservation_and_invalid_edge_cases() -> None:
    # DATE_FORMATS %Y-%m-%d %H:%M:%S%:z must preserve +02:00 -> 10:30:00+00:00
    result_offset = feedparser_rs.parse(
        _atom_with_dates("2025-01-15 12:30:00+02:00", "2025-01-15 12:30:00+02:00")
    )
    entry_offset = result_offset.entries[0]
    assert entry_offset.dc_date == "2025-01-15T10:30:00+00:00"
    assert entry_offset.updated_parsed is not None
    assert entry_offset.updated_parsed.tm_hour == 10
    assert entry_offset.updated_parsed.tm_wday == 2  # Wednesday (Monday=0)
    assert entry_offset.updated_parsed.tm_yday == 15
    assert entry_offset.updated_parsed.tm_isdst == 0

    # >9 fractional digits and non-ASCII 7-byte "abcä-1" must return None without panicking
    result_over9 = feedparser_rs.parse(
        _atom_with_dates("2025-01-15T12:30:00.1234567890Z", "2025-01-15T12:30:00.1234567890Z")
    )
    assert result_over9.entries[0].dc_date is None

    result_non_ascii = feedparser_rs.parse(_atom_with_dates("abcä-1", "abc"))
    assert result_non_ascii.entries[0].dc_date is None
    replies_link = next(link for link in result_non_ascii.entries[0].links if link.rel == "replies")
    assert replies_link.thr_updated is None
