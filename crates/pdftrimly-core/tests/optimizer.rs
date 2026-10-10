mod common;

use std::fs;

use lopdf::{dictionary, Document, Object, Stream};
use pdftrimly_core::{optimize, OptimizeError, OptimizeOptions, Outcome};
use tempfile::tempdir;

#[test]
fn accepts_exact_numeric_equivalence_after_pdf_serialization() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("decimal-opacity.pdf");
    let output = dir.path().join("output.pdf");
    let content =
        b"q /gs0 gs BT /F1 12 Tf 72 720 Td (Keep opacity and text) Tj ET Q\n".repeat(1000);
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"endstream");
    let bytes = common::raw_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 4 0 R /Resources 5 0 R /MediaBox [0 0 595 842] >>"
            .to_vec(),
        stream,
        b"<< /ExtGState << /gs0 << /CA 1.0 /ca 1.0 >> >> /Font << /F1 6 0 R >> >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ]);
    fs::write(&input, &bytes).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Reduced);
    let after = Document::load(output).unwrap();
    assert_eq!(
        after
            .get_dictionary((5, 0))
            .unwrap()
            .get(b"ExtGState")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"gs0")
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"ca")
            .unwrap()
            .as_float()
            .unwrap(),
        1.0
    );
}

#[test]
fn preserves_multiple_pages_sharing_a_content_stream() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("multipage.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(2000, false);
    let first = doc.get_pages()[&1];
    let page = doc.get_object(first).unwrap().clone();
    let second = doc.add_object(page);
    let pages = doc
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let tree = doc.get_dictionary_mut(pages).unwrap();
    tree.set(
        "Kids",
        vec![Object::Reference(first), Object::Reference(second)],
    );
    tree.set("Count", 2);
    doc.save(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.pages, 2);
    assert_eq!(report.outcome, Outcome::Reduced);
    let after = Document::load(output).unwrap();
    assert_eq!(
        after.extract_text(&[1]).unwrap(),
        after.extract_text(&[2]).unwrap()
    );
}

#[test]
fn enforces_combined_decoded_budget_across_streams() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(100, false);
    let extra = doc.add_object(Stream::new(dictionary! {}, vec![42; 8000]));
    doc.catalog_mut().unwrap().set("FixtureStream", extra);
    doc.save(&input).unwrap();
    let options = OptimizeOptions {
        max_total_decoded_bytes: 10000,
        ..Default::default()
    };
    assert!(matches!(
        optimize(&input, &output, &options),
        Err(OptimizeError::LimitExceeded(_))
    ));
    assert!(!output.exists());
}

#[test]
fn copies_original_when_rewriting_modern_storage_would_increase_size() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("modern.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(3, false);
    let records: Vec<Object> = (0..1000)
        .map(|_| Object::Reference(doc.add_object(dictionary! {"FixtureValue" => 42})))
        .collect();
    doc.catalog_mut().unwrap().set("FixtureRecords", records);
    let mut bytes = Vec::new();
    doc.save_modern(&mut bytes).unwrap();
    fs::write(&input, &bytes).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Unchanged);
    assert_eq!(report.optimized_streams, 0);
    assert_eq!(fs::read(output).unwrap(), bytes);
}

#[test]
fn rejects_encryption_with_and_without_a_password() {
    use lopdf::{EncryptionState, EncryptionVersion, Permissions};
    for password in ["", "secret"] {
        let dir = tempdir().unwrap();
        let input = dir.path().join("encrypted.pdf");
        let output = dir.path().join("output.pdf");
        let mut doc = common::document(100, false);
        doc.trailer.set(
            "ID",
            vec![
                Object::string_literal(b"fixture-id-00001".to_vec()),
                Object::string_literal(b"fixture-id-00001".to_vec()),
            ],
        );
        let state = EncryptionState::try_from(EncryptionVersion::V1 {
            document: &doc,
            owner_password: "owner",
            user_password: password,
            permissions: Permissions::all(),
        })
        .unwrap();
        doc.encrypt(&state).unwrap();
        doc.save(&input).unwrap();
        assert!(matches!(
            optimize(&input, &output, &OptimizeOptions::default()),
            Err(OptimizeError::Encrypted)
        ));
        assert!(!output.exists());
    }
}

#[test]
fn rejects_truncated_flate_instead_of_silently_accepting_missing_checksum() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("truncated.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(2000, true);
    for object in doc.objects.values_mut() {
        if let Object::Stream(stream) = object {
            let mut bytes = stream.content.clone();
            bytes.truncate(bytes.len() - 4);
            stream.set_content(bytes);
        }
    }
    doc.save(&input).unwrap();
    assert!(optimize(&input, &output, &OptimizeOptions::default()).is_err());
    assert!(!output.exists());
}

#[test]
fn recompresses_simple_flate_without_changing_decoded_bytes() {
    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(2000, false);
    for object in doc.objects.values_mut() {
        if let Object::Stream(stream) = object {
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::none());
            encoder.write_all(&stream.content).unwrap();
            stream.set_content(encoder.finish().unwrap());
            stream.dict.set("Filter", "FlateDecode");
        }
    }
    doc.save(&input).unwrap();
    let before = Document::load(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Reduced);
    let after = Document::load(&output).unwrap();
    assert_eq!(
        before.get_page_content(before.get_pages()[&1]),
        after.get_page_content(after.get_pages()[&1])
    );
}

#[test]
fn preserves_unsupported_filter_and_predictor_streams() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(2000, false);
    let unsupported = doc.add_object(Stream::new(
        dictionary! {"Filter" => "RunLengthDecode"},
        vec![128],
    ));
    let predictor = doc.add_object(Stream::new(
        dictionary! {
            "Filter" => "FlateDecode", "DecodeParms" => dictionary! {"Predictor" => 12},
        },
        vec![1, 2, 3],
    ));
    doc.catalog_mut().unwrap().set(
        "FixtureStreams",
        vec![Object::Reference(unsupported), Object::Reference(predictor)],
    );
    doc.save(&input).unwrap();
    let before = Document::load(&input).unwrap();
    optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    let after = Document::load(&output).unwrap();
    for id in [unsupported, predictor] {
        assert_eq!(
            before.get_object(id).unwrap(),
            after.get_object(id).unwrap()
        );
    }
}

#[test]
fn rejects_cyclic_page_tree_and_does_not_leave_temporary_files() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(100, false);
    let pages = doc
        .catalog()
        .unwrap()
        .get(b"Pages")
        .unwrap()
        .as_reference()
        .unwrap();
    let dict = doc.get_dictionary_mut(pages).unwrap();
    dict.set("Kids", vec![Object::Reference(pages)]);
    dict.set("Parent", pages);
    doc.save(&input).unwrap();
    assert!(optimize(&input, &output, &OptimizeOptions::default()).is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn reduces_real_pdf_and_preserves_text_vectors_and_original() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    common::document(2000, false).save(&input).unwrap();
    let original = fs::read(&input).unwrap();
    let before = Document::load(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    let after = Document::load(&output).unwrap();
    assert_eq!(report.outcome, Outcome::Reduced);
    assert!(report.output_bytes < report.input_bytes);
    assert!(report.savings_percent() > 0.0);
    assert_eq!(report.pages, 1);
    assert!(report.optimized_streams > 0);
    assert_eq!(fs::read(&input).unwrap(), original);
    assert_eq!(
        before.extract_text(&[1]).unwrap(),
        after.extract_text(&[1]).unwrap()
    );
    let before_page = before.get_pages()[&1];
    let after_page = after.get_pages()[&1];
    assert_eq!(
        before.get_page_content(before_page),
        after.get_page_content(after_page)
    );
    assert_eq!(report.output_bytes, fs::metadata(&output).unwrap().len());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn no_op_copies_original_exactly_and_reports_no_savings() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    common::document(0, false).save(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    assert_eq!(report.outcome, Outcome::Unchanged);
    assert_eq!(report.input_bytes, report.output_bytes);
    assert_eq!(report.savings_percent(), 0.0);
    assert_eq!(fs::read(input).unwrap(), fs::read(output).unwrap());
}

#[test]
fn never_overwrites_existing_output_or_input() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("existing.pdf");
    common::document(100, false).save(&input).unwrap();
    fs::write(&output, b"keep existing file").unwrap();
    let original = fs::read(&input).unwrap();
    for destination in [&input, &output] {
        assert!(matches!(
            optimize(&input, destination, &OptimizeOptions::default()),
            Err(OptimizeError::OutputExists)
        ));
    }
    assert_eq!(fs::read(input).unwrap(), original);
    assert_eq!(fs::read(output).unwrap(), b"keep existing file");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn rejects_signature_fields_in_nested_catalog_dictionary() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("signed.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(100, false);
    doc.catalog_mut().unwrap().set(
        "AcroForm",
        dictionary! {
            "Fields" => vec![Object::Dictionary(dictionary! {"FT" => "Sig"})],
        },
    );
    doc.save(&input).unwrap();
    assert!(matches!(
        optimize(&input, &output, &OptimizeOptions::default()),
        Err(OptimizeError::Signed)
    ));
    assert!(!output.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn rejects_invalid_pdf_and_dangling_references_without_outputs() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    fs::write(&input, b"not a PDF").unwrap();
    assert!(optimize(&input, &output, &OptimizeOptions::default()).is_err());
    let mut doc = common::document(100, false);
    doc.catalog_mut()
        .unwrap()
        .set("Metadata", Object::Reference((9999, 0)));
    doc.save(&input).unwrap();
    assert!(optimize(&input, &output, &OptimizeOptions::default()).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn respects_input_and_decoded_stream_limits() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    let output = dir.path().join("output.pdf");
    common::document(2000, true).save(&input).unwrap();
    let options = OptimizeOptions {
        max_input_bytes: 100,
        ..Default::default()
    };
    assert!(matches!(
        optimize(&input, &output, &options),
        Err(OptimizeError::LimitExceeded(_))
    ));
    let options = OptimizeOptions {
        max_stream_bytes: 100,
        ..Default::default()
    };
    assert!(matches!(
        optimize(&input, &output, &options),
        Err(OptimizeError::LimitExceeded(_))
    ));
    assert!(!output.exists());
}

#[test]
fn preserves_images_annotations_metadata_and_form_fields() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("mixed.pdf");
    let output = dir.path().join("output.pdf");
    let mut doc = common::document(2000, false);
    let image = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image", "Width" => 10,
            "Height" => 10, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        vec![128; 300],
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
    let annotation = doc.add_object(dictionary! {
        "Type" => "Annot", "Subtype" => "Link", "Rect" => vec![0.into(), 0.into(), 20.into(), 20.into()],
        "A" => dictionary! {"S" => "URI", "URI" => Object::string_literal("https://example.org")},
    });
    doc.get_dictionary_mut(page)
        .unwrap()
        .set("Annots", vec![Object::Reference(annotation)]);
    let info = doc.add_object(dictionary! {"Title" => Object::string_literal("Keep metadata")});
    doc.trailer.set("Info", info);
    doc.catalog_mut().unwrap().set("AcroForm", dictionary! {
        "Fields" => vec![Object::Dictionary(dictionary! {"FT" => "Tx", "T" => Object::string_literal("name")})],
    });
    doc.save(&input).unwrap();
    let before = Document::load(&input).unwrap();
    let report = optimize(&input, &output, &OptimizeOptions::default()).unwrap();
    let after = Document::load(&output).unwrap();
    assert_eq!(report.outcome, Outcome::Reduced);
    for id in [image, annotation, info] {
        assert_eq!(
            before.get_object(id).unwrap(),
            after.get_object(id).unwrap()
        );
    }
    assert_eq!(
        before.catalog().unwrap().get(b"AcroForm").unwrap(),
        after.catalog().unwrap().get(b"AcroForm").unwrap()
    );
}
