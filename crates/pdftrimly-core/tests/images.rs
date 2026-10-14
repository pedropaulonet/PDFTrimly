mod common;

use std::fs;

use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use pdftrimly_core::{optimize, CompressionLevel, OptimizeOptions, Outcome};
use tempfile::tempdir;

fn image_document() -> (Document, ObjectId) {
    let mut doc = common::document(20, true);
    let pixels: Vec<u8> = (0..600 * 300)
        .flat_map(|index| {
            let x = index % 600;
            let y = index / 600;
            [(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8]
        })
        .collect();
    let image = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 600,
            "Height" => 300, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        pixels,
    ));
    let page = doc.get_pages()[&1];
    let resources = doc
        .get_dictionary(page)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_reference()
        .unwrap();
    doc.get_dictionary_mut(resources)
        .unwrap()
        .set("XObject", dictionary! {"Im1" => image});
    doc.add_page_contents(page, b"q 72 0 0 36 72 200 cm /Im1 Do Q".to_vec())
        .unwrap();
    (doc, image)
}

#[test]
fn decodes_flate_images_and_png_predictors_before_jpeg_conversion() {
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;
    for predictor in [false, true] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("output.pdf");
        let (mut doc, id) = image_document();
        let stream = doc.get_object_mut(id).unwrap().as_stream_mut().unwrap();
        if predictor {
            let mut rows = Vec::new();
            for row in stream.content.as_chunks::<1800>().0 {
                rows.push(0); // PNG None filter byte per row.
                rows.extend_from_slice(row);
            }
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&rows).unwrap();
            stream.set_content(encoder.finish().unwrap());
            stream.dict.set("Filter", "FlateDecode");
            stream.dict.set(
                "DecodeParms",
                dictionary! {
                    "Predictor" => 15, "Columns" => 600, "Colors" => 3, "BitsPerComponent" => 8,
                },
            );
        } else {
            stream.compress().unwrap();
        }
        doc.save(&input).unwrap();
        let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
        assert_eq!(report.optimized_images, 1);
        let result = Document::load(output).unwrap();
        let stream = result.get_object(id).unwrap().as_stream().unwrap();
        assert!(!stream.dict.has(b"DecodeParms"));
        let decoded =
            image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (150, 75));
    }
}

#[test]
fn recompresses_rgb_jpeg_and_supports_grayscale_without_color_conversion() {
    for grayscale in [false, true] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("output.pdf");
        let (mut doc, id) = image_document();
        let stream = doc.get_object_mut(id).unwrap().as_stream_mut().unwrap();
        let pixels = if grayscale {
            stream.dict.set("ColorSpace", "DeviceGray");
            stream
                .content
                .as_chunks::<3>()
                .0
                .iter()
                .map(|rgb| rgb[0])
                .collect::<Vec<_>>()
        } else {
            stream.content.clone()
        };
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 100)
            .encode(
                &pixels,
                600,
                300,
                if grayscale {
                    image::ExtendedColorType::L8
                } else {
                    image::ExtendedColorType::Rgb8
                },
            )
            .unwrap();
        stream.set_content(jpeg);
        stream.dict.set("Filter", "DCTDecode");
        doc.save(&input).unwrap();
        let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
        assert_eq!(report.optimized_images, 1);
        let result = Document::load(output).unwrap();
        let stream = result.get_object(id).unwrap().as_stream().unwrap();
        let decoded =
            image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg).unwrap();
        assert_eq!(
            decoded.color(),
            if grayscale {
                image::ColorType::L8
            } else {
                image::ColorType::Rgb8
            }
        );
    }
}

#[test]
fn preserves_image_also_used_in_an_annotation_appearance() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let (mut doc, id) = image_document();
    let appearance = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 288.into(), 144.into()],
            "Resources" => dictionary! {"XObject" => dictionary! {"Im1" => id}},
        },
        b"q 288 0 0 144 0 0 cm /Im1 Do Q".to_vec(),
    ));
    let annotation = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Stamp", "Rect" => vec![0.into(), 0.into(), 288.into(), 144.into()],
        "AP" => dictionary! {"N" => appearance},
    });
    let page = doc.get_pages()[&1];
    doc.get_dictionary_mut(page)
        .unwrap()
        .set("Annots", vec![Object::Reference(annotation)]);
    doc.save(&input).unwrap();
    let original = Document::load(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.optimized_images, 0);
    let result = Document::load(output).unwrap();
    assert_eq!(
        original.get_object(id).unwrap(),
        result.get_object(id).unwrap()
    );
}

#[test]
fn preserves_invisible_ocr_layer_when_compressing_scan_images() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("ocr.pdf");
    let (mut doc, image_id) = image_document();
    let page = doc.get_pages()[&1];
    doc.add_page_contents(
        page,
        b"q BT /F1 9 Tf 3 Tr 72 210 Td (Invisible OCR searchable layer) Tj ET Q".to_vec(),
    )
    .unwrap();
    doc.save(&input).unwrap();
    let original = Document::load(&input).unwrap();
    let text = original.extract_text(&[1]).unwrap();
    assert!(text.contains("Invisible OCR searchable layer"));
    for (level, width) in [
        (CompressionLevel::Normal, 150),
        (CompressionLevel::Ultra, 96),
        (CompressionLevel::Maximum, 72),
    ] {
        let output = dir.path().join(format!("ocr-{level:?}.pdf"));
        let report = optimize(
            &input,
            &output,
            &OptimizeOptions {
                level,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(report.optimized_images, 1);
        let result = Document::load(output).unwrap();
        assert_eq!(result.extract_text(&[1]).unwrap(), text);
        assert_eq!(
            result.get_page_content(page),
            original.get_page_content(page)
        );
        assert_eq!(
            result
                .get_object(image_id)
                .unwrap()
                .as_stream()
                .unwrap()
                .dict
                .get(b"Width")
                .unwrap()
                .as_i64()
                .unwrap(),
            width
        );
    }
}

#[test]
fn normal_resizes_to_effective_150_dpi_and_preserves_searchable_text() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let (mut doc, id) = image_document();
    doc.save(&input).unwrap();
    let before = fs::read(&input).unwrap();
    let report = optimize(
        &input,
        &output,
        &OptimizeOptions {
            level: CompressionLevel::Normal,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(report.outcome, Outcome::Reduced);
    assert_eq!(report.optimized_images, 1);
    assert!(report.output_bytes < report.input_bytes);
    let result = Document::load(output).unwrap();
    let image = result.get_object(id).unwrap().as_stream().unwrap();
    assert_eq!(image.dict.get(b"Width").unwrap().as_i64().unwrap(), 150);
    assert_eq!(image.dict.get(b"Height").unwrap().as_i64().unwrap(), 75);
    assert_eq!(
        image.dict.get(b"Filter").unwrap().as_name().unwrap(),
        b"DCTDecode"
    );
    assert_eq!(
        doc.extract_text(&[1]).unwrap(),
        result.extract_text(&[1]).unwrap()
    );
    assert_eq!(fs::read(input).unwrap(), before);
}

#[test]
fn ultra_and_maximum_use_their_dpi_targets_while_light_preserves_image_bytes() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let (mut doc, id) = image_document();
    doc.save(&input).unwrap();
    let original = Document::load(&input).unwrap();
    for (level, width) in [
        (CompressionLevel::Light, 600),
        (CompressionLevel::Ultra, 96),
        (CompressionLevel::Maximum, 72),
    ] {
        let output = dir.path().join(format!("{level:?}.pdf"));
        let report = optimize(
            &input,
            &output,
            &OptimizeOptions {
                level,
                ..Default::default()
            },
        )
        .unwrap();
        let result = Document::load(output).unwrap();
        let image = result.get_object(id).unwrap().as_stream().unwrap();
        assert_eq!(image.dict.get(b"Width").unwrap().as_i64().unwrap(), width);
        if level == CompressionLevel::Light {
            assert_eq!(report.optimized_images, 0);
            assert_eq!(
                original.get_object(id).unwrap(),
                result.get_object(id).unwrap()
            );
        }
    }
}

#[test]
fn shared_image_accounts_for_its_largest_placement() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let (mut doc, id) = image_document();
    let page = doc.get_pages()[&1];
    doc.add_page_contents(page, b"q 288 0 0 144 0 0 cm /Im1 Do Q".to_vec())
        .unwrap();
    doc.save(&input).unwrap();
    optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    let result = Document::load(output).unwrap();
    assert_eq!(
        result
            .get_object(id)
            .unwrap()
            .as_stream()
            .unwrap()
            .dict
            .get(b"Width")
            .unwrap()
            .as_i64()
            .unwrap(),
        600
    );
}

#[test]
fn nested_forms_and_form_matrices_are_included_in_effective_dpi() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let (mut doc, id) = image_document();
    let form = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Form",
            "BBox" => vec![0.into(), 0.into(), 72.into(), 36.into()],
            "Matrix" => vec![2.into(), 0.into(), 0.into(), 2.into(), 0.into(), 0.into()],
            "Resources" => dictionary! {"XObject" => dictionary! {"Im1" => id}},
        },
        b"q 72 0 0 36 0 0 cm /Im1 Do Q".to_vec(),
    ));
    let page = doc.get_pages()[&1];
    let resources = doc
        .get_dictionary(page)
        .unwrap()
        .get(b"Resources")
        .unwrap()
        .as_reference()
        .unwrap();
    doc.get_dictionary_mut(resources)
        .unwrap()
        .set("XObject", dictionary! {"Fm" => form, "Im1" => id});
    doc.add_page_contents(page, b"q /Fm Do Q".to_vec()).unwrap();
    doc.save(&input).unwrap();
    optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    let result = Document::load(output).unwrap();
    assert_eq!(
        result
            .get_object(id)
            .unwrap()
            .as_stream()
            .unwrap()
            .dict
            .get(b"Width")
            .unwrap()
            .as_i64()
            .unwrap(),
        300
    );
}

#[test]
fn masks_cmyk_and_resource_limits_preserve_image_objects() {
    for kind in ["mask", "cmyk", "limit"] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("output.pdf");
        let (mut doc, id) = image_document();
        let image = doc.get_object_mut(id).unwrap().as_stream_mut().unwrap();
        match kind {
            "mask" => image.dict.set(
                "Mask",
                vec![0.into(), 0.into(), 0.into(), 0.into(), 0.into(), 0.into()],
            ),
            "cmyk" => image.dict.set("ColorSpace", "DeviceCMYK"),
            _ => {}
        }
        doc.save(&input).unwrap();
        let original = Document::load(&input).unwrap();
        let options = OptimizeOptions {
            level: CompressionLevel::Maximum,
            max_image_pixels: if kind == "limit" { 100 } else { 32_000_000 },
            ..Default::default()
        };
        let report = optimize(&input, &output, &options).unwrap();
        let result = Document::load(output).unwrap();
        assert_eq!(report.optimized_images, 0);
        assert_eq!(
            original.get_object(id).unwrap(),
            result.get_object(id).unwrap()
        );
    }
}
