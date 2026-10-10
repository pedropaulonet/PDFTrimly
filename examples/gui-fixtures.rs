//! Synthetic, project-owned documents for the native GUI checklist.
//! Refuses an existing directory so previous checks and outputs remain intact.
#[path = "../crates/pdftrimly-core/tests/common/mod.rs"]
mod common;
use lopdf::{dictionary, Object};
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let root = arguments
        .next()
        .map(PathBuf::from)
        .ok_or("Usage: cargo run --example gui-fixtures -- /tmp/pdftrimly-gui-check-NEW")?;
    if arguments.next().is_some() {
        return Err("Expected exactly one new directory".into());
    }
    fs::create_dir(&root)?;
    let inputs = root.join("inputs");
    fs::create_dir(&inputs)?;
    fs::create_dir(root.join("outputs"))?;
    common::document(10000, false).save(inputs.join(
        "Relatório sintético com espaços, acentos e nome longo para conferir a interface.PDF",
    ))?;
    common::document(1, true).save(inputs.join("already-small.pdf"))?;
    let mut signed = common::document(100, false);
    signed.catalog_mut()?.set(
        "AcroForm",
        dictionary! {"Fields" => vec![Object::Dictionary(dictionary! {"FT" => "Sig"})]},
    );
    signed.save(inputs.join("signature-field.pdf"))?;
    fs::write(inputs.join("invalid.pdf"), b"Synthetic invalid PDF")?;
    fs::write(inputs.join("unsupported.txt"), b"Synthetic non-PDF file")?;
    fs::create_dir(inputs.join("directory.pdf"))?;
    println!("Synthetic inputs: {}", inputs.display());
    println!("Output directory: {}", root.join("outputs").display());
    println!("Signature fixture contains only a signature field, not a real signature.");
    Ok(())
}
