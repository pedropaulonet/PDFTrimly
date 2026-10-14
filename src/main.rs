#[cfg(test)]
#[path = "../crates/pdftrimly-core/tests/common/mod.rs"]
mod test_common;

mod app;
mod batch;
mod config;
mod desktop;
mod discovery;
mod gui_state;
mod i18n;
mod messages;

use i18n::Language;
use pdftrimly_core::{optimize, CompressionLevel, OptimizeOptions, Outcome};
use std::{env, ffi::OsStr, path::Path, process::ExitCode};

fn invalid_arguments(language: Language) -> ExitCode {
    eprintln!("{}", language.text("cli_invalid"));
    ExitCode::from(2)
}

fn compress(
    input: &OsStr,
    output: &OsStr,
    level: CompressionLevel,
    language: Language,
) -> ExitCode {
    let options = OptimizeOptions {
        level,
        ..Default::default()
    };
    match optimize(Path::new(input), Path::new(output), &options) {
        Ok(report) => {
            let level_key = match level {
                CompressionLevel::Light => "light",
                CompressionLevel::Normal => "normal",
                CompressionLevel::Ultra => "ultra",
                CompressionLevel::Maximum => "maximum",
            };
            println!(
                "{}",
                language.format("cli_level", &[("level", language.text(level_key).into())])
            );
            println!(
                "{}",
                language.text(match level {
                    CompressionLevel::Light => "lossless",
                    CompressionLevel::Normal => "normal_hint",
                    CompressionLevel::Ultra => "ultra_hint",
                    CompressionLevel::Maximum => "maximum_hint",
                })
            );
            println!(
                "{}",
                language.format(
                    "cli_outcome",
                    &[(
                        "outcome",
                        language
                            .text(if report.outcome == Outcome::Reduced {
                                "reduced"
                            } else {
                                "unchanged"
                            })
                            .into()
                    )]
                )
            );
            println!(
                "{}",
                language.format(
                    "cli_metrics",
                    &[
                        ("input", report.input_bytes.to_string()),
                        ("output", report.output_bytes.to_string()),
                        ("savings", language.decimal(report.savings_percent(), 2)),
                        ("pages", report.pages.to_string()),
                        ("streams", report.optimized_streams.to_string()),
                        ("images", report.optimized_images.to_string()),
                        ("preserved", report.skipped_images.to_string()),
                        ("unchanged", report.skipped_streams.to_string()),
                        (
                            "duration",
                            language.decimal(report.duration.as_secs_f64(), 3)
                        ),
                    ]
                )
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "{}",
                language.format(
                    "error",
                    &[("reason", messages::engine_error(language, &error))]
                )
            );
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    if let Err(error) = i18n::validate_catalogs() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }
    let language = config::load(config::config_path().as_deref()).0.language();
    let args: Vec<_> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => match app::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!(
                    "{}",
                    language.format("cli_gui_error", &[("detail", error.to_string())])
                );
                ExitCode::FAILURE
            }
        },
        [arg] if arg == "--version" || arg == "-V" => {
            println!("pdftrimly {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [arg] if arg == "--help" || arg == "-h" => {
            println!(
                "PDFTrimly {}\n\n{}",
                env!("CARGO_PKG_VERSION"),
                language.text("cli_help")
            );
            ExitCode::SUCCESS
        }
        [command, input, output] if command == "compress" => {
            compress(input, output, CompressionLevel::Normal, language)
        }
        [command, input, output, flag, level] if command == "compress" && flag == "--level" => {
            let level = match level.to_str() {
                Some("light" | "leve") => CompressionLevel::Light,
                Some("normal") => CompressionLevel::Normal,
                Some("ultra") => CompressionLevel::Ultra,
                Some("maximum") => CompressionLevel::Maximum,
                _ => return invalid_arguments(language),
            };
            compress(input, output, level, language)
        }
        _ => invalid_arguments(language),
    }
}
