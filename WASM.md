# WASM Usage Guide: `cum-rs`

## Building

```bash
wasm-pack build --target web --features wasm
# or via Trunk (see examples/unmark/):
cd examples/unmark && trunk serve
```

## Browser Usage (vanilla JS)

```html
<script type="module">
  import init, {
    clean_text_wasm,
    inspect_text_wasm,
    clean_bytes_wasm,
  } from "./pkg/cum_rs.js";

  await init();

  const result = JSON.parse(clean_text_wasm("Hello\u200b world\ufeff!"));
  console.log(result.cleaned); // "Hello world!"
  console.log(result.removed_count); // 2
</script>
```

## Yew / Rust front-end

See the live example in [`examples/unmark/`](examples/unmark/): a split-panel Yew 0.22 CSR app that accepts text or file upload on the left and shows the cleaned output on the right with Copy / Download buttons.

## CORS

`cum-rs` operates entirely client-side (pure computation on input bytes). No network requests are made.

## 🔍 See Also: Core Logic & Architecture

`cum-rs` leverages WASM to deliver native speeds directly into UI runtimes (Vanilla JS, React, Yew) entirely client-side. No API keys or remote queries are required, offering perfect privacy to end-users.

All formats (HTML, Markdown, Images, text) are processed via byte-level buffer allocations using standard `Uint8Array` memory shares between JS and WASM linear memory.
