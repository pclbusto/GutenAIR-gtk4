use crate::prelude::*;

pub(crate) fn add_to_history(settings: &gio::Settings, path: &str) {
    let mut history: Vec<String> = settings
        .strv("history")
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    history.retain(|p| p != path);
    history.insert(0, path.to_string());
    history.truncate(100);
    let refs: Vec<&str> = history.iter().map(|s| s.as_str()).collect();
    let _ = settings.set_strv("history", refs);
}

// ─── EPUB extraction ─────────────────────────────────────────────────────────

pub(crate) fn extract_epub(epub_path: &Path) -> Result<std::path::PathBuf, String> {
    let cache = glib::user_cache_dir().join("gutenair");
    let stem = epub_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("epub");
    let dest = cache.join(stem);

    if dest.exists() {
        return Ok(dest);
    }

    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let file = std::fs::File::open(epub_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().replace('\\', "/");
        let name = name.trim_start_matches('/');
        let out = dest.join(name);
        if !out.starts_with(&dest) {
            continue;
        }
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(dest)
}

pub(crate) fn load_path(path_str: &str, state: &Rc<UiState>) {
    let path = Path::new(path_str);
    if path.is_dir() {
        load_book(path_str, state);
    } else if path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("epub"))
        .unwrap_or(false)
    {
        match extract_epub(path) {
            Ok(dir) => load_book(&dir.to_string_lossy(), state),
            Err(e) => eprintln!("Error extrayendo epub: {}", e),
        }
    }
}

// ─── Sidebar helpers ─────────────────────────────────────────────────────────

pub(crate) fn sync_stylesheet_links(
    content: &str,
    core: &gutencore::GutenCore,
    chapter_id: &str,
) -> String {
    // Eliminar todos los link de stylesheet existentes (con su posible indentación y newline)
    let link_re =
        regex::Regex::new(r#"(?m)[ \t]*<link\b[^>]*\brel=["']stylesheet["'][^>]*/>\r?\n?"#)
            .unwrap();
    let without = link_re.replace_all(content, "").to_string();

    // Construir los nuevos link tags según el config del capítulo
    let styles = core.get_chapter_styles(chapter_id);
    let new_links: Vec<String> = styles
        .iter()
        .filter_map(|id| core.manifest.get(id))
        .filter(|item| item.media_type == "text/css")
        .map(|item| {
            format!(
                r#"  <link rel="stylesheet" type="text/css" href="../{}"/>"#,
                item.href
            )
        })
        .collect();

    if new_links.is_empty() {
        return without;
    }

    let block = format!("\n{}", new_links.join("\n"));

    // Insertar justo después de </title>; si no hay title, después de <head>
    if let Some(pos) = without.find("</title>") {
        let at = pos + "</title>".len();
        let mut out = without.clone();
        out.insert_str(at, &block);
        out
    } else if let Some(pos) = without.find("<head>") {
        let at = pos + "<head>".len();
        let mut out = without.clone();
        out.insert_str(at, &block);
        out
    } else {
        without
    }
}

pub(crate) fn save_current_item(state: &Rc<UiState>) {
    let item_id_opt = state.open_item_id.borrow().clone();
    let path_opt = state.current_path.borrow().clone();
    let media_type_opt = state.open_item_media_type.borrow().clone();

    // No guardar archivos binarios (imágenes, fuentes, audio)
    if let Some(ref mt) = media_type_opt {
        if mt.starts_with("image/")
            || mt.starts_with("font/")
            || mt.starts_with("audio/")
            || mt.starts_with("video/")
        {
            return;
        }
    }

    if let (Some(item_id), Some(path_str)) = (item_id_opt, path_opt) {
        let core = match gutencore::GutenCore::open_folder(&path_str) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[Guardado] No se pudo abrir el proyecto para guardar '{}': {}",
                    item_id, e
                );
                return;
            }
        };
        let full_path = match core.get_resource_path(&item_id) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[Guardado] No se encontró la ruta de '{}': {}", item_id, e);
                return;
            }
        };
        let buffer = state.editor.buffer();
        let raw = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        // Para capítulos XHTML con auto_inject activo: re-sincronizar los <link> de estilos
        let is_xhtml = media_type_opt
            .as_deref()
            .map(|m| m.contains("html"))
            .unwrap_or(false);
        let text = if is_xhtml && core.config.auto_inject {
            sync_stylesheet_links(&raw, &core, &item_id)
        } else {
            raw
        };

        println!("[Guardado] Guardando cambios en {}...", item_id);
        let save_result = if is_xhtml {
            match std::fs::write(&full_path, &text) {
                Ok(_) => {
                    let mut core = core;
                    core.build_index()
                }
                Err(e) => Err(gutencore::GutenError::Io(e)),
            }
        } else {
            std::fs::write(&full_path, &text).map_err(gutencore::GutenError::Io)
        };

        if let Err(e) = save_result {
            eprintln!("[Error] No se pudo guardar {}: {}", item_id, e);
        }
    }
}

pub(crate) fn open_item(state: &Rc<UiState>, item_id: &str, media_type: &str) {
    // Save previous item if exists
    save_current_item(state);
    load_item_without_saving(state, item_id, media_type);
}

pub(crate) fn load_item_without_saving(state: &Rc<UiState>, item_id: &str, media_type: &str) {
    if let Some(path_str) = state.current_path.borrow().clone() {
        if let Ok(core) = gutencore::GutenCore::open_folder(&path_str) {
            if let Ok(full_path) = core.get_resource_path(item_id) {
                let filename = full_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(item_id);
                state.header_title.set_subtitle(filename);

                // Update current open ID and enable stats button
                *state.open_item_id.borrow_mut() = Some(item_id.to_string());
                *state.open_item_media_type.borrow_mut() = Some(media_type.to_string());
                state.stats_btn.set_sensitive(true);

                if media_type.starts_with("image/") {
                    match gdk_pixbuf::Pixbuf::from_file(&full_path) {
                        Ok(pixbuf) => {
                            let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                            state.image_viewer.set_paintable(Some(&texture));
                            state.preview_inner.set_visible_child_name("image");
                            state.main_stack.set_visible_child_name("preview");
                        }
                        Err(e) => eprintln!("[image] error cargando imagen: {}", e),
                    }
                } else {
                    // Cargar contenido de texto en el editor
                    let content = std::fs::read_to_string(&full_path).unwrap_or_default();
                    if let Ok(buffer) = state.editor.buffer().downcast::<sourceview5::Buffer>() {
                        buffer.set_text(&content);
                        let lang_manager = sourceview5::LanguageManager::default();
                        let lang_id = if media_type.contains("html") || media_type.contains("xhtml")
                        {
                            "html"
                        } else if media_type.contains("css") {
                            "css"
                        } else if media_type.contains("xml") {
                            "xml"
                        } else {
                            "txt"
                        };
                        if let Some(lang) = lang_manager.language(lang_id) {
                            buffer.set_language(Some(&lang));
                        }
                    }

                    // Refresh context menu with new styles
                    setup_editor_context_menu(state);

                    let is_html = media_type.contains("html") || media_type.contains("xhtml");
                    if is_html {
                        let uri = glib::filename_to_uri(&full_path, None).unwrap_or_else(|_| {
                            format!("file://{}", full_path.to_string_lossy()).into()
                        });
                        state.preview_inner.set_visible_child_name("web");
                        state.preview.load_uri(&uri);
                    }

                    // Si es HTML, solo cambiamos a editor si no estábamos ya en vista previa.
                    // Si NO es HTML (ej. CSS, TXT, XML), forzamos la vista de editor.
                    if !is_html
                        || state.main_stack.visible_child_name().as_deref() != Some("preview")
                    {
                        state.main_stack.set_visible_child_name("editor");
                    }
                }
            }
        }
    }
}

pub(crate) fn load_book(path_str: &str, state: &Rc<UiState>) {
    match gutencore::GutenCore::open_folder(path_str) {
        Ok(core) => {
            add_to_history(&state.settings, path_str);

            if let Some(meta) = &core.metadata {
                state.header_title.set_title(&meta.title);
            }
            state.header_title.set_subtitle("");

            let manifest_groups: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                core.manifest
                    .values()
                    .filter_map(|item| {
                        Path::new(&item.href)
                            .parent()
                            .and_then(|p| p.to_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                    })
                    .filter(|f| seen.insert(f.clone()))
                    .collect()
            };

            let mut configured: Vec<String> = state
                .settings
                .strv("sidebar-groups")
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            let hidden: Vec<String> = state
                .settings
                .strv("hidden-sidebar-groups")
                .into_iter()
                .map(|s| s.to_string())
                .collect();

            let mut changed = false;
            for group in &manifest_groups {
                if !configured.contains(group) && !hidden.contains(group) {
                    configured.push(group.clone());
                    changed = true;
                }
            }
            if changed {
                let refs: Vec<&str> = configured.iter().map(|s| s.as_str()).collect();
                let _ = state.settings.set_strv("sidebar-groups", refs);
            }

            *state.current_path.borrow_mut() = Some(path_str.to_string());
            *state.manifest_groups.borrow_mut() = manifest_groups;
            state.selected_items.borrow_mut().clear();
            *state.last_clicked.borrow_mut() = None;

            populate_sidebar(state, &core);

            state.sidebar_scrolled.set_visible(true);
            if state.paned.position() < 10 {
                let saved = state.settings.int("sidebar-width");
                state
                    .paned
                    .set_position(if saved > 10 { saved } else { 260 });
            }
        }
        Err(e) => eprintln!("Error abriendo libro: {}", e),
    }
}

// ─── Preferences ─────────────────────────────────────────────────────────────
