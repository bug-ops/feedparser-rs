import assert from 'node:assert';
import { describe, it } from 'node:test';
import { parse } from '../index.js';

function atomWithDates(dcDate, thrUpdated) {
  return `<?xml version="1.0" encoding="utf-8"?>
    <feed xmlns="http://www.w3.org/2005/Atom"
          xmlns:dc="http://purl.org/dc/elements/1.1/"
          xmlns:thr="http://purl.org/syndication/thread/1.0">
      <title>Test Feed</title>
      <id>urn:uuid:feed</id>
      <updated>2025-01-15T12:30:00Z</updated>
      <entry>
        <title>Test Entry</title>
        <id>urn:uuid:entry</id>
        <updated>${dcDate}</updated>
        <dc:date>${dcDate}</dc:date>
        <link rel="replies" href="https://example.com/replies" thr:updated="${thrUpdated}"/>
      </entry>
    </feed>`;
}

describe('jiff AutoSi RFC 3339 formatting and date edge cases', () => {
  it('formats 0, 3, 6, and 9 fractional digits matching AutoSi with +00:00 suffix', () => {
    const cases = [
      // 0 fractional digits
      ['2025-01-15T12:30:00Z', '2025-01-15T12:30:00+00:00', 1736944200000],
      // 1-digit subsecond pads to 3 digits (ms-aligned)
      ['2025-01-15T12:30:00.5Z', '2025-01-15T12:30:00.500+00:00', 1736944200500],
      // 3-digit subsecond stays 3 digits
      ['2025-01-15T14:30:00.123+02:00', '2025-01-15T12:30:00.123+00:00', 1736944200123],
      // 4-digit subsecond pads to 6 digits (us-aligned)
      ['2025-01-15T12:30:00.1234Z', '2025-01-15T12:30:00.123400+00:00', 1736944200123],
      // 6-digit subsecond stays 6 digits
      ['2025-01-15T07:30:00.123456-05:00', '2025-01-15T12:30:00.123456+00:00', 1736944200123],
      // 7-digit subsecond pads to 9 digits (ns)
      ['2025-01-15T12:30:00.1234567Z', '2025-01-15T12:30:00.123456700+00:00', 1736944200123],
      // 9-digit subsecond stays 9 digits
      ['2025-01-15T12:30:00.123456789Z', '2025-01-15T12:30:00.123456789+00:00', 1736944200123],
    ];

    for (const [input, expectedRfc3339, expectedMillis] of cases) {
      const feed = parse(atomWithDates(input, input));
      const entry = feed.entries[0];
      const repliesLink = entry.links.find((l) => l.rel === 'replies');

      assert.strictEqual(entry.dcDate, expectedRfc3339, `dcDate mismatch for ${input}`);
      assert.strictEqual(entry.dcDateParsed, expectedMillis, `dcDateParsed mismatch for ${input}`);
      assert.strictEqual(
        repliesLink.thrUpdated,
        expectedRfc3339,
        `thrUpdated mismatch for ${input}`,
      );
      assert.strictEqual(
        repliesLink.thrUpdatedParsed,
        expectedMillis,
        `thrUpdatedParsed mismatch for ${input}`,
      );
    }
  });

  it('preserves DATE_FORMATS offset on space-separated W3C timestamps', () => {
    const feed = parse(atomWithDates('2025-01-15 12:30:00+02:00', '2025-01-15 12:30:00+02:00'));
    const entry = feed.entries[0];
    const repliesLink = entry.links.find((l) => l.rel === 'replies');

    assert.strictEqual(entry.dcDate, '2025-01-15T10:30:00+00:00');
    assert.strictEqual(entry.dcDateParsed, 1736937000000);
    assert.strictEqual(repliesLink.thrUpdated, '2025-01-15T10:30:00+00:00');
  });

  it('returns undefined for >9 fractional digits without failing feed parse', () => {
    const feed = parse(
      atomWithDates('2025-01-15T12:30:00.1234567890Z', '2025-01-15T12:30:00.1234567890Z'),
    );
    const entry = feed.entries[0];
    const repliesLink = entry.links.find((l) => l.rel === 'replies');

    assert.strictEqual(entry.dcDate, undefined);
    assert.strictEqual(entry.dcDateParsed, undefined);
    assert.strictEqual(repliesLink.thrUpdated, undefined);
    assert.strictEqual(repliesLink.thrUpdatedParsed, undefined);
  });
});
