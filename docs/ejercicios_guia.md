# Guía Práctica de Ejercicios - CMA Chemistry

Esta guía proporciona una serie de ejercicios prácticos diseñados para ser resueltos con los distintos módulos de **CMA Chemistry**. Estos problemas abarcan desde termodinámica básica hasta simulación de diagramas de estabilidad y balances exergéticos.

---

## 1. Módulo de Simulación de Procesos (Balances de Masa y Energía)
> [!TIP]
> Útil para resolver flujogramas complejos de plantas químicas y metalúrgicas usando conservación de masa y energía.

**Ejercicio:**
Se desea modelar un **Reformador de Metano con Vapor (SMR)**. 
- **Corriente de entrada 1:** 100 kmol/h de Metano (CH₄) a 298 K.
- **Corriente de entrada 2:** 250 kmol/h de Vapor de Agua (H₂O) a 450 K.
- Configura un reactor de conversión y establece una conversión del 85% para el metano.
- **Objetivo:** Calcular la entalpía total requerida (Calor, $Q$) para mantener el reactor isotérmico a 1100 K y determinar la composición de los gases de síntesis producidos (H₂ y CO). Verifica el balance de masa global en la tabla de Resultados.

---

## 2. Módulo Calculadora Termodinámica (Propiedades H, S, Cp, G)
**Ejercicio:**
El proceso de tostación de la Pirita (FeS₂) es fundamental en la metalurgia extractiva. 
- Introduce la especie `FeS2`.
- **Objetivo:** Grafica la Capacidad Calorífica ($C_p$) y la Energía Libre de Gibbs ($G$) de la pirita en un rango de temperatura desde 298 K hasta 1500 K. Identifica gráficamente si ocurre alguna transición de fase térmica y anota los valores de entalpía a 1000 K para ser usados en tus balances manuales.

---

## 3. Módulo de Minimización de Gibbs (Equilibrio)
> [!IMPORTANT]
> A diferencia de un reactor por conversión, este módulo predice los productos espontáneos buscando el mínimo nivel de energía libre.

**Ejercicio:**
Quema de carbón en un ambiente deficiente de oxígeno.
- **Reactivos:** 1 kmol de Carbono (`C`) y 0.5 kmol de Oxígeno (`O2`) a temperatura ambiente.
- **Fase Gas (posibles productos a seleccionar):** `CO2`, `CO`, `O2`.
- **Fase Sólida:** `C`.
- **Objetivo:** Utiliza el minimizador a temperaturas variando entre 500 K y 2000 K para observar la competencia entre la formación de monóxido de carbono (CO) y dióxido de carbono (CO₂). Analiza a qué temperatura el CO se vuelve la especie dominante.

---

## 4. Módulo de Diagramas Eh-pH (Pourbaix)
> [!NOTE]
> Crucial en hidrometalurgia para determinar regiones de lixiviación, pasivación y corrosión.

**Ejercicio:**
Estudio de lixiviación del Cobre.
- **Sistema Principal:** Cobre (`Cu`).
- **Límites:** pH de 0 a 14 y Potencial (Eh) de -1.0 V a 1.5 V.
- **Concentración:** Configura la molalidad de la especie principal en $1 \times 10^{-4}$ mol/kg.
- **Objetivo:** Genera el diagrama y determina el rango de pH y potencial oxidante (Eh) necesario para disolver el Cobre sólido (`Cu`) y pasarlo a su fase soluble sulfato/iónica (`Cu^{+2}`).

---

## 5. Módulo Análisis Exergético
**Ejercicio:**
Eficiencia termodinámica de un Intercambiador de Calor.
- Modela un intercambiador simple en el lienzo.
- **Lado Caliente:** Entran 50 kg/s de agua a 90°C y salen a 60°C.
- **Lado Frío:** Entran 40 kg/s de agua a 20°C y se calientan absorbiendo la energía del lado caliente.
- **Objetivo:** Activa el panel de Resultados de Exergía. Calcula cuánta energía "útil" (exergía) se ha destruido ($Ex_{loss}$) por culpa de la irreversibilidad en la transferencia de calor (considerando la temperatura ambiente como $T_0 = 298.15 K$). 

---

## 6. Módulo de Pérdida de Calor (Heat Loss)
**Ejercicio:**
Aislamiento de tuberías.
- Selecciona la geometría **Cilindro (Tubería)**.
- **Condiciones:** Fluido interno a 300°C. Diámetro interno de 0.1 m.
- **Aislante:** Agrega una capa de 0.05 m de espesor de *Lana de roca* (conductividad térmica de ~0.04 W/mK).
- **Ambiente:** Aire exterior a 25°C con coeficiente convectivo de $10 \: W/m^2K$.
- **Objetivo:** Determina el flujo de calor disipado ($Q$) por metro lineal de tubería. Reduce el espesor del aislante a la mitad y analiza porcentualmente cuánto aumenta la pérdida térmica.

---

## 7. Módulo de Estimación (Joback-Reid / Benson)
**Ejercicio:**
Cuando una molécula orgánica compleja no se encuentra en la base de datos, debes predecir sus propiedades.
- **Compuesto de prueba:** Etanol ($C_2H_6O$).
- **Método Joback:** Dibuja o ingresa los grupos estructurales para el etanol (e.g. un `-CH3`, un `-CH2-`, y un `-OH`).
- **Objetivo:** Estimar la temperatura de ebullición ($T_b$), entalpía de formación ($\Delta H_f$) y capacidad calorífica del líquido. Compara el resultado generado con los datos experimentales típicos del etanol (Tb ≈ 78 °C) para medir la precisión del método de contribución de grupos.

---

## 8. Análisis de Ciclo de Vida (LCA)
**Ejercicio:**
Huella de Carbono del proceso.
- En tu simulación completada de un horno rotatorio para cemento, abre el panel LCA.
- **Asignación de impactos:** Asigna el factor de GWP (Global Warming Potential) a la corriente de emisión que contiene CO₂ y a los combustibles fósiles (carbón) ingresados en función de los factores del Ecoinvent.
- **Objetivo:** Estima los kg de CO₂-eq emitidos por cada Tonelada de Clinker (producto útil) generado.

---

## 9. Módulo Stock de Materiales Map-GPS (Geo-Minerológico)
**Ejercicio:**
Rastreo logístico de mineral de hierro.
- Carga un dataset CSV con coordenadas GPS (latitud, longitud) y leyes (grados de concentración) de un pit minero o pilas de lixiviación.
- **Objetivo:** Usa la herramienta para trazar en el mapa la distribución de calidades del mineral (concentraciones Altas vs Bajas coloreadas en el mapa) y proyectar de qué sector provendrán los próximos alimentadores a la planta.
