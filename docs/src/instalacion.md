# Instalacion

## Requisitos

Necesitas Rust y las bibliotecas nativas de GTK usadas por la aplicacion:

- GTK4
- Libadwaita
- WebKitGTK para GTK4 (`webkit6`)
- GtkSourceView 5
- GLib/GSettings

En Fedora:

```bash
sudo dnf install gtk4-devel libadwaita-devel webkit6gtk-devel gtksourceview5-devel
```

En Arch Linux:

```bash
sudo pacman -S gtk4 libadwaita webkitgtk-6.0 gtksourceview5
```

En Debian o Ubuntu, los nombres exactos pueden variar segun la version de la distribucion. Busca los paquetes de desarrollo equivalentes para GTK4, Libadwaita, WebKitGTK 6 y GtkSourceView 5.

## Dependencia local gutencore

Este repositorio depende de `gutencore` mediante una ruta local:

```toml
gutencore = { path = "../GutenAIR" }
```

Por eso, para compilar sin cambios, la estructura esperada es:

```text
Rust projects/
  GutenAIR/
  GutenAIR-gtk4/
```

Si `GutenAIR` esta en otra ubicacion, ajusta la ruta en `Cargo.toml`.

## Esquemas de GSettings

La aplicacion usa el esquema `com.gutenair.gtk4.gschema.xml`. Antes de ejecutar localmente, compila los esquemas:

```bash
glib-compile-schemas .
```

El archivo generado `gschemas.compiled` queda en la raiz del proyecto.

## Compilacion

```bash
cargo build
```

Para compilar en modo release:

```bash
cargo build --release
```
