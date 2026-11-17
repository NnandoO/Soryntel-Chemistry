# Referencias Científicas y Técnicas — Módulo 15: Excel Add-Ins

1. **Outokumpu Research Oy / Outotec (2006).** *HSC Chemistry 6.0 User's Guide — Chapter 27: Excel Add-In Functions* (`Help/27 Excel Add-Ins.doc`, `Help/27 Excel Add-Ins.txt`), Peter Björklund, August 10, 2006 (`06120-ORC-T`).
2. **Outokumpu Research Oy (2006).** *HSC Chemistry 6.0 User's Guide — Chapter 35: HSC DLL Tools* (`Help/35 HSC DLL Tools.txt`) & *Chapter 57: Mineral Based Models* (`Help/57 Mineral Based Models.txt`, definiciones de `StreamX`, `RecoveryX`, `ParticleRecX`, `FractionX`, `MineralX`).
3. **Artefactos Binarios y Hojas de Referencia de HSC6 (`/home/nando/Descargas/HSC6/AddIns/`):**
   - `HSC6.XLL` (`xlAutoOpen`, tabla de registro `xlfRegister` de 69 funciones bajo categoría `"HSC"`).
   - `HSC6.DLL` (servidor termoquímico unificado HSC + Gibbs).
   - `AddInSample.xls` (122 filas de prueba y valores de referencia para especies, reacciones, flujos y bloque matricial `StreamEQ`).
   - `Update XLS Files.xls` (macro VBA de migración `HSC5.xla` $\to$ `HSC6.XLL`).
4. **Microsoft Excel C API / XLL SDK:** Convención `xlfRegister` de tipos de argumento (`R`, `C`, `I`, `J`, marca volátil `!`) y semántica de errores de hoja de cálculo (`#VALUE!`, `#NAME?`, `#NUM!`, `#N/A`) y bloques matriciales CSE (`Ctrl+Shift+Enter`).
5. **Métodos Termodinámicos y Numéricos Subyacentes:**
   - **K. K. Kelley (1960):** Parametrización polinomial estándar de capacidad calorífica $C_p(T) = A + BT\cdot 10^{-3} + C T^{-2}\cdot 10^5 + D T^2\cdot 10^{-6}$.
   - **G. Eriksson (1971, 1975) — SOLGASMIX / White-Johnson-Dantzig:** Minimización de energía libre de Gibbs multifase para `StreamEQ`.
   - **Brent / Bisección Robusta en Intervalos:** Búsqueda de raíces monótonas para `Tat*`, `StreamT*` y `FlowT*`.
   - **Propiedades Volumétricas de Electrolitos Acuosos (`DensityA`):** Basado en `Databases/Aqueous Solution Density.xls` y volúmenes molares aparentes de Millero / Pitzer.
