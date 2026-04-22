use adw::prelude::*;
use adw::{
    ActionRow, Application, ApplicationWindow, ComboRow, ExpanderRow, HeaderBar,
    PreferencesDialog, PreferencesGroup, PreferencesPage, ViewStack, ViewSwitcher, WindowTitle,
    Window as AdwWindow, AboutWindow,
};
use gtk::{
    Box, Button, Entry, FileChooserAction, FileChooserNative, GestureClick, Image, Label, ListBox,
    MenuButton, Orientation, Paned, Picture, Popover, ResponseType, ScrolledWindow, SearchEntry,
    SpinButton, Switch,
};
use gtk::gio;
use adw::glib;
use gdk_pixbuf;
use webkit6::prelude::*;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::rc::Rc;

const APP_ID: &str = "com.gutenair.gtk4";

// Returns the canonical content folder names defined by the core (strips "OEBPS/" prefix).
fn core_content_folders() -> Vec<&'static str> {
    gutencore::GutenCore::get_base_folders()
        .into_iter()
        .filter_map(|f| f.strip_prefix("OEBPS/"))
        .collect()
}

// (id, row, check_icon)
type GroupRows = Vec<(String, ActionRow, Image)>;

struct UiState {
    main_stack: ViewStack,
    editor: sourceview5::View,
    preview: webkit6::WebView,
    preview_inner: gtk::Stack,
    image_viewer: Picture,
    sidebar_box: Box,
    sidebar_scrolled: ScrolledWindow,
    paned: Paned,
    settings: gio::Settings,
    window: adw::ApplicationWindow,
    header_title: WindowTitle,
    stats_btn: Button,
    current_path: RefCell<Option<String>>,
    open_item_id: RefCell<Option<String>>,
    open_item_media_type: RefCell<Option<String>>,
    manifest_groups: RefCell<Vec<String>>,
    selected_items: RefCell<Vec<(String, String)>>,  // (folder, id)
    last_clicked: RefCell<Option<(String, String)>>,  // anchor for shift-click
    search_ctx: sourceview5::SearchContext,
}

// ─── App entry ───────────────────────────────────────────────────────────────

fn main() -> adw::glib::ExitCode {
    let application = Application::builder()
        .application_id(APP_ID)
        .build();

    application.connect_startup(setup_actions);
    application.connect_activate(build_ui);
    application.run()
}

fn setup_actions(app: &Application) {
    let action_about = gio::SimpleAction::new("about", None);
    action_about.connect_activate(move |_, _| show_about());
    app.add_action(&action_about);

    let action_pref = gio::SimpleAction::new("preferences", None);
    app.add_action(&action_pref);

    let action_help = gio::SimpleAction::new("help", None);
    action_help.connect_activate(|_, _| {
        let _ = gtk::show_uri(gtk::Window::NONE, "https://github.com", gtk::gdk::CURRENT_TIME);
    });
    app.add_action(&action_help);
}

fn show_about() {
    let about = AboutWindow::builder()
        .application_name("GutenAIR")
        .application_icon("com.gutenair.gtk4")
        .developer_name("GutenAIR Team")
        .version("0.1.0")
        .website("https://github.com/pedro/GutenAIR")
        .copyright("© 2026 GutenAIR Team")
        .issue_url("https://github.com/pedro/GutenAIR/issues")
        .license_type(gtk::License::MitX11)
        .build();

    about.add_credit_section(Some("Desarrollo"), &["Pedro"]);
    about.add_credit_section(Some("Asistente de Diseño"), &["Antigravity AI"]);

    if let Some(win) = Application::default().active_window() {
        about.set_transient_for(Some(&win));
    }
    about.present();
}

// ─── History ─────────────────────────────────────────────────────────────────

fn add_to_history(settings: &gio::Settings, path: &str) {
    let mut history: Vec<String> = settings.strv("history").into_iter().map(|s| s.to_string()).collect();
    history.retain(|p| p != path);
    history.insert(0, path.to_string());
    history.truncate(100);
    let refs: Vec<&str> = history.iter().map(|s| s.as_str()).collect();
    let _ = settings.set_strv("history", refs);
}

// ─── EPUB extraction ─────────────────────────────────────────────────────────

fn extract_epub(epub_path: &Path) -> Result<std::path::PathBuf, String> {
    let cache = glib::user_cache_dir().join("gutenair");
    let stem = epub_path.file_stem().and_then(|s| s.to_str()).unwrap_or("epub");
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

fn load_path(path_str: &str, state: &Rc<UiState>) {
    let path = Path::new(path_str);
    if path.is_dir() {
        load_book(path_str, state);
    } else if path.extension().map(|e| e.eq_ignore_ascii_case("epub")).unwrap_or(false) {
        match extract_epub(path) {
            Ok(dir) => load_book(&dir.to_string_lossy(), state),
            Err(e) => eprintln!("Error extrayendo epub: {}", e),
        }
    }
}

// ─── Sidebar helpers ─────────────────────────────────────────────────────────

fn folder_display_name(folder: &str) -> &str {
    match folder {
        "Text"   => "Texto",
        "Styles" => "Estilos",
        "Images" => "Imágenes",
        "Fonts"  => "Fuentes",
        "Audio"  => "Audio",
        "Video"  => "Video",
        "Misc"   => "Miscelánea",
        other    => other,
    }
}

fn icon_for_media_type(media_type: &str) -> &'static str {
    if media_type.contains("xhtml") || media_type.contains("html") {
        "text-x-generic-symbolic"
    } else if media_type.contains("css") {
        "text-x-script-symbolic"
    } else if media_type.starts_with("image/") {
        "image-x-generic-symbolic"
    } else if media_type.starts_with("font/") || media_type.contains("opentype") || media_type.contains("truetype") {
        "font-x-generic-symbolic"
    } else if media_type.starts_with("audio/") {
        "audio-x-generic-symbolic"
    } else if media_type.starts_with("video/") {
        "video-x-generic-symbolic"
    } else {
        "text-x-generic-symbolic"
    }
}

fn update_group_visuals(group_rows: &Rc<RefCell<GroupRows>>, state: &Rc<UiState>) {
    let sel = state.selected_items.borrow();
    let selected_ids: std::collections::HashSet<&str> =
        sel.iter().map(|(_, id)| id.as_str()).collect();
    for (id, _row, check_icon) in group_rows.borrow().iter() {
        check_icon.set_visible(selected_ids.contains(id.as_str()));
    }
}

fn save_current_item(state: &Rc<UiState>) {
    let item_id_opt = state.open_item_id.borrow().clone();
    let path_opt = state.current_path.borrow().clone();
    let media_type_opt = state.open_item_media_type.borrow().clone();

    // No guardar archivos binarios (imágenes, fuentes, audio)
    if let Some(ref mt) = media_type_opt {
        if mt.starts_with("image/") || mt.starts_with("font/") || mt.starts_with("audio/") || mt.starts_with("video/") {
            return;
        }
    }

    if let (Some(item_id), Some(path_str)) = (item_id_opt, path_opt) {
        if let Ok(core) = gutencore::GutenCore::open_folder(&path_str) {
            if let Ok(full_path) = core.get_resource_path(&item_id) {
                let buffer = state.editor.buffer();
                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();

                println!("[Guardado] Guardando cambios en {}...", item_id);
                if let Err(e) = std::fs::write(&full_path, text) {
                    eprintln!("[Error] No se pudo guardar {}: {}", item_id, e);
                }
            }
        }
    }
}

fn open_item(state: &Rc<UiState>, item_id: &str, media_type: &str) {
    // Save previous item if exists
    save_current_item(state);

    if let Some(path_str) = state.current_path.borrow().clone() {
        if let Ok(core) = gutencore::GutenCore::open_folder(&path_str) {
                if let Ok(full_path) = core.get_resource_path(item_id) {
                    let filename = full_path.file_name()
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
                        let lang_id = if media_type.contains("html") || media_type.contains("xhtml") {
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

                    if media_type.contains("html") || media_type.contains("xhtml") {
                        let uri = glib::filename_to_uri(&full_path, None)
                            .unwrap_or_else(|_| format!("file://{}", full_path.to_string_lossy()).into());
                        state.preview_inner.set_visible_child_name("web");
                        state.preview.load_uri(&uri);
                    }
                    if media_type.contains("html") || media_type.contains("xhtml") || media_type.contains("css") {
                        state.main_stack.set_visible_child_name("editor");
                    }
                }
            }
        }
    }
}

fn format_number(n: usize) -> String {
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

fn show_chapter_report(state: &Rc<UiState>) {
    let (path, item_id) = match (
        state.current_path.borrow().clone(),
        state.open_item_id.borrow().clone(),
    ) {
        (Some(p), Some(id)) => (p, id),
        _ => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => { eprintln!("chapter report: {}", e); return; }
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
        .margin_start(12).margin_end(12).margin_top(12).margin_bottom(6)
        .build();
    content_group.add(&make_row("Palabras",               format_number(stats.word_count)));
    content_group.add(&make_row("Caracteres sin espacios", format_number(stats.characters_no_spaces)));
    content_group.add(&make_row("Caracteres con espacios", format_number(stats.characters_with_spaces)));
    content_group.add(&make_row("Párrafos",               format_number(stats.paragraph_count)));
    content_group.add(&make_row("Tiempo de lectura",      reading_time));
    vbox.append(&content_group);

    let file_group = PreferencesGroup::builder()
        .title("Archivo")
        .margin_start(12).margin_end(12).margin_top(6).margin_bottom(24)
        .build();
    file_group.add(&make_row("Líneas",             format_number(stats.line_count)));
    file_group.add(&make_row("Caracteres totales", format_number(stats.total_file_size_chars)));
    vbox.append(&file_group);

    dialog.set_content(Some(&vbox));
    dialog.present();
}

fn show_book_report(state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => { eprintln!("book report: {}", e); return; }
    };

    let stats = match core.get_book_stats() {
        Ok(s) => s,
        Err(e) => { eprintln!("book report: {}", e); return; }
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
        .margin_start(12).margin_end(12).margin_top(12).margin_bottom(24)
        .build();
    group.add(&make_row("Capítulos",          format_number(stats.chapter_count)));
    group.add(&make_row("Palabras totales",   format_number(stats.total_word_count)));
    group.add(&make_row("Caracteres totales", format_number(stats.total_characters)));
    group.add(&make_row("Párrafos totales",   format_number(stats.total_paragraph_count)));
    group.add(&make_row("Tiempo de lectura",  reading_time));
    vbox.append(&group);

    dialog.set_content(Some(&vbox));
    dialog.present();
}

fn show_export_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
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
        .margin_start(24).margin_end(24).margin_top(24).margin_bottom(24)
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

fn show_export_epub_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    // Nombre sugerido: título del libro o nombre de carpeta
    let suggested = {
        let core = gutencore::GutenCore::open_folder(&path).ok();
        core.and_then(|c| {
            c.metadata.as_ref()
                .map(|m| m.title.clone())
        })
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
        if res != ResponseType::Accept { return; }
        let out_path = match n.file().and_then(|f| f.path()) {
            Some(p) => p,
            None => return,
        };
        let out_path = if out_path.extension().map(|e| e != "epub").unwrap_or(true) {
            out_path.with_extension("epub")
        } else {
            out_path
        };

        let mut core = match gutencore::GutenCore::open_folder(&state_c.current_path.borrow().clone().unwrap_or_default()) {
            Ok(c) => c,
            Err(e) => { eprintln!("export epub: {}", e); return; }
        };

        match core.export_epub(&out_path) {
            Ok(_) => eprintln!("export epub: guardado en {}", out_path.display()),
            Err(e) => eprintln!("export epub ERROR: {}", e),
        }
    });

    native.show();
}

fn show_export_text_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => { eprintln!("export: {}", e); return; }
    };

    // Spine chapters in order
    let chapters: Vec<(String, String)> = core.spine.iter()
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
        .margin_start(12).margin_end(12).margin_top(12).margin_bottom(6)
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
        chapters.iter().map(|(_, label)| {
            gtk::CheckButton::builder().label(label.as_str()).active(true).build()
        }).collect()
    );

    for (i, (_, label)) in chapters.iter().enumerate() {
        let row = ActionRow::builder().title(label.as_str()).activatable_widget(&checks[i]).build();
        row.add_prefix(&checks[i]);
        chap_group.add(&row);
    }

    // Select all / none handlers
    {
        let checks = checks.clone();
        sel_all_btn.connect_clicked(move |_| {
            for c in checks.iter() { c.set_active(true); }
        });
    }
    {
        let checks = checks.clone();
        sel_none_btn.connect_clicked(move |_| {
            for c in checks.iter() { c.set_active(false); }
        });
    }

    inner.append(&chap_group);

    // Output directory group
    let dest_group = PreferencesGroup::builder()
        .title("Carpeta de destino")
        .margin_start(12).margin_end(12).margin_top(6).margin_bottom(24)
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
            let selected_ids: Vec<String> = chapters.iter()
                .enumerate()
                .filter(|(i, _)| checks[*i].is_active())
                .map(|(_, (id, _))| id.clone())
                .collect();

            if selected_ids.is_empty() { return; }

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

fn nav_state_path(project_path: &str) -> std::path::PathBuf {
    std::path::Path::new(project_path).join(".gutenair_nav.json")
}

fn load_nav_state(project_path: &str) -> Option<Vec<gutencore::DocToc>> {
    let content = std::fs::read_to_string(nav_state_path(project_path)).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_nav_state(project_path: &str, data: &[gutencore::DocToc]) {
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = std::fs::write(nav_state_path(project_path), json);
    }
}

// Fusiona el escaneo fresco con el estado guardado: preserva los flags include
// para los headings que ya existían, agrega nuevos como include=true.
fn merge_toc_data(
    fresh: Vec<gutencore::DocToc>,
    saved: Option<&[gutencore::DocToc]>,
) -> Vec<gutencore::DocToc> {
    let saved = match saved {
        Some(s) => s,
        None => return fresh,
    };
    fresh.into_iter().map(|mut doc| {
        if let Some(saved_doc) = saved.iter().find(|d| d.href == doc.href) {
            doc.include = saved_doc.include;
            for item in &mut doc.items {
                if let Some(saved_item) = saved_doc.items.iter().find(|i| {
                    i.level == item.level && i.anchor == item.anchor
                }) {
                    item.include = saved_item.include;
                }
            }
        }
        doc
    }).collect()
}

fn show_nav_builder_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
    let project_path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&project_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("nav: {}", e); return; }
    };

    let fresh = match core.get_full_toc_data() {
        Ok(d) => d,
        Err(e) => { eprintln!("nav: {}", e); return; }
    };

    let saved = load_nav_state(&project_path);
    let merged = merge_toc_data(fresh, saved.as_deref());
    let toc_data: Rc<RefCell<Vec<gutencore::DocToc>>> = Rc::new(RefCell::new(merged));

    let dialog = adw::Window::builder()
        .title("Generate Table Of Contents")
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
    
    let lbl_hdr_title = Label::builder().label("TOC Entry / Heading Title").xalign(0.0).hexpand(true).build();
    let lbl_hdr_level = Label::builder().label("Level").width_request(80).xalign(0.0).build();
    let lbl_hdr_incl = Label::builder().label("Include").width_request(60).build();
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
    let btn_rename = Button::builder().label("Rename").build();
    
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
        Item(usize, usize)
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
            
            let title_text = if doc.title.is_empty() { &doc.href } else { &doc.title };
            let lbl_title = Label::builder().label(title_text).xalign(0.0).hexpand(true).ellipsize(gtk::pango::EllipsizeMode::End).build();
            
            let lbl_lvl = Label::builder().label("doc").width_request(80).xalign(0.0).build();
            
            let chk_incl = gtk::CheckButton::builder().active(doc.include).halign(gtk::Align::Center).width_request(60).build();
            
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

                let title_text = if item.title.is_empty() { "Untitled" } else { &item.title };
                let lbl_title = Label::builder().label(title_text).xalign(0.0).hexpand(true).ellipsize(gtk::pango::EllipsizeMode::End).build();
                
                let lbl_lvl = Label::builder().label(&format!("h{}", item.level)).width_request(80).xalign(0.0).build();
                lbl_lvl.add_css_class("dim-label");
                
                let chk_incl = gtk::CheckButton::builder().active(item.include).halign(gtk::Align::Center).width_request(60).build();

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
    let chk_show_only = gtk::CheckButton::builder().label("Show TOC items only").build();
    
    let model = gtk::StringList::new(&[
        "<Select headings to include in TOC>",
        "Include all headings",
        "Include up to level 1",
        "Include up to level 2",
        "Include up to level 3",
        "Include up to level 4",
        "Clear all",
    ]);
    let combo = gtk::DropDown::new(Some(model), gtk::Expression::NONE);
    
    vbox_bottom_left.append(&chk_show_only);
    vbox_bottom_left.append(&combo);
    hbox_bottom.append(&vbox_bottom_left);
    
    let btn_ok = Button::builder().label("OK").css_classes(["suggested-action"]).width_request(80).build();
    let btn_cancel = Button::builder().label("Cancel").width_request(80).build();
    
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
    combo.connect_selected_notify(move |cb| {
        let sel = cb.selected();
        if sel == 0 { return; } 
        
        let mut data = toc.borrow_mut();
        for doc in data.iter_mut() {
            doc.include = sel != 6;
            for item in doc.items.iter_mut() {
                item.include = match sel {
                    1 => true,
                    2 => item.level <= 1,
                    3 => item.level <= 2,
                    4 => item.level <= 3,
                    5 => item.level <= 4,
                    6 => false,
                    _ => item.include,
                };
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
            Err(e) => { eprintln!("nav gen: {}", e); return; }
        };
        match core.build_nav_from_data(&data) {
            Ok(_) => {
                match core.save() {
                    Ok(_) => eprintln!("nav: generado y guardado"),
                    Err(e) => eprintln!("nav save: {}", e),
                }
            }
            Err(e) => eprintln!("nav gen ERROR: {}", e),
        }
        dialog_ref.destroy();
    });

    dialog.set_content(Some(&vbox_main));
    dialog.present();
}

fn show_epub_check(state: &Rc<UiState>) {
    let path = match state.current_path.borrow().clone() {
        Some(p) => p,
        None => return,
    };

    let core = match gutencore::GutenCore::open_folder(&path) {
        Ok(c) => c,
        Err(e) => { eprintln!("epub-check: {}", e); return; }
    };

    let (errors, orphans) = core.validate_integrity_deep();

    let dialog = adw::Window::builder()
        .title("Verificación del EPUB")
        .modal(true)
        .transient_for(&state.window)
        .default_width(480)
        .default_height(400)
        .build();

    let hbar = HeaderBar::new();
    let vbox = Box::new(Orientation::Vertical, 0);
    vbox.append(&hbar);

    let scrolled = ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    let inner = Box::new(Orientation::Vertical, 0);

    if errors.is_empty() && orphans.is_empty() {
        let ok_group = PreferencesGroup::builder()
            .title("Sin problemas")
            .description("El EPUB no tiene errores de integridad ni archivos huérfanos.")
            .margin_start(12).margin_end(12).margin_top(12).margin_bottom(12)
            .build();
        inner.append(&ok_group);
    } else {
        if !errors.is_empty() {
            let err_group = PreferencesGroup::builder()
                .title(&format!("Errores del manifiesto ({})", errors.len()))
                .description("Archivos referenciados en el OPF que no existen en disco.")
                .margin_start(12).margin_end(12).margin_top(12).margin_bottom(6)
                .build();
            for msg in &errors {
                let row = ActionRow::builder()
                    .title(msg.as_str())
                    .build();
                let icon = Image::from_icon_name("dialog-error-symbolic");
                icon.add_css_class("error");
                row.add_prefix(&icon);
                err_group.add(&row);
            }
            inner.append(&err_group);
        }

        if !orphans.is_empty() {
            let orp_group = PreferencesGroup::builder()
                .title(&format!("Archivos huérfanos ({})", orphans.len()))
                .description("Archivos en disco que no están registrados en el manifiesto.")
                .margin_start(12).margin_end(12).margin_top(6).margin_bottom(24)
                .build();
            for path in &orphans {
                let name = path.to_string_lossy();
                let row = ActionRow::builder()
                    .title(name.as_ref())
                    .build();
                let icon = Image::from_icon_name("dialog-warning-symbolic");
                icon.add_css_class("warning");
                row.add_prefix(&icon);
                orp_group.add(&row);
            }
            inner.append(&orp_group);
        }
    }

    scrolled.set_child(Some(&inner));
    vbox.append(&scrolled);
    dialog.set_content(Some(&vbox));
    dialog.present();
}

fn show_context_popover(parent: gtk::Widget, x: f64, y: f64, state: &Rc<UiState>) {
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
        let media_type = state.current_path.borrow().as_ref()
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
                .margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
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
                    Err(e) => { eprintln!("set_cover: {}", e); return; }
                };
                let img_path = match core.get_resource_path(&item_id_c) {
                    Ok(p) => p,
                    Err(e) => { eprintln!("set_cover get_path: {}", e); return; }
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
                .margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
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
                        Err(e) => { eprintln!("paste especial: {}", e); return; }
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

    // Separador y botón eliminar (siempre visible)
    {
        let sep_del = gtk::Separator::new(Orientation::Horizontal);
        vbox.append(&sep_del);

        let del_btn = Button::builder()
            .label(if sel_count == 1 { "Eliminar archivo" } else { "Eliminar archivos" })
            .has_frame(false)
            .margin_start(4).margin_end(4).margin_top(4).margin_bottom(4)
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

fn show_delete_confirm_dialog(state: &Rc<UiState>, items: Vec<(String, String)>) {
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
            format!("Se eliminarán: {}.\nEsta acción no se puede deshacer.", names.join(", "))
        })
        .build();

    dialog.add_response("cancel", "Cancelar");
    dialog.add_response("delete", "Eliminar");
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    let state_c = state.clone();
    dialog.connect_response(None, move |_, response| {
        if response != "delete" { return; }

        let path = match state_c.current_path.borrow().clone() {
            Some(p) => p,
            None => return,
        };
        let mut core = match gutencore::GutenCore::open_folder(&path) {
            Ok(c) => c,
            Err(e) => { eprintln!("delete: {}", e); return; }
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
            state_c.image_viewer.set_paintable(gtk::gdk::Paintable::NONE);
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

fn show_rename_dialog(state: &Rc<UiState>) {
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
        Err(e) => { eprintln!("rename: {}", e); return; }
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
    let items: Vec<(String, String, String, String)> = sel_sorted.iter().filter_map(|(folder, id)| {
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
    }).collect();

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
    let old_hdr = Label::builder().label("Nombre actual").hexpand(true).xalign(0.0).build();
    let new_hdr = Label::builder().label("Nombre nuevo").hexpand(true).xalign(0.0).build();
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

fn populate_sidebar(state: &Rc<UiState>, core: &gutencore::GutenCore) {
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
    let ordered: Vec<String> = configured.iter()
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
        let key = groups.keys().find(|g| g.eq_ignore_ascii_case(folder)).cloned();
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
            let is_selected = state.selected_items.borrow()
                .iter().any(|(_, id)| id == &item.id);
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
                let drop_target = gtk::DropTarget::new(glib::Type::STRING, gtk::gdk::DragAction::MOVE);
                let state_drop = state.clone();
                let target_id = item.id.clone();
                drop_target.connect_drop(move |_, value, _, _| {
                    let dropped_id = match value.get::<String>() {
                        Ok(id) => id,
                        Err(e) => { eprintln!("DnD: error leyendo valor: {}", e); return false; }
                    };
                    if dropped_id == target_id { return false; }
                    let path = match state_drop.current_path.borrow().clone() {
                        Some(p) => p,
                        None => return false,
                    };
                    let mut core = match gutencore::GutenCore::open_folder(&path) {
                        Ok(c) => c,
                        Err(e) => { eprintln!("DnD: error abriendo libro: {}", e); return false; }
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
                                    if let Some(pos) = sel.iter().position(|(_, id)| id == &item_id) {
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
                                    Some((ref anchor_folder, ref anchor_id)) if anchor_folder == &folder_g => {
                                        let gi = group_rows_g.borrow();
                                        let pa = gi.iter().position(|(id, _, _)| id == anchor_id);
                                        let pc = gi.iter().position(|(id, _, _)| id == &item_id);
                                        if let (Some(pa), Some(pc)) = (pa, pc) {
                                            let (lo, hi) = if pa <= pc { (pa, pc) } else { (pc, pa) };
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
                                        state_g.selected_items.borrow_mut()
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
            group_rows.borrow_mut().push((item.id.clone(), action_row, check_icon));
        }

        list.append(&expander);
    }

    sidebar_box.append(&list);
}

// ─── Book loading ─────────────────────────────────────────────────────────────

fn load_book(path_str: &str, state: &Rc<UiState>) {
    match gutencore::GutenCore::open_folder(path_str) {
        Ok(core) => {
            add_to_history(&state.settings, path_str);

            if let Some(meta) = &core.metadata {
                state.header_title.set_title(&meta.title);
            }
            state.header_title.set_subtitle("");

            let manifest_groups: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                core.manifest.values()
                    .filter_map(|item| {
                        Path::new(&item.href).parent()
                            .and_then(|p| p.to_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string())
                    })
                    .filter(|f| seen.insert(f.clone()))
                    .collect()
            };

            let mut configured: Vec<String> = state.settings
                .strv("sidebar-groups")
                .into_iter()
                .map(|s| s.to_string())
                .collect();
            let hidden: Vec<String> = state.settings
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
                state.paned.set_position(if saved > 10 { saved } else { 260 });
            }
        }
        Err(e) => eprintln!("Error abriendo libro: {}", e),
    }
}

// ─── Preferences ─────────────────────────────────────────────────────────────

type GroupEntries = Vec<(String, String, bool)>;

fn save_groups(entries: &GroupEntries, settings: &gio::Settings) {
    let all: Vec<&str> = entries.iter()
        .map(|(k, _, _)| k.as_str())
        .collect();
    let hidden: Vec<&str> = entries.iter()
        .filter(|(_, _, v)| !*v)
        .map(|(k, _, _)| k.as_str())
        .collect();
    let _ = settings.set_strv("sidebar-groups", all);
    let _ = settings.set_strv("hidden-sidebar-groups", hidden);
}

fn refresh_sidebar(ui_state: &Rc<UiState>) {
    if let Some(path) = ui_state.current_path.borrow().clone() {
        if let Ok(core) = gutencore::GutenCore::open_folder(&path) {
            populate_sidebar(ui_state, &core);
        }
    }
}

fn rebuild_groups_ui(
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

fn show_preferences(parent: &impl IsA<gtk::Widget>, settings: &gio::Settings, ui_state: &Rc<UiState>) {
    let stored: Vec<String> = settings.strv("sidebar-groups")
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let hidden: Vec<String> = settings.strv("hidden-sidebar-groups")
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let manifest_groups = ui_state.manifest_groups.borrow().clone();

    // Core folders always appear in settings (in canonical order), even if currently empty.
    // Extra folders from imported EPUBs appear after, alphabetically.
    let core_folders = core_content_folders();

    let make_entry = |key: &str| -> (String, String, bool) {
        (key.to_string(), folder_display_name(key).to_string(), !hidden.contains(&key.to_string()))
    };

    let mut entries: GroupEntries = core_folders.iter()
        .map(|&f| {
            // If the manifest has this folder with different casing, use that name
            let actual = manifest_groups.iter()
                .find(|m| m.eq_ignore_ascii_case(f))
                .map(|s| s.as_str())
                .unwrap_or(f);
            make_entry(actual)
        })
        .collect();

    // Extra folders from imported EPUBs not covered by core canonical list
    let mut extra: Vec<(String, String, bool)> = manifest_groups.iter()
        .filter(|m| !core_folders.iter().any(|&f| f.eq_ignore_ascii_case(m)))
        .map(|m| make_entry(m))
        .collect();
    extra.sort_by(|a, b| a.0.cmp(&b.0));
    entries.extend(extra);

    let pref_state: Rc<RefCell<GroupEntries>> = Rc::new(RefCell::new(entries));

    let dialog = PreferencesDialog::builder()
        .title("Preferencias")
        .build();

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

    let editor_group = PreferencesGroup::builder()
        .title("Comportamiento")
        .build();

    let wrap_row = ActionRow::builder()
        .title("Ajuste de línea automático")
        .subtitle("Ajustar líneas largas al ancho visible de la ventana")
        .build();

    let wrap_switch = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .build();

    settings.bind("editor-wrap-text", &wrap_switch, "active")
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
    let status_label = Label::builder()
        .label("")
        .halign(gtk::Align::Start)
        .build();
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

            glib::idle_add_local(move || {
                match rx.try_recv() {
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
                }
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

fn show_import_chapters_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
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
            if count == 0 { n.destroy(); return; }

            match gutencore::GutenCore::open_folder(&path) {
                Err(e) => eprintln!("import chapters: {}", e),
                Ok(mut core) => {
                    let mut imported = 0;
                    for i in 0..count {
                        let file = match files.item(i).and_then(|o| o.downcast::<gio::File>().ok()) {
                            Some(f) => f,
                            None => continue,
                        };
                        let file_path = match file.path() {
                            Some(p) => p,
                            None => continue,
                        };
                        let ext = file_path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let stem = file_path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("capitulo")
                            .to_string();
                        let content = match std::fs::read_to_string(&file_path) {
                            Ok(c) => c,
                            Err(e) => { eprintln!("import: leyendo {}: {}", stem, e); continue; }
                        };

                        let xhtml = match ext.as_str() {
                            "xhtml" | "html" => content,
                            "txt" => core.text_to_xhtml(&content, &stem),
                            "md"  => {
                                eprintln!("import: .md aún no implementado, saltando {}", stem);
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
                                let href_taken = core.manifest.values().any(|it| it.href == href_candidate);
                                if !id_taken && !href_taken { break; }
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
                            Err(e) => eprintln!("import: add_document '{}': {}", id, e),
                        }
                    }

                    if imported > 0 {
                        match core.save() {
                            Ok(_) => refresh_sidebar(&state),
                            Err(e) => eprintln!("import: save() falló: {}", e),
                        }
                    }
                }
            }
        }
        n.destroy();
    });

    native.show();
}

fn show_new_project_dialog(parent: &impl IsA<gtk::Window>, state: &Rc<UiState>) {
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
    let folder_btn = Button::builder()
        .label("…")
        .build();
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
            let lang = if lang.is_empty() { "es".to_string() } else { lang };

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

fn show_add_resource_dialog(
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

fn mime_for_extension(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "png"         => "image/png",
        "jpg" | "jpeg"=> "image/jpeg",
        "gif"         => "image/gif",
        "webp"        => "image/webp",
        "svg"         => "image/svg+xml",
        "otf"         => "font/otf",
        "ttf"         => "font/ttf",
        "woff"        => "font/woff",
        "woff2"       => "font/woff2",
        "mp3"         => "audio/mpeg",
        "ogg"         => "audio/ogg",
        "wav"         => "audio/wav",
        "mp4"         => "video/mp4",
        "webm"        => "video/webm",
        "css"         => "text/css",
        "js"          => "application/javascript",
        _             => "application/octet-stream",
    }
}

fn show_import_dialog(
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
            if count == 0 { n.destroy(); return; }

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

fn run_ollama_generation(
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
        format!("Contexto:\n---\n{}\n---\n\nInstrucción: {}", input_text, prompt)
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
    let response = client.post(&url)
        .json(&body)
        .send()
        .map_err(|e| {
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

    println!("[IA] Generación completada con éxito ({} caracteres)", output.len());
    Ok(output)
}

fn show_ai_dialog(parent: &ApplicationWindow, state: &Rc<UiState>, selected_text: &str) {
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
    let prompt_group = PreferencesGroup::builder()
        .title("Instrucción")
        .build();
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
    
    let run_btn = Button::builder()
        .label("Generar")
        .build();
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
        let prompt = prompt_view_c.buffer().text(&prompt_view_c.buffer().start_iter(), &prompt_view_c.buffer().end_iter(), false).to_string();

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

        glib::idle_add_local(move || {
            match rx.try_recv() {
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
            }
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

fn setup_editor_context_menu(state: &Rc<UiState>) {
    let menu = gio::Menu::new();
    
    // IA section
    let ai_section = gio::Menu::new();
    ai_section.append(Some("Asistente IA..."), Some("editor.ai"));
    menu.append_section(None, &ai_section);
    
    // Submenu for Styles
    let styles_submenu = gio::Menu::new();
    menu.append_submenu(Some("Estilos"), &styles_submenu);

    // Initial population (common tags)
    let common_styles = vec![("Negrita", "b"), ("Cursiva", "i"), ("Título 1", "h1"), ("Título 2", "h2"), ("Párrafo", "p")];
    for (label, tag) in common_styles {
        let item = gio::MenuItem::new(Some(label), Some(&format!("editor.apply-tag('{}')", tag)));
        styles_submenu.append_item(&item);
    }

    // Try to add styles from core if possible (per chapter)
    let item_id_opt = state.open_item_id.borrow().clone();
    let path_opt = state.current_path.borrow().clone();

    if let (Some(item_id), Some(path_str)) = (item_id_opt, path_opt) {
        println!("[Menu] Buscando estilos para el capítulo: {} en {}", item_id, path_str);
        if let Ok(core) = gutencore::GutenCore::open_folder(&path_str) {
            // Log CSS IDs from config (default_styles / exceptions)
            let config_style_ids = core.get_chapter_styles(&item_id);
            println!("[Menu] IDs de CSS según config (default_styles/exceptions): {:?}", config_style_ids);

            // Log CSS hrefs linked in the XHTML <link> tags
            if let Ok(xhtml_path) = core.get_resource_path(&item_id) {
                if let Ok(xhtml_content) = std::fs::read_to_string(&xhtml_path) {
                    let link_re = regex::Regex::new(r#"<link[^>]+href="([^"]+\.css)"#).unwrap();
                    let linked: Vec<&str> = link_re.captures_iter(&xhtml_content)
                        .filter_map(|c| c.get(1).map(|m| m.as_str()))
                        .collect();
                    println!("[Menu] CSS referenciados en el XHTML <link>: {:?}", linked);
                }
            }

            // Use get_style_catalog to get the actual CSS class names
            match core.get_style_catalog(&item_id) {
                Ok(catalogs) => {
                    // (tag, class) — tag is the wrapping element
                    let mut entries: Vec<(String, String)> = Vec::new();
                    for catalog in &catalogs {
                        let bloque: Vec<&str> = catalog.estilos.bloque.iter().map(|e| e.clase.as_str()).collect();
                        let linea: Vec<&str> = catalog.estilos.linea.iter().map(|e| e.clase.as_str()).collect();
                        println!("[Menu] CSS '{}' — clases bloque: {:?}, clases línea: {:?}",
                            catalog.archivo_origen, bloque, linea);
                        for entry in &catalog.estilos.bloque {
                            let tag = entry.tag_sugerido.clone().unwrap_or_else(|| "p".to_string());
                            entries.push((tag, entry.clase.clone()));
                        }
                        for entry in &catalog.estilos.linea {
                            let tag = entry.tag_sugerido.clone().unwrap_or_else(|| "span".to_string());
                            entries.push((tag, entry.clase.clone()));
                        }
                    }

                    if !entries.is_empty() {
                        styles_submenu.append_section(None, &gio::Menu::new());
                        for (tag, class) in &entries {
                            // encode as "tag|class" so the action knows which element to use
                            let label = format!("{}.{}", tag, class);
                            let target = format!("{}|{}", tag, class);
                            let item = gio::MenuItem::new(Some(&label), Some(&format!("editor.apply-tag-class('{}')", target)));
                            styles_submenu.append_item(&item);
                        }
                        println!("[Menu] Entradas construidas en el menú: {:?}", entries);
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

    state.editor.set_extra_menu(Some(&menu));

    let action_group = gio::SimpleActionGroup::new();
    state.editor.insert_action_group("editor", Some(&action_group));

    // IA Action
    let ai_action = gio::SimpleAction::new("ai", None);
    let state_ai = state.clone();
    ai_action.connect_activate(move |_, _| {
        let buffer = state_ai.editor.buffer();
        let text = if let Some((start, end)) = buffer.selection_bounds() {
            buffer.text(&start, &end, false).to_string()
        } else {
            "".to_string()
        };
        show_ai_dialog(&state_ai.window, &state_ai, &text);
    });
    action_group.add_action(&ai_action);

    // Apply Tag Action (e.g. <b>...</b>)
    let tag_apply_action = gio::SimpleAction::new("apply-tag", Some(glib::VariantTy::STRING));
    let state_tag = state.clone();
    tag_apply_action.connect_activate(move |_, variant| {
        if let Some(tag_name) = variant.and_then(|v| v.get::<String>()) {
            let buffer = state_tag.editor.buffer();
            if let Some((start, end)) = buffer.selection_bounds() {
                let text = buffer.text(&start, &end, false).to_string();
                let tagged = format!("<{}>{}</{}>", tag_name, text, tag_name);
                buffer.delete_selection(true, true);
                buffer.insert_at_cursor(&tagged);
            }
        }
    });
    action_group.add_action(&tag_apply_action);

    // Apply Tag+Class Action — variant is "tag|class", e.g. "p|sub" → <p class="sub">…</p>
    let tag_class_action = gio::SimpleAction::new("apply-tag-class", Some(glib::VariantTy::STRING));
    let state_tc = state.clone();
    tag_class_action.connect_activate(move |_, variant| {
        if let Some(raw) = variant.and_then(|v| v.get::<String>()) {
            let (tag, class) = raw.split_once('|')
                .map(|(t, c)| (t.to_string(), c.to_string()))
                .unwrap_or_else(|| ("span".to_string(), raw.clone()));
            let buffer = state_tc.editor.buffer();
            if let Some((start, end)) = buffer.selection_bounds() {
                let text = buffer.text(&start, &end, false).to_string();
                let tagged = format!("<{} class=\"{}\">{}</{}>", tag, class, text, tag);
                buffer.delete_selection(true, true);
                buffer.insert_at_cursor(&tagged);
            }
        }
    });
    action_group.add_action(&tag_class_action);
}

// ─── Main UI ─────────────────────────────────────────────────────────────────

fn navigate_search(state: &Rc<UiState>, forward: bool) {
    let buffer = state.editor.buffer();
    let cursor = buffer.iter_at_mark(&buffer.get_insert());
    let result = if forward {
        state.search_ctx.forward(&cursor)
    } else {
        state.search_ctx.backward(&cursor)
    };
    if let Some((start, end, _wrapped)) = result {
        buffer.select_range(&start, &end);
        state.editor.scroll_to_iter(&mut start.clone(), 0.1, true, 0.5, 0.5);
    }
}

fn format_match_count(count: i32, has_query: bool) -> String {
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

fn build_ui(app: &Application) {
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
            let theme_name = if sm.is_dark() { "Adwaita-dark" } else { "Adwaita" };
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
    main_stack.add_titled_with_icon(&editor_scrolled, Some("editor"), "Editor", "text-editor-symbolic");

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

    main_stack.add_titled_with_icon(&preview_inner, Some("preview"), "Vista Previa", "web-browser-symbolic");

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
                let _ = state.search_ctx.replace(&mut start, &mut end, replace_text.as_str());
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
                let _ = state.search_ctx.replace(&mut start, &mut end, replace_text.as_str());
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
    ui_state.main_stack.connect_notify_local(Some("visible-child"), {
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
    app.set_accels_for_action("app.find-replace", &["<Control>h", "<Control>f"]);

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

    let add_chapter_action = gio::SimpleAction::new("add-chapter", None);
    add_chapter_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| {
            show_add_resource_dialog(&win, &state, "Capítulo", "Text", "application/xhtml+xml");
        }
    });
    app.add_action(&add_chapter_action);

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
    app.set_accels_for_action("app.book-report", &["<Control><Shift>i"]);

    // --- Export action ---
    let export_action = gio::SimpleAction::new("export", None);
    export_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_export_dialog(&win, &state)
    });
    app.add_action(&export_action);

    // --- Nav builder action ---
    let nav_builder_action = gio::SimpleAction::new("nav-builder", None);
    nav_builder_action.connect_activate({
        let win = window.clone();
        let state = ui_state.clone();
        move |_, _| show_nav_builder_dialog(&win, &state)
    });
    app.add_action(&nav_builder_action);

    // --- EPUB check action + shortcut ---
    let epub_check_action = gio::SimpleAction::new("epub-check", None);
    epub_check_action.connect_activate({
        let state = ui_state.clone();
        move |_, _| show_epub_check(&state)
    });
    app.add_action(&epub_check_action);
    app.set_accels_for_action("app.epub-check", &["<Control><Shift>v"]);

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
            if sidebar.is_visible() {
                let _ = s.set_int("sidebar-width", p.position());
                sidebar.set_visible(false);
            } else {
                sidebar.set_visible(true);
                let saved = s.int("sidebar-width");
                p.set_position(if saved > 10 { saved } else { 260 });
            }
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
                if count >= 15 { break; }
                let path_str = path_glib.to_string();
                let path = Path::new(&path_str);
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(&path_str);

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

    window.present();
}

fn fetch_ollama_models(base_url: &str) -> Result<Vec<String>, String> {
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
