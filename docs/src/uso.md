# Manual de Interfaz de Usuario - Aplicación GutenAIR

## 1. Barra Superior (Cabecera Principal)

La barra superior se organiza en tres zonas: izquierda, centro y derecha.

### 1.1 Zona Izquierda

| Orden | Elemento | Función |
| :---: | :--- | :--- |
| 1 | **Botón de lista de recursos** | Oculta o muestra la lista de recursos del EPUB. |
| 2 | **Drop down "Abrir"** | Ver sección 1.1.1 para detalles completos. |
| 3 | **Botón "+"** | Permite añadir nuevos recursos al proyecto (imágenes, archivos XHTML, etc.). |

#### 1.1.1 Popover del botón "Abrir"

Al hacer clic en **"Abrir"**, se muestra un popover con:

| Elemento | Descripción |
| :--- | :--- |
| **Cuadro de búsqueda** | Filtra la lista de últimos documentos abiertos. |
| **Botón "Abrir carpeta"** | Abre una carpeta que debe contener la estructura completa de un EPUB. |
| **Botón "Abrir EPUB"** | Abre un archivo en formato EPUB (libro electrónico). |
| **Lista de recientes** | Últimos 10 documentos usados. |
| **Botón de eliminar (por cada ítem)** | Elimina ese documento de la lista de recientes (no borra el archivo físico). |

#### 1.1.2 Comportamiento detallado de "Abrir carpeta"

Al seleccionar una carpeta, la aplicación asume que contiene la estructura completa de un proyecto EPUB compatible:

```text
nombre_del_proyecto/
├── manifiesto (archivo obligatorio)
├── texto/
├── imagenes/
├── fuentes/
└── [otros componentes]
```

**Validación y errores:**
*   Si la carpeta **no** tiene una estructura válida (falta el manifiesto o las carpetas requeridas), la aplicación simplemente **no cargará ningún contenido**.
*   Actualmente, la interfaz **no muestra mensajes de error** en este caso; la ventana permanecerá en su estado actual.

### 1.2 Zona Centro

| Elemento | Función |
| :--- | :--- |
| **Botón Edición / Vista previa** | Alterna entre modo edición y modo vista previa del documento seleccionado. |

### 1.3 Zona Derecha

| Orden | Elemento | Función |
| :---: | :--- | :--- |
| 1 | **Botón propiedades** | Muestra las propiedades del documento actual (metadatos). |
| 2 | **Menú botón** | Acceso a opciones adicionales (Preferencias, Acerca de). |
| 3 | **Botón cerrar** | Cierra la aplicación. |

---

## 2. Panel Lateral (Explorador de Recursos)

El panel lateral izquierdo es la herramienta principal para gestionar la estructura del EPUB, organizar capítulos y administrar recursos multimedia.

Debido a su importancia y cantidad de funciones, este componente tiene su propio manual detallado:

👉 **[Ver Manual Detallado del Panel Lateral](panel-lateral.md)**

---

## 3. Zona de Trabajo

La parte central de la aplicación cambia según el modo seleccionado.

### 3.1 Modo Edición (GtkSourceView)
El modo edición permite modificar directamente el contenido de los archivos. Al activarse, aparece una **barra de herramientas secundaria** situada debajo de la cabecera principal.

#### 3.1.1 Barra de Herramientas Dinámica
Esta barra es adaptativa y cambia sus opciones según el tipo de recurso seleccionado:

*   **Para Archivos de Texto (XHTML/HTML)**:
    *   **Formato**: Negrita, cursiva, subrayado, tachado, subíndice y superíndice.
    *   **Enlaces**: Herramienta para insertar y gestionar links.
    *   **Transformación**: Conversión de texto a mayúsculas, minúsculas o capitalización automática.
*   **Para Archivos de Imagen**:
    *   **Transformación**: Rotación y escalado de la imagen.
    *   **Accesibilidad**: Campo para definir el **texto alternativo** (alt text).
*   **Otros Recursos**: Si el recurso seleccionado no cuenta con herramientas específicas, la barra secundaria se **oculta automáticamente**.

#### 3.1.2 Funcionalidades del Editor
*   **Resaltado de sintaxis**: Soporte completo para HTML, XHTML y CSS.
*   **Numeración de líneas**: Facilita la localización de errores y la navegación.
*   **Autoguardado**: Los cambios se conservan temporalmente para evitar pérdidas de información.

### 3.2 Modo Vista Previa (WebKitGTK)
*   **Renderizado fiel**: Muestra el contenido tal como se vería en un lector de e-books.
*   **Refresco automático**: Se actualiza al cambiar desde el modo edición.

---

## 4. Herramientas y Menús

### 4.1 Menú de Aplicación (Hamburgo)
*   **Preferencias**: Configuración de tema (claro/oscuro) y fuentes.
*   **Validación**: Comprueba si el EPUB cumple con los estándares.
*   **Exportación**: Permite generar el archivo final `.epub` o en texto plano.

---

## 5. Glosario

| Término | Significado |
| :--- | :--- |
| **EPUB** | Formato estándar de libro electrónico basado en XML/XHTML. |
| **XHTML** | Variante de HTML usada para el contenido de los capítulos. |
| **Libadwaita** | Biblioteca que proporciona los componentes visuales modernos de GNOME. |
| **Ollama** | (Experimental) Integración de IA para asistencia en la escritura. |
