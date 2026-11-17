# Soryntel Chemistry

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Next.js (App Router, TypeScript) con Tailwind CSS y shadcn/ui, en `web/`, exportable como sitio estático. Aislado del ejecutable de escritorio.

## Product Purpose

Presentar y demostrar visualmente las capacidades de Soryntel Chemistry, suite de escritorio en Rust (egui) con 20 módulos, distribuida para Linux y Windows (v0.1.0 en `soryntel/dist/`). La versión C++ previa se identificaba como CMA Chemistry.

## Users

Público principal: académico. Docentes, investigadores y estudiantes de química, ingeniería química y metalúrgica que necesitan una herramienta accesible de termodinámica aplicada. Acción principal: descargar la aplicación gratis (Windows o Linux).

## Capabilities and Constraints

El repositorio incluye cálculos de equilibrio por minimización de Gibbs, propiedades de agua y vapor IF97, equilibrio acuoso con IPhreeqc y MINTEQ, procesamiento mineral, balances y diagramas de estabilidad. Las demostraciones de la web deben distinguirse de resultados ejecutados por el motor científico. No se debe afirmar equivalencia científica con HSC ni calibración industrial general.

## Evidence on Hand

- `README.md`: estructura y módulos actuales.
- `docs/SCIENTIFIC_VALIDATION.md`: casos verificados y límites.
- `docs/HSC_PARITY_ROADMAP.md`: alcance presente y pendientes.
- Grabaciones reales de la app compilada (agua/vapor, reacciones, Benson 3D, tabla periódica) y capturas de módulos en `web/public/media/`, generadas por `web/scripts/capture/`.
- La interfaz de la app aún muestra textos "Outotec" y "HSC" en algunos módulos y el icono de empaquetado dice "HSC": no deben aparecer en la web.

## Brand Commitments

Nombre: Soryntel Chemistry. Idioma de la web: español. Sin marcas de terceros (HSC, Outotec, Metso) en la web.

Dirección visual: estándar de la categoría (software técnico con video real del producto), elegido de forma explícita. Vara de calidad: Rerun.io.

## Product Principles

- Mostrar qué hace el software con ejemplos que se entiendan a primera vista.
- Distinguir visualización explicativa de cálculo científico real.
- Mantener la web separada del motor para adaptarla a la futura versión en Rust.
