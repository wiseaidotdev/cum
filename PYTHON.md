# Python Usage Guide: `cum-rs`

## Installation

```bash
pip install cum-rs
```

Or build from source with [maturin](https://maturin.rs/):

```bash
pip install maturin
maturin develop --features python
```

## Text Cleaning

```python
>>> import cum_rs
>>>
>>> result = cum_rs.clean_text("Hello\u200b world\ufeff!")
>>> print(result.cleaned)
Hello world!
>>> print(result.removed_count)
2
>>> print(result.summary)
['zwj_family: 2']
```

## Text Inspection

```python
>>> import cum_rs
>>>
>>> report = cum_rs.inspect_text("Hello\u200b world!")
>>> print(f"Length: {report.length}")
Length: 13
>>> print(f"Suspicious: {report.suspicious_total}")
Suspicious: 1
>>> for hit in report.hits:
...     print(f"  {hit.label} x{hit.count} [{hit.confidence}]")
...
  U+200B ZERO WIDTH SPACE (Format) x1 [probable]
```

## File / Image Cleaning

```python
import cum_rs

with open("photo.png", "rb") as f:
    data = f.read()

cleaned = cum_rs.clean_bytes(data)

with open("photo.cleaned.png", "wb") as f:
    f.write(cleaned)
```

## 🔍 See Also: Core Logic

The PyO3 Python hooks seamlessly route queries into the Rust core, ensuring no Python loops block processing. String inspection is zero-copy until modification is detected.

For architecture and heuristic details on _how_ cleaning works natively, see:

- `cum_rs::unicode` for text algorithms.
- `cum_rs::image_meta` for PNG/JPEG headers.
