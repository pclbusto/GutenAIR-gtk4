# Arquitectura

El binario principal esta en `src/main.rs`. Crea una aplicacion Libadwaita con el identificador `com.gutenair.gtk4`, conecta las acciones de arranque y construye la interfaz.

## Modulos

- `app.rs`: acciones globales, ventana Acerca de y acciones de aplicacion.
- `ui.rs`: construccion de la ventana principal, editor, vista previa, menus y barras.
- `state.rs`: estado compartido de la interfaz.
- `sidebar.rs`: panel lateral y navegacion por recursos.
- `editor.rs`: logica relacionada con edicion de archivos.
- `export.rs`: dialogos y acciones de exportacion.
- `preferences.rs`: dialogo de preferencias y persistencia de opciones.
- `reports.rs`: estadisticas e informes.
- `nav.rs`: herramientas de tabla de contenidos.
- `validation.rs`: validacion de EPUB.
- `resources.rs`: carga y manejo de recursos.
- `book.rs`: operaciones relacionadas con libro/proyecto.
- `i18n.rs`: soporte de textos/localizacion.
- `constants.rs`: constantes globales.
- `prelude.rs`: imports compartidos.

## Estado de UI

`UiState` agrupa widgets y datos necesarios entre modulos:

- stack principal
- editor
- vista previa
- panel lateral
- configuracion GSettings
- ruta del proyecto abierto
- recurso actualmente abierto
- seleccion del panel lateral
- contexto de busqueda

Este enfoque simplifica la conexion de callbacks GTK, aunque conviene mantener cambios futuros bien acotados para que el estado compartido no crezca sin control.
