use crate::prelude::*;

pub(crate) fn setup_actions(app: &Application) {
    let action_about = gio::SimpleAction::new("about", None);
    action_about.connect_activate(move |_, _| show_about());
    app.add_action(&action_about);

    let action_pref = gio::SimpleAction::new("preferences", None);
    app.add_action(&action_pref);

    let action_help = gio::SimpleAction::new("help", None);
    action_help.connect_activate(|_, _| {
        let _ = gtk::show_uri(
            gtk::Window::NONE,
            "https://github.com",
            gtk::gdk::CURRENT_TIME,
        );
    });
    app.add_action(&action_help);
}

pub(crate) fn show_about() {
    let about = AboutWindow::builder()
        .application_name("GutenAIR")
        .application_icon("com.gutenair.gtk4")
        .developer_name("GutenAIR Team")
        .version("0.1.0")
        .website("https://github.com/pedro/GutenAIR")
        .copyright("© 2026 GutenAIR Team")
        .issue_url("https://github.com/pedro/GutenAIR/issues")
        .license_type(gtk::License::MitX11)
        .build();

    about.add_credit_section(Some("Desarrollo"), &["Pedro"]);
    about.add_credit_section(Some("Asistente de Diseño"), &["Antigravity AI"]);

    if let Some(win) = Application::default().active_window() {
        about.set_transient_for(Some(&win));
    }
    about.present();
}

// ─── History ─────────────────────────────────────────────────────────────────
