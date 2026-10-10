# Image optimization and OCR preservation

Implemented on 2026-10-10. Processing remains local and originals are preserved.

| Level | Effective image resolution | JPEG quality |
| --- | --- | --- |
| Light | Original image data | Unchanged |
| Normal (default) | 150 DPI | 80 |
| Ultra | 96 DPI | 60 |

Normal and Ultra intentionally change image quality. The engine edits eligible
image objects, preserving page text, fonts, positioning and invisible OCR text.
It does not create OCR. A regression fixture contains invisible text (`3 Tr`)
over a scan; both levels must preserve extracted text and exact decoded page
commands while reducing image resolution.

Supported images are 8-bit DeviceRGB/DeviceGray using raw, Flate or JPEG data.
Eligible Flate predictors are decoded before encoding. Placement analysis
includes inherited resources, page UserUnit, nested forms and shared images;
the largest placement determines resolution. Images are never enlarged.
Masks, complex color spaces, embedded JPEG ICC profiles and images used in
annotation appearances or other unanalyzed contexts are preserved. Incomplete
placement analysis preserves images. Candidates need a material byte saving.

Default bounds include 32 million pixels per image, 128 MiB decoded bytes per
image and 512 MiB cumulative image decoding. Unsupported or oversized images
are skipped. Structural validation rejects signatures and encryption, checks
references and page trees, and validates unchanged object semantics. Exact
integer/real equivalence is allowed; approximate numeric changes are rejected.

## Local measurements

Private fixtures remain outside version control. Sizes use decimal MB.

| Document | Original | Normal | Ultra |
| --- | ---: | ---: | ---: |
| 10-page image document | 26.43 MB | 9.57 MB (63.79% saved) | 6.64 MB (74.86% saved) |
| 11-page text/vector document | 68,638 bytes | 67,115 bytes (2.22%) | 67,115 bytes (2.22%) |
| 4-page signed document | 442,802 bytes | Rejected | Rejected |

Eight scan images were optimized; four masked image objects were preserved.
All output page counts and extracted text matched. The text/vector document
rendered pixel-identically on all 11 pages at 72 DPI. The image document has
intentional rendering differences on eight pages. Normal pages 1, 3 and 8,
and Ultra page 3 were visually inspected at a 1600-pixel render; this is a
sample review, not a guarantee of visual equivalence for all documents.
The image document has no OCR, so OCR preservation is separately covered by
the invisible-text regression fixture.

The automated suite contains 29 tests across CLI, engine and image behavior.
Broader coverage of scan formats, photographs and complex PDF features remains
necessary before considering the engine production-ready.

## Reproduce

```sh
cargo build --release --locked
./target/release/pdftrimly compress input.pdf output.pdf --level normal
python3 scripts/verify-directory.py testes normal
python3 scripts/verify-directory.py testes ultra
python3 scripts/verify-poc.py
```

Independent validation requires Poppler tools. Each run creates a new output
directory and JSON report with original hashes, page/text comparisons and
per-page rendering differences. Lossy image differences require visual review;
a signed fixture produces an expected rejection and a nonzero script exit.
