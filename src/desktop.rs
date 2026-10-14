//! Desktop integration stays out of the engine and runs on a worker.
use std::{path::Path, sync::Mutex};
pub const SITE: &str = "https://made.pedropaulo.net/projects/pdftrimly/";
pub const SUPPORT: &str = "https://ko-fi.com/pedropaulo";
pub const SUPPORT_PIX: &str = "https://livepix.gg/pedropaulo";
pub fn support_url(language: crate::i18n::Language) -> &'static str {
    if language == crate::i18n::Language::Portuguese {
        SUPPORT_PIX
    } else {
        SUPPORT
    }
}
pub trait Desktop: Send + Sync {
    fn copy(&self, text: &str) -> Result<(), String>;
    fn folder(&self, path: &Path) -> Result<(), String>;
    fn file(&self, path: &Path) -> Result<(), String>;
    fn link(&self, url: &str) -> Result<(), String>;
}
#[derive(Default)]
pub struct NativeDesktop {
    clipboard: Mutex<Option<arboard::Clipboard>>,
}
impl Desktop for NativeDesktop {
    fn copy(&self, text: &str) -> Result<(), String> {
        let mut clipboard = self.clipboard.lock().map_err(|e| e.to_string())?;
        if clipboard.is_none() {
            *clipboard = Some(arboard::Clipboard::new().map_err(|e| e.to_string())?);
        }
        clipboard
            .as_mut()
            .unwrap()
            .set_text(text)
            .map_err(|e| e.to_string())
    }
    fn folder(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() || !path.is_dir() {
            return Err("Output folder is unavailable".into());
        }
        #[cfg(target_os = "windows")]
        let mut command = std::process::Command::new("explorer.exe");
        #[cfg(not(target_os = "windows"))]
        let mut command = std::process::Command::new("xdg-open");
        #[cfg(target_os = "windows")]
        {
            command.arg(path).spawn().map_err(|e| e.to_string())?;
            Ok(())
        }
        #[cfg(not(target_os = "windows"))]
        {
            let status = command
                .arg(path)
                .output()
                .map_err(|e| e.to_string())?
                .status;
            if status.success() {
                Ok(())
            } else {
                Err(format!("Desktop opener failed: {status}"))
            }
        }
    }
    fn file(&self, path: &Path) -> Result<(), String> {
        if !path.is_absolute() || !path.is_file() {
            return Err("Output file is unavailable".into());
        }
        #[cfg(target_os = "windows")]
        let mut command = std::process::Command::new("explorer.exe");
        #[cfg(target_os = "macos")]
        let mut command = std::process::Command::new("open");
        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        let mut command = std::process::Command::new("xdg-open");

        command
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
    fn link(&self, url: &str) -> Result<(), String> {
        if ![SITE, SUPPORT, SUPPORT_PIX].contains(&url) {
            return Err("Unsupported product link".into());
        }
        webbrowser::open(url).map_err(|e| e.to_string())
    }
}
