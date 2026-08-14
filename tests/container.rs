// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for container metadata cleaning round-trips.

use cum_rs::container_meta::{ContainerFormat, clean_file, inspect_file};

#[test]
fn test_html_round_trip() {
    let html = b"<!DOCTYPE html>\n<html>\n<head>\n<meta name=\"generator\" content=\"Claude 3.5 Sonnet\" />\n<title>Test</title>\n</head>\n<body data-ai-provider=\"claude\">\n<p>Some content</p>\n</body>\n</html>";

    let before = inspect_file(html, &ContainerFormat::Html);
    assert!(!before.is_empty(), "should detect AI metadata before clean");

    let (cleaned, stats) = clean_file(html, &ContainerFormat::Html).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();

    assert!(!cleaned_str.contains("generator"), "generator meta removed");
    assert!(
        !cleaned_str.contains("data-ai-provider"),
        "data-ai-provider removed"
    );
    assert!(
        cleaned_str.contains("Some content"),
        "body content preserved"
    );
    assert!(
        cleaned_str.contains("<title>Test</title>"),
        "title preserved"
    );
    assert!(stats.removed_count > 0 || stats.metadata_chunks_removed > 0);

    let after = inspect_file(&cleaned, &ContainerFormat::Html);
    assert!(
        !after.iter().any(|f| f.description.contains("generator")),
        "generator should not appear in post-clean inspection"
    );
}

#[test]
fn test_markdown_round_trip() {
    let md = b"---\ntitle: My Blog Post\ngenerator: claude-3.5-sonnet\nai-model: claude\nauthor: Jane Doe\ndate: 2026-08-14\n---\n\n# My Blog Post\n\nThis is the post content.\n";

    let before = inspect_file(md, &ContainerFormat::Markdown);
    assert!(
        before.iter().any(|f| f.description.contains("generator")),
        "should detect generator key"
    );

    let (cleaned, stats) = clean_file(md, &ContainerFormat::Markdown).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();

    assert!(
        !cleaned_str.contains("generator: claude"),
        "generator key removed"
    );
    assert!(
        !cleaned_str.contains("ai-model: claude"),
        "ai-model key removed"
    );
    assert!(
        cleaned_str.contains("title: My Blog Post"),
        "title preserved"
    );
    assert!(cleaned_str.contains("author: Jane Doe"), "author preserved");
    assert!(
        cleaned_str.contains("# My Blog Post"),
        "body heading preserved"
    );
    assert!(
        cleaned_str.contains("This is the post content."),
        "body text preserved"
    );
    assert_eq!(stats.removed_count, 2);
}

#[test]
fn test_pdf_xmp_round_trip() {
    let pdf = b"%PDF-1.7\nstream\n<?xpacket begin='\xef\xbb\xbf' id='W5M0MpCehiHzreSzNTczkc9d'?>\n<x:xmpmeta xmlns:x='adobe:ns:meta/'>\n  <rdf:Description rdf:about='' ai:agent='claude-3.5'/>\n</x:xmpmeta>\n<?xpacket end='w'?>\nendstream";

    let before = inspect_file(pdf, &ContainerFormat::Pdf);
    assert!(before.iter().any(|f| f.description.contains("XMP")));

    let (cleaned, stats) = clean_file(pdf, &ContainerFormat::Pdf).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();

    assert!(!cleaned_str.contains("<?xpacket"), "xpacket header removed");
    assert!(
        !cleaned_str.contains("<?xpacket end="),
        "xpacket trailer removed"
    );
    assert!(cleaned_str.contains("%PDF-1.7"), "PDF header preserved");
    assert_eq!(stats.metadata_chunks_removed, 1);
}

#[test]
fn test_html_json_ld_c2pa_round_trip() {
    let html = b"<html><head><script type=\"application/ld+json\">{\"@context\": \"https://schema.org\", \"trainedAlgorithmicMedia\": true, \"creator\": {\"@type\": \"Organization\", \"name\": \"Anthropic\"}}</script></head><body><p>Hello</p></body></html>";

    let before = inspect_file(html, &ContainerFormat::Html);
    assert!(
        before
            .iter()
            .any(|f| f.description.contains("trainedAlgorithmicMedia"))
    );

    let (cleaned, _stats) = clean_file(html, &ContainerFormat::Html).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();

    assert!(
        !cleaned_str.contains("trainedAlgorithmicMedia"),
        "AI JSON-LD block removed"
    );
    assert!(
        cleaned_str.contains("<p>Hello</p>"),
        "body content preserved"
    );
}
