//! GUI state keeps semantic results so changing language never reruns a batch.
use crate::{
    batch::{BatchError, BatchEvent},
    i18n::Language,
    messages::{self, Totals},
};
use pdftrimly_core::{OptimizeReport, Outcome};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
};

pub enum RowStatus {
    Selected,
    Queued,
    Running,
    Finished(Result<(OptimizeReport, PathBuf), BatchError>),
    Cancelled,
    WorkerFailed,
}

pub struct Entry {
    pub path: PathBuf,
    pub bytes: u64,
    pub root: Option<PathBuf>,
    pub status: RowStatus,
}

impl Entry {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
    pub fn short_status(&self, language: Language) -> String {
        match &self.status {
            RowStatus::Selected => "—".into(),
            RowStatus::Queued => language.text("queued").into(),
            RowStatus::Running => language.text("running").into(),
            RowStatus::Cancelled => language.text("cancelled").into(),
            RowStatus::WorkerFailed => language.text("row_error").into(),
            RowStatus::Finished(Err(_)) => language.text("row_error").into(),
            RowStatus::Finished(Ok((report, _))) => format!(
                "{} · {}%",
                language.text(if report.outcome == Outcome::Reduced {
                    "reduced"
                } else {
                    "unchanged"
                }),
                language.decimal(report.savings_percent(), 2),
            ),
        }
    }
    pub fn detail(&self, language: Language) -> String {
        let status = match &self.status {
            RowStatus::Selected => messages::size(language, self.bytes),
            RowStatus::Queued => language.text("queued").into(),
            RowStatus::Running => language.text("running").into(),
            RowStatus::Cancelled => language.text("cancelled").into(),
            RowStatus::WorkerFailed => language.text("worker_failed").into(),
            RowStatus::Finished(Ok((report, output))) => messages::result(language, report, output),
            RowStatus::Finished(Err(error)) => messages::error(language, error),
        };
        format!("{}\n{}", self.path.display(), status)
    }
}

pub enum Status {
    Key(&'static str),
    Processing(usize),
    Selection {
        added: usize,
        duplicates: usize,
        rejected: usize,
    },
    Summary {
        stopped: bool,
        total: usize,
    },
}

pub struct Notice {
    pub key: &'static str,
    pub detail: Option<String>,
}

pub struct State {
    pub notice: Option<Notice>,
    pub ignored: usize,
    pub entries: Vec<Entry>,
    pub destination: Option<PathBuf>,
    pub stop: Arc<AtomicBool>,
    pub totals: Totals,
    pub close_pending: bool,
    pub language: Language,
    pub status: Status,
}

impl Default for State {
    fn default() -> Self {
        Self {
            notice: None,
            ignored: 0,
            entries: Vec::new(),
            destination: None,
            stop: Arc::new(AtomicBool::new(false)),
            totals: Totals::default(),
            close_pending: false,
            language: Language::detect(),
            status: Status::Key("initial"),
        }
    }
}

impl State {
    pub fn notice_text(&self) -> String {
        self.notice
            .as_ref()
            .map(|n| match &n.detail {
                Some(detail) => self.language.format(
                    n.key,
                    &[("detail", messages::task_detail(self.language, detail))],
                ),
                None => self.language.text(n.key).into(),
            })
            .unwrap_or_default()
    }

    #[cfg(test)]
    pub fn add_inputs(&mut self, files: impl IntoIterator<Item = PathBuf>) {
        self.add_discovery(crate::discovery::scan(
            files.into_iter().collect(),
            self.destination.clone(),
            false,
        ));
    }

    pub fn add_discovery(&mut self, report: crate::discovery::Report) {
        use std::collections::HashSet;
        let mut seen: HashSet<_> = self
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect();
        let mut duplicates = report.duplicates;
        let mut added = 0;
        for input in report.inputs {
            if !seen.insert(input.path.clone()) {
                duplicates += 1;
                continue;
            }
            self.entries.push(Entry {
                path: input.path,
                bytes: input.bytes,
                root: input.root,
                status: RowStatus::Selected,
            });
            added += 1;
        }
        let rejected = report.skipped + report.errors;
        self.ignored += rejected + duplicates;
        self.status = Status::Selection {
            added,
            duplicates,
            rejected,
        };
    }

    pub fn exclude_destination(&mut self) {
        if let Some(destination) = &self.destination {
            let output = crate::discovery::location(destination);
            let before = self.entries.len();
            self.entries
                .retain(|entry| entry.root.is_none() || !entry.path.starts_with(&output));
            self.ignored += before - self.entries.len();
        }
    }

    pub fn begin(&mut self) {
        self.stop = Arc::new(AtomicBool::new(false));
        self.totals = Totals::default();
        self.close_pending = false;
        self.status = Status::Key("queued");
        for entry in &mut self.entries {
            entry.status = RowStatus::Queued;
        }
    }

    pub fn apply_event(&mut self, event: BatchEvent) -> bool {
        match event {
            BatchEvent::Started { index } => {
                if let Some(entry) = self.entries.get_mut(index) {
                    entry.status = RowStatus::Running;
                }
                if !self.close_pending && !self.stop.load(Ordering::Acquire) {
                    self.status = Status::Processing(index);
                }
            }
            BatchEvent::Finished { index, result } => {
                self.totals.completed += 1;
                match &result {
                    Ok((report, _)) => {
                        self.totals.input_bytes += report.input_bytes;
                        self.totals.output_bytes += report.output_bytes;
                        match report.outcome {
                            Outcome::Reduced => self.totals.reduced += 1,
                            Outcome::Unchanged => self.totals.unchanged += 1,
                        }
                    }
                    Err(_) => self.totals.failures += 1,
                }
                if let Some(entry) = self.entries.get_mut(index) {
                    entry.status = RowStatus::Finished(result);
                }
            }
            BatchEvent::Completed { stopped } => {
                for entry in &mut self.entries {
                    if matches!(entry.status, RowStatus::Queued) {
                        entry.status = RowStatus::Cancelled;
                    }
                }
                self.status = Status::Summary {
                    stopped,
                    total: self.entries.len(),
                };
                return true;
            }
        }
        false
    }

    pub fn worker_lost(&mut self) {
        for entry in &mut self.entries {
            if matches!(entry.status, RowStatus::Queued | RowStatus::Running) {
                entry.status = RowStatus::WorkerFailed;
            }
        }
        self.status = Status::Key("worker_failed");
    }

    pub fn cancel_close(&mut self) {
        self.close_pending = false;
        if self.stop.load(Ordering::Acquire) {
            self.status = Status::Key("stopping");
        } else if let Some(index) = self
            .entries
            .iter()
            .position(|entry| matches!(entry.status, RowStatus::Running))
        {
            self.status = Status::Processing(index);
        } else if self
            .entries
            .iter()
            .any(|entry| matches!(entry.status, RowStatus::Queued))
        {
            self.status = Status::Key("queued");
        }
    }

    pub fn compact_status(&self) -> String {
        if matches!(
            self.status,
            Status::Summary { .. } | Status::Processing(_) | Status::Key("queued")
        ) {
            let savings = if self.totals.input_bytes == 0 {
                0.0
            } else {
                self.totals
                    .input_bytes
                    .saturating_sub(self.totals.output_bytes) as f64
                    * 100.0
                    / self.totals.input_bytes as f64
            };
            self.language.format(
                "compact_status",
                &[
                    ("count", self.totals.completed.to_string()),
                    ("savings", self.language.decimal(savings, 2)),
                    ("errors", self.totals.failures.to_string()),
                ],
            )
        } else {
            self.status_text()
        }
    }

    pub fn status_text(&self) -> String {
        match &self.status {
            Status::Key(key) => self.language.text(key).into(),
            Status::Processing(index) => self
                .entries
                .get(*index)
                .map(|entry| {
                    messages::processing(self.language, *index, self.entries.len(), &entry.name())
                })
                .unwrap_or_default(),
            Status::Selection {
                added,
                duplicates,
                rejected,
            } => self.language.format(
                "selection_feedback",
                &[
                    ("added", added.to_string()),
                    ("duplicates", duplicates.to_string()),
                    ("rejected", rejected.to_string()),
                ],
            ),
            Status::Summary { stopped, total } => {
                format!(
                    "{}\n{}",
                    self.totals.summary(self.language, *stopped, *total),
                    self.language
                        .format("ignored_items", &[("count", self.ignored.to_string())])
                )
            }
        }
    }
}

// A single receive loop handles every event, including arrivals during a poll.
// Probing a second time after try_iter() can silently consume and discard data.
pub fn drain_channel<T>(receiver: &Receiver<T>, mut handle: impl FnMut(T)) -> bool {
    loop {
        match receiver.try_recv() {
            Ok(event) => handle(event),
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => return true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn events_arriving_during_poll_are_handled_and_disconnect_is_distinct() {
        let (sender, receiver) = mpsc::channel();
        sender.send(1).unwrap();
        let mut received = Vec::new();
        assert!(!drain_channel(&receiver, |event| {
            received.push(event);
            if event == 1 {
                sender.send(2).unwrap();
            }
        }));
        assert_eq!(received, vec![1, 2]);
        drop(sender);
        assert!(drain_channel(&receiver, |_| panic!("no unhandled event")));
    }

    #[test]
    fn changing_language_preserves_results_and_translates_summary_and_errors() {
        let mut state = State {
            language: Language::English,
            ..State::default()
        };
        state.entries.push(Entry {
            path: "signed.pdf".into(),
            bytes: 42,
            root: None,
            status: RowStatus::Queued,
        });
        state.apply_event(BatchEvent::Finished {
            index: 0,
            result: Err(BatchError::Engine(pdftrimly_core::OptimizeError::Signed)),
        });
        assert!(state.apply_event(BatchEvent::Completed { stopped: false }));
        assert!(state.entries[0]
            .detail(state.language)
            .contains("Signed PDFs"));
        state.language = Language::Portuguese;
        assert!(state.entries[0]
            .detail(state.language)
            .contains("PDFs assinados"));
        assert!(state.status_text().contains("Lote concluído: 1/1"));
        assert_eq!(state.totals.failures, 1);
    }

    #[test]
    fn duplicate_files_are_not_reported_as_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("scan.PDF");
        std::fs::write(&file, b"dummy").unwrap();
        let mut state = State::default();
        state.add_inputs([file.clone(), file, dir.path().join("unsupported.txt")]);
        assert_eq!(state.entries.len(), 1);
        assert!(matches!(
            state.status,
            Status::Selection {
                added: 1,
                duplicates: 1,
                rejected: 1
            }
        ));
    }

    #[test]
    fn cancelling_close_restores_progress_and_new_batch_clears_old_request() {
        let mut state = State::default();
        state.entries.push(Entry {
            path: "scan.pdf".into(),
            bytes: 42,
            root: None,
            status: RowStatus::Running,
        });
        state.close_pending = true;
        state.status = Status::Key("close_waiting");
        state.cancel_close();
        assert!(!state.close_pending);
        assert!(matches!(state.status, Status::Processing(0)));
        state.close_pending = true;
        state.begin();
        assert!(!state.close_pending);
        state.stop.store(true, Ordering::Release);
        state.status = Status::Key("stopping");
        state.apply_event(BatchEvent::Started { index: 0 });
        assert!(matches!(state.status, Status::Key("stopping")));
    }
}
