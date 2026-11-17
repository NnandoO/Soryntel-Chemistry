# Plan de Validación — Módulo 11: Benson Estimation

Todos los casos comparan la estimación en fase gas ($298.15\text{ K}$, $1\text{ bar}$) frente a valores experimentales de referencia (NIST Chemistry WebBook / Benson 1976 / `Maindb6.HSC`):

| Caso | Compuesto | SMILES | $\Delta_f H^\circ_{298}$ Ref (kcal/mol) | $S^\circ_{298}$ Ref (cal/mol·K) | Tolerancia |
|---|---|---|---|---|---|
| B1 | n-Butano (`C4H10`) | `CCCC` | `-30.0` | `74.1` | $|\Delta H| \le 1.5$, $|\Delta S| \le 1.5$ |
| B2 | Isobutano (`C4H10`) | `CC(C)C` | `-32.1` | `70.4` | $|\Delta H| \le 1.5$, $|\Delta S| \le 1.5$ |
| B3 | Benceno (`C6H6`) | `c1ccccc1` | `+19.8` | `64.3` | $|\Delta H| \le 1.5$, $|\Delta S| \le 1.5$ |
| B4 | Tolueno (`C7H8`) | `Cc1ccccc1` | `+12.0` | `76.6` | $|\Delta H| \le 1.5$, $|\Delta S| \le 2.0$ |
| B5 | Etanol (`C2H6O`) | `CCO` | `-56.2` | `67.5` | $|\Delta H| \le 1.5$, $|\Delta S| \le 2.0$ |
| B6 | Ácido Acético (`C2H4O2`) | `CC(=O)O` | `-103.3` | `67.5` | $|\Delta H| \le 2.0$, $|\Delta S| \le 2.5$ |
| B7 | Ciclohexano (`C6H12`) | `C1CCCCC1` | `-29.5` | `71.3` | $|\Delta H| \le 1.5$, $|\Delta S| \le 2.0$ |
| B8 | Ajuste Kelley $C_p(T)$ | Todos | — | — | Error máx. ajuste Kelley $< 2.5\text{ J/(mol}\cdot\text{K)}$ en $300\text{–}1500\text{ K}$ |
