# Troubleshooting

## No compila por `gutencore`

Verifica que exista el repositorio o crate local esperado:

```text
../GutenAIR
```

Si esta en otra ruta, cambia la dependencia en `Cargo.toml`.

## Error de GSettings

Si aparece un error sobre claves o esquemas no encontrados, recompila los esquemas:

```bash
glib-compile-schemas .
```

Ejecuta la aplicacion desde la raiz del proyecto para que encuentre `gschemas.compiled`.

## Faltan bibliotecas nativas

Si falla la compilacion de crates GTK/WebKit/SourceView, instala los paquetes de desarrollo de la distribucion. El error suele mencionar paquetes `pkg-config` faltantes.

## La exportacion no muestra errores

Ejecuta la aplicacion desde una terminal:

```bash
cargo run
```

Algunos errores de exportacion se imprimen por stderr.

## mdBook no esta instalado

Instalalo con:

```bash
cargo install mdbook
```

Luego ejecuta:

```bash
mdbook build docs
```
