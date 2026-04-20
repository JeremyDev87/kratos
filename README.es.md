# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | Español | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos es una herramienta CLI para encontrar código muerto escondido dentro de tu proyecto, incluidos archivos no usados, imports rotos y módulos huérfanos, y sugerir candidatos de eliminación seguros. La distribución actual combina un core/CLI en Rust con un launcher de npm y se centra en un análisis conservador y en limpiezas seguras.

## Capacidades Principales

- Detectar archivos no usados
- Detectar dead exports
- Detectar broken imports
- Detectar módulos y componentes huérfanos
- Sugerir candidatos de eliminación seguros
- Generar reportes para adelgazar la base de código

## Inicio Rápido

Para usar el paquete, la entrada por defecto es `npx`.

```bash
npx kratos scan ./your-project
npx kratos report ./your-project/.kratos/latest-report.json
npx kratos clean ./your-project/.kratos/latest-report.json
```

- `scan` escribe `.kratos/latest-report.json` por defecto.
- `clean` es dry-run por defecto y solo elimina archivos cuando añades `--apply`.

## Desarrollo Local

En un checkout del repositorio, usa el CLI de Rust y los scripts de npm.

Requisitos:

- Node.js 18+
- npm 9+
- Rust stable toolchain

Instalación:

```bash
npm install
```

Verificación recomendada:

```bash
cargo test --workspace
npm run verify
npm run smoke
```

Para probar el CLI dentro del repo:

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

En un checkout es posible que todavía no existan los paquetes publicados del addon nativo, así que conviene usar los comandos anteriores o `cargo run -p kratos-cli -- ...` en lugar de `npx kratos ...`.

## Comandos

### `kratos scan [root]`

Escanea un proyecto y guarda el último reporte.

- ruta de salida por defecto: `<root>/.kratos/latest-report.json`
- `--output <path>`: define una ruta personalizada para el reporte
- `--json`: imprime el JSON completo en stdout en lugar del resumen de consola

### `kratos report [report-path-or-root]`

Imprime un reporte guardado en formato summary, JSON o Markdown.

- `--format summary`: salida resumida por defecto
- `--format json`: salida JSON cruda
- `--format md`: salida del reporte en Markdown
- si pasas la raíz del proyecto en lugar del path del reporte, Kratos resuelve automáticamente el último reporte

### `kratos clean [report-path-or-root]`

Muestra candidatos de eliminación o los elimina realmente.

- dry-run por defecto
- `--apply`: ejecuta la eliminación real
- si pasas la raíz del proyecto en lugar del path del reporte, Kratos resuelve automáticamente el último reporte

## Esquema Del Reporte

La salida actual del scan en Rust escribe `schemaVersion: 2`.

```json
{
  "schemaVersion": 2,
  "summary": {
    "filesScanned": 5,
    "entrypoints": 1,
    "brokenImports": 1,
    "orphanFiles": 2,
    "deadExports": 3,
    "unusedImports": 0,
    "routeEntrypoints": 1,
    "deletionCandidates": 2
  }
}
```

- la ubicación guardada por defecto es `.kratos/latest-report.json`
- `report` y `clean` aceptan tanto la ruta del reporte como la raíz del proyecto
- la salida Markdown incluye broken imports, orphan files, dead exports, route entrypoints y deletion candidates

## Cobertura Actual

- Analizador en Rust con parsing de imports/exports JS/TS basado en Oxc
- relative import / require / dynamic import
- `baseUrl` y `paths` de `tsconfig.json` / `jsconfig.json`
- heurísticas de route entrypoints para Next.js `app/` / `pages/`
- entrypoints `main`, `module`, `bin` y `exports` de `package.json`
- candidatos de orphan file / orphan component
- candidatos de dead export
- candidatos de unused import
- broken internal imports

## Configuración

Opcionalmente puedes añadir `kratos.config.json`.

```json
{
  "ignore": ["storybook-static", "generated"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"]
}
```

- `ignore`: nombres de directorios adicionales que se deben ignorar
- `entry`: rutas de archivos que deben tratarse como entrypoints
- `roots`: carpetas para limitar el alcance del escaneo

## Recomendado Para

- Proyectos React / Next.js antiguos
- Equipos con muchas funciones lanzadas y mucho código acumulado
- Equipos que buscan el momento adecuado para refactorizar

## Flujo De Release

La automatización de release de Kratos se basa en etiquetas semánticas como `v0.2.0-alpha.1` y `v0.2.0`.

Preparación del alpha:

- mantén la versión del paquete raíz en `0.2.0-alpha.1`
- antes de etiquetar, ejecuta `cargo test --workspace`, `npm run verify`, `npm run smoke` y los smoke tests de `scan/report/clean` con fixtures
- crea y publica la etiqueta alpha solo después de la confirmación del maintainer

El [release workflow](.github/workflows/release.yml) se ejecuta con una etiqueta push o con un disparo manual sobre una etiqueta existente y luego:

- resuelve la metadata del release y asigna a las prereleases el dist-tag `next`
- verifica por separado el paquete de Node y el workspace de Rust
- construye artefactos nativos para macOS arm64/x64, Linux x64/arm64 y Windows x64
- empaqueta y smoke-testea primero los paquetes npm addon por plataforma
- publica al final el paquete raíz `kratos` y crea el GitHub Release

Promoción a stable:

- después de validar el alpha, sube `package.json` de `0.2.0-alpha.1` a `0.2.0` en un commit de release-prep solo de versión
- crea la etiqueta stable `v0.2.0` en un paso separado, que publica a npm `latest`

La configuración recomendada de publicación es npm Trusted Publishing (OIDC). Cuando haga falta, también puede usarse el fallback con el secret `NPM_TOKEN` del repositorio.

## Open Source

Kratos es un proyecto open source publicado bajo la licencia MIT.

- Usa GitHub Issues para reportes de bugs y solicitudes de funcionalidades.
- No reportes problemas de seguridad públicamente; sigue el proceso de [SECURITY.md](SECURITY.md).
- Lee [CONTRIBUTING.md](CONTRIBUTING.md) y [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) antes de contribuir.
- Si quieres apoyar el proyecto, puedes patrocinarlo mediante [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Nota

La alpha actual usa un core en Rust y parsing basado en Oxc, pero la detección de entrypoints y los candidatos de borrado seguro todavía incluyen heurísticas conservadoras. Revisa siempre el reporte antes de ejecutar `--apply`.
