use pdftrimly_core::{optimize, CompressionLevel, OptimizeError, OptimizeOptions, OptimizeReport};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
pub struct BatchRequest {
    pub inputs: Vec<BatchInput>,
    pub output_dir: Option<PathBuf>,
    pub level: CompressionLevel,
    pub suffix: String,
}
pub struct BatchInput {
    pub path: PathBuf,
    pub relative_dir: PathBuf,
}
impl From<PathBuf> for BatchInput {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            relative_dir: PathBuf::new(),
        }
    }
}
pub enum BatchEvent {
    Started {
        index: usize,
    },
    Finished {
        index: usize,
        result: Result<(OptimizeReport, PathBuf), BatchError>,
    },
    Completed {
        stopped: bool,
    },
}
#[derive(Debug)]
pub enum BatchError {
    Engine(OptimizeError),
    NameExhausted,
    Unexpected,
}
pub fn run_batch(request: BatchRequest, stop: Arc<AtomicBool>, mut emit: impl FnMut(BatchEvent)) {
    for (index, input) in request.inputs.iter().enumerate() {
        if stop.load(Ordering::Acquire) {
            break;
        }
        emit(BatchEvent::Started { index });
        let result = std::panic::catch_unwind(|| {
            if input
                .relative_dir
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
            {
                return Err(BatchError::Unexpected);
            }
            let output_root = request.output_dir.clone().unwrap_or_else(|| {
                input
                    .path
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf()
            });
            let directory = if request.output_dir.is_some() {
                output_root.join(&input.relative_dir)
            } else {
                output_root.clone()
            };
            let expected_root = crate::discovery::location(&output_root);
            if !crate::discovery::location(&directory).starts_with(&expected_root) {
                return Err(BatchError::Unexpected);
            }
            if let Err(error) = std::fs::create_dir_all(&directory) {
                return Err(BatchError::Engine(OptimizeError::Io(error)));
            }
            // Reject existing symlink directories that redirect output outside the destination.
            let root = crate::discovery::location(&output_root);
            if !crate::discovery::location(&directory).starts_with(&root) {
                return Err(BatchError::Unexpected);
            }
            let stem = input.path.file_stem().unwrap_or_default();
            for number in 0..100 {
                let mut name = stem.to_os_string();
                name.push(if number == 0 {
                    format!("-{}.pdf", request.suffix)
                } else {
                    format!("-{}-{number}.pdf", request.suffix)
                });
                let output = directory.join(name);
                match optimize(
                    &input.path,
                    &output,
                    &OptimizeOptions {
                        level: request.level,
                        ..Default::default()
                    },
                ) {
                    Err(OptimizeError::OutputExists) => continue,
                    Ok(report) => return Ok((report, output)),
                    Err(error) => return Err(BatchError::Engine(error)),
                }
            }
            Err(BatchError::NameExhausted)
        })
        .unwrap_or(Err(BatchError::Unexpected));
        emit(BatchEvent::Finished { index, result });
    }
    emit(BatchEvent::Completed {
        stopped: stop.load(Ordering::Acquire),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_common as common;

    #[test]
    fn stop_finishes_current_file_and_never_starts_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("first.pdf");
        let second = dir.path().join("second.pdf");
        common::document(100, false).save(&input).unwrap();
        common::document(100, false).save(&second).unwrap();
        let original = std::fs::read(&input).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let mut events = Vec::new();
        run_batch(
            BatchRequest {
                inputs: vec![input.clone().into(), second.into()],
                output_dir: Some(dir.path().into()),
                level: CompressionLevel::Normal,
                suffix: "compactado".into(),
            },
            stop.clone(),
            |event| {
                if matches!(event, BatchEvent::Started { index: 0 }) {
                    stop.store(true, Ordering::Release);
                }
                events.push(event);
            },
        );
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], BatchEvent::Started { index: 0 }));
        assert!(matches!(
            events[1],
            BatchEvent::Finished {
                index: 0,
                result: Ok(_)
            }
        ));
        assert!(matches!(events[2], BatchEvent::Completed { stopped: true }));
        assert!(dir.path().join("first-compactado.pdf").exists());
        assert!(!dir.path().join("second-compactado.pdf").exists());
        assert_eq!(std::fs::read(input).unwrap(), original);
    }
    #[test]
    fn relative_paths_cannot_escape_destination() {
        let dir = tempfile::tempdir().unwrap();
        for relative_dir in [PathBuf::from("../escape"), dir.path().join("absolute")] {
            let mut failed = false;
            run_batch(
                BatchRequest {
                    inputs: vec![BatchInput {
                        path: dir.path().join("missing.pdf"),
                        relative_dir,
                    }],
                    output_dir: Some(dir.path().join("output")),
                    level: CompressionLevel::Normal,
                    suffix: "compactado".into(),
                },
                Arc::new(AtomicBool::new(false)),
                |event| {
                    if let BatchEvent::Finished { result, .. } = event {
                        failed = matches!(result, Err(BatchError::Unexpected));
                    }
                },
            );
            assert!(failed);
        }
        assert!(!dir.path().join("output").exists());
        assert!(!dir.path().join("absolute").exists());
    }

    #[cfg(unix)]
    #[test]
    fn output_symlink_cannot_create_directories_outside_destination() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("output");
        let outside = dir.path().join("outside");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, output.join("nested")).unwrap();
        let mut failed = false;
        run_batch(
            BatchRequest {
                inputs: vec![BatchInput {
                    path: dir.path().join("missing.pdf"),
                    relative_dir: "nested/new".into(),
                }],
                output_dir: Some(output),
                level: CompressionLevel::Normal,
                suffix: "compactado".into(),
            },
            Arc::new(AtomicBool::new(false)),
            |event| {
                if let BatchEvent::Finished { result, .. } = event {
                    failed = matches!(result, Err(BatchError::Unexpected));
                }
            },
        );
        assert!(failed);
        assert!(!outside.join("new").exists());
    }

    #[test]
    fn overlapping_relative_names_get_distinct_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let mut inputs = Vec::new();
        for root in ["one", "two"] {
            let source = dir.path().join(root);
            std::fs::create_dir_all(&source).unwrap();
            let path = source.join("same.pdf");
            common::document(100, false).save(&path).unwrap();
            inputs.push(BatchInput {
                path,
                relative_dir: "nested".into(),
            });
        }
        let output = dir.path().join("output");
        let mut successes = 0;
        run_batch(
            BatchRequest {
                inputs,
                output_dir: Some(output.clone()),
                level: CompressionLevel::Normal,
                suffix: "compactado".into(),
            },
            Arc::new(AtomicBool::new(false)),
            |event| {
                if matches!(event, BatchEvent::Finished { result: Ok(_), .. }) {
                    successes += 1;
                }
            },
        );
        assert_eq!(successes, 2);
        assert!(output.join("nested/same-compactado.pdf").exists());
        assert!(output.join("nested/same-compactado-1.pdf").exists());
    }
}
