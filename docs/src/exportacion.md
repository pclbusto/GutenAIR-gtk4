# Exportacion

GutenAIR tiene dos caminos de exportacion desde la interfaz:

- EPUB
- Texto plano

## Exportar EPUB

La exportacion EPUB usa `gutencore::GutenCore::export_epub`. La aplicacion sugiere el nombre del archivo a partir del titulo del libro, si existe, o del nombre de la carpeta del proyecto.

Si el nombre elegido no termina en `.epub`, la aplicacion agrega la extension automaticamente.

## Exportar texto plano

La exportacion a texto plano permite seleccionar capitulos segun el orden del `spine`.

El destino predeterminado es la carpeta del proyecto, pero se puede elegir otra carpeta antes de exportar.

## Errores

Actualmente varios errores de exportacion se imprimen por stderr. Si una exportacion no aparece en la interfaz, ejecuta la aplicacion desde una terminal para ver el mensaje.
