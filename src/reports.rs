use crate::prelude::*;

pub(crate) fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

pub(crate) fn show_chapter_report(state: &Rc<UiState>) {
    let (path, item_id) = match (
        state.current_path.borrow().clone(),
        state.open_item_id.borrow().clone(),
    ) {
        (Some(p), Some(id)) => (p, id),
        _ => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("chapter report: {}", e);
            return;
        }
    };

    let stats = match core.get_chapter_stats(&item_id) {
        Ok(s) => s,
        Err(_) => return,
    };

    let reading_time = {
        let total_sec = (stats.reading_time_min * 60.0) as usize;
        if total_sec == 0 {
            "< 1 seg".to_string()
        } else if total_sec < 60 {
            format!("{} seg", total_sec)
        } else {
            format!("{} min {} seg", total_sec / 60, total_sec % 60)
        }
    };

    let dialog = adw::Window::builder()
        .title("Informe del Capítulo")
        .modal(true)
        .transient_for(&state.window)
        .default_width(380)
        .resizable(false)
        .build();

    let hbar = HeaderBar::new();
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let make_row = |title: &str, value: String| -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        let lbl = Label::new(Some(&value));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("numeric");
        lbl.set_valign(gtk::Align::Center);
        row.add_suffix(&lbl);
        row
    };

    let content_group = PreferencesGroup::builder()
        .title("Contenido")
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(6)
        .build();
    content_group.add(&make_row("Palabras", format_number(stats.word_count)));
    content_group.add(&make_row(
        "Caracteres sin espacios",
        format_number(stats.characters_no_spaces),
    ));
    content_group.add(&make_row(
        "Caracteres con espacios",
        format_number(stats.characters_with_spaces),
    ));
    content_group.add(&make_row("Párrafos", format_number(stats.paragraph_count)));
    content_group.add(&make_row("Tiempo de lectura", reading_time));
    vbox.append(&content_group);

    let file_group = PreferencesGroup::builder()
        .title("Archivo")
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(24)
        .build();
    file_group.add(&make_row("Líneas", format_number(stats.line_count)));
    file_group.add(&make_row(
        "Caracteres totales",
        format_number(stats.total_file_size_chars),
    ));
    vbox.append(&file_group);

    dialog.set_content(Some(&vbox));
    dialog.present();
}

pub(crate) fn show_book_report(state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("book report: {}", e);
            return;
        }
    };

    let stats = match core.get_book_stats() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("book report: {}", e);
            return;
        }
    };

    let reading_time = {
        let total_sec = (stats.total_reading_time_min * 60.0) as usize;
        if total_sec < 60 {
            format!("{} seg", total_sec)
        } else if total_sec < 3600 {
            format!("{} min {} seg", total_sec / 60, total_sec % 60)
        } else {
            format!("{} h {} min", total_sec / 3600, (total_sec % 3600) / 60)
        }
    };

    let dialog = adw::Window::builder()
        .title("Informe del Libro")
        .modal(true)
        .transient_for(&state.window)
        .default_width(380)
        .resizable(false)
        .build();

    let hbar = HeaderBar::new();
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let make_row = |title: &str, value: String| -> ActionRow {
        let row = ActionRow::builder().title(title).build();
        let lbl = Label::new(Some(&value));
        lbl.add_css_class("dim-label");
        lbl.add_css_class("numeric");
        lbl.set_valign(gtk::Align::Center);
        row.add_suffix(&lbl);
        row
    };

    let group = PreferencesGroup::builder()
        .title("Estadísticas del libro")
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(24)
        .build();
    group.add(&make_row("Capítulos", format_number(stats.chapter_count)));
    group.add(&make_row(
        "Palabras totales",
        format_number(stats.total_word_count),
    ));
    group.add(&make_row(
        "Caracteres totales",
        format_number(stats.total_characters),
    ));
    group.add(&make_row(
        "Párrafos totales",
        format_number(stats.total_paragraph_count),
    ));
    group.add(&make_row("Tiempo de lectura", reading_time));
    vbox.append(&group);

    dialog.set_content(Some(&vbox));
    dialog.present();
}
