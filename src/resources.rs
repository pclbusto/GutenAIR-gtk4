use crate::prelude::*;

pub(crate) fn show_import_chapters_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path_opt = state.current_path.borrow().clone();
    let Some(path) = path_opt else { return };

    let native = FileChooserNative::new(
        Some("Importar capítulos"),
        Some(parent),
        FileChooserAction::Open,
        Some("Importar"),
        Some("Cancelar"),
    );
    native.set_select_multiple(true);

    let filter_txt = gtk::FileFilter::new();
    filter_txt.set_name(Some("Capítulos (.xhtml, .txt, .md)"));
    filter_txt.add_pattern("*.xhtml");
    filter_txt.add_pattern("*.html");
    filter_txt.add_pattern("*.txt");
    filter_txt.add_pattern("*.md");
    native.add_filter(&filter_txt);

    let state = state.clone();
    native.connect_response(move |n, res| {
        if res == ResponseType::Accept {
            let files = n.files();
            let count = files.n_items();
            if count == 0 {
                n.destroy();
                return;
            }

            match gutencore::GutenCore::open_folder(&path) {
                Err(e) => eprintln!("import chapters: {}", e),
                Ok(mut core) => {
                    let mut imported = 0;
                    let mut errors: Vec<(String, String)> = Vec::new();
                    for i in 0..count {
                        let file = match files.item(i).and_then(|o| o.downcast::<gio::File>().ok())
                        {
                            Some(f) => f,
                            None => continue,
                        };
                        let file_path = match file.path() {
                            Some(p) => p,
                            None => continue,
                        };
                        let ext = file_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let stem = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("capitulo")
                            .to_string();
                        let content = match std::fs::read_to_string(&file_path) {
                            Ok(c) => c,
                            Err(e) => {
                                errors.push((stem.clone(), e.to_string()));
                                continue;
                            }
                        };

                        let xhtml = match ext.as_str() {
                            "xhtml" | "html" => content,
                            "txt" => core.text_to_xhtml(&content, &stem),
                            "md" => {
                                errors.push((
                                    stem.clone(),
                                    "formato .md aún no implementado".to_string(),
                                ));
                                continue;
                            }
                            _ => continue,
                        };

                        let base_id = core.sanitize_filename(&stem);
                        // Deduplicate by both ID and generated href (Text/{id}.xhtml)
                        let id = {
                            let mut candidate = base_id.clone();
                            let mut n = 2;
                            loop {
                                let href_candidate = format!("Text/{}.xhtml", candidate);
                                let id_taken = core.manifest.contains_key(&candidate);
                                let href_taken =
                                    core.manifest.values().any(|it| it.href == href_candidate);
                                if !id_taken && !href_taken {
                                    break;
                                }
                                candidate = format!("{}_{}", base_id, n);
                                n += 1;
                            }
                            candidate
                        };

                        match core.add_document(&id, &xhtml) {
                            Ok(_) => {
                                core.spine_insert(id.clone(), None);
                                imported += 1;
                            }
                            Err(e) => errors.push((stem.clone(), e.to_string())),
                        }
                    }

                    if imported > 0 {
                        match core.save() {
                            Ok(_) => refresh_sidebar(&state),
                            Err(e) => errors.push(("Guardar".to_string(), e.to_string())),
                        }
                    }

                    show_import_summary(&state.window, imported, errors);
                }
            }
        }
        n.destroy();
    });

    native.show();
}

pub(crate) fn show_import_summary(
    parent: &adw::ApplicationWindow,
    imported: usize,
    errors: Vec<(String, String)>,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(if errors.is_empty() {
            "Importación completada".to_string()
        } else if imported == 0 {
            "Importación fallida".to_string()
        } else {
            "Importación con advertencias".to_string()
        })
        .build();

    dialog.add_response("ok", "Aceptar");
    dialog.set_default_response(Some("ok"));

    let vbox = Box::new(Orientation::Vertical, 8);

    if imported > 0 {
        let ok_label = Label::builder()
            .label(format!(
                "{} capítulo{} importado{} correctamente.",
                imported,
                if imported == 1 { "" } else { "s" },
                if imported == 1 { "" } else { "s" }
            ))
            .halign(gtk::Align::Start)
            .wrap(true)
            .build();
        ok_label.add_css_class("success");
        vbox.append(&ok_label);
    }

    if !errors.is_empty() {
        let err_label = Label::builder()
            .label(format!(
                "{} error{}:",
                errors.len(),
                if errors.len() == 1 { "" } else { "s" }
            ))
            .halign(gtk::Align::Start)
            .build();
        err_label.add_css_class("heading");
        vbox.append(&err_label);

        let scroll = ScrolledWindow::builder()
            .min_content_height(120)
            .max_content_height(240)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();

        let err_box = Box::new(Orientation::Vertical, 4);
        for (name, msg) in &errors {
            let row_box = Box::new(Orientation::Vertical, 2);

            let name_lbl = Label::builder()
                .label(name)
                .halign(gtk::Align::Start)
                .wrap(true)
                .build();
            name_lbl.add_css_class("caption-heading");

            let msg_lbl = Label::builder()
                .label(msg)
                .halign(gtk::Align::Start)
                .wrap(true)
                .build();
            msg_lbl.add_css_class("caption");
            msg_lbl.add_css_class("dim-label");

            row_box.append(&name_lbl);
            row_box.append(&msg_lbl);
            err_box.append(&row_box);
        }
        scroll.set_child(Some(&err_box));
        vbox.append(&scroll);
    }

    dialog.set_extra_child(Some(&vbox));
    dialog.present(Some(parent));
}

pub(crate) fn show_new_project_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let dialog = gtk::Dialog::builder()
        .title("Nuevo Proyecto")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_spacing(10);

    let title_entry = Entry::builder()
        .placeholder_text("Título del libro")
        .build();
    let lang_entry = Entry::builder()
        .placeholder_text("Idioma (ej: es, en, fr)")
        .text("es")
        .build();

    let folder_row = Box::new(Orientation::Horizontal, 6);
    let folder_entry = Entry::builder()
        .placeholder_text("Carpeta de destino")
        .hexpand(true)
        .build();
    let folder_btn = Button::builder().label("…").build();
    folder_row.append(&folder_entry);
    folder_row.append(&folder_btn);

    // Folder chooser button
    {
        let win = parent.clone().upcast::<gtk::Window>();
        let entry = folder_entry.clone();
        folder_btn.connect_clicked(move |_| {
            let native = FileChooserNative::new(
                Some("Seleccionar carpeta de destino"),
                Some(&win),
                FileChooserAction::SelectFolder,
                Some("Seleccionar"),
                Some("Cancelar"),
            );
            let entry = entry.clone();
            native.connect_response(move |n, res| {
                if res == ResponseType::Accept {
                    if let Some(f) = n.file() {
                        if let Some(p) = f.path() {
                            entry.set_text(&p.to_string_lossy());
                        }
                    }
                }
                n.destroy();
            });
            native.show();
        });
    }

    content.append(&title_entry);
    content.append(&lang_entry);
    content.append(&folder_row);

    dialog.add_button("Cancelar", ResponseType::Cancel);
    dialog.add_button("Crear", ResponseType::Accept);

    let state = state.clone();
    dialog.connect_response(move |d, res| {
        if res == ResponseType::Accept {
            let title = title_entry.text().to_string();
            let lang = lang_entry.text().to_string();
            let folder = folder_entry.text().to_string();

            if title.is_empty() || folder.is_empty() {
                return;
            }
            let lang = if lang.is_empty() {
                "es".to_string()
            } else {
                lang
            };

            match gutencore::GutenCore::new_project(&folder, &title, &lang) {
                Ok(_) => {
                    load_book(&folder, &state);
                }
                Err(e) => eprintln!("Error creando proyecto: {}", e),
            }
        }
        d.destroy();
    });

    dialog.show();
}

pub(crate) fn show_split_chapter_dialog(
    parent: &impl IsA<gtk::Window>,
    state: &Rc<UiState>,
    source_id: &str,
    suggested_split_id: &str,
) {
    let path_opt = state.current_path.borrow().clone();
    let Some(path) = path_opt else { return };

    let dialog = gtk::Dialog::builder()
        .title("Dividir Capítulo")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_spacing(12);

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(vec!["boxed-list".to_string()])
        .build();

    let new_id_row = EntryRow::builder()
        .title("ID del nuevo capítulo")
        .text(&format!("{}_b", source_id))
        .build();

    let split_id_row = EntryRow::builder()
        .title("ID del punto de corte")
        .text(suggested_split_id)
        .build();

    let title_row = EntryRow::builder()
        .title("Título del nuevo capítulo (opcional)")
        .build();

    list.append(&new_id_row);
    list.append(&split_id_row);
    list.append(&title_row);

    content.append(&list);

    dialog.add_button("Cancelar", ResponseType::Cancel);
    dialog.add_button("Dividir", ResponseType::Accept);

    let state = state.clone();
    let source_id = source_id.to_string();
    dialog.connect_response(move |d, res| {
        if res == ResponseType::Accept {
            let new_id = new_id_row.text().to_string();
            let split_id = split_id_row.text().to_string();
            let new_title = title_row.text().to_string();
            let new_title_opt = if new_title.is_empty() {
                None
            } else {
                Some(new_title)
            };

            if !new_id.is_empty() && !split_id.is_empty() {
                if let Ok(mut core) = gutencore::GutenCore::open_folder(&path) {
                    let options = gutencore::SplitChapterOptions {
                        source_id: source_id.clone(),
                        new_id: new_id.clone(),
                        split_at: gutencore::SplitPoint::ElementId(split_id),
                        new_title: new_title_opt,
                    };

                    match core.split_chapter(options) {
                        Ok(_) => {
                            let _ = core.save();
                            refresh_sidebar(&state);
                            load_item_without_saving(&state, &source_id, "application/xhtml+xml");
                        }
                        Err(e) => eprintln!("Error dividiendo capítulo: {}", e),
                    }
                }
            }
        }
        d.destroy();
    });

    dialog.show();
}

pub(crate) fn show_error_dialog(parent: &impl IsA<gtk::Window>, title: &str, message: &str) {
    let dialog = gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(420)
        .build();

    let label = Label::builder()
        .label(message)
        .wrap(true)
        .xalign(0.0)
        .margin_top(16)
        .margin_bottom(16)
        .margin_start(16)
        .margin_end(16)
        .build();

    dialog.content_area().append(&label);
    dialog.add_button("Aceptar", ResponseType::Accept);
    dialog.connect_response(|d, _| d.destroy());
    dialog.show();
}

pub(crate) fn show_add_chapters_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path_opt = state.current_path.borrow().clone();
    let Some(path) = path_opt else { return };

    let dialog = gtk::Dialog::builder()
        .title("Agregar Capítulos")
        .transient_for(parent)
        .modal(true)
        .default_width(400)
        .build();

    let content = dialog.content_area();
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_spacing(12);

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(vec!["boxed-list".to_string()])
        .build();

    let prefix_row = EntryRow::builder()
        .title("Prefijo")
        .text("capitulo_")
        .build();

    let quantity_row = SpinRow::builder()
        .title("Cantidad de capítulos")
        .adjustment(&gtk::Adjustment::new(1.0, 1.0, 1000.0, 1.0, 10.0, 0.0))
        .build();

    let start_row = SpinRow::builder()
        .title("Número inicial")
        .adjustment(&gtk::Adjustment::new(1.0, 0.0, 10000.0, 1.0, 10.0, 0.0))
        .build();

    let digits_row = SpinRow::builder()
        .title("Dígitos de numeración")
        .adjustment(&gtk::Adjustment::new(2.0, 1.0, 10.0, 1.0, 1.0, 0.0))
        .build();

    list.append(&prefix_row);
    list.append(&quantity_row);
    list.append(&start_row);
    list.append(&digits_row);

    content.append(&list);

    dialog.add_button("Cancelar", ResponseType::Cancel);
    dialog.add_button("Agregar", ResponseType::Accept);

    let state = state.clone();
    dialog.connect_response(move |d, res| {
        if res == ResponseType::Accept {
            let prefix = prefix_row.text().to_string();
            let quantity = quantity_row.value() as i32;
            let start = start_row.value() as i32;
            let digits = digits_row.value() as usize;

            if !prefix.is_empty() && quantity > 0 {
                if let Ok(mut core) = gutencore::GutenCore::open_folder(&path) {
                    for i in 0..quantity {
                        let n = start + i;
                        let id = format!("{}{:0width$}", prefix, n, width = digits);
                        let _ = core
                            .add_document(&id, &format!("<h1>Capítulo {}</h1>", n))
                            .and_then(|_| {
                                core.spine_insert(id.clone(), None);
                                Ok(())
                            });
                    }
                    let _ = core.save();
                    refresh_sidebar(&state);
                }
            }
        }
        d.destroy();
    });

    dialog.show();
}

pub(crate) fn show_add_resource_dialog(
    parent: &impl IsA<gtk::Window>,
    state: &Rc<UiState>,
    label: &str,
    _folder: &str,
    mime: &str,
) {
    let path_opt = state.current_path.borrow().clone();
    let Some(path) = path_opt else { return };

    let dialog = gtk::Dialog::builder()
        .title(&format!("Agregar {}", label))
        .transient_for(parent)
        .modal(true)
        .build();

    let content = dialog.content_area();
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_spacing(12);

    let entry = Entry::builder()
        .placeholder_text("ID del recurso (ej: cap2, estilos_nuevos)")
        .build();
    content.append(&entry);

    dialog.add_button("Cancelar", ResponseType::Cancel);
    dialog.add_button("Agregar", ResponseType::Accept);

    let state = state.clone();
    let mime_owned = mime.to_string();
    dialog.connect_response(move |d, res| {
        if res == ResponseType::Accept {
            let id = entry.text().to_string();
            if !id.is_empty() {
                if let Ok(mut core) = gutencore::GutenCore::open_folder(&path) {
                    let result = if mime_owned == "application/xhtml+xml" {
                        core.add_document(&id, "<h1>Nuevo Capítulo</h1>")
                            .and_then(|_| {
                                core.spine_insert(id.clone(), None);
                                Ok(())
                            })
                    } else if mime_owned == "text/css" {
                        core.add_style(&id, "/* Nuevos estilos */")
                    } else {
                        Ok(())
                    };

                    match result {
                        Ok(_) => {
                            let _ = core.save();
                            refresh_sidebar(&state);
                        }
                        Err(e) => eprintln!("Error agregando recurso: {}", e),
                    }
                }
            }
        }
        d.destroy();
    });

    dialog.show();
}

pub(crate) fn mime_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "css" => "text/css",
        "js" => "application/javascript",
        _ => "application/octet-stream",
    }
}

pub(crate) fn show_import_dialog(
    parent: &impl IsA<gtk::Window>,
    state: &Rc<UiState>,
    label: &str,
    folder: &str,
    _mime_filter: &str,
) {
    let path_opt = state.current_path.borrow().clone();
    let Some(path) = path_opt else { return };
    let folder_owned = folder.to_string();

    let native = FileChooserNative::new(
        Some(&format!("Importar {}", label)),
        Some(parent),
        FileChooserAction::Open,
        Some("Importar"),
        Some("Cancelar"),
    );
    native.set_select_multiple(true);

    let state = state.clone();
    native.connect_response(move |n, res| {
        if res == ResponseType::Accept {
            let files = n.files();
            let count = files.n_items();
            if count == 0 {
                n.destroy();
                return;
            }

            if let Ok(mut core) = gutencore::GutenCore::open_folder(&path) {
                let mut imported = 0;
                for i in 0..count {
                    let file = match files.item(i).and_then(|o| o.downcast::<gio::File>().ok()) {
                        Some(f) => f,
                        None => continue,
                    };
                    let src = match file.path() {
                        Some(p) => p,
                        None => continue,
                    };
                    let name = match file.basename() {
                        Some(b) => b,
                        None => continue,
                    };
                    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
                    let mime = mime_for_extension(ext);
                    let target_href = format!("{}/{}", folder_owned, name.to_string_lossy());
                    // Skip if already in manifest
                    if core.manifest.values().any(|it| it.href == target_href) {
                        continue;
                    }
                    let id = core.sanitize_filename(&name.to_string_lossy());
                    if core.import_file(src, id, &target_href, mime).is_ok() {
                        imported += 1;
                    }
                }
                if imported > 0 {
                    let _ = core.save();
                    refresh_sidebar(&state);
                }
            }
        }
        n.destroy();
    });

    native.show();
}
