// Copyright 2026 Mahmoud Harmouch.
//
// Licensed under the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for the container metadata watermark strippers.

use cum_rs::container_meta::{ContainerFormat, clean_file, inspect_file};

#[test]
fn test_detect_pdf_format() {
    let pdf = b"%PDF-1.7 fake pdf content";
    assert!(ContainerFormat::detect(pdf).is_some());
    assert_eq!(ContainerFormat::detect(pdf).unwrap(), ContainerFormat::Pdf);
}

#[test]
fn test_inspect_pdf_finds_xmp_packet() {
    let pdf = b"%PDF-1.7\n<?xpacket begin='\\xef\\xbb\\xbf' id='W5M0MpCehiHzreSzNTczkc9d'?>\n<x:xmpmeta>ai-generated-by-claude</x:xmpmeta><?xpacket end='w'?>";
    let findings = inspect_file(pdf, &ContainerFormat::Pdf);
    assert!(!findings.is_empty(), "should detect XMP packet in PDF");
    assert!(findings.iter().any(|f| f.description.contains("XMP")));
}

#[test]
fn test_inspect_pdf_finds_info_dict_keys() {
    let pdf = b"%PDF-1.7\n/Info << /Creator (Claude 3.5) /Producer (Anthropic AI) >>";
    let findings = inspect_file(pdf, &ContainerFormat::Pdf);
    assert!(
        findings
            .iter()
            .any(|f| f.description.contains("Creator") || f.description.contains("Producer"))
    );
}

#[test]
fn test_clean_pdf_removes_xmp_packet() {
    let pdf = b"%PDF-1.7\n<?xpacket begin='' id='test'?>\n<x:xmpmeta>AI metadata</x:xmpmeta>\n<?xpacket end='w'?>\nstream content";
    let (cleaned, stats) = clean_file(pdf, &ContainerFormat::Pdf).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(
        !cleaned_str.contains("<?xpacket"),
        "XMP packet should be removed"
    );
    assert_eq!(stats.metadata_chunks_removed, 1);
}

#[test]
fn test_inspect_html_finds_generator_meta() {
    let html =
        b"<!DOCTYPE html><html><head><meta name='generator' content='Claude AI'/></head></html>";
    let findings = inspect_file(html, &ContainerFormat::Html);
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.description.contains("generator")));
}

#[test]
fn test_inspect_html_finds_data_ai_attributes() {
    let html = b"<html><body data-ai-provider='claude'></body></html>";
    let findings = inspect_file(html, &ContainerFormat::Html);
    assert!(findings.iter().any(|f| f.description.contains("data-ai-")));
}

#[test]
fn test_inspect_html_finds_json_ld_provenance() {
    let html = b"<script type='application/ld+json'>{\"trainedAlgorithmicMedia\": true}</script>";
    let findings = inspect_file(html, &ContainerFormat::Html);
    assert!(
        findings
            .iter()
            .any(|f| f.description.contains("trainedAlgorithmicMedia"))
    );
}

#[test]
fn test_clean_html_removes_generator_meta() {
    let html = b"<html><head><meta name=\"generator\" content=\"Claude AI\"/></head><body>text</body></html>";
    let (cleaned, _stats) = clean_file(html, &ContainerFormat::Html).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(!cleaned_str.contains("generator"));
    assert!(
        cleaned_str.contains("text"),
        "body content should be preserved"
    );
}

#[test]
fn test_clean_html_removes_data_ai_attrs() {
    let html = b"<html><body data-ai-provider=\"claude\"><p>Hello</p></body></html>";
    let (cleaned, _stats) = clean_file(html, &ContainerFormat::Html).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(!cleaned_str.contains("data-ai-provider"));
    assert!(
        cleaned_str.contains("Hello"),
        "body text should be preserved"
    );
}

#[test]
fn test_inspect_markdown_finds_ai_frontmatter() {
    let md = b"---\ngenerator: claude-3.5\ntitle: My Post\n---\n# Hello\n";
    let findings = inspect_file(md, &ContainerFormat::Markdown);
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|f| f.description.contains("generator")));
}

#[test]
fn test_inspect_markdown_no_frontmatter_no_findings() {
    let md = b"# Hello\nThis is a post.\n";
    let findings = inspect_file(md, &ContainerFormat::Markdown);
    assert!(
        findings.is_empty(),
        "Markdown without front-matter should have no findings"
    );
}

#[test]
fn test_clean_markdown_removes_ai_keys() {
    let md = b"---\ngenerator: claude\ntitle: Test Post\nauthor: Jane\n---\n# Hello\n";
    let (cleaned, stats) = clean_file(md, &ContainerFormat::Markdown).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(
        !cleaned_str.contains("generator: claude"),
        "generator key should be removed"
    );
    assert!(
        cleaned_str.contains("title: Test Post"),
        "non-AI keys should be preserved"
    );
    assert!(
        cleaned_str.contains("author: Jane"),
        "author key should be preserved"
    );
    assert_eq!(stats.removed_count, 1);
}

#[test]
fn test_clean_markdown_preserves_body() {
    let md = b"---\ngenerator: claude\n---\n# Hello\nBody text here.\n";
    let (cleaned, _stats) = clean_file(md, &ContainerFormat::Markdown).unwrap();
    let cleaned_str = std::str::from_utf8(&cleaned).unwrap();
    assert!(
        cleaned_str.contains("# Hello"),
        "body heading should be preserved"
    );
    assert!(
        cleaned_str.contains("Body text here."),
        "body text should be preserved"
    );
}

#[test]
fn test_clean_markdown_no_frontmatter_unchanged() {
    let md = b"# Hello\nNo front matter here.\n";
    let (cleaned, stats) = clean_file(md, &ContainerFormat::Markdown).unwrap();
    assert_eq!(cleaned, md);
    assert_eq!(stats.removed_count, 0);
}

#[test]
fn test_container_format_as_str() {
    assert_eq!(ContainerFormat::Pdf.as_str(), "PDF");
    assert_eq!(ContainerFormat::Html.as_str(), "HTML");
    assert_eq!(ContainerFormat::Markdown.as_str(), "Markdown");
}
