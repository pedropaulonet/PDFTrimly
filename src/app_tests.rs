//! Tests the real GUI callbacks, timers and worker with an in-memory Slint window.
//! Native portal responses are supplied here; portal and OS drop delivery need manual checks.
use super::*;
use slint::platform::{
    software_renderer::MinimalSoftwareWindow, Platform, WindowAdapter, WindowEvent,
};
use std::{collections::VecDeque, fs, sync::Mutex, time::Instant};

use crate::test_common as common;

struct Headless;
impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(MinimalSoftwareWindow::new(Default::default()))
    }
}

#[derive(Default)]
struct ScriptedPicker {
    files: Mutex<VecDeque<Option<Vec<PathBuf>>>>,
    folders: Mutex<VecDeque<Option<PathBuf>>>,
}
impl FilePicker for ScriptedPicker {
    fn files(&self) -> Option<Vec<PathBuf>> {
        self.files.lock().unwrap().pop_front().flatten()
    }
    fn folder(&self) -> Option<PathBuf> {
        self.folders.lock().unwrap().pop_front().flatten()
    }
}
fn window(picker: Arc<dyn FilePicker>) -> MainWindow {
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let ui = create_window(picker, None, Arc::new(MockDesktop::default())).unwrap();
    ui.invoke_language_changed(Language::English as i32);
    ui.show().unwrap();
    ui
}
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        slint::platform::update_timers_and_animations();
        if predicate() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "GUI did not reach the expected state within 10s"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn drop_paths(ui: &MainWindow, paths: &[PathBuf]) {
    let mut transfer = slint::DataTransfer::default();
    transfer.set_file_paths(paths);
    ui.invoke_add_dropped_files(transfer);
    until(|| !ui.get_dialog_open());
}
fn choose_folder(ui: &MainWindow, picker: &ScriptedPicker, path: &std::path::Path) {
    picker.folders.lock().unwrap().push_back(Some(path.into()));
    ui.invoke_choose_folder();
    assert!(ui.get_dialog_open());
    until(|| !ui.get_dialog_open());
    assert!(ui.get_has_destination());
}
fn request_close(ui: &MainWindow) {
    ui.window().dispatch_event(WindowEvent::CloseRequested);
}

#[test]
fn gui_processes_real_batch_errors_language_changes_and_output_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let good = dir
        .path()
        .join("Relatório com espaços, acentos e nome longo.PDF");
    let bad = dir.path().join("invalid.pdf");
    common::document(2000, false).save(&good).unwrap();
    fs::write(&bad, b"invalid PDF").unwrap();
    let original = fs::read(&good).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    // The same callback receives actual paths from both picker and drop.
    picker
        .files
        .lock()
        .unwrap()
        .push_back(Some(vec![good.clone(), bad.clone()]));
    ui.invoke_add_files();
    until(|| !ui.get_dialog_open());
    drop_paths(&ui, &[good.clone(), dir.path().join("unsupported.txt")]);
    assert_eq!(ui.get_rows().row_count(), 2);
    assert!(ui.get_status().contains("already selected: 1"));
    assert!(ui.get_status().contains("unsupported or unavailable: 1"));
    choose_folder(&ui, &picker, dir.path());
    ui.invoke_start();
    assert!(ui.get_busy());
    // Guards must work even when invoked directly while UI controls are disabled.
    ui.invoke_remove(0);
    ui.invoke_clear_files();
    ui.invoke_add_files();
    drop_paths(&ui, &[dir.path().join("third.pdf")]);
    assert_eq!(ui.get_rows().row_count(), 2);
    assert!(!ui.get_dialog_open());
    ui.invoke_language_changed(Language::French as i32);
    until(|| !ui.get_busy());
    assert_eq!(ui.get_progress(), 1.0);
    assert!(!ui.get_close_confirm());
    assert!(ui.get_status().contains("2/2"));
    assert!(ui
        .get_rows()
        .row_data(1)
        .unwrap()
        .detail
        .contains("Impossible de lire la structure du PDF"));
    let first = dir
        .path()
        .join("Relatório com espaços, acentos e nome longo-compressed.pdf");
    let bytes = fs::read(&first).unwrap();
    assert!(bytes.len() < original.len());
    ui.invoke_language_changed(Language::English as i32);
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert!(dir
        .path()
        .join("Relatório com espaços, acentos e nome longo-compressed-1.pdf")
        .exists());
    assert_eq!(fs::read(&first).unwrap(), bytes);
    assert_eq!(fs::read(&good).unwrap(), original);
    ui.invoke_clear_files();
    assert_eq!(ui.get_rows().row_count(), 0);
    assert_eq!(ui.get_progress(), 0.0);
    ui.hide().unwrap();
}

#[test]
fn editing_selection_resets_completed_batch_progress() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.pdf");
    let second = dir.path().join("second.pdf");
    common::document(100, false).save(&first).unwrap();
    common::document(100, false).save(&second).unwrap();
    let ui = window(Arc::new(ScriptedPicker::default()));
    drop_paths(&ui, &[first]);
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert_eq!(ui.get_progress(), 1.0);
    drop_paths(&ui, &[second]);
    assert_eq!(ui.get_rows().row_count(), 2);
    assert_eq!(ui.get_progress(), 0.0);
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert_eq!(ui.get_progress(), 1.0);
    ui.invoke_remove(0);
    assert_eq!(ui.get_rows().row_count(), 1);
    assert_eq!(ui.get_progress(), 0.0);
    ui.hide().unwrap();
}

#[test]
fn gui_cancelled_picker_releases_window_and_preserves_selection() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(10, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    drop_paths(&ui, &[input]);
    choose_folder(&ui, &picker, dir.path());
    let destination = ui.get_destination();
    ui.invoke_add_files(); // Empty script models cancellation.
    assert!(ui.get_dialog_open());
    request_close(&ui);
    assert!(ui.window().is_visible());
    assert!(!ui.get_close_confirm());
    ui.invoke_clear_files();
    assert_eq!(ui.get_rows().row_count(), 1);
    until(|| !ui.get_dialog_open());
    ui.invoke_choose_folder();
    until(|| !ui.get_dialog_open());
    assert_eq!(ui.get_destination(), destination);
    assert_eq!(ui.get_rows().row_count(), 1);
    ui.invoke_remove(-1);
    ui.invoke_remove(42);
    assert_eq!(ui.get_rows().row_count(), 1);
    ui.invoke_remove(0);
    assert_eq!(ui.get_rows().row_count(), 0);
    request_close(&ui);
    assert!(!ui.window().is_visible());
}

#[test]
fn gui_close_confirmation_handles_completion_before_or_after_user_choice() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(100, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    drop_paths(&ui, &[input]);
    choose_folder(&ui, &picker, dir.path());
    ui.invoke_start();
    request_close(&ui);
    assert!(ui.get_close_confirm());
    ui.invoke_close_back();
    assert!(!ui.get_close_confirm());
    until(|| !ui.get_busy());
    assert!(ui.window().is_visible());
    // Completion while the user is deciding dismisses the confirmation.
    ui.invoke_start();
    request_close(&ui);
    until(|| !ui.get_busy());
    assert!(!ui.get_close_confirm());
    assert!(ui.window().is_visible());
    // A late choice must hide immediately rather than wait for a finished worker.
    ui.invoke_close_wait();
    assert!(!ui.window().is_visible());
    ui.show().unwrap();
    ui.invoke_start();
    request_close(&ui);
    ui.invoke_close_wait();
    until(|| !ui.get_busy());
    assert!(!ui.window().is_visible());
    // A new batch must clear the earlier close request and stop flag.
    ui.show().unwrap();
    ui.invoke_start();
    ui.invoke_stop();
    assert!(ui.get_stop_requested());
    until(|| !ui.get_busy());
    assert!(ui.window().is_visible());
    ui.invoke_start();
    assert!(!ui.get_stop_requested());
    request_close(&ui);
    ui.invoke_close_stop();
    until(|| !ui.get_busy());
    assert!(!ui.window().is_visible());
}

#[test]
fn gui_reports_output_failure_without_sticking_in_busy_state() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(100, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    drop_paths(&ui, &[input]);
    // A file used as a directory forces an I/O failure even when running as root.
    let destination = dir.path().join("not-a-folder");
    fs::write(&destination, b"preserve me").unwrap();
    choose_folder(&ui, &picker, &destination);
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert!(ui.get_status().contains("1/1"));
    assert!(ui
        .get_rows()
        .row_data(0)
        .unwrap()
        .detail
        .contains("File access failed"));
    assert_eq!(fs::read(destination).unwrap(), b"preserve me");
    ui.hide().unwrap();
}

#[test]
fn gui_picker_disconnection_is_reported_and_controls_recover() {
    struct FailedPicker;
    impl FilePicker for FailedPicker {
        fn files(&self) -> Option<Vec<PathBuf>> {
            panic!("injected picker failure")
        }
        fn folder(&self) -> Option<PathBuf> {
            None
        }
    }
    let ui = window(Arc::new(FailedPicker));
    ui.invoke_add_files();
    until(|| !ui.get_dialog_open());
    assert_eq!(ui.get_status(), Language::English.text("dialog_failed"));
    ui.invoke_choose_folder();
    until(|| !ui.get_dialog_open());
    request_close(&ui);
    assert!(!ui.window().is_visible());
}

#[test]
fn gui_keyboard_shortcuts_open_pickers_and_escape_cancels_close() {
    use slint::platform::Key;
    fn shortcut(ui: &MainWindow, shift: bool) {
        ui.window().dispatch_event(WindowEvent::KeyPressed {
            text: Key::Control.into(),
        });
        if shift {
            ui.window().dispatch_event(WindowEvent::KeyPressed {
                text: Key::Shift.into(),
            });
        }
        ui.window()
            .dispatch_event(WindowEvent::KeyPressed { text: "o".into() });
        ui.window()
            .dispatch_event(WindowEvent::KeyReleased { text: "o".into() });
        if shift {
            ui.window().dispatch_event(WindowEvent::KeyReleased {
                text: Key::Shift.into(),
            });
        }
        ui.window().dispatch_event(WindowEvent::KeyReleased {
            text: Key::Control.into(),
        });
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(100, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    picker.files.lock().unwrap().push_back(Some(vec![input]));
    picker
        .folders
        .lock()
        .unwrap()
        .push_back(Some(dir.path().into()));
    let ui = window(picker);
    ui.invoke_focus_main();
    shortcut(&ui, false);
    assert!(ui.get_dialog_open(), "Ctrl+O must open the file picker");
    until(|| !ui.get_dialog_open());
    assert_eq!(ui.get_rows().row_count(), 1);
    shortcut(&ui, true);
    assert!(
        ui.get_dialog_open(),
        "Ctrl+Shift+O must open the folder picker"
    );
    until(|| !ui.get_dialog_open());
    assert!(ui.get_has_destination());
    ui.invoke_start();
    request_close(&ui);
    assert!(ui.get_close_confirm());
    ui.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Escape.into(),
    });
    assert!(!ui.get_close_confirm());
    until(|| !ui.get_busy());
    assert!(ui.window().is_visible());
    ui.hide().unwrap();
}

#[derive(Default)]
struct MockDesktop {
    calls: Mutex<Vec<String>>,
}
impl crate::desktop::Desktop for MockDesktop {
    fn copy(&self, text: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push(format!("copy:{text}"));
        Ok(())
    }
    fn folder(&self, path: &std::path::Path) -> Result<(), String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("folder:{}", path.display()));
        Ok(())
    }
    fn link(&self, url: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push(url.into());
        Ok(())
    }
}

#[test]
fn settings_save_restart_cancel_and_failed_save_preserve_defaults() {
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config/preferences.toml");
    let picker = Arc::new(ScriptedPicker::default());
    let desktop = Arc::new(MockDesktop::default());
    let ui = create_window(picker.clone(), Some(path.clone()), desktop.clone()).unwrap();
    ui.show().unwrap();
    ui.invoke_open_settings();
    assert!(ui.get_settings_open());
    ui.set_draft_language(3); // French, after the System option.
    ui.set_draft_theme(2);
    ui.set_draft_level(2);
    ui.set_draft_folder(dir.path().to_str().unwrap().into());
    ui.invoke_save_settings();
    assert!(ui.get_settings_busy());
    request_close(&ui);
    assert!(ui.window().is_visible());
    until(|| !ui.get_settings_busy());
    assert!(!ui.get_settings_open());
    assert_eq!(ui.get_level(), 2);
    assert_eq!(ui.get_theme_index(), 2);
    assert_eq!(ui.get_language_index(), Language::French as i32);
    assert_eq!(ui.get_destination(), dir.path().to_str().unwrap());
    let bytes = fs::read(&path).unwrap();
    ui.invoke_open_settings();
    ui.set_draft_theme(1);
    ui.set_draft_level(0);
    ui.invoke_cancel_settings();
    assert_eq!(ui.get_theme_index(), 2);
    assert_eq!(ui.get_level(), 2);
    assert_eq!(fs::read(&path).unwrap(), bytes);
    ui.invoke_open_settings();
    ui.set_draft_folder(dir.path().join("missing").to_str().unwrap().into());
    ui.invoke_save_settings();
    until(|| !ui.get_settings_busy());
    assert!(ui.get_settings_open());
    assert!(!ui.get_settings_notice().is_empty());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(ui.get_level(), 2);
    ui.invoke_cancel_settings();
    ui.hide().unwrap();
    drop(ui);
    let restarted = create_window(picker, Some(path), desktop).unwrap();
    assert_eq!(restarted.get_level(), 2);
    assert_eq!(restarted.get_theme_index(), 2);
    assert_eq!(restarted.get_destination(), dir.path().to_str().unwrap());
    assert_eq!(
        restarted.get_language_index(),
        crate::config::load(Some(&dir.path().join("config/preferences.toml")))
            .0
            .language() as i32
    );
}

#[test]
fn settings_folder_picker_and_invalid_startup_config_recover() {
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("preferences.toml");
    fs::write(&path, "theme = 'invalid'").unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = create_window(
        picker.clone(),
        Some(path.clone()),
        Arc::new(MockDesktop::default()),
    )
    .unwrap();
    ui.invoke_language_changed(Language::English as i32);
    assert!(ui.get_notice().contains("could not be read"));
    assert_eq!(ui.get_theme_index(), 0);
    ui.show().unwrap();
    ui.invoke_open_settings();
    picker
        .folders
        .lock()
        .unwrap()
        .push_back(Some(dir.path().into()));
    ui.invoke_settings_folder();
    assert!(ui.get_settings_busy());
    until(|| !ui.get_settings_busy());
    assert_eq!(ui.get_draft_folder(), dir.path().to_str().unwrap());
    assert!(!ui.get_has_destination()); // A draft doesn't change the batch.
    ui.invoke_cancel_settings();
    assert_eq!(fs::read_to_string(&path).unwrap(), "theme = 'invalid'");
    ui.hide().unwrap();
}

#[test]
fn result_actions_pass_exact_summary_path_and_product_links_to_desktop() {
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("input.pdf");
    common::document(100, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let desktop = Arc::new(MockDesktop::default());
    let ui = create_window(picker.clone(), None, desktop.clone()).unwrap();
    ui.invoke_language_changed(Language::English as i32);
    ui.show().unwrap();
    ui.invoke_copy_summary();
    assert!(desktop.calls.lock().unwrap().is_empty());
    drop_paths(&ui, &[input]);
    choose_folder(&ui, &picker, dir.path());
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert!(ui.get_has_summary());
    let summary = ui.get_status();
    ui.invoke_copy_summary();
    assert!(ui.get_action_busy());
    until(|| !ui.get_action_busy());
    assert_eq!(desktop.calls.lock().unwrap()[0], format!("copy:{summary}"));
    assert_eq!(ui.get_status(), summary);
    assert!(ui.get_notice().contains("Summary copied"));
    ui.invoke_language_changed(Language::Portuguese as i32);
    assert!(ui.get_notice().contains("Resumo copiado"));
    ui.invoke_open_output();
    until(|| !ui.get_action_busy());
    ui.invoke_open_site();
    until(|| !ui.get_action_busy());
    ui.invoke_open_support();
    until(|| !ui.get_action_busy());
    let calls = desktop.calls.lock().unwrap();
    assert_eq!(calls[1], format!("folder:{}", dir.path().display()));
    assert_eq!(calls[2], crate::desktop::SITE);
    assert_eq!(calls[3], crate::desktop::SUPPORT_PIX);
    drop(calls);
    assert!(ui.get_about_text().contains("livepix.gg/pedropaulo"));
    assert_eq!(ui.global::<Labels>().get_support(), "Apoiar com um Pix");
    ui.invoke_language_changed(Language::English as i32);
    ui.invoke_open_support();
    until(|| !ui.get_action_busy());
    assert_eq!(desktop.calls.lock().unwrap()[4], crate::desktop::SUPPORT);
    assert!(ui.get_about_text().contains("ko-fi.com/pedropaulo"));
    ui.hide().unwrap();
}

#[test]
fn desktop_action_failure_preserves_summary_and_recovers_controls() {
    struct FailingDesktop;
    impl crate::desktop::Desktop for FailingDesktop {
        fn copy(&self, _: &str) -> Result<(), String> {
            Err("unavailable".into())
        }
        fn folder(&self, _: &std::path::Path) -> Result<(), String> {
            Err("unavailable".into())
        }
        fn link(&self, _: &str) -> Result<(), String> {
            Err("unavailable".into())
        }
    }
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let ui = create_window(
        Arc::new(ScriptedPicker::default()),
        None,
        Arc::new(FailingDesktop),
    )
    .unwrap();
    ui.invoke_language_changed(Language::English as i32);
    let status = ui.get_status();
    ui.invoke_open_site();
    until(|| !ui.get_action_busy());
    assert!(ui.get_notice().contains("Desktop action failed"));
    assert_eq!(ui.get_status(), status);
    ui.invoke_open_support();
    until(|| !ui.get_action_busy());
}

#[test]
fn settings_render_in_all_languages_sizes_and_themes() {
    use std::io::Write;
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let ui = create_window(
        Arc::new(ScriptedPicker::default()),
        None,
        Arc::new(MockDesktop::default()),
    )
    .unwrap();
    ui.show().unwrap();
    let captures = std::env::var_os("PDFTRIMLY_TEST_SCREENSHOTS").map(PathBuf::from);
    if let Some(path) = &captures {
        fs::create_dir_all(path).unwrap();
    }
    for (width, height) in [(680u32, 800u32), (960, 900)] {
        ui.window()
            .set_size(slint::PhysicalSize::new(width, height));
        for language in 0..5 {
            ui.set_settings_open(false);
            ui.invoke_language_changed(language);
            ui.invoke_open_settings();
            let mut light = 0;
            for theme in 0..3 {
                ui.set_theme_index(theme);
                ui.invoke_apply_theme();
                slint::platform::update_timers_and_animations();
                let snapshot = ui.window().take_snapshot().unwrap();
                assert_eq!((snapshot.width(), snapshot.height()), (width, height));
                let background = snapshot.as_slice()[0].r;
                if theme == 1 {
                    light = background;
                }
                if theme == 2 {
                    assert!(
                        light > background + 20,
                        "Light and Dark must affect the rendered palette"
                    );
                }
                if let Some(path) = &captures {
                    let mut file = fs::File::create(
                        path.join(format!("settings-{width}-{language}-{theme}.ppm")),
                    )
                    .unwrap();
                    write!(file, "P6\n{width} {height}\n255\n").unwrap();
                    let pixels: Vec<u8> = snapshot
                        .as_slice()
                        .iter()
                        .flat_map(|pixel| [pixel.r, pixel.g, pixel.b])
                        .collect();
                    file.write_all(&pixels).unwrap();
                }
            }
        }
    }
    ui.hide().unwrap();
}

#[test]
fn brand_title_renders_in_main_window_and_about() {
    use std::io::Write;
    let ui = window(Arc::new(ScriptedPicker::default()));
    ui.invoke_language_changed(Language::Portuguese as i32);
    ui.set_theme_index(1);
    ui.invoke_apply_theme();
    ui.window().set_size(slint::PhysicalSize::new(680, 800));
    for about in [false, true] {
        if about {
            let position = slint::LogicalPosition::new(610.0, 50.0);
            ui.window().dispatch_event(WindowEvent::PointerPressed {
                position,
                button: slint::platform::PointerEventButton::Left,
            });
            ui.window().dispatch_event(WindowEvent::PointerReleased {
                position,
                button: slint::platform::PointerEventButton::Left,
            });
            assert!(ui.get_modal());
        }
        let snapshot = ui.window().take_snapshot().unwrap();
        let (x, y) = if about { (140, 205) } else { (84, 28) };
        let mut red_x = Vec::new();
        for row in y..y + 44 {
            for col in x..x + 155 {
                let p = snapshot.as_slice()[row * 680 + col];
                if p.r > 160 && p.g < 100 && p.b < 110 {
                    red_x.push(col);
                }
            }
        }
        assert!(!red_x.is_empty(), "Trimly must render beside PDF");
        assert!(*red_x.iter().min().unwrap() < x + 70);
        if let Some(path) = std::env::var_os("PDFTRIMLY_TEST_SCREENSHOTS") {
            let path = PathBuf::from(path);
            fs::create_dir_all(&path).unwrap();
            let name = if about {
                "brand-about.ppm"
            } else {
                "brand-main.ppm"
            };
            let mut file = fs::File::create(path.join(name)).unwrap();
            write!(file, "P6\n680 800\n255\n").unwrap();
            let pixels: Vec<u8> = snapshot
                .as_slice()
                .iter()
                .flat_map(|p| [p.r, p.g, p.b])
                .collect();
            file.write_all(&pixels).unwrap();
        }
    }
    ui.hide().unwrap();
}

#[test]
fn desktop_worker_disconnection_recovers_and_reports_in_current_language() {
    struct DisconnectedDesktop;
    impl crate::desktop::Desktop for DisconnectedDesktop {
        fn copy(&self, _: &str) -> Result<(), String> {
            unreachable!()
        }
        fn folder(&self, _: &std::path::Path) -> Result<(), String> {
            unreachable!()
        }
        fn link(&self, _: &str) -> Result<(), String> {
            panic!("injected desktop failure")
        }
    }
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let ui = create_window(
        Arc::new(ScriptedPicker::default()),
        None,
        Arc::new(DisconnectedDesktop),
    )
    .unwrap();
    ui.invoke_language_changed(Language::Portuguese as i32);
    ui.invoke_open_site();
    until(|| !ui.get_action_busy());
    assert!(ui
        .get_notice()
        .contains(Language::Portuguese.text("task_disconnected")));
    ui.invoke_language_changed(Language::French as i32);
    assert!(ui
        .get_notice()
        .contains(Language::French.text("task_disconnected")));
}

#[test]
fn folder_import_preserves_structure_and_destination_change_excludes_previous_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let input_dir = dir.path().join("inputs");
    fs::create_dir_all(input_dir.join("nested")).unwrap();
    fs::create_dir_all(input_dir.join("output")).unwrap();
    common::document(100, false)
        .save(input_dir.join("same.pdf"))
        .unwrap();
    common::document(100, false)
        .save(input_dir.join("nested/same.pdf"))
        .unwrap();
    common::document(100, false)
        .save(input_dir.join("output/old.pdf"))
        .unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    ui.set_recursive(true);
    picker
        .folders
        .lock()
        .unwrap()
        .push_back(Some(input_dir.clone()));
    ui.invoke_add_folder();
    until(|| !ui.get_dialog_open());
    assert_eq!(ui.get_rows().row_count(), 3);
    choose_folder(&ui, &picker, &input_dir.join("output"));
    assert_eq!(ui.get_rows().row_count(), 2);
    ui.invoke_start();
    until(|| !ui.get_busy());
    assert!(input_dir.join("output/same-compressed.pdf").exists());
    assert!(input_dir.join("output/nested/same-compressed.pdf").exists());
    assert!(!input_dir.join("output/old-compressed.pdf").exists());
    assert!(ui.get_status().contains("Items not added: 1"));
    drop_paths(&ui, std::slice::from_ref(&input_dir));
    assert_eq!(ui.get_rows().row_count(), 2);
    ui.invoke_select_row(1);
    assert!(ui.get_selected_detail().contains(
        std::path::Path::new("nested")
            .join("same.pdf")
            .to_str()
            .unwrap()
    ));
    ui.hide().unwrap();
}

#[test]
fn compact_list_renders_large_batch_and_last_item_details_are_accessible() {
    slint::platform::set_platform(Box::new(Headless)).unwrap();
    let ui = create_window(
        Arc::new(ScriptedPicker::default()),
        None,
        Arc::new(MockDesktop::default()),
    )
    .unwrap();
    ui.invoke_language_changed(Language::Portuguese as i32);
    let state = State {
        language: Language::Portuguese,
        entries: (0..2000)
            .map(|i| crate::gui_state::Entry {
                path: PathBuf::from(format!(
                    "/synthetic/Relatório muito longo para conferir listas grandes número {i}.pdf"
                )),
                bytes: 2048,
                root: None,
                status: crate::gui_state::RowStatus::Selected,
            })
            .collect(),
        ..Default::default()
    };
    let rows = Rc::new(VecModel::default());
    ui.set_rows(rows.clone().into());
    refresh(&ui, &state, &rows);
    ui.show().unwrap();
    ui.window().set_size(slint::PhysicalSize::new(680, 800));
    ui.set_selected_row(1999);
    refresh_detail(&ui, &state);
    assert!(ui.get_selected_detail().contains("1999.pdf"));
    assert_eq!(rows.row_count(), 2000);
    let snapshot = ui.window().take_snapshot().unwrap();
    assert_eq!(snapshot.width(), 680);
    if let Some(path) = std::env::var_os("PDFTRIMLY_TEST_SCREENSHOTS") {
        use std::io::Write;
        let path = PathBuf::from(path);
        fs::create_dir_all(&path).unwrap();
        let mut file = fs::File::create(path.join("large-list.ppm")).unwrap();
        write!(file, "P6\n680 800\n255\n").unwrap();
        let bytes: Vec<u8> = snapshot
            .as_slice()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b])
            .collect();
        file.write_all(&bytes).unwrap();
    }
    ui.hide().unwrap();
}

#[test]
fn removing_an_earlier_row_keeps_the_selected_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<_> = ["a.pdf", "b.pdf", "c.pdf"]
        .into_iter()
        .map(|name| {
            let path = dir.path().join(name);
            fs::write(&path, b"discovery fixture").unwrap();
            path
        })
        .collect();
    let ui = window(Arc::new(ScriptedPicker::default()));
    drop_paths(&ui, &paths);
    ui.invoke_select_row(2);
    assert!(ui.get_selected_detail().contains("c.pdf"));
    ui.invoke_remove(0);
    assert_eq!(ui.get_selected_row(), 1);
    assert!(ui.get_selected_detail().contains("c.pdf"));
    ui.invoke_remove(1);
    assert_eq!(ui.get_selected_row(), 0);
    assert!(ui.get_selected_detail().contains("b.pdf"));
    ui.invoke_clear_files();
    assert_eq!(ui.get_selected_row(), -1);
    drop_paths(&ui, &paths);
    assert_eq!(ui.get_selected_row(), 0);
    assert!(ui.get_selected_detail().contains("a.pdf"));
}

#[test]
fn output_suffix_follows_language_at_batch_start() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("report.pdf");
    common::document(100, false).save(&input).unwrap();
    let picker = Arc::new(ScriptedPicker::default());
    let ui = window(picker.clone());
    drop_paths(&ui, &[input]);
    choose_folder(&ui, &picker, dir.path());
    for (language, suffix) in [
        (Language::English, "compressed"),
        (Language::Portuguese, "compactado"),
        (Language::French, "compresse"),
        (Language::Spanish, "comprimido"),
        (Language::Italian, "compresso"),
    ] {
        ui.invoke_language_changed(language as i32);
        ui.invoke_start();
        until(|| !ui.get_busy());
        assert!(dir.path().join(format!("report-{suffix}.pdf")).exists());
    }
}

#[test]
fn no_destination_saves_beside_each_pdf_including_subfolders() {
    let dir = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();
    for folder in ["one", "two/nested"] {
        let parent = dir.path().join(folder);
        fs::create_dir_all(&parent).unwrap();
        let input = parent.join("report.pdf");
        common::document(100, false).save(&input).unwrap();
        paths.push(input);
    }
    let originals: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    let ui = window(Arc::new(ScriptedPicker::default()));
    ui.set_recursive(true);
    drop_paths(&ui, &[dir.path().into()]);
    assert!(!ui.get_has_destination());
    ui.invoke_start();
    until(|| !ui.get_busy());
    for (input, original) in paths.iter().zip(originals) {
        assert!(input
            .parent()
            .unwrap()
            .join("report-compressed.pdf")
            .exists());
        assert_eq!(fs::read(input).unwrap(), original);
    }
    ui.invoke_start();
    until(|| !ui.get_busy());
    for input in paths {
        assert!(input
            .parent()
            .unwrap()
            .join("report-compressed-1.pdf")
            .exists());
    }
}
