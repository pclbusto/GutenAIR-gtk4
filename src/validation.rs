use crate::prelude::*;

pub(crate) fn show_epub_check(state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("epub-check: {}", e);
            return;
        }
    };

    let (errors, orphans) = core.validate_integrity_deep();

    let dialog = adw::Window::builder()
        .title("Verificación del EPUB")
        .modal(true)
        .transient_for(&state.window)
        .default_width(480)
        .default_height(400)
        .build();

    let hbar = HeaderBar::new();
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let inner = Box::new(Orientation::Vertical, 0);

    if errors.is_empty() && orphans.is_empty() {
        let ok_group = PreferencesGroup::builder()
            .title("Sin problemas")
            .description("El EPUB no tiene errores de integridad ni archivos huérfanos.")
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        inner.append(&ok_group);
    } else {
        if !errors.is_empty() {
            let err_group = PreferencesGroup::builder()
                .title(&format!("Errores del manifiesto ({})", errors.len()))
                .description("Archivos referenciados en el OPF que no existen en disco.")
                .margin_start(12)
                .margin_end(12)
                .margin_top(12)
                .margin_bottom(6)
                .build();
            for msg in &errors {
                let row = ActionRow::builder().title(msg.as_str()).build();
                let icon = Image::from_icon_name("dialog-error-symbolic");
                icon.add_css_class("error");
                row.add_prefix(&icon);
                err_group.add(&row);
            }
            inner.append(&err_group);
        }

        if !orphans.is_empty() {
            let orp_group = PreferencesGroup::builder()
                .title(&format!("Archivos huérfanos ({})", orphans.len()))
                .description("Archivos en disco que no están registrados en el manifiesto.")
                .margin_start(12)
                .margin_end(12)
                .margin_top(6)
                .margin_bottom(24)
                .build();
            for path in &orphans {
                let name = path.to_string_lossy();
                let row = ActionRow::builder().title(name.as_ref()).build();
                let icon = Image::from_icon_name("dialog-warning-symbolic");
                icon.add_css_class("warning");
                row.add_prefix(&icon);
                orp_group.add(&row);
            }
            inner.append(&orp_group);
        }
    }

    scrolled.set_child(Some(&inner));
    vbox.append(&scrolled);
    dialog.set_content(Some(&vbox));
    dialog.present();
}
