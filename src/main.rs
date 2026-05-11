mod app;
mod book;
mod constants;
mod editor;
mod export;
mod i18n;
mod nav;
mod preferences;
mod prelude;
mod reports;
mod resources;
mod sidebar;
mod state;
mod ui;
mod validation;

use crate::prelude::*;

fn main() -> adw::glib::ExitCode {
    let application = Application::builder().application_id(APP_ID).build();

    application.connect_startup(setup_actions);
    application.connect_activate(build_ui);
    application.run()
}
