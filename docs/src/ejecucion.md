# Ejecucion

Despues de compilar los esquemas de GSettings, ejecuta:

```bash
cargo run
```

Si la aplicacion no encuentra las claves de configuracion, vuelve a ejecutar:

```bash
glib-compile-schemas .
```

## Documentacion local

La documentacion de Rust se genera con:

```bash
cargo doc --no-deps
```

La documentacion de usuario y desarrollo usa mdBook. Instala mdBook si no esta disponible:

```bash
cargo install mdbook
```

Luego construye este manual:

```bash
mdbook build docs
```

Para verlo con recarga local:

```bash
mdbook serve docs
```
