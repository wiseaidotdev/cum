# Ethics & Responsible Use

`cum-rs` is a research and privacy-hygiene tool. Using it responsibly is the user's responsibility.

## ✅ Intended Use

- Removing invisible tracking characters from **text you wrote or own** before publishing.
- Stripping metadata from **images or documents you created** before sharing.
- Academic research into Unicode-based watermarking techniques.
- Testing your own content pipelines for unintended provenance leakage.

## ❌ Out of Scope

- Attempting to deceive AI safety detectors on content that violates platform terms of service.
- Claiming human authorship of AI-generated content where disclosure is required by law or policy.
- Removing watermarks from content **you do not own**.

## ⚠️ Limitations

Layer A (Unicode) and file-metadata stripping are **deterministic and verifiable**: what is removed is documented precisely in the stats output.

Statistical (Layer B) watermarks are embedded in the _wording_ of the text itself. No tool can certify that a statistical detector will fail after cleaning; the only reliable approach is to rewrite the text in your own words.

## 📄 License

MIT: see [LICENSE](LICENSE).
