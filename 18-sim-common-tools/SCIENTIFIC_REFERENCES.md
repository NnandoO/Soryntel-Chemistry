# Referencias Científicas y Fuentes Primarias — Módulo 18: Sim Common Tools

## 1. Documentación Primaria del Producto (Clean-Room Source Baseline)

1. **Metso Outotec (2023).** *HSC Chemistry® 10 User's Manual — Chapter 40: Sim Common Tools* (versión documentada del módulo en capturas: `HSC Sim Flowsheet Module 10.3.1.0`, `6 de septiembre de 2023`, páginas 1–37, figuras 1–72).
   - Autores del capítulo: **Petri Kobylin**, **Lauri Mäenpää**, **Matti Hietala**, **Jussi-Pekka Kentala**, **Fedor Vasilyev**, **Ran Liu**.
   - Archivo fuente: [40-sim-common-tools.md](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/40-sim-common-tools.md) e [imagenes/](file:///home/nando/Documentos/Docs%20HSC/40-sim-common-tools/imagenes).
   - Desarrolladores y asesores técnicos listados en `About HSC Sim` (`Fig. 43`): **Jussi-Pekka Kentala**, **Matti Peltomäki**, **Matti Hietala**, **Bishal Karki**, **Jarkko Mansikka-aho**, **Pertti Lamberg** (Software Developers); **Pertti Lamberg**, **Antti Remes**, **Peter Björklund**, **Tuukka Kotiranta**, **Heikki Eerola**, **Markus Reuter** (Technical Advisors).

2. **Outokumpu Research Oy (2006).** *HSC Chemistry® 6 Help — Chapter 40: Sim Flowsheet* (doc. ref. `06120-ORC-T`, autor **Antti Roine**).
   - Fuente heredada para trazabilidad histórica (`*.fls` + libros `*.xls` externos por unidad): [40 Sim Flowsheet.txt](file:///home/nando/Descargas/HSC6/Help/40%20Sim%20Flowsheet.txt).

3. **Artefactos del Workspace Relacionados (Capítulos Adyacentes y Base Local):**
   - Análisis de Ciclo de Vida sobre diagramas de flujo: [49-sim-lca](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca).
   - Simulación dinámica detallada: [55-sim-dynamic-simulations-2025](file:///home/nando/Documentos/Docs%20HSC/55-sim-dynamic-simulations-2025).
   - Prototipo de esquema JSON en disco: [proceso_hsc.simjson](file:///home/nando/HSC6-Rust/proceso_hsc.simjson).
   - Implementación objetivo en Rust: [soryntel](file:///home/nando/Documentos/HSC-REA/soryntel).

---

## 2. Referencias Metodológicas de Ingeniería de Procesos y Algoritmos Clean-Room

Para la implementación independiente (*clean-room*) de los algoritmos de resolución estructural, control e incertidumbre referenciados nominalmente en el capítulo `40`:

4. **Tarjan, R. E. (1972).** *"Depth-first search and linear graph algorithms"*, *SIAM Journal on Computing*, 1(2), 146–160. *(Base estándar para descomposición en componentes fuertemente conexas —SCC— e identificación de lazos de reciclo en grafos de flowsheet).*
5. **Wegstein, J. H. (1958).** *"Accelerating convergence of iterative processes"*, *Communications of the ACM*, 1(6), 9–13; y **Westerberg, A. W., Hutchison, H. P., Motard, R. L., Winter, P. (1979).** *Process Flowsheeting*, Cambridge University Press. *(Fundamento de simulación secuencial modular, corrientes de corte y convergencia de reciclos `MB`/`EB`).*
6. **Åström, K. J., & Hägglund, T. (2006).** *Advanced PID Control*, ISA — The Instrumentation, Systems, and Automation Society; y **Press, W. H. et al. (2007).** *Numerical Recipes*, 3rd Ed. *(Métodos de la secante/cuasi-Newton para `Tangential (fast)` en lazos estáticos y discretización PID con saturación y anti-windup para simulación dinámica).*
7. **Devroye, L. (1986).** *Non-Uniform Random Variate Generation*, Springer-Verlag; y **van Dorp, J. R., & Kotz, S. (2003).** *"Generalizations of two-sided power distributions and their convolution" / Distribuciones trapezoidales y Laplace en análisis Monte Carlo*. *(Base estadística para el muestreador de `Scenario Editor`: `Uniform`, `Normal` truncada, `Laplace` y `Trapezoid`).*
8. **Riekki, M., & Schmidt, M. (2008) / Schmidt, M. (2008).** *"The Sankey Diagram in Energy and Material Flow Management"*, *Journal of Industrial Ecology*, 12(1–2), 82–94, 173–185. *(Referencia conceptual para visualización proporcional de flujos de materia y energía mediante anchos de línea y rangos cromáticos).*
