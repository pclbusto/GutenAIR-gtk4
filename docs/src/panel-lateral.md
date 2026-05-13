# Panel Lateral (Explorador de Recursos)

Al abrir un proyecto o archivo EPUB, el panel lateral izquierdo genera automáticamente una vista jerárquica organizada por categorías. Este panel es el corazón de la organización editorial del libro.

## 1. Organización de Secciones
El software clasifica los recursos del libro en las secciones que se detallan a continuación. Todas las secciones son **colapsables**, lo que permite una navegación fluida incluso en proyectos con gran volumen de archivos (especialmente en la sección de Texto).

| Sección | Contenido |
| :--- | :--- |
| **Texto** | Archivos XHTML/HTML (capítulos del libro). |
| **Imágenes** | Recursos gráficos (JPG, PNG, SVG). |
| **Fuentes** | Archivos de tipografía (TTF, OTF). |
| **Estilos** | Hojas de estilo CSS. |

## 2. Herramientas de Gestión
Cada sección cuenta con herramientas específicas que se adaptan al tipo de recurso. Todas las secciones mantienen el botón de **Lupa** para filtrado rápido.

### 2.1 Secciones de Texto y Estilos
Al pulsar el botón **(+)**, se presentan dos opciones:
*   **Crear nuevo**: Genera un archivo vacío (XHTML o CSS) dentro del proyecto.
*   **Importar existente**: Permite traer un archivo desde el disco o desde otro proyecto.

### 2.2 Secciones de Imágenes y Fuentes
En estas categorías, el botón **(+)** solo permite **Importar**, ya que son recursos que deben generarse externamente.

### 2.3 Configuración Especial de Estilos (Engranaje)
La sección de **Estilos** incluye un botón de **Engranaje** que permite configurar la automatización de hojas de estilo. Al pulsarlo, se despliega un popover con las siguientes funciones:

*   **Selección de Estilos**: Se muestra una lista de todos los archivos CSS registrados en el proyecto.
*   **Estilos Predeterminados**: El usuario puede marcar los estilos que desee que se incluyan por defecto en cada nuevo capítulo (XHTML) que se genere desde la aplicación.
*   **Inyección Automática**: Al pulsar **Aplicar**, la configuración se guarda. A partir de ese momento, cualquier archivo nuevo de texto incluirá automáticamente las etiquetas de enlace a los estilos seleccionados en su cabecera (`<head>`).

---

## 3. Acciones e Interacción

### 3.1 Menú Contextual
Al realizar click derecho sobre uno o varios recursos del mismo tipo, se despliega un menú contextual con opciones específicas:

*   **Renombrar**: Disponible para todos los recursos (Texto, Imágenes, Fuentes o Estilos). Permite el renombrado individual o masivo.
*   **Gestionar estilos**: Disponible únicamente para la sección de **Texto**. Permite asociar o desvincular hojas de estilo a los archivos seleccionados.
*   **Pegado especial**: Opción exclusiva para la sección de **Texto** que facilita la creación de contenido a partir de texto plano.
*   **Eliminar archivos**: Disponible para todos los recursos. Permite la eliminación definitiva de los archivos seleccionados.
*   **Establecer como portada**: Opción exclusiva para la sección de **Imágenes** que genera la portada del libro de forma automática.

#### Ventana de Renombrado Masivo
Al seleccionar **Renombrar** con múltiples archivos, se abre una ventana que automatiza la nomenclatura secuencial:

*   **Prefijo**: Parte de texto fija que precederá al número (ej: `capitulo_`).
*   **Inicio**: Valor numérico desde el cual comenzará la cuenta (ej: `1`).
*   **Dígitos**: Cantidad de cifras para el contador (útil para mantener el orden alfabético con ceros a la izquierda).
    *   *Ejemplo (Dígitos 2)*: Generará `01`, `02`, etc. en lugar de `1`, `2`.

La ventana incluye una tabla comparativa que muestra el **Nombre Actual** frente al **Nuevo Nombre** generado según los parámetros anteriores.

#### Ventana de Gestión de Estilos
Al seleccionar esta opción para uno o varios archivos de texto, se abre una ventana que facilita la vinculación de hojas de estilo:

*   **Lista de Estilos**: Muestra todos los archivos CSS disponibles en el proyecto con una casilla de verificación (**checkbox**) al lado de cada uno.
*   **Vinculación Masiva**: El usuario puede marcar o desmarcar uno o varios estilos. Al aceptar, el software ajusta automáticamente las cabeceras de los archivos XHTML seleccionados para incluir las referencias a los estilos elegidos.

#### Pegado Especial
Esta función está diseñada para agilizar la importación de texto desde el portapapeles (**clipboard**). Al utilizarla:

1.  El sistema toma el texto copiado en el sistema.
2.  Identifica cada salto de línea y lo envuelve automáticamente en etiquetas de párrafo (`<p>...</p>`).
3.  Genera una estructura XHTML válida, reemplazando íntegramente el contenido del cuerpo (`<body>`) del archivo seleccionado con el nuevo texto formateado.

> [!NOTE]
> Esta funcionalidad está sujeta a cambios. Se planea su integración directa en el editor para permitir inserciones parciales. Ver [Pegado Especial en el Editor](pendientes.md#1-funcionalidades-del-editor) en la sección de tareas pendientes.
> 
> **Progreso de documentación:**
> - [x] Update `panel-lateral.md` with renaming tool details
> - [x] Verify the content and formatting
> - [x] Create walkthrough
> - [x] Add "Gestionar estilos" to context menu documentation
> - [x] Document "Gestionar estilos" window
> - [x] Add "Pegado especial" documentation
> - [x] Create `pendientes.md` for future improvements
> - [x] Add hyperlink between documents
> - [x] Add "Eliminar archivos" documentation
> - [x] Detail "Default Styles" configuration
> - [x] Add "Establecer como portada" documentation

#### Eliminación de Recursos
La opción **Eliminar archivos** permite quitar recursos del proyecto de forma permanente. Al confirmar la acción en el diálogo de seguridad:

1.  Los archivos se eliminan físicamente del almacenamiento del proyecto.
2.  Se actualiza automáticamente el manifiesto (`manifest`) y el orden de lectura (`spine`) en el archivo de control `.opf` para mantener la integridad del EPUB.

> [!CAUTION]
> Esta acción es **irreversible**. Una vez confirmada la eliminación, los archivos no podrán recuperarse a través del software.

#### Gestión de Portada
Al seleccionar **Establecer como portada** sobre una imagen en la sección correspondiente:

1.  Se genera automáticamente un nuevo archivo llamado `cover.xhtml`.
2.  Este archivo se posiciona al inicio del **Spine** (Orden de Lectura), asegurando que sea lo primero que vea el lector al abrir el libro.
3.  La imagen seleccionada se inserta en el XHTML con los parámetros necesarios para una visualización correcta en lectores de EPUB.
4.  El usuario mantiene la libertad de renombrar el archivo `cover.xhtml` o desplazarlo manualmente sin perder la configuración de portada.

### 3.2 Orden de Lectura (Spine)
El orden en que aparecen los capítulos en la sección de **Texto** define la estructura del libro:
*   **Orden del Spine**: La disposición visual corresponde exactamente al orden en el que se incluirán en el **Spine** del manifiesto (`.opf`).
*   **Arrastrar y soltar**: Se utiliza esta función para reorganizar los capítulos. Al mover un archivo, se modifica directamente el orden secuencial de lectura del libro.

---

## 4. Restricciones de Soporte
Por decisión de diseño y alcance del software, **GutenAIR no soporta** los siguientes elementos multimedia:
*   **Video**: No se permite la inclusión ni previsualización de archivos de video.
*   **Sonido**: No hay soporte para archivos de audio.
