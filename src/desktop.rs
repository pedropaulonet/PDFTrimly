//! Desktop integration stays out of the engine and runs on a worker.
use std::{path::Path, sync::Mutex};
pub const SITE: &str = "https://dev.pedropaulo.net/pdftrimly";
pub const SUPPORT: &str = "https://ko-fi.com/pedropaulo";
pub trait Desktop: Send + Sync {
    fn copy(&self, text: &str) -> Result<(), String>;
    fn folder(&self, path: &Path) -> Result<(), String>;
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
    fn link(&self, url: &str) -> Result<(), String> {
        if ![SITE, SUPPORT].contains(&url) {
            return Err("Unsupported product link".into());
        }
        webbrowser::open(url).map_err(|e| e.to_string())
    }
}
