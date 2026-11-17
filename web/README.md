# Web de Soryntel Chemistry

Página de producto en Next.js (App Router) con Tailwind CSS 4 y shadcn/ui. Se exporta como sitio estático.

```sh
npm install
npm run dev     # http://localhost:3000
npm run build   # genera out/, listo para cualquier hosting estático
```

- Contenido (clips, módulos, descargas): `src/content/site.ts`
- Página: `src/app/page.tsx`; componentes en `src/components/`
- Tipografía Geist autoalojada en `src/app/fonts/` (SIL OFL)
- Logo: `public/brand/` (color, `-white` para fondos oscuros, `-mono` con `currentColor` e `-icon` cuadrado), vectorizado desde `design/logo-source.png`
- Las descargas salen de `src/content/release.json`, que genera `npm run sync-release` desde la versión publicada en GitHub (nombre, tamaño real y URL de cada archivo). Un archivo que no exista en la versión no se ofrece.

## Publicar una versión de la app

1. Subir `version` en `soryntel/Cargo.toml` (`[workspace.package]`) y hacer commit en `main`.
2. `git tag v0.2.0 && git push origin v0.2.0`
3. `.github/workflows/release.yml` compila Linux (Ubuntu 22.04), Windows (MinGW + NSIS) y Arch (contenedor), comprueba los cinco archivos que ofrece la web, crea la versión en GitHub Releases con `SHA256SUMS.txt` y pide a Cloudflare reconstruir la web, que toma los nuevos enlaces y tamaños.

La etiqueta debe coincidir con la versión de `Cargo.toml`; si no, el flujo se detiene.

## Despliegue en Cloudflare Pages

El proyecto `soryntel-chemistry` de Cloudflare Pages está conectado a `jhulio435m/Soryntel-Chemistry` y publica `chemistry.soryntel.com`:

- Cada push a `main` que toque `web/` despliega producción; las demás ramas y los PR generan vistas previas.
- Configuración de build: directorio raíz `web`, comando `node scripts/sync-release.mjs --soft && npm run build`, salida `out`, variables `NODE_VERSION=26` y `GITHUB_REPOSITORY=jhulio435m/Soryntel-Chemistry`.
- Al publicar una versión, `release.yml` llama al Deploy Hook guardado en el secreto `CF_PAGES_DEPLOY_HOOK` para reconstruir la web con los nuevos enlaces y tamaños.
- DNS de la zona `soryntel.com`: `CNAME chemistry → soryntel-chemistry.pages.dev` con proxy.

Las cabeceras de caché y seguridad están en `public/_headers`. El dominio público de los metadatos es `SITE_URL` en `src/content/site.ts` (sobrescribible con `NEXT_PUBLIC_SITE_URL`).

## Clips del programa (`public/media/`)

Son grabaciones reales del binario compilado, generadas con `scripts/capture/`:

```sh
export APP=/ruta/a/soryntel-desktop        # binario de Linux de soryntel/dist
cd scripts/capture
./record.sh water water                    # módulo, nombre del guion en acts/
./post.sh water                            # 60 fps, cursor, mp4 + webm + póster
```

Requisitos: `Xvfb`, `xdotool`, `ffmpeg`, `magick`. La app renderiza por software en la pantalla virtual, así que el guion la maneja a 1/3 de velocidad (`SLOW=3`) y `post.sh` acelera el video el mismo factor; el cursor se compone a partir de marcas de tiempo de reloj (`-copyts`), alineado con cada fotograma.

Las capturas `*.webp` se recortan para excluir la barra de estado de la app, que aún muestra marcas de terceros.
