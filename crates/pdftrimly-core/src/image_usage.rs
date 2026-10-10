//! Conservative effective image sizing through page and Form XObject transforms.
use std::collections::{HashMap, HashSet};

use lopdf::{content::Content, Dictionary, Document, Object, ObjectId};

use crate::OptimizeOptions;

#[derive(Clone, Copy)]
struct Matrix([f64; 6]);

impl Matrix {
    const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn from_objects(objects: &[Object]) -> Option<Self> {
        if objects.len() != 6 {
            return None;
        }
        let mut values = [0.0; 6];
        for (value, object) in values.iter_mut().zip(objects) {
            *value = f64::from(object.as_float().ok()?);
            if !value.is_finite() {
                return None;
            }
        }
        Some(Self(values))
    }

    fn concatenate(self, other: Self) -> Option<Self> {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = other.0;
        let result = Self([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ]);
        result
            .0
            .iter()
            .all(|value| value.is_finite())
            .then_some(result)
    }

    fn size(self) -> (f64, f64) {
        (self.0[0].hypot(self.0[1]), self.0[2].hypot(self.0[3]))
    }
}

pub(crate) struct Usage {
    pub sizes: HashMap<ObjectId, (f64, f64)>,
    pub blocked: HashSet<ObjectId>,
}

fn resolve<'a>(doc: &'a Document, object: &'a Object) -> Option<&'a Object> {
    doc.dereference(object).ok().map(|(_, object)| object)
}

fn dictionary<'a>(doc: &'a Document, object: &'a Object) -> Option<&'a Dictionary> {
    resolve(doc, object)?.as_dict().ok()
}

fn page_resources(doc: &Document, page: ObjectId) -> Option<&Dictionary> {
    let mut id = page;
    for _ in 0..64 {
        let node = doc.get_dictionary(id).ok()?;
        if let Ok(resources) = node.get(b"Resources") {
            return dictionary(doc, resources);
        }
        id = node.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

struct Analyzer<'a> {
    doc: &'a Document,
    options: &'a OptimizeOptions,
    sizes: HashMap<ObjectId, (f64, f64)>,
    form_stack: HashSet<ObjectId>,
    remaining_operations: usize,
    remaining_content_bytes: usize,
}

impl Analyzer<'_> {
    fn walk(
        &mut self,
        bytes: &[u8],
        resources: &Dictionary,
        initial: Matrix,
        depth: usize,
    ) -> Option<()> {
        if depth > 32 || bytes.len() > self.remaining_content_bytes {
            return None;
        }
        self.remaining_content_bytes -= bytes.len();
        // Lenient parsing could miss placements; only fully parsed content is used.
        let content = Content::decode_strict(bytes).ok()?;
        if content.operations.len() > self.remaining_operations {
            return None;
        }
        self.remaining_operations -= content.operations.len();
        let mut matrix = initial;
        let mut stack = Vec::new();
        for operation in content.operations {
            match operation.operator.as_str() {
                "q" => {
                    if stack.len() >= 64 {
                        return None;
                    }
                    stack.push(matrix);
                }
                "Q" => matrix = stack.pop()?,
                "cm" => matrix = matrix.concatenate(Matrix::from_objects(&operation.operands)?)?,
                "Do" => {
                    let name = operation.operands.first()?.as_name().ok()?;
                    let xobjects = dictionary(self.doc, resources.get(b"XObject").ok()?)?;
                    let reference = xobjects.get(name).ok()?;
                    let (id, object) = self.doc.dereference(reference).ok()?;
                    let id = id?;
                    let stream = object.as_stream().ok()?;
                    match stream.dict.get(b"Subtype").and_then(Object::as_name).ok()? {
                        b"Image" => {
                            let (width, height) = matrix.size();
                            if width <= 0.0 || height <= 0.0 {
                                return None;
                            }
                            let entry = self.sizes.entry(id).or_insert((0.0, 0.0));
                            entry.0 = entry.0.max(width);
                            entry.1 = entry.1.max(height);
                        }
                        b"Form" => {
                            if !self.form_stack.insert(id) {
                                return None;
                            }
                            let transform = match stream.dict.get(b"Matrix") {
                                Ok(value) => Matrix::from_objects(
                                    resolve(self.doc, value)?.as_array().ok()?,
                                )?,
                                Err(_) => Matrix::IDENTITY,
                            };
                            let form_resources = match stream.dict.get(b"Resources") {
                                Ok(value) => dictionary(self.doc, value)?,
                                Err(_) => resources,
                            };
                            let bytes = stream
                                .get_plain_content_with_limit(self.options.max_stream_bytes)
                                .ok()?;
                            self.walk(
                                &bytes,
                                form_resources,
                                matrix.concatenate(transform)?,
                                depth + 1,
                            )?;
                            self.form_stack.remove(&id);
                        }
                        _ => return None,
                    }
                }
                _ => {}
            }
        }
        if !stack.is_empty() {
            return None;
        }
        Some(())
    }
}

// Images reachable from patterns, annotation appearances, thumbnails or masks
// cannot be resized based solely on the page/Form placements analyzed above.
fn blocked_images(doc: &Document) -> HashSet<ObjectId> {
    let sensitive = [
        b"AP".as_slice(),
        b"Pattern",
        b"Thumb",
        b"SMask",
        b"Mask",
        b"Alternates",
        b"OPI",
    ];
    let mut roots = Vec::new();
    let mut inspect: Vec<_> = doc.objects.values().collect();
    while let Some(object) = inspect.pop() {
        let dict = match object {
            Object::Dictionary(dict) => Some(dict),
            Object::Stream(stream) => Some(&stream.dict),
            Object::Array(array) => {
                inspect.extend(array);
                None
            }
            _ => None,
        };
        if let Some(dict) = dict {
            for (key, value) in dict.iter() {
                if sensitive.contains(&key.as_slice()) {
                    roots.push(value);
                }
                if !matches!(value, Object::Reference(_)) {
                    inspect.push(value);
                }
            }
        }
    }
    let mut seen = HashSet::new();
    let mut blocked = HashSet::new();
    while let Some(object) = roots.pop() {
        match object {
            Object::Reference(id) if seen.insert(*id) => {
                if let Some(target) = doc.objects.get(id) {
                    if target.as_stream().ok().is_some_and(|stream| {
                        stream.dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Image")
                    }) {
                        blocked.insert(*id);
                    }
                    roots.push(target);
                }
            }
            Object::Dictionary(dict) => roots.extend(dict.iter().map(|(_, value)| value)),
            Object::Stream(stream) => roots.extend(stream.dict.iter().map(|(_, value)| value)),
            Object::Array(array) => roots.extend(array),
            _ => {}
        }
    }
    blocked
}

pub(crate) fn analyze(doc: &Document, options: &OptimizeOptions) -> Option<Usage> {
    let mut analyzer = Analyzer {
        doc,
        options,
        sizes: HashMap::new(),
        form_stack: HashSet::new(),
        remaining_operations: 1_000_000,
        remaining_content_bytes: options.max_total_decoded_bytes,
    };
    for page in doc.get_pages().values() {
        let bytes = doc
            .get_page_content_with_limit(*page, options.max_stream_bytes)
            .ok()?;
        if bytes.is_empty() {
            continue;
        }
        let resources = page_resources(doc, *page)?;
        let user_unit = match doc.get_dictionary(*page).ok()?.get(b"UserUnit") {
            Ok(value) => f64::from(value.as_float().ok()?),
            Err(_) => 1.0,
        };
        if !user_unit.is_finite() || user_unit <= 0.0 {
            return None;
        }
        analyzer.walk(
            &bytes,
            resources,
            Matrix([user_unit, 0.0, 0.0, user_unit, 0.0, 0.0]),
            0,
        )?;
    }
    Some(Usage {
        sizes: analyzer.sizes,
        blocked: blocked_images(doc),
    })
}
