---
name: Soryntel Chemistry
description: Landing de software científico de escritorio; el producto se demuestra con grabaciones reales.
colors:
  ink: "oklch(0.22 0.02 170)"
  paper: "oklch(0.968 0.004 145)"
  sheet: "oklch(0.995 0.002 145)"
  wash: "oklch(0.935 0.006 150)"
  graphite: "oklch(0.47 0.02 165)"
  hairline: "oklch(0.885 0.008 150)"
  reagent-green: "oklch(0.52 0.1 170)"
  reagent-wash: "oklch(0.92 0.04 165)"
  window-glass: "oklch(0.955 0.003 250)"
  night-band: "oklch(0.2 0.02 170)"
  night-text: "oklch(0.95 0.006 160)"
  ink-dark: "oklch(0.94 0.008 160)"
  paper-dark: "oklch(0.17 0.012 170)"
  sheet-dark: "oklch(0.205 0.013 170)"
  graphite-dark: "oklch(0.72 0.015 165)"
  reagent-green-dark: "oklch(0.78 0.12 168)"
typography:
  display:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(2.4rem, 6.4vw, 4.6rem)"
    fontWeight: 600
    lineHeight: 1.02
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "clamp(1.85rem, 3.6vw, 2.75rem)"
    fontWeight: 600
    lineHeight: 1.08
    letterSpacing: "-0.028em"
  title:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 500
    lineHeight: 1.4
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 400
    lineHeight: 1.625
  label:
    fontFamily: "Geist, ui-sans-serif, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.33
    fontFeature: "\"tnum\""
  mono:
    fontFamily: "Geist Mono, ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 400
rounded:
  sm: "7px"
  md: "12px"
  window: "14px"
  panel: "16px"
  pill: "9999px"
spacing:
  gutter: "16px"
  gutter-md: "32px"
  container: "72rem"
  stack: "20px"
  section: "clamp(4.5rem, 10vw, 8rem)"
components:
  button-primary:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    typography: "{typography.title}"
    rounded: "{rounded.pill}"
    padding: "0 24px"
    height: "48px"
  button-primary-hover:
    backgroundColor: "{colors.reagent-green}"
    textColor: "{colors.paper}"
  button-header:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.paper}"
    rounded: "{rounded.pill}"
    padding: "0 16px"
    height: "32px"
  button-icon-round:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.pill}"
    size: "40px"
  tab-demo-active:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "14px 16px 20px"
  badge-proof:
    backgroundColor: "{colors.reagent-wash}"
    textColor: "{colors.reagent-green}"
    rounded: "6px"
    padding: "4px 8px"
  app-window:
    backgroundColor: "{colors.window-glass}"
    rounded: "{rounded.window}"
  download-card:
    backgroundColor: "{colors.night-band}"
    textColor: "{colors.night-text}"
    rounded: "{rounded.panel}"
    padding: "8px"
---

# Design System: Soryntel Chemistry

## Overview

**Creative North Star: "El cuaderno de laboratorio con el programa encendido"**

Canon de software técnico, al nivel de Rerun.io: papel casi blanco con un leve verdor, tinta verde-negra y un único acento verde químico. La página no ilustra el producto; lo enmarca. Las grabaciones reales de la app, a 60 fps, viven dentro de ventanas con barra fina y sombra profunda, y son la única imagen fuerte. Todo lo demás (titulares, listas de definiciones, filetes de 1px) es tipografía sobria al servicio de esos clips.

La densidad es de documento académico: columnas de 72rem, párrafos de 58–60ch, listas `dl` separadas por filetes en lugar de tarjetas. El cierre de la página invierte a una banda nocturna (descarga y pie) para marcar el final de la lectura. El modo oscuro es equivalente y sigue `prefers-color-scheme`.

**Key Characteristics:**
- Prueba por video real dentro de un marco de ventana de app.
- Un solo acento cromático, reservado para estado y progreso.
- Filetes de 1px y listas de definiciones en lugar de tarjetas de "features".
- Sans de trabajo (Geist) con interletraje negativo en titulares; cifras tabulares.
- Una sola animación de autor: la ventana del héroe se asienta plana al hacer scroll.

## Colors

Neutros verdosos de baja croma alrededor del tono 145–170, con un acento verde químico.

### Primary
- **Verde reactivo** (`reagent-green`): hover del botón de descarga y enlaces, barra de progreso del reproductor, anillo de foco. En oscuro sube a `reagent-green-dark`.
- **Lavado reactivo** (`reagent-wash`): fondo de la insignia "Grabación real" y de la selección de texto.

### Neutral
- **Tinta** (`ink`): texto principal y fondo del botón primario.
- **Papel** (`paper`): fondo de página.
- **Hoja** (`sheet`): barras de ventana, pies de figura, banda "aula", pestaña activa.
- **Lavado** (`wash`): superficies secundarias/muted.
- **Grafito** (`graphite`): texto secundario y segunda línea del titular.
- **Filete** (`hairline`): bordes y separadores de 1px.
- **Cristal de ventana** (`window-glass`): fondo detrás del video, ligeramente frío.
- **Banda nocturna / texto nocturno** (`night-band`, `night-text`): sección de descarga y pie, en ambos modos.

### Named Rules
**The One Reagent Rule.** El verde reactivo solo marca estado (hover, foco, progreso, prueba). Nunca rellena superficies ni titulares.

**The Grey Second Line Rule.** Los titulares de dos tiempos ponen la segunda frase en grafito; el contraste lo da el valor, no un color.

## Typography

**Display Font:** Geist (variable 100–900, autoalojada; fallback ui-sans-serif)
**Body Font:** Geist
**Label/Mono Font:** Geist Mono, solo para nombres de archivo

**Character:** Una sola familia de trabajo, compacta en titulares por interletraje negativo y relajada en el cuerpo.

### Hierarchy
- **Display**: titular del héroe, centrado, en dos líneas (segunda en grafito).
- **Headline**: H2 de sección, con `text-balance`, alineado a la izquierda salvo la descarga.
- **Title**: nombres de pestaña, enlaces de acción, términos `dt`.
- **Body**: lede de sección a 17px, máx. 60ch, en grafito.
- **Label**: notas de descarga, pies de ventana; cifras siempre tabulares.

### Named Rules
**The Tabular Figures Rule.** Tamaños, versiones y valores numéricos llevan `tabular-nums`. El mono queda para nombres de archivo literales.

## Layout

Contenedor centrado `min(100% - 2rem, 72rem)` (64px de margen total desde `md`). Secciones con ritmo vertical fluido `clamp(4.5rem, 10vw, 8rem)`. Héroe centrado; el resto alinea a la izquierda con encabezado de máx. 42rem y mb de 48px. Rejillas a dos columnas desde `md` (texto | `dl`), módulos a 2/4 columnas en `sm`/`lg`. El reproductor pasa de pestañas horizontales con scroll (móvil) a una columna lateral de 300px (`lg`). La galería sangra hacia el borde derecho con scroll-snap. En móvil los videos recortan a 4:5 con `object-position` por clip.

## Elevation & Depth

Plano por defecto; la profundidad se reserva a los marcos que contienen el producto.

### Shadow Vocabulary
- **Ventana** (`--shadow-window`): borde de 1px como sombra más caída larga y difusa tintada de tinta. Solo en ventanas de app.
- **Captura**: anillo de 1px + caída corta, en las imágenes de galería.
- **Anillo**: `0 0 0 1px var(--border)` para pestaña activa y botones redondos; sustituye al borde.

### Named Rules
**The Only Windows Float Rule.** Solo el producto proyecta sombra profunda. Texto, listas y secciones permanecen planos.

## Shapes

Radios suaves y consistentes: 12px en pestañas y capturas, 14px en ventanas, 16px en tarjetas de descarga, píldora completa en botones y controles de icono. Separadores de 1px; las secciones se delimitan con filetes horizontales o cambio de superficie, nunca con cajas.

## Components

### Buttons
- **Shape:** píldora completa.
- **Primary:** tinta sobre papel, 48px de alto, 24px laterales; detecta el SO y se acompaña de una nota tabular debajo.
- **Hover / Focus:** pasa a verde reactivo (200ms); foco con contorno de 2px en anillo y 3px de separación.
- **Secundario:** enlace de texto subrayado con filete, offset 6px; subrayado y texto a verde en hover.
- **Icono redondo:** 36–40px, hoja con anillo de 1px; en galería invierte a tinta en hover.

### Chips
- **Insignia de prueba:** lavado reactivo con texto verde, 12px medio, radio 6px. Solo para declarar que un clip es grabación real.

### Navigation
- Cabecera sticky de 64px, papel al 80% con desenfoque; el filete inferior aparece tras 8px de scroll. Enlaces 14px en grafito → tinta. En móvil se ocultan los enlaces y queda el botón "Descargar".

### App Window (signature)
Figura con barra de 36px (tres puntos neutros y título "Soryntel Chemistry — módulo"), video y pie opcional en hoja. Es el único marco de producto del sistema.

### Demo Player (signature)
Lista de pestañas con título y resumen desplegable; la activa lleva barra de progreso de 2px en verde reactivo que avanza con el clip y salta al siguiente al terminar. Control de pausa siempre visible; respeta `prefers-reduced-motion`.

## Do's and Don'ts

### Do:
- **Do** mostrar el producto solo con grabaciones o capturas reales dentro de la ventana de app.
- **Do** usar listas de definiciones con filetes de 1px para enumerar capacidades.
- **Do** limitar el verde reactivo a hover, foco, progreso y la insignia de prueba.
- **Do** pausar video fuera de pantalla y no autoreproducir con movimiento reducido.

### Don't:
- **Don't** usar mockups ilustrados ni rejillas de tarjetas de "features".
- **Don't** aplicar sombra profunda a nada que no sea una ventana de app.
- **Don't** mostrar marcas de terceros en texto ni en capturas.
