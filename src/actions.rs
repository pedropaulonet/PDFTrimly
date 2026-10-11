use super::*;
use crate::desktop::{self, Desktop};

pub(super) fn install(ui: &MainWindow, state: SharedState, desktop: Arc<dyn Desktop>) {
    let timer = Rc::new(slint::Timer::default());
    let weak = ui.as_weak();
    let shared = state.clone();
    let service = desktop.clone();
    let copy_timer = timer.clone();
    ui.on_copy_summary(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_action_busy() || ui.get_busy() || !ui.get_has_summary() || ui.get_modal() {
            return;
        }
        let text = shared.borrow().status_text();
        ui.set_action_busy(true);
        let service = service.clone();
        let shared = shared.clone();
        ui_tasks::start(
            &ui,
            copy_timer.clone(),
            move || service.copy(&text),
            move |ui, result| {
                finish(ui, &shared, result, Some("summary_copied"));
            },
        );
    });
    let weak = ui.as_weak();
    let shared = state.clone();
    let service = desktop.clone();
    let folder_timer = timer.clone();
    ui.on_open_output(move || {
        let Some(ui) = weak.upgrade() else { return };
        if ui.get_action_busy() || ui.get_modal() {
            return;
        }
        let Some(path) = shared.borrow().destination.clone() else {
            return;
        };
        ui.set_action_busy(true);
        let service = service.clone();
        let shared = shared.clone();
        ui_tasks::start(
            &ui,
            folder_timer.clone(),
            move || service.folder(&path),
            move |ui, result| {
                finish(ui, &shared, result, None);
            },
        );
    });
    for support in [false, true] {
        let weak = ui.as_weak();
        let shared = state.clone();
        let service = desktop.clone();
        let timer = timer.clone();
        let action = move || {
            let Some(ui) = weak.upgrade() else { return };
            if ui.get_action_busy() {
                return;
            }
            let url = if support {
                desktop::support_url(shared.borrow().language)
            } else {
                desktop::SITE
            };
            ui.set_action_busy(true);
            let service = service.clone();
            let shared = shared.clone();
            ui_tasks::start(
                &ui,
                timer.clone(),
                move || service.link(url),
                move |ui, result| {
                    finish(ui, &shared, result, None);
                },
            );
        };
        if support {
            ui.on_open_support(action);
        } else {
            ui.on_open_site(action);
        }
    }
}
fn finish(
    ui: &MainWindow,
    state: &SharedState,
    result: Result<(), String>,
    success: Option<&'static str>,
) {
    ui.set_action_busy(false);
    let mut state = state.borrow_mut();
    state.notice = match result {
        Ok(()) => success.map(|key| Notice { key, detail: None }),
        Err(detail) => Some(Notice {
            key: "action_failed",
            detail: Some(detail),
        }),
    };
    ui.set_notice(state.notice_text().into());
}
