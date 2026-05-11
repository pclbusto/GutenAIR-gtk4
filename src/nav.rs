use crate::prelude::*;

pub(crate) fn nav_state_path(project_path: &str) -> std::path::PathBuf {
    std::path::Path::new(project_path).join(".gutenair_nav.json")
}

pub(crate) fn load_nav_state(project_path: &str) -> Option<Vec<gutencore::DocToc>> {
    let content = std::fs::read_to_string(nav_state_path(project_path)).ok()?;
    serde_json::from_str(&content).ok()
}

pub(crate) fn save_nav_state(project_path: &str, data: &[gutencore::DocToc]) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(nav_state_path(project_path), json);
    }
}

// Fusiona el escaneo fresco con el estado guardado: preserva los flags include
// para los headings que ya existían, agrega nuevos como include=true.

pub(crate) fn merge_toc_data(
    fresh: Vec<gutencore::DocToc>,
    saved: Option<&[gutencore::DocToc]>,
) -> Vec<gutencore::DocToc> {
    let saved = match saved {
        Some(s) => s,
        None => return fresh,
    };
    fresh
        .into_iter()
        .map(|mut doc| {
            if let Some(saved_doc) = saved.iter().find(|d| d.href == doc.href) {
                doc.include = saved_doc.include;
                for item in &mut doc.items {
                    if let Some(saved_item) = saved_doc
                        .items
                        .iter()
                        .find(|i| i.level == item.level && i.anchor == item.anchor)
                    {
                        item.include = saved_item.include;
                    }
                }
            }
            doc
        })
        .collect()
}

pub(crate) fn show_nav_builder_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let project_path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&project_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("nav: {}", e);
            return;
        }
    };

    let fresh = match core.get_full_toc_data() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("nav: {}", e);
            return;
        }
    };

    let saved = load_nav_state(&project_path);
    let merged = merge_toc_data(fresh, saved.as_deref());
    let toc_data: Rc<RefCell<Vec<gutencore::DocToc>>> = Rc::new(RefCell::new(merged));

    let dialog = adw::Window::builder()
        .title(tr("nav.title"))
        .modal(true)
        .transient_for(parent)
        .default_width(650)
        .default_height(550)
        .build();

    let vbox_main = Box::new(Orientation::Vertical, 8);
    vbox_main.set_margin_start(16);
    vbox_main.set_margin_end(16);
    vbox_main.set_margin_top(16);
    vbox_main.set_margin_bottom(16);

    let hbox_content = Box::new(Orientation::Horizontal, 16);
    hbox_content.set_vexpand(true);

    // Left side: List area
    let vbox_list = Box::new(Orientation::Vertical, 4);
    vbox_list.set_hexpand(true);
    vbox_list.set_vexpand(true);

    // Headers
    let hbox_headers = Box::new(Orientation::Horizontal, 8);
    hbox_headers.set_margin_start(4);
    hbox_headers.set_margin_end(4);

    let lbl_hdr_title = Label::builder()
        .label(tr("nav.header.title"))
        .xalign(0.0)
        .hexpand(true)
        .build();
    let lbl_hdr_level = Label::builder()
        .label(tr("nav.header.level"))
        .width_request(80)
        .xalign(0.0)
        .build();
    let lbl_hdr_incl = Label::builder()
        .label(tr("nav.header.include"))
        .width_request(60)
        .build();
    lbl_hdr_title.add_css_class("caption-heading");
    lbl_hdr_level.add_css_class("caption-heading");
    lbl_hdr_incl.add_css_class("caption-heading");

    hbox_headers.append(&lbl_hdr_title);
    hbox_headers.append(&lbl_hdr_level);
    hbox_headers.append(&lbl_hdr_incl);

    let sep1 = gtk::Separator::new(Orientation::Horizontal);
    vbox_list.append(&hbox_headers);
    vbox_list.append(&sep1);

    // ListBox inside ScrolledWindow
    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .has_frame(true)
        .build();

    let list_box = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();
    list_box.add_css_class("navigation-sidebar");

    scrolled.set_child(Some(&list_box));
    vbox_list.append(&scrolled);
    hbox_content.append(&vbox_list);

    // Right side: buttons
    let vbox_right = Box::new(Orientation::Vertical, 12);
    let btn_rename = Button::builder().label(tr("nav.rename")).build();

    let hbox_arrows = Box::new(Orientation::Horizontal, 6);
    hbox_arrows.set_halign(gtk::Align::Center);
    let btn_up = Button::builder().icon_name("go-up-symbolic").build();
    let btn_down = Button::builder().icon_name("go-down-symbolic").build();
    hbox_arrows.append(&btn_up);
    hbox_arrows.append(&btn_down);

    vbox_right.append(&btn_rename);
    vbox_right.append(&hbox_arrows);

    hbox_content.append(&vbox_right);
    vbox_main.append(&hbox_content);

    #[derive(Clone)]
    enum TocElement {
        Doc(usize),
        Item(usize, usize),
    }

    let mut row_map = Vec::new();

    {
        let data = toc_data.borrow();
        for (d_idx, doc) in data.iter().enumerate() {
            let row_box = Box::new(Orientation::Horizontal, 8);
            row_box.set_margin_start(4);
            row_box.set_margin_top(4);
            row_box.set_margin_bottom(4);
            row_box.set_margin_end(4);

            let icon_name = "text-x-generic-symbolic";
            let icon = Image::from_icon_name(icon_name);

            let title_text = if doc.title.is_empty() {
                &doc.href
            } else {
                &doc.title
            };
            let lbl_title = Label::builder()
                .label(title_text)
                .xalign(0.0)
                .hexpand(true)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();

            let lbl_lvl = Label::builder()
                .label("doc")
                .width_request(80)
                .xalign(0.0)
                .build();

            let chk_incl = gtk::CheckButton::builder()
                .active(doc.include)
                .halign(gtk::Align::Center)
                .width_request(60)
                .build();

            let title_box = Box::new(Orientation::Horizontal, 6);
            title_box.set_hexpand(true);
            title_box.append(&icon);
            title_box.append(&lbl_title);

            row_box.append(&title_box);
            row_box.append(&lbl_lvl);
            row_box.append(&chk_incl);

            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&row_box));
            list_box.append(&row);
            row_map.push(TocElement::Doc(d_idx));

            let toc = toc_data.clone();
            chk_incl.connect_toggled(move |b| {
                toc.borrow_mut()[d_idx].include = b.is_active();
            });

            for (i_idx, item) in doc.items.iter().enumerate() {
                let row_box = Box::new(Orientation::Horizontal, 8);
                row_box.set_margin_start(24 + (item.level as i32 * 12));
                row_box.set_margin_top(2);
                row_box.set_margin_bottom(2);
                row_box.set_margin_end(4);

                let title_text = if item.title.is_empty() {
                    tr("common.untitled")
                } else {
                    &item.title
                };
                let lbl_title = Label::builder()
                    .label(title_text)
                    .xalign(0.0)
                    .hexpand(true)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .build();

                let lbl_lvl = Label::builder()
                    .label(&format!("h{}", item.level))
                    .width_request(80)
                    .xalign(0.0)
                    .build();
                lbl_lvl.add_css_class("dim-label");

                let chk_incl = gtk::CheckButton::builder()
                    .active(item.include)
                    .halign(gtk::Align::Center)
                    .width_request(60)
                    .build();

                row_box.append(&lbl_title);
                row_box.append(&lbl_lvl);
                row_box.append(&chk_incl);

                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&row_box));
                list_box.append(&row);
                row_map.push(TocElement::Item(d_idx, i_idx));

                let toc = toc_data.clone();
                chk_incl.connect_toggled(move |b| {
                    toc.borrow_mut()[d_idx].items[i_idx].include = b.is_active();
                });
            }
        }
    }

    // Bottom section
    let hbox_bottom = Box::new(Orientation::Horizontal, 12);
    hbox_bottom.set_margin_top(8);

    let vbox_bottom_left = Box::new(Orientation::Vertical, 6);
    let chk_show_only = gtk::CheckButton::builder()
        .label(tr("nav.show_only"))
        .build();

    #[derive(Clone)]
    enum NavPreset {
        Placeholder,
        MarkAll,
        MarkLevel(u8),
        ClearAll,
    }

    let heading_levels: std::collections::BTreeSet<u8> = toc_data
        .borrow()
        .iter()
        .flat_map(|doc| doc.items.iter().map(|item| item.level))
        .collect();

    let mut preset_actions = vec![NavPreset::Placeholder, NavPreset::MarkAll];
    let mut preset_labels = vec![
        tr("nav.select_headings").to_string(),
        tr("nav.mark_all").to_string(),
    ];
    for level in heading_levels {
        preset_actions.push(NavPreset::MarkLevel(level));
        preset_labels.push(format!("Marcar H{}", level));
    }
    preset_actions.push(NavPreset::ClearAll);
    preset_labels.push(tr("nav.clear_all").to_string());

    let preset_label_refs: Vec<&str> = preset_labels.iter().map(|s| s.as_str()).collect();
    let model = gtk::StringList::new(&preset_label_refs);
    let combo = gtk::DropDown::new(Some(model), gtk::Expression::NONE);

    vbox_bottom_left.append(&chk_show_only);
    vbox_bottom_left.append(&combo);
    hbox_bottom.append(&vbox_bottom_left);

    let btn_ok = Button::builder()
        .label(tr("common.accept"))
        .css_classes(["suggested-action"])
        .width_request(80)
        .build();
    let btn_cancel = Button::builder()
        .label(tr("common.cancel"))
        .width_request(80)
        .build();

    let hbox_ok_cancel = Box::new(Orientation::Horizontal, 8);
    hbox_ok_cancel.set_halign(gtk::Align::End);
    hbox_ok_cancel.set_valign(gtk::Align::End);
    hbox_ok_cancel.set_hexpand(true);
    hbox_ok_cancel.append(&btn_ok);
    hbox_ok_cancel.append(&btn_cancel);

    hbox_bottom.append(&hbox_ok_cancel);
    vbox_main.append(&hbox_bottom);

    let dialog_ref = dialog.clone();
    btn_cancel.connect_clicked(move |_| {
        dialog_ref.destroy();
    });

    let toc = toc_data.clone();
    let list_box_c = list_box.clone();
    let row_map_c = row_map.clone();
    let preset_actions_c = preset_actions.clone();
    combo.connect_selected_notify(move |cb| {
        let sel = cb.selected();
        let Some(action) = preset_actions_c.get(sel as usize).cloned() else {
            return;
        };
        if matches!(action, NavPreset::Placeholder) {
            return;
        }

        let mut data = toc.borrow_mut();
        match action {
            NavPreset::Placeholder => {}
            NavPreset::MarkAll => {
                for doc in data.iter_mut() {
                    doc.include = true;
                    for item in doc.items.iter_mut() {
                        item.include = true;
                    }
                }
            }
            NavPreset::MarkLevel(level) => {
                for doc in data.iter_mut() {
                    let mut has_included_heading = false;
                    for item in doc.items.iter_mut() {
                        item.include = item.level == level;
                        has_included_heading |= item.include;
                    }
                    doc.include = has_included_heading;
                }
            }
            NavPreset::ClearAll => {
                for doc in data.iter_mut() {
                    doc.include = false;
                    for item in doc.items.iter_mut() {
                        item.include = false;
                    }
                }
            }
        }
        drop(data);

        let mut i = 0;
        let mut child = list_box_c.first_child();
        while let Some(widget) = child {
            if let Ok(row) = widget.clone().downcast::<gtk::ListBoxRow>() {
                if let Some(row_content) = row.child() {
                    let box_child = row_content.downcast::<gtk::Box>().unwrap();
                    if let Some(chk_widget) = box_child.last_child() {
                        if let Ok(chk) = chk_widget.downcast::<gtk::CheckButton>() {
                            if let Some(el) = row_map_c.get(i) {
                                let inc = match el {
                                    TocElement::Doc(d) => toc.borrow()[*d].include,
                                    TocElement::Item(d, i) => toc.borrow()[*d].items[*i].include,
                                };
                                if chk.is_active() != inc {
                                    chk.set_active(inc);
                                }
                            }
                        }
                    }
                }
            }
            child = widget.next_sibling();
            i += 1;
        }
    });

    let toc = toc_data.clone();
    let path = project_path.clone();
    let dialog_ref = dialog.clone();
    btn_ok.connect_clicked(move |_| {
        let data = toc.borrow().clone();
        save_nav_state(&path, &data);

        let mut core = match gutencore::GutenCore::open_folder(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("nav gen: {}", e);
                return;
            }
        };
        match core.build_nav_from_data(&data) {
            Ok(_) => match core.save() {
                Ok(_) => {
                    if let Err(e) = core.build_nav_from_data(&data) {
                        eprintln!("nav gen ERROR después de save: {}", e);
                    } else {
                        eprintln!("nav: generado y guardado");
                    }
                }
                Err(e) => eprintln!("nav save: {}", e),
            },
            Err(e) => eprintln!("nav gen ERROR: {}", e),
        }
        dialog_ref.destroy();
    });

    dialog.set_content(Some(&vbox_main));
    dialog.present();
}
