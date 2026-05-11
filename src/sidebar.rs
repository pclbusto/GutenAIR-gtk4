use crate::prelude::*;

pub(crate) fn folder_display_name(folder: &str) -> &str {
    match folder {
        "Text" => "Texto",
        "Styles" => "Estilos",
        "Images" => "Imágenes",
        "Fonts" => "Fuentes",
        "Audio" => "Audio",
        "Video" => "Video",
        "Misc" => "Miscelánea",
        other => other,
    }
}

pub(crate) fn icon_for_media_type(media_type: &str) -> &'static str {
    if media_type.contains("xhtml") || media_type.contains("html") {
        "text-x-generic-symbolic"
    } else if media_type.contains("css") {
        "text-x-script-symbolic"
    } else if media_type.starts_with("image/") {
        "image-x-generic-symbolic"
    } else if media_type.starts_with("font/")
        || media_type.contains("opentype")
        || media_type.contains("truetype")
    {
        "font-x-generic-symbolic"
    } else if media_type.starts_with("audio/") {
        "audio-x-generic-symbolic"
    } else if media_type.starts_with("video/") {
        "video-x-generic-symbolic"
    } else {
        "text-x-generic-symbolic"
    }
}

pub(crate) fn update_group_visuals(group_rows: &Rc<RefCell<GroupRows>>, state: &Rc<UiState>) {
    let sel = state.selected_items.borrow();
    let selected_ids: std::collections::HashSet<&str> =
        sel.iter().map(|(_, id)| id.as_str()).collect();
    for (id, _row, check_icon) in group_rows.borrow().iter() {
        check_icon.set_visible(selected_ids.contains(id.as_str()));
    }
}

/// Reemplaza los `<link rel="stylesheet">` del <head> con los que indica el config.
/// Solo toca esas líneas; el resto del contenido queda intacto.

pub(crate) fn show_context_popover(parent: gtk::Widget, x: f64, y: f64, state: &Rc<UiState>) {
    let sel_count = state.selected_items.borrow().len();
    if sel_count == 0 {
        return;
    }

    let popover = Popover::new();
    popover.set_parent(&parent);
    popover.set_has_arrow(false);

    let rect = gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
    popover.set_pointing_to(Some(&rect));

    let vbox = Box::new(Orientation::Vertical, 0);

    let label = Label::builder()
        .label(&format!("{} archivo(s) seleccionado(s)", sel_count))
        .margin_start(8)
        .margin_end(8)
        .margin_top(6)
        .margin_bottom(6)
        .xalign(0.0)
        .build();
    label.add_css_class("caption");
    label.add_css_class("dim-label");
    vbox.append(&label);

    let sep = gtk::Separator::new(Orientation::Horizontal);
    vbox.append(&sep);

    let rename_btn = Button::builder()
        .label("Renombrar selección…")
        .has_frame(false)
        .margin_start(4)
        .margin_end(4)
        .margin_top(4)
        .margin_bottom(4)
        .build();

    let state_c = state.clone();
    let popover_c = popover.clone();
    rename_btn.connect_clicked(move |_| {
        popover_c.popdown();
        show_rename_dialog(&state_c);
    });

    vbox.append(&rename_btn);

    // Opciones de ítem único
    if sel_count == 1 {
        let (_, item_id) = state.selected_items.borrow()[0].clone();
        let media_type = state
            .current_path
            .borrow()
            .as_ref()
            .and_then(|p| gutencore::GutenCore::open_folder(p).ok())
            .and_then(|core| core.manifest.get(&item_id).map(|i| i.media_type.clone()))
            .unwrap_or_default();
        let is_xhtml = media_type.contains("html") || media_type.contains("xhtml");
        let is_image = media_type.starts_with("image/");

        // "Establecer como portada" — solo para imágenes
        if is_image {
            let sep2 = gtk::Separator::new(Orientation::Horizontal);
            vbox.append(&sep2);

            let cover_btn = Button::builder()
                .label("Establecer como portada")
                .has_frame(false)
                .margin_start(4)
                .margin_end(4)
                .margin_top(4)
                .margin_bottom(4)
                .build();

            let state_c = state.clone();
            let popover_c = popover.clone();
            let item_id_c = item_id.clone();
            cover_btn.connect_clicked(move |_| {
                popover_c.popdown();
                let path = match state_c.current_path.borrow().clone() {
                    Some(p) => p,
                    None => return,
                };
                let mut core = match gutencore::GutenCore::open_folder(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("set_cover: {}", e);
                        return;
                    }
                };
                let img_path = match core.get_resource_path(&item_id_c) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("set_cover get_path: {}", e);
                        return;
                    }
                };
                if let Err(e) = core.set_cover(&img_path) {
                    eprintln!("set_cover ERROR: {}", e);
                    return;
                }
                if let Err(e) = core.save() {
                    eprintln!("set_cover save ERROR: {}", e);
                    return;
                }
                if let Ok(fresh_core) = gutencore::GutenCore::open_folder(&path) {
                    populate_sidebar(&state_c, &fresh_core);
                }
            });

            vbox.append(&cover_btn);
        }

        // "Pegado especial" — solo para un único item xhtml
        if is_xhtml {
            let sep2 = gtk::Separator::new(Orientation::Horizontal);
            vbox.append(&sep2);

            let paste_btn = Button::builder()
                .label("Pegado especial")
                .has_frame(false)
                .margin_start(4)
                .margin_end(4)
                .margin_top(4)
                .margin_bottom(4)
                .build();

            let state_c = state.clone();
            let popover_c = popover.clone();
            paste_btn.connect_clicked(move |_| {
                popover_c.popdown();

                let item_id = item_id.clone();
                let state_c = state_c.clone();

                let clipboard = gtk::gdk::Display::default()
                    .expect("no display")
                    .clipboard();

                clipboard.read_text_async(gio::Cancellable::NONE, move |res| {
                    let text = match res {
                        Ok(Some(t)) if !t.is_empty() => t.to_string(),
                        _ => return,
                    };
                    let path = match state_c.current_path.borrow().clone() {
                        Some(p) => p,
                        None => return,
                    };
                    let core = match gutencore::GutenCore::open_folder(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("paste especial: {}", e);
                            return;
                        }
                    };
                    let item = match core.manifest.get(&item_id) {
                        Some(i) => i,
                        None => return,
                    };
                    let title = std::path::Path::new(&item.href)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&item_id)
                        .to_string();
                    let xhtml = core.text_to_xhtml(&text, &title);

                    if let Ok(resource_path) = core.get_resource_path(&item_id) {
                        if std::fs::write(&resource_path, &xhtml).is_ok() {
                            // Si el archivo está abierto en el editor, recargarlo
                            if state_c.open_item_id.borrow().as_deref() == Some(&item_id) {
                                let buffer = state_c.editor.buffer();
                                if let Ok(b) = buffer.downcast::<sourceview5::Buffer>() {
                                    b.set_text(&xhtml);
                                }
                            }
                        }
                    }
                });
            });

            vbox.append(&paste_btn);
        }
    }

    // "Gestionar estilos" — visible cuando toda la selección son capítulos XHTML
    {
        let all_xhtml = state
            .current_path
            .borrow()
            .as_ref()
            .and_then(|p| gutencore::GutenCore::open_folder(p).ok())
            .map(|core| {
                state.selected_items.borrow().iter().all(|(_, id)| {
                    core.manifest
                        .get(id)
                        .map(|item| item.media_type.contains("html"))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);

        if all_xhtml {
            let sep_s = gtk::Separator::new(Orientation::Horizontal);
            vbox.append(&sep_s);

            let styles_btn = Button::builder()
                .label("Gestionar estilos…")
                .has_frame(false)
                .margin_start(4)
                .margin_end(4)
                .margin_top(4)
                .margin_bottom(4)
                .build();

            let state_c = state.clone();
            let popover_c = popover.clone();
            styles_btn.connect_clicked(move |_| {
                popover_c.popdown();
                show_style_manager_dialog(&state_c);
            });

            vbox.append(&styles_btn);
        }
    }

    // Separador y botón eliminar (siempre visible)
    {
        let sep_del = gtk::Separator::new(Orientation::Horizontal);
        vbox.append(&sep_del);

        let del_btn = Button::builder()
            .label(if sel_count == 1 {
                "Eliminar archivo"
            } else {
                "Eliminar archivos"
            })
            .has_frame(false)
            .margin_start(4)
            .margin_end(4)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        del_btn.add_css_class("destructive-action");

        let state_c = state.clone();
        let popover_c = popover.clone();
        let selected = state.selected_items.borrow().clone();
        del_btn.connect_clicked(move |_| {
            popover_c.popdown();
            show_delete_confirm_dialog(&state_c, selected.clone());
        });

        vbox.append(&del_btn);
    }

    popover.set_child(Some(&vbox));
    popover.popup();
}

pub(crate) fn show_style_manager_dialog(state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };
    let selected_ids: Vec<String> = state
        .selected_items
        .borrow()
        .iter()
        .map(|(_, id)| id.clone())
        .collect();
    if selected_ids.is_empty() {
        return;
    }

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("style_manager: {}", e);
            return;
        }
    };

    // Collect CSS files from manifest, sorted by filename
    let mut css_entries: Vec<(String, String)> = core
        .manifest
        .values()
        .filter(|item| item.media_type == "text/css")
        .map(|item| {
            let name = std::path::Path::new(&item.href)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&item.id)
                .to_string();
            (item.id.clone(), name)
        })
        .collect();
    css_entries.sort_by(|a, b| a.1.cmp(&b.1));

    // Compute initial tri-state for each CSS
    let initial_states: Vec<TriState> = css_entries
        .iter()
        .map(|(css_id, _)| {
            let count_with = selected_ids
                .iter()
                .filter(|ch| core.get_chapter_styles(ch).contains(css_id))
                .count();
            if count_with == 0 {
                TriState::None
            } else if count_with == selected_ids.len() {
                TriState::All
            } else {
                TriState::Mixed
            }
        })
        .collect();

    // Window
    let win = adw::Window::builder()
        .title(if selected_ids.len() == 1 {
            "Gestionar estilos".to_string()
        } else {
            format!("Gestionar estilos — {} capítulos", selected_ids.len())
        })
        .transient_for(&state.window)
        .modal(true)
        .default_width(360)
        .build();

    let outer = Box::new(Orientation::Vertical, 0);

    let header = HeaderBar::new();
    outer.append(&header);

    let content = Box::new(Orientation::Vertical, 12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);

    if css_entries.is_empty() {
        let lbl = Label::builder()
            .label("No hay archivos CSS en este proyecto.")
            .wrap(true)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .vexpand(true)
            .build();
        lbl.add_css_class("dim-label");
        content.append(&lbl);
        outer.append(&content);
        win.set_content(Some(&outer));
        win.present();
        return;
    }

    let list = ListBox::new();
    list.add_css_class("boxed-list");
    list.set_selection_mode(gtk::SelectionMode::None);

    // Per-CSS mutable state (None=0, All=1, Mixed=2)
    let tri_states: Vec<Rc<RefCell<TriState>>> = initial_states
        .iter()
        .map(|s| Rc::new(RefCell::new(*s)))
        .collect();

    fn apply_tristate_to_check(check: &gtk::CheckButton, ts: TriState) {
        match ts {
            TriState::All => {
                check.set_inconsistent(false);
                check.set_active(true);
            }
            TriState::None => {
                check.set_inconsistent(false);
                check.set_active(false);
            }
            TriState::Mixed => {
                check.set_active(false);
                check.set_inconsistent(true);
            }
        }
    }

    for (i, (css_id, css_name)) in css_entries.iter().enumerate() {
        let row = ActionRow::builder()
            .title(css_name.as_str())
            .subtitle(css_id.as_str())
            .activatable(true)
            .build();

        let check = gtk::CheckButton::new();
        apply_tristate_to_check(&check, initial_states[i]);
        row.add_prefix(&check);

        let ts_ref = tri_states[i].clone();
        let check_c = check.clone();

        // Intercept toggle: drive state manually
        check.connect_toggled(move |_| {
            let next = match *ts_ref.borrow() {
                TriState::None => TriState::All,
                TriState::All => TriState::None,
                TriState::Mixed => TriState::All,
            };
            *ts_ref.borrow_mut() = next;
            apply_tristate_to_check(&check_c, next);
        });

        // Clicking the row also toggles
        let ts_row = tri_states[i].clone();
        let check_row = check.clone();
        row.connect_activated(move |_| {
            let next = match *ts_row.borrow() {
                TriState::None => TriState::All,
                TriState::All => TriState::None,
                TriState::Mixed => TriState::All,
            };
            *ts_row.borrow_mut() = next;
            apply_tristate_to_check(&check_row, next);
        });

        list.append(&row);
    }

    let scroll = ScrolledWindow::builder()
        .child(&list)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .max_content_height(400)
        .propagate_natural_height(true)
        .build();
    content.append(&scroll);

    // Apply button
    let apply_btn = Button::builder()
        .label("Aplicar")
        .halign(gtk::Align::End)
        .build();
    apply_btn.add_css_class("suggested-action");
    content.append(&apply_btn);

    outer.append(&content);
    win.set_content(Some(&outer));

    let state_c = state.clone();
    let css_entries_c = css_entries.clone();
    let selected_ids_c = selected_ids.clone();
    let win_c = win.clone();
    apply_btn.connect_clicked(move |_| {
        let path = match state_c.current_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        let mut core = match gutencore::GutenCore::open_folder(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("style_manager apply: {}", e);
                return;
            }
        };

        for (i, (css_id, _)) in css_entries_c.iter().enumerate() {
            let target = *tri_states[i].borrow();
            for ch_id in &selected_ids_c {
                match target {
                    TriState::All => {
                        let mut styles = core.get_chapter_styles(ch_id);
                        if !styles.contains(css_id) {
                            styles.push(css_id.clone());
                            core.config.exceptions.insert(ch_id.clone(), styles);
                        }
                    }
                    TriState::None => {
                        let _ = core.remove_style_from_chapter(ch_id, css_id);
                    }
                    TriState::Mixed => {} // sin cambio
                }
            }
        }

        if let Err(e) = core.save() {
            eprintln!("style_manager save: {}", e);
        }
        win_c.close();
    });

    win.present();
}

pub(crate) fn show_delete_confirm_dialog(state: &Rc<UiState>, items: Vec<(String, String)>) {
    let count = items.len();
    let names: Vec<String> = items.iter().map(|(name, _)| name.clone()).collect();

    let dialog = adw::AlertDialog::builder()
        .heading(if count == 1 {
            format!("¿Eliminar \"{}\"?", names[0])
        } else {
            format!("¿Eliminar {} archivos?", count)
        })
        .body(if count == 1 {
            "Esta acción no se puede deshacer.".to_string()
        } else {
            format!(
                "Se eliminarán: {}.\nEsta acción no se puede deshacer.",
                names.join(", ")
            )
        })
        .build();

    dialog.add_response("cancel", "Cancelar");
    dialog.add_response("delete", "Eliminar");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let state_c = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "delete" {
            return;
        }

        let path = match state_c.current_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        let mut core = match gutencore::GutenCore::open_folder(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("delete: {}", e);
                return;
            }
        };

        let open_id = state_c.open_item_id.borrow().clone();
        let mut cleared_editor = false;

        for (_, item_id) in &items {
            if let Err(e) = core.delete_item(item_id) {
                eprintln!("delete {}: {}", item_id, e);
            }
            if open_id.as_deref() == Some(item_id) {
                cleared_editor = true;
            }
        }

        if let Err(e) = core.save() {
            eprintln!("delete save: {}", e);
        }

        if cleared_editor {
            if let Ok(buffer) = state_c.editor.buffer().downcast::<sourceview5::Buffer>() {
                buffer.set_text("");
            }
            state_c
                .image_viewer
                .set_paintable(gtk::gdk::Paintable::NONE);
            state_c.main_stack.set_visible_child_name("editor");
            *state_c.open_item_id.borrow_mut() = None;
            *state_c.open_item_media_type.borrow_mut() = None;
            state_c.header_title.set_subtitle("");
            state_c.stats_btn.set_sensitive(false);
        }

        state_c.selected_items.borrow_mut().clear();

        if let Ok(fresh_core) = gutencore::GutenCore::open_folder(&path) {
            populate_sidebar(&state_c, &fresh_core);
        }
    });

    dialog.present(Some(&state.window));
}

pub(crate) fn show_rename_dialog(state: &Rc<UiState>) {
    let selected = state.selected_items.borrow().clone();
    if selected.is_empty() {
        return;
    }

    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };
    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rename: {}", e);
            return;
        }
    };

    // Sort selected by spine order
    let spine = core.get_spine().clone();
    let mut sel_sorted = selected.clone();
    sel_sorted.sort_by(|(_, id_a), (_, id_b)| {
        let pa = spine.iter().position(|r| r == id_a);
        let pb = spine.iter().position(|r| r == id_b);
        match (pa, pb) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => id_a.cmp(id_b),
        }
    });

    // (id, folder, old_stem, ext)
    let items: Vec<(String, String, String, String)> = sel_sorted
        .iter()
        .filter_map(|(folder, id)| {
            core.manifest.get(id).map(|item| {
                let filename = Path::new(&item.href)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&item.href)
                    .to_string();
                let stem = Path::new(&filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&filename)
                    .to_string();
                let ext = Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!(".{}", e))
                    .unwrap_or_default();
                (id.clone(), folder.clone(), stem, ext)
            })
        })
        .collect();

    if items.is_empty() {
        return;
    }

    let dialog = gtk::Dialog::builder()
        .title("Renombrar archivos")
        .modal(true)
        .default_width(520)
        .transient_for(&state.window)
        .build();

    let content = dialog.content_area();
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_spacing(10);

    // Controls row
    let controls = Box::new(Orientation::Horizontal, 8);

    let prefix_label = Label::builder().label("Prefijo:").build();
    let prefix_entry = Entry::builder()
        .placeholder_text("capitulo_")
        .hexpand(true)
        .build();

    let start_label = Label::builder().label("Inicio:").build();
    let start_adj = gtk::Adjustment::new(1.0, 0.0, 9999.0, 1.0, 10.0, 0.0);
    let start_spin = SpinButton::new(Some(&start_adj), 1.0, 0);
    start_spin.set_width_chars(5);

    let digits_label = Label::builder().label("Dígitos:").build();
    let digits_adj = gtk::Adjustment::new(2.0, 1.0, 6.0, 1.0, 1.0, 0.0);
    let digits_spin = SpinButton::new(Some(&digits_adj), 1.0, 0);
    digits_spin.set_width_chars(4);

    controls.append(&prefix_label);
    controls.append(&prefix_entry);
    controls.append(&start_label);
    controls.append(&start_spin);
    controls.append(&digits_label);
    controls.append(&digits_spin);
    content.append(&controls);

    // Column headers
    let header_box = Box::new(Orientation::Horizontal, 0);
    header_box.set_margin_start(12);
    header_box.set_margin_end(12);
    let old_hdr = Label::builder()
        .label("Nombre actual")
        .hexpand(true)
        .xalign(0.0)
        .build();
    let new_hdr = Label::builder()
        .label("Nombre nuevo")
        .hexpand(true)
        .xalign(0.0)
        .build();
    old_hdr.add_css_class("heading");
    new_hdr.add_css_class("heading");
    header_box.append(&old_hdr);
    header_box.append(&new_hdr);
    content.append(&header_box);

    // Preview list
    let preview_list = ListBox::new();
    preview_list.add_css_class("boxed-list");
    preview_list.set_selection_mode(gtk::SelectionMode::None);

    let preview_scrolled = ScrolledWindow::builder()
        .child(&preview_list)
        .vexpand(true)
        .min_content_height(180)
        .build();
    content.append(&preview_scrolled);

    let items_rc = Rc::new(items);

    // Live preview update closure
    let update_preview: Rc<dyn Fn()> = {
        let items_rc = items_rc.clone();
        let preview_list = preview_list.clone();
        let prefix_entry = prefix_entry.clone();
        let start_spin = start_spin.clone();
        let digits_spin = digits_spin.clone();

        Rc::new(move || {
            while let Some(child) = preview_list.first_child() {
                preview_list.remove(&child);
            }
            let prefix = prefix_entry.text().to_string();
            let start = start_spin.value_as_int();
            let digits = digits_spin.value_as_int() as usize;

            for (i, (_, _, old_stem, ext)) in items_rc.iter().enumerate() {
                let num = start + i as i32;
                let new_stem = format!("{}{:0>width$}", prefix, num, width = digits);
                let new_name = format!("{}{}", new_stem, ext);
                let old_name = format!("{}{}", old_stem, ext);

                let row = gtk::ListBoxRow::new();
                row.set_activatable(false);
                let hbox = Box::new(Orientation::Horizontal, 8);
                hbox.set_margin_top(8);
                hbox.set_margin_bottom(8);
                hbox.set_margin_start(12);
                hbox.set_margin_end(12);

                let old_lbl = Label::builder()
                    .label(&old_name)
                    .hexpand(true)
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build();
                let arrow = Label::builder().label("→").build();
                arrow.add_css_class("dim-label");
                let new_lbl = Label::builder()
                    .label(&new_name)
                    .hexpand(true)
                    .xalign(0.0)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build();
                new_lbl.add_css_class("accent");

                hbox.append(&old_lbl);
                hbox.append(&arrow);
                hbox.append(&new_lbl);
                row.set_child(Some(&hbox));
                preview_list.append(&row);
            }
        })
    };

    update_preview();

    prefix_entry.connect_changed({
        let up = update_preview.clone();
        move |_| up()
    });
    start_spin.connect_value_changed({
        let up = update_preview.clone();
        move |_| up()
    });
    digits_spin.connect_value_changed({
        let up = update_preview.clone();
        move |_| up()
    });

    dialog.add_button("Cancelar", ResponseType::Cancel);
    dialog.add_button("Renombrar", ResponseType::Accept);

    let state_d = state.clone();
    dialog.connect_response(move |d, res| {
        if res == ResponseType::Accept {
            let prefix = prefix_entry.text().to_string();
            let start = start_spin.value_as_int();
            let digits = digits_spin.value_as_int() as usize;

            if let Some(p) = state_d.current_path.borrow().clone() {
                match gutencore::GutenCore::open_folder(&p) {
                    Ok(mut core) => {
                        let mut renames = HashMap::new();
                        for (i, (id, folder, _, ext)) in items_rc.iter().enumerate() {
                            let num = start + i as i32;
                            let new_stem = format!("{}{:0>width$}", prefix, num, width = digits);
                            let new_filename = format!("{}{}", new_stem, ext);
                            let new_href = format!("{}/{}", folder, new_filename);
                            renames.insert(id.clone(), new_href);
                        }
                        match core.rename_files(renames) {
                            Ok(_) => match core.save() {
                                Ok(_) => {
                                    state_d.selected_items.borrow_mut().clear();
                                    *state_d.last_clicked.borrow_mut() = None;
                                    refresh_sidebar(&state_d);
                                }
                                Err(e) => eprintln!("rename: save() falló: {}", e),
                            },
                            Err(e) => eprintln!("rename: rename_files() falló: {}", e),
                        }
                    }
                    Err(e) => eprintln!("rename: open_folder() falló: {}", e),
                }
            }
        }
        d.destroy();
    });

    dialog.show();
}

pub(crate) fn build_css_rows(
    list_box: &ListBox,
    css_state: &Rc<RefCell<Vec<(String, String, bool)>>>,
) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    let items = css_state.borrow().clone();
    let n = items.len();
    for (i, (_, css_name, checked)) in items.iter().enumerate() {
        let row = ActionRow::builder()
            .title(css_name.as_str())
            .activatable(false)
            .build();

        let check = gtk::CheckButton::builder()
            .active(*checked)
            .valign(gtk::Align::Center)
            .build();
        {
            let css_state_c = css_state.clone();
            check.connect_toggled(move |btn| {
                css_state_c.borrow_mut()[i].2 = btn.is_active();
            });
        }
        row.add_prefix(&check);

        if i > 0 {
            let up_btn = Button::from_icon_name("go-up-symbolic");
            up_btn.add_css_class("flat");
            up_btn.set_valign(gtk::Align::Center);
            let css_state_c = css_state.clone();
            let list_box_c = list_box.clone();
            up_btn.connect_clicked(move |_| {
                css_state_c.borrow_mut().swap(i, i - 1);
                build_css_rows(&list_box_c, &css_state_c);
            });
            row.add_suffix(&up_btn);
        }

        if i < n - 1 {
            let down_btn = Button::from_icon_name("go-down-symbolic");
            down_btn.add_css_class("flat");
            down_btn.set_valign(gtk::Align::Center);
            let css_state_c = css_state.clone();
            let list_box_c = list_box.clone();
            down_btn.connect_clicked(move |_| {
                css_state_c.borrow_mut().swap(i, i + 1);
                build_css_rows(&list_box_c, &css_state_c);
            });
            row.add_suffix(&down_btn);
        }

        list_box.append(&row);
    }
}

pub(crate) fn show_default_styles_popover(btn: &Button, state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };
    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("default_styles popover: {}", e);
            return;
        }
    };

    // Build list: default_styles items first (in config order), then non-default CSS
    let mut css_list: Vec<(String, String, bool)> = Vec::new();
    for css_id in &core.config.default_styles {
        if let Some(item) = core.manifest.get(css_id) {
            let name = Path::new(&item.href)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(css_id)
                .to_string();
            css_list.push((css_id.clone(), name, true));
        }
    }
    for (id, item) in &core.manifest {
        if item.media_type == "text/css" && !core.config.default_styles.contains(id) {
            let name = Path::new(&item.href)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(id)
                .to_string();
            css_list.push((id.clone(), name, false));
        }
    }
    let auto_inject_init = core.config.auto_inject;

    let css_state: Rc<RefCell<Vec<(String, String, bool)>>> = Rc::new(RefCell::new(css_list));

    let popover = Popover::new();
    popover.set_parent(btn);
    popover.set_has_arrow(true);

    let outer = Box::new(Orientation::Vertical, 10);
    outer.set_margin_start(10);
    outer.set_margin_end(10);
    outer.set_margin_top(10);
    outer.set_margin_bottom(10);
    outer.set_width_request(300);

    let title_lbl = Label::builder()
        .label("Estilos predeterminados")
        .halign(gtk::Align::Start)
        .build();
    title_lbl.add_css_class("heading");
    outer.append(&title_lbl);

    let hint_lbl = Label::builder()
        .label("Aplicados a todos los capítulos sin excepción. El orden determina la cascada CSS.")
        .halign(gtk::Align::Start)
        .wrap(true)
        .build();
    hint_lbl.add_css_class("caption");
    hint_lbl.add_css_class("dim-label");
    outer.append(&hint_lbl);

    let list_box = ListBox::new();
    list_box.add_css_class("boxed-list");
    list_box.set_selection_mode(gtk::SelectionMode::None);
    build_css_rows(&list_box, &css_state);

    let scroll = ScrolledWindow::builder()
        .child(&list_box)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .max_content_height(280)
        .build();
    outer.append(&scroll);

    // Auto-inject toggle
    let inject_box = Box::new(Orientation::Horizontal, 8);
    let inject_lbl = Label::builder()
        .label("Inyección automática")
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();
    let inject_sw = Switch::builder()
        .active(auto_inject_init)
        .valign(gtk::Align::Center)
        .build();
    inject_box.append(&inject_lbl);
    inject_box.append(&inject_sw);
    outer.append(&inject_box);

    // Apply button
    let apply_btn = Button::builder()
        .label("Aplicar")
        .halign(gtk::Align::End)
        .build();
    apply_btn.add_css_class("suggested-action");
    outer.append(&apply_btn);

    popover.set_child(Some(&outer));

    let state_c = state.clone();
    let css_state_c = css_state.clone();
    let popover_c = popover.clone();
    apply_btn.connect_clicked(move |_| {
        let path = match state_c.current_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        let mut core = match gutencore::GutenCore::open_folder(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("default_styles apply: {}", e);
                return;
            }
        };
        let new_defaults: Vec<String> = css_state_c
            .borrow()
            .iter()
            .filter(|(_, _, checked)| *checked)
            .map(|(id, _, _)| id.clone())
            .collect();
        core.config.default_styles = new_defaults;
        core.config.auto_inject = inject_sw.is_active();
        if let Err(e) = core.save() {
            eprintln!("default_styles save: {}", e);
        }
        popover_c.popdown();
    });

    popover.popup();
}

pub(crate) fn populate_sidebar(state: &Rc<UiState>, core: &gutencore::GutenCore) {
    let sidebar_box = &state.sidebar_box;
    let settings = &state.settings;
    while let Some(child) = sidebar_box.first_child() {
        sidebar_box.remove(&child);
    }

    let configured: Vec<String> = settings
        .strv("sidebar-groups")
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let hidden: Vec<String> = settings
        .strv("hidden-sidebar-groups")
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let mut groups: BTreeMap<String, Vec<gutencore::ManifestItem>> = BTreeMap::new();
    for item in core.manifest.values() {
        let folder = Path::new(&item.href)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        groups.entry(folder).or_default().push(item.clone());
    }

    // Settings is the source of truth: show every configured, non-hidden group,
    // even if empty — the user needs to see the folder to be able to import into it.
    let ordered: Vec<String> = configured
        .iter()
        .filter(|c| !hidden.contains(*c))
        .cloned()
        .collect();

    if ordered.is_empty() {
        return;
    }

    let list = ListBox::new();
    list.add_css_class("boxed-list");
    list.set_margin_start(12);
    list.set_margin_end(12);
    list.set_margin_top(12);
    list.set_margin_bottom(12);
    list.set_selection_mode(gtk::SelectionMode::None);

    for folder in &ordered {
        // Case-insensitive match against actual EPUB folders (handles imported books)
        let key = groups
            .keys()
            .find(|g| g.eq_ignore_ascii_case(folder))
            .cloned();
        let mut items = key.and_then(|k| groups.remove(&k)).unwrap_or_default();

        let spine = core.get_spine();
        items.sort_by(|a, b| {
            let pos_a = spine.iter().position(|r| r == &a.id);
            let pos_b = spine.iter().position(|r| r == &b.id);
            match (pos_a, pos_b) {
                (Some(pa), Some(pb)) => pa.cmp(&pb),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.href.cmp(&b.href),
            }
        });

        let expander = ExpanderRow::builder()
            .title(folder_display_name(folder))
            .expanded(true)
            .build();

        let search_btn = Button::from_icon_name("system-search-symbolic");
        search_btn.add_css_class("flat");
        search_btn.add_css_class("circular");
        search_btn.set_valign(gtk::Align::Center);
        expander.add_suffix(&search_btn);

        let add_btn = Button::from_icon_name("list-add-symbolic");
        add_btn.add_css_class("flat");
        add_btn.add_css_class("circular");
        add_btn.set_valign(gtk::Align::Center);
        expander.add_suffix(&add_btn);

        let del_section_btn = Button::from_icon_name("list-remove-symbolic");
        del_section_btn.add_css_class("flat");
        del_section_btn.add_css_class("circular");
        del_section_btn.set_valign(gtk::Align::Center);
        expander.add_suffix(&del_section_btn);

        let styles_config_btn: Option<Button> = if folder.eq_ignore_ascii_case("styles") {
            let btn = Button::from_icon_name("emblem-system-symbolic");
            btn.add_css_class("flat");
            btn.add_css_class("circular");
            btn.set_valign(gtk::Align::Center);
            expander.add_suffix(&btn);
            Some(btn)
        } else {
            None
        };

        let search_entry = SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_margin_start(6);
        search_entry.set_margin_end(6);
        search_entry.set_margin_top(4);
        search_entry.set_margin_bottom(4);

        let search_revealer = gtk::Revealer::builder()
            .child(&search_entry)
            .reveal_child(false)
            .transition_type(gtk::RevealerTransitionType::SlideDown)
            .build();

        expander.add_row(&search_revealer);

        let group_rows: Rc<RefCell<GroupRows>> = Rc::new(RefCell::new(Vec::new()));

        for item in &items {
            let filename = Path::new(&item.href)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&item.href)
                .to_string();

            let action_row = ActionRow::builder()
                .title(filename.as_str())
                .activatable(true)
                .build();

            // Check icon for selection state (hidden by default)
            let check_icon = Image::from_icon_name("object-select-symbolic");
            check_icon.set_pixel_size(16);
            let is_selected = state
                .selected_items
                .borrow()
                .iter()
                .any(|(_, id)| id == &item.id);
            check_icon.set_visible(is_selected);
            action_row.add_prefix(&check_icon);

            let file_icon = Image::from_icon_name(icon_for_media_type(&item.media_type));
            file_icon.set_pixel_size(16);
            action_row.add_prefix(&file_icon);

            if core.spine.contains(&item.id) {
                let badge = Label::builder().label("lectura").build();
                badge.add_css_class("caption");
                badge.add_css_class("dim-label");
                badge.set_valign(gtk::Align::Center);
                action_row.add_suffix(&badge);

                // Drag source
                let drag_source = gtk::DragSource::builder()
                    .actions(gtk::gdk::DragAction::MOVE)
                    .build();
                let dragged_id = item.id.clone();
                drag_source.connect_prepare(move |_, _, _| {
                    let value = dragged_id.to_value();
                    let provider = gtk::gdk::ContentProvider::for_value(&value);
                    Some(provider)
                });
                action_row.add_controller(drag_source);

                // Drop target
                let drop_target =
                    gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
                let state_drop = state.clone();
                let target_id = item.id.clone();
                drop_target.connect_drop(move |_, value, _, _| {
                    let dropped_id = match value.get::<String>() {
                        Ok(id) => id,
                        Err(e) => {
                            eprintln!("DnD: error leyendo valor: {}", e);
                            return false;
                        }
                    };
                    if dropped_id == target_id {
                        return false;
                    }
                    let path = match state_drop.current_path.borrow().clone() {
                        Some(p) => p,
                        None => return false,
                    };
                    let mut core = match gutencore::GutenCore::open_folder(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("DnD: error abriendo libro: {}", e);
                            return false;
                        }
                    };
                    let spine = core.get_spine().clone();
                    let from = match spine.iter().position(|r| r == &dropped_id) {
                        Some(p) => p,
                        None => return false,
                    };
                    let to = match spine.iter().position(|r| r == &target_id) {
                        Some(p) => p,
                        None => return false,
                    };
                    let insert_at = if from < to { to } else { to };
                    if let Err(e) = core.spine_move(&dropped_id, insert_at) {
                        eprintln!("DnD: spine_move falló: {}", e);
                        return false;
                    }
                    if let Err(e) = core.save() {
                        eprintln!("DnD: save() falló: {}", e);
                        return false;
                    }
                    refresh_sidebar(&state_drop);
                    true
                });
                action_row.add_controller(drop_target);
            }

            // Unified click handler (left + right)
            {
                let gesture = GestureClick::new();
                gesture.set_button(0); // all buttons

                let state_g = state.clone();
                let item_id = item.id.clone();
                let folder_g = folder.clone();
                let group_rows_g = group_rows.clone();
                let item_media = item.media_type.clone();

                gesture.connect_released(move |gest, _n, x, y| {
                    let button = gest.current_button();
                    let modifiers = gest.current_event_state();
                    let ctrl = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK);
                    let shift = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);

                    match button {
                        1 => {
                            if ctrl {
                                {
                                    let mut sel = state_g.selected_items.borrow_mut();
                                    if let Some(pos) = sel.iter().position(|(_, id)| id == &item_id)
                                    {
                                        sel.remove(pos);
                                    } else {
                                        // Enforce same-folder selection
                                        if sel.iter().any(|(f, _)| f != &folder_g) {
                                            sel.clear();
                                        }
                                        sel.push((folder_g.clone(), item_id.clone()));
                                    }
                                }
                                *state_g.last_clicked.borrow_mut() =
                                    Some((folder_g.clone(), item_id.clone()));
                                update_group_visuals(&group_rows_g, &state_g);
                            } else if shift {
                                let anchor = state_g.last_clicked.borrow().clone();
                                match anchor {
                                    Some((ref anchor_folder, ref anchor_id))
                                        if anchor_folder == &folder_g =>
                                    {
                                        let gi = group_rows_g.borrow();
                                        let pa = gi.iter().position(|(id, _, _)| id == anchor_id);
                                        let pc = gi.iter().position(|(id, _, _)| id == &item_id);
                                        if let (Some(pa), Some(pc)) = (pa, pc) {
                                            let (lo, hi) =
                                                if pa <= pc { (pa, pc) } else { (pc, pa) };
                                            let mut sel = state_g.selected_items.borrow_mut();
                                            // Keep only same-folder, clear cross-folder
                                            sel.retain(|(f, _)| f == &folder_g);
                                            for i in lo..=hi {
                                                let id = gi[i].0.clone();
                                                if !sel.iter().any(|(_, sid)| sid == &id) {
                                                    sel.push((folder_g.clone(), id));
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        // Different folder or no anchor: start fresh
                                        state_g.selected_items.borrow_mut().clear();
                                        state_g
                                            .selected_items
                                            .borrow_mut()
                                            .push((folder_g.clone(), item_id.clone()));
                                        *state_g.last_clicked.borrow_mut() =
                                            Some((folder_g.clone(), item_id.clone()));
                                    }
                                }
                                update_group_visuals(&group_rows_g, &state_g);
                            } else {
                                // Normal click: clear selection, open file
                                state_g.selected_items.borrow_mut().clear();
                                *state_g.last_clicked.borrow_mut() =
                                    Some((folder_g.clone(), item_id.clone()));
                                update_group_visuals(&group_rows_g, &state_g);
                                open_item(&state_g, &item_id, &item_media);
                            }
                        }
                        3 => {
                            // Right click: select item if not already selected, then show menu
                            {
                                let mut sel = state_g.selected_items.borrow_mut();
                                if !sel.iter().any(|(_, id)| id == &item_id) {
                                    if sel.iter().any(|(f, _)| f != &folder_g) {
                                        sel.clear();
                                    }
                                    sel.push((folder_g.clone(), item_id.clone()));
                                    *state_g.last_clicked.borrow_mut() =
                                        Some((folder_g.clone(), item_id.clone()));
                                }
                            }
                            update_group_visuals(&group_rows_g, &state_g);
                            if let Some(widget) = gest.widget() {
                                show_context_popover(widget, x, y, &state_g);
                            }
                        }
                        _ => {}
                    }
                });

                action_row.add_controller(gesture);
            }

            expander.add_row(&action_row);
            group_rows
                .borrow_mut()
                .push((item.id.clone(), action_row, check_icon));
        }

        // Toggle search entry on button click
        {
            let rev = search_revealer.clone();
            let entry = search_entry.clone();
            let rows = group_rows.clone();
            search_btn.connect_clicked(move |_| {
                let reveal = !rev.reveals_child();
                rev.set_reveal_child(reveal);
                if reveal {
                    entry.grab_focus();
                } else {
                    entry.set_text("");
                    for (_, row, _) in rows.borrow().iter() {
                        row.set_visible(true);
                    }
                }
            });
        }

        // Filter rows as user types
        {
            let rows = group_rows.clone();
            search_entry.connect_search_changed(move |entry| {
                let query = entry.text().to_lowercase();
                for (_, row, _) in rows.borrow().iter() {
                    let title = row.title().to_lowercase();
                    row.set_visible(query.is_empty() || title.contains(&query));
                }
            });
        }

        // Escape closes search and restores all rows
        {
            let rows = group_rows.clone();
            let rev = search_revealer.clone();
            search_entry.connect_stop_search(move |entry| {
                entry.set_text("");
                rev.set_reveal_child(false);
                for (_, row, _) in rows.borrow().iter() {
                    row.set_visible(true);
                }
            });
        }

        // (+) button: popover with "Nuevo" and "Importar…"
        {
            let folder_c = folder.clone();
            let state_c = state.clone();
            add_btn.connect_clicked(move |btn| {
                let popover = Popover::new();
                popover.set_parent(btn);
                popover.set_has_arrow(true);

                let vbox = Box::new(Orientation::Vertical, 0);
                vbox.set_margin_start(4);
                vbox.set_margin_end(4);
                vbox.set_margin_top(4);
                vbox.set_margin_bottom(4);

                let (new_mime, new_label) = match folder_c.to_lowercase().as_str() {
                    "text" => (Some("application/xhtml+xml"), "Nuevo capítulo"),
                    "styles" => (Some("text/css"), "Nueva hoja de estilo"),
                    _ => (None, ""),
                };

                if let Some(mime) = new_mime {
                    let nuevo_btn = Button::builder()
                        .label(new_label)
                        .has_frame(false)
                        .halign(gtk::Align::Fill)
                        .build();
                    let state_cc = state_c.clone();
                    let folder_cc = folder_c.clone();
                    let mime_s = mime.to_string();
                    let popover_c = popover.clone();
                    nuevo_btn.connect_clicked(move |_| {
                        popover_c.popdown();
                        if folder_cc.eq_ignore_ascii_case("text") {
                            show_add_chapters_dialog(&state_cc.window, &state_cc);
                        } else {
                            let label = folder_display_name(&folder_cc);
                            show_add_resource_dialog(
                                &state_cc.window,
                                &state_cc,
                                label,
                                &folder_cc,
                                &mime_s,
                            );
                        }
                    });
                    vbox.append(&nuevo_btn);
                    vbox.append(&gtk::Separator::new(Orientation::Horizontal));
                }

                let import_btn = Button::builder()
                    .label("Importar…")
                    .has_frame(false)
                    .halign(gtk::Align::Fill)
                    .build();
                let state_cc = state_c.clone();
                let folder_cc = folder_c.clone();
                let popover_c = popover.clone();
                import_btn.connect_clicked(move |_| {
                    popover_c.popdown();
                    if folder_cc.eq_ignore_ascii_case("text") {
                        show_import_chapters_dialog(&state_cc.window, &state_cc);
                    } else {
                        let label = folder_display_name(&folder_cc);
                        show_import_dialog(&state_cc.window, &state_cc, label, &folder_cc, "");
                    }
                });
                vbox.append(&import_btn);

                popover.set_child(Some(&vbox));
                popover.popup();
            });
        }

        // (-) button: delete current selection
        {
            let state_c = state.clone();
            del_section_btn.connect_clicked(move |_| {
                let selected = state_c.selected_items.borrow().clone();
                if !selected.is_empty() {
                    show_delete_confirm_dialog(&state_c, selected);
                }
            });
        }

        // (⚙) button: manage default_styles config (Styles section only)
        if let Some(cfg_btn) = styles_config_btn {
            let state_c = state.clone();
            cfg_btn.connect_clicked(move |btn| {
                show_default_styles_popover(btn, &state_c);
            });
        }

        list.append(&expander);
    }

    sidebar_box.append(&list);
}

// ─── Book loading ─────────────────────────────────────────────────────────────

pub(crate) fn refresh_sidebar(ui_state: &Rc<UiState>) {
    if let Some(path) = ui_state.current_path.borrow().clone() {
        if let Ok(core) = gutencore::GutenCore::open_folder(&path) {
            populate_sidebar(ui_state, &core);
        }
    }
}
