use crate::prelude::*;

pub(crate) fn build_ui(app: &Application) {
    let settings = gio::Settings::new(APP_ID);

    // --- Sidebar ---
    let sidebar_box = Box::new(Orientation::Vertical, 0);
    let sidebar_scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&sidebar_box)
        .build();
    sidebar_scrolled.set_visible(false);

    // --- Main content (Stack) ---
    let main_stack = ViewStack::new();

    // Page 1: Editor
    let editor = sourceview5::View::new();
    let buffer = sourceview5::Buffer::new(None);
    editor.set_buffer(Some(&buffer));
    editor.set_show_line_numbers(true);
    editor.set_monospace(true);

    let initial_wrap = if settings.boolean("editor-wrap-text") {
        gtk::WrapMode::WordChar
    } else {
        gtk::WrapMode::None
    };
    editor.set_wrap_mode(initial_wrap);

    settings.connect_changed(Some("editor-wrap-text"), {
        let editor = editor.clone();
        move |settings, _| {
            let wrap = if settings.boolean("editor-wrap-text") {
                gtk::WrapMode::WordChar
            } else {
                gtk::WrapMode::None
            };
            editor.set_wrap_mode(wrap);
        }
    });

    let style_manager = adw::StyleManager::default();
    let update_theme = {
        let buffer = buffer.clone();
        move |sm: &adw::StyleManager| {
            let scheme_manager = sourceview5::StyleSchemeManager::default();
            let theme_name = if sm.is_dark() {
                "Adwaita-dark"
            } else {
                "Adwaita"
            };
            if let Some(scheme) = scheme_manager.scheme(theme_name) {
                buffer.set_style_scheme(Some(&scheme));
            }
        }
    };
    update_theme(&style_manager);
    style_manager.connect_dark_notify(update_theme);

    let search_settings = sourceview5::SearchSettings::new();
    search_settings.set_wrap_around(true);
    let search_ctx = sourceview5::SearchContext::new(&buffer, Some(&search_settings));
    search_ctx.set_highlight(true);

    let editor_scrolled = ScrolledWindow::builder()
        .child(&editor)
        .vexpand(true)
        .hexpand(true)
        .build();
    main_stack.add_titled_with_icon(
        &editor_scrolled,
        Some("editor"),
        "Editor",
        "text-editor-symbolic",
    );

    // Page 2: Preview
    // Page 2: Preview (WebKit + Image viewer compartiendo solapa)
    let preview = webkit6::WebView::new();
    let preview_scrolled = ScrolledWindow::builder()
        .child(&preview)
        .vexpand(true)
        .hexpand(true)
        .build();

    let image_viewer = Picture::new();
    image_viewer.set_keep_aspect_ratio(true);
    image_viewer.set_can_shrink(true);
    image_viewer.set_vexpand(true);
    image_viewer.set_hexpand(true);

    let preview_inner = gtk::Stack::new();
    preview_inner.set_vexpand(true);
    preview_inner.set_hexpand(true);
    preview_inner.add_named(&preview_scrolled, Some("web"));
    preview_inner.add_named(&image_viewer, Some("image"));

    main_stack.add_titled_with_icon(
        &preview_inner,
        Some("preview"),
        "Vista Previa",
        "web-browser-symbolic",
    );

    main_stack.set_vexpand(true);
    main_stack.set_hexpand(true);

    // --- Find / Replace bar ---
    let find_bar_box = Box::new(Orientation::Vertical, 0);
    find_bar_box.add_css_class("toolbar");

    let find_row = Box::new(Orientation::Horizontal, 4);
    find_row.set_margin_start(6);
    find_row.set_margin_end(6);
    find_row.set_margin_top(4);
    find_row.set_margin_bottom(2);

    let find_icon = gtk::Image::from_icon_name("edit-find-symbolic");
    find_icon.set_pixel_size(16);
    find_row.append(&find_icon);

    let find_entry = Entry::builder()
        .placeholder_text("Buscar…")
        .hexpand(true)
        .build();
    find_row.append(&find_entry);

    let btn_prev = Button::builder()
        .icon_name("go-up-symbolic")
        .tooltip_text("Coincidencia anterior  (Mayús+Intro)")
        .has_frame(false)
        .sensitive(false)
        .build();
    let btn_next = Button::builder()
        .icon_name("go-down-symbolic")
        .tooltip_text("Siguiente coincidencia  (Intro)")
        .has_frame(false)
        .sensitive(false)
        .build();
    find_row.append(&btn_prev);
    find_row.append(&btn_next);

    let match_label = Label::builder()
        .label("")
        .css_classes(vec!["dim-label".to_string()])
        .margin_start(6)
        .margin_end(6)
        .build();
    find_row.append(&match_label);

    let btn_close_bar = Button::builder()
        .icon_name("window-close-symbolic")
        .has_frame(false)
        .tooltip_text("Cerrar barra  (Escape)")
        .build();
    find_row.append(&btn_close_bar);

    let replace_row = Box::new(Orientation::Horizontal, 4);
    replace_row.set_margin_start(6);
    replace_row.set_margin_end(6);
    replace_row.set_margin_top(2);
    replace_row.set_margin_bottom(4);

    let replace_icon = gtk::Image::from_icon_name("edit-find-replace-symbolic");
    replace_icon.set_pixel_size(16);
    replace_row.append(&replace_icon);

    let replace_entry = Entry::builder()
        .placeholder_text("Reemplazar con…")
        .hexpand(true)
        .build();
    replace_row.append(&replace_entry);

    let btn_replace = Button::builder()
        .label("Reemplazar")
        .sensitive(false)
        .build();
    let btn_replace_all = Button::builder()
        .label("Reemplazar todo")
        .sensitive(false)
        .build();
    replace_row.append(&btn_replace);
    replace_row.append(&btn_replace_all);

    find_bar_box.append(&find_row);
    find_bar_box.append(&replace_row);

    let find_revealer = gtk::Revealer::builder()
        .child(&find_bar_box)
        .transition_type(gtk::RevealerTransitionType::SlideDown)
        .transition_duration(150)
        .reveal_child(false)
        .build();

    // --- Paned ---
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_start_child(Some(&sidebar_scrolled));
    paned.set_end_child(Some(&main_stack));
    paned.set_resize_start_child(false);
    paned.set_shrink_start_child(false);
    paned.set_resize_end_child(true);
    paned.set_shrink_end_child(false);
    paned.set_position(settings.int("sidebar-width"));
    paned.set_vexpand(true);

    // --- Header bar ---
    let header_bar = HeaderBar::new();

    let sidebar_toggle = Button::builder()
        .icon_name("sidebar-show-symbolic")
        .tooltip_text("Alternar barra lateral")
        .build();
    let open_menu_btn = MenuButton::builder().label("Abrir").build();

    let add_menu = gio::Menu::new();
    add_menu.append(Some("Nuevo Proyecto"), Some("app.new-project"));
    add_menu.append(Some("Nuevo Capítulo"), Some("app.add-chapter"));
    add_menu.append(Some("Importar Capítulos…"), Some("app.import-chapters"));
    add_menu.append(Some("Nueva Hoja de Estilo"), Some("app.add-style"));
    add_menu.append(Some("Importar Imagen"), Some("app.import-image"));
    add_menu.append(Some("Importar Fuente"), Some("app.import-font"));

    let add_btn = MenuButton::builder()
        .icon_name("list-add-symbolic")
        .menu_model(&add_menu)
        .tooltip_text("Agregar recurso")
        .build();

    header_bar.pack_start(&sidebar_toggle);
    header_bar.pack_start(&open_menu_btn);
    header_bar.pack_start(&add_btn);

    let header_title = WindowTitle::new("GutenAIR", "");
    header_title.set_margin_start(12);
    header_bar.pack_start(&header_title);

    let switcher = ViewSwitcher::builder()
        .stack(&main_stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();
    header_bar.set_title_widget(Some(&switcher));

    let menu = gio::Menu::new();
    menu.append(Some("Preferencias"), Some("app.preferences"));
    menu.append(Some("Exportar…"), Some("app.export"));
    menu.append(Some("Tabla de Contenidos…"), Some("app.nav-builder"));
    menu.append(Some("Verificar EPUB"), Some("app.epub-check"));
    menu.append(Some("Atajos de teclado"), Some("app.shortcuts"));
    menu.append(Some("Ayuda"), Some("app.help"));
    menu.append(Some("Acerca de GutenAIR"), Some("app.about"));
    let menu_button = MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();
    let stats_btn = Button::builder()
        .icon_name("document-properties-symbolic")
        .tooltip_text("Informe del capítulo")
        .sensitive(false)
        .build();
    header_bar.pack_end(&menu_button);
    header_bar.pack_end(&stats_btn);

    // --- Layout ---
    let window_box = Box::new(Orientation::Vertical, 0);
    window_box.append(&header_bar);
    window_box.append(&find_revealer);
    window_box.append(&paned);

    // --- Window (created before UiState so we can store a reference) ---
    let window = ApplicationWindow::builder()
        .application(app)
        .title("GutenAIR")
        .default_width(settings.int("window-width"))
        .default_height(settings.int("window-height"))
        .content(&window_box)
        .build();

    if settings.boolean("window-maximized") {
        window.maximize();
    }

    // --- UI state ---
    let ui_state = Rc::new(UiState {
        main_stack,
        editor,
        preview,
        preview_inner: preview_inner.clone(),
        image_viewer: image_viewer.clone(),
        sidebar_box,
        sidebar_scrolled: sidebar_scrolled.clone(),
        paned: paned.clone(),
        settings: settings.clone(),
        window: window.clone(),
        header_title,
        stats_btn: stats_btn.clone(),
        current_path: RefCell::new(None),
        open_item_id: RefCell::new(None),
        open_item_media_type: RefCell::new(None),
        manifest_groups: RefCell::new(Vec::new()),
        selected_items: RefCell::new(Vec::new()),
        last_clicked: RefCell::new(None),
        search_ctx: search_ctx.clone(),
    });

    setup_editor_context_menu(&ui_state);

    // --- Global editor/navigation shortcuts ---
    let open_action = gio::SimpleAction::new("open", None);
    open_action.connect_activate({
        let btn = open_menu_btn.clone();
        move |_, _| btn.popup()
    });
    app.add_action(&open_action);
    app.set_accels_for_action("app.open", &["<Control>o"]);

    let shortcuts_action = gio::SimpleAction::new("shortcuts", None);
    shortcuts_action.connect_activate({
        let win = window.clone();
        move |_, _| show_shortcuts_dialog(&win)
    });
    app.add_action(&shortcuts_action);
    app.set_accels_for_action("app.shortcuts", &["F1"]);

    let toggle_preview_action = gio::SimpleAction::new("toggle-preview", None);
    toggle_preview_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| toggle_editor_preview(&state)
    });
    app.add_action(&toggle_preview_action);
    app.set_accels_for_action("app.toggle-preview", &["<Control>Right"]);

    let toggle_sidebar_action = gio::SimpleAction::new("toggle-sidebar", None);
    toggle_sidebar_action.connect_activate({
        let sidebar = sidebar_scrolled.clone();
        let p = paned.clone();
        let s = settings.clone();
        move |_, _| toggle_sidebar(&sidebar, &p, &s)
    });
    app.add_action(&toggle_sidebar_action);
    app.set_accels_for_action("app.toggle-sidebar", &["<Control><Shift>s"]);

    let ai_action = gio::SimpleAction::new("ai", None);
    ai_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| show_ai_for_selection(&state)
    });
    app.add_action(&ai_action);
    app.set_accels_for_action("app.ai", &["<Control><Shift>i"]);

    let split_paragraph_action = gio::SimpleAction::new("split-paragraph", None);
    split_paragraph_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| split_paragraph_at_cursor(&state)
    });
    app.add_action(&split_paragraph_action);
    app.set_accels_for_action("app.split-paragraph", &["<Control>d"]);

    let split_chapter_action = gio::SimpleAction::new("split-chapter", None);
    split_chapter_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| split_chapter_at_cursor(&state)
    });
    app.add_action(&split_chapter_action);
    app.set_accels_for_action("app.split-chapter", &["<Control><Shift>d"]);

    let strip_tags_action = gio::SimpleAction::new("strip-tags", None);
    strip_tags_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| strip_tags_from_selection(&state)
    });
    app.add_action(&strip_tags_action);
    app.set_accels_for_action("app.strip-tags", &["<Control>Delete"]);

    let unordered_list_action = gio::SimpleAction::new("list-unordered", None);
    unordered_list_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| create_list_from_selection(&state, gutencore::ListKind::Unordered)
    });
    app.add_action(&unordered_list_action);
    app.set_accels_for_action("app.list-unordered", &["<Control>a"]);

    let ordered_list_action = gio::SimpleAction::new("list-ordered", None);
    ordered_list_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| create_list_from_selection(&state, gutencore::ListKind::Ordered)
    });
    app.add_action(&ordered_list_action);
    app.set_accels_for_action("app.list-ordered", &["<Control><Shift>a"]);

    for (action_name, tag, accel) in [
        ("format-strong", "strong", "<Control>b"),
        ("format-em", "em", "<Control>k"),
        ("format-h1", "h1", "<Control>h"),
        ("format-p", "p", "<Control>g"),
    ] {
        let action = gio::SimpleAction::new(action_name, None);
        action.connect_activate({
            let state = ui_state.clone();
            let tag = tag.to_string();
            move |_, _| apply_tag_to_selection(&state, tag.clone())
        });
        app.add_action(&action);
        app.set_accels_for_action(&format!("app.{}", action_name), &[accel]);
    }

    // --- Proyectos recientes (Ctrl+1..5) ---
    for i in 1u32..=5 {
        let action = gio::SimpleAction::new(&format!("open-recent-{i}"), None);
        action.connect_activate({
            let settings = settings.clone();
            let state = ui_state.clone();
            move |_, _| {
                let history: Vec<String> = settings
                    .strv("history")
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect();
                if let Some(path) = history.get((i - 1) as usize) {
                    load_path(path, &state);
                }
            }
        });
        app.add_action(&action);
        app.set_accels_for_action(&format!("app.open-recent-{i}"), &[&format!("<Control>{i}")]);
    }

    // --- Find / Replace bar signals ---

    // Actualizar texto de búsqueda y sensibilidad de botones al escribir
    find_entry.connect_changed({
        let settings = search_settings.clone();
        let ctx = search_ctx.clone();
        let lbl = match_label.clone();
        let btn_p = btn_prev.clone();
        let btn_n = btn_next.clone();
        let btn_r = btn_replace.clone();
        let btn_ra = btn_replace_all.clone();
        move |entry| {
            let text = entry.text();
            let has_text = !text.is_empty();
            settings.set_search_text(if has_text { Some(text.as_str()) } else { None });
            let count = ctx.occurrences_count();
            lbl.set_label(&format_match_count(count, has_text));
            btn_p.set_sensitive(has_text);
            btn_n.set_sensitive(has_text);
            btn_r.set_sensitive(has_text);
            btn_ra.set_sensitive(has_text);
        }
    });

    // Actualizar contador cuando GtkSourceView termina de contar
    search_ctx.connect_occurrences_count_notify({
        let lbl = match_label.clone();
        let fe = find_entry.clone();
        move |ctx| {
            let has_text = !fe.text().is_empty();
            lbl.set_label(&format_match_count(ctx.occurrences_count(), has_text));
        }
    });

    // Siguiente
    btn_next.connect_clicked({
        let state = ui_state.clone();
        move |_| navigate_search(&state, true)
    });

    // Anterior
    btn_prev.connect_clicked({
        let state = ui_state.clone();
        move |_| navigate_search(&state, false)
    });

    // Intro en la entrada de búsqueda → siguiente
    find_entry.connect_activate({
        let state = ui_state.clone();
        move |_| navigate_search(&state, true)
    });

    // Reemplazar coincidencia actual
    btn_replace.connect_clicked({
        let state = ui_state.clone();
        let re = replace_entry.clone();
        move |_| {
            let buffer = state.editor.buffer();
            if let Some((mut start, mut end)) = buffer.selection_bounds() {
                let replace_text = re.text();
                let _ = state
                    .search_ctx
                    .replace(&mut start, &mut end, replace_text.as_str());
                save_current_item(&state);
            }
            navigate_search(&state, true);
        }
    });

    // Intro en la entrada de reemplazo → reemplazar y avanzar
    replace_entry.connect_activate({
        let state = ui_state.clone();
        let re_clone = replace_entry.clone();
        move |_| {
            let buffer = state.editor.buffer();
            if let Some((mut start, mut end)) = buffer.selection_bounds() {
                let replace_text = re_clone.text();
                let _ = state
                    .search_ctx
                    .replace(&mut start, &mut end, replace_text.as_str());
                save_current_item(&state);
            }
            navigate_search(&state, true);
        }
    });

    // Reemplazar todo
    btn_replace_all.connect_clicked({
        let state = ui_state.clone();
        let re = replace_entry.clone();
        move |_| {
            let replace_text = re.text();
            if let Err(e) = state.search_ctx.replace_all(replace_text.as_str()) {
                eprintln!("[find/replace] replace_all error: {}", e);
            } else {
                save_current_item(&state);
            }
        }
    });

    // Cerrar barra
    btn_close_bar.connect_clicked({
        let rev = find_revealer.clone();
        let settings = search_settings.clone();
        let lbl = match_label.clone();
        let btn_p = btn_prev.clone();
        let btn_n = btn_next.clone();
        let btn_r = btn_replace.clone();
        let btn_ra = btn_replace_all.clone();
        move |_| {
            rev.set_reveal_child(false);
            settings.set_search_text(None);
            lbl.set_label("");
            btn_p.set_sensitive(false);
            btn_n.set_sensitive(false);
            btn_r.set_sensitive(false);
            btn_ra.set_sensitive(false);
        }
    });

    // Escape en la entrada de búsqueda → cerrar barra
    let key_ctrl = gtk::EventControllerKey::new();
    key_ctrl.connect_key_pressed({
        let rev = find_revealer.clone();
        let settings = search_settings.clone();
        move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                rev.set_reveal_child(false);
                settings.set_search_text(None);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    });
    find_entry.add_controller(key_ctrl);

    // Forzar guardado al cambiar a la vista previa
    ui_state
        .main_stack
        .connect_notify_local(Some("visible-child"), {
            let state = ui_state.clone();
            move |stack, _| {
                if stack.visible_child_name().as_deref() == Some("preview") {
                    save_current_item(&state);
                    state.preview.reload();
                }
            }
        });

    // --- Find / Replace action ---
    let find_replace_action = gio::SimpleAction::new("find-replace", None);
    find_replace_action.connect_activate({
        let rev = find_revealer.clone();
        let fe = find_entry.clone();
        move |_, _| {
            rev.set_reveal_child(true);
            fe.grab_focus();
        }
    });
    app.add_action(&find_replace_action);
    app.set_accels_for_action("app.find-replace", &["<Control>f"]);

    // --- Actions: Add Resources ---
    let new_project_action = gio::SimpleAction::new("new-project", None);
    new_project_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_new_project_dialog(&win, &state)
    });
    app.add_action(&new_project_action);

    let import_chapters_action = gio::SimpleAction::new("import-chapters", None);
    import_chapters_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_import_chapters_dialog(&win, &state)
    });
    app.add_action(&import_chapters_action);
    app.set_accels_for_action("app.import-chapters", &["<Control>t"]);

    let add_chapter_action = gio::SimpleAction::new("add-chapter", None);
    add_chapter_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| {
            show_add_chapters_dialog(&win, &state);
        }
    });
    app.add_action(&add_chapter_action);
    app.set_accels_for_action("app.add-chapter", &["<Control>n"]);

    let add_style_action = gio::SimpleAction::new("add-style", None);
    add_style_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| {
            show_add_resource_dialog(&win, &state, "Estilo", "Styles", "text/css");
        }
    });
    app.add_action(&add_style_action);

    let import_image_action = gio::SimpleAction::new("import-image", None);
    import_image_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| {
            show_import_dialog(&win, &state, "Imagen", "Images", "image/*");
        }
    });
    app.add_action(&import_image_action);

    let import_font_action = gio::SimpleAction::new("import-font", None);
    import_font_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| {
            show_import_dialog(&win, &state, "Fuente", "Fonts", "font/*");
        }
    });
    app.add_action(&import_font_action);

    // --- Wire up preferences action ---
    let action_pref = gio::SimpleAction::new("preferences", None);
    action_pref.connect_activate({
        let win = window.clone();
        let s = settings.clone();
        let uis = ui_state.clone();
        move |_, _| show_preferences(&win, &s, &uis)
    });
    app.add_action(&action_pref);

    // --- Chapter / book report actions + shortcuts ---
    let chapter_report_action = gio::SimpleAction::new("chapter-report", None);
    chapter_report_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| show_chapter_report(&state)
    });
    app.add_action(&chapter_report_action);
    app.set_accels_for_action("app.chapter-report", &["<Control>i"]);

    let book_report_action = gio::SimpleAction::new("book-report", None);
    book_report_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| show_book_report(&state)
    });
    app.add_action(&book_report_action);
    app.set_accels_for_action("app.book-report", &["<Control><Alt>i"]);

    // --- Export action ---
    let export_action = gio::SimpleAction::new("export", None);
    export_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_export_dialog(&win, &state)
    });
    app.add_action(&export_action);
    app.set_accels_for_action("app.export", &["<Control><Shift>t"]);

    // --- Nav builder action ---
    let nav_builder_action = gio::SimpleAction::new("nav-builder", None);
    nav_builder_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_nav_builder_dialog(&win, &state)
    });
    app.add_action(&nav_builder_action);
    app.set_accels_for_action("app.nav-builder", &["<Control><Shift>n"]);

    // --- EPUB check action + shortcut ---
    let epub_check_action = gio::SimpleAction::new("epub-check", None);
    epub_check_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| show_epub_check(&state)
    });
    app.add_action(&epub_check_action);
    app.set_accels_for_action("app.epub-check", &["<Control><Shift>v"]);

    // --- Guardar archivo actual (Ctrl+S) ---
    let save_action = gio::SimpleAction::new("save", None);
    save_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| save_current_item(&state)
    });
    app.add_action(&save_action);
    app.set_accels_for_action("app.save", &["<Control>s"]);

    // --- Stats button ---
    stats_btn.connect_clicked({
        let state = ui_state.clone();
        move |_| show_chapter_report(&state)
    });

    // --- Sidebar toggle ---
    sidebar_toggle.connect_clicked({
        let sidebar = sidebar_scrolled.clone();
        let p = paned.clone();
        let s = settings.clone();
        move |_| {
            toggle_sidebar(&sidebar, &p, &s);
        }
    });

    // --- Sidebar width persistence ---
    paned.connect_position_notify({
        let sidebar = sidebar_scrolled.clone();
        let s = settings.clone();
        move |p| {
            if sidebar.is_visible() && p.position() > 10 {
                let _ = s.set_int("sidebar-width", p.position());
            }
        }
    });

    // --- Open popover ---
    let popover = Popover::new();
    let popover_box = Box::new(Orientation::Vertical, 6);
    popover_box.set_margin_start(6);
    popover_box.set_margin_end(6);
    popover_box.set_margin_top(6);
    popover_box.set_margin_bottom(6);
    popover_box.set_width_request(320);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Buscar en recientes...")
        .hexpand(true)
        .build();
    popover_box.append(&search_entry);

    let browse_folder_btn = Button::builder()
        .label("Abrir carpeta")
        .icon_name("folder-symbolic")
        .hexpand(true)
        .build();
    let browse_epub_btn = Button::builder()
        .label("Abrir .epub")
        .icon_name("document-open-symbolic")
        .hexpand(true)
        .build();
    let btn_box = Box::new(Orientation::Horizontal, 6);
    btn_box.append(&browse_folder_btn);
    btn_box.append(&browse_epub_btn);
    popover_box.append(&btn_box);

    let hist_list_box = ListBox::new();
    hist_list_box.add_css_class("boxed-list");
    hist_list_box.set_selection_mode(gtk::SelectionMode::None);
    let hist_scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(200)
        .child(&hist_list_box)
        .build();
    popover_box.append(&hist_scrolled);

    popover.set_child(Some(&popover_box));
    open_menu_btn.set_popover(Some(&popover));

    let update_hist_list = {
        let list = hist_list_box.clone();
        let settings = settings.clone();
        let search = search_entry.clone();
        let state = ui_state.clone();
        let pop = popover.clone();

        move || {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let filter = search.text().to_lowercase();
            let history = settings.strv("history");
            let mut count = 0;

            for path_glib in history.into_iter() {
                if count >= 15 {
                    break;
                }
                let path_str = path_glib.to_string();
                let path = Path::new(&path_str);
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path_str);

                if filter.is_empty()
                    || path_str.to_lowercase().contains(&filter)
                    || name.to_lowercase().contains(&filter)
                {
                    let is_epub = path_str.to_lowercase().ends_with(".epub");

                    let icon = Image::from_icon_name(if is_epub {
                        "x-office-document-symbolic"
                    } else {
                        "folder-symbolic"
                    });
                    icon.set_pixel_size(16);

                    let del = Button::builder()
                        .icon_name("user-trash-symbolic")
                        .has_frame(false)
                        .valign(gtk::Align::Center)
                        .build();

                    let row = ActionRow::builder()
                        .title(name)
                        .subtitle(path_str.as_str())
                        .subtitle_lines(1)
                        .activatable(true)
                        .build();
                    row.add_prefix(&icon);
                    row.add_suffix(&del);

                    let state_load = state.clone();
                    let p_str = path_str.clone();
                    let pop_close = pop.clone();
                    row.connect_activated(move |_| {
                        load_path(&p_str, &state_load);
                        pop_close.popdown();
                    });

                    let settings_del = settings.clone();
                    let p_del = path_str.clone();
                    del.connect_clicked(move |_| {
                        let mut h: Vec<String> = settings_del
                            .strv("history")
                            .into_iter()
                            .map(|s| s.to_string())
                            .collect();
                        h.retain(|p| p != &p_del);
                        let refs: Vec<&str> = h.iter().map(|s| s.as_str()).collect();
                        let _ = settings_del.set_strv("history", refs);
                    });

                    list.append(&row);
                    count += 1;
                }
            }
        }
    };

    settings.connect_changed(Some("history"), {
        let update = update_hist_list.clone();
        move |_, _| update()
    });
    search_entry.connect_search_changed({
        let update = update_hist_list.clone();
        move |_| update()
    });
    popover.connect_show({
        let update = update_hist_list.clone();
        move |_| update()
    });

    browse_folder_btn.connect_clicked({
        let win = window.clone();
        let state = ui_state.clone();
        let pop = popover.clone();
        move |_| {
            let native = FileChooserNative::new(
                Some("Abrir Carpeta de Proyecto"),
                Some(&win),
                FileChooserAction::SelectFolder,
                Some("Abrir"),
                Some("Cancelar"),
            );
            let state = state.clone();
            let pop = pop.clone();
            native.connect_response(move |n, res| {
                if res == ResponseType::Accept {
                    if let Some(f) = n.file() {
                        if let Some(p) = f.path() {
                            load_book(&p.to_string_lossy(), &state);
                            pop.popdown();
                        }
                    }
                }
                n.destroy();
            });
            native.show();
        }
    });

    browse_epub_btn.connect_clicked({
        let win = window.clone();
        let state = ui_state.clone();
        let pop = popover.clone();
        move |_| {
            let native = FileChooserNative::new(
                Some("Abrir archivo EPUB"),
                Some(&win),
                FileChooserAction::Open,
                Some("Abrir"),
                Some("Cancelar"),
            );
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("EPUB"));
            filter.add_pattern("*.epub");
            native.add_filter(&filter);
            let state = state.clone();
            let pop = pop.clone();
            native.connect_response(move |n, res| {
                if res == ResponseType::Accept {
                    if let Some(f) = n.file() {
                        if let Some(p) = f.path() {
                            load_path(&p.to_string_lossy(), &state);
                            pop.popdown();
                        }
                    }
                }
                n.destroy();
            });
            native.show();
        }
    });

    // Window size persistence
    window.connect_default_width_notify({
        let s = settings.clone();
        move |win| {
            if !win.is_maximized() {
                let _ = s.set_int("window-width", win.default_width());
            }
        }
    });
    window.connect_default_height_notify({
        let s = settings.clone();
        move |win| {
            if !win.is_maximized() {
                let _ = s.set_int("window-height", win.default_height());
            }
        }
    });
    window.connect_maximized_notify({
        let s = settings.clone();
        move |win| {
            let _ = s.set_boolean("window-maximized", win.is_maximized());
        }
    });

    // --- Guardar al cerrar la ventana ---
    window.connect_close_request({
        let state = ui_state.clone();
        move |_| {
            save_current_item(&state);
            glib::Propagation::Proceed
        }
    });

    window.present();
}
