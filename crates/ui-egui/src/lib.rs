//! Folio UI: the launcher's window contents, in egui.
//!
//! [`Launcher`] owns the model and draws it; the desktop shell only has to call
//! [`Launcher::ui`] once per frame.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

mod model;
mod theme;
mod views;

use folio_native::Backend;
pub use model::{AppState, Job, Model};

pub struct Launcher {
    backend: Backend,
    model: Model,
    dark: bool,
}

impl Launcher {
    pub fn new(ctx: &egui::Context) -> Launcher {
        let backend = Backend::new();
        let model = Model::new(backend.platform());
        backend.refresh_all(&folio_core::SUITE);
        let dark = ctx.style().visuals.dark_mode;
        theme::apply(ctx, dark);
        Launcher { backend, model, dark }
    }

    /// Draws one frame and applies whatever the background workers reported.
    pub fn ui(&mut self, ctx: &egui::Context) {
        let events = self.backend.drain();
        let loading = self.model.apps.iter().any(|a| a.latest.is_none());
        let busy = self.model.apply(events) || self.model.any_working() || loading;
        views::show(ctx, self);
        // Poll while lookups or installs are in flight, so their results show up even when the
        // window is otherwise idle; once everything has settled, egui sleeps.
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
    }

    pub(crate) fn toggle_theme(&mut self, ctx: &egui::Context) {
        self.dark = !self.dark;
        theme::apply(ctx, self.dark);
    }
}
