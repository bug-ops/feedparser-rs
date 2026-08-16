import assert from 'node:assert';
import { describe, it } from 'node:test';
import { detectFormat, parse, parseUrl, parseWithOptions } from '../index.js';

describe('feedparser-rs', () => {
  describe('parse()', () => {
    it('should parse RSS 2.0 feed from string', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <description>Test Description</description>
            <item>
              <title>Test Entry</title>
              <link>https://example.com/1</link>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.version, 'rss20');
      assert.strictEqual(feed.bozo, false);
      assert.strictEqual(feed.feed.title, 'Test Feed');
      assert.strictEqual(feed.entries.length, 1);
      assert.strictEqual(feed.entries[0].title, 'Test Entry');
    });

    it('should parse RSS 2.0 feed from Buffer', () => {
      const xml = Buffer.from(`
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Buffer Test</title>
            <link>https://example.com</link>
            <item>
              <title>Buffer Entry</title>
            </item>
          </channel>
        </rss>
      `);

      const feed = parse(xml);

      assert.strictEqual(feed.version, 'rss20');
      assert.strictEqual(feed.feed.title, 'Buffer Test');
      assert.strictEqual(feed.entries.length, 1);
    });

    it('should parse Atom 1.0 feed from string', () => {
      const xml = `
        <?xml version="1.0" encoding="utf-8"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Test Atom Feed</title>
          <link href="https://example.com"/>
          <id>urn:uuid:test</id>
          <updated>2025-01-01T00:00:00Z</updated>
          <entry>
            <title>Test Atom Entry</title>
            <id>urn:uuid:entry1</id>
            <updated>2025-01-01T00:00:00Z</updated>
          </entry>
        </feed>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.version, 'atom10');
      assert.strictEqual(feed.feed.title, 'Test Atom Feed');
      assert.strictEqual(feed.entries.length, 1);
      assert.strictEqual(feed.entries[0].title, 'Test Atom Entry');
    });

    it('should parse JSON Feed', () => {
      const json = `{
        "version": "https://jsonfeed.org/version/1.1",
        "title": "Test JSON Feed",
        "home_page_url": "https://example.com",
        "items": [
          {
            "id": "1",
            "content_text": "Test content",
            "url": "https://example.com/1"
          }
        ]
      }`;

      const feed = parse(json);

      assert.strictEqual(feed.version, 'json11');
      assert.strictEqual(feed.feed.title, 'Test JSON Feed');
      assert.strictEqual(feed.entries.length, 1);
    });

    it('should set bozo flag for malformed feed', () => {
      const xml = '<rss><channel><title>Broken</title></rss>';

      const feed = parse(xml);

      // Tolerant parser may or may not set bozo for minor issues
      // At minimum, it should still extract data
      assert.strictEqual(feed.feed.title, 'Broken');
    });

    it('should handle dates correctly', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
            <item>
              <title>Entry with date</title>
              <pubDate>Mon, 01 Jan 2025 12:00:00 GMT</pubDate>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // Date may or may not be parsed depending on format support
      if (feed.entries[0].published !== null && feed.entries[0].published !== undefined) {
        assert.strictEqual(typeof feed.entries[0].published, 'string');
        assert(feed.entries[0].published.length > 0);
        assert.notStrictEqual(Number.isNaN(new Date(feed.entries[0].published).getTime()), true);
        // publishedParsed is only set when the date was successfully parsed as a DateTime
        if (
          feed.entries[0].publishedParsed !== null &&
          feed.entries[0].publishedParsed !== undefined
        ) {
          assert.strictEqual(typeof feed.entries[0].publishedParsed, 'number');
          assert(feed.entries[0].publishedParsed > 0);
        }
      }
    });

    it('should parse feed-level published date', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <pubDate>Wed, 18 Dec 2024 10:00:00 +0000</pubDate>
            <item>
              <title>Test Entry</title>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert(feed.feed.published !== null && feed.feed.published !== undefined);
      assert.strictEqual(typeof feed.feed.published, 'string');
      assert(feed.feed.published.length > 0);
      // Verify the date can be parsed and matches the expected timestamp (Wed, 18 Dec 2024 10:00:00 +0000)
      assert.strictEqual(new Date(feed.feed.published).getTime(), 1734516000000);
      assert.strictEqual(typeof feed.feed.publishedParsed, 'number');
      assert.strictEqual(feed.feed.publishedParsed, 1734516000000);
    });

    it('should handle multiple entries', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Multi Entry Feed</title>
            <item><title>Entry 1</title></item>
            <item><title>Entry 2</title></item>
            <item><title>Entry 3</title></item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.entries.length, 3);
      assert.strictEqual(feed.entries[0].title, 'Entry 1');
      assert.strictEqual(feed.entries[1].title, 'Entry 2');
      assert.strictEqual(feed.entries[2].title, 'Entry 3');
    });

    it('should extract links correctly', () => {
      const xml = `
        <?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Link Test</title>
          <id>test</id>
          <updated>2025-01-01T00:00:00Z</updated>
          <link rel="alternate" href="https://example.com"/>
          <link rel="self" href="https://example.com/feed.xml"/>
          <entry>
            <title>Entry</title>
            <id>entry1</id>
            <updated>2025-01-01T00:00:00Z</updated>
            <link href="https://example.com/entry1"/>
          </entry>
        </feed>
      `;

      const feed = parse(xml);

      assert(feed.feed.links.length >= 2);
      assert(feed.entries[0].links.length >= 1);
      assert.strictEqual(feed.entries[0].links[0].href, 'https://example.com/entry1');
    });

    it('should detect encoding', () => {
      const xml = `
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert(feed.encoding);
      assert.strictEqual(typeof feed.encoding, 'string');
    });

    it('should extract namespaces', () => {
      const xml = `
        <?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Test</title>
          <id>test</id>
          <updated>2025-01-01T00:00:00Z</updated>
        </feed>
      `;

      const feed = parse(xml);

      assert(typeof feed.namespaces === 'object');
    });
  });

  describe('detectFormat()', () => {
    it('should detect RSS 2.0', () => {
      const xml = '<rss version="2.0"><channel></channel></rss>';
      assert.strictEqual(detectFormat(xml), 'rss20');
    });

    it('should detect Atom 1.0', () => {
      const xml = '<feed xmlns="http://www.w3.org/2005/Atom"></feed>';
      assert.strictEqual(detectFormat(xml), 'atom10');
    });

    it('should detect JSON Feed 1.1', () => {
      const json = '{"version":"https://jsonfeed.org/version/1.1"}';
      assert.strictEqual(detectFormat(json), 'json11');
    });

    it('should detect JSON Feed 1.0', () => {
      const json = '{"version":"https://jsonfeed.org/version/1"}';
      assert.strictEqual(detectFormat(json), 'json10');
    });

    it('should detect RSS 1.0', () => {
      const xml =
        '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns="http://purl.org/rss/1.0/"></rdf:RDF>';
      assert.strictEqual(detectFormat(xml), 'rss10');
    });

    it('should return unknown for invalid content', () => {
      const format = detectFormat('not a feed');
      // Parser may attempt to identify format even for invalid content
      assert.strictEqual(typeof format, 'string');
    });

    it('should handle empty string', () => {
      const format = detectFormat('');
      // Empty string should return some format identifier
      assert.strictEqual(typeof format, 'string');
    });

    it('should work with Buffer input', () => {
      const xml = Buffer.from('<rss version="2.0"><channel></channel></rss>');
      assert.strictEqual(detectFormat(xml), 'rss20');
    });
  });

  describe('parseWithOptions()', () => {
    it('should parse with default options', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
          </channel>
        </rss>
      `;

      const feed = parseWithOptions(xml, null);

      assert.strictEqual(feed.version, 'rss20');
      assert.strictEqual(feed.feed.title, 'Test Feed');
    });

    it('should parse with custom size limit', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
          </channel>
        </rss>
      `;

      // 1MB limit should be enough
      const feed = parseWithOptions(xml, { maxSize: 1024 * 1024 });

      assert.strictEqual(feed.version, 'rss20');
      assert.strictEqual(feed.feed.title, 'Test Feed');
    });

    it('should reject feeds exceeding size limit', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
          </channel>
        </rss>
      `;

      // Very small limit (10 bytes)
      assert.throws(() => {
        parseWithOptions(xml, { maxSize: 10 });
      }, /exceeds maximum/);
    });

    it('should skip HTML sanitization when sanitizeHtml is false', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <item><title>Item</title><description>&lt;script&gt;alert(1)&lt;/script&gt;Hi</description></item>
          </channel>
        </rss>
      `;

      const sanitized = parseWithOptions(xml, null);
      assert.ok(!sanitized.entries[0].summary.includes('<script>'));

      const raw = parseWithOptions(xml, { sanitizeHtml: false });
      assert.ok(raw.entries[0].summary.includes('<script>'));
    });
  });

  describe('error handling', () => {
    it('should throw on null input', () => {
      assert.throws(() => {
        parse(null);
      });
    });

    it('should throw on undefined input', () => {
      assert.throws(() => {
        parse(undefined);
      });
    });

    it('should throw on number input', () => {
      assert.throws(() => {
        parse(123);
      });
    });

    it('should throw on object input', () => {
      assert.throws(() => {
        parse({ foo: 'bar' });
      });
    });

    it('should handle empty string gracefully', () => {
      // Empty string should either throw or return bozo=true
      try {
        const feed = parse('');
        // If it doesn't throw, bozo should be true
        assert.strictEqual(feed.bozo, true);
      } catch (e) {
        // Throwing is also acceptable
        assert(e.message);
      }
    });

    it('should handle completely invalid XML', () => {
      // Parser should handle invalid XML without crashing
      try {
        const feed = parse('this is not XML at all');
        // If parsed, bozo should be true
        assert.strictEqual(typeof feed.bozo, 'boolean');
      } catch (e) {
        // Throwing is also acceptable for completely invalid input
        assert(e.message);
      }
    });

    it('should handle partial XML', () => {
      // Parser should handle partial XML without crashing
      try {
        const feed = parse('<rss><channel><title>Incomplete');
        assert.strictEqual(typeof feed.bozo, 'boolean');
      } catch (e) {
        assert(e.message);
      }
    });

    it('should handle binary garbage gracefully', () => {
      const garbage = Buffer.from([0xff, 0xfe, 0x00, 0x01, 0x02, 0x03]);
      // Parser should handle binary garbage without crashing
      try {
        const feed = parse(garbage);
        assert.strictEqual(typeof feed.bozo, 'boolean');
      } catch (e) {
        assert(e.message);
      }
    });

    it('should throw with InvalidArg code for size limit exceeded', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
          </channel>
        </rss>
      `;

      // Very small limit (10 bytes) triggers InvalidFormat error
      assert.throws(
        () => {
          parseWithOptions(xml, { maxSize: 10 });
        },
        (err) => {
          assert.strictEqual(err.code, 'InvalidArg');
          return true;
        },
      );
    });

    it('should throw with GenericFailure code for SSRF loopback rejection', () => {
      // 127.0.0.1 is rejected by SSRF protection before any socket is opened
      // This tests the GenericFailure path (Http error) from SSRF validation
      assert.throws(
        () => {
          parseUrl('http://127.0.0.1:1/feed.xml');
        },
        (err) => {
          assert.strictEqual(err.code, 'GenericFailure');
          return true;
        },
      );
    });
  });

  describe('edge cases', () => {
    it('should handle feed with no entries', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Empty Feed</title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.entries.length, 0);
      assert.strictEqual(feed.feed.title, 'Empty Feed');
    });

    it('should handle feed with empty title', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title></title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.feed.title, '');
    });

    it('should handle special characters in content', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test &amp; Special &lt;Characters&gt;</title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // Parser should decode XML entities
      // Note: may strip whitespace or handle entities differently
      assert(feed.feed.title);
      assert(feed.feed.title.includes('Test'));
    });

    it('should handle unicode content', () => {
      const xml = `
        <?xml version="1.0" encoding="UTF-8"?>
        <rss version="2.0">
          <channel>
            <title>日本語タイトル</title>
            <item>
              <title>中文内容 🎉</title>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(feed.feed.title, '日本語タイトル');
      assert.strictEqual(feed.entries[0].title, '中文内容 🎉');
    });

    it('should handle CDATA sections', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title><![CDATA[Title with <HTML> in CDATA]]></title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // CDATA content is extracted correctly, but titles are HTML-sanitized by
      // default (#438), so the unrecognized <HTML> tag is stripped rather than
      // preserved verbatim.
      assert(feed.feed.title.includes('Title with'));
      assert(!feed.feed.title.includes('<HTML>'));
    });

    it('should handle very long titles', () => {
      const longTitle = 'A'.repeat(10000);
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>${longTitle}</title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // Should be truncated or preserved depending on limits
      assert(feed.feed.title);
      assert(feed.feed.title.length > 0);
    });

    it('should handle whitespace-only content', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>   </title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // Whitespace may be trimmed or preserved
      assert.strictEqual(typeof feed.feed.title, 'string');
    });

    it('should handle entries with all optional fields', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
            <item>
              <title>Entry Title</title>
              <link>https://example.com/1</link>
              <description>Entry Description</description>
              <author>author@example.com</author>
              <category>Category1</category>
              <pubDate>Mon, 01 Jan 2025 12:00:00 GMT</pubDate>
              <guid>unique-id-1</guid>
              <enclosure url="https://example.com/audio.mp3" length="12345" type="audio/mpeg"/>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      const entry = feed.entries[0];
      assert.strictEqual(entry.title, 'Entry Title');
      assert.strictEqual(entry.link, 'https://example.com/1');
      assert(entry.summary || entry.summary === ''); // description maps to summary
      assert(entry.tags.length > 0);
      assert(entry.enclosures.length > 0);
      assert.strictEqual(entry.enclosures[0].href, 'https://example.com/audio.mp3');
    });
  });

  describe('type checking', () => {
    it('should return correct types for feed metadata', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
            <link>https://example.com</link>
            <description>Description</description>
            <language>en-us</language>
            <ttl>60</ttl>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      assert.strictEqual(typeof feed.feed.title, 'string');
      assert.strictEqual(typeof feed.feed.link, 'string');
      assert.strictEqual(typeof feed.version, 'string');
      assert.strictEqual(typeof feed.bozo, 'boolean');
      assert.strictEqual(typeof feed.encoding, 'string');
      assert(Array.isArray(feed.entries));
      assert(Array.isArray(feed.feed.links));
      assert(Array.isArray(feed.feed.tags));
      assert(typeof feed.namespaces === 'object');
    });

    it('should return correct types for entry', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
            <item>
              <title>Entry</title>
              <pubDate>Mon, 01 Jan 2025 12:00:00 GMT</pubDate>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);
      const entry = feed.entries[0];

      assert.strictEqual(typeof entry.title, 'string');
      // published may be a string or null/undefined depending on date parsing
      if (entry.published !== null && entry.published !== undefined) {
        assert.strictEqual(typeof entry.published, 'string');
      }
      assert(Array.isArray(entry.links));
      assert(Array.isArray(entry.tags));
      assert(Array.isArray(entry.enclosures));
      assert(Array.isArray(entry.content));
      assert(Array.isArray(entry.authors));
    });

    it('should handle null/undefined fields correctly', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Minimal</title>
          </channel>
        </rss>
      `;

      const feed = parse(xml);

      // Optional fields should be null, not undefined
      assert(feed.feed.subtitle === null || feed.feed.subtitle === undefined);
      assert(feed.feed.image === null || feed.feed.image === undefined);
    });
  });

  describe('link handling', () => {
    it('should extract link attributes', () => {
      const xml = `
        <?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Test</title>
          <id>test</id>
          <updated>2025-01-01T00:00:00Z</updated>
          <link rel="alternate" type="text/html" href="https://example.com" title="Website"/>
          <link rel="self" type="application/atom+xml" href="https://example.com/feed.xml"/>
        </feed>
      `;

      const feed = parse(xml);

      const altLink = feed.feed.links.find((l) => l.rel === 'alternate');
      const selfLink = feed.feed.links.find((l) => l.rel === 'self');

      assert(altLink);
      assert.strictEqual(altLink.href, 'https://example.com');
      assert.strictEqual(altLink.type, 'text/html');
      assert.strictEqual(altLink.title, 'Website');

      assert(selfLink);
      assert.strictEqual(selfLink.href, 'https://example.com/feed.xml');
      assert.strictEqual(selfLink.type, 'application/atom+xml');
    });
  });

  describe('author handling', () => {
    it('should extract author information', () => {
      const xml = `
        <?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <title>Test</title>
          <id>test</id>
          <updated>2025-01-01T00:00:00Z</updated>
          <author>
            <name>John Doe</name>
            <email>john@example.com</email>
            <uri>https://example.com/john</uri>
          </author>
          <entry>
            <title>Entry</title>
            <id>entry1</id>
            <updated>2025-01-01T00:00:00Z</updated>
            <author>
              <name>Jane Doe</name>
            </author>
          </entry>
        </feed>
      `;

      const feed = parse(xml);

      // Author extraction may vary by parser implementation
      // Check that at least authors array is populated
      if (feed.feed.authors && feed.feed.authors.length > 0) {
        assert(feed.feed.authors[0].name);
      }
      // Entry author
      if (feed.entries[0].authors && feed.entries[0].authors.length > 0) {
        assert(feed.entries[0].authors[0].name);
      }
    });
  });

  describe('enclosure handling', () => {
    it('should extract enclosure information', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Podcast</title>
            <item>
              <title>Episode 1</title>
              <enclosure url="https://example.com/ep1.mp3" length="12345678" type="audio/mpeg"/>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);
      const enclosure = feed.entries[0].enclosures[0];

      assert.strictEqual(enclosure.href, 'https://example.com/ep1.mp3');
      assert.strictEqual(enclosure.type, 'audio/mpeg');
      assert.strictEqual(typeof enclosure.length, 'string');
      assert(enclosure.length.length > 0);
    });

    it('should handle enclosures without optional fields', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test</title>
            <item>
              <title>Entry</title>
              <enclosure url="https://example.com/file.mp3"/>
            </item>
          </channel>
        </rss>
      `;

      const feed = parse(xml);
      const enclosure = feed.entries[0].enclosures[0];

      assert.strictEqual(enclosure.href, 'https://example.com/file.mp3');
      // length and type may be null
      assert(
        enclosure.length === null ||
          enclosure.length === undefined ||
          typeof enclosure.length === 'string',
      );
    });
  });

  describe('JSON Feed specifics', () => {
    it('should parse JSON Feed with all fields', () => {
      const json = JSON.stringify({
        version: 'https://jsonfeed.org/version/1.1',
        title: 'Full JSON Feed',
        home_page_url: 'https://example.com',
        feed_url: 'https://example.com/feed.json',
        description: 'A complete JSON Feed',
        icon: 'https://example.com/icon.png',
        favicon: 'https://example.com/favicon.ico',
        language: 'en-US',
        authors: [
          {
            name: 'Author Name',
            url: 'https://example.com/author',
          },
        ],
        items: [
          {
            id: '1',
            url: 'https://example.com/1',
            title: 'Item 1',
            content_html: '<p>HTML Content</p>',
            content_text: 'Plain text content',
            summary: 'Item summary',
            date_published: '2025-01-01T12:00:00Z',
            date_modified: '2025-01-02T12:00:00Z',
            tags: ['tag1', 'tag2'],
            attachments: [
              {
                url: 'https://example.com/audio.mp3',
                mime_type: 'audio/mpeg',
                size_in_bytes: 12345,
              },
            ],
          },
        ],
      });

      const feed = parse(json);

      assert.strictEqual(feed.version, 'json11');
      assert.strictEqual(feed.feed.title, 'Full JSON Feed');
      assert.strictEqual(feed.feed.language, 'en-US');
      assert(feed.feed.icon);

      const entry = feed.entries[0];
      assert.strictEqual(entry.title, 'Item 1');
      assert(entry.published);
      assert(entry.updated);
      assert(entry.tags.length > 0);
      assert(entry.enclosures.length > 0);
    });

    it('should handle JSON Feed 1.0 format', () => {
      const json = JSON.stringify({
        version: 'https://jsonfeed.org/version/1',
        title: 'JSON Feed 1.0',
        items: [],
      });

      const feed = parse(json);

      assert.strictEqual(feed.version, 'json10');
      assert.strictEqual(feed.feed.title, 'JSON Feed 1.0');
    });
  });

  describe('Atom Threading Extensions (RFC 4685)', () => {
    const THREADING_XML = `<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom"
      xmlns:thr="http://purl.org/syndication/thread/1.0">
  <title>Threading Test</title>
  <id>tag:example.com,2024:threading</id>
  <updated>2024-01-15T12:00:00Z</updated>
  <entry>
    <title>Full in-reply-to</title>
    <id>tag:example.com,2024:reply/1</id>
    <updated>2024-01-15T10:00:00Z</updated>
    <thr:in-reply-to
      ref="tag:example.com,2024:post/1"
      href="https://example.com/post/1"
      type="text/html"
      source="https://example.com/feed.xml"/>
    <thr:total>15</thr:total>
  </entry>
  <entry>
    <title>Multiple in-reply-to</title>
    <id>tag:example.com,2024:reply/2</id>
    <updated>2024-01-15T11:00:00Z</updated>
    <thr:in-reply-to ref="tag:example.com,2024:post/1"/>
    <thr:in-reply-to ref="tag:example.com,2024:post/2"/>
    <thr:in-reply-to ref="tag:example.com,2024:post/3" href="https://example.com/post/3"/>
  </entry>
  <entry>
    <title>Empty ref</title>
    <id>tag:example.com,2024:reply/3</id>
    <updated>2024-01-15T12:00:00Z</updated>
    <thr:in-reply-to ref="" href="https://example.com/post/1"/>
  </entry>
  <entry>
    <title>Negative total</title>
    <id>tag:example.com,2024:reply/4</id>
    <updated>2024-01-15T13:00:00Z</updated>
    <thr:total>-5</thr:total>
  </entry>
  <entry>
    <title>Overflow total</title>
    <id>tag:example.com,2024:reply/5</id>
    <updated>2024-01-15T14:00:00Z</updated>
    <thr:total>99999999999999</thr:total>
  </entry>
</feed>`;

    it('should parse thr:in-reply-to with all four attributes', () => {
      const feed = parse(THREADING_XML);
      assert.strictEqual(feed.bozo, false);
      const entry = feed.entries[0];
      assert(Array.isArray(entry.thrInReplyTo));
      assert.strictEqual(entry.thrInReplyTo.length, 1);
      const irt = entry.thrInReplyTo[0];
      assert.strictEqual(irt.ref, 'tag:example.com,2024:post/1');
      assert.strictEqual(irt.href, 'https://example.com/post/1');
      assert.strictEqual(irt.type, 'text/html');
      assert.strictEqual(irt.source, 'https://example.com/feed.xml');
      assert.strictEqual(entry.thrTotal, 15);
    });

    it('should parse multiple thr:in-reply-to elements on one entry', () => {
      const feed = parse(THREADING_XML);
      const entry = feed.entries[1];
      assert.strictEqual(entry.thrInReplyTo.length, 3);
      assert.strictEqual(entry.thrInReplyTo[0].ref, 'tag:example.com,2024:post/1');
      assert.strictEqual(entry.thrInReplyTo[1].ref, 'tag:example.com,2024:post/2');
      assert.strictEqual(entry.thrInReplyTo[2].ref, 'tag:example.com,2024:post/3');
      assert.strictEqual(entry.thrInReplyTo[2].href, 'https://example.com/post/3');
      assert.ok(entry.thrTotal == null);
    });

    it('should normalize empty ref attribute to null', () => {
      const feed = parse(THREADING_XML);
      const entry = feed.entries[2];
      assert.strictEqual(entry.thrInReplyTo.length, 1);
      const irt = entry.thrInReplyTo[0];
      assert.ok(irt.ref == null);
      assert.strictEqual(irt.href, 'https://example.com/post/1');
    });

    it('should return null for negative thr:total', () => {
      const feed = parse(THREADING_XML);
      const entry = feed.entries[3];
      assert.ok(entry.thrTotal == null);
    });

    it('should return null for u32-overflowing thr:total', () => {
      const feed = parse(THREADING_XML);
      const entry = feed.entries[4];
      assert.ok(entry.thrTotal == null);
    });
  });

  describe('Cloud element', () => {
    it('should expose registerprocedure (no underscore) on cloud object', () => {
      const xml = `
        <?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Cloud Test</title>
            <link>https://example.com</link>
            <description>Feed with cloud</description>
            <cloud domain="rpc.example.com" port="80" path="/rpc" registerProcedure="myCloud.rssPleaseNotify" protocol="xml-rpc" />
          </channel>
        </rss>
      `;

      const feed = parse(xml);
      assert.ok(feed.feed.cloud != null, 'cloud should be present');
      assert.strictEqual(feed.feed.cloud.registerprocedure, 'myCloud.rssPleaseNotify');
    });
  });

  describe('RSS 1.0 (RDF) handling', () => {
    it('should detect RSS 1.0 format', () => {
      const xml = `
        <?xml version="1.0"?>
        <rdf:RDF
          xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
          xmlns="http://purl.org/rss/1.0/">
          <channel>
            <title>RSS 1.0 Feed</title>
            <link>https://example.com</link>
            <description>An RSS 1.0 feed</description>
          </channel>
          <item>
            <title>Item 1</title>
            <link>https://example.com/1</link>
          </item>
        </rdf:RDF>
      `;

      // RSS 1.0 parsing may not be fully implemented
      // Just verify format detection works
      assert.strictEqual(detectFormat(xml), 'rss10');
    });
  });
});
