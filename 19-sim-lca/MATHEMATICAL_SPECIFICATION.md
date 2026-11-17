# Especificación Matemática — Módulo 19: Sim LCA (`HSC - Sim LCA`)

Las reglas de conversión física, normalización LCI, prevención de doble conteo e integración hacia evaluación de impacto (LCIA) proceden directamente del capítulo 49 de [49-sim-lca.md](file:///home/nando/Documentos/Docs%20HSC/49-sim-lca/49-sim-lca.md) y de la verificación numérica de sus casos oficiales **Zn TSL Fuming** (`49.3`, `Figs. 9–17`) e **Iron Process** (`49.4`, `Figs. 15, 22–40`). En aquellos componentes cuyo motor matemático interno reside fuera de HSC (por ejemplo, el cálculo LCIA dentro de `openLCA`), se especifica la **formulación matricial ISO-LCA clean-room equivalente**.

---

## 1. Conjunto de Frontera LCI (Modelo de Caja Negra)

Dado el grafo dirigido de corrientes del diagrama $G = (V, E \cup E_{\text{in}} \cup E_{\text{out}})$:

- $\mathcal{S}_{\text{in}} \subset E_{\text{in}}$: conjunto de corrientes **azules** (sin unidad de origen dentro del diagrama).
- $\mathcal{S}_{\text{out}} \subset E_{\text{out}}$: conjunto de corrientes **rojas** (sin unidad de destino dentro del diagrama).
- $\mathcal{S}_{\text{int}} = E$: corrientes internas **negras** (origen y destino dentro del diagrama), **estrictamente excluidas** del LCI:

$$\mathcal{B} = \left(\mathcal{S}_{\text{in}} \cup \mathcal{M}_{\text{in}}\right) \sqcup \left(\mathcal{S}_{\text{out}} \cup \mathcal{M}_{\text{out}}\right) \qquad (1)$$

donde $\mathcal{M}_{\text{in}}$ y $\mathcal{M}_{\text{out}}$ representan las corrientes manuales introducidas por el usuario para la evaluación ambiental (p.ej. electricidad de edificios o `Extra electricity`). En el cálculo, ninguna arista $e \in \mathcal{S}_{\text{int}}$ aporta términos a la matriz de intercambio del proceso caja negra.

---

## 2. Conversión Física Base: Masa, Energía y Exergía

### 2.1. Modalidad Clásica (`49.3`, Horizonte Base $\Delta t = 1\,\text{h}$)

Para cada corriente $s \in \mathcal{S}_{\text{in}} \cup \mathcal{S}_{\text{out}}$ proveniente de HSC Sim con tasa másica $\dot{m}_s$ en $\text{t/h}$, tasa térmica/energética $\dot{E}_s$ en $\text{kW}$ y flujo exergético $\dot{E}x_s$ en $\text{kW}$:

- **Modo estándar (`Use Exergy = false`):**
  $$a_s^{\text{std}} = \begin{cases} 1000\,\dot{m}_s \;[\text{kg}], & \text{si la corriente es másica/material} \\ 1\,\dot{E}_s \;[\text{kWh}], & \text{si la corriente es flujo puro de energía/calor/potencia} \end{cases} \qquad (2)$$
  Verificado en `Zn TSL Fuming` (`Figs. 9–10`): `Residues = 100 t/h -> 100000.00 kg`; `Final ZnO Dust = 8.01299 t/h -> 8012.99 kg`; `Power = 63020.50 kW -> 63020.50 kWh`.

- **Modo exergético (`Use Exergy = true`):**
  $$a_s^{\text{ex}} = \dot{E}x_s \cdot 1\,\text{h} \;[\text{kWh}] \qquad (3)$$
  Cada corriente activada con `Use Exergy` sustituye su cantidad másica o calorífica por su equivalente exergético en $\text{kWh}$.

### 2.2. Modalidad Beta IPC (`49.4`, Conversión hacia `openLCA`)

En la hoja de trabajo (`Inputs`, `Outputs`), las tasas se mantienen en unidades instantáneas de simulación ($\text{t/h}$, $\text{kW}$ o $\text{MJ}$). Al generar los intercambios dentro de `openLCA` con horizonte de 1 hora (`Figs. 38–39`), aplican los factores de escala exactos:

$$a_s^{\text{IPC}} = \begin{cases} 1000\,\dot{m}_s \;[\text{kg}], & \text{si } u_s = \text{t/h} \\ 3.6\,\dot{E}_s \;[\text{MJ}], & \text{si } u_s = \text{kW} \quad \left(\text{pues } 1\,\text{kW}\cdot 1\,\text{h} = 3600\,\text{kJ} = 3.6\,\text{MJ}\right) \\ 1\,E_s \;[\text{MJ}], & \text{si } u_s = \text{MJ} \end{cases} \qquad (4)$$

Verificado exactamente en las capturas oficiales de `Iron Process`:
- `Pig Iron`: $\dot{m}_{\text{ref}} = 138.217\,\text{t/h} \implies 1.38217 \times 10^5\,\text{kg}$ (`Fig. 38`).
- `Extra electricity`: $\dot{E} = 10000.00\,\text{kW} \implies 10000 \times 3.6 = 3.60000 \times 10^4\,\text{MJ}$ (`Fig. 39`).

---

## 3. Reglas de Indicadores, Comodines y No Doble Conteo

### 3.1. Sintaxis de Búsqueda por Comodines (`49.3.4`)

Sea un patrón de indicador $p$ evaluado contra el nombre químico/fase de cada especie $i$ perteneciente a una corriente de frontera $s$ (filtrada como salida `"To Nature"` cuando corresponde, o en las corrientes habilitadas):

- `*` coincide con cero o más caracteres cualesquiera (equivalente regex `.*`).
- `?` coincide con exactamente un carácter (equivalente regex `.`).
- `#` coincide con exactamente un dígito decimal `0..9` (equivalente regex `[0-9]`).

La cantidad agregada del indicador $p$ en el lote clásico viene dada por:

$$A_p = \sum_{s \in \mathcal{S}_{\text{out}}} \sum_{\substack{i \in \text{Species}(s) \\ i \models p}} \mathbf{1}_{\text{Counted}(s, i)} \cdot \tilde{m}_{s, i} \qquad (5)$$

donde $\tilde{m}_{s,i} = 1000\,\dot{m}_{s,i}$ ($\text{kg}$) y el indicador $\mathbf{1}_{\text{Counted}(s,i)} \in \{0, 1\}$ impone la **restricción estricta anti doble conteo**:
- Si un usuario desmarca una fila fuente dentro de un indicador o marca explícitamente las entradas manuales/corrientes mapeadas, ninguna pareja $(s, i)$ puede contribuir simultáneamente a dos indicadores activos ni sumarse dos veces cuando la corriente padre completa ya fue exportada con masa bruta equivalente (salvo separación explícita en corrientes elementales de emisión como `CO2(g)` dentro de `Off-gas to further treatment`).

### 3.2. Desglose Bidireccional en Modo Beta (`49.4`, `Figs. 15–16`)

Para una corriente padre $s$ compuesta por fases y especies $\{(k, \dot{m}_{s,k})\}$ con $\sum_k \dot{m}_{s,k} = \dot{m}_s$:
- **`Convert Stream to Indicators`** (habilitado únicamente si `FlowMapping(s) = ∅`): desactiva la exportación del nodo padre $s$ y promueve sus especies hijas $k$ como filas mapeables independientes con su respectivo `LCA Flowtype`.
- **`Convert Indicators back to Stream`**: solo procede si ninguna especie hija mantiene un mapeo incompatible o tras limpiar mapeos previos; restaura la fila única agregada $s$.

---

## 4. Normalización respecto al Producto Principal / Flujo de Referencia

Debe existir **exactamente una** corriente o flujo marcado como referencia (`Main Product = true` en `49.3`, o `LCA Flowtype = Reference Flow (Product)` en `49.4`) dentro de la evaluación:

$$\sum_{b \in \mathcal{B}} \mathbf{1}_{\text{Ref}}(b) = 1 \qquad (6)$$

### 4.1. Normalización Clásica a `1 kg` de Producto Principal (`49.3.6`)

Sea $Q_{\text{ref}} > 0$ la cantidad sin normalizar (en $\text{kg}$) del flujo `Main Product`. Para cada flujo distinto agrupado por la clave compuesta $g = (\text{Direction}, \text{LCA Equivalent}, \text{LCA Group})$ con $\text{LCA Equivalent} \neq \text{No Mapping}$:

$$\hat{a}_g = \frac{1}{Q_{\text{ref}}} \sum_{b \in \mathcal{B}:\; \text{key}(b) = g} a_b \qquad (7)$$

- Todas las corrientes con idéntica clave $g$ se fusionan en una sola fila del bloque `Normalized Flows`.
- Toda corriente con `LCA Equivalent = No Mapping` queda excluida del inventario normalizado y de los archivos XML/JSON-LD exportados.
- Caso verificado `Zn TSL Fuming` ($Q_{\text{ref}} = 8012.99\,\text{kg}$ de `Zinc oxide`):
  $$\hat{a}_{\text{Air}} = \frac{34590.74 + 1.29}{8012.99} = 4.3170\,\text{kg/kg ZnO} \approx 4.32\,\text{kg}$$
  $$\hat{a}_{\text{Waste heat}} = \frac{6302.05 + 3327.22 + 441.09}{8012.99} = 1.2568\,\text{kWh/kg ZnO} \approx 1.26\,\text{kWh}$$
  $$\hat{a}_{\text{Zinc oxide}} = \frac{8012.99}{8012.99} = 1.00\,\text{kg}$$

### 4.2. Normalización en Modo Beta por Flujo de Referencia (`49.4`, `Fig. 34`)

En `Results`, cada corriente e indicador mapeado reporta su tasa normalizada relativa al caudal del flujo de referencia $\dot{m}_{\text{ref}}$ (en `t/h` para el ejemplo `Pig Iron = 138.217 t/h`, mostrado como `138,22 t/h`):

$$\tilde{q}_b = \frac{a_b}{\dot{m}_{\text{ref}}} \quad \left[\frac{\text{unidad}(b)}{\text{t/h de } s_{\text{ref}}}\right] \qquad (8)$$

Verificación en `Iron Process` (`Fig. 34`):
- `Coke`: $50.00 / 138.217 = 0.3617\,\text{t/h}$ (mostrado `0,36 t/h`).
- `CO2(g)`: $101.34 / 138.217 = 0.7332\,\text{t/h}$ (mostrado `0,73 t/h`).
- `Extra electricity`: $10000.00 / 138.217 = 72.3499\,\text{kW}$ (mostrado `72,35 kW`).
- `Pig Iron` (referencia): $138.217 / 138.217 = 1.00\,\text{t/h}$.

---

## 5. Formulación Matricial LCIA *(Clean-Room ISO 14044 y Modo IPC)*

Para un sistema de producto ensamblado con matriz tecnológica $\mathbf{A} \in \mathbb{R}^{n \times n}$, vector de demanda final $\mathbf{f} \in \mathbb{R}^n$ (normalizado a $1\,\text{t/h} = 1000\,\text{kg}$ del flujo de referencia en `Main Process`), matriz de intervenciones elementales $\mathbf{B} \in \mathbb{R}^{m \times n}$ y matriz de factores de caracterización $\mathbf{Q} \in \mathbb{R}^{K \times m}$ correspondiente al método seleccionado (`Calculation Method`, con restricción de que todas las categorías activas pertenezcan al mismo método):

$$\mathbf{s} = \mathbf{A}^{-1} \mathbf{f}, \qquad \mathbf{g} = \mathbf{B}\mathbf{s}, \qquad \mathbf{h} = \mathbf{Q}\mathbf{g} \qquad (9)$$

La atribución aditiva por unidad de proceso $u \in \mathcal{U} \cup \{\text{Manual Streams}\}$ y por corriente mapeada $b$ expuesta en la hoja `Results` cumple la descomposición exacta de contribuciones estructurales:

$$h_c^{\text{Total}} = \sum_{u} h_{c,u} = \sum_{u} \left( \sum_{b \in \mathcal{S}_{\text{in},u}^{\text{mapped}}} h_{c,u,b} + \sum_{b \in \mathcal{S}_{\text{out},u}^{\text{mapped}}} h_{c,u,b} \right) \qquad (10)$$

donde:
- Para un **flujo elemental directo** (p.ej. emisión de `CO2(g)` mapeada a `Carbon dioxide, fossil` sin proveedor):
  $$h_{c,u,b}^{\text{elem}} = Q_{c, f(b)} \cdot \tilde{m}_{b,\text{kg}} \qquad (11)$$
  Comprobado numéricamente en `Fig. 34` (`ReCiPe Midpoint (H) - climate change - GWP100`, donde $Q_{\text{GWP100}, \text{CO2 fossil}} = 1.0\,\text{kg CO}_2\text{-eq/kg}$):
  $$h_{\text{CO2(g)}} = 1.0 \times \frac{101340\,\text{kg}}{138.217\,\text{t/h}} \approx 733.22\,\text{kg CO}_2\text{-Eq per t/h of Pig Iron}$$
  y para gases sin potencial GWP100 en ese método (`CO(g)`, `SO2(g)`): $h = 0.00$.
- Para un **flujo de producto o residuo** (`Product Flow` / `Waste Flow`) enlazado a un proveedor $p$ de la base de datos:
  $$h_{c,u,b}^{\text{upstream}} = \tilde{q}_{b,\text{openLCA}} \cdot u_c(p) \qquad (12)$$
  donde $u_c(p)$ es la intensidad de impacto unitaria acumulada *cradle-to-gate* del proceso proveedor seleccionado (p.ej. `market for petroleum coke`, `GLO`: $\tilde{q} \approx 0.3617\,\text{t/h} \to 136.60\,\text{kg CO}_2\text{-eq}$; `market group for electricity, high voltage`, `GLO`: $72.35\,\text{kWh} \to 13464.22\,\text{kg CO}_2\text{-eq}$).

Cuando cambia un caudal en el simulador **sin** alterar mapeos ni categorías, el recálculo puede actualizar proporcionalmente las contribuciones locales de flujos normalizados sin requerir reconstrucción de la matriz tecnológica $\mathbf{A}$ mientras el factor unitario $u_c(p)$ permanezca invariante y los pesos normalizados se reescalen al nuevo $\dot{m}_{\text{ref}}$ (*nota*: el manual `49.4` confirma explícitamente la actualización en vivo de resultados al variar valores del diagrama incluso con el servidor gRPC cerrado).

---

## 6. Reglas de Validación Estructural e Integridad

1. **Compatibilidad dimensional:** Ningún mapeo es válido si la magnitud física del flujo HSC (`Mass` en `t/h`/`kg` vs. `Energy` en `kW`/`MJ`/`kWh`) es incompatible con la propiedad de flujo de referencia en la base LCA seleccionada.
2. **Unicidad del producto de referencia:** Exactamente una fila en `Inputs`/`Outputs` puede marcarse como `Main Product` (`49.3`) o `Reference Flow (Product)` (`49.4`).
3. **Filtro jerárquico de proveedores (`> 15`):** Sea $N_{\text{prov}}(f)$ el número de procesos productores del flujo $f$ en la base activa. Si $N_{\text{prov}}(f) > 15$, la selección directa en el desplegable `Provider` se bloquea hasta fijar una entrada no nula en `Location`, reduciendo el conjunto candidato a $\{p \in \text{Providers}(f) \mid \text{Loc}(p) = L_{\text{sel}}\}$.
4. **Restricciones de nomenclatura de hojas personalizadas (`49.4`):** Un nombre de hoja personalizada $S$ debe satisfacer:
   $$S \notin \mathcal{S}_{\text{existing}} \quad \land \quad \neg \text{StartsWith}(S, \{\text{"Saved"}, \text{"Norm"}, \text{"Ext"}\}) \qquad (13)$$
5. **Unicidad del método LCIA (`49.4`):** Todas las categorías cargadas en la hoja `Results` deben pertenecer a un mismo `Impact Method` padre; agregar una categoría de otro método debe exigir limpieza previa o rechazo explícito.
