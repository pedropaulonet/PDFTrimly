//! Preferences editing is separate from batch execution and current selection.
use super::*;
use crate::config::{self, Preferences, Theme};

pub(super) fn apply(ui: &MainWindow, state: &mut State, preferences: &Preferences) {
    state.language = preferences.language.unwrap_or_else(Language::detect);
    state.destination = preferences.folder.clone().filter(|p| p.is_dir());
    state.exclude_destination();
    ui.set_language_index(state.language as i32);
    ui.set_theme_index(preferences.theme_index());
    ui.invoke_apply_theme();
    ui.set_level(preferences.level);
    ui.set_recursive(preferences.recursive);
    apply_labels(ui, state.language);
}

pub(super) fn install(
    ui: &MainWindow,
    state: SharedState,
    rows: Rows,
    picker: Arc<dyn FilePicker>,
    path: Option<PathBuf>,
    saved: Preferences,
) {
    let saved = Rc::new(RefCell::new(saved));
    let timer = Rc::new(slint::Timer::default());
    let folder_timer = Rc::new(slint::Timer::default());
    let weak = ui.as_weak();
    let preferences = saved.clone();
    ui.on_open_settings(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !editable(&ui) {
            return;
        }
        let p = preferences.borrow();
        ui.set_draft_language(p.language.map(|l| l as i32 + 1).unwrap_or(0));
        ui.set_draft_theme(p.theme_index());
        ui.set_draft_level(p.level);
        ui.set_draft_recursive(p.recursive);
        ui.set_draft_folder(
            p.folder
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
                .into(),
        );
        ui.set_settings_notice("".into());
        ui.set_settings_open(true);
        ui.invoke_focus_settings();
    });
    let weak = ui.as_weak();
    ui.on_cancel_settings(move || {
        if let Some(ui) = weak.upgrade() {
            if !ui.get_settings_busy() {
                ui.set_settings_open(false);
                ui.invoke_focus_main();
            }
        }
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_settings_folder(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !ui.get_settings_open() || ui.get_settings_busy() {
            return;
        }
        ui.set_settings_busy(true);
        let picker = picker.clone();
        let shared = shared.clone();
        ui_tasks::start(
            &ui,
            folder_timer.clone(),
            move || Ok(picker.folder()),
            move |ui, result| {
                ui.set_settings_busy(false);
                match result {
                    Ok(Some(path)) => match path.to_str() {
                        Some(path) => ui.set_draft_folder(path.into()),
                        None => ui.set_settings_notice(
                            shared
                                .borrow()
                                .language
                                .text("config_folder_missing")
                                .into(),
                        ),
                    },
                    Ok(None) => {}
                    Err(_) => ui
                        .set_settings_notice(shared.borrow().language.text("dialog_failed").into()),
                }
            },
        );
    });
    let weak = ui.as_weak();
    ui.on_save_settings(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !ui.get_settings_open() || ui.get_settings_busy() {
            return;
        }
        let Some(path) = path.clone() else {
            ui.set_settings_notice(state.borrow().language.text("config_unavailable").into());
            return;
        };
        let p = Preferences {
            language: if ui.get_draft_language() == 0 {
                None
            } else {
                Some(Language::from_index(ui.get_draft_language() - 1))
            },
            theme: match ui.get_draft_theme() {
                1 => Theme::Light,
                2 => Theme::Dark,
                _ => Theme::System,
            },
            level: ui.get_draft_level(),
            recursive: ui.get_draft_recursive(),
            folder: if ui.get_draft_folder().is_empty() {
                None
            } else {
                Some(PathBuf::from(ui.get_draft_folder().as_str()))
            },
        };
        ui.set_settings_busy(true);
        ui.set_settings_notice("".into());
        let shared = state.clone();
        let model = rows.clone();
        let saved = saved.clone();
        ui_tasks::start(
            &ui,
            timer.clone(),
            move || {
                config::save(&path, &p).map_err(|e| e.to_string())?;
                Ok(p)
            },
            move |ui, result| {
                ui.set_settings_busy(false);
                match result {
                    Ok(p) => {
                        *saved.borrow_mut() = p.clone();
                        let mut state = shared.borrow_mut();
                        apply(ui, &mut state, &p);
                        state.notice = Some(Notice {
                            key: "config_saved",
                            detail: None,
                        });
                        refresh(ui, &state, &model);
                        ui.set_settings_open(false);
                        ui.invoke_focus_main();
                    }
                    Err(detail) => ui.set_settings_notice(
                        shared
                            .borrow()
                            .language
                            .format(
                                "config_save_failed",
                                &[(
                                    "detail",
                                    crate::messages::task_detail(shared.borrow().language, &detail),
                                )],
                            )
                            .into(),
                    ),
                }
            },
        );
    });
}
