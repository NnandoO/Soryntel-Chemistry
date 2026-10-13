#include "pourbaix.hpp"
#include "../nucleo/tabla_periodica.hpp"
#include <cmath>
#include <algorithm>
#include <iostream>

namespace cma { namespace motores {

// ═══════════════════════════════════════════════════════════════════════════
// Helpers privados
// ═══════════════════════════════════════════════════════════════════════════

namespace {

// Limpia la fórmula eliminando tags de estado: (g), (l), (s), (a), (aq),
// (α), (+2a), (-a), etc. Devuelve la fórmula química pura.
std::string limpiar_formula(const std::string& f) {
    std::string out;
    size_t i = 0;
    while (i < f.size()) {
        if (f[i] == '(') {
            size_t close = f.find(')', i);
            if (close != std::string::npos) {
                std::string tag = f.substr(i + 1, close - i - 1);
                bool es_tag_estado = false;
                if (tag == "g" || tag == "l" || tag == "s" || tag == "a" ||
                    tag == "aq" || tag == "α" || tag == "β" || tag == "γ")
                    es_tag_estado = true;
                else if (!tag.empty()) {
                    // Tags que contienen +, -, o 'a' (cargas acuosas: +2a, -a, +a)
                    if (tag.find('a') != std::string::npos ||
                        tag.find('+') != std::string::npos ||
                        tag.find('-') != std::string::npos)
                        es_tag_estado = true;
                }
                if (es_tag_estado) {
                    i = close + 1;
                    continue;
                }
            }
        }
        out += f[i++];
    }
    return out;
}

// Extrae la carga de una fórmula con tag: Fe(+2a) -> +2, SO4(-2a) -> -2
int extraer_carga(const std::string& f) {
    // Buscar (+ o (- seguido de dígito
    size_t pos = f.find("(+");
    if (pos != std::string::npos && pos + 2 < f.size() && std::isdigit(f[pos + 2])) {
        int signo = 1;
        int val = 0;
        size_t j = pos + 2;
        while (j < f.size() && std::isdigit(f[j])) {
            val = val * 10 + (f[j] - '0');
            j++;
        }
        return signo * val;
    }
    pos = f.find("(-");
    if (pos != std::string::npos && pos + 2 < f.size() && std::isdigit(f[pos + 2])) {
        int val = 0;
        size_t j = pos + 2;
        while (j < f.size() && std::isdigit(f[j])) {
            val = val * 10 + (f[j] - '0');
            j++;
        }
        return -val;
    }
    return 0;
}

// Determina el estado (fase) a partir del tag en la fórmula
// 0: gas, 1: líquido, 2: sólido, 3: acuoso
int extraer_estado(const std::string& f) {
    if (f.find("(g)") != std::string::npos) return 0;
    if (f.find("(l)") != std::string::npos) return 1;
    if (f.find("(s)") != std::string::npos) return 2;
    // Tags acuosos: (a), (aq), (+2a), (-a), etc.
    if (f.find("(a)") != std::string::npos || f.find("(aq)") != std::string::npos)
        return 3;
    {
        size_t pos = f.find("(+");
        if (pos != std::string::npos) {
            size_t close = f.find(')', pos);
            if (close != std::string::npos) {
                std::string tag = f.substr(pos + 1, close - pos - 1);
                if (tag.find('a') != std::string::npos) return 3;
            }
        }
    }
    {
        size_t pos = f.find("(-");
        if (pos != std::string::npos) {
            size_t close = f.find(')', pos);
            if (close != std::string::npos) {
                std::string tag = f.substr(pos + 1, close - pos - 1);
                if (tag.find('a') != std::string::npos) return 3;
            }
        }
    }
    // Sin tag: usar fase_std de la especie
    return -1; // indeterminado
}

// Cuenta átomos de un elemento específico en una fórmula parseada
int contar_elemento(const std::vector<nucleo::TablaPeriodica::CompAtom>& atomos,
                    const std::string& simbolo) {
    for (auto& ca : atomos)
        if (ca.elem == simbolo) return ca.cuenta;
    return 0;
}

} // namespace anónimo

// ═══════════════════════════════════════════════════════════════════════════
// Setters
// ═══════════════════════════════════════════════════════════════════════════

void CalculadorPourbaix::establecer_elemento(const std::string& elemento) {
    elemento_ = elemento;
}

void CalculadorPourbaix::establecer_temperatura(double T_K) {
    T_K_ = T_K;
}

void CalculadorPourbaix::establecer_concentracion(double conc) {
    conc_ = conc;
}

// ═══════════════════════════════════════════════════════════════════════════
// buscar_especies: usa el parser de fórmulas real en vez de substring naïve
// ═══════════════════════════════════════════════════════════════════════════

void CalculadorPourbaix::buscar_especies() {
    especies_activas_.clear();
    if (elemento_.empty()) return;

    const auto& db = repository_;
    auto& tp = nucleo::TablaPeriodica::instancia();
    auto todas = db.todas_especies();

    const double R = nucleo::R_GAS;       // 8.314 J/(mol·K)
    const double RT = R * T_K_;            // J/mol
    const double RT_kJ = RT / 1000.0;      // kJ/mol

    for (const auto* sp : todas) {
        std::string f_clean = limpiar_formula(sp->formula);

        // Parsear la fórmula con el analizador de la tabla periódica
        auto atomos = tp.analizar_formula(f_clean);

        // Verificar si contiene el elemento principal
        int coef = contar_elemento(atomos, elemento_);
        if (coef <= 0) continue;

        EspeciePourbaix ep;
        ep.formula = sp->formula;
        ep.nombre = sp->nombre;
        ep.coef_elemento = coef;
        ep.carga = extraer_carga(sp->formula);

        // Determinar estado
        int est = extraer_estado(sp->formula);
        if (est == -1) {
            // Sin tag explícito: usar fase_std
            if (sp->fase_std == nucleo::Fase::Gas)      est = 0;
            else if (sp->fase_std == nucleo::Fase::Liquido) est = 1;
            else if (sp->fase_std == nucleo::Fase::Solido)  est = 2;
            else est = 3;
        }
        ep.estado = est;

        // ΔGf° = ΔHf° - T·ΔSf° (kJ/mol)
        // Nota: h_formacion está en kJ/mol, s_formacion en J/(mol·K)
        ep.dG_f = sp->h_formacion - T_K_ * (sp->s_formacion / 1000.0);

        // Contar H y O normalizados por 1 átomo del elemento principal
        int n_H_total = contar_elemento(atomos, "H");
        int n_O_total = contar_elemento(atomos, "O");
        ep.n_H = (double)n_H_total / coef;
        ep.n_O = (double)n_O_total / coef;

        // Corrección por concentración para especies acuosas
        if (est == 3) {
            double c = std::max(conc_ / coef, 1e-30);
            ep.dG_f += RT_kJ * std::log(c);
        }

        especies_activas_.push_back(ep);
    }

    // Ordenar por ΔGf° para reproducibilidad
    std::sort(especies_activas_.begin(), especies_activas_.end(),
              [](const EspeciePourbaix& a, const EspeciePourbaix& b) {
                  return a.dG_f < b.dG_f;
              });
}

// ═══════════════════════════════════════════════════════════════════════════
// generar_lineas: calcula líneas de equilibrio entre pares de especies
//
// Reacción general entre especie A (oxidada) y especie B (reducida),
// ambas referidas a 1 átomo del elemento principal:
//
//   A + (n_H) H+ + n_e e-  ↔  B + (n_O) H2O
//
//   ΔG_rxn = ΔGf(B) - ΔGf(A) - n_O · ΔGf(H2O,l)
//
//   Eh = -ΔG_rxn / (n_e · F)          (si n_e > 0)
//
//   Eh = E° - (RT / n_e · F) · ln(Q)
//   Para reacciones con H+ y sin e- (ácido-base puro): línea vertical
// ═══════════════════════════════════════════════════════════════════════════

void CalculadorPourbaix::generar_lineas() {
    lineas_.clear();

    const double R = nucleo::R_GAS;
    const double F = nucleo::FARADAY;
    const double RT = R * T_K_;

    // ΔGf° de H2O(l) a T_K_: aproximar usando la especie de la DB
    dGf_H2O_l_ = -237.13; // valor estándar a 298.15 K
    {
        auto* h2o_l = repository_.especie_por_nombre("Agua (líquida)");
        if (h2o_l) {
            dGf_H2O_l_ = h2o_l->h_formacion - T_K_ * (h2o_l->s_formacion / 1000.0);
        }
    }

    // ── Líneas de estabilidad del agua ──
    // O2 + 4H+ + 4e- → 2H2O,  E° = 1.229 V
    // 2H+ + 2e- → H2,        E° = 0.000 V
    // Pendiente Nernst: dEh/dpH = -RT·ln(10) / (n_e · F)
    {
        double slope_ox = -(RT * std::log(10.0)) / (4.0 * F);
        double slope_red = -(RT * std::log(10.0)) / (2.0 * F);

        // Corrección de E° por T: E°(T) ≈ E°(298) + (ΔS/nF)·(T-298)
        // Para agua usamos aproximación lineal
        double dT = T_K_ - 298.15;
        double E0_ox  = 1.229 - 0.00085 * dT;   // ~-0.85 mV/K
        double E0_red = 0.000;

        LineaPourbaix agua_ox  = {"O2(g)", "H2O", slope_ox,  E0_ox,  false, 0.0, true};
        LineaPourbaix agua_red = {"H+",    "H2(g)", slope_red, E0_red, false, 0.0, true};
        lineas_.push_back(agua_ox);
        lineas_.push_back(agua_red);
    }

    // ── Líneas entre especies del elemento principal ──
    size_t NS = especies_activas_.size();
    if (NS < 2) return;

    for (size_t i = 0; i < NS; ++i) {
        for (size_t j = i + 1; j < NS; ++j) {
            const auto& A = especies_activas_[i]; // especie 1
            const auto& B = especies_activas_[j]; // especie 2

            // Diferencia en composición (referida a 1 átomo del elemento)
            double dH = A.n_H - B.n_H;  // H+ consumidos/producidos
            double dO = A.n_O - B.n_O;  // H2O consumidos/producidos
            double dZ = A.carga - B.carga; // electrones transferidos

            // Balancear con H2O y H+:
            //   A ↔ B + dO·H2O - dH·H+ + dZ·e-
            // Reacción: A + (dH - 2·dO)·H+ + dZ·e- → B + dO·H2O
            //   (cada O requiere 2H para formar H2O)
            double n_H_plus = dH - 2.0 * dO;  // moles de H+
            double n_e = dZ;                   // moles de electrones

            // Debe haber al menos un intercambio de H+ o e-
            if (std::abs(n_H_plus) < 1e-12 && std::abs(n_e) < 1e-12)
                continue;

            // ΔG de reacción: B - A - dO·H2O(l)
            double dG_rxn = B.dG_f - A.dG_f - dO * dGf_H2O_l_;

            LineaPourbaix linea;
            linea.especie_a = A.formula;
            linea.especie_b = B.formula;
            linea.es_agua = false;

            if (std::abs(n_e) < 1e-12) {
                // ── Reacción ácido-base pura (sin e-): línea vertical ──
                // ΔG_rxn + n_H_plus · RT·ln(10) · pH = 0
                // pH_eq = -ΔG_rxn·1000 / (n_H_plus · RT · ln(10))
                //   (ΔG_rxn en kJ, multiplicar por 1000 para J)
                if (std::abs(n_H_plus) < 1e-12) continue;

                double pH_eq = -(dG_rxn * 1000.0) / (n_H_plus * RT * std::log(10.0));

                if (pH_eq >= -2.0 && pH_eq <= 16.0) {
                    linea.vertical = true;
                    linea.x_val = pH_eq;
                    linea.m = 0.0;
                    linea.b = 0.0;
                    lineas_.push_back(linea);
                }
            } else {
                // ── Reacción redox (con o sin H+): línea inclinada ──
                // Eh = -ΔG_rxn·1000 / (n_e · F) - (n_H_plus · RT · ln(10)) / (n_e · F) · pH
                double E0 = -(dG_rxn * 1000.0) / (n_e * F);
                double slope = -(n_H_plus * RT * std::log(10.0)) / (n_e * F);

                // Filtrar líneas absurdas (fuera del rango típico)
                double Eh_at_0 = E0;
                double Eh_at_14 = E0 + slope * 14.0;
                if (Eh_at_0 > -3.0 && Eh_at_0 < 5.0 &&
                    Eh_at_14 > -3.0 && Eh_at_14 < 5.0) {
                    linea.vertical = false;
                    linea.m = slope;
                    linea.b = E0;
                    lineas_.push_back(linea);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// calcular_dominancia: grilla pH×Eh, en cada celda se evalúa el ΔG aparente
// de cada especie y se determina la dominante.
// ═══════════════════════════════════════════════════════════════════════════

void CalculadorPourbaix::calcular_dominancia() {
    const int NP = 80;   // resolución pH
    const int NE = 80;   // resolución Eh
    const double pH_lo = 0.0, pH_hi = 14.0;
    const double Eh_lo = -2.0, Eh_hi = 2.0;

    dominancia_.n_pH = NP;
    dominancia_.n_Eh = NE;
    dominancia_.pH_min = pH_lo;
    dominancia_.pH_max = pH_hi;
    dominancia_.Eh_min = Eh_lo;
    dominancia_.Eh_max = Eh_hi;
    dominancia_.grid.assign(NP * NE, -1);

    const double R = nucleo::R_GAS;
    const double F = nucleo::FARADAY;
    const double RT = R * T_K_;
    const double RT_kJ = RT / 1000.0;

    size_t NS = especies_activas_.size();
    if (NS == 0) return;

    // Agregar H2O(l) y H+ como referencia para el agua
    // (no es necesario para dominancia del elemento, pero ayuda a filtrar
    //  regiones fuera de estabilidad del agua)

    for (int ip = 0; ip < NP; ++ip) {
        double pH = pH_lo + (pH_hi - pH_lo) * (double)ip / (double)(NP - 1);
        double aH = std::pow(10.0, -pH);  // actividad H+

        for (int ie = 0; ie < NE; ++ie) {
            double Eh = Eh_lo + (Eh_hi - Eh_lo) * (double)ie / (double)(NE - 1);

            double best_dG = 1e30;
            int best_idx = -1;

            for (size_t k = 0; k < NS; ++k) {
                const auto& sp = especies_activas_[k];

                // ΔG aparente de formación de la especie referida a los
                // elementos en solución a este pH y Eh.
                //
                // Para una especie M_aO_bH_c con carga z:
                //   Reacción de formación desde iones:
                //     M + b·H2O → especie + (2b - c)·H+ + z·e-
                //   ΔG_app = ΔGf° + (2·n_O - n_H)·RT·ln(aH) + n_e·F·Eh
                //         + corrección de concentración (ya en dG_f)
                //
                // Simplificado: ΔG_app = ΔGf°(corregido) + n_O·ΔGf(H2O)
                //   + (2·n_O - n_H)·RT_kJ·ln(aH) + (carga - 0)·F·Eh/1000

                double n_O = sp.n_O;
                double n_H = sp.n_H;
                double z = sp.carga;

                // Energía de formación desde los elementos en solución:
                // Se ajusta por pH (actividad H+) y potencial Eh
                double term_H = (2.0 * n_O - n_H) * RT_kJ * std::log(std::max(aH, 1e-30));
                double term_Eh = -z * F * Eh / 1000.0;  // kJ/mol
                double term_H2O = n_O * dGf_H2O_l_;

                double dG_app = sp.dG_f + term_H2O + term_H + term_Eh;

                if (dG_app < best_dG) {
                    best_dG = dG_app;
                    best_idx = (int)k;
                }
            }

            dominancia_.grid[ip * NE + ie] = best_idx;
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// calcular: orquesta todo el cálculo
// ═══════════════════════════════════════════════════════════════════════════

bool CalculadorPourbaix::calcular() {
    buscar_especies();
    if (especies_activas_.empty()) return false;
    generar_lineas();
    calcular_dominancia();
    return true;
}

}} // namespace
