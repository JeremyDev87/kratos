# Kratos

[![CI](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml/badge.svg)](https://github.com/JeremyDev87/kratos/actions/workflows/ci.yml)

[한국어](README.md) | [English](README.en.md) | [中文](README.zh-CN.md) | Español | [日本語](README.ja.md)

Destroy dead code ruthlessly.

[License](LICENSE) · [Contributing](CONTRIBUTING.md) · [Code of Conduct](CODE_OF_CONDUCT.md) · [Security](SECURITY.md) · [Sponsor](https://github.com/sponsors/JeremyDev87)

Kratos es una herramienta CLI para encontrar código muerto escondido dentro de tu proyecto, incluidos archivos no usados, imports rotos y módulos huérfanos, y sugerir candidatos de eliminación seguros. A medida que se acumula el legado, la base de código se vuelve más pesada y el costo de mantenimiento aumenta. Kratos se enfoca en exponer esos restos innecesarios y ayudar a que el código vuelva a sentirse ágil.

## Capacidades Principales

- Detectar archivos no usados
- Detectar dead exports
- Detectar broken imports
- Detectar módulos y componentes huérfanos
- Sugerir candidatos de eliminación seguros
- Generar reportes para adelgazar la base de código

## Inicio Rápido

```bash
npm install
npx kratos scan
npx kratos report
npx kratos clean
```

Durante el desarrollo local, también puedes ejecutar:

```bash
npm run scan -- ./fixtures/demo-app
npm run report -- ./fixtures/demo-app/.kratos/latest-report.json
npm run clean -- ./fixtures/demo-app/.kratos/latest-report.json
```

## Comandos

### `kratos scan [root]`

Escanea un proyecto y guarda el resultado del análisis en `.kratos/latest-report.json`.

Opciones:

- `--output <path>`: define una ruta personalizada para el reporte
- `--json`: imprime el JSON completo en lugar del resumen en consola

### `kratos report [report-path-or-root]`

Lee el reporte más reciente guardado y lo muestra en un formato más legible.

Opciones:

- `--format summary`: salida resumida por defecto
- `--format json`: salida JSON cruda
- `--format md`: salida del reporte en Markdown

### `kratos clean [report-path-or-root]`

Muestra candidatos de eliminación o los elimina realmente.

Opciones:

- `--apply`: ejecuta la eliminación real

El comportamiento por defecto es dry-run. Sin `--apply`, no se elimina ningún archivo.

## Qué Detecta El MVP Actual

- Grafo de archivos JS / JSX / TS / TSX / MJS / CJS
- relative import / require / dynamic import
- `baseUrl` y `paths` de `tsconfig.json` / `jsconfig.json`
- Heurísticas de entrypoints para Next.js `app/` / `pages/`
- Entrypoints de package.json `main`, `module`, `bin` y `exports`
- Candidatos de orphan file / orphan component
- Candidatos de dead export
- Candidatos de unused import
- Broken internal import

## Ejemplo De Reporte

```bash
$ npm run scan -- ./fixtures/demo-app
Kratos scan complete.

Root: /.../fixtures/demo-app
Files scanned: 5
Entrypoints: 1
Broken imports: 1
Orphan files: 2
Dead exports: 3
Unused imports: 0
Deletion candidates: 2

Saved report: /.../fixtures/demo-app/.kratos/latest-report.json
```

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

## Releases

Kratos usa etiquetas de versión semántica como `v0.2.0-alpha.1` o `v1.0.0` para publicar releases.

```bash
npm version 0.2.0-alpha.1 --no-git-tag-version
git add package*.json
git commit -m "chore: release v0.2.0-alpha.1"
git tag v0.2.0-alpha.1
git push origin HEAD
git push origin v0.2.0-alpha.1
```

Cuando se envía una etiqueta, el [release workflow](.github/workflows/release.yml):

- ejecuta `npm run verify`
- genera el tarball de publicación para npm
- publica las releases estables en npm `latest` y las prereleases en npm `next`
- crea un GitHub Release y adjunta el tarball

La configuración recomendada es npm Trusted Publishing (OIDC). Si todavía no está configurado, el workflow puede usar como fallback el secret `NPM_TOKEN` del repositorio.

## Open Source

Kratos es un proyecto open source publicado bajo la licencia MIT.

- Usa GitHub Issues para reportes de bugs y solicitudes de funcionalidades.
- No reportes problemas de seguridad públicamente; sigue el proceso de [SECURITY.md](SECURITY.md).
- Lee [CONTRIBUTING.md](CONTRIBUTING.md) y [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) antes de contribuir.
- Si quieres apoyar el proyecto, puedes patrocinarlo mediante [GitHub Sponsors](https://github.com/sponsors/JeremyDev87).

## Nota

Esta versión es un MVP heurístico, no un analizador basado en AST. Está optimizado para revisar proyectos grandes rápidamente, y siempre deberías revisar el reporte antes de borrar algo.
