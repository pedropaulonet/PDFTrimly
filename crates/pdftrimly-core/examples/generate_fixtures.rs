#[path = "../tests/common/mod.rs"]
mod common;

use std::{
    env,
    fs::{self, OpenOptions},
    path::PathBuf,
};

use lopdf::{dictionary, Document, Object, Stream};

fn save(mut doc: Document, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    doc.save_to(&mut file)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let directory = PathBuf::from(
        args.next()
            .ok_or("Usage: generate_fixtures <new-directory>")?,
    );
    if args.next().is_some() {
        return Err("Only one directory is accepted".into());
    }
    fs::create_dir(&directory)?;
    save(common::document(2000, false), directory.join("text.pdf"))?;
    save(
        common::document(2000, true),
        directory.join("already-compressed.pdf"),
    )?;

    let mut vectors = common::document(0, false);
    for object in vectors.objects.values_mut() {
        if let Object::Stream(stream) = object {
            stream.set_content(b"0 0 100 100 re S\n10 10 m 200 200 l S\n".repeat(4000));
        }
    }
    save(vectors, directory.join("vectors.pdf"))?;

    for (name, repetitions) in [("synthetic-scan.pdf", 0), ("mixed.pdf", 1000)] {
        let mut doc = common::document(repetitions, false);
        // Synthetic RGB image with a checker pattern; not a real scanned document.
        let pixels: Vec<u8> = (0..256 * 256)
            .flat_map(|index| {
                let value = if (index / 256 / 16 + index % 256 / 16) % 2 == 0 {
                    240
                } else {
                    32
                };
                [value, value, value]
            })
            .collect();
        let image = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 256,
                "Height" => 256, "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
            },
            pixels,
        ));
        let page = doc.get_pages()[&1];
        let resources = doc
            .get_dictionary(page)?
            .get(b"Resources")?
            .as_reference()?;
        doc.get_dictionary_mut(resources)?
            .set("XObject", dictionary! {"Im1" => image});
        let contents = doc.get_page_contents(page)[0];
        let stream = doc.get_object_mut(contents)?.as_stream_mut()?;
        let mut content = stream.content.clone();
        content.extend_from_slice(b"q 256 0 0 256 72 200 cm /Im1 Do Q\n");
        stream.set_content(content);
        save(doc, directory.join(name))?;
    }
    println!("Created five project-owned PDF fixtures.");
    Ok(())
}
