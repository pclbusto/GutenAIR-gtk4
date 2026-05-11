use crate::prelude::*;

pub(crate) fn core_content_folders() -> Vec<&'static str> {
    gutencore::GutenCore::get_base_folders()
        .into_iter()
        .filter_map(|f| f.strip_prefix("OEBPS/"))
        .collect()
}

// (id, row, check_icon)

pub(crate) type GroupRows = Vec<(String, ActionRow, Image)>;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum TriState {
    All,
    None,
    Mixed,
}

pub(crate) struct UiState {
    pub(crate) main_stack: ViewStack,
    pub(crate) editor: sourceview5::View,
    pub(crate) preview: webkit6::WebView,
    pub(crate) preview_inner: gtk::Stack,
    pub(crate) image_viewer: Picture,
    pub(crate) sidebar_box: Box,
    pub(crate) sidebar_scrolled: ScrolledWindow,
    pub(crate) paned: Paned,
    pub(crate) settings: gio::Settings,
    pub(crate) window: adw::ApplicationWindow,
    pub(crate) header_title: WindowTitle,
    pub(crate) stats_btn: Button,
    pub(crate) current_path: RefCell<Option<String>>,
    pub(crate) open_item_id: RefCell<Option<String>>,
    pub(crate) open_item_media_type: RefCell<Option<String>>,
    pub(crate) manifest_groups: RefCell<Vec<String>>,
    pub(crate) selected_items: RefCell<Vec<(String, String)>>, // (folder, id)
    pub(crate) last_clicked: RefCell<Option<(String, String)>>, // anchor for shift-click
    pub(crate) search_ctx: sourceview5::SearchContext,
}

// ─── App entry ───────────────────────────────────────────────────────────────
