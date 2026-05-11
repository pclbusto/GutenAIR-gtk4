use crate::prelude::*;

pub(crate) type GroupEntries = Vec<(String, String, bool)>;

pub(crate) fn save_groups(entries: &GroupEntries, settings: &gio::Settings) {
    let all: Vec<&str> = entries.iter().map(|(k, _, _)| k.as_str()).collect();
    let hidden: Vec<&str> = entries
        .iter()
        .filter(|(_, _, v)| !*v)
        .map(|(k, _, _)| k.as_str())
        .collect();
    let _ = settings.set_strv("sidebar-groups", all);
    let _ = settings.set_strv("hidden-sidebar-groups", hidden);
}

pub(crate) fn rebuild_groups_ui(
    list: &ListBox,
    pref_state: &Rc<RefCell<GroupEntries>>,
    settings: &gio::Settings,
    ui_state: &Rc<UiState>,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let entries = pref_state.borrow().clone();
    let total = entries.len();

    for (idx, (key, display, visible)) in entries.iter().enumerate() {
        let row = ActionRow::builder().title(display.as_str()).build();

        let up_btn = Button::builder()
            .icon_name("go-up-symbolic")
            .has_frame(false)
            .valign(gtk::Align::Center)
            .sensitive(idx > 0)
            .build();
        let down_btn = Button::builder()
            .icon_name("go-down-symbolic")
            .has_frame(false)
            .valign(gtk::Align::Center)
            .sensitive(idx < total - 1)
            .build();
        let sw = Switch::builder()
            .active(*visible)
            .valign(gtk::Align::Center)
            .build();

        row.add_prefix(&up_btn);
        row.add_prefix(&down_btn);
        row.add_suffix(&sw);

        {
            let ps = pref_state.clone();
            let s = settings.clone();
            let uis = ui_state.clone();
            let key = key.clone();
            sw.connect_active_notify(move |sw| {
                if let Some(e) = ps.borrow_mut().iter_mut().find(|(k, _, _)| k == &key) {
                    e.2 = sw.is_active();
                }
                save_groups(&ps.borrow(), &s);
                refresh_sidebar(&uis);
            });
        }

        if idx > 0 {
            let ps = pref_state.clone();
            let s = settings.clone();
            let uis = ui_state.clone();
            let l = list.clone();
            up_btn.connect_clicked(move |_| {
                ps.borrow_mut().swap(idx, idx - 1);
                save_groups(&ps.borrow(), &s);
                refresh_sidebar(&uis);
                rebuild_groups_ui(&l, &ps, &s, &uis);
            });
        }

        if idx < total - 1 {
            let ps = pref_state.clone();
            let s = settings.clone();
            let uis = ui_state.clone();
            let l = list.clone();
            down_btn.connect_clicked(move |_| {
                ps.borrow_mut().swap(idx, idx + 1);
                save_groups(&ps.borrow(), &s);
                refresh_sidebar(&uis);
                rebuild_groups_ui(&l, &ps, &s, &uis);
            });
        }

        list.append(&row);
    }
}

pub(crate) fn show_preferences(
    parent: &impl IsA<gtk::Widget>,
    settings: &gio::Settings,
    ui_state: &Rc<UiState>,
) {
    let hidden: Vec<String> = settings
        .strv("hidden-sidebar-groups")
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let manifest_groups = ui_state.manifest_groups.borrow().clone();

    // Core folders always appear in settings (in canonical order), even if currently empty.
    // Extra folders from imported EPUBs appear after, alphabetically.
    let core_folders = core_content_folders();

    let make_entry = |key: &str| -> (String, String, bool) {
        (
            key.to_string(),
            folder_display_name(key).to_string(),
            !hidden.contains(&key.to_string()),
        )
    };

    let mut entries: GroupEntries = core_folders
        .iter()
        .map(|&f| {
            // If the manifest has this folder with different casing, use that name
            let actual = manifest_groups
                .iter()
                .find(|m| m.eq_ignore_ascii_case(f))
                .map(|s| s.as_str())
                .unwrap_or(f);
            make_entry(actual)
        })
        .collect();

    // Extra folders from imported EPUBs not covered by core canonical list
    let mut extra: Vec<(String, String, bool)> = manifest_groups
        .iter()
        .filter(|m| !core_folders.iter().any(|&f| f.eq_ignore_ascii_case(m)))
        .map(|m| make_entry(m))
        .collect();
    extra.sort_by(|a, b| a.0.cmp(&b.0));
    entries.extend(extra);

    let pref_state: Rc<RefCell<GroupEntries>> = Rc::new(RefCell::new(entries));

    let dialog = PreferencesDialog::builder().title("Preferencias").build();

    let page = PreferencesPage::builder()
        .title("Sidebar")
        .icon_name("sidebar-show-symbolic")
        .build();

    let group = PreferencesGroup::builder()
        .title("Grupos de recursos")
        .description("Activa o desactiva grupos y cambia su orden en el panel lateral")
        .build();

    let list = ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .build();
    list.add_css_class("boxed-list");

    rebuild_groups_ui(&list, &pref_state, settings, ui_state);

    group.add(&list);
    page.add(&group);
    dialog.add(&page);

    let editor_page = PreferencesPage::builder()
        .title("Editor")
        .icon_name("text-editor-symbolic")
        .build();

    let editor_group = PreferencesGroup::builder().title("Comportamiento").build();

    let wrap_row = ActionRow::builder()
        .title("Ajuste de línea automático")
        .subtitle("Ajustar líneas largas al ancho visible de la ventana")
        .build();

    let wrap_switch = gtk::Switch::builder().valign(gtk::Align::Center).build();

    settings
        .bind("editor-wrap-text", &wrap_switch, "active")
        .flags(gio::SettingsBindFlags::DEFAULT)
        .build();

    wrap_row.add_suffix(&wrap_switch);
    editor_group.add(&wrap_row);
    editor_page.add(&editor_group);
    dialog.add(&editor_page);

    // ── Ollama page ──────────────────────────────────────────────────────────
    let ollama_page = PreferencesPage::builder()
        .title("IA")
        .icon_name("applications-science-symbolic")
        .build();

    let ollama_group = PreferencesGroup::builder()
        .title("Ollama")
        .description("Configura la conexión con el servidor Ollama local")
        .build();

    // URL entry row
    let url_row = ActionRow::builder()
        .title("URL del servidor")
        .subtitle("Dirección del servidor Ollama")
        .build();

    let url_entry = gtk::Entry::builder()
        .placeholder_text("http://localhost:11434")
        .valign(gtk::Align::Center)
        .width_chars(28)
        .build();

    let saved_url: String = settings.string("ollama-url").to_string();
    url_entry.set_text(&saved_url);

    {
        let s = settings.clone();
        url_entry.connect_changed(move |e| {
            let _ = s.set_string("ollama-url", &e.text());
        });
    }

    url_row.add_suffix(&url_entry);
    url_row.set_activatable_widget(Some(&url_entry));
    ollama_group.add(&url_row);

    // ComboRow for model selection (populated after fetching)
    let model_list = gtk::StringList::new(&[]);
    let model_row = ComboRow::builder()
        .title("Modelo")
        .subtitle("Selecciona el modelo a usar")
        .model(&model_list)
        .build();

    let saved_model: String = settings.string("ollama-model").to_string();
    if !saved_model.is_empty() {
        model_list.append(&saved_model);
        model_row.set_selected(0);
    }

    {
        let s = settings.clone();
        let ml = model_list.clone();
        model_row.connect_selected_notify(move |row| {
            if let Some(name) = ml.string(row.selected()) {
                let _ = s.set_string("ollama-model", &name);
            }
        });
    }

    // Status label (shown below model row)
    let status_label = Label::builder().label("").halign(gtk::Align::Start).build();
    status_label.add_css_class("dim-label");
    status_label.add_css_class("caption");

    // "Cargar modelos" button row
    let load_row = ActionRow::builder()
        .title("Cargar modelos disponibles")
        .subtitle("Obtiene los modelos instalados en el servidor")
        .build();

    let load_btn = Button::builder()
        .label("Cargar")
        .valign(gtk::Align::Center)
        .build();
    load_btn.add_css_class("suggested-action");
    load_row.add_suffix(&load_btn);

    {
        let url_entry_c = url_entry.clone();
        let model_list_c = model_list.clone();
        let model_row_c = model_row.clone();
        let status_label_c = status_label.clone();
        let s = settings.clone();
        let saved_model_c = saved_model.clone();

        load_btn.connect_clicked(move |_| {
            let url = url_entry_c.text().to_string();
            status_label_c.set_text("Conectando…");

            let (tx, rx) = std::sync::mpsc::channel::<Result<Vec<String>, String>>();

            std::thread::spawn(move || {
                let _ = tx.send(fetch_ollama_models(&url));
            });

            let ml = model_list_c.clone();
            let mr = model_row_c.clone();
            let sl = status_label_c.clone();
            let s2 = s.clone();
            let sm = saved_model_c.clone();

            glib::idle_add_local(move || match rx.try_recv() {
                Ok(result) => {
                    match result {
                        Ok(models) => {
                            while ml.n_items() > 0 {
                                ml.remove(0);
                            }
                            for m in &models {
                                ml.append(m);
                            }
                            let pos = models.iter().position(|m| m == &sm).unwrap_or(0);
                            mr.set_selected(pos as u32);
                            if let Some(name) = ml.string(pos as u32) {
                                let _ = s2.set_string("ollama-model", &name);
                            }
                            sl.set_text(&format!("{} modelo(s) cargado(s)", models.len()));
                        }
                        Err(e) => {
                            sl.set_text(&format!("Error: {e}"));
                        }
                    }
                    glib::ControlFlow::Break
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
            });
        });
    }

    ollama_group.add(&load_row);
    ollama_group.add(&model_row);
    ollama_group.add(&status_label);

    ollama_page.add(&ollama_group);
    dialog.add(&ollama_page);

    dialog.present(Some(parent));
}
