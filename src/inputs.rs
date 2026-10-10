//! Discovery runs together with the picker, never on the graphical thread.
use super::*;
pub(super) fn install(
    ui: &MainWindow,
    shared: SharedState,
    rows: Rows,
    picker: Arc<dyn FilePicker>,
) {
    let timer = Rc::new(slint::Timer::default());
    let weak = ui.as_weak();
    let state = shared.clone();
    let model = rows.clone();
    let service = picker.clone();
    let file_timer = timer.clone();
    ui.on_add_files(move || {
        let Some(ui) = weak.upgrade() else { return };
        let destination = state.borrow().destination.clone();
        let recursive = ui.get_recursive();
        let service = service.clone();
        start_dialog(
            &ui,
            state.clone(),
            model.clone(),
            file_timer.clone(),
            move || {
                service
                    .files()
                    .map(|paths| crate::discovery::scan(paths, destination, recursive))
            },
            |state, report| state.add_discovery(report),
        );
    });
    let weak = ui.as_weak();
    let state = shared.clone();
    let model = rows.clone();
    let folder_timer = timer.clone();
    ui.on_add_folder(move || {
        let Some(ui) = weak.upgrade() else { return };
        let destination = state.borrow().destination.clone();
        let recursive = ui.get_recursive();
        let service = picker.clone();
        start_dialog(
            &ui,
            state.clone(),
            model.clone(),
            folder_timer.clone(),
            move || {
                service
                    .folder()
                    .map(|path| crate::discovery::scan(vec![path], destination, recursive))
            },
            |state, report| state.add_discovery(report),
        );
    });
    ui.on_can_accept_files(|transfer| {
        transfer.file_paths().is_ok_and(|mut paths| {
            paths.any(|path| {
                path.is_dir()
                    || path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
            })
        })
    });
    let weak = ui.as_weak();
    // Winit emits one event per file. Coalesce events before discovery disables input.
    use slint::winit_030::{winit::event::WindowEvent, EventResult, WinitWindowAccessor};
    let pending = Rc::new(RefCell::new(Vec::<PathBuf>::new()));
    let drop_timer = Rc::new(slint::Timer::default());
    let native_weak = ui.as_weak();
    ui.window().on_winit_window_event(move |_, event| {
        if let WindowEvent::DroppedFile(path) = event {
            if let Some(ui) = native_weak.upgrade() {
                if editable(&ui) {
                    pending.borrow_mut().push(path.clone());
                    let pending = pending.clone();
                    let weak = ui.as_weak();
                    drop_timer.start(
                        slint::TimerMode::SingleShot,
                        Duration::from_millis(50),
                        move || {
                            let paths = std::mem::take(&mut *pending.borrow_mut());
                            if let Some(ui) = weak.upgrade() {
                                let mut transfer = slint::DataTransfer::default();
                                transfer.set_file_paths(&paths);
                                ui.invoke_add_dropped_files(transfer);
                            }
                        },
                    );
                }
            }
        }
        EventResult::Propagate
    });
    ui.on_add_dropped_files(move |transfer| {
        let Some(ui) = weak.upgrade() else { return };
        if !editable(&ui) {
            return;
        }
        let Ok(paths) = transfer.file_paths() else {
            return;
        };
        let paths = paths.map(PathBuf::from).collect();
        let destination = shared.borrow().destination.clone();
        let recursive = ui.get_recursive();
        ui.set_dialog_open(true);
        let state = shared.clone();
        let model = rows.clone();
        ui_tasks::start(
            &ui,
            timer.clone(),
            move || Ok(crate::discovery::scan(paths, destination, recursive)),
            move |ui, result| {
                ui.set_dialog_open(false);
                let mut state = state.borrow_mut();
                match result {
                    Ok(report) => state.add_discovery(report),
                    Err(_) => state.status = Status::Key("dialog_failed"),
                }
                refresh(ui, &state, &model);
            },
        );
    });
}
