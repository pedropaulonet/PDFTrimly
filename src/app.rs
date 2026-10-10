use crate::{
    batch::{self, BatchRequest},
    gui_state::{drain_channel, Notice, State, Status},
    i18n::{self, Language},
};
use pdftrimly_core::CompressionLevel;
use slint::{ComponentHandle, Model, VecModel};
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{atomic::Ordering, mpsc, Arc},
    time::Duration,
};
slint::include_modules!();
#[path = "actions.rs"]
mod actions;
#[path = "inputs.rs"]
mod inputs;
#[path = "settings.rs"]
mod settings;
#[path = "ui_tasks.rs"]
mod ui_tasks;

type SharedState = Rc<RefCell<State>>;
type Rows = Rc<VecModel<PdfRow>>;

// Native dialogs stay at the boundary; tests supply responses without a desktop.
trait FilePicker: Send + Sync {
    fn files(&self) -> Option<Vec<PathBuf>>;
    fn folder(&self) -> Option<PathBuf>;
}

struct NativePicker;
impl FilePicker for NativePicker {
    fn files(&self) -> Option<Vec<PathBuf>> {
        rfd::FileDialog::new()
            .add_filter("PDF", &["pdf"])
            .pick_files()
    }
    fn folder(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }
}

fn apply_labels(ui: &MainWindow, language: Language) {
    let labels = ui.global::<Labels>();
    macro_rules! label {
        ($setter:ident, $key:literal) => {
            labels.$setter(language.text($key).into());
        };
    }
    label!(set_add_folder, "add_folder");
    label!(set_recursive, "recursive");
    label!(set_select_details, "select_details");
    label!(set_ignored_items, "ignored_items");
    label!(set_working, "working");
    label!(set_settings, "settings");
    label!(set_save_settings, "save_settings");
    label!(set_cancel, "cancel");
    label!(set_theme, "theme");
    label!(set_system, "system");
    label!(set_theme_light, "theme_light");
    label!(set_theme_dark, "theme_dark");
    label!(set_default_level, "default_level");
    label!(set_default_folder, "default_folder");
    label!(set_no_default_folder, "no_default_folder");
    label!(set_clear_default, "clear_default");
    label!(set_settings_hint, "settings_hint");
    label!(set_copy_summary, "copy_summary");
    label!(set_open_output, "open_output");
    label!(set_site, "site");
    label!(set_support, "support");
    label!(set_destination, "destination");
    label!(set_initial, "initial");
    label!(set_subtitle, "subtitle");
    label!(set_add, "add");
    label!(set_remove, "remove");
    label!(set_clear, "clear");
    label!(set_compression, "compression");
    label!(set_light, "light");
    label!(set_normal, "normal");
    label!(set_ultra, "ultra");
    label!(set_lossless, "lossless");
    label!(set_normal_hint, "normal_hint");
    label!(set_ultra_hint, "ultra_hint");
    label!(set_folder, "folder");
    label!(set_start, "start");
    label!(set_stop, "stop");
    label!(set_about, "about");
    label!(set_close, "close");
    label!(set_language, "language");
    label!(set_drop_hint, "drop_hint");
    label!(set_drop_hover, "drop_hover");
    label!(set_close_question, "close_question");
    label!(set_close_wait, "close_wait");
    label!(set_close_stop, "close_stop");
    label!(set_close_back, "close_back");
    label!(set_empty_list, "empty_list");
    label!(set_privacy, "privacy");
    label!(set_busy_hint, "busy_hint");
    label!(set_progress_hint, "progress_hint");
    ui.set_about_text(
        format!(
            "PDFTrimly {}\n{}\n\n{}\n{}\n\ndev.pedropaulo.net/pdftrimly\nko-fi.com/pedropaulo",
            env!("CARGO_PKG_VERSION"),
            language.text("author"),
            language.text("privacy"),
            language.text("license")
        )
        .into(),
    );
}

fn refresh(ui: &MainWindow, state: &State, rows: &Rows) {
    if matches!(
        state.status,
        Status::Selection { .. } | Status::Key("initial" | "empty_list")
    ) {
        ui.set_progress(0.0);
    }
    while rows.row_count() > state.entries.len() {
        rows.remove(rows.row_count() - 1);
    }
    for (index, entry) in state.entries.iter().enumerate() {
        let row = PdfRow {
            name: entry.name().into(),
            detail: entry.detail(state.language).into(),
            summary: entry.short_status(state.language).into(),
            size: crate::messages::size(
                state.language,
                match &entry.status {
                    crate::gui_state::RowStatus::Finished(Ok((report, _))) => report.input_bytes,
                    _ => entry.bytes,
                },
            )
            .into(),
        };
        if index < rows.row_count() {
            if rows.row_data(index).is_none_or(|old| old != row) {
                rows.set_row_data(index, row);
            }
        } else {
            rows.push(row);
        }
    }
    refresh_detail(ui, state);
    ui.set_notice(state.notice_text().into());
    ui.set_has_summary(matches!(state.status, Status::Summary { .. }));
    ui.set_status(state.status_text().into());
    ui.set_compact_status(state.compact_status().into());
    ui.set_selection_count(state.language.file_count(state.entries.len()).into());
    ui.set_has_destination(state.destination.is_some());
    ui.set_destination(
        state
            .destination
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default()
            .into(),
    );
    ui.set_stop_requested(state.stop.load(Ordering::Acquire));
}

fn refresh_detail(ui: &MainWindow, state: &State) {
    let selected = if state.entries.is_empty() {
        -1
    } else {
        ui.get_selected_row()
            .max(0)
            .min(state.entries.len() as i32 - 1)
    };
    ui.set_selected_row(selected);
    ui.set_selected_detail(
        state
            .entries
            .get(selected as usize)
            .map(|entry| entry.detail(state.language))
            .unwrap_or_else(|| state.language.text("select_details").into())
            .into(),
    );
}

fn editable(ui: &MainWindow) -> bool {
    !ui.get_busy() && !ui.get_dialog_open() && !ui.get_modal()
}

fn start_dialog<T: Send + 'static>(
    ui: &MainWindow,
    shared: SharedState,
    rows: Rows,
    timer: Rc<slint::Timer>,
    picker: impl FnOnce() -> Option<T> + Send + 'static,
    accept: impl Fn(&mut State, T) + 'static,
) {
    if !editable(ui) {
        return;
    }
    ui.set_dialog_open(true);
    let (sender, receiver) = mpsc::channel();
    if std::thread::Builder::new()
        .name("pdftrimly-picker".into())
        .spawn(move || {
            let _ = sender.send(picker());
        })
        .is_err()
    {
        ui.set_dialog_open(false);
        let mut state = shared.borrow_mut();
        state.status = Status::Key("dialog_failed");
        refresh(ui, &state, &rows);
        return;
    }
    let weak = ui.as_weak();
    let keep = Rc::downgrade(&timer);
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(50),
        move || {
            let Some(ui) = weak.upgrade() else {
                if let Some(timer) = keep.upgrade() {
                    timer.stop();
                }
                return;
            };
            let response = match receiver.try_recv() {
                Ok(result) => result,
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    shared.borrow_mut().status = Status::Key("dialog_failed");
                    None
                }
            };
            if let Some(timer) = keep.upgrade() {
                timer.stop();
            }
            ui.set_dialog_open(false);
            let mut state = shared.borrow_mut();
            if let Some(value) = response {
                accept(&mut state, value);
            }
            refresh(&ui, &state, &rows);
        },
    );
}

fn create_window(
    picker: Arc<dyn FilePicker>,
    config_path: Option<PathBuf>,
    desktop: Arc<dyn crate::desktop::Desktop>,
) -> Result<MainWindow, Box<dyn std::error::Error>> {
    i18n::validate_catalogs()?;
    let ui = MainWindow::new()?;
    let (preferences, warning) = crate::config::load(config_path.as_deref());
    let state = Rc::new(RefCell::new(State::default()));
    settings::apply(&ui, &mut state.borrow_mut(), &preferences);
    state.borrow_mut().language = preferences.language();
    let warning = warning.or_else(|| {
        if preferences.folder.as_ref().is_some_and(|p| !p.is_dir()) {
            Some("config_folder_missing")
        } else {
            None
        }
    });
    if let Some(key) = warning {
        state.borrow_mut().notice = Some(Notice { key, detail: None });
    }
    let rows = Rc::new(VecModel::<PdfRow>::default());

    let folder_timer = Rc::new(slint::Timer::default());
    let batch_timer = Rc::new(slint::Timer::default());
    ui.set_rows(rows.clone().into());
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    ui.set_language_index(state.borrow().language as i32);
    apply_labels(&ui, state.borrow().language);
    refresh(&ui, &state.borrow(), &rows);

    settings::install(
        &ui,
        state.clone(),
        rows.clone(),
        picker.clone(),
        config_path,
        preferences,
    );
    actions::install(&ui, state.clone(), desktop);

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_language_changed(move |index| {
        if let Some(ui) = weak.upgrade() {
            let mut state = shared.borrow_mut();
            state.language = Language::from_index(index);
            apply_labels(&ui, state.language);
            refresh(&ui, &state, &model);
        }
    });

    inputs::install(&ui, state.clone(), rows.clone(), picker.clone());
    let weak = ui.as_weak();
    let shared = state.clone();
    ui.on_select_row(move |index| {
        if let Some(ui) = weak.upgrade() {
            if index >= 0 && (index as usize) < shared.borrow().entries.len() {
                ui.set_selected_row(index);
                refresh_detail(&ui, &shared.borrow());
            }
        }
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_choose_folder(move || {
        if let Some(ui) = weak.upgrade() {
            let picker = picker.clone();
            start_dialog(
                &ui,
                shared.clone(),
                model.clone(),
                folder_timer.clone(),
                move || picker.folder(),
                |state, path| {
                    state.destination = Some(path);
                    state.exclude_destination();
                },
            );
        }
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_remove(move |index| {
        let Some(ui) = weak.upgrade() else { return };
        if !editable(&ui) {
            return;
        }
        let mut state = shared.borrow_mut();
        if let Ok(index) = usize::try_from(index) {
            if index < state.entries.len() {
                state.entries.remove(index);
                let selected = ui.get_selected_row();
                if selected > index as i32 {
                    ui.set_selected_row(selected - 1);
                }
            }
        }
        state.status = Status::Key(if state.entries.is_empty() {
            "empty_list"
        } else {
            "initial"
        });
        refresh(&ui, &state, &model);
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_clear_files(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !editable(&ui) {
            return;
        }
        let mut state = shared.borrow_mut();
        state.entries.clear();
        ui.invoke_reset_list_scroll();
        ui.set_detail_open(false);
        state.ignored = 0;
        state.status = Status::Key("empty_list");
        ui.set_progress(0.0);
        refresh(&ui, &state, &model);
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_stop(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !ui.get_busy() {
            return;
        }
        let mut state = shared.borrow_mut();
        state.stop.store(true, Ordering::Release);
        state.status = Status::Key("stopping");
        refresh(&ui, &state, &model);
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_start(move || {
        let Some(ui) = weak.upgrade() else { return };
        if !editable(&ui) {
            return;
        }
        let mut state = shared.borrow_mut();
        let destination = state.destination.clone();
        if state.entries.is_empty() {
            return;
        }
        state.exclude_destination();
        if state.entries.is_empty() {
            state.status = Status::Key("empty_list");
            refresh(&ui, &state, &model);
            return;
        }
        state.begin();
        let request = BatchRequest {
            inputs: state
                .entries
                .iter()
                .map(|entry| batch::BatchInput {
                    path: entry.path.clone(),
                    relative_dir: entry
                        .root
                        .as_ref()
                        .and_then(|root| entry.path.strip_prefix(root).ok())
                        .and_then(std::path::Path::parent)
                        .unwrap_or(std::path::Path::new(""))
                        .to_path_buf(),
                })
                .collect(),
            output_dir: destination,
            suffix: state.language.text("output_suffix").into(),
            level: match ui.get_level() {
                0 => CompressionLevel::Light,
                2 => CompressionLevel::Ultra,
                _ => CompressionLevel::Normal,
            },
        };
        let stop = state.stop.clone();
        ui.set_busy(true);
        ui.set_progress(0.0);
        refresh(&ui, &state, &model);
        drop(state);
        let (sender, receiver) = mpsc::channel();
        if std::thread::Builder::new()
            .name("pdftrimly-batch".into())
            .spawn(move || {
                batch::run_batch(request, stop, |event| {
                    let _ = sender.send(event);
                });
            })
            .is_err()
        {
            shared.borrow_mut().worker_lost();
            ui.set_busy(false);
            refresh(&ui, &shared.borrow(), &model);
            return;
        }
        let weak = weak.clone();
        let shared = shared.clone();
        let model = model.clone();
        let keep = Rc::downgrade(&batch_timer);
        batch_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(50),
            move || {
                let Some(ui) = weak.upgrade() else {
                    if let Some(timer) = keep.upgrade() {
                        timer.stop();
                    }
                    return;
                };
                let mut completed = false;
                let mut received = false;
                let mut state = shared.borrow_mut();
                let disconnected = drain_channel(&receiver, |event| {
                    received = true;
                    completed |= state.apply_event(event);
                });
                if disconnected && !completed {
                    state.worker_lost();
                }
                if received || disconnected {
                    ui.set_progress(
                        state.totals.completed as f32 / state.entries.len().max(1) as f32,
                    );
                    refresh(&ui, &state, &model);
                }
                if completed || disconnected {
                    if let Some(timer) = keep.upgrade() {
                        timer.stop();
                    }
                    ui.set_busy(false);
                    ui.set_close_confirm(false);
                    if state.close_pending {
                        let _ = ui.hide();
                    }
                }
            },
        );
    });

    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_close_wait(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut state = shared.borrow_mut();
        state.close_pending = true;
        ui.set_close_confirm(false);
        if !ui.get_busy() {
            let _ = ui.hide();
            return;
        }
        state.status = Status::Key("close_waiting");
        refresh(&ui, &state, &model);
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_close_stop(move || {
        let Some(ui) = weak.upgrade() else { return };
        let mut state = shared.borrow_mut();
        state.close_pending = true;
        state.stop.store(true, Ordering::Release);
        ui.set_close_confirm(false);
        if !ui.get_busy() {
            let _ = ui.hide();
            return;
        }
        state.status = Status::Key("close_stopping");
        refresh(&ui, &state, &model);
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let model = rows.clone();
    ui.on_close_back(move || {
        if let Some(ui) = weak.upgrade() {
            let mut state = shared.borrow_mut();
            state.cancel_close();
            ui.set_close_confirm(false);
            refresh(&ui, &state, &model);
            ui.invoke_focus_main();
        }
    });
    let weak = ui.as_weak();
    ui.window().on_close_requested(move || {
        if let Some(ui) = weak.upgrade() {
            if ui.get_dialog_open() || ui.get_settings_busy() {
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            if ui.get_busy() {
                ui.set_close_confirm(true);
                ui.invoke_focus_close();
                return slint::CloseRequestResponse::KeepWindowShown;
            }
        }
        slint::CloseRequestResponse::HideWindow
    });
    Ok(ui)
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Winit 0.30 has no file-drop support on Wayland; use XWayland if available.
    #[cfg(target_os = "linux")]
    if std::env::var_os("DISPLAY").is_some() {
        use slint::winit_030::winit::platform::x11::EventLoopBuilderExtX11;
        let mut builder = slint::winit_030::winit::event_loop::EventLoop::<
            slint::winit_030::SlintEvent,
        >::with_user_event();
        builder.with_x11();
        slint::BackendSelector::new()
            .with_winit_event_loop_builder(builder)
            .select()?;
    }

    create_window(
        Arc::new(NativePicker),
        crate::config::config_path(),
        Arc::new(crate::desktop::NativeDesktop::default()),
    )?
    .run()?;
    Ok(())
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
