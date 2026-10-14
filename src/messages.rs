use crate::{batch::BatchError, i18n::Language};
use pdftrimly_core::{OptimizeError, OptimizeReport, Outcome};
use std::path::Path;

pub fn size(language: Language, bytes: u64) -> String {
    if bytes < 1_000 {
        format!("{bytes} B")
    } else if bytes < 1_000_000 {
        format!("{} KB", language.decimal(bytes as f64 / 1_000.0, 2))
    } else {
        format!("{} MB", language.decimal(bytes as f64 / 1_000_000.0, 2))
    }
}

pub fn processing(language: Language, index: usize, total: usize, name: &str) -> String {
    language.format(
        "processing",
        &[
            ("current", (index + 1).to_string()),
            ("total", total.to_string()),
            ("name", name.into()),
        ],
    )
}

fn engine_detail(language: Language, detail: &str) -> String {
    let key = match detail {
        "a entrada deve ser um arquivo regular" => "detail_regular_file",
        "tamanho do arquivo de entrada" => "detail_input_size",
        "fluxos descomprimidos" => "detail_decoded_streams",
        "dados extras após fluxo Flate" => "detail_flate_extra",
        "a recompressão alterou o fluxo" => "detail_stream_changed",
        "profundidade de objetos" => "detail_object_depth",
        "referência a objeto inexistente" => "detail_missing_object",
        "PDF linearizado ainda não suportado" => "detail_linearized",
        "árvore de páginas cíclica ou profunda" => "detail_page_tree",
        "tipo inválido na árvore de páginas" => "detail_page_type",
        "Parent de página inconsistente" => "detail_page_parent",
        "contagem de páginas inconsistente" => "detail_page_count",
        "quantidade ou identificadores de objetos" => "detail_object_count",
        "catálogo inválido" => "detail_catalog",
        "documento sem páginas" => "detail_no_pages",
        "objetos ou páginas mudaram após gravação" => "detail_objects_changed",
        "o trailer mudou após gravação" => "detail_trailer_changed",
        _ => return detail.into(),
    };
    language.text(key).into()
}

pub fn engine_error(language: Language, error: &OptimizeError) -> String {
    let key = match error {
        OptimizeError::OutputExists => "error_output_exists",
        OptimizeError::Encrypted => "error_encrypted",
        OptimizeError::Signed => "error_signed",
        OptimizeError::LimitExceeded(_) => "error_limit",
        OptimizeError::Structure(_) => "error_structure",
        OptimizeError::Pdf(_) => "error_pdf",
        OptimizeError::Io(error) => {
            return language.format("error_io", &[("detail", error.to_string())])
        }
    };
    if let OptimizeError::Structure(detail) | OptimizeError::LimitExceeded(detail) = error {
        language.format(
            "error_detail",
            &[
                ("reason", language.text(key).into()),
                ("detail", engine_detail(language, detail)),
            ],
        )
    } else {
        language.text(key).into()
    }
}

pub fn error(language: Language, error: &BatchError) -> String {
    let reason = match error {
        BatchError::Engine(error) => engine_error(language, error),
        BatchError::NameExhausted => language.text("error_name_exhausted").into(),
        BatchError::Unexpected => language.text("error_unexpected").into(),
    };
    language.format("error", &[("reason", reason)])
}

pub fn result(language: Language, report: &OptimizeReport, path: &Path) -> String {
    language.format(
        "result",
        &[
            (
                "outcome",
                language
                    .text(if report.outcome == Outcome::Reduced {
                        "reduced"
                    } else {
                        "unchanged"
                    })
                    .into(),
            ),
            ("input", size(language, report.input_bytes)),
            ("output", size(language, report.output_bytes)),
            ("savings", language.decimal(report.savings_percent(), 2)),
            ("path", path.display().to_string()),
        ],
    )
}

pub fn comparison(language: Language, report: &OptimizeReport) -> String {
    let saved = report.input_bytes.saturating_sub(report.output_bytes);
    let savings = if report.outcome == Outcome::Unchanged {
        language.text("no_savings").to_owned()
    } else {
        format!("{}%", language.decimal(report.savings_percent(), 2))
    };
    language.format(
        "comparison",
        &[
            ("input", size(language, report.input_bytes)),
            ("output", size(language, report.output_bytes)),
            ("saved", size(language, saved)),
            ("savings", savings),
        ],
    )
}

#[derive(Default)]
pub struct Totals {
    pub completed: usize,
    pub failures: usize,
    pub reduced: usize,
    pub unchanged: usize,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

impl Totals {
    pub fn summary(&self, language: Language, stopped: bool, total: usize) -> String {
        let savings = if self.input_bytes == 0 {
            0.0
        } else {
            self.input_bytes.saturating_sub(self.output_bytes) as f64 * 100.0
                / self.input_bytes as f64
        };
        language.format(
            "summary",
            &[
                (
                    "state",
                    language
                        .text(if stopped { "stopped" } else { "completed" })
                        .into(),
                ),
                ("completed", self.completed.to_string()),
                ("total", total.to_string()),
                ("reduced", self.reduced.to_string()),
                ("unchanged", self.unchanged.to_string()),
                ("errors", self.failures.to_string()),
                ("input", size(language, self.input_bytes)),
                ("output", size(language, self.output_bytes)),
                ("savings", language.decimal(savings, 2)),
            ],
        )
    }
}

// Translate fixed application diagnostics while preserving OS-provided detail.
pub fn task_detail(language: Language, detail: &str) -> String {
    match detail {
        "Default folder is unavailable" | "Output folder is unavailable" => {
            language.text("config_folder_missing").into()
        }
        "Background task disconnected" => language.text("task_disconnected").into(),
        "Invalid level" | "Missing config directory" | "Folder is not UTF-8" => {
            language.text("invalid_preference").into()
        }
        _ => detail
            .strip_prefix("Desktop opener failed: ")
            .map(|status| language.format("desktop_open_failed", &[("detail", status.into())]))
            .unwrap_or_else(|| detail.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn report(outcome: Outcome, input_bytes: u64, output_bytes: u64) -> OptimizeReport {
        OptimizeReport {
            outcome,
            input_bytes,
            output_bytes,
            pages: 1,
            optimized_streams: 0,
            optimized_images: 0,
            skipped_images: 0,
            skipped_streams: 0,
            duration: Duration::ZERO,
        }
    }

    #[test]
    fn comparison_reports_measured_sizes_and_saved_bytes() {
        let text = comparison(Language::English, &report(Outcome::Reduced, 2_000, 1_000));
        assert_eq!(text, "2.00 KB → 1.00 KB · saved 1.00 KB (50.00%)");
    }

    #[test]
    fn unchanged_comparison_says_no_savings() {
        let text = comparison(Language::English, &report(Outcome::Unchanged, 1_000, 1_000));
        assert_eq!(text, "1.00 KB → 1.00 KB · saved 0 B (no savings)");
    }
}
