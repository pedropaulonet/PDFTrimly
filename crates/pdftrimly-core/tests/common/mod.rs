use lopdf::{dictionary, Document, Object, Stream};

// Project-owned fixtures. Content includes searchable text and vector graphics.
pub fn document(repetitions: usize, compressed: bool) -> Document {
    let mut doc = Document::with_version("1.4");
    let pages = doc.new_object_id();
    let font = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let resources = doc.add_object(dictionary! {"Font" => dictionary! {"F1" => font}});
    let data = b"BT /F1 12 Tf 72 720 Td (PDFTrimly searchable text) Tj ET\n0 0 100 100 re S\n"
        .repeat(repetitions);
    let mut stream = Stream::new(dictionary! {}, data);
    if compressed {
        stream.compress().unwrap();
    }
    let contents = doc.add_object(stream);
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages, "Resources" => resources,
        "Contents" => contents, "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    doc.objects.insert(
        pages,
        dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }
        .into(),
    );
    let catalog = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
    doc.trailer.set("Root", catalog);
    doc
}

#[allow(dead_code)] // This shared fixture module is also compiled by CLI/image tests.
pub fn raw_pdf(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut pdf = b"%PDF-1.4\n%\x80\x81\x82\x83\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Root 1 0 R /Size {} >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}
