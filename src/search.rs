use crate::prelude::*;

pub(crate) fn show_book_search_dialog(state: &Rc<UiState>, replace_mode: bool) {
    if state.current_path.borrow().is_none() {
        return;
    }

    let win = adw::Window::builder()
        .title(if replace_mode {
            "Buscar y reemplazar en el libro"
        } else {
            "Buscar en el libro"
        })
        .transient_for(&state.window)
        .modal(false)
        .default_width(660)
        .default_height(520)
        .build();

    let outer = Box::new(Orientation::Vertical, 0);
    let header = HeaderBar::new();
    outer.append(&header);

    let content_box = Box::new(Orientation::Vertical, 6);
    content_box.set_margin_start(12);
    content_box.set_margin_end(12);
    content_box.set_margin_top(8);
    content_box.set_margin_bottom(12);

    // ── Search row ────────────────────────────────────────────────────────────
    let search_row = Box::new(Orientation::Horizontal, 6);
    let search_entry = SearchEntry::builder()
        .placeholder_text("Buscar en todos los archivos…")
        .hexpand(true)
        .build();
    let case_btn = gtk::ToggleButton::builder()
        .label("Aa")
        .tooltip_text("Distinguir mayúsculas/minúsculas")
        .valign(gtk::Align::Center)
        .build();
    case_btn.add_css_class("flat");
    search_row.append(&search_entry);
    search_row.append(&case_btn);
    content_box.append(&search_row);

    // ── Replace entry row (revealed in replace mode) ───────────────────────
    let replace_entry = Entry::builder()
        .placeholder_text("Reemplazar con…")
        .hexpand(true)
        .build();
    let replace_revealer = gtk::Revealer::builder()
        .child(&replace_entry)
        .reveal_child(replace_mode)
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(150)
        .build();
    content_box.append(&replace_revealer);

    // ── Replace buttons row (revealed in replace mode) ─────────────────────
    let btns_row = Box::new(Orientation::Horizontal, 6);
    btns_row.set_halign(gtk::Align::End);

    let btn_replace_next = Button::builder()
        .label("Reemplazar siguiente")
        .sensitive(false)
        .build();
    let btn_replace_all_book = Button::builder()
        .label("Reemplazar todo en el libro")
        .sensitive(false)
        .build();
    btn_replace_all_book.add_css_class("suggested-action");

    btns_row.append(&btn_replace_next);
    btns_row.append(&btn_replace_all_book);

    let btns_revealer = gtk::Revealer::builder()
        .child(&btns_row)
        .reveal_child(replace_mode)
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(150)
        .build();
    content_box.append(&btns_revealer);

    content_box.append(&gtk::Separator::new(Orientation::Horizontal));

    // ── Status label ──────────────────────────────────────────────────────────
    let status_label = Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .build();
    status_label.add_css_class("caption");
    status_label.add_css_class("dim-label");
    content_box.append(&status_label);

    // ── Results list ──────────────────────────────────────────────────────────
    let results_list = ListBox::new();
    results_list.set_selection_mode(gtk::SelectionMode::None);
    let scrolled = ScrolledWindow::builder()
        .child(&results_list)
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    content_box.append(&scrolled);

    outer.append(&content_box);
    win.set_content(Some(&outer));

    // ── Search closure ────────────────────────────────────────────────────────
    let do_search: Rc<dyn Fn()> = Rc::new({
        let search_entry = search_entry.clone();
        let case_btn = case_btn.clone();
        let results_list = results_list.clone();
        let status_label = status_label.clone();
        let btn_replace_next = btn_replace_next.clone();
        let btn_replace_all_book = btn_replace_all_book.clone();
        let state = state.clone();

        move || {
            while let Some(child) = results_list.first_child() {
                results_list.remove(&child);
            }

            let query = search_entry.text().to_string();
            let case_sensitive = case_btn.is_active();
            let has_query = query.len() >= 2;

            btn_replace_next.set_sensitive(has_query);
            btn_replace_all_book.set_sensitive(has_query);

            if !has_query {
                status_label.set_label("");
                state.search_settings.set_search_text(None);
                return;
            }

            let query_cmp = if case_sensitive {
                query.clone()
            } else {
                query.to_lowercase()
            };

            // Keep editor highlights in sync with the search entry
            state.search_settings.set_search_text(Some(&query));
            if !case_sensitive {
                state.search_settings.set_case_sensitive(false);
            } else {
                state.search_settings.set_case_sensitive(true);
            }

            let path = match state.current_path.borrow().clone() {
                Some(p) => p,
                None => return,
            };
            let core = match gutencore::GutenCore::open_folder_quick(&path) {
                Ok(c) => c,
                Err(_) => return,
            };

            // Sort XHTML items by spine (reading order)
            let spine = core.get_spine().clone();
            let mut items: Vec<(String, gutencore::ManifestItem)> = core
                .manifest
                .iter()
                .filter(|(_, item)| item.media_type.contains("html"))
                .map(|(id, item)| (id.clone(), item.clone()))
                .collect();
            items.sort_by(|(a, _), (b, _)| {
                let pa = spine.iter().position(|r| r == a);
                let pb = spine.iter().position(|r| r == b);
                match (pa, pb) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.cmp(b),
                }
            });

            let mut total = 0usize;
            const MAX: usize = 200;
            let mut capped = false;

            'files: for (item_id, item) in &items {
                let fpath = match core.get_resource_path(item_id) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let raw = match std::fs::read_to_string(&fpath) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let plain = strip_html_for_search(&raw);
                let search_in = if case_sensitive {
                    plain.clone()
                } else {
                    plain.to_lowercase()
                };

                let filename = Path::new(&item.href)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(item_id.as_str())
                    .to_string();

                let mut pos = 0usize;
                while let Some(rel) = search_in[pos..].find(query_cmp.as_str()) {
                    let abs = pos + rel;
                    let snippet = build_snippet(&plain, abs, query.chars().count(), 80);

                    let row = ActionRow::builder()
                        .title(snippet.as_str())
                        .subtitle(filename.as_str())
                        .activatable(true)
                        .build();

                    let state_r = state.clone();
                    let id_r = item_id.clone();
                    let media_r = item.media_type.clone();
                    let query_r = query.clone();
                    let case_r = case_sensitive;

                    row.connect_activated(move |_| {
                        open_item(&state_r, &id_r, &media_r);

                        // Set search_settings so the editor highlights all matches
                        state_r.search_settings.set_search_text(Some(&query_r));
                        state_r.search_settings.set_case_sensitive(case_r);

                        // Defer jump-to-first-match until buffer is populated
                        let state_idle = state_r.clone();
                        let query_idle = query_r.clone();
                        glib::idle_add_local_once(move || {
                            jump_to_first_match(&state_idle, &query_idle, case_r);
                        });
                    });

                    results_list.append(&row);
                    total += 1;
                    pos = abs + query_cmp.len().max(1);

                    if total >= MAX {
                        capped = true;
                        break 'files;
                    }
                }
            }

            if total == 0 {
                status_label.set_label("Sin resultados");
            } else if capped {
                status_label.set_label(&format!(
                    "Mostrando los primeros {} resultados — refina la búsqueda",
                    MAX
                ));
            } else {
                status_label.set_label(&format!(
                    "{} coincidencia{}",
                    total,
                    if total == 1 { "" } else { "s" }
                ));
            }
        }
    });

    search_entry.connect_search_changed({
        let ds = do_search.clone();
        move |_| ds()
    });
    case_btn.connect_toggled({
        let ds = do_search.clone();
        move |_| ds()
    });

    // ── Reemplazar siguiente ──────────────────────────────────────────────────
    // Replaces the currently selected match in the editor and advances to next.
    btn_replace_next.connect_clicked({
        let state = state.clone();
        let replace_entry = replace_entry.clone();
        let do_search = do_search.clone();
        move |_| {
            let replacement = replace_entry.text().to_string();
            let buffer = state.editor.buffer();
            if let Some((mut start, mut end)) = buffer.selection_bounds() {
                let _ = state.search_ctx.replace(&mut start, &mut end, &replacement);
                save_current_item(&state);
            }
            navigate_search(&state, true);
            do_search();
        }
    });

    // ── Reemplazar todo en el libro ───────────────────────────────────────────
    // Replaces all occurrences across every XHTML file on disk.
    btn_replace_all_book.connect_clicked({
        let state = state.clone();
        let search_entry = search_entry.clone();
        let replace_entry = replace_entry.clone();
        let case_btn = case_btn.clone();
        let status_label = status_label.clone();
        let do_search = do_search.clone();
        move |_| {
            let query = search_entry.text().to_string();
            if query.len() < 2 {
                return;
            }
            let replacement = replace_entry.text().to_string();
            let case_sensitive = case_btn.is_active();

            let path = match state.current_path.borrow().clone() {
                Some(p) => p,
                None => return,
            };
            let core = match gutencore::GutenCore::open_folder_quick(&path) {
                Ok(c) => c,
                Err(_) => return,
            };

            let pattern = if case_sensitive {
                regex::escape(&query)
            } else {
                format!("(?i){}", regex::escape(&query))
            };
            let re = match regex::Regex::new(&pattern) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("book replace_all: regex error: {}", e);
                    return;
                }
            };

            let mut total_replaced = 0usize;
            let open_id = state.open_item_id.borrow().clone();

            for (id, item) in &core.manifest {
                if !item.media_type.contains("html") {
                    continue;
                }
                let fpath = match core.get_resource_path(id) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let raw = match std::fs::read_to_string(&fpath) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let count = re.find_iter(&raw).count();
                if count == 0 {
                    continue;
                }

                let new_content = re.replace_all(&raw, regex::NoExpand(replacement.as_str()));
                if std::fs::write(&fpath, new_content.as_bytes()).is_err() {
                    continue;
                }
                total_replaced += count;

                // If this file is open in the editor, reload the buffer
                if open_id.as_deref() == Some(id.as_str()) {
                    if let Ok(buf) = state.editor.buffer().downcast::<sourceview5::Buffer>() {
                        buf.set_text(&new_content);
                    }
                }
            }

            if total_replaced == 0 {
                status_label.set_label("Sin coincidencias para reemplazar");
            } else {
                status_label.set_label(&format!(
                    "Reemplazadas {} coincidencia{}",
                    total_replaced,
                    if total_replaced == 1 { "" } else { "s" }
                ));
            }

            do_search();
        }
    });

    // Clear editor highlights when the dialog is closed
    win.connect_close_request({
        let settings = state.search_settings.clone();
        move |_| {
            settings.set_search_text(None);
            glib::Propagation::Proceed
        }
    });

    win.present();
    search_entry.grab_focus();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// Finds the first occurrence of `query` in the editor buffer and selects it.
fn jump_to_first_match(state: &Rc<UiState>, query: &str, case_sensitive: bool) {
    let buffer = state.editor.buffer();
    let raw = buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), false)
        .to_string();

    let search_in = if case_sensitive {
        raw.clone()
    } else {
        raw.to_lowercase()
    };
    let query_cmp = if case_sensitive {
        query.to_string()
    } else {
        query.to_lowercase()
    };

    if let Some(byte_pos) = search_in.find(&query_cmp) {
        let char_start = raw[..byte_pos].chars().count() as i32;
        let char_end = char_start + query.chars().count() as i32;
        let start_iter = buffer.iter_at_offset(char_start);
        let end_iter = buffer.iter_at_offset(char_end);
        buffer.select_range(&start_iter, &end_iter);
        state
            .editor
            .scroll_to_iter(&mut start_iter.clone(), 0.1, true, 0.5, 0.5);
    }
}

// Strips HTML/XML tags and entity references from `html`, leaving plain text.
fn strip_html_for_search(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_entity = false;

    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => {
                in_tag = false;
            }
            '&' if !in_tag => {
                in_entity = true;
            }
            ';' if in_entity => {
                in_entity = false;
                out.push(' ');
            }
            _ if !in_tag && !in_entity => out.push(ch),
            _ => {}
        }
    }
    out
}

// Extracts ~`context_chars` characters centered on the byte offset of the match.
fn build_snippet(text: &str, match_byte: usize, query_char_len: usize, context_chars: usize) -> String {
    let prefix_chars = text.get(..match_byte).map(|s| s.chars().count()).unwrap_or(0);
    let total_chars = text.chars().count();
    let half = context_chars / 2;
    let start_char = prefix_chars.saturating_sub(half);
    let end_char = (prefix_chars + query_char_len + half).min(total_chars);

    let snippet: String = text
        .chars()
        .skip(start_char)
        .take(end_char - start_char)
        .collect();
    let snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");

    let prefix = if start_char > 0 { "…" } else { "" };
    let suffix = if end_char < total_chars { "…" } else { "" };
    format!("{}{}{}", prefix, snippet.trim(), suffix)
}
