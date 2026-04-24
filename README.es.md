# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | Español | [日本語](README.ja.md)

Elimina código muerto sin piedad.

[Licencia](LICENSE) · [Contribución](CONTRIBUTING.md) · [Código de conducta](CODE_OF_CONDUCT.md) · [Seguridad](SECURITY.md) · [Patrocinio](https://github.com/sponsors/JeremyDev87)

Kratos es una herramienta CLI para proyectos JavaScript y TypeScript. Encuentra archivos no usados, imports rotos, exports no usados y módulos huérfanos, y escribe los resultados en un reporte. La implementación actual combina un core/CLI en Rust con un launcher de npm, y el paquete npm `@jeremyfellaz/kratos` carga un addon nativo opcional específico de la plataforma.

Kratos es una herramienta de análisis para un flujo de limpieza seguro, no un bot de eliminación automática. `clean` usa dry-run por defecto, y los archivos solo se eliminan después de revisar el reporte y pasar `--apply` explícitamente.

## Capacidades Principales

- Detectar archivos no usados y candidatos a componentes o módulos huérfanos
- Detectar imports internos rotos
- Detectar candidatos a exports e imports no usados
- Aplicar heurísticas de route entrypoints para Next.js `app/` / `pages/`
- Resolver aliases `baseUrl` y `paths` de `tsconfig.json` / `jsconfig.json`
- Resolver entrypoints `main`, `module`, `types`, `bin` y `exports` de `package.json`
- Imprimir reportes guardados como resumen, JSON o Markdown
- Comparar cambios de hallazgos entre dos reportes
- Previsualizar candidatos de eliminación segura con un umbral de confianza

## Inicio Rápido

Para quienes usan el paquete, el entrypoint por defecto es `npx`.

```bash
npx @jeremyfellaz/kratos scan ./my-app
npx @jeremyfellaz/kratos report ./my-app
npx @jeremyfellaz/kratos report ./my-app --format md
npx @jeremyfellaz/kratos clean ./my-app --min-confidence 0.9
```

Añade `--apply` solo después de revisar el reporte y decidir eliminar los objetivos listados.

```bash
npx @jeremyfellaz/kratos clean ./my-app --apply --min-confidence 0.9
```

También puedes comparar reportes de dos momentos distintos.

```bash
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/before.json
# limpia código o cambia de rama
npx @jeremyfellaz/kratos scan ./my-app --output .kratos/after.json
npx @jeremyfellaz/kratos diff ./my-app/.kratos/before.json ./my-app/.kratos/after.json
```

Cuando `scan --output` recibe una ruta relativa, se resuelve desde la raíz escaneada. El reporte guardado por defecto es `<root>/.kratos/latest-report.json`.

## Comandos

### `kratos scan [root] [--output path] [--json]`

Analiza un proyecto y escribe un archivo JSON de reporte.

- Omite `root` para escanear el directorio de trabajo actual.
- `--output path` define la ruta de salida del reporte.
- `--json` imprime el JSON completo en stdout en lugar del resumen de consola.
- La ruta de salida por defecto es `<root>/.kratos/latest-report.json`.

### `kratos report [report-path-or-root] [--format summary|json|md]`

Imprime un reporte guardado en un formato legible o como JSON original.

- `summary` es el resumen de consola por defecto.
- `json` imprime el JSON guardado con formato.
- `md` imprime un reporte Markdown fácil de compartir.
- Si la entrada es una raíz de proyecto, Kratos resuelve `.kratos/latest-report.json` automáticamente.

### `kratos diff [before-report-path-or-root] [after-report-path-or-root] [--format summary|json|md]`

Compara los cambios de hallazgos entre dos reportes.

- El formato por defecto es `summary`.
- `json` imprime hallazgos introduced/resolved/persisted en una forma legible por máquina.
- `md` imprime un diff Markdown útil para revisiones o issues.
- Cada entrada puede ser una ruta de reporte o una raíz de proyecto.

### `kratos clean [report-path-or-root] [--apply] [--min-confidence value]`

Previsualiza candidatos de eliminación o los elimina.

- El comportamiento por defecto es dry-run.
- Los archivos solo se eliminan cuando `--apply` está presente.
- `--min-confidence value` es un umbral de confianza de `0.0` a `1.0`.
- Si omites `--min-confidence`, Kratos lee `thresholds.cleanMinConfidence` de `kratos.config.json`; si no existe esa configuración, usa `0.0`.

## Ejemplos De Salida

Al escanear `fixtures/demo-app`, el resumen tiene esta forma.

```text
Kratos scan complete.

Root: <root>
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Route entrypoints: 1
Deletion candidates: 2

Saved report: <root>/.kratos/latest-report.json

Broken imports:
- <root>/src/lib/broken.ts -> ./missing-helper

Orphan files:
- <root>/src/components/DeadWidget.tsx
- <root>/src/lib/broken.ts

Dead exports:
- <root>/src/components/DeadWidget.tsx#DeadWidget
- <root>/src/lib/broken.ts#brokenFeature
- <root>/src/lib/math.ts#multiply
```

`clean --min-confidence 0.9` separa los objetivos de eliminación de los candidatos omitidos por el umbral.

```text
Kratos clean dry run.

Deletion targets: 1
- <root>/src/components/DeadWidget.tsx (confidence 0.92, Component-like module has no inbound references.)

Threshold-skipped targets: 1
- <root>/src/lib/broken.ts (confidence 0.88, Module has no inbound references and is not treated as an entrypoint.)

Re-run with --apply to delete these files.
```

Comparar reportes idénticos no muestra hallazgos introducidos ni resueltos, solo conteos persistentes.

```text
Kratos diff complete.

Before: <before-report>
After: <after-report>

Broken imports: introduced 0, resolved 0, persisted 1
Orphan files: introduced 0, resolved 0, persisted 2
Dead exports: introduced 0, resolved 0, persisted 3
Unused imports: introduced 0, resolved 0, persisted 0
Route entrypoints: introduced 0, resolved 0, persisted 1
Deletion candidates: introduced 0, resolved 0, persisted 2

Totals: introduced 0, resolved 0, persisted 9
```

## Esquema Del Reporte

Actualmente `scan` escribe reportes con `schemaVersion: 2`.

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

`findings` contiene `brokenImports`, `orphanFiles`, `deadExports`, `unusedImports`, `routeEntrypoints` y `deletionCandidates`. `graph.modules` registra rutas de módulos analizados, estado de entrypoint y conteos de imports/exports.

## Configuración

Puedes colocar `kratos.config.json` en la raíz del proyecto. Se aceptan comentarios estilo JSONC y comas finales.

```json
{
  "ignore": ["storybook-static", "generated"],
  "ignorePatterns": ["src/generated/**", "!src/generated/keep.ts"],
  "entry": ["src/bootstrap.ts"],
  "roots": ["src", "app", "pages"],
  "thresholds": {
    "cleanMinConfidence": 0.85
  },
  "suppressions": [
    {
      "kind": "deadExport",
      "file": "src/components/LazyCard.tsx",
      "export": "default",
      "reason": "Loaded dynamically by route metadata."
    },
    {
      "kind": "brokenImport",
      "file": "src/legacy/shim.ts",
      "source": "./generated-shim",
      "reason": "Generated at deploy time."
    }
  ]
}
```

- `ignore`: nombres de directorios añadidos a la lista de ignore por defecto.
- `ignorePatterns`: patrones de ruta estilo `.gitignore`. Usa negación con `!` para excepciones.
- Después de los directorios ignorados por defecto, Kratos también lee automáticamente el `.gitignore` de la raíz del proyecto y luego aplica `ignorePatterns` para excepciones u overrides.
- `entry`: archivos relativos a la raíz del proyecto que deben forzarse como entrypoints.
- `roots`: directorios relativos a la raíz del proyecto que limitan el alcance del escaneo.
- `thresholds.cleanMinConfidence`: umbral de confianza por defecto para `clean`.
- `suppressions`: hallazgos que deben ignorarse intencionadamente. `kind` debe ser uno de `brokenImport`, `orphanFile`, `deadExport`, `unusedImport` o `deletionCandidate`.

Si existe `.kratos/suppressions.json`, Kratos lo lee con el mismo formato de suppression. Los valores `file` deben ser relativos a la raíz del proyecto.

## Desarrollo Local

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

En un checkout del repositorio, los paquetes publicados del addon nativo pueden no existir todavía, así que estos comandos son más seguros que `npx @jeremyfellaz/kratos ...`.

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
cargo run -p kratos-cli -- diff ./fixtures/demo-app ./fixtures/demo-app
```

## Distribución

- El paquete npm raíz es `@jeremyfellaz/kratos`.
- El nombre del binario CLI es `kratos`.
- Los paquetes addon por plataforma apuntan a macOS arm64/x64, Linux x64/arm64 y Windows x64.
- Las `optionalDependencies` del paquete raíz apuntan a paquetes addon de plataforma con la misma versión de lanzamiento.
- Un checkout sin publicar puede no tener addon nativo, pero el launcher del paquete publicado carga el addon de la plataforma actual.

## Flujo De Release

Los lanzamientos se basan en tags semver como `vX.Y.Z` o `vX.Y.Z-prerelease.N`.

- El workflow `Manual Release Bump` prepara un PR solo de versión que alinea `package.json` y las `optionalDependencies` de plataforma.
- Antes de crear el tag, el mismo commit debe pasar `cargo test --workspace`, `npm run verify` y la CI de empaquetado nativo.
- El workflow `Release Publish` hace checkout del tag exacto, verifica el paquete Node, ejecuta los tests del workspace Rust y construye artefactos nativos por plataforma.
- Los paquetes npm addon de plataforma se empaquetan, se smoke-testean y se publican primero; el paquete raíz se publica al final y después se crea o actualiza el GitHub Release.
- El workflow `Release Published Follow-up` audita si el lanzamiento publicado tiene un publish run exitoso correspondiente y assets de release. No vuelve a ejecutar la publicación.

Acciones como publicar tags de lanzamiento, publicar en npm o publicar un GitHub Release solo deben ocurrir después de la confirmación del maintainer.

## Código Abierto

Kratos es un proyecto de código abierto bajo la licencia MIT.

- Usa GitHub Issues para reportar bugs y solicitar funcionalidades.
- No reportes problemas de seguridad públicamente; sigue el proceso de [SECURITY.md](SECURITY.md).
- Lee [CONTRIBUTING.md](CONTRIBUTING.md) y [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) antes de contribuir.
- Si quieres apoyar el proyecto, puedes patrocinarlo mediante [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Nota

Kratos combina análisis estático conservador con heurísticas. Los imports dinámicos, las convenciones de frameworks, los archivos generados y los entrypoints solo usados en runtime pueden variar según el proyecto, así que revisa el reporte y el diff antes de ejecutar `--apply`.
