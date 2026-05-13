# Proyectos EPUB

GutenAIR trabaja sobre proyectos EPUB abiertos como carpetas. La capa `gutencore` se encarga de leer la estructura del libro, incluyendo manifiesto, spine, metadatos y recursos.

## Estructura esperada

La estructura exacta la define `gutencore`, pero el codigo de la interfaz espera carpetas base bajo `OEBPS/`, por ejemplo:

```text
OEBPS/
  Text/
  Styles/
  Images/
  Fonts/
```

Los grupos visibles en el panel lateral salen de esas carpetas y de los recursos detectados en el manifiesto del libro.

## Recursos

La aplicacion distingue entre contenido editable y recursos de vista previa:

- HTML, XHTML, CSS y XML se abren en el editor.
- Imagenes se muestran en el visor de imagen.
- Otros recursos pueden aparecer agrupados segun el manifiesto.
