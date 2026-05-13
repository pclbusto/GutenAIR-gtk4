# Desarrollo

## Comandos utiles

Compilar:

```bash
cargo build
```

Ejecutar:

```bash
cargo run
```

Formatear:

```bash
cargo fmt
```

Revisar errores sin generar binario final:

```bash
cargo check
```

Generar rustdoc:

```bash
cargo doc --no-deps
```

## Documentacion con mdBook

Instalar mdBook:

```bash
cargo install mdbook
```

Construir el manual:

```bash
mdbook build docs
```

Servir localmente:

```bash
mdbook serve docs
```

## Estilo de cambios

- Mantener el README como entrada breve al proyecto.
- Documentar flujos largos en `docs/src/`.
- Usar rustdoc para funciones o modulos donde la intencion no sea obvia.
- Evitar que la documentacion prometa funciones que todavia no esten implementadas.

## GSettings

Si agregas nuevas preferencias, actualiza:

- `com.gutenair.gtk4.gschema.xml`
- el codigo que lee o enlaza la preferencia
- esta documentacion, si afecta al usuario
