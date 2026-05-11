pub(crate) fn tr(key: &str) -> &'static str {
    match key {
        "nav.title" => "Tabla de contenidos",
        "nav.header.title" => "Entrada / título",
        "nav.header.level" => "Nivel",
        "nav.header.include" => "Incluir",
        "nav.rename" => "Renombrar",
        "nav.show_only" => "Mostrar solo incluidos",
        "nav.select_headings" => "Seleccionar headings para incluir",
        "nav.mark_all" => "Marcar todos",
        "nav.clear_all" => "Desmarcar todos",
        "common.accept" => "Aceptar",
        "common.cancel" => "Cancelar",
        "common.untitled" => "Sin título",
        _ => "",
    }
}

// Returns the canonical content folder names defined by the core (strips "OEBPS/" prefix).
