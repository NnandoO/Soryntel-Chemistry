# GHIDRA_ANALYSIS — Ghidra Necessity & Technical Evaluation

## 1. Evaluación de Necesidad Técnica de Ghidra

Se ha evaluado la conveniencia de someter los binarios de HSC Chemistry 6.0 (`HSC.EXE` y `HSC6.DLL`) a descompilación estática profunda en Ghidra.

### Criterios de Evaluación:
1. **Naturaleza del Binario:**
   - La evidencia de REA y el análisis de cadenas demuestran que `HSC.EXE` es una aplicación Win32 de 32 bits generada por el compilador de **Microsoft Visual Basic 6.0** vinculada a la máquina virtual `MSVBVM60.DLL` y `VBA6.DLL`.
   - Las aplicaciones VB6 estructuran su lógica alrededor de tablas de despacho de eventos COM (`__vba*`), llamadas tardías de automatización (`__vbaLateIdCall`) y despacho de métodos OLE IDispatch.
   - Ghidra decompila a nivel de ensamblador C/x86, lo cual en binarios VB6 no reconstruye limpiamente el código fuente original, sino miles de llamadas opacas a funciones de soporte de cadenas y variantes del runtime de Visual Basic (`__vbaStrCmp`, `__vbaVarMove`, etc.).

2. **Suficiencia de Fuentes Teóricas y Documentales:**
   - A diferencia de un protocolo de red cerrado o un algoritmo criptográfico propietario, **las ecuaciones de termodinámica química de HSC Chemistry están total y completamente documentadas en el manual oficial del autor (Antti Roine), en el archivo de texto `08 Introduction.txt`, `10 Reaction Equations.txt`, `28 HSC - Databases.txt` y en la literatura revisada por pares (Barin, Knacke, Kubaschewski, Kelley, Criss & Cobble)**.
   - Las expresiones de integración de $C_p$, entalpía $H(T)$, entropía $S(T)$, energía de Gibbs $G(T)$, constantes $K(T)$ y potenciales redox de Nernst $E^\circ$ son leyes físicas inmutables estándar de la fisicoquímica.
   - No existe un "algoritmo secreto propietario" en este módulo que justifique una ingeniería inversa destructiva o compleja con Ghidra.

3. **Restricciones Éticas y de Licencia:**
   - De acuerdo con la **Regla 10 y Sección 5** de la Instrucción Maestra, se debe evitar el descompilado innecesario y la copia o reutilización de código propietario o ingeniería inversa invasiva cuando los fundamentos científicos ya son transparentes y verificables por literatura técnica abierta.

---

## 2. Decisión del Equipo de Investigación

- **Dictamen:** **No es necesario ni recomendable someter los binarios VB6 a Ghidra para el módulo Reaction Equations.**
- **Justificación:** Los modelos matemáticos, ecuaciones de estado, polinomios de Kelley, métodos de extrapolación (Criss-Cobble) y convenciones termodinámicas (IUPAC, Barin-Knacke) se encuentran al 100% formalizados y validados a partir de las fuentes científicas y la documentación técnica oficial. 
- La implementación en **Rust** para **Soryntel Chemistry** se desarrollará a partir de principios termodinámicos puros, garantizando total independencia tecnológica y máxima eficiencia computacional.
