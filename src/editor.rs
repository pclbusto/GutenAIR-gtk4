use crate::prelude::*;

pub(crate) struct ParagraphSplitTarget {
    pub(crate) paragraph_id: String,
    pub(crate) text_offset: usize,
}

pub(crate) fn char_offset_to_byte(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

pub(crate) fn extract_id_attr(tag: &str) -> Option<String> {
    let re = regex::Regex::new(r#"(?is)\bid\s*=\s*(?:"([^"]+)"|'([^']+)')"#).ok()?;
    let caps = re.captures(tag)?;
    caps.get(1)
        .or_else(|| caps.get(2))
        .map(|m| m.as_str().to_string())
}

pub(crate) fn text_offset_in_xhtml_fragment(fragment: &str, limit: usize) -> Option<usize> {
    let mut idx = 0;
    let mut count = 0;

    while idx < limit {
        let rest = &fragment[idx..];
        let ch = rest.chars().next()?;

        if ch == '<' {
            let end = rest.find('>')?;
            let next = idx + end + 1;
            if next > limit {
                return None;
            }
            idx = next;
        } else if ch == '&' {
            if let Some(end) = rest.find(';') {
                let next = idx + end + 1;
                if next <= limit {
                    count += 1;
                    idx = next;
                    continue;
                }
            }
            count += 1;
            idx += ch.len_utf8();
        } else {
            count += 1;
            idx += ch.len_utf8();
        }
    }

    Some(count)
}

pub(crate) fn find_paragraph_split_target(
    text: &str,
    cursor_char_offset: usize,
) -> Result<ParagraphSplitTarget, String> {
    let cursor_byte = char_offset_to_byte(text, cursor_char_offset);
    let p_re = regex::Regex::new(r#"(?is)<p\b[^>]*>"#).map_err(|e| e.to_string())?;
    let close_p_re = regex::Regex::new(r#"(?is)</p\s*>"#).map_err(|e| e.to_string())?;

    for start_tag in p_re.find_iter(text) {
        if start_tag.end() > cursor_byte {
            break;
        }

        let after_open = &text[start_tag.end()..];
        let Some(close_match) = close_p_re.find(after_open) else {
            continue;
        };
        let close_byte = start_tag.end() + close_match.start();

        if cursor_byte > close_byte {
            continue;
        }

        let tag = &text[start_tag.start()..start_tag.end()];
        let paragraph_id = extract_id_attr(tag)
            .ok_or_else(|| "El párrafo bajo el cursor no tiene atributo id.".to_string())?;
        let inner = &text[start_tag.end()..close_byte];
        let relative_cursor = cursor_byte.saturating_sub(start_tag.end());
        let text_offset =
            text_offset_in_xhtml_fragment(inner, relative_cursor).ok_or_else(|| {
                "El cursor está dentro de una etiqueta XHTML; ponelo en texto del párrafo."
                    .to_string()
            })?;

        return Ok(ParagraphSplitTarget {
            paragraph_id,
            text_offset,
        });
    }

    Err("No encontré un <p id=\"...\"> que contenga el cursor.".to_string())
}

pub(crate) fn split_paragraph_at_cursor(state: &Rc<UiState>) {
    let media_type = state
        .open_item_media_type
        .borrow()
        .clone()
        .unwrap_or_default();
    if !media_type.contains("html") && !media_type.contains("xhtml") {
        show_error_dialog(
            &state.window,
            "Dividir párrafo",
            "Esta acción solo funciona en capítulos XHTML.",
        );
        return;
    }

    let Some(chapter_id) = state.open_item_id.borrow().clone() else {
        show_error_dialog(
            &state.window,
            "Dividir párrafo",
            "No hay un capítulo abierto.",
        );
        return;
    };
    let Some(path) = state.current_path.borrow().clone() else {
        show_error_dialog(
            &state.window,
            "Dividir párrafo",
            "No hay un proyecto abierto.",
        );
        return;
    };

    let buffer = state.editor.buffer();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let text = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string();
    let target = match find_paragraph_split_target(&text, cursor.offset() as usize) {
        Ok(target) => target,
        Err(e) => {
            show_error_dialog(&state.window, "Dividir párrafo", &e);
            return;
        }
    };

    save_current_item(state);

    let mut core = match gutencore::GutenCore::open_folder(&path) {
        Ok(core) => core,
        Err(e) => {
            show_error_dialog(
                &state.window,
                "Dividir párrafo",
                &format!("No se pudo abrir el proyecto: {}", e),
            );
            return;
        }
    };

    let options = gutencore::SplitParagraphOptions {
        chapter_id: chapter_id.clone(),
        paragraph_id: target.paragraph_id,
        text_offset: target.text_offset,
        new_paragraph_id: None,
    };

    if let Err(e) = core.split_paragraph(options) {
        show_error_dialog(&state.window, "Dividir párrafo", &format!("{}", e));
        return;
    }

    let full_path = match core.get_resource_path(&chapter_id) {
        Ok(path) => path,
        Err(e) => {
            show_error_dialog(
                &state.window,
                "Dividir párrafo",
                &format!("No se pudo recargar el capítulo: {}", e),
            );
            return;
        }
    };

    match std::fs::read_to_string(&full_path) {
        Ok(content) => state.editor.buffer().set_text(&content),
        Err(e) => {
            show_error_dialog(
                &state.window,
                "Dividir párrafo",
                &format!("No se pudo recargar el capítulo: {}", e),
            );
            return;
        }
    }

    let uri = glib::filename_to_uri(&full_path, None)
        .unwrap_or_else(|_| format!("file://{}", full_path.to_string_lossy()).into());
    state.preview.load_uri(&uri);
}

pub(crate) fn run_ollama_generation(
    base_url: &str,
    model: &str,
    prompt: &str,
    input_text: &str,
) -> Result<String, String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    println!("[IA] Preparando petición para {} con modelo {}", url, model);

    let full_prompt = if input_text.is_empty() {
        prompt.to_string()
    } else {
        format!(
            "Contexto:\n---\n{}\n---\n\nInstrucción: {}",
            input_text, prompt
        )
    };

    let body = serde_json::json!({
        "model": model,
        "prompt": full_prompt,
        "stream": false
    });

    println!("[IA] Construyendo cliente HTTP...");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120)) // Increased timeout
        .build()
        .map_err(|e| e.to_string())?;

    println!("[IA] Enviando petición a Ollama (esto puede tardar según el modelo)...");
    let response = client.post(&url).json(&body).send().map_err(|e| {
        println!("[IA] Error en el envío: {}", e);
        e.to_string()
    })?;

    println!("[IA] Respuesta recibida con estado: {}", response.status());
    if !response.status().is_success() {
        return Err(format!("Error del servidor: {}", response.status()));
    }

    println!("[IA] Procesando JSON de respuesta...");
    let json: serde_json::Value = response.json().map_err(|e| e.to_string())?;
    let output = json["response"]
        .as_str()
        .ok_or_else(|| "Respuesta inválida de Ollama".to_string())?
        .to_string();

    println!(
        "[IA] Generación completada con éxito ({} caracteres)",
        output.len()
    );
    Ok(output)
}

pub(crate) fn show_ai_dialog(parent: &ApplicationWindow, state: &Rc<UiState>, selected_text: &str) {
    let dialog = AdwWindow::builder()
        .title("Asistente IA")
        .default_width(700)
        .default_height(600)
        .transient_for(parent)
        .modal(true)
        .build();

    let content = Box::new(Orientation::Vertical, 0);

    let header_bar = HeaderBar::new();
    content.append(&header_bar);

    let main_vbox = Box::new(Orientation::Vertical, 12);
    main_vbox.set_margin_top(12);
    main_vbox.set_margin_bottom(12);
    main_vbox.set_margin_start(12);
    main_vbox.set_margin_end(12);
    content.append(&main_vbox);

    // Context / Selection
    let sel_group = PreferencesGroup::builder()
        .title("Contexto (Texto seleccionado)")
        .build();
    let sel_view = gtk::TextView::builder()
        .editable(false)
        .wrap_mode(gtk::WrapMode::WordChar)
        .height_request(100)
        .build();
    sel_view.buffer().set_text(selected_text);
    let sel_scrolled = ScrolledWindow::builder()
        .child(&sel_view)
        .propagate_natural_height(true)
        .min_content_height(100)
        .build();
    sel_group.add(&sel_scrolled);
    main_vbox.append(&sel_group);

    // Prompt
    let prompt_group = PreferencesGroup::builder().title("Instrucción").build();
    let prompt_view = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .height_request(80)
        .build();
    let prompt_scrolled = ScrolledWindow::builder()
        .child(&prompt_view)
        .min_content_height(80)
        .build();
    prompt_group.add(&prompt_scrolled);
    main_vbox.append(&prompt_group);

    // Output
    let out_group = PreferencesGroup::builder()
        .title("Respuesta de la IA")
        .build();
    let out_view = gtk::TextView::builder()
        .wrap_mode(gtk::WrapMode::WordChar)
        .vexpand(true)
        .build();
    let out_scrolled = ScrolledWindow::builder()
        .child(&out_view)
        .vexpand(true)
        .min_content_height(200)
        .build();
    out_group.add(&out_scrolled);
    main_vbox.append(&out_group);

    // Bottom buttons
    let btn_box = Box::new(Orientation::Horizontal, 12);
    btn_box.set_halign(gtk::Align::End);

    let status_label = Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .hexpand(true)
        .build();
    status_label.add_css_class("dim-label");

    let run_btn = Button::builder().label("Generar").build();
    run_btn.add_css_class("suggested-action");

    let apply_btn = Button::builder()
        .label("Aplicar al editor")
        .sensitive(false)
        .build();

    btn_box.append(&status_label);
    btn_box.append(&run_btn);
    btn_box.append(&apply_btn);
    main_vbox.append(&btn_box);

    dialog.set_content(Some(&content));

    // Generation logic
    let state_c = state.clone();
    let prompt_view_c = prompt_view.clone();
    let out_view_c = out_view.clone();
    let status_label_c = status_label.clone();
    let run_btn_c = run_btn.clone();
    let apply_btn_c = apply_btn.clone();
    let input_text = selected_text.to_string();

    run_btn.connect_clicked(move |_| {
        let url = state_c.settings.string("ollama-url").to_string();
        let model = state_c.settings.string("ollama-model").to_string();
        let prompt = prompt_view_c
            .buffer()
            .text(
                &prompt_view_c.buffer().start_iter(),
                &prompt_view_c.buffer().end_iter(),
                false,
            )
            .to_string();

        if url.is_empty() || model.is_empty() {
            status_label_c.set_text("Error: Configura Ollama en Preferencias");
            return;
        }

        if prompt.is_empty() {
            status_label_c.set_text("Error: Escribe una instrucción");
            return;
        }

        status_label_c.set_text("Generando... (ver consola para detalles)");
        run_btn_c.set_sensitive(false);
        apply_btn_c.set_sensitive(false);

        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        let input_text = input_text.clone();

        std::thread::spawn(move || {
            let res = run_ollama_generation(&url, &model, &prompt, &input_text);
            let _ = tx.send(res);
        });

        let out_view = out_view_c.clone();
        let status_label = status_label_c.clone();
        let run_btn = run_btn_c.clone();
        let apply_btn = apply_btn_c.clone();

        glib::idle_add_local(move || match rx.try_recv() {
            Ok(res) => {
                run_btn.set_sensitive(true);
                match res {
                    Ok(text) => {
                        out_view.buffer().set_text(&text);
                        status_label.set_text("Listo");
                        apply_btn.set_sensitive(true);
                    }
                    Err(e) => {
                        status_label.set_text(&format!("Error: {}", e));
                    }
                }
                glib::ControlFlow::Break
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(_) => glib::ControlFlow::Break,
        });
    });

    // Apply logic
    let state_c = state.clone();
    let out_view_c = out_view.clone();
    let dialog_c = dialog.clone();
    apply_btn.connect_clicked(move |_| {
        let buffer = out_view_c.buffer();
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        let editor_buffer = state_c.editor.buffer();
        if editor_buffer.has_selection() {
            editor_buffer.delete_selection(true, true);
        }
        editor_buffer.insert_at_cursor(&text);
        dialog_c.destroy();
    });

    dialog.present();
}

pub(crate) fn selected_editor_text(state: &Rc<UiState>) -> Option<String> {
    let buffer = state.editor.buffer();
    buffer
        .selection_bounds()
        .map(|(start, end)| buffer.text(&start, &end, false).to_string())
}

pub(crate) fn replace_editor_selection(state: &Rc<UiState>, replacement: &str) {
    let buffer = state.editor.buffer();
    if buffer.has_selection() {
        buffer.delete_selection(true, true);
        buffer.insert_at_cursor(replacement);
    }
}

pub(crate) fn show_ai_for_selection(state: &Rc<UiState>) {
    let text = selected_editor_text(state).unwrap_or_default();
    show_ai_dialog(&state.window, state, &text);
}

pub(crate) fn split_chapter_at_cursor(state: &Rc<UiState>) {
    let buffer = state.editor.buffer();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());

    // Buscar el id="..." más cercano hacia atrás desde el cursor.
    let text = buffer
        .text(&buffer.start_iter(), &cursor, false)
        .to_string();
    let re = regex::Regex::new(r#"id="([^"]+)""#).unwrap();
    let found_id = re
        .captures_iter(&text)
        .last()
        .map(|cap| cap[1].to_string())
        .unwrap_or_default();

    if let Some(item_id) = state.open_item_id.borrow().clone() {
        show_split_chapter_dialog(&state.window, state, &item_id, &found_id);
    }
}

pub(crate) fn strip_tags_from_selection(state: &Rc<UiState>) {
    if let Some(selected) = selected_editor_text(state) {
        let plain = gutencore::GutenCore::extract_text(&selected);
        replace_editor_selection(state, &plain);
    }
}

pub(crate) fn create_list_from_selection(state: &Rc<UiState>, kind: gutencore::ListKind) {
    if let Some(selected) = selected_editor_text(state) {
        let formatted = gutencore::GutenCore::create_list(gutencore::CreateListOptions {
            input: selected,
            kind,
            mode: gutencore::CreateListInputMode::Auto,
            class_name: None,
        });

        match formatted {
            Ok(list_html) => replace_editor_selection(state, &list_html),
            Err(e) => show_error_dialog(&state.window, "Crear lista", &e.to_string()),
        }
    }
}

pub(crate) fn apply_tag_to_selection(state: &Rc<UiState>, tag_name: String) {
    if let Some(text) = selected_editor_text(state) {
        let formatted = gutencore::GutenCore::apply_format(gutencore::ApplyFormatOptions {
            input: text,
            mode: gutencore::FormatInputMode::HtmlFragment,
            format: gutencore::TextFormat::Tag { tag: tag_name },
        });

        match formatted {
            Ok(tagged) => replace_editor_selection(state, &tagged),
            Err(e) => show_error_dialog(&state.window, "Aplicar formato", &e.to_string()),
        }
    }
}

pub(crate) fn apply_tag_class_to_selection(state: &Rc<UiState>, raw: String) {
    let (tag, class) = raw
        .split_once('|')
        .map(|(t, c)| (t.to_string(), c.to_string()))
        .unwrap_or_else(|| ("span".to_string(), raw.clone()));

    if let Some(text) = selected_editor_text(state) {
        let formatted = gutencore::GutenCore::apply_format(gutencore::ApplyFormatOptions {
            input: text,
            mode: gutencore::FormatInputMode::HtmlFragment,
            format: gutencore::TextFormat::Class {
                tag: Some(tag),
                class_name: class,
            },
        });

        match formatted {
            Ok(tagged) => replace_editor_selection(state, &tagged),
            Err(e) => show_error_dialog(&state.window, "Aplicar formato", &e.to_string()),
        }
    }
}

pub(crate) fn toggle_sidebar(sidebar: &ScrolledWindow, paned: &Paned, settings: &gio::Settings) {
    if sidebar.is_visible() {
        let _ = settings.set_int("sidebar-width", paned.position());
        sidebar.set_visible(false);
    } else {
        sidebar.set_visible(true);
        let saved = settings.int("sidebar-width");
        paned.set_position(if saved > 10 { saved } else { 260 });
    }
}

pub(crate) fn toggle_editor_preview(state: &Rc<UiState>) {
    match state.main_stack.visible_child_name().as_deref() {
        Some("preview") => state.main_stack.set_visible_child_name("editor"),
        _ => state.main_stack.set_visible_child_name("preview"),
    }
}

pub(crate) fn shortcuts_section(title: &str, items: &[(&str, &str)]) -> ShortcutsSection {
    let section = ShortcutsSection::new(Some(title));
    for (label, accel) in items {
        section.add(ShortcutsItem::new(label, accel));
    }
    section
}

pub(crate) fn show_shortcuts_dialog(parent: &ApplicationWindow) {
    let dialog = ShortcutsDialog::new();

    dialog.add(shortcuts_section(
        "Proyecto",
        &[
            ("Abrir proyecto/libro", "<Control>o"),
            ("Guardar archivo actual", "<Control>s"),
            ("Nuevo capítulo", "<Control>n"),
            ("Importar capítulos", "<Control>t"),
            ("Exportar", "<Control><Shift>t"),
            ("Tabla de contenidos", "<Control><Shift>n"),
            ("Proyecto reciente 1", "<Control>1"),
            ("Proyecto reciente 2", "<Control>2"),
            ("Proyecto reciente 3", "<Control>3"),
            ("Proyecto reciente 4", "<Control>4"),
            ("Proyecto reciente 5", "<Control>5"),
        ],
    ));

    dialog.add(shortcuts_section(
        "Vista",
        &[
            ("Alternar editor/vista previa", "<Control>Right"),
            ("Alternar barra lateral", "<Control><Shift>s"),
            ("Mostrar atajos de teclado", "F1"),
        ],
    ));

    dialog.add(shortcuts_section(
        "Edición",
        &[
            ("Buscar/reemplazar", "<Control>f"),
            ("Asistente IA", "<Control><Shift>i"),
            ("Dividir párrafo", "<Control>d"),
            ("Dividir capítulo", "<Control><Shift>d"),
            ("Quitar tags", "<Control>Delete"),
        ],
    ));

    dialog.add(shortcuts_section(
        "Formatos",
        &[
            ("Negrita", "<Control>b"),
            ("Cursiva", "<Control>k"),
            ("Título 1", "<Control>h"),
            ("Párrafo", "<Control>g"),
            ("Lista con viñetas", "<Control>a"),
            ("Lista numerada", "<Control><Shift>a"),
        ],
    ));

    dialog.add(shortcuts_section(
        "Informes y validación",
        &[
            ("Informe del capítulo", "<Control>i"),
            ("Informe del libro", "<Control><Alt>i"),
            ("Verificar EPUB", "<Control><Shift>v"),
        ],
    ));

    dialog.present(Some(parent));
}

pub(crate) fn setup_editor_context_menu(state: &Rc<UiState>) {
    let menu = gio::Menu::new();

    // IA section
    let ai_section = gio::Menu::new();
    ai_section.append(Some("Asistente IA..."), Some("editor.ai"));
    menu.append_section(None, &ai_section);

    // Split Section
    let split_section = gio::Menu::new();
    split_section.append(Some("Dividir párrafo aquí"), Some("editor.split-paragraph"));
    split_section.append(
        Some("Dividir capítulo aquí..."),
        Some("editor.split-chapter"),
    );
    menu.append_section(None, &split_section);

    // Submenu for Styles
    let styles_submenu = gio::Menu::new();
    menu.append_submenu(Some("Estilos"), &styles_submenu);

    styles_submenu.append(Some("Quitar tags"), Some("editor.strip-tags"));
    styles_submenu.append(Some("Lista con viñetas"), Some("editor.create-list('ul')"));
    styles_submenu.append(Some("Lista numerada"), Some("editor.create-list('ol')"));
    styles_submenu.append_section(None, &gio::Menu::new());

    let common_styles = vec![
        ("Negrita", "strong"),
        ("Cursiva", "em"),
        ("Título 1", "h1"),
        ("Título 2", "h2"),
        ("Párrafo", "p"),
    ];
    let mut classes_by_tag: BTreeMap<String, Vec<String>> = BTreeMap::new();

    // Try to load styles from core if possible (per chapter)
    let item_id_opt = state.open_item_id.borrow().clone();
    let path_opt = state.current_path.borrow().clone();

    if let (Some(item_id), Some(path_str)) = (item_id_opt, path_opt) {
        println!(
            "[Menu] Buscando estilos para el capítulo: {} en {}",
            item_id, path_str
        );
        if let Ok(core) = gutencore::GutenCore::open_folder(&path_str) {
            // Log CSS IDs from config (default_styles / exceptions)
            let config_style_ids = core.get_chapter_styles(&item_id);
            println!(
                "[Menu] IDs de CSS según config (default_styles/exceptions): {:?}",
                config_style_ids
            );

            // Log CSS hrefs linked in the XHTML <link> tags
            if let Ok(xhtml_path) = core.get_resource_path(&item_id) {
                if let Ok(xhtml_content) = std::fs::read_to_string(&xhtml_path) {
                    let link_re = regex::Regex::new(r#"<link[^>]+href="([^"]+\.css)"#).unwrap();
                    let linked: Vec<&str> = link_re
                        .captures_iter(&xhtml_content)
                        .filter_map(|c| c.get(1).map(|m| m.as_str()))
                        .collect();
                    println!("[Menu] CSS referenciados en el XHTML <link>: {:?}", linked);
                }
            }

            // Use get_style_catalog to get the actual CSS class names
            match core.get_style_catalog(&item_id) {
                Ok(catalogs) => {
                    for catalog in &catalogs {
                        let bloque: Vec<&str> = catalog
                            .estilos
                            .bloque
                            .iter()
                            .map(|e| e.clase.as_str())
                            .collect();
                        let linea: Vec<&str> = catalog
                            .estilos
                            .linea
                            .iter()
                            .map(|e| e.clase.as_str())
                            .collect();
                        println!(
                            "[Menu] CSS '{}' — clases bloque: {:?}, clases línea: {:?}",
                            catalog.archivo_origen, bloque, linea
                        );
                        for entry in &catalog.estilos.bloque {
                            let tag = entry
                                .tag_sugerido
                                .clone()
                                .unwrap_or_else(|| "p".to_string());
                            let classes = classes_by_tag.entry(tag).or_default();
                            if !classes.contains(&entry.clase) {
                                classes.push(entry.clase.clone());
                            }
                        }
                        for entry in &catalog.estilos.linea {
                            let tag = entry
                                .tag_sugerido
                                .clone()
                                .unwrap_or_else(|| "span".to_string());
                            let classes = classes_by_tag.entry(tag).or_default();
                            if !classes.contains(&entry.clase) {
                                classes.push(entry.clase.clone());
                            }
                        }
                    }
                    if !classes_by_tag.is_empty() {
                        println!("[Menu] Clases agrupadas por tag: {:?}", classes_by_tag);
                    } else {
                        println!("[Menu] No se encontraron clases CSS para este capítulo.");
                    }
                }
                Err(e) => {
                    println!("[Menu] Error obteniendo catálogo de estilos: {}", e);
                }
            }
        } else {
            println!("[Menu] Error: No se pudo abrir la carpeta del libro para extraer estilos.");
        }
    }

    for (label, tag) in &common_styles {
        if let Some(classes) = classes_by_tag.remove(*tag) {
            let tag_menu = gio::Menu::new();
            tag_menu.append(Some(label), Some(&format!("editor.apply-tag('{}')", tag)));
            tag_menu.append_section(None, &gio::Menu::new());
            for class in classes {
                let item_label = format!("{}.{}", tag, class);
                let target = format!("{}|{}", tag, class);
                let item = gio::MenuItem::new(
                    Some(&item_label),
                    Some(&format!("editor.apply-tag-class('{}')", target)),
                );
                tag_menu.append_item(&item);
            }
            styles_submenu.append_submenu(Some(label), &tag_menu);
        } else {
            let item =
                gio::MenuItem::new(Some(label), Some(&format!("editor.apply-tag('{}')", tag)));
            styles_submenu.append_item(&item);
        }
    }

    for (tag, classes) in classes_by_tag {
        let tag_menu = gio::Menu::new();
        tag_menu.append(Some(&tag), Some(&format!("editor.apply-tag('{}')", tag)));
        tag_menu.append_section(None, &gio::Menu::new());
        for class in classes {
            let item_label = format!("{}.{}", tag, class);
            let target = format!("{}|{}", tag, class);
            let item = gio::MenuItem::new(
                Some(&item_label),
                Some(&format!("editor.apply-tag-class('{}')", target)),
            );
            tag_menu.append_item(&item);
        }
        styles_submenu.append_submenu(Some(&tag), &tag_menu);
    }

    state.editor.set_extra_menu(Some(&menu));

    let action_group = gio::SimpleActionGroup::new();
    state
        .editor
        .insert_action_group("editor", Some(&action_group));

    // IA Action
    let ai_action = gio::SimpleAction::new("ai", None);
    let state_ai = state.clone();
    ai_action.connect_activate(move |_, _| {
        show_ai_for_selection(&state_ai);
    });
    action_group.add_action(&ai_action);

    // Split Chapter Action
    let split_action = gio::SimpleAction::new("split-chapter", None);
    let state_split = state.clone();
    split_action.connect_activate(move |_, _| {
        split_chapter_at_cursor(&state_split);
    });
    action_group.add_action(&split_action);

    // Split Paragraph Action
    let split_paragraph_action = gio::SimpleAction::new("split-paragraph", None);
    let state_split_paragraph = state.clone();
    split_paragraph_action.connect_activate(move |_, _| {
        split_paragraph_at_cursor(&state_split_paragraph);
    });
    action_group.add_action(&split_paragraph_action);

    // Strip tags from the current selection.
    let strip_tags_action = gio::SimpleAction::new("strip-tags", None);
    let state_strip_tags = state.clone();
    strip_tags_action.connect_activate(move |_, _| {
        strip_tags_from_selection(&state_strip_tags);
    });
    action_group.add_action(&strip_tags_action);

    // Create an unordered or ordered list from selected lines.
    let create_list_action = gio::SimpleAction::new("create-list", Some(glib::VariantTy::STRING));
    let state_create_list = state.clone();
    create_list_action.connect_activate(move |_, variant| {
        let Some(kind_raw) = variant.and_then(|v| v.get::<String>()) else {
            return;
        };
        let kind = match kind_raw.as_str() {
            "ul" => gutencore::ListKind::Unordered,
            "ol" => gutencore::ListKind::Ordered,
            _ => {
                show_error_dialog(
                    &state_create_list.window,
                    "Crear lista",
                    &format!("Tipo de lista desconocido: {}", kind_raw),
                );
                return;
            }
        };

        create_list_from_selection(&state_create_list, kind);
    });
    action_group.add_action(&create_list_action);

    // Apply Tag Action (e.g. <b>...</b>)
    let tag_apply_action = gio::SimpleAction::new("apply-tag", Some(glib::VariantTy::STRING));
    let state_tag = state.clone();
    tag_apply_action.connect_activate(move |_, variant| {
        if let Some(tag_name) = variant.and_then(|v| v.get::<String>()) {
            apply_tag_to_selection(&state_tag, tag_name);
        }
    });
    action_group.add_action(&tag_apply_action);

    // Apply Tag+Class Action — variant is "tag|class", e.g. "p|sub" → <p class="sub">…</p>
    let tag_class_action = gio::SimpleAction::new("apply-tag-class", Some(glib::VariantTy::STRING));
    let state_tc = state.clone();
    tag_class_action.connect_activate(move |_, variant| {
        if let Some(raw) = variant.and_then(|v| v.get::<String>()) {
            apply_tag_class_to_selection(&state_tc, raw);
        }
    });
    action_group.add_action(&tag_class_action);
}

// ─── Main UI ─────────────────────────────────────────────────────────────────

pub(crate) fn navigate_search(state: &Rc<UiState>, forward: bool) {
    let buffer = state.editor.buffer();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let result = if forward {
        state.search_ctx.forward(&cursor)
    } else {
        state.search_ctx.backward(&cursor)
    };
    if let Some((start, end, _wrapped)) = result {
        buffer.select_range(&start, &end);
        state
            .editor
            .scroll_to_iter(&mut start.clone(), 0.1, true, 0.5, 0.5);
    }
}

pub(crate) fn format_match_count(count: i32, has_query: bool) -> String {
    if !has_query {
        String::new()
    } else if count < 0 {
        "…".to_string()
    } else if count == 0 {
        "Sin coincidencias".to_string()
    } else if count == 1 {
        "1 coincidencia".to_string()
    } else {
        format!("{} coincidencias", count)
    }
}

pub(crate) fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let response = reqwest::blocking::get(&url).map_err(|e| e.to_string())?;
    let json: serde_json::Value = response.json().map_err(|e| e.to_string())?;
    let models = json["models"]
        .as_array()
        .ok_or_else(|| "respuesta inesperada del servidor".to_string())?
        .iter()
        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
        .collect();
    Ok(models)
}
