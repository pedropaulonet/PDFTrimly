use std::collections::HashSet;

use lopdf::{Dictionary, Document, Object, ObjectId};

use crate::OptimizeError;

fn name_is(dict: &Dictionary, key: &[u8], name: &[u8]) -> bool {
    dict.get(key).and_then(Object::as_name).ok() == Some(name)
}

fn storage_object(object: &Object) -> bool {
    object
        .type_name()
        .ok()
        .is_some_and(|name| name == b"XRef" || name == b"ObjStm")
}

fn inspect(object: &Object, document: &Document, depth: usize) -> Result<(), OptimizeError> {
    if depth > 64 {
        return Err(OptimizeError::LimitExceeded("profundidade de objetos"));
    }
    match object {
        Object::Reference(id) if !document.objects.contains_key(id) => {
            return Err(OptimizeError::Structure("referência a objeto inexistente"));
        }
        Object::Array(array) => {
            for value in array {
                inspect(value, document, depth + 1)?;
            }
        }
        Object::Dictionary(dict) => inspect_dictionary(dict, document, depth)?,
        Object::Stream(stream) => inspect_dictionary(&stream.dict, document, depth)?,
        _ => {}
    }
    Ok(())
}

fn inspect_dictionary(
    dict: &Dictionary,
    document: &Document,
    depth: usize,
) -> Result<(), OptimizeError> {
    if name_is(dict, b"Type", b"Sig") || name_is(dict, b"FT", b"Sig") || dict.has(b"ByteRange") {
        return Err(OptimizeError::Signed);
    }
    if dict.has(b"Linearized") {
        return Err(OptimizeError::Structure(
            "PDF linearizado ainda não suportado",
        ));
    }
    for (_, value) in dict.iter() {
        inspect(value, document, depth + 1)?;
    }
    Ok(())
}

// Strictly walk the page tree instead of accepting a partially recovered count.
fn page_count(
    document: &Document,
    id: ObjectId,
    seen: &mut HashSet<ObjectId>,
    depth: usize,
) -> Result<usize, OptimizeError> {
    if depth > 64 || !seen.insert(id) {
        return Err(OptimizeError::Structure(
            "árvore de páginas cíclica ou profunda",
        ));
    }
    let dict = document.get_dictionary(id).map_err(OptimizeError::Pdf)?;
    if name_is(dict, b"Type", b"Page") {
        return Ok(1);
    }
    if !name_is(dict, b"Type", b"Pages") {
        return Err(OptimizeError::Structure(
            "tipo inválido na árvore de páginas",
        ));
    }
    let kids = dict
        .get(b"Kids")
        .and_then(Object::as_array)
        .map_err(OptimizeError::Pdf)?;
    let mut count = 0usize;
    for kid in kids {
        let kid = kid.as_reference().map_err(OptimizeError::Pdf)?;
        let parent = document
            .get_dictionary(kid)
            .and_then(|dict| dict.get(b"Parent"))
            .and_then(Object::as_reference)
            .map_err(OptimizeError::Pdf)?;
        if parent != id {
            return Err(OptimizeError::Structure("Parent de página inconsistente"));
        }
        count += page_count(document, kid, seen, depth + 1)?;
    }
    if dict
        .get(b"Count")
        .and_then(Object::as_i64)
        .map_err(OptimizeError::Pdf)?
        != count as i64
    {
        return Err(OptimizeError::Structure(
            "contagem de páginas inconsistente",
        ));
    }
    Ok(count)
}

pub(crate) fn validate_input(document: &Document) -> Result<usize, OptimizeError> {
    if document.is_encrypted() || document.was_encrypted() || document.trailer.has(b"Encrypt") {
        return Err(OptimizeError::Encrypted);
    }
    if document.objects.len() > 100_000 || document.max_id > 1_000_000 {
        return Err(OptimizeError::LimitExceeded(
            "quantidade ou identificadores de objetos",
        ));
    }
    for object in document.objects.values() {
        inspect(object, document, 0)?;
    }
    for (_, object) in document.trailer.iter() {
        inspect(object, document, 0)?;
    }
    let catalog = document.catalog().map_err(OptimizeError::Pdf)?;
    if !name_is(catalog, b"Type", b"Catalog") {
        return Err(OptimizeError::Structure("catálogo inválido"));
    }
    let root = catalog
        .get(b"Pages")
        .and_then(Object::as_reference)
        .map_err(OptimizeError::Pdf)?;
    let count = page_count(document, root, &mut HashSet::new(), 0)?;
    if count == 0 {
        return Err(OptimizeError::Structure("documento sem páginas"));
    }
    Ok(count)
}

pub(crate) fn prepare_for_save(document: &mut Document) {
    document.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    document.objects.retain(|_, object| !storage_object(object));
    for key in [
        b"Prev".as_slice(),
        b"XRefStm",
        b"Size",
        b"Type",
        b"W",
        b"Index",
        b"Length",
        b"Filter",
        b"DecodeParms",
    ] {
        document.trailer.remove(key);
    }
}

pub(crate) fn validate_output(
    expected: &Document,
    actual: &Document,
    pages: usize,
) -> Result<(), OptimizeError> {
    if validate_input(actual)? != pages
        || expected.objects.len() != actual.objects.len()
        || expected.objects.iter().any(|(id, object)| {
            actual
                .objects
                .get(id)
                .is_none_or(|other| !equivalent(object, other, 0))
        })
    {
        return Err(OptimizeError::Structure(
            "objetos ou páginas mudaram após gravação",
        ));
    }
    for (key, value) in expected.trailer.iter() {
        if key != b"Size"
            && actual
                .trailer
                .get(key)
                .ok()
                .is_none_or(|other| !equivalent(value, other, 0))
        {
            return Err(OptimizeError::Structure("o trailer mudou após gravação"));
        }
    }
    Ok(())
}

fn equivalent_dictionary(a: &Dictionary, b: &Dictionary, depth: usize) -> bool {
    a.len() == b.len()
        && a.iter().all(|(key, value)| {
            b.get(key)
                .ok()
                .is_some_and(|other| equivalent(value, other, depth + 1))
        })
}

fn equivalent(a: &Object, b: &Object, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match (a, b) {
        (Object::Real(real), Object::Integer(integer))
        | (Object::Integer(integer), Object::Real(real)) => {
            // Every integer in this range is exactly representable as f64.
            // Compare exactly; never use a tolerance that could hide alterations.
            integer.unsigned_abs() <= (1u64 << 53) && f64::from(*real) == *integer as f64
        }
        (Object::Array(a), Object::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equivalent(a, b, depth + 1))
        }
        (Object::Dictionary(a), Object::Dictionary(b)) => equivalent_dictionary(a, b, depth),
        (Object::Stream(a), Object::Stream(b)) => {
            a.content == b.content && equivalent_dictionary(&a.dict, &b.dict, depth)
        }
        (Object::String(a, _), Object::String(b, _)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::equivalent;
    use lopdf::Object;

    #[test]
    fn numeric_validation_never_accepts_a_changed_value() {
        assert!(equivalent(&Object::Real(1.0), &Object::Integer(1), 0));
        assert!(!equivalent(&Object::Real(1.001), &Object::Integer(1), 0));
        assert!(!equivalent(&Object::Real(1.0), &Object::Integer(2), 0));
        assert!(!equivalent(
            &Object::Real(9007199254740992.0),
            &Object::Integer(9007199254740993),
            0
        ));
    }
}
