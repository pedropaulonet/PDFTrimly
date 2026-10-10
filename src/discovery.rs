//! Deterministic local discovery. Directory symlinks are never traversed.
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug)]
pub struct Input {
    pub path: PathBuf,
    pub bytes: u64,
    pub root: Option<PathBuf>,
}
#[derive(Default)]
pub struct Report {
    pub inputs: Vec<Input>,
    pub skipped: usize,
    pub duplicates: usize,
    pub errors: usize,
}

// Resolve existing ancestors too, so a not-yet-created output beneath a symlink
// is excluded by its eventual canonical location.
pub fn location(path: &Path) -> PathBuf {
    if !path.is_absolute() {
        if let Ok(current) = std::env::current_dir() {
            return location(&current.join(path));
        }
    }
    if let Ok(path) = path.canonicalize() {
        return path;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        if parent != path {
            return location(parent).join(name);
        }
    }
    path.to_path_buf()
}
pub fn scan(paths: Vec<PathBuf>, destination: Option<PathBuf>, recursive: bool) -> Report {
    let destination = destination.as_deref().map(location);
    let mut report = Report::default();
    let mut seen = HashSet::new();
    for path in paths {
        let Ok(meta) = path.metadata() else {
            report.errors += 1;
            continue;
        };
        if meta.is_dir() && fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
            report.skipped += 1;
            continue;
        }
        if meta.is_dir() {
            let root = location(&path);
            let mut pending = vec![root.clone()];
            while let Some(dir) = pending.pop() {
                if destination
                    .as_ref()
                    .is_some_and(|output| dir.starts_with(output))
                {
                    report.skipped += 1;
                    continue;
                }
                let Ok(reader) = fs::read_dir(&dir) else {
                    report.errors += 1;
                    continue;
                };
                let mut children = Vec::new();
                for entry in reader {
                    match entry {
                        Ok(entry) => children.push(entry.path()),
                        Err(_) => report.errors += 1,
                    }
                }
                children.sort();
                let mut directories = Vec::new();
                for child in children {
                    let Ok(meta) = fs::symlink_metadata(&child) else {
                        report.errors += 1;
                        continue;
                    };
                    if meta.file_type().is_symlink() {
                        report.skipped += 1;
                        continue;
                    }
                    if meta.is_dir() {
                        if recursive {
                            directories.push(child);
                        } else {
                            report.skipped += 1;
                        }
                    } else {
                        add(
                            child,
                            Some(root.clone()),
                            &destination,
                            &mut report,
                            &mut seen,
                        );
                    }
                }
                pending.extend(directories.into_iter().rev());
            }
        } else {
            add(path, None, &None, &mut report, &mut seen);
        }
    }
    report
}
fn add(
    path: PathBuf,
    root: Option<PathBuf>,
    destination: &Option<PathBuf>,
    report: &mut Report,
    seen: &mut HashSet<PathBuf>,
) {
    if !path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
    {
        report.skipped += 1;
        return;
    }
    let Ok(meta) = path.metadata() else {
        report.errors += 1;
        return;
    };
    if !meta.is_file() {
        report.skipped += 1;
        return;
    }
    let path = location(&path);
    if destination
        .as_ref()
        .is_some_and(|output| path.starts_with(output))
    {
        report.skipped += 1;
        return;
    }
    if !seen.insert(path.clone()) {
        report.duplicates += 1;
        return;
    }
    report.inputs.push(Input {
        path,
        bytes: meta.len(),
        root,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn file(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"synthetic").unwrap();
    }
    #[test]
    fn recursive_order_excludes_output_and_deduplicates_overlapping_inputs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        file(&root.join("a.PDF"));
        file(&root.join("nested/b.pdf"));
        file(&root.join("nested/output/old.pdf"));
        file(&root.join("ignore.txt"));
        let report = scan(
            vec![root.into(), root.join("nested/b.pdf")],
            Some(root.join("nested/output")),
            true,
        );
        assert_eq!(report.inputs.len(), 2);
        assert_eq!(report.duplicates, 1);
        assert_eq!(report.skipped, 2);
        assert_eq!(report.inputs[0].path, location(&root.join("a.PDF")));
        assert_eq!(report.inputs[1].root, Some(location(root)));
        let shallow = scan(vec![root.into()], None, false);
        assert_eq!(shallow.inputs.len(), 1);
    }
    #[test]
    fn output_equal_to_root_is_excluded_and_missing_inputs_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        file(&dir.path().join("input.pdf"));
        let report = scan(
            vec![dir.path().into(), dir.path().join("missing.pdf")],
            Some(dir.path().into()),
            true,
        );
        assert!(report.inputs.is_empty());
        assert_eq!(report.skipped, 1);
        assert_eq!(report.errors, 1);
        assert_eq!(
            scan(
                vec![dir.path().join("input.pdf")],
                Some(dir.path().into()),
                true
            )
            .inputs
            .len(),
            1
        );
    }
    #[cfg(unix)]
    #[test]
    fn symlink_cycles_are_skipped_and_output_alias_is_excluded() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        file(&dir.path().join("source/a.pdf"));
        file(&dir.path().join("source/result/old.pdf"));
        symlink(dir.path().join("source"), dir.path().join("source/loop")).unwrap();
        symlink(
            dir.path().join("source/result"),
            dir.path().join("output-alias"),
        )
        .unwrap();
        let report = scan(
            vec![dir.path().join("source")],
            Some(dir.path().join("output-alias")),
            true,
        );
        assert_eq!(report.inputs.len(), 1);
        assert_eq!(report.skipped, 2);
    }
}
