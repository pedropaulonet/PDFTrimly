//! Run short I/O actions off the GUI thread and publish their result once.
use super::*;
pub(super) fn start<T: Send + 'static>(
    ui: &MainWindow,
    timer: Rc<slint::Timer>,
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
    done: impl Fn(&MainWindow, Result<T, String>) + 'static,
) {
    let (sender, receiver) = mpsc::channel();
    if let Err(error) = std::thread::Builder::new()
        .name("pdftrimly-desktop".into())
        .spawn(move || {
            let _ = sender.send(work());
        })
    {
        done(ui, Err(error.to_string()));
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
                Err(mpsc::TryRecvError::Disconnected) => Err("Background task disconnected".into()),
            };
            if let Some(timer) = keep.upgrade() {
                timer.stop();
            }
            done(&ui, response);
        },
    );
}
