# Historical lossless baseline: Engine proof of concept

Implemented and checked on Linux on 2026-10-10, using Rust 1.99.0.

## Usage

```sh
cargo run --locked -- compress "input.pdf" "output.pdf"
```

The output must not exist, including a dangling symlink. Existing destinations
are rejected. A temporary file is created in the destination directory and
published without overwrite after validation and synchronization. Errors drop
the temporary file. If the candidate is larger or unchanged, a byte-identical
copy of the input is published and reported as **sem economia**.

The CLI prints original/final byte sizes, percentage saved, pages, optimized
stream count, streams skipped during analysis and processing duration. Errors
use a nonzero exit code. No network or external compression tools are used
during processing; downloading build dependencies is separate.

## Implemented optimization and checks

- Zlib/Flate compression of unfiltered, internal, non-image streams only when
  the reduction covers the additional filter dictionary overhead.
- Recompression of streams with a single direct `/FlateDecode` filter and no
  decode parameters, only when the encoded data becomes smaller.
- Each new compressed stream is decoded and compared with the original
  decoded bytes before replacement.
- Strict PDF parsing, existing-object references, catalog and page tree
  checks, cycle detection, consistent parent links and declared page counts.
- When publishing a smaller candidate, reload it and compare the complete
  semantic object map against the prepared document and compare trailer
  entries and page count. Storage-only xref/object streams are rewritten as
  classic xref tables. No semantic objects are pruned or renumbered.
- Reject encrypted documents (including empty-password encryption), signature
  dictionaries, signature fields and linearized documents.
- Preserve image streams, unsupported filters, predictor streams and external
  stream-file dictionaries unchanged. This does not validate those codecs.

The candidate is written using the source PDF version. Searchable text and
vector commands are never rasterized. Tests additionally check image objects,
link annotations, form dictionaries, metadata and shared page content.

## Resource limits and remaining work

Default input limit: 64 MiB; decoded eligible-stream limit: 32 MiB per stream
and 128 MiB combined. Input is read with a bound. Parser decompression for
object/xref streams also receives a 32 MiB per-stream bound. Object count is
limited to 100,000, IDs to 1,000,000, and inspected object/page nesting to 64.

These bounds do not cap all parser memory or total execution time. An isolated
worker, aggregate parser limits and cooperative cancellation remain future
work. This is an in-process proof of concept, not a hardened document service.

Image downsampling/recompression, Normal/Ultra behavior, batch processing,
GUI and cancellation are not implemented. Complex streams are preserved, not
optimized. Structural/object validation does not establish perfect visual
equivalence. An independent Poppler comparison of the five generated fixtures
found identical rendered pixels at 72 DPI between each original and its output.
This is bounded synthetic coverage, not proof for arbitrary documents. Windows
execution has not been performed locally; CI is configured for Linux and Windows.

## Reproducible synthetic collection

```sh
mkdir -p dist
# The generator requires a new directory and never overwrites fixture files.
cargo run -p pdftrimly-core --example generate_fixtures --locked -- dist/poc-fixtures
cargo run --locked -- compress dist/poc-fixtures/text.pdf dist/poc-fixtures/text.optimized.pdf --level light
# Optional independent validation; requires pdftoppm installed separately.
python3 scripts/verify-poc.py
```

All fixtures are generated from project-owned content. They include repetitive
searchable text, vector drawing commands, a synthetic RGB checker image,
mixed content and an already-compressed document. They are intentionally
small and synthetic; they are not a representative real-world benchmark.

Measured debug-build results from this collection:

| Fixture | Input bytes | Output bytes | Saved | Result |
| --- | ---: | ---: | ---: | --- |
| Text | 148,528 | 1,172 | 99.21% | Reduced |
| Vectors | 148,528 | 1,060 | 99.29% | Reduced |
| Synthetic scan | 197,335 | 197,335 | 0% | Original copied |
| Mixed | 271,338 | 197,743 | 27.12% | Reduced |
| Already compressed | 1,138 | 1,138 | 0% | Original copied |

High reductions in text/vector fixtures reflect deliberately repetitive,
uncompressed streams and are not expected savings for arbitrary PDFs.
These baseline measurements use Light, which preserves image data.
The scan/mixed fixtures actually place the synthetic image on the page.

## Verification and acceptance status

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The initial integration suite had 15 engine tests and two CLI tests. It checks genuine
reduction, lossless decoded content, output preservation, size-growth fallback,
encrypted/signed rejection, malformed references and cyclic pages, decompression
limits, unsupported filters, multipage/shared content and CLI behavior.

The executable demonstrates real reduction and truthful unchanged outcomes.
Full Stage 2 acceptance from `projeto.md` remains pending a representative
collection of real text documents, scans, photographs, vectors and mixed PDFs,
plus broader feature coverage and visual comparisons. For current image support, real-document measurements and OCR regression
coverage, see [image optimization](image-optimization.md).
