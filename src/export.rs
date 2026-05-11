use crate::prelude::*;

pub(crate) fn show_export_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let dialog = adw::Window::builder()
        .title("Exportar")
        .modal(true)
        .transient_for(parent)
        .default_width(360)
        .build();

    let hbar = HeaderBar::new();
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let group = PreferencesGroup::builder()
        .title("Formato de exportación")
        .description("Elegí el formato al que querés exportar el libro.")
        .margin_start(24)
        .margin_end(24)
        .margin_top(24)
        .margin_bottom(24)
        .build();

    let text_row = adw::ActionRow::builder()
        .title("Texto plano")
        .subtitle("Exporta los capítulos seleccionados como archivo .txt")
        .activatable(true)
        .build();
    let text_chevron = gtk::Image::from_icon_name("go-next-symbolic");
    text_row.add_suffix(&text_chevron);

    let epub_row = adw::ActionRow::builder()
        .title("EPUB")
        .subtitle("Exporta el libro como archivo .epub")
        .activatable(true)
        .build();
    let epub_chevron = gtk::Image::from_icon_name("go-next-symbolic");
    epub_row.add_suffix(&epub_chevron);

    group.add(&text_row);
    group.add(&epub_row);
    vbox.append(&group);
    dialog.set_content(Some(&vbox));

    {
        let parent = parent.clone().upcast::<gtk::Window>();
        let state = state.clone();
        let dialog_ref = dialog.clone();
        text_row.connect_activated(move |_| {
            dialog_ref.destroy();
            show_export_text_dialog(&parent, &state);
        });
    }

    {
        let parent = parent.clone().upcast::<gtk::Window>();
        let state = state.clone();
        let dialog_ref = dialog.clone();
        epub_row.connect_activated(move |_| {
            dialog_ref.destroy();
            show_export_epub_dialog(&parent, &state);
        });
    }

    dialog.present();
}

pub(crate) fn show_export_epub_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    // Nombre sugerido: título del libro o nombre de carpeta
    let suggested = {
        let core = gutencore::GutenCore::open_folder(&path).ok();
        core.and_then(|c| c.metadata.as_ref().map(|m| m.title.clone()))
            .unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("libro")
                    .to_string()
            })
    };

    let native = FileChooserNative::new(
        Some("Guardar EPUB"),
        Some(parent),
        FileChooserAction::Save,
        Some("Exportar"),
        Some("Cancelar"),
    );
    native.set_current_name(&format!("{}.epub", suggested));

    let state_c = state.clone();
    native.connect_response(move |n, res| {
        if res != ResponseType::Accept {
            return;
        }
        let out_path = match n.file().and_then(|f| f.path()) {
            Some(p) => p,
            None => return,
        };
        let out_path = if out_path.extension().map(|e| e != "epub").unwrap_or(true) {
            out_path.with_extension("epub")
        } else {
            out_path
        };

        let mut core = match gutencore::GutenCore::open_folder(
            &state_c.current_path.borrow().clone().unwrap_or_default(),
        ) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("export epub: {}", e);
                return;
            }
        };

        match core.export_epub(&out_path) {
            Ok(_) => eprintln!("export epub: guardado en {}", out_path.display()),
            Err(e) => eprintln!("export epub ERROR: {}", e),
        }
    });

    native.show();
}

pub(crate) fn show_export_text_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("export: {}", e);
            return;
        }
    };

    // Spine chapters in order
    let chapters: Vec<(String, String)> = core
        .spine
        .iter()
        .filter_map(|id| {
            core.manifest.get(id).map(|item| {
                let label = std::path::Path::new(&item.href)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(id)
                    .to_string();
                (id.clone(), label)
            })
        })
        .collect();

    let output_dir: Rc<RefCell<String>> = Rc::new(RefCell::new(path.clone()));

    let dialog = adw::Window::builder()
        .title("Exportar como texto")
        .modal(true)
        .transient_for(parent)
        .default_width(460)
        .resizable(false)
        .build();

    let hbar = HeaderBar::new();

    let export_btn = Button::builder()
        .label("Exportar")
        .css_classes(["suggested-action"])
        .build();
    hbar.pack_end(&export_btn);

    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(200)
        .max_content_height(400)
        .build();
    let inner = Box::new(Orientation::Vertical, 0);

    // Chapter selection group
    let chap_group = PreferencesGroup::builder()
        .title("Capítulos a exportar")
        .description("Seleccioná los capítulos en el orden del spine.")
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(6)
        .build();

    // Select all / none header buttons
    let sel_box = Box::new(Orientation::Horizontal, 6);
    sel_box.set_margin_bottom(4);
    let sel_all_btn = Button::builder().label("Todos").has_frame(false).build();
    let sel_none_btn = Button::builder().label("Ninguno").has_frame(false).build();
    sel_all_btn.add_css_class("caption");
    sel_none_btn.add_css_class("caption");
    sel_box.append(&sel_all_btn);
    sel_box.append(&sel_none_btn);
    chap_group.set_header_suffix(Some(&sel_box));

    let checks: Rc<Vec<gtk::CheckButton>> = Rc::new(
        chapters
            .iter()
            .map(|(_, label)| {
                gtk::CheckButton::builder()
                    .label(label.as_str())
                    .active(true)
                    .build()
            })
            .collect(),
    );

    for (i, (_, label)) in chapters.iter().enumerate() {
        let row = ActionRow::builder()
            .title(label.as_str())
            .activatable_widget(&checks[i])
            .build();
        row.add_prefix(&checks[i]);
        chap_group.add(&row);
    }

    // Select all / none handlers
    {
        let checks = checks.clone();
        sel_all_btn.connect_clicked(move |_| {
            for c in checks.iter() {
                c.set_active(true);
            }
        });
    }
    {
        let checks = checks.clone();
        sel_none_btn.connect_clicked(move |_| {
            for c in checks.iter() {
                c.set_active(false);
            }
        });
    }

    inner.append(&chap_group);

    // Output directory group
    let dest_group = PreferencesGroup::builder()
        .title("Carpeta de destino")
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(24)
        .build();

    let dest_row = ActionRow::builder()
        .title(path.as_str())
        .subtitle("Por defecto: carpeta del proyecto")
        .build();
    let dest_row = Rc::new(dest_row);

    let choose_btn = Button::builder()
        .icon_name("folder-open-symbolic")
        .tooltip_text("Elegir carpeta")
        .valign(gtk::Align::Center)
        .build();

    {
        let output_dir = output_dir.clone();
        let dest_row = dest_row.clone();
        let dialog_ref = dialog.clone();
        choose_btn.connect_clicked(move |_| {
            let native = FileChooserNative::new(
                Some("Carpeta de destino"),
                Some(&dialog_ref),
                FileChooserAction::SelectFolder,
                Some("Seleccionar"),
                Some("Cancelar"),
            );
            let od = output_dir.clone();
            let dr = dest_row.clone();
            native.connect_response(move |n, res| {
                if res == ResponseType::Accept {
                    if let Some(p) = n.file().and_then(|f| f.path()) {
                        let s = p.to_string_lossy().to_string();
                        *od.borrow_mut() = s.clone();
                        dr.set_title(s.as_str());
                    }
                }
                n.destroy();
            });
            native.show();
        });
    }

    dest_row.add_suffix(&choose_btn);
    dest_group.add(&*dest_row);
    inner.append(&dest_group);

    scrolled.set_child(Some(&inner));
    vbox.append(&scrolled);
    dialog.set_content(Some(&vbox));

    // Export button handler
    {
        let checks = checks.clone();
        let chapters = chapters.clone();
        let output_dir = output_dir.clone();
        let dialog_ref = dialog.clone();
        export_btn.connect_clicked(move |_| {
            let selected_ids: Vec<String> = chapters
                .iter()
                .enumerate()
                .filter(|(i, _)| checks[*i].is_active())
                .map(|(_, (id, _))| id.clone())
                .collect();

            if selected_ids.is_empty() {
                return;
            }

            if let Ok(core) = gutencore::GutenCore::open_folder(&path) {
                let dir = output_dir.borrow().clone();
                match core.export_to_text_file(&dir, None, Some(selected_ids)) {
                    Ok(out_path) => eprintln!("export: guardado en {}", out_path.display()),
                    Err(e) => eprintln!("export: {}", e),
                }
            }
            dialog_ref.destroy();
        });
    }

    dialog.present();
}

// ─── Nav builder ─────────────────────────────────────────────────────────────
