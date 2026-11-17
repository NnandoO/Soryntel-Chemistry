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
3. `.github/workflows/release.yml` compila Linux (Ubuntu 22.04), Windows (MinGW + NSIS) y Arch (contenedor), comprueba los cinco archivos que ofrece la web, crea la versión en GitHub Releases con `SHA256SUMS.txt` y lanza el despliegue de la web, que toma los nuevos enlaces y tamaños.

La etiqueta debe coincidir con la versión de `Cargo.toml`; si no, el flujo se detiene.

## Despliegue en Cloudflare Pages

`.github/workflows/web.yml` despliega `out/` en cada push a `main` que toque `web/`, en cada versión publicada y a mano. Los PR generan una vista previa.

Configuración única:

1. En GitHub → Settings → Secrets and variables → Actions, crear `CLOUDFLARE_API_TOKEN` (token con permiso *Cloudflare Pages: Edit*) y `CLOUDFLARE_ACCOUNT_ID`. Opcional: variable `CF_PAGES_PROJECT` (por defecto `soryntel-chemistry`).
2. El proyecto de Pages `soryntel-chemistry` ya existe y tiene vinculado `chemistry.soryntel.com`; si se recrea, el flujo lo crea de nuevo.
3. DNS de la zona `soryntel.com`: registro `CNAME chemistry → soryntel-chemistry.pages.dev` con proxy activado.

El dominio público se usa en los metadatos (`SITE_URL` en `src/content/site.ts`, sobrescribible con `NEXT_PUBLIC_SITE_URL`). Las cabeceras de caché y seguridad están en `public/_headers`.

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
