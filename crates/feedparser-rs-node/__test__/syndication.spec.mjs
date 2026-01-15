import assert from 'node:assert';
import { describe, it } from 'node:test';
import { parse } from '../index.js';

describe('syndication', () => {
  it('should parse syndication updatePeriod', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test Feed</title>
          <link>https://example.com</link>
          <syn:updatePeriod>daily</syn:updatePeriod>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    assert.ok(feed.feed.syndication);
    assert.strictEqual(feed.feed.syndication.updatePeriod, 'daily');
  });

  it('should parse syndication updateFrequency', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test Feed</title>
          <link>https://example.com</link>
          <syn:updateFrequency>2</syn:updateFrequency>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    assert.ok(feed.feed.syndication);
    assert.strictEqual(feed.feed.syndication.updateFrequency, 2);
  });

  it('should parse complete syndication metadata', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test Feed</title>
          <link>https://example.com</link>
          <syn:updatePeriod>hourly</syn:updatePeriod>
          <syn:updateFrequency>1</syn:updateFrequency>
          <syn:updateBase>2024-01-01T00:00:00Z</syn:updateBase>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    const syn = feed.feed.syndication;
    assert.ok(syn);
    assert.strictEqual(syn.updatePeriod, 'hourly');
    assert.strictEqual(syn.updateFrequency, 1);
    assert.strictEqual(syn.updateBase, '2024-01-01T00:00:00Z');
  });

  it('should return undefined when syndication data is missing', () => {
    const xml = `<?xml version="1.0"?>
      <rss version="2.0">
        <channel>
          <title>Test Feed</title>
          <link>https://example.com</link>
        </channel>
      </rss>`;

    const feed = parse(xml);
    assert.strictEqual(feed.feed.syndication, undefined);
  });

  it('should parse Dublin Core fields', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:dc="http://purl.org/dc/elements/1.1/">
        <channel rdf:about="https://example.com">
          <title>Test Feed</title>
          <link>https://example.com</link>
          <dc:creator>John Doe</dc:creator>
          <dc:publisher>ACME Corp</dc:publisher>
          <dc:rights>Copyright 2024</dc:rights>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    assert.strictEqual(feed.feed.dcCreator, 'John Doe');
    assert.strictEqual(feed.feed.dcPublisher, 'ACME Corp');
    assert.strictEqual(feed.feed.dcRights, 'Copyright 2024');
  });

  it('should handle invalid updatePeriod gracefully (bozo pattern)', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test</title>
          <link>https://example.com</link>
          <syn:updatePeriod>invalid</syn:updatePeriod>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    // Should not crash, syndication should be undefined or updatePeriod undefined
    assert.ok(!feed.feed.syndication || !feed.feed.syndication.updatePeriod);
  });

  it('should handle case-insensitive updatePeriod', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test</title>
          <link>https://example.com</link>
          <syn:updatePeriod>HOURLY</syn:updatePeriod>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    assert.ok(feed.feed.syndication);
    assert.strictEqual(feed.feed.syndication.updatePeriod, 'hourly');
  });

  it('should parse feed with partial syndication fields', () => {
    const xml = `<?xml version="1.0"?>
      <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
               xmlns="http://purl.org/rss/1.0/"
               xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
        <channel>
          <title>Test</title>
          <link>https://example.com</link>
          <syn:updatePeriod>weekly</syn:updatePeriod>
        </channel>
      </rdf:RDF>`;

    const feed = parse(xml);
    assert.ok(feed.feed.syndication);
    assert.strictEqual(feed.feed.syndication.updatePeriod, 'weekly');
    assert.strictEqual(feed.feed.syndication.updateFrequency, undefined);
    assert.strictEqual(feed.feed.syndication.updateBase, undefined);
  });
});
