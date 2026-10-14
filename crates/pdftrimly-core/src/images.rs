//! Optimize only understood 8-bit DeviceRGB/DeviceGray images, without masks.
use std::io::Cursor;

use image::{
    codecs::jpeg::{JpegDecoder, JpegEncoder},
    imageops::FilterType,
    ColorType, DynamicImage, GrayImage, ImageDecoder, Limits, RgbImage,
};
use lopdf::{Document, Object, Stream};

use crate::{image_usage, CompressionLevel, OptimizeOptions};

pub(crate) struct ImageStats {
    pub optimized: usize,
    pub skipped: usize,
}

fn integer(stream: &Stream, key: &[u8]) -> Option<u32> {
    let value = stream.dict.get(key).ok()?.as_i64().ok()?;
    u32::try_from(value).ok().filter(|value| *value > 0)
}

fn parameters_supported(stream: &Stream, width: u32, channels: u32) -> bool {
    let Ok(params) = stream.dict.get(b"DecodeParms") else {
        return true;
    };
    let Ok(params) = params.as_dict() else {
        return false;
    };
    let predictor = params
        .get(b"Predictor")
        .and_then(Object::as_i64)
        .unwrap_or(1);
    if predictor == 1 {
        return true;
    }
    matches!(predictor, 2 | 10..=15)
        && params.get(b"Columns").and_then(Object::as_i64).ok() == Some(i64::from(width))
        && params.get(b"Colors").and_then(Object::as_i64).unwrap_or(1) == i64::from(channels)
        && params
            .get(b"BitsPerComponent")
            .and_then(Object::as_i64)
            .unwrap_or(8)
            == 8
}

// Check JPEG component count independently: image decoders may convert CMYK
// to RGB automatically, which is not an authorized PDF color-space conversion.
fn jpeg_components(bytes: &[u8]) -> Option<u8> {
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            return None;
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        let marker = *bytes.get(offset)?;
        offset += 1;
        if matches!(marker, 0xd9 | 0xda) {
            return None;
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let size = u16::from_be_bytes([*bytes.get(offset)?, *bytes.get(offset + 1)?]) as usize;
        if size < 2 || offset.checked_add(size)? > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            return (size >= 8).then(|| bytes[offset + 7]);
        }
        offset += size;
    }
    None
}

fn decode(
    stream: &Stream,
    width: u32,
    height: u32,
    channels: u32,
    limit: usize,
) -> Option<DynamicImage> {
    let filter = match stream.dict.get(b"Filter") {
        Err(_) => None,
        Ok(Object::Name(name)) => Some(name.as_slice()),
        // Filter chains and indirect filters are left untouched.
        _ => return None,
    };
    let expected = (u64::from(width) * u64::from(height) * u64::from(channels)) as usize;
    if filter == Some(b"DCTDecode") {
        if stream.dict.has(b"DecodeParms") || jpeg_components(&stream.content)? != channels as u8 {
            return None;
        }
        let mut decoder = JpegDecoder::new(Cursor::new(&stream.content)).ok()?;
        let color = if channels == 3 {
            ColorType::Rgb8
        } else {
            ColorType::L8
        };
        if decoder.dimensions() != (width, height)
            || decoder.color_type() != color
            || decoder.icc_profile().ok()?.is_some()
        {
            return None;
        }
        let mut limits = Limits::default();
        limits.max_alloc = Some(limit as u64);
        limits.max_image_width = Some(width);
        limits.max_image_height = Some(height);
        decoder.set_limits(limits).ok()?;
        DynamicImage::from_decoder(decoder).ok()
    } else if filter.is_none() || filter == Some(b"FlateDecode") {
        if !parameters_supported(stream, width, channels) {
            return None;
        }
        let bytes = stream.get_plain_content_with_limit(limit).ok()?;
        if bytes.len() != expected {
            return None;
        }
        if channels == 3 {
            Some(DynamicImage::ImageRgb8(RgbImage::from_raw(
                width, height, bytes,
            )?))
        } else {
            Some(DynamicImage::ImageLuma8(GrayImage::from_raw(
                width, height, bytes,
            )?))
        }
    } else {
        None
    }
}

fn candidate(
    stream: &Stream,
    size: (f64, f64),
    options: &OptimizeOptions,
    budget: &mut usize,
) -> Option<Stream> {
    // Do not change alpha/masks, color-key masks, decode mappings or complex spaces.
    if [
        b"SMask".as_slice(),
        b"Mask",
        b"Decode",
        b"Alternates",
        b"OPI",
        b"F",
        b"FFilter",
        b"FDecodeParms",
    ]
    .iter()
    .any(|key| stream.dict.has(key))
        || stream
            .dict
            .get(b"ImageMask")
            .ok()
            .is_some_and(|value| value != &Object::Boolean(false))
        || stream
            .dict
            .get(b"SMaskInData")
            .ok()
            .is_some_and(|value| value != &Object::Integer(0))
        || stream
            .dict
            .get(b"BitsPerComponent")
            .and_then(Object::as_i64)
            .ok()
            != Some(8)
    {
        return None;
    }
    let channels = match stream
        .dict
        .get(b"ColorSpace")
        .and_then(Object::as_name)
        .ok()?
    {
        b"DeviceRGB" => 3,
        b"DeviceGray" => 1,
        _ => return None,
    };
    let width = integer(stream, b"Width")?;
    let height = integer(stream, b"Height")?;
    let pixels = u64::from(width) * u64::from(height);
    let bytes = pixels.checked_mul(u64::from(channels))?;
    if pixels > options.max_image_pixels
        || bytes > options.max_image_decoded_bytes as u64
        || bytes > *budget as u64
    {
        return None;
    }
    // Charge attempted decoding too, even if a candidate does not save bytes.
    *budget -= bytes as usize;
    let image = decode(
        stream,
        width,
        height,
        channels,
        options.max_image_decoded_bytes,
    )?;
    let (dpi, quality) = match options.level {
        CompressionLevel::Light => return None,
        CompressionLevel::Normal => (150.0, 80),
        CompressionLevel::Ultra => (96.0, 60),
        CompressionLevel::Maximum => (72.0, 40),
    };
    // Account for the largest placement on either axis; never upscale an image.
    let scale = (size.0 * dpi / (72.0 * f64::from(width)))
        .max(size.1 * dpi / (72.0 * f64::from(height)))
        .min(1.0);
    let target_width = (f64::from(width) * scale).ceil().max(1.0) as u32;
    let target_height = (f64::from(height) * scale).ceil().max(1.0) as u32;
    let image = if (target_width, target_height) == (width, height) {
        image
    } else {
        image.resize_exact(target_width, target_height, FilterType::Lanczos3)
    };
    let mut encoded = Vec::new();
    JpegEncoder::new_with_quality(&mut encoded, quality)
        .encode(
            image.as_bytes(),
            target_width,
            target_height,
            image.color().into(),
        )
        .ok()?;
    // Require a meaningful saving to avoid repeatedly degrading near-identical JPEGs.
    if encoded.len().saturating_add(64) >= stream.content.len().saturating_mul(95) / 100 {
        return None;
    }
    let check = JpegDecoder::new(Cursor::new(&encoded)).ok()?;
    if check.dimensions() != (target_width, target_height) {
        return None;
    }
    let mut result = stream.clone();
    result.set_content(encoded);
    result.dict.set("Filter", "DCTDecode");
    result.dict.set("Width", i64::from(target_width));
    result.dict.set("Height", i64::from(target_height));
    result.dict.remove(b"DecodeParms");
    Some(result)
}

pub(crate) fn optimize_images(document: &mut Document, options: &OptimizeOptions) -> ImageStats {
    let usage = if options.level == CompressionLevel::Light {
        None
    } else {
        image_usage::analyze(document, options)
    };
    let mut budget = options.max_total_image_decoded_bytes;
    let mut stats = ImageStats {
        optimized: 0,
        skipped: 0,
    };
    for (id, object) in &mut document.objects {
        let Object::Stream(stream) = object else {
            continue;
        };
        if stream.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Image") {
            continue;
        }
        let replacement = usage.as_ref().and_then(|usage| {
            if usage.blocked.contains(id) {
                return None;
            }
            candidate(stream, *usage.sizes.get(id)?, options, &mut budget)
        });
        if let Some(replacement) = replacement {
            *stream = replacement;
            stats.optimized += 1;
        } else {
            stats.skipped += 1;
        }
    }
    stats
}
