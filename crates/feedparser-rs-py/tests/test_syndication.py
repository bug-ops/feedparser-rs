import feedparser_rs


def test_syndication_update_period():
    """Test syn:updatePeriod parsing"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
        <syn:updatePeriod>daily</syn:updatePeriod>
      </channel>
    </rdf:RDF>"""

    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is not None
    assert d.feed.syndication.update_period == "daily"


def test_syndication_update_frequency():
    """Test syn:updateFrequency parsing"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
        <syn:updateFrequency>2</syn:updateFrequency>
      </channel>
    </rdf:RDF>"""

    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is not None
    assert d.feed.syndication.update_frequency == 2


def test_syndication_update_base():
    """Test syn:updateBase parsing"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
        <syn:updateBase>2024-12-18T00:00:00Z</syn:updateBase>
      </channel>
    </rdf:RDF>"""

    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is not None
    assert d.feed.syndication.update_base == "2024-12-18T00:00:00Z"


def test_syndication_complete():
    """Test all syndication fields together"""
    feed_xml = b"""<?xml version="1.0"?>
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
    </rdf:RDF>"""

    d = feedparser_rs.parse(feed_xml)
    syn = d.feed.syndication
    assert syn is not None
    assert syn.update_period == "hourly"
    assert syn.update_frequency == 1
    assert syn.update_base == "2024-01-01T00:00:00Z"


def test_syndication_missing():
    """Test feed without syndication data"""
    feed_xml = b"""<?xml version="1.0"?>
    <rss version="2.0">
      <channel>
        <title>Test Feed</title>
        <link>https://example.com</link>
      </channel>
    </rss>"""

    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is None


def test_dublin_core_fields():
    """Test Dublin Core fields"""
    feed_xml = b"""<?xml version="1.0"?>
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
    </rdf:RDF>"""

    d = feedparser_rs.parse(feed_xml)
    assert d.feed.dc_creator == "John Doe"
    assert d.feed.dc_publisher == "ACME Corp"
    assert d.feed.dc_rights == "Copyright 2024"


def test_invalid_update_period():
    """Test invalid updatePeriod is handled gracefully (bozo pattern)"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test</title>
        <link>https://example.com</link>
        <syn:updatePeriod>invalid</syn:updatePeriod>
      </channel>
    </rdf:RDF>"""
    d = feedparser_rs.parse(feed_xml)
    # Should not crash, syndication should be None or update_period None
    assert d.feed.syndication is None or d.feed.syndication.update_period is None


def test_case_insensitive_update_period():
    """Test updatePeriod is case-insensitive"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test</title>
        <link>https://example.com</link>
        <syn:updatePeriod>HOURLY</syn:updatePeriod>
      </channel>
    </rdf:RDF>"""
    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is not None
    assert d.feed.syndication.update_period == "hourly"


def test_partial_syndication():
    """Test feed with only some syndication fields"""
    feed_xml = b"""<?xml version="1.0"?>
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
             xmlns="http://purl.org/rss/1.0/"
             xmlns:syn="http://purl.org/rss/1.0/modules/syndication/">
      <channel>
        <title>Test</title>
        <link>https://example.com</link>
        <syn:updatePeriod>weekly</syn:updatePeriod>
      </channel>
    </rdf:RDF>"""
    d = feedparser_rs.parse(feed_xml)
    assert d.feed.syndication is not None
    assert d.feed.syndication.update_period == "weekly"
    assert d.feed.syndication.update_frequency is None
    assert d.feed.syndication.update_base is None
