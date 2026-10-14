#[path = "../crates/pdftrimly-core/tests/common/mod.rs"]
mod common;

use std::{fs, process::Command};

#[test]
fn compress_command_reports_measured_savings_and_never_overwrites() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input with spaces.pdf");
    let output = dir.path().join("output with spaces.pdf");
    common::document(2000, false).save(&input).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_pdftrimly"))
        .env("PDFTRIMLY_LANG", "pt-BR")
        .arg("compress")
        .arg(&input)
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("Economia:"));
    assert!(fs::metadata(&output).unwrap().len() < fs::metadata(&input).unwrap().len());
    let original_output = fs::read(&output).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_pdftrimly"))
        .env("PDFTRIMLY_LANG", "pt-BR")
        .arg("compress")
        .arg(&input)
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(fs::read(output).unwrap(), original_output);
}

#[test]
fn invalid_arguments_fail_and_help_describes_compress_command() {
    let binary = env!("CARGO_BIN_EXE_pdftrimly");
    let result = Command::new(binary).arg("--help").output().unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("compress"));
    assert!(!Command::new(binary)
        .env("PDFTRIMLY_LANG", "pt-BR")
        .arg("compress")
        .output()
        .unwrap()
        .status
        .success());
}

#[test]
fn accepts_levels_and_rejects_unknown_level() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(100, false).save(&input).unwrap();
    for level in ["light", "normal", "ultra", "maximum", "unknown"] {
        let output = dir.path().join(format!("{level}.pdf"));
        let result = Command::new(env!("CARGO_BIN_EXE_pdftrimly"))
            .env("PDFTRIMLY_LANG", "pt-BR")
            .arg("compress")
            .arg(&input)
            .arg(&output)
            .args(["--level", level])
            .output()
            .unwrap();
        assert_eq!(result.status.success(), level != "unknown");
        assert_eq!(output.exists(), level != "unknown");
    }
}

#[test]
fn cli_localizes_help_and_errors_in_all_six_languages() {
    let binary = env!("CARGO_BIN_EXE_pdftrimly");
    for (locale, help, error) in [
        ("en", "Usage:", "Invalid arguments."),
        ("pt_BR.UTF-8", "Uso:", "Argumentos inválidos."),
        ("fr-FR", "Utilisation", "Arguments invalides."),
        ("es", "Uso:", "Argumentos inválidos."),
        ("it", "Uso:", "Argomenti non validi."),
        ("de_DE.UTF-8", "Verwendung:", "Ungültige Argumente."),
        ("unknown", "Usage:", "Invalid arguments."),
    ] {
        let output = Command::new(binary)
            .env("PDFTRIMLY_LANG", locale)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains(help));
        let output = Command::new(binary)
            .env("PDFTRIMLY_LANG", locale)
            .arg("invalid")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains(error));
    }
}

#[test]
fn cli_uses_saved_language_and_environment_override_in_isolated_config_directory() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("pdftrimly");
    fs::create_dir(&config).unwrap();
    fs::write(
        config.join("preferences.toml"),
        "version = 1\nlanguage = 'fr'\n",
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_pdftrimly"));
    command
        .env("XDG_CONFIG_HOME", dir.path())
        .env("APPDATA", dir.path())
        .env_remove("PDFTRIMLY_LANG")
        .arg("--help");
    let result = command.output().unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Utilisation"));
    let result = command.env("PDFTRIMLY_LANG", "pt-BR").output().unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Uso:"));
}
