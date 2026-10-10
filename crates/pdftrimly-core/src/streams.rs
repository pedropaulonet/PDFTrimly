use std::io::{Read, Write};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use lopdf::{Document, Object};

use crate::{OptimizeError, OptimizeOptions};

pub(crate) struct StreamStats {
    pub optimized: usize,
    pub skipped: usize,
}

pub(crate) fn optimize_streams(
    document: &mut Document,
    options: &OptimizeOptions,
) -> Result<StreamStats, OptimizeError> {
    let mut stats = StreamStats {
        optimized: 0,
        skipped: 0,
    };
    let mut total_decoded = 0usize;
    for object in document.objects.values_mut() {
        let Object::Stream(stream) = object else {
            continue;
        };
        // Leave image codecs, predictors and external stream files untouched.
        let image = stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image");
        let special = stream
            .dict
            .get(b"Type")
            .and_then(Object::as_name)
            .ok()
            .is_some_and(|name| name == b"XRef" || name == b"ObjStm");
        if image
            || special
            || [b"DecodeParms".as_slice(), b"F", b"FFilter", b"FDecodeParms"]
                .iter()
                .any(|key| stream.dict.has(key))
        {
            stats.skipped += 1;
            continue;
        }
        let flate = match stream.dict.get(b"Filter") {
            Err(_) => false,
            Ok(Object::Name(name)) if name == b"FlateDecode" => true,
            _ => {
                stats.skipped += 1;
                continue;
            }
        };
        let remaining = options
            .max_total_decoded_bytes
            .saturating_sub(total_decoded);
        let limit = options.max_stream_bytes.min(remaining);
        let plain = if flate {
            let mut decoder = ZlibDecoder::new(stream.content.as_slice());
            let mut plain = Vec::new();
            decoder
                .by_ref()
                .take((limit as u64).saturating_add(1))
                .read_to_end(&mut plain)?;
            if plain.len() > limit {
                return Err(OptimizeError::LimitExceeded("fluxos descomprimidos"));
            }
            // Do not silently discard trailing data in malformed zlib streams.
            if decoder.total_in() as usize != stream.content.len() {
                return Err(OptimizeError::Structure("dados extras após fluxo Flate"));
            }
            plain
        } else {
            if stream.content.len() > limit {
                return Err(OptimizeError::LimitExceeded("fluxos descomprimidos"));
            }
            stream.content.clone()
        };
        total_decoded += plain.len();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
        encoder.write_all(&plain)?;
        let compressed = encoder.finish()?;
        // Cover dictionary overhead when adding a Filter to a plain stream.
        let overhead = if flate { 0 } else { 32 };
        if compressed.len().saturating_add(overhead) < stream.content.len() {
            // Independently decode the candidate before replacing original data.
            let mut verified = Vec::new();
            ZlibDecoder::new(compressed.as_slice())
                .take(plain.len() as u64 + 1)
                .read_to_end(&mut verified)?;
            if verified != plain {
                return Err(OptimizeError::Structure("a recompressão alterou o fluxo"));
            }
            stream.set_content(compressed);
            stream.dict.set("Filter", "FlateDecode");
            stats.optimized += 1;
        } else {
            stats.skipped += 1;
        }
    }
    Ok(stats)
}
