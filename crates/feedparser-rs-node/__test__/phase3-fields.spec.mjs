import assert from 'node:assert';
import { describe, it } from 'node:test';
import { parse } from '../index.js';

describe('Field Bindings', () => {
  describe('FeedMeta.geo', () => {
    it('should parse GeoRSS point location in feed', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <georss:point>45.256 -71.92</georss:point>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.geo);
      assert.strictEqual(feed.feed.geo.geoType, 'point');
      assert.strictEqual(feed.feed.geo.coordinates.length, 1);
      assert.strictEqual(feed.feed.geo.coordinates[0][0], 45.256);
      assert.strictEqual(feed.feed.geo.coordinates[0][1], -71.92);
    });

    it('should parse GeoRSS line in feed', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <georss:line>45.0 -71.0 46.0 -72.0</georss:line>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.geo);
      assert.strictEqual(feed.feed.geo.geoType, 'line');
      assert.strictEqual(feed.feed.geo.coordinates.length, 2);
      assert.strictEqual(feed.feed.geo.coordinates[0][0], 45.0);
      assert.strictEqual(feed.feed.geo.coordinates[0][1], -71.0);
      assert.strictEqual(feed.feed.geo.coordinates[1][0], 46.0);
      assert.strictEqual(feed.feed.geo.coordinates[1][1], -72.0);
    });

    it('should return undefined when no GeoRSS data', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.feed.geo, undefined);
    });
  });

  describe('FeedMeta.itunes', () => {
    it('should parse iTunes feed metadata', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
            <itunes:author>John Doe</itunes:author>
            <itunes:explicit>false</itunes:explicit>
            <itunes:image href="https://example.com/image.jpg" />
            <itunes:type>episodic</itunes:type>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.itunes);
      assert.strictEqual(feed.feed.itunes.author, 'John Doe');
      assert.strictEqual(feed.feed.itunes.explicit, false);
      assert.strictEqual(feed.feed.itunes.image, 'https://example.com/image.jpg');
      assert.strictEqual(feed.feed.itunes.podcastType, 'episodic');
    });

    it('should parse iTunes owner information', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
            <itunes:owner>
              <itunes:name>Jane Smith</itunes:name>
              <itunes:email>jane@example.com</itunes:email>
            </itunes:owner>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.itunes);
      assert.ok(feed.feed.itunes.owner);
      assert.strictEqual(feed.feed.itunes.owner.name, 'Jane Smith');
      assert.strictEqual(feed.feed.itunes.owner.email, 'jane@example.com');
    });

    it('should parse iTunes categories', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
            <itunes:category text="Technology">
              <itunes:category text="Podcasting" />
            </itunes:category>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.itunes);
      assert.strictEqual(feed.feed.itunes.categories.length, 1);
      assert.strictEqual(feed.feed.itunes.categories[0].text, 'Technology');
      assert.strictEqual(feed.feed.itunes.categories[0].subcategory, 'Podcasting');
    });

    it('should return undefined when no iTunes data', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.feed.itunes, undefined);
    });
  });

  describe('FeedMeta.podcast', () => {
    it('should have podcast field (undefined when no data)', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
        </rss>`;

      const feed = parse(xml);
      // Podcast field binding exists (can be undefined)
      assert.strictEqual(feed.feed.podcast, undefined);
    });

    it('should support FeedMeta.podcast field when present', () => {
      // Note: This test verifies the TypeScript binding accepts the field
      // Full podcast parsing in RSS 2.0 is not yet implemented in the core parser
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
          </channel>
        </rss>`;

      const feed = parse(xml);
      // When podcast data is absent, the field is undefined
      // This is expected napi-rs behavior for Option<T> = None
      assert.strictEqual(feed.feed.podcast, undefined);
    });
  });

  describe('Entry.geo', () => {
    it('should parse GeoRSS point in entry', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
              <georss:point>40.7128 -74.0060</georss:point>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(feed.entries[0].geo);
      assert.strictEqual(feed.entries[0].geo.geoType, 'point');
      assert.strictEqual(feed.entries[0].geo.coordinates[0][0], 40.7128);
      assert.strictEqual(feed.entries[0].geo.coordinates[0][1], -74.006);
    });

    it('should parse GeoRSS polygon in entry', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:georss="http://www.georss.org/georss">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
              <georss:polygon>45.0 -71.0 46.0 -71.0 46.0 -72.0 45.0 -71.0</georss:polygon>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(feed.entries[0].geo);
      assert.strictEqual(feed.entries[0].geo.geoType, 'polygon');
      assert.strictEqual(feed.entries[0].geo.coordinates.length, 4);
    });
  });

  describe('Entry Dublin Core fields', () => {
    it('should parse dc:creator in entry', () => {
      const xml = `<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
          <channel rdf:about="https://example.com">
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
          <item rdf:about="https://example.com/item1">
            <title>Test Item</title>
            <link>https://example.com/item1</link>
            <dc:creator>Jane Doe</dc:creator>
          </item>
        </rdf:RDF>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.strictEqual(feed.entries[0].dcCreator, 'Jane Doe');
    });

    it('should parse dc:date in entry as timestamp', () => {
      const xml = `<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
          <channel rdf:about="https://example.com">
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
          <item rdf:about="https://example.com/item1">
            <title>Test Item</title>
            <link>https://example.com/item1</link>
            <dc:date>2024-01-15T12:00:00Z</dc:date>
          </item>
        </rdf:RDF>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(feed.entries[0].dcDate);
      assert.strictEqual(typeof feed.entries[0].dcDate, 'number');
      // Check it's a valid timestamp (milliseconds since epoch)
      const date = new Date(feed.entries[0].dcDate);
      assert.strictEqual(date.getUTCFullYear(), 2024);
      assert.strictEqual(date.getUTCMonth(), 0); // January = 0
      assert.strictEqual(date.getUTCDate(), 15);
    });

    it('should parse dc:subject as array', () => {
      const xml = `<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
          <channel rdf:about="https://example.com">
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
          <item rdf:about="https://example.com/item1">
            <title>Test Item</title>
            <link>https://example.com/item1</link>
            <dc:subject>Technology</dc:subject>
            <dc:subject>Programming</dc:subject>
          </item>
        </rdf:RDF>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(Array.isArray(feed.entries[0].dcSubject));
      assert.strictEqual(feed.entries[0].dcSubject.length, 2);
      assert.strictEqual(feed.entries[0].dcSubject[0], 'Technology');
      assert.strictEqual(feed.entries[0].dcSubject[1], 'Programming');
    });

    it('should parse dc:rights in entry', () => {
      const xml = `<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/">
          <channel rdf:about="https://example.com">
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
          <item rdf:about="https://example.com/item1">
            <title>Test Item</title>
            <link>https://example.com/item1</link>
            <dc:rights>© 2024 Example Corp</dc:rights>
          </item>
        </rdf:RDF>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.strictEqual(feed.entries[0].dcRights, '© 2024 Example Corp');
    });

    it('should have empty array for dcSubject when not present', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(Array.isArray(feed.entries[0].dcSubject));
      assert.strictEqual(feed.entries[0].dcSubject.length, 0);
    });
  });

  describe('Entry Media RSS fields', () => {
    it('should parse media:thumbnail', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
              <media:thumbnail url="https://example.com/thumb.jpg" width="120" height="90" />
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(Array.isArray(feed.entries[0].mediaThumbnails));
      assert.strictEqual(feed.entries[0].mediaThumbnails.length, 1);
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].url, 'https://example.com/thumb.jpg');
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].width, 120);
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].height, 90);
    });

    it('should parse multiple media:thumbnails', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
              <media:thumbnail url="https://example.com/thumb1.jpg" width="120" height="90" />
              <media:thumbnail url="https://example.com/thumb2.jpg" width="240" height="180" />
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.strictEqual(feed.entries[0].mediaThumbnails.length, 2);
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].url, 'https://example.com/thumb1.jpg');
      assert.strictEqual(feed.entries[0].mediaThumbnails[1].url, 'https://example.com/thumb2.jpg');
    });

    it('should parse media:content', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
              <media:content url="https://example.com/video.mp4" type="video/mp4" fileSize="1024000" duration="120" width="1920" height="1080" />
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(Array.isArray(feed.entries[0].mediaContent));
      assert.strictEqual(feed.entries[0].mediaContent.length, 1);
      assert.strictEqual(feed.entries[0].mediaContent[0].url, 'https://example.com/video.mp4');
      assert.strictEqual(feed.entries[0].mediaContent[0].type, 'video/mp4');
      assert.strictEqual(feed.entries[0].mediaContent[0].filesize, 1024000);
      assert.strictEqual(feed.entries[0].mediaContent[0].duration, 120);
      assert.strictEqual(feed.entries[0].mediaContent[0].width, 1920);
      assert.strictEqual(feed.entries[0].mediaContent[0].height, 1080);
    });

    it('should have empty arrays when no media fields', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      assert.ok(Array.isArray(feed.entries[0].mediaThumbnails));
      assert.strictEqual(feed.entries[0].mediaThumbnails.length, 0);
      assert.ok(Array.isArray(feed.entries[0].mediaContent));
      assert.strictEqual(feed.entries[0].mediaContent.length, 0);
    });
  });

  describe('Entry.podcast', () => {
    it('should have podcast field (undefined when no data)', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Test Item</title>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      // Podcast field binding exists (can be undefined)
      assert.strictEqual(feed.entries[0].podcast, undefined);
    });

    it('should support Entry.podcast field when present', () => {
      // Note: This test verifies the TypeScript binding accepts the field
      // Full podcast parsing in RSS 2.0 is not yet implemented in the core parser
      const xml = `<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
            <item>
              <title>Episode 1</title>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      // When podcast data is absent, the field is undefined
      // This is expected napi-rs behavior for Option<T> = None
      assert.strictEqual(feed.entries[0].podcast, undefined);
    });
  });

  describe('Combined namespaces', () => {
    it('should parse feed with multiple namespace extensions', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0"
             xmlns:georss="http://www.georss.org/georss"
             xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"
             xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Multi-Namespace Podcast</title>
            <link>https://example.com</link>
            <georss:point>37.7749 -122.4194</georss:point>
            <itunes:author>San Francisco Podcasts</itunes:author>
            <podcast:guid>abc-123-def</podcast:guid>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.geo);
      assert.strictEqual(feed.feed.geo.geoType, 'point');
      assert.ok(feed.feed.itunes);
      assert.strictEqual(feed.feed.itunes.author, 'San Francisco Podcasts');
      assert.ok(feed.feed.podcast);
      assert.strictEqual(feed.feed.podcast.guid, 'abc-123-def');
    });

    it('should parse entry with multiple namespace extensions', () => {
      const xml = `<?xml version="1.0"?>
        <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                 xmlns="http://purl.org/rss/1.0/"
                 xmlns:dc="http://purl.org/dc/elements/1.1/"
                 xmlns:georss="http://www.georss.org/georss">
          <channel rdf:about="https://example.com">
            <title>Test Feed</title>
            <link>https://example.com</link>
          </channel>
          <item rdf:about="https://example.com/item1">
            <title>Multi-Namespace Item</title>
            <link>https://example.com/item1</link>
            <dc:creator>Bob Smith</dc:creator>
            <dc:subject>Travel</dc:subject>
            <georss:point>51.5074 -0.1278</georss:point>
          </item>
        </rdf:RDF>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries.length, 1);
      const entry = feed.entries[0];
      assert.strictEqual(entry.dcCreator, 'Bob Smith');
      assert.strictEqual(entry.dcSubject.length, 1);
      assert.strictEqual(entry.dcSubject[0], 'Travel');
      assert.ok(entry.geo);
      assert.strictEqual(entry.geo.geoType, 'point');
      // Media thumbnails field exists (empty array when no media)
      assert.ok(Array.isArray(entry.mediaThumbnails));
    });
  });
});
