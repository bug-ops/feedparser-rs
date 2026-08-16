import assert from 'node:assert';
import { describe, it } from 'node:test';
import { parse } from '../index.js';

describe('Field Bindings', () => {
  describe('FeedMeta.where', () => {
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
      assert.ok(feed.feed.where);
      assert.strictEqual(feed.feed.where.geoType, 'point');
      assert.strictEqual(feed.feed.where.coordinates.length, 1);
      assert.strictEqual(feed.feed.where.coordinates[0][0], 45.256);
      assert.strictEqual(feed.feed.where.coordinates[0][1], -71.92);
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
      assert.ok(feed.feed.where);
      assert.strictEqual(feed.feed.where.geoType, 'line');
      assert.strictEqual(feed.feed.where.coordinates.length, 2);
      assert.strictEqual(feed.feed.where.coordinates[0][0], 45.0);
      assert.strictEqual(feed.feed.where.coordinates[0][1], -71.0);
      assert.strictEqual(feed.feed.where.coordinates[1][0], 46.0);
      assert.strictEqual(feed.feed.where.coordinates[1][1], -72.0);
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
      assert.strictEqual(feed.feed.where, undefined);
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
      // "false" is not a truthy itunes:explicit value (only "yes"/"true"/"explicit" map to Some(true))
      assert.strictEqual(feed.feed.itunes.explicit, undefined);
      assert.strictEqual(feed.feed.itunes.image, 'https://example.com/image.jpg');
      assert.strictEqual(feed.feed.itunes.podcastType, 'episodic');
    });

    it('should parse itunes:explicit truthy value as true', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd">
          <channel>
            <title>Test Podcast</title>
            <link>https://example.com</link>
            <itunes:explicit>yes</itunes:explicit>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.ok(feed.feed.itunes);
      assert.strictEqual(feed.feed.itunes.explicit, true);
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

  describe('Entry.where', () => {
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
      assert.ok(feed.entries[0].where);
      assert.strictEqual(feed.entries[0].where.geoType, 'point');
      assert.strictEqual(feed.entries[0].where.coordinates[0][0], 40.7128);
      assert.strictEqual(feed.entries[0].where.coordinates[0][1], -74.006);
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
      assert.ok(feed.entries[0].where);
      assert.strictEqual(feed.entries[0].where.geoType, 'polygon');
      assert.strictEqual(feed.entries[0].where.coordinates.length, 4);
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
      assert.strictEqual(typeof feed.entries[0].dcDate, 'string');
      assert.ok(feed.entries[0].dcDateParsed);
      assert.strictEqual(typeof feed.entries[0].dcDateParsed, 'number');
      // Check it's a valid timestamp (milliseconds since epoch)
      const date = new Date(feed.entries[0].dcDateParsed);
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
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].width, '120');
      assert.strictEqual(feed.entries[0].mediaThumbnails[0].height, '90');
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
      assert.strictEqual(feed.entries[0].mediaContent[0].filesize, '1024000');
      assert.strictEqual(feed.entries[0].mediaContent[0].duration, '120');
      assert.strictEqual(feed.entries[0].mediaContent[0].width, '1920');
      assert.strictEqual(feed.entries[0].mediaContent[0].height, '1080');
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

  describe('Podcast chat/podping/valueTimeSplit', () => {
    it('should parse podcast:chat at feed level', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:chat server="matrix.example.com" protocol="matrix" accountId="@podcast:example.com" space="!room:example.com"/>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.feed.podcast.chat.length, 1);
      assert.strictEqual(feed.feed.podcast.chat[0].server, 'matrix.example.com');
      assert.strictEqual(feed.feed.podcast.chat[0].protocol, 'matrix');
      assert.strictEqual(feed.feed.podcast.chat[0].accountId, '@podcast:example.com');
      assert.strictEqual(feed.feed.podcast.chat[0].space, '!room:example.com');
    });

    it('should parse podcast:chat at entry level', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <item>
              <title>Episode</title>
              <podcast:chat server="xmpp.example.com" protocol="xmpp"/>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.entries[0].podcast.chat.length, 1);
      assert.strictEqual(feed.entries[0].podcast.chat[0].server, 'xmpp.example.com');
      assert.strictEqual(feed.entries[0].podcast.chat[0].protocol, 'xmpp');
    });

    it('should parse podcast:podping usesPodping attribute', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:podping usesPodping="true"/>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.feed.podcast.podpingUsesPodping, true);
    });

    it('should parse podcast:valueTimeSplit with recipients', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:value type="lightning" method="keysend">
              <podcast:valueTimeSplit startTime="60" duration="30">
                <podcast:valueRecipient type="node" address="addr1" split="100"/>
              </podcast:valueTimeSplit>
            </podcast:value>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const [split] = feed.feed.podcast.value.timeSplits;
      assert.strictEqual(feed.feed.podcast.value.timeSplits.length, 1);
      assert.strictEqual(split.startTime, 60);
      assert.strictEqual(split.duration, 30);
      assert.strictEqual(split.remotePercentage, 100);
      assert.strictEqual(split.recipients.length, 1);
      assert.strictEqual(split.recipients[0].address, 'addr1');
      assert.strictEqual(split.remoteItem, undefined);
    });

    it('should parse podcast:valueTimeSplit with a remote item', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:value type="lightning" method="keysend">
              <podcast:valueTimeSplit startTime="10" duration="20" remoteStartTime="5" remotePercentage="50">
                <podcast:remoteItem feedGuid="feed-guid-1" feedUrl="https://example.com/feed.xml" itemGuid="abc123" medium="podcast" title="Remote Episode"/>
              </podcast:valueTimeSplit>
            </podcast:value>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const [split] = feed.feed.podcast.value.timeSplits;
      assert.strictEqual(split.remoteStartTime, 5);
      assert.strictEqual(split.remotePercentage, 50);
      assert.strictEqual(split.remoteItem.feedGuid, 'feed-guid-1');
      assert.strictEqual(split.remoteItem.feedUrl, 'https://example.com/feed.xml');
      assert.strictEqual(split.remoteItem.itemGuid, 'abc123');
      assert.strictEqual(split.remoteItem.medium, 'podcast');
      assert.strictEqual(split.remoteItem.title, 'Remote Episode');
    });

    it('should default chat/podpingUsesPodping/timeSplits when absent', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:guid>abc-123-def</podcast:guid>
            <podcast:value type="lightning" method="keysend">
              <podcast:valueRecipient type="node" address="addr1" split="100"/>
            </podcast:value>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.feed.podcast.chat.length, 0);
      assert.strictEqual(feed.feed.podcast.podpingUsesPodping, undefined);
      assert.strictEqual(feed.feed.podcast.value.timeSplits.length, 0);
    });

    it('should silently drop a self-closing podcast:valueTimeSplit without swallowing the feed', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:value type="lightning" method="keysend">
              <podcast:valueTimeSplit startTime="1" duration="2"/>
              <podcast:valueRecipient type="node" address="addr1" split="100"/>
            </podcast:value>
          </channel>
        </rss>`;

      const feed = parse(xml);
      assert.strictEqual(feed.bozo, false);
      assert.strictEqual(feed.feed.podcast.value.timeSplits.length, 0);
      assert.strictEqual(feed.feed.podcast.value.recipients.length, 1);
    });
  });

  describe('Podcast podroll/location/txt/updateFrequency/follow/alternateEnclosure/socialInteract', () => {
    it('should parse podcast:podroll/location/txt/updateFrequency/follow at feed level', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:location geo="geo:37.786971,-122.399677" osm="R113314">San Francisco</podcast:location>
            <podcast:podroll>
              <podcast:remoteItem feedGuid="abc123" feedUrl="https://example.com/feed.xml" title="Example Podcast"/>
              <podcast:remoteItem feedGuid="def456" medium="podcast"/>
            </podcast:podroll>
            <podcast:txt purpose="verify">abc123verify</podcast:txt>
            <podcast:txt>plain text record</podcast:txt>
            <podcast:updateFrequency rrule="FREQ=WEEKLY" dtstart="2023-01-01T00:00:00Z" complete="false">weekly</podcast:updateFrequency>
            <podcast:follow url="https://mastodon.social/@podcast" platform="activitypub"/>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const podcast = feed.feed.podcast;

      assert.strictEqual(podcast.location.name, 'San Francisco');
      assert.strictEqual(podcast.location.geo, 'geo:37.786971,-122.399677');
      assert.strictEqual(podcast.location.osm, 'R113314');

      assert.strictEqual(podcast.podroll.length, 2);
      assert.strictEqual(podcast.podroll[0].feedGuid, 'abc123');
      assert.strictEqual(podcast.podroll[0].feedUrl, 'https://example.com/feed.xml');
      assert.strictEqual(podcast.podroll[0].title, 'Example Podcast');
      assert.strictEqual(podcast.podroll[1].feedGuid, 'def456');
      assert.strictEqual(podcast.podroll[1].medium, 'podcast');

      assert.strictEqual(podcast.txt.length, 2);
      assert.strictEqual(podcast.txt[0].purpose, 'verify');
      assert.strictEqual(podcast.txt[0].value, 'abc123verify');
      assert.strictEqual(podcast.txt[1].purpose, undefined);
      assert.strictEqual(podcast.txt[1].value, 'plain text record');

      assert.strictEqual(podcast.updateFrequency.rrule, 'FREQ=WEEKLY');
      assert.strictEqual(podcast.updateFrequency.dtstart, '2023-01-01T00:00:00Z');
      assert.strictEqual(podcast.updateFrequency.complete, false);
      assert.strictEqual(podcast.updateFrequency.label, 'weekly');

      assert.strictEqual(podcast.follow.length, 1);
      assert.strictEqual(podcast.follow[0].url, 'https://mastodon.social/@podcast');
      assert.strictEqual(podcast.follow[0].platform, 'activitypub');
    });

    it('should default podroll/location/txt/updateFrequency/follow at feed level when absent', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <podcast:guid>abc-123-def</podcast:guid>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const podcast = feed.feed.podcast;

      assert.strictEqual(podcast.location, undefined);
      assert.strictEqual(podcast.podroll.length, 0);
      assert.strictEqual(podcast.txt.length, 0);
      assert.strictEqual(podcast.updateFrequency, undefined);
      assert.strictEqual(podcast.follow.length, 0);
    });

    it('should parse item-level podcast:value with valueTimeSplit', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <item>
              <title>Episode</title>
              <podcast:value type="lightning" method="keysend">
                <podcast:valueRecipient type="node" address="addr1" split="100"/>
                <podcast:valueTimeSplit startTime="60" duration="30" remoteStartTime="0" remotePercentage="100">
                  <podcast:valueRecipient type="node" address="addr2" split="100"/>
                </podcast:valueTimeSplit>
              </podcast:value>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const entry = feed.entries[0];
      assert.ok(entry.podcast.value);
      assert.strictEqual(entry.podcast.value.type, 'lightning');
      assert.strictEqual(entry.podcast.value.recipients.length, 1);
      assert.strictEqual(entry.podcast.value.timeSplits.length, 1);
      const [split] = entry.podcast.value.timeSplits;
      assert.strictEqual(split.startTime, 60);
      assert.strictEqual(split.duration, 30);
      assert.strictEqual(split.recipients[0].address, 'addr2');
    });

    it('should parse podcast:alternateEnclosure/location/socialInteract/txt/follow at entry level', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <item>
              <title>Episode 1</title>
              <podcast:alternateEnclosure type="audio/mpeg" length="12345" bitrate="128" default="true">
                <podcast:source uri="https://example.com/ep1.mp3"/>
                <podcast:source uri="https://cdn.example.com/ep1.mp3" contentType="audio/mpeg"/>
                <podcast:integrity type="sri">sha256-abc123==</podcast:integrity>
              </podcast:alternateEnclosure>
              <podcast:location geo="geo:40.7128,-74.0060">New York</podcast:location>
              <podcast:socialInteract uri="https://mastodon.social/@host/status/1" protocol="activitypub" accountId="@host@mastodon.social" priority="1"/>
              <podcast:txt purpose="license">MIT</podcast:txt>
              <podcast:follow url="https://twitter.com/podcast" platform="twitter"/>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const podcast = feed.entries[0].podcast;

      const [ae] = podcast.alternateEnclosures;
      assert.strictEqual(podcast.alternateEnclosures.length, 1);
      assert.strictEqual(ae.type, 'audio/mpeg');
      // File size round-trips exactly for ordinary values, though it is stored as f64
      // (napi has no FromNapiValue for u64); see PodcastAlternateEnclosure.length doc comment.
      assert.strictEqual(ae.length, 12345);
      assert.strictEqual(ae.bitrate, 128);
      assert.strictEqual(ae.default, true);
      assert.strictEqual(ae.sources.length, 2);
      assert.strictEqual(ae.sources[0].uri, 'https://example.com/ep1.mp3');
      assert.strictEqual(ae.sources[1].uri, 'https://cdn.example.com/ep1.mp3');
      assert.strictEqual(ae.sources[1].contentType, 'audio/mpeg');
      assert.strictEqual(ae.integrity.type, 'sri');
      assert.strictEqual(ae.integrity.value, 'sha256-abc123==');

      assert.strictEqual(podcast.location.name, 'New York');
      assert.strictEqual(podcast.location.geo, 'geo:40.7128,-74.0060');
      assert.strictEqual(podcast.location.osm, undefined);

      assert.strictEqual(podcast.socialInteract.length, 1);
      const [si] = podcast.socialInteract;
      assert.strictEqual(si.uri, 'https://mastodon.social/@host/status/1');
      assert.strictEqual(si.protocol, 'activitypub');
      assert.strictEqual(si.accountId, '@host@mastodon.social');
      assert.strictEqual(si.priority, 1);

      assert.strictEqual(podcast.txt.length, 1);
      assert.strictEqual(podcast.txt[0].purpose, 'license');
      assert.strictEqual(podcast.txt[0].value, 'MIT');

      assert.strictEqual(podcast.follow.length, 1);
      assert.strictEqual(podcast.follow[0].url, 'https://twitter.com/podcast');
      assert.strictEqual(podcast.follow[0].platform, 'twitter');
    });

    it('should parse podcast:alternateEnclosure with multiple sources and integrity', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <title>Feed</title>
            <item>
              <title>Episode</title>
              <podcast:alternateEnclosure type="audio/mpeg" length="123456" bitrate="128000" height="720" lang="en" title="High quality" rel="default" codecs="mp4a.40.2" default="true">
                <podcast:source uri="https://example.com/ep1.mp3" contentType="audio/mpeg"/>
                <podcast:source uri="https://example.com/ep1-mirror.mp3"/>
                <podcast:integrity type="sri">sha256-abc</podcast:integrity>
              </podcast:alternateEnclosure>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const entry = feed.entries[0];
      assert.strictEqual(entry.podcast.alternateEnclosures.length, 1);
      const [enc] = entry.podcast.alternateEnclosures;
      assert.strictEqual(enc.type, 'audio/mpeg');
      assert.strictEqual(enc.length, 123456);
      assert.strictEqual(typeof enc.length, 'number');
      assert.strictEqual(enc.bitrate, 128000);
      assert.strictEqual(enc.height, 720);
      assert.strictEqual(enc.lang, 'en');
      assert.strictEqual(enc.title, 'High quality');
      assert.strictEqual(enc.rel, 'default');
      assert.strictEqual(enc.codecs, 'mp4a.40.2');
      assert.strictEqual(enc.default, true);
      assert.strictEqual(enc.sources.length, 2);
      assert.strictEqual(enc.sources[0].uri, 'https://example.com/ep1.mp3');
      assert.strictEqual(enc.sources[0].contentType, 'audio/mpeg');
      assert.strictEqual(enc.sources[1].uri, 'https://example.com/ep1-mirror.mp3');
      assert.strictEqual(enc.sources[1].contentType, undefined);
      assert.ok(enc.integrity);
      assert.strictEqual(enc.integrity.type, 'sri');
      assert.strictEqual(enc.integrity.value, 'sha256-abc');
    });

    it('should default value/alternateEnclosures/location/socialInteract/txt/follow at entry level when absent', () => {
      const xml = `<?xml version="1.0"?>
        <rss version="2.0" xmlns:podcast="https://podcastindex.org/namespace/1.0">
          <channel>
            <item>
              <title>Episode 1</title>
              <podcast:season number="3">Season Three</podcast:season>
            </item>
          </channel>
        </rss>`;

      const feed = parse(xml);
      const podcast = feed.entries[0].podcast;

      assert.strictEqual(podcast.value, undefined);
      assert.strictEqual(podcast.alternateEnclosures.length, 0);
      assert.strictEqual(podcast.location, undefined);
      assert.strictEqual(podcast.socialInteract.length, 0);
      assert.strictEqual(podcast.txt.length, 0);
      assert.strictEqual(podcast.follow.length, 0);
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
      assert.ok(feed.feed.where);
      assert.strictEqual(feed.feed.where.geoType, 'point');
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
      assert.ok(entry.where);
      assert.strictEqual(entry.where.geoType, 'point');
      // Media thumbnails field exists (empty array when no media)
      assert.ok(Array.isArray(entry.mediaThumbnails));
    });
  });
});
