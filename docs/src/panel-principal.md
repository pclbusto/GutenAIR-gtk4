# Panel Principal (Editor y Vista Previa)

El panel principal es el área de trabajo central donde se visualiza y edita el contenido del EPUB. Se organiza mediante dos pestañas principales: **Editor** y **Vista Previa**.

## 1. Pestaña de Vista Previa

La Vista Previa permite visualizar el contenido tal como aparecerá en un lector de libros electrónicos.

*   **Motor de Renderizado**: Utiliza **WebKit** para procesar y mostrar el XHTML cocinado.
*   **Gestión de Imágenes**: Para evitar problemas con rutas relativas en el entorno de desarrollo, el software convierte automáticamente todas las imágenes del capítulo a formato **Base64** y las inyecta directamente en el HTML antes de renderizarlo.

## 2. Pestaña de Editor

El Editor es una herramienta de edición de código XHTML con funcionalidades optimizadas para la creación literaria.

### 2.1 Características Técnicas
*   **Resaltado de Sintaxis**: Soporte completo para etiquetas HTML/XHTML.
*   **Opciones de Visualización**: El usuario puede activar o desactivar la numeración de líneas y el ajuste de línea (parrafado).

### 2.2 Menú Contextual del Editor
Al realizar click derecho dentro del área de edición, se despliegan las siguientes opciones:

*   **Acciones Básicas**: Cortar, Copiar, Eliminar, Deshacer, Rehacer y Seleccionar todo.
*   **Inserción de Elementos**:
    *   **Insertar Emoticon**: Acceso rápido a la paleta de emojis.
*   **Herramientas Avanzadas**:
    *   **Asistente IA**: Integración con Ollama para generación o corrección de texto.
    *   **Dividir párrafo aquí**: Inserta un salto de párrafo en la posición del cursor.
    *   **Dividir capítulo aquí**: Divide el archivo XHTML actual en dos, creando un nuevo recurso en el panel lateral a partir del punto de corte.
    *   **Estilos**: Acceso rápido a la gestión de hojas de estilo para el documento actual.
