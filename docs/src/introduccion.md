# Introduccion

GutenAIR es una aplicacion de escritorio para editar y gestionar proyectos de libros digitales en formato EPUB.

La interfaz esta construida con Rust, GTK4, Libadwaita, GtkSourceView y WebKitGTK. La logica de libro se apoya en `gutencore`, que vive como dependencia local fuera de este repositorio.

## Objetivo

El objetivo de GutenAIR es ofrecer un flujo de trabajo visual para abrir, editar, previsualizar, validar y exportar libros digitales sin depender de editar todos los archivos del EPUB manualmente.

## Estado actual

El proyecto esta en version `0.1.0`. Varias funciones existen en la aplicacion, pero la documentacion todavia debe tratarse como guia de trabajo inicial y no como manual cerrado de producto.

## Componentes principales

- Editor con GtkSourceView.
- Vista previa con WebKitGTK.
- Panel lateral para recursos del libro.
- Exportacion a EPUB y texto plano.
- Preferencias con GSettings.
- Integracion experimental con Ollama.
