# Node.js Usage Guide: `cum-rs`

## Installation

```bash
npm install cum-rs
```

Or build from source:

```bash
npm install
npm run build
```

## Text Cleaning

```javascript
// If installed via npm: const { cleanText } = require("cum-rs");
// For local development:
const { cleanText } = require(".");

const result = cleanText("Hello\u200b world\ufeff!");
console.log(result.cleaned); // "Hello world!"
console.log(result.removedCount); // 2
```

## Text Inspection

```javascript
// If installed via npm: const { inspectText } = require("cum-rs");
// For local development:
const { inspectText } = require(".");

const report = inspectText("Hello\u200b world!");
console.log(`Length: ${report.length}`);
console.log(`Suspicious: ${report.suspiciousTotal}`);
report.hits.forEach((hit) => {
  console.log(`  ${hit.label} x${hit.count} [${hit.confidence}]`);
});
```

## File / Image Cleaning

```javascript
// If installed via npm: const { cleanBytes } = require("cum-rs");
// For local development:
const { cleanBytes } = require(".");
const fs = require("fs");

const data = fs.readFileSync("photo.png");
const cleaned = cleanBytes(data);
fs.writeFileSync("photo.cleaned.png", cleaned);
```

## TypeScript

```typescript
// If installed via npm: import { cleanText } from "cum-rs";
// For local development:
import { cleanText, inspectText, CleanTextResult, TextInspectReport } from ".";

const result: CleanTextResult = cleanText("Hello\u200b!");
```

## 🔍 See Also: Core Logic

The napi-rs hooks expose standard JavaScript strings and `Uint8Array` primitives directly into the Rust parser.

For details on the engine architecture and detection heuristics:

- The rust core leverages zero-copy buffer modifications where possible.
- Text strings are O(1) matching against pre-computed static codepoint slices in `clean_text`.
