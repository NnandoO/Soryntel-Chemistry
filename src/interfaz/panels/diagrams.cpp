#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wformat-security"

#include "shared.hpp"
#include "diagrams/PropertyDiagrams.hpp"
#include "nucleo/tabla_periodica.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <string>
#include <vector>

namespace cma { namespace interfaz {

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Exergía
// ═══════════════════════════════════════════════════════════════════════════
void PanelExergia::renderizar() {
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(8.0f, 6.0f));
    ImGui::Text(tr("Análisis Exergético"));
    ImGui::Separator();
    ImGui::InputDouble("H entrada (kJ/mol)", &h_ent_);
    ImGui::InputDouble("H salida (kJ/mol)", &h_sal_);
    ImGui::InputDouble("H₀ ref. (kJ/mol)", &h0_);
    ImGui::InputDouble("T entrada (K)", &T_ent_);
    ImGui::InputDouble("T salida (K)", &T_sal_);
    ImGui::InputDouble("T₀ ambiente (K)", &T0_);
    ImGui::InputDouble("S entrada (kJ/(mol·K))", &s_ent_);
    ImGui::InputDouble("S salida (kJ/(mol·K))", &s_sal_);
    ImGui::InputDouble("S₀ ref. (kJ/(mol·K))", &s0_);
    ImGui::InputDouble("Trabajo útil (kJ/mol)", &W_util_);
    ImGui::InputDouble("Calor aportado (kJ/mol)", &Q_aporte_);
    if (ImGui::Button(tr("Calcular exergía"), ImVec2(-1, 30))) {
        resultado_ = motor_.analizar_proceso(h_ent_, h_sal_, h0_, T_ent_, T_sal_, T0_, s_ent_, s_sal_, s0_, W_util_, Q_aporte_);
        calculado_ = true;
    }
    if (calculado_) {
        ImGui::Separator();
        ImGui::Text(tr("Exergía física: %.4f kJ/mol"), resultado_.exergia_fisica);
        ImGui::Text(tr("Exergía química: %.4f kJ/mol"), resultado_.exergia_quimica);
        ImGui::Text(tr("Exergía destruida: %.4f kJ/mol"), resultado_.exergia_destruida);
        ImGui::Text(tr("Exergía total: %.4f kJ/mol"), resultado_.exergia_total);
        ImGui::Text(tr("Eficiencia exergética: %.2f%%"), resultado_.eficiencia_exergetica);

        double vals[3] = {resultado_.exergia_fisica, resultado_.exergia_quimica, resultado_.exergia_destruida};
        const char* labels[3] = {"Física", "Química", "Destruida"};
        if (ImPlot::BeginPlot("Desglose Exergético", ImVec2(-1, 200), ImPlotFlags_NoTitle)) {
            ImPlot::SetupAxes("Componente", "Exergía (kJ/mol)");
            double max_val = 1.0;
            for (int i = 0; i < 3; i++) if (vals[i] > max_val) max_val = vals[i];
            ImPlot::SetupAxesLimits(-0.5, 2.5, 0, max_val * 1.2);
            for (int i = 0; i < 3; i++) {
                ImPlot::PlotBars(labels[i], &vals[i], 1, 0.5, i - 0.25);
            }
            ImPlot::EndPlot();
        }
    }
    ImGui::PopStyleVar();
}

// ═══════════════════════════════════════════════════════════════════════════
// 19. H, S, C, and G Diagrams Module (HSC Thermochemical Data Diagrams — Dia)
// ═══════════════════════════════════════════════════════════════════════════
namespace {

constexpr int kNumFilasXData = 25;

// Cuadrícula de elementos en 9 columnas alfabéticas (Réplica de Fig. 1, 2, 3)
constexpr const char* kGridElementosDia[12][9] = {
    {"Ac", "Bi", "Cr", "Ga", "K",  "Nd", "Po", "Sb", "Th"},
    {"Ag", "Bk", "Cs", "Gd", "Kr", "Ne", "Pr", "Sc", "Ti"},
    {"Al", "Br", "Cu", "Ge", "La", "Ni", "Pt", "Se", "Tl"},
    {"Am", "C",  "Dy", "H",  "Li", "Np", "Pu", "Si", "Tm"},
    {"Ar", "Ca", "Er", "He", "Lu", "O",  "Ra", "Sm", "U" },
    {"As", "Cd", "Es", "Hf", "Mg", "Os", "Rb", "Sn", "V" },
    {"At", "Ce", "Eu", "Hg", "Mn", "P",  "Re", "Sr", "W" },
    {"Au", "Cf", "F",  "Ho", "Mo", "Pa", "Rh", "Ta", "Xe"},
    {"B",  "Cl", "Fe", "I",  "N",  "Pb", "Rn", "Tb", "Y" },
    {"Ba", "Cm", "Fm", "In", "Na", "Pd", "Ru", "Tc", "Yb"},
    {"Be", "Co", "Fr", "Ir", "Nb", "Pm", "S",  "Te", "Zn"},
    {"",   "",   "",   "",   "",   "",   "",   "",   "Zr"}
};

const ImVec4 kColoresCurvasDia[] = {
    ImVec4(0.32f, 0.52f, 0.22f, 1.0f), // Verde oliva oscuro (Cu2O / Fe3O4)
    ImVec4(0.12f, 0.45f, 0.58f, 1.0f), // Azul petróleo (FeO)
    ImVec4(0.38f, 0.40f, 0.42f, 1.0f), // Gris pizarra (Fe2O3)
    ImVec4(0.45f, 0.50f, 0.28f, 1.0f), // Verde musgo (ZnO)
    ImVec4(0.78f, 0.52f, 0.16f, 1.0f), // Ocre / Dorado (Cr2O3 / CoO)
    ImVec4(0.14f, 0.28f, 0.42f, 1.0f), // Azul marino (MgO)
    ImVec4(0.20f, 0.48f, 0.78f, 1.0f), // Azul brillante (CO(g))
    ImVec4(0.10f, 0.62f, 0.68f, 1.0f), // Cian oscuro (CO2(g))
    ImVec4(0.12f, 0.56f, 0.48f, 1.0f), // Verde esmeralda (H2O(g))
    ImVec4(0.68f, 0.24f, 0.28f, 1.0f), // Rojo óxido
    ImVec4(0.48f, 0.30f, 0.64f, 1.0f), // Púrpura
    ImVec4(0.60f, 0.42f, 0.24f, 1.0f)  // Bronce
};
constexpr size_t kNumColoresCurvasDia = sizeof(kColoresCurvasDia) / sizeof(kColoresCurvasDia[0]);

struct FilaXDataDia {
    char formula[64] = "";
    double coef = 1.0;
    double tmin_C = 25.0;
    double tmax_C = 3726.85;
    double t1_C = 0.0;
    double t2_C = 25.0;
    int db_no = 0; // 0 = Auto, 1 = Own DB, 2 = Main DB
    bool visible = true;
};

struct EstadoModuloDia {
    bool inicializado = false;
    FilaXDataDia filas[kNumFilasXData];
    int fila_activa = 0;
    char active_species_buf[64] = "Cu2O";
    int hoja_activa = 0; // 0 = X-data, 1 = Y-data, 2 = Intersections & Nomograph

    // 9 tipos de diagrama del manual HSC (Sección 19):
    // 0: H Enthalpy (total)
    // 1: H Heat Content
    // 2: S Entropy
    // 3: Cp Heat Capacity
    // 4: G Gibbs Energy
    // 5: Delta H
    // 6: Delta S
    // 7: Delta G (Ellingham)
    // 8: Ex Exergy (total)
    int idx_tipo_diagrama = 7; // Por defecto Delta G (Ellingham) (Fig. 2–4)

    // X-axis Range (Temperature)
    double x_min = 0.0;
    double x_max = 2000.0;
    double x_step = 50.0;
    bool temp_en_celsius = true;

    // Y-axis Range
    double y_min = -2000.0;
    double y_max = 0.0;
    bool y_auto = true;
    bool energia_en_kcal = false; // false = kJ/mol, true = kcal/mol
    bool por_kg = false;          // Checkbox "kg"

    // Balance Element Amount
    std::string elemento_balance = "O";
    double cantidad_balance = 2.0;

    // Opciones visuales del gráfico
    bool mostrar_etiquetas_curvas = true;
    bool mostrar_leyenda_interna = true;
    bool mostrar_transiciones = true;
    bool mostrar_iso_po2 = false;
    bool mostrar_iso_co_co2 = false;
    bool mostrar_iso_h2_h2o = false;
    bool reset_ejes_plot = true;

    // Resultados calculados
    std::vector<domain::CurvaDiagramaTermo> curvas;
    std::vector<domain::InterseccionEllingham> intersecciones;
    std::vector<domain::LineaIsoPotencial> iso_pO2;
    std::vector<domain::LineaIsoPotencial> iso_CO_CO2;
    std::vector<domain::LineaIsoPotencial> iso_H2_H2O;
    std::string estado_msg;

    // Sub-pestaña Reaction Equations & pO2 Equilibrium (Fig. 5, Fig. 6, Fig. 7)
    int preset_rxn_idx = 0;
    char rxn_eq_buf[128] = "2Cu2O + 2Fe = 4Cu + 2FeO";
    double rxn_t_from_C = 0.0;
    double rxn_t_to_C = 1000.0;
    double rxn_t_step_C = 100.0;
    int rxn_y_axis_mode = 0; // 0 = ΔG (kJ) (Fig. 5), 1 = K = p(O2(g)) log scale (Fig. 6 & Fig. 7)
    bool rxn_comparar_zn_ebullicion = false; // Para Fig. 7: superponer 2ZnO = 2Zn + O2(g) y 2ZnO = 2Zn(g) + O2(g)
    domain::CurvaDiagramaTermo curva_rxn_principal;
    domain::CurvaDiagramaTermo curva_rxn_secundaria;
};

EstadoModuloDia& estado_dia() {
    static EstadoModuloDia st;
    return st;
}

domain::PropertyDiagramType mapear_tipo_diagrama(int idx_ui) {
    switch (idx_ui) {
    case 0: return domain::PropertyDiagramType::Enthalpy_H;
    case 1: return domain::PropertyDiagramType::HeatContent_dH298;
    case 2: return domain::PropertyDiagramType::Entropy_S;
    case 3: return domain::PropertyDiagramType::HeatCapacity_Cp;
    case 4: return domain::PropertyDiagramType::Gibbs_G;
    case 5: return domain::PropertyDiagramType::DeltaH;
    case 6: return domain::PropertyDiagramType::DeltaS;
    case 7: return domain::PropertyDiagramType::DeltaG_Ellingham;
    case 8: return domain::PropertyDiagramType::Exergy_Ex;
    default: return domain::PropertyDiagramType::DeltaG_Ellingham;
    }
}

const char* titulo_tipo_diagrama(int idx_ui) {
    static const char* kTitulos[] = {
        "H Enthalpy (total)",
        "H Heat Content",
        "S Entropy",
        "Cp Heat Capacity",
        "G Gibbs Energy",
        "Delta H",
        "Delta S",
        "Delta G (Ellingham)",
        "Ex Exergy (total)"
    };
    if (idx_ui < 0 || idx_ui >= 9) return "Delta G (Ellingham)";
    return kTitulos[idx_ui];
}

std::string etiqueta_eje_y_dia(int idx_ui, bool en_kcal, bool por_kg) {
    const bool es_entropia_o_cp = (idx_ui == 2 || idx_ui == 3 || idx_ui == 6);
    if (es_entropia_o_cp) {
        std::string u = en_kcal ? "cal" : "J";
        u += por_kg ? "/(kg·K)" : "/(mol·K)";
        return u;
    }
    std::string u = en_kcal ? "kcal" : "kJ";
    u += por_kg ? "/kg" : "/mol";
    return u;
}

std::string formatear_etiqueta_curva_hsc(double coef, const std::string& formula, int idx_tipo) {
    if (idx_tipo == 7 && std::abs(coef - 1.0) > 1e-3) {
        char buf[96];
        std::snprintf(buf, sizeof(buf), "%.2f %s", coef, formula.c_str());
        return std::string(buf);
    }
    return formula;
}

void limpiar_filas_dia(EstadoModuloDia& st) {
    for (int i = 0; i < kNumFilasXData; ++i) {
        st.filas[i] = FilaXDataDia{};
    }
}

void ejecutar_calculo_dia(EstadoModuloDia& st, const cma::application::modules::PropertyDiagrams& motor) {
    auto& repo = base_datos::GestorDB::instancia();
    st.curvas.clear();

    const double t_min_K = st.temp_en_celsius ? (st.x_min + 273.15) : st.x_min;
    const double t_max_K = st.temp_en_celsius ? (st.x_max + 273.15) : st.x_max;
    const double span_T = std::max(1.0, std::abs(st.x_max - st.x_min));
    const double step_T = std::clamp(st.x_step, 1.0, span_T);
    const int n_steps = std::clamp(static_cast<int>(std::round(span_T / step_T)), 10, 400);

    double y_lo = 1e30;
    double y_hi = -1e30;
    const auto tipo_dom = mapear_tipo_diagrama(st.idx_tipo_diagrama);

    for (int i = 0; i < kNumFilasXData; ++i) {
        auto& f = st.filas[i];
        std::string form = f.formula;
        while (!form.empty() && std::isspace(static_cast<unsigned char>(form.back()))) form.pop_back();
        if (form.empty()) continue;

        auto c = motor.calcular_curva_especie(repo, form, t_min_K, t_max_K, n_steps, true, f.coef);
        if (c.points.empty()) continue;

        f.tmin_C = c.T_min_db_C;
        f.tmax_C = c.T_max_db_C;
        f.t1_C = (c.T_min_db_C < 0.0) ? c.T_min_db_C : 0.0;
        f.t2_C = (c.T_min_db_C < 0.0) ? 0.0 : 25.001;

        c.label = formatear_etiqueta_curva_hsc(f.coef, form, st.idx_tipo_diagrama);
        c.visible = f.visible;

        for (size_t k = 0; k < c.points.size(); ++k) {
            const double val = c.evaluar_propiedad(tipo_dom, k, st.energia_en_kcal, st.por_kg);
            if (std::isfinite(val)) {
                y_lo = std::min(y_lo, val);
                y_hi = std::max(y_hi, val);
            }
        }
        st.curvas.push_back(std::move(c));
    }

    if (st.y_auto && y_lo < y_hi) {
        if (st.idx_tipo_diagrama == 1) {
            // H Heat Content (Fig. 1: -3.60 a 554.00)
            st.y_min = std::min(0.0, y_lo);
            st.y_max = std::ceil(y_hi * 1.05);
        } else if (st.idx_tipo_diagrama == 7) {
            // Delta G Ellingham (Fig. 2 & 3: -1200 o -2000 a 0)
            st.y_min = std::floor(y_lo / 100.0) * 100.0;
            st.y_max = std::max(0.0, std::ceil(y_hi / 50.0) * 50.0);
        } else {
            const double pad = 0.05 * std::max(1.0, y_hi - y_lo);
            st.y_min = y_lo - pad;
            st.y_max = y_hi + pad;
        }
    }

    st.intersecciones = motor.encontrar_intersecciones(st.curvas, tipo_dom);
    motor.generar_lineas_isopotenciales(repo, t_min_K, t_max_K, st.iso_pO2, st.iso_CO_CO2, st.iso_H2_H2O);
    st.reset_ejes_plot = true;

    std::ostringstream os;
    os << titulo_tipo_diagrama(st.idx_tipo_diagrama) << ": " << st.curvas.size()
       << " especies calculadas, " << st.intersecciones.size() << " intersecciones.";
    st.estado_msg = os.str();
}

void balancear_elemento_dia(EstadoModuloDia& st, const cma::application::modules::PropertyDiagrams& motor) {
    for (int i = 0; i < kNumFilasXData; ++i) {
        std::string form = st.filas[i].formula;
        if (form.empty()) continue;
        st.filas[i].coef = motor.balancear_cantidad_elemento(form, st.elemento_balance, st.cantidad_balance);
    }
    ejecutar_calculo_dia(st, motor);
}

void recalcular_rxn_tab(EstadoModuloDia& st, const cma::application::modules::PropertyDiagrams& motor) {
    auto& repo = base_datos::GestorDB::instancia();
    const double t0_K = st.rxn_t_from_C + 273.15;
    const double t1_K = st.rxn_t_to_C + 273.15;
    const int steps = std::clamp(static_cast<int>(std::round((st.rxn_t_to_C - st.rxn_t_from_C) / std::max(1.0, st.rxn_t_step_C))), 5, 200);

    // En Fig. 7 (2ZnO = 2Zn + O2(g) vs 2ZnO = 2Zn(g) + O2(g)), cuando se compara Zn condensado vs Zn(g),
    // la curva 1 usa la fase condensada sin ebullición a 907 °C y la curva 2 usa explícitamente Zn(g)
    const bool usar_env_1 = !st.rxn_comparar_zn_ebullicion;
    st.curva_rxn_principal = motor.calcular_curva_ecuacion_texto(
        repo, st.rxn_eq_buf, domain::EllinghamNormalizationBasis::PerMoleReaction, t0_K, t1_K, steps, usar_env_1);

    if (st.rxn_comparar_zn_ebullicion) {
        st.curva_rxn_secundaria = motor.calcular_curva_ecuacion_texto(
            repo, "2ZnO = 2Zn(g) + O2(g)", domain::EllinghamNormalizationBasis::PerMoleReaction, t0_K, t1_K, steps, false);
    } else {
        st.curva_rxn_secundaria = domain::CurvaDiagramaTermo{};
    }
}

void cargar_preset_dia(int preset_id, EstadoModuloDia& st, const cma::application::modules::PropertyDiagrams& motor) {
    limpiar_filas_dia(st);
    st.temp_en_celsius = true;
    st.x_min = 0.0;
    st.x_max = 2000.0;
    st.x_step = 50.0;
    st.energia_en_kcal = false;
    st.por_kg = false;
    st.y_auto = true;

    if (preset_id == 1) {
        // Fig. 1: H Heat Content (Cu2O, FeO, Fe2O3, Fe3O4, CoO, Cr2O3, MgO, CO(g))
        const char* sps[] = {"Cu2O", "FeO", "Fe2O3", "Fe3O4", "CoO", "Cr2O3", "MgO", "CO(g)"};
        for (size_t i = 0; i < 8; ++i) {
            std::snprintf(st.filas[i].formula, sizeof(st.filas[i].formula), "%s", sps[i]);
            st.filas[i].coef = 1.00;
        }
        st.idx_tipo_diagrama = 1; // H Heat Content
        st.elemento_balance = "O";
        st.cantidad_balance = 1.00;
        st.fila_activa = 0;
        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "%s", st.filas[0].formula);
    } else if (preset_id == 2) {
        // Fig. 2, 3, 4: Delta G (Ellingham) de Óxidos Metalúrgicos balanceados a 2.00 átomos de O (1 mol O2)
        const char* sps[] = {"Cu2O", "FeO", "Fe2O3", "ZnO", "Cr2O3", "MgO", "CO(g)", "CO2(g)", "H2O(g)"};
        for (size_t i = 0; i < 9; ++i) {
            std::snprintf(st.filas[i].formula, sizeof(st.filas[i].formula), "%s", sps[i]);
        }
        st.idx_tipo_diagrama = 7; // Delta G (Ellingham)
        st.elemento_balance = "O";
        st.cantidad_balance = 2.00;
        st.fila_activa = 8; // H2O(g) activo como en Fig. 2 y Fig. 3
        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "H2O(g)");
        balancear_elemento_dia(st, motor);
        recalcular_rxn_tab(st, motor);
        return;
    } else if (preset_id == 3) {
        // Sulfuros metalúrgicos balanceados a 2.00 átomos de S (1 mol S2)
        const char* sps[] = {"Cu2S", "FeS", "FeS2", "ZnS", "PbS", "MoS2", "H2S", "SO2"};
        for (size_t i = 0; i < 8; ++i) {
            std::snprintf(st.filas[i].formula, sizeof(st.filas[i].formula), "%s", sps[i]);
        }
        st.idx_tipo_diagrama = 7;
        st.elemento_balance = "S";
        st.cantidad_balance = 2.00;
        st.fila_activa = 0;
        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "Cu2S");
        balancear_elemento_dia(st, motor);
        return;
    } else if (preset_id == 4) {
        // Cloruros metalúrgicos balanceados a 2.00 átomos de Cl (1 mol Cl2)
        const char* sps[] = {"CaCl2", "MgCl2", "AlCl3", "TiCl4", "ZnCl2", "FeCl2", "PbCl2", "HCl"};
        for (size_t i = 0; i < 8; ++i) {
            std::snprintf(st.filas[i].formula, sizeof(st.filas[i].formula), "%s", sps[i]);
        }
        st.idx_tipo_diagrama = 7;
        st.elemento_balance = "Cl";
        st.cantidad_balance = 2.00;
        st.fila_activa = 0;
        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "CaCl2");
        balancear_elemento_dia(st, motor);
        return;
    }

    ejecutar_calculo_dia(st, motor);
    recalcular_rxn_tab(st, motor);
}

} // namespace

void PanelDiagramasTermo::recalcular() {
    auto& st = estado_dia();
    if (!st.inicializado) {
        st.inicializado = true;
        cargar_preset_dia(2, st, motor_);
    } else {
        ejecutar_calculo_dia(st, motor_);
    }
    calculado_ = !st.curvas.empty();
}

void PanelDiagramasTermo::renderizar() {
    auto& st = estado_dia();
    if (!st.inicializado) {
        st.inicializado = true;
        cargar_preset_dia(2, st, motor_);
        calculado_ = !st.curvas.empty();
    }

    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(8.0f, 6.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6.0f, 4.0f));

    // ── Barra Superior de Menús y Presets Rápidos (Estilo HSC Dia) ──
    ImGui::BeginChild("##DiaTopToolbar", ImVec2(0.0f, 38.0f), true, ImGuiWindowFlags_NoScrollbar);
    {
        if (ImGui::Button("File  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##DiaFileMenu");
        if (ImGui::BeginPopup("##DiaFileMenu")) {
            ImGui::TextDisabled("HSC Thermochemical Data Diagrams (Section 19)");
            ImGui::Separator();
            if (ImGui::MenuItem("Load Fig. 1: H Heat Content (Oxides 0–2000 °C)")) cargar_preset_dia(1, st, motor_);
            if (ImGui::MenuItem("Load Fig. 2–4: Delta G Ellingham (Oxides, O = 2.00)")) cargar_preset_dia(2, st, motor_);
            if (ImGui::MenuItem("Load Ellingham Sulfides (S = 2.00)")) cargar_preset_dia(3, st, motor_);
            if (ImGui::MenuItem("Load Ellingham Chlorides (Cl = 2.00)")) cargar_preset_dia(4, st, motor_);
            ImGui::Separator();
            if (ImGui::MenuItem("Export Y-Data & Intersections CSV...")) {
                std::ofstream ofs("hsc_thermochemical_diagrams.csv");
                if (ofs) {
                    ofs << motor_.exportar_csv(st.curvas, st.intersecciones,
                                               mapear_tipo_diagrama(st.idx_tipo_diagrama),
                                               st.temp_en_celsius, st.energia_en_kcal);
                    st.estado_msg = "Exportado a hsc_thermochemical_diagrams.csv";
                }
            }
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Edit  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##DiaEditMenu");
        if (ImGui::BeginPopup("##DiaEditMenu")) {
            if (ImGui::MenuItem("Copy Y-Data & Intersections to Clipboard")) {
                const std::string csv = motor_.exportar_csv(
                    st.curvas, st.intersecciones, mapear_tipo_diagrama(st.idx_tipo_diagrama),
                    st.temp_en_celsius, st.energia_en_kcal);
                ImGui::SetClipboardText(csv.c_str());
                st.estado_msg = "Datos copiados al portapapeles.";
            }
            if (ImGui::MenuItem("Balance Element Amount Now")) {
                balancear_elemento_dia(st, motor_);
            }
            if (ImGui::MenuItem("Clear All Species Rows")) {
                limpiar_filas_dia(st);
                st.curvas.clear();
                st.intersecciones.clear();
            }
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Format  v", ImVec2(78.0f, 24.0f))) ImGui::OpenPopup("##DiaFormatMenu");
        if (ImGui::BeginPopup("##DiaFormatMenu")) {
            ImGui::MenuItem("Show On-Curve Formula Labels", nullptr, &st.mostrar_etiquetas_curvas);
            ImGui::MenuItem("Show Interactive Legend Box", nullptr, &st.mostrar_leyenda_interna);
            ImGui::MenuItem("Show Phase Transition Markers (Tm / Tb)", nullptr, &st.mostrar_transiciones);
            ImGui::Separator();
            ImGui::TextDisabled("Ellingham Nomographic Scales (Appendix 1)");
            ImGui::MenuItem("Overlay Iso-pO2 Lines (Eq. 11)", nullptr, &st.mostrar_iso_po2);
            ImGui::MenuItem("Overlay Iso-CO/CO2 Lines (Eq. 19)", nullptr, &st.mostrar_iso_co_co2);
            ImGui::MenuItem("Overlay Iso-H2/H2O Lines (Eq. 21)", nullptr, &st.mostrar_iso_h2_h2o);
            ImGui::EndPopup();
        }

        ImGui::SameLine();
        if (ImGui::Button("Help  v", ImVec2(68.0f, 24.0f))) ImGui::OpenPopup("##DiaHelpMenu");
        if (ImGui::BeginPopup("##DiaHelpMenu")) {
            ImGui::TextColored(ImVec4(0.20f, 0.48f, 0.85f, 1.0f), "19. H, S, C, and G Diagrams Module (Dia)");
            ImGui::Separator();
            ImGui::BulletText("1. Type species formulae in the first column of the X-data sheet.");
            ImGui::BulletText("2. Select one of the 9 diagram types from the Diagram Type list.");
            ImGui::BulletText("3. For Delta G (Ellingham): select an element (O, S, Cl) and press Balance Element Amount.");
            ImGui::BulletText("4. Press Diagram to draw the curves and inspect phase transitions & intersections.");
            ImGui::EndPopup();
        }

        ImGui::SameLine(0.0f, 16.0f);
        ImGui::TextDisabled("Reference Examples:");
        ImGui::SameLine();
        if (ImGui::SmallButton("Fig. 1: H Heat Content")) cargar_preset_dia(1, st, motor_);
        ImGui::SameLine();
        if (ImGui::SmallButton("Fig. 2-4: Ellingham (Oxides)")) cargar_preset_dia(2, st, motor_);
        ImGui::SameLine();
        if (ImGui::SmallButton("Sulfides (S=2)")) cargar_preset_dia(3, st, motor_);
        ImGui::SameLine();
        if (ImGui::SmallButton("Chlorides (Cl=2)")) cargar_preset_dia(4, st, motor_);
    }
    ImGui::EndChild();

    ImGui::Dummy(ImVec2(0.0f, 2.0f));

    // ═══════════════════════════════════════════════════════════════════════════
    // COLUMNA 1 (IZQUIERDA): Hoja de Datos X-data / Y-data / Intersections
    // ═══════════════════════════════════════════════════════════════════════════
    const float col_left_w = 420.0f;
    const float col_mid_w = 305.0f;

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(10.0f, 10.0f));
    ImGui::BeginChild("##DiaLeftSheetCol", ImVec2(col_left_w, 0.0f), true);
    {
        // Barra "Active species [ H2O(g) ]" como en Fig. 1, 2, 3
        ImGui::AlignTextToFramePadding();
        ImGui::TextUnformatted("Active species");
        ImGui::SameLine(105.0f);
        ImGui::SetNextItemWidth(-1.0f);
        if (ImGui::InputText("##DiaActiveSpeciesBox", st.active_species_buf, sizeof(st.active_species_buf),
                             ImGuiInputTextFlags_EnterReturnsTrue)) {
            if (st.fila_activa >= 0 && st.fila_activa < kNumFilasXData) {
                std::snprintf(st.filas[st.fila_activa].formula, sizeof(st.filas[st.fila_activa].formula),
                              "%s", st.active_species_buf);
                ejecutar_calculo_dia(st, motor_);
            }
        }

        ImGui::Dummy(ImVec2(0.0f, 3.0f));

        // Contenido de la hoja activa (X-data / Y-data / Intersections)
        const float sheet_h = ImGui::GetContentRegionAvail().y - 36.0f;
        ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(6.0f, 4.0f));

        if (st.hoja_activa == 0) {
            // Hoja X-data (Columnas A..F exactas de Fig. 1, 2 y 3)
            const char* tmin_hdr = st.temp_en_celsius ? "TMin (°C)" : "TMin (K)";
            const char* tmax_hdr = st.temp_en_celsius ? "TMax (°C)" : "TMax (K)";
            if (ImGui::BeginTable("##DiaXDataSheet", 7,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
                                  ImVec2(0.0f, sheet_h))) {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 26.0f);
                ImGui::TableSetupColumn("A: Species", ImGuiTableColumnFlags_WidthFixed, 92.0f);
                ImGui::TableSetupColumn("B: Coef.", ImGuiTableColumnFlags_WidthFixed, 56.0f);
                ImGui::TableSetupColumn(tmin_hdr, ImGuiTableColumnFlags_WidthFixed, 68.0f);
                ImGui::TableSetupColumn(tmax_hdr, ImGuiTableColumnFlags_WidthFixed, 72.0f);
                ImGui::TableSetupColumn("E: T1", ImGuiTableColumnFlags_WidthFixed, 54.0f);
                ImGui::TableSetupColumn("F: T2", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();

                for (int r = 0; r < kNumFilasXData; ++r) {
                    auto& row = st.filas[r];
                    ImGui::TableNextRow();
                    ImGui::PushID(r + 100);

                    ImGui::TableSetColumnIndex(0);
                    const bool is_row_sel = (st.fila_activa == r);
                    char r_lbl[12];
                    std::snprintf(r_lbl, sizeof(r_lbl), "%d", r + 1);
                    if (ImGui::Selectable(r_lbl, is_row_sel, ImGuiSelectableFlags_None, ImVec2(22.0f, 18.0f))) {
                        st.fila_activa = r;
                        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "%s", row.formula);
                    }

                    ImGui::TableSetColumnIndex(1);
                    ImGui::SetNextItemWidth(-1.0f);
                    if (ImGui::InputText("##sp", row.formula, sizeof(row.formula))) {
                        st.fila_activa = r;
                        std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "%s", row.formula);
                    }
                    if (ImGui::IsItemDeactivatedAfterEdit()) {
                        ejecutar_calculo_dia(st, motor_);
                    }

                    ImGui::TableSetColumnIndex(2);
                    ImGui::SetNextItemWidth(-1.0f);
                    float cf = static_cast<float>(row.coef);
                    if (ImGui::InputFloat("##cf", &cf, 0.0f, 0.0f, "%.2f")) {
                        row.coef = std::max(0.001, static_cast<double>(cf));
                    }
                    if (ImGui::IsItemDeactivatedAfterEdit()) {
                        ejecutar_calculo_dia(st, motor_);
                    }

                    const bool has_sp = (row.formula[0] != '\0');
                    const double disp_tmin = st.temp_en_celsius ? row.tmin_C : (row.tmin_C + 273.15);
                    const double disp_tmax = st.temp_en_celsius ? row.tmax_C : (row.tmax_C + 273.15);

                    ImGui::TableSetColumnIndex(3);
                    if (has_sp) ImGui::Text("%.2f", disp_tmin);
                    ImGui::TableSetColumnIndex(4);
                    if (has_sp) ImGui::Text("%.2f", disp_tmax);
                    ImGui::TableSetColumnIndex(5);
                    if (has_sp) ImGui::Text("%.2f", row.t1_C);
                    ImGui::TableSetColumnIndex(6);
                    if (has_sp) ImGui::Text("%.2f", row.t2_C);

                    ImGui::PopID();
                }
                ImGui::EndTable();
            }
        } else if (st.hoja_activa == 1) {
            // Hoja Y-data (Valores numéricos tabulados en cada paso de temperatura)
            const int n_cols = std::min(static_cast<int>(st.curvas.size()) + 1, 12);
            if (n_cols > 1 && ImGui::BeginTable("##DiaYDataSheet", n_cols,
                                                ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                                ImGuiTableFlags_ScrollY | ImGuiTableFlags_ScrollX,
                                                ImVec2(0.0f, sheet_h))) {
                ImGui::TableSetupScrollFreeze(1, 1);
                ImGui::TableSetupColumn(st.temp_en_celsius ? "T (°C)" : "T (K)", ImGuiTableColumnFlags_WidthFixed, 68.0f);
                for (int c = 0; c < n_cols - 1; ++c) {
                    ImGui::TableSetupColumn(st.curvas[c].label.c_str(), ImGuiTableColumnFlags_WidthFixed, 82.0f);
                }
                ImGui::TableHeadersRow();

                const auto tipo_dom = mapear_tipo_diagrama(st.idx_tipo_diagrama);
                const size_t n_pts = st.curvas.front().points.size();
                for (size_t p = 0; p < n_pts; ++p) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    const double t_val = st.temp_en_celsius
                                             ? (st.curvas.front().points[p].T - 273.15)
                                             : st.curvas.front().points[p].T;
                    ImGui::Text("%.1f", t_val);
                    for (int c = 0; c < n_cols - 1; ++c) {
                        ImGui::TableSetColumnIndex(c + 1);
                        const double y_val = st.curvas[c].evaluar_propiedad(tipo_dom, p, st.energia_en_kcal, st.por_kg);
                        ImGui::Text("%.2f", y_val);
                    }
                }
                ImGui::EndTable();
            } else {
                ImGui::BeginChild("##EmptyYData", ImVec2(0.0f, sheet_h), true);
                ImGui::TextDisabled("No active curves calculated.");
                ImGui::EndChild();
            }
        } else {
            // Hoja Intersections & Nomograph (Cruces de inversión de reducción + pO2, CO/CO2, H2/H2O)
            if (ImGui::BeginTable("##DiaIntersectionsSheet", 6,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable,
                                  ImVec2(0.0f, sheet_h))) {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("Curve A", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("Curve B", ImGuiTableColumnFlags_WidthFixed, 80.0f);
                ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 58.0f);
                ImGui::TableSetupColumn("Value Y", ImGuiTableColumnFlags_WidthFixed, 66.0f);
                ImGui::TableSetupColumn("log pO2", ImGuiTableColumnFlags_WidthFixed, 58.0f);
                ImGui::TableSetupColumn("CO/CO2", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();

                for (const auto& inter : st.intersecciones) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(inter.curve_a.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(inter.curve_b.c_str());
                    ImGui::TableSetColumnIndex(2); ImGui::Text("%.1f", inter.T_C);
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.1f", inter.value_Y);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%.1f", inter.log10_pO2);
                    ImGui::TableSetColumnIndex(5); ImGui::Text("10^(%.1f)", inter.log10_CO_CO2);
                }
                ImGui::EndTable();
            }
        }
        ImGui::PopStyleVar(); // CellPadding

        // Barra inferior de pestañas estilo hoja de cálculo: X-data | Y-data | Intersections
        ImGui::Dummy(ImVec2(0.0f, 2.0f));
        if (ImGui::Button("X-data", ImVec2(95.0f, 24.0f))) st.hoja_activa = 0;
        ImGui::SameLine();
        if (ImGui::Button("Y-data", ImVec2(95.0f, 24.0f))) st.hoja_activa = 1;
        ImGui::SameLine();
        if (ImGui::Button("Intersections", ImVec2(120.0f, 24.0f))) st.hoja_activa = 2;
    }
    ImGui::EndChild();
    ImGui::PopStyleVar(); // WindowPadding

    ImGui::SameLine(0.0f, 8.0f);

    // ═══════════════════════════════════════════════════════════════════════════
    // COLUMNA 2 (CENTRAL): Diagram Type, X/Y Ranges, Balance Element Amount, Diagram
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(10.0f, 10.0f));
    ImGui::BeginChild("##DiaMiddleControlsCol", ImVec2(col_mid_w, 0.0f), true);
    {
        // 1. Diagram Type (Los 9 tipos de diagrama de la Sección 19)
        ImGui::TextColored(ImVec4(0.20f, 0.45f, 0.75f, 1.0f), "Diagram Type");
        ImGui::SetNextItemWidth(-1.0f);
        const char* kTiposCombo =
            "H Enthalpy (total)\0"
            "H Heat Content\0"
            "S Entropy\0"
            "Cp Heat Capacity\0"
            "G Gibbs Energy\0"
            "Delta H\0"
            "Delta S\0"
            "Delta G (Ellingham)\0"
            "Ex Exergy (total)\0";
        if (ImGui::Combo("##DiaTypeCombo", &st.idx_tipo_diagrama, kTiposCombo)) {
            st.y_auto = true;
            ejecutar_calculo_dia(st, motor_);
        }

        ImGui::Dummy(ImVec2(0.0f, 4.0f));
        ImGui::Separator();

        // 2. X-axis Range (Temperature): MIN | MAX | STEP + °C / K
        ImGui::TextColored(ImVec4(0.20f, 0.45f, 0.75f, 1.0f), "X-axis Range (Temperature)");
        if (ImGui::BeginTable("##DiaXRangeTbl", 3, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("MIN");
            ImGui::TableSetColumnIndex(1); ImGui::TextDisabled("MAX");
            ImGui::TableSetColumnIndex(2); ImGui::TextDisabled("STEP");

            ImGui::TableNextRow();
            float xm = static_cast<float>(st.x_min);
            float xx = static_cast<float>(st.x_max);
            float xs = static_cast<float>(st.x_step);
            ImGui::TableSetColumnIndex(0); ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputFloat("##xmin", &xm, 0.0f, 0.0f, "%.2f")) st.x_min = xm;
            ImGui::TableSetColumnIndex(1); ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputFloat("##xmax", &xx, 0.0f, 0.0f, "%.2f")) st.x_max = xx;
            ImGui::TableSetColumnIndex(2); ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputFloat("##xstp", &xs, 0.0f, 0.0f, "%.2f")) st.x_step = std::max(1.0f, xs);
            ImGui::EndTable();
        }
        int u_temp = st.temp_en_celsius ? 0 : 1;
        if (ImGui::RadioButton("°C##dia_c", &u_temp, 0)) {
            if (!st.temp_en_celsius) {
                st.x_min -= 273.15;
                st.x_max -= 273.15;
                st.temp_en_celsius = true;
                ejecutar_calculo_dia(st, motor_);
            }
        }
        ImGui::SameLine(0.0f, 24.0f);
        if (ImGui::RadioButton("K##dia_k", &u_temp, 1)) {
            if (st.temp_en_celsius) {
                st.x_min += 273.15;
                st.x_max += 273.15;
                st.temp_en_celsius = false;
                ejecutar_calculo_dia(st, motor_);
            }
        }

        ImGui::Dummy(ImVec2(0.0f, 4.0f));
        ImGui::Separator();

        // 3. Y-axis Range: MIN | MAX | Auto + kcal/mol | kJ/mol | kg
        ImGui::TextColored(ImVec4(0.20f, 0.45f, 0.75f, 1.0f), "Y-axis Range");
        if (ImGui::BeginTable("##DiaYRangeTbl", 3, ImGuiTableFlags_SizingStretchSame)) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("MIN");
            ImGui::TableSetColumnIndex(1); ImGui::TextDisabled("MAX");
            ImGui::TableSetColumnIndex(2); ImGui::TextDisabled("SCALE");

            ImGui::TableNextRow();
            float ym = static_cast<float>(st.y_min);
            float yx = static_cast<float>(st.y_max);
            ImGui::TableSetColumnIndex(0); ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputFloat("##ymin", &ym, 0.0f, 0.0f, "%.2f")) { st.y_min = ym; st.y_auto = false; st.reset_ejes_plot = true; }
            ImGui::TableSetColumnIndex(1); ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputFloat("##ymax", &yx, 0.0f, 0.0f, "%.2f")) { st.y_max = yx; st.y_auto = false; st.reset_ejes_plot = true; }
            ImGui::TableSetColumnIndex(2);
            if (ImGui::Checkbox("Auto", &st.y_auto) && st.y_auto) ejecutar_calculo_dia(st, motor_);
            ImGui::EndTable();
        }
        int u_en = st.energia_en_kcal ? 0 : 1;
        if (ImGui::RadioButton("kcal/mol", &u_en, 0)) {
            st.energia_en_kcal = true;
            ejecutar_calculo_dia(st, motor_);
        }
        ImGui::SameLine(0.0f, 12.0f);
        if (ImGui::RadioButton("kJ/mol", &u_en, 1)) {
            st.energia_en_kcal = false;
            ejecutar_calculo_dia(st, motor_);
        }
        ImGui::SameLine(0.0f, 12.0f);
        if (ImGui::Checkbox("kg", &st.por_kg)) {
            ejecutar_calculo_dia(st, motor_);
        }

        ImGui::Dummy(ImVec2(0.0f, 4.0f));
        ImGui::Separator();

        // 4. Balance Element Amount + Cuadrícula de Elementos (Fig. 1, 2, 3)
        if (ImGui::Button("Balance Element Amount", ImVec2(185.0f, 26.0f))) {
            balancear_elemento_dia(st, motor_);
        }
        ImGui::SameLine();
        float bal_amt = static_cast<float>(st.cantidad_balance);
        ImGui::SetNextItemWidth(-1.0f);
        if (ImGui::InputFloat("##DiaBalAmt", &bal_amt, 0.0f, 0.0f, "%.2f")) {
            st.cantidad_balance = std::max(0.1, static_cast<double>(bal_amt));
        }

        ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(3.0f, 2.5f));
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(1.0f, 1.5f));
        ImGui::BeginChild("##DiaElementGridBox", ImVec2(0.0f, 196.0f), true);
        {
            for (int r = 0; r < 12; ++r) {
                for (int c = 0; c < 9; ++c) {
                    const char* sym = kGridElementosDia[r][c];
                    if (!sym || sym[0] == '\0') {
                        if (c > 0) ImGui::SameLine(6.0f + c * 30.0f);
                        ImGui::Dummy(ImVec2(27.5f, 17.5f));
                        continue;
                    }
                    if (c > 0) ImGui::SameLine(6.0f + c * 30.0f);

                    const bool is_sel = (st.elemento_balance == sym);
                    if (is_sel) {
                        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.22f, 0.50f, 0.86f, 0.95f));
                        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
                    }
                    std::string bid = std::string(sym) + "##dia_el";
                    if (ImGui::Button(bid.c_str(), ImVec2(27.5f, 17.5f))) {
                        st.elemento_balance = sym;
                        if (st.idx_tipo_diagrama == 7) {
                            balancear_elemento_dia(st, motor_);
                        }
                    }
                    if (is_sel) ImGui::PopStyleColor(2);
                }
            }
        }
        ImGui::EndChild();
        ImGui::PopStyleVar(2);

        ImGui::Dummy(ImVec2(0.0f, 4.0f));

        // 5. Botones inferiores: Browse Database y Diagram (Fig. 1, 2, 3)
        if (ImGui::Button("Browse Database", ImVec2(-1.0f, 28.0f))) {
            show_select_species_modal_ = true;
            ImGui::OpenPopup("Browse Database");
        }
        if (ImGui::Button("Diagram", ImVec2(-1.0f, 30.0f))) {
            ejecutar_calculo_dia(st, motor_);
        }

        if (!st.estado_msg.empty()) {
            ImGui::TextDisabled("%s", st.estado_msg.c_str());
        }
    }
    ImGui::EndChild();
    ImGui::PopStyleVar(); // WindowPadding

    ImGui::SameLine(0.0f, 8.0f);

    // ═══════════════════════════════════════════════════════════════════════════
    // COLUMNA 3 (DERECHA): Lienzo Gráfico Principal + Reaction / pO2 Equilibrium (Fig. 5–7)
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(12.0f, 10.0f));
    ImGui::BeginChild("##DiaRightPlotCol", ImVec2(0.0f, 0.0f), true);
    {
        if (ImGui::BeginTabBar("##DiaRightTabs")) {
            // ── PESTAÑA 1: Gráfico Termoquímico / Ellingham (Fig. 1, 2, 4) ──
            if (ImGui::BeginTabItem("Thermochemical Diagram (Dia)")) {
                const char* chart_title = titulo_tipo_diagrama(st.idx_tipo_diagrama);
                const std::string y_lbl = etiqueta_eje_y_dia(st.idx_tipo_diagrama, st.energia_en_kcal, st.por_kg);
                const char* x_lbl = st.temp_en_celsius ? "Temperature (°C)" : "Temperature (K)";
                const auto tipo_dom = mapear_tipo_diagrama(st.idx_tipo_diagrama);

                ImPlot::PushStyleColor(ImPlotCol_PlotBg, ImVec4(0.985f, 0.988f, 0.992f, 1.0f));
                ImPlot::PushStyleColor(ImPlotCol_AxisGrid, ImVec4(0.75f, 0.78f, 0.82f, 0.55f));

                std::string plot_id = std::string(chart_title) + "##DiaMainPlot";
                const ImPlotFlags pflags = st.mostrar_leyenda_interna ? ImPlotFlags_None : ImPlotFlags_NoLegend;

                if (ImPlot::BeginPlot(plot_id.c_str(), ImVec2(-1.0f, -1.0f), pflags)) {
                    ImPlot::SetupAxes(x_lbl, y_lbl.c_str());
                    if (st.reset_ejes_plot) {
                        ImPlot::SetupAxesLimits(st.x_min, st.x_max, st.y_min, st.y_max, ImGuiCond_Always);
                        st.reset_ejes_plot = false;
                    } else {
                        ImPlot::SetupAxesLimits(st.x_min, st.x_max, st.y_min, st.y_max, ImGuiCond_Once);
                    }
                    if (st.mostrar_leyenda_interna) {
                        ImPlot::SetupLegend(ImPlotLocation_SouthEast, ImPlotLegendFlags_None);
                    }

                    // 1. Abanicos nomográficos opcionales (Apéndice 1: pO2, CO/CO2, H2/H2O)
                    if (st.idx_tipo_diagrama == 7) {
                        auto plot_iso_fan = [&](const std::vector<domain::LineaIsoPotencial>& lines, const ImVec4& col) {
                            const double kf = st.energia_en_kcal ? (1.0 / 4.184) : 1.0;
                            for (const auto& ln : lines) {
                                if (ln.T_K.size() < 2) continue;
                                double xs[2] = {
                                    st.temp_en_celsius ? (ln.T_K[0] - 273.15) : ln.T_K[0],
                                    st.temp_en_celsius ? (ln.T_K[1] - 273.15) : ln.T_K[1]
                                };
                                double ys[2] = {ln.delta_G_kJ[0] * kf, ln.delta_G_kJ[1] * kf};
                                ImPlot::SetNextLineStyle(col, 1.0f);
                                ImPlot::PlotLine(("##iso_" + ln.label).c_str(), xs, ys, 2);
                                ImPlot::PlotText(ln.label.c_str(), xs[1] * 0.88 + xs[0] * 0.12, ys[1] * 0.88 + ys[0] * 0.12);
                            }
                        };
                        if (st.mostrar_iso_po2) plot_iso_fan(st.iso_pO2, ImVec4(0.55f, 0.55f, 0.60f, 0.55f));
                        if (st.mostrar_iso_co_co2) plot_iso_fan(st.iso_CO_CO2, ImVec4(0.25f, 0.50f, 0.78f, 0.55f));
                        if (st.mostrar_iso_h2_h2o) plot_iso_fan(st.iso_H2_H2O, ImVec4(0.20f, 0.62f, 0.45f, 0.55f));
                    }

                    // 2. Curvas de especies y etiquetas directas sobre las curvas (Fig. 1, 2, 4)
                    ImDrawList* dl = ImPlot::GetPlotDrawList();
                    for (size_t c = 0; c < st.curvas.size(); ++c) {
                        const auto& cur = st.curvas[c];
                        if (!cur.visible || cur.points.empty()) continue;

                        std::vector<double> xs(cur.points.size());
                        std::vector<double> ys(cur.points.size());
                        for (size_t k = 0; k < cur.points.size(); ++k) {
                            xs[k] = st.temp_en_celsius ? (cur.points[k].T - 273.15) : cur.points[k].T;
                            ys[k] = cur.evaluar_propiedad(tipo_dom, k, st.energia_en_kcal, st.por_kg);
                        }

                        const ImVec4 col = kColoresCurvasDia[c % kNumColoresCurvasDia];
                        ImPlot::SetNextLineStyle(col, 2.0f);
                        ImPlot::PlotLine(cur.label.c_str(), xs.data(), ys.data(), static_cast<int>(xs.size()));

                        // Etiqueta coloreada junto a la curva (distribuida escalonadamente como en Fig. 1 y Fig. 4)
                        if (st.mostrar_etiquetas_curvas && xs.size() >= 4) {
                            const double frac = (st.idx_tipo_diagrama == 1)
                                                    ? (0.78 + 0.025 * static_cast<double>(c % 6))
                                                    : (0.12 + 0.095 * static_cast<double>(c % 8));
                            const size_t idx_lbl = std::clamp(
                                static_cast<size_t>(frac * static_cast<double>(xs.size() - 1)),
                                size_t(1), xs.size() - 2);
                            const ImVec2 px = ImPlot::PlotToPixels(xs[idx_lbl], ys[idx_lbl]);
                            dl->AddText(ImVec2(px.x + 4.0f, px.y - 15.0f),
                                        ImGui::ColorConvertFloat4ToU32(col), cur.label.c_str());
                        }
                    }

                    // 3. Sonda termodinámica y nomográfica al pasar el ratón
                    if (ImPlot::IsPlotHovered()) {
                        const ImPlotPoint mp = ImPlot::GetPlotMousePos();
                        const double T_K = st.temp_en_celsius ? (mp.x + 273.15) : mp.x;
                        ImGui::BeginTooltip();
                        ImGui::Text("T = %.1f °C (%.2f K) | %s = %.2f %s",
                                    T_K - 273.15, T_K, chart_title, mp.y, y_lbl.c_str());
                        if (st.idx_tipo_diagrama == 7 && std::abs(st.cantidad_balance - 2.0) < 0.05 && st.elemento_balance == "O") {
                            const double dG_kJ = st.energia_en_kcal ? (mp.y * 4.184) : mp.y;
                            auto nom = motor_.evaluar_escala_nomografica(base_datos::GestorDB::instancia(), T_K, dG_kJ);
                            ImGui::Separator();
                            ImGui::Text("Equilibrium pO2(g):   10^(%.2f) bar", nom.log10_pO2);
                            ImGui::Text("CO / CO2 Ratio:       10^(%.2f)", nom.log10_CO_CO2);
                            ImGui::Text("H2 / H2O Ratio:       10^(%.2f)", nom.log10_H2_H2O);
                        }
                        ImGui::EndTooltip();
                    }

                    ImPlot::EndPlot();
                }
                ImPlot::PopStyleColor(2);
                ImGui::EndTabItem();
            }

            // ── PESTAÑA 2: Reaction Equations & pO2 Equilibrium (Fig. 5, Fig. 6, Fig. 7) ──
            if (ImGui::BeginTabItem("Reaction & pO2 Equilibrium (Fig. 5–7)")) {
                if (st.curva_rxn_principal.points.empty()) {
                    recalcular_rxn_tab(st, motor_);
                }

                // Barra de ecuaciones preconfiguradas del manual (Eq. 3 / Fig. 5, Fig. 6, Fig. 7)
                ImGui::AlignTextToFramePadding();
                ImGui::TextDisabled("Manual Equations:");
                ImGui::SameLine();
                if (ImGui::SmallButton("Fig. 5: 2Cu2O + 2Fe = 4Cu + 2FeO")) {
                    std::snprintf(st.rxn_eq_buf, sizeof(st.rxn_eq_buf), "2Cu2O + 2Fe = 4Cu + 2FeO");
                    st.rxn_t_from_C = 0.0; st.rxn_t_to_C = 1000.0; st.rxn_t_step_C = 100.0;
                    st.rxn_y_axis_mode = 0; st.rxn_comparar_zn_ebullicion = false;
                    recalcular_rxn_tab(st, motor_);
                }
                ImGui::SameLine();
                if (ImGui::SmallButton("Fig. 6: 2FeO = 2Fe + O2(g) [K = pO2]")) {
                    std::snprintf(st.rxn_eq_buf, sizeof(st.rxn_eq_buf), "2FeO = 2Fe + O2(g)");
                    st.rxn_t_from_C = 0.0; st.rxn_t_to_C = 1000.0; st.rxn_t_step_C = 100.0;
                    st.rxn_y_axis_mode = 1; st.rxn_comparar_zn_ebullicion = false;
                    recalcular_rxn_tab(st, motor_);
                }
                ImGui::SameLine();
                if (ImGui::SmallButton("Fig. 7: 2ZnO = 2Zn/Zn(g) + O2(g) [Boiling Point 907 °C]")) {
                    std::snprintf(st.rxn_eq_buf, sizeof(st.rxn_eq_buf), "2ZnO = 2Zn + O2(g)");
                    st.rxn_t_from_C = 0.0; st.rxn_t_to_C = 2000.0; st.rxn_t_step_C = 100.0;
                    st.rxn_y_axis_mode = 1; st.rxn_comparar_zn_ebullicion = true;
                    recalcular_rxn_tab(st, motor_);
                }

                // Entrada de ecuación y rango térmico
                ImGui::SetNextItemWidth(280.0f);
                ImGui::InputText("Reaction Equation", st.rxn_eq_buf, sizeof(st.rxn_eq_buf));
                ImGui::SameLine();
                ImGui::SetNextItemWidth(95.0f);
                ImGui::Combo("Y - Axis", &st.rxn_y_axis_mode, "ΔG (kJ)\0K = p(O2(g))\0");
                ImGui::SameLine();
                if (ImGui::Button("Calculate", ImVec2(100.0f, 0.0f))) {
                    recalcular_rxn_tab(st, motor_);
                }

                ImGui::Dummy(ImVec2(0.0f, 3.0f));

                // Mitad izquierda: Tabla Reaction Data (T °C | ΔH kJ | ΔS J/K | ΔG kJ | K | Log K) (Fig. 5 y 6)
                const float rxn_left_w = 430.0f;
                ImGui::BeginChild("##RxnTablePane", ImVec2(rxn_left_w, 0.0f), true);
                {
                    ImGui::TextColored(ImVec4(0.18f, 0.42f, 0.72f, 1.0f), "Reaction Equation: %s", st.rxn_eq_buf);
                    ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(6.0f, 4.0f));
                    if (ImGui::BeginTable("##RxnDataGrid", 6,
                                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
                        ImGui::TableSetupScrollFreeze(0, 1);
                        ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 58.0f);
                        ImGui::TableSetupColumn("ΔH (kJ)", ImGuiTableColumnFlags_WidthFixed, 66.0f);
                        ImGui::TableSetupColumn("ΔS (J/K)", ImGuiTableColumnFlags_WidthFixed, 62.0f);
                        ImGui::TableSetupColumn("ΔG (kJ)", ImGuiTableColumnFlags_WidthFixed, 68.0f);
                        ImGui::TableSetupColumn("K", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                        ImGui::TableSetupColumn("Log K", ImGuiTableColumnFlags_WidthStretch);
                        ImGui::TableHeadersRow();

                        for (const auto& pt : st.curva_rxn_principal.points) {
                            ImGui::TableNextRow();
                            ImGui::TableSetColumnIndex(0); ImGui::Text("%.1f", pt.T - 273.15);
                            ImGui::TableSetColumnIndex(1); ImGui::Text("%.3f", pt.dHf);
                            ImGui::TableSetColumnIndex(2); ImGui::Text("%.3f", pt.dSf);
                            ImGui::TableSetColumnIndex(3); ImGui::Text("%.3f", pt.dGf);
                            ImGui::TableSetColumnIndex(4);
                            const double K_val = std::pow(10.0, std::clamp(pt.log10K, -300.0, 300.0));
                            ImGui::Text("%.3E", K_val);
                            ImGui::TableSetColumnIndex(5); ImGui::Text("%.3f", pt.log10K);
                        }
                        ImGui::EndTable();
                    }
                    ImGui::PopStyleVar();
                }
                ImGui::EndChild();

                ImGui::SameLine(0.0f, 8.0f);

                // Mitad derecha: Gráfico de Reaction Gibbs Free Energy (Fig. 5) o Equilibrium Constant K (Fig. 6 & Fig. 7)
                ImGui::BeginChild("##RxnChartPane", ImVec2(0.0f, 0.0f), true);
                {
                    const char* r_title = (st.rxn_y_axis_mode == 0)
                                              ? "Reaction Gibbs free energy##rxn_plot"
                                              : "Equilibrium constant (log10 K = log10 pO2)##rxn_plot";
                    const char* ry_lbl = (st.rxn_y_axis_mode == 0) ? "ΔG (kJ)" : "log10 K  [K = p(O2(g))]";

                    if (ImPlot::BeginPlot(r_title, ImVec2(-1.0f, -1.0f))) {
                        ImPlot::SetupAxes("T (°C)", ry_lbl, ImPlotAxisFlags_AutoFit, ImPlotAxisFlags_AutoFit);
                        auto plot_rxn_curve = [&](const domain::CurvaDiagramaTermo& crv, const char* lbl, const ImVec4& col) {
                            if (crv.points.empty()) return;
                            std::vector<double> xs(crv.points.size());
                            std::vector<double> ys(crv.points.size());
                            for (size_t i = 0; i < crv.points.size(); ++i) {
                                xs[i] = crv.points[i].T - 273.15;
                                ys[i] = (st.rxn_y_axis_mode == 0) ? crv.points[i].dGf : crv.points[i].log10K;
                            }
                            ImPlot::SetNextLineStyle(col, 2.2f);
                            ImPlot::PlotLine(lbl, xs.data(), ys.data(), static_cast<int>(xs.size()));
                        };

                        plot_rxn_curve(st.curva_rxn_principal, st.rxn_eq_buf, ImVec4(0.28f, 0.52f, 0.22f, 1.0f));
                        if (st.rxn_comparar_zn_ebullicion && !st.curva_rxn_secundaria.points.empty()) {
                            plot_rxn_curve(st.curva_rxn_secundaria, "2ZnO = 2Zn(g) + O2(g)", ImVec4(0.12f, 0.48f, 0.76f, 1.0f));
                        }
                        ImPlot::EndPlot();
                    }
                }
                ImGui::EndChild();

                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
    }
    ImGui::EndChild();
    ImGui::PopStyleVar(); // WindowPadding

    // ═══════════════════════════════════════════════════════════════════════════
    // MODAL: Browse Database (Para agregar especies a la hoja X-data)
    // ═══════════════════════════════════════════════════════════════════════════
    ImGui::SetNextWindowSize(ImVec2(640.0f, 480.0f), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Browse Database", &show_select_species_modal_)) {
        auto db_especies = base_datos::GestorDB::instancia().todas_especies();
        ImGui::SetNextItemWidth(-1.0f);
        ImGui::InputTextWithHint("##SearchDiaDB", "Search species by formula or name (e.g. Fe2O3, ZnO, TiO2)...",
                                 search_buffer_, IM_ARRAYSIZE(search_buffer_));
        std::string search_str = search_buffer_;
        std::transform(search_str.begin(), search_str.end(), search_str.begin(), ::tolower);

        if (ImGui::BeginTable("##DiaDBTable", 4,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0.0f, -42.0f))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Action", ImGuiTableColumnFlags_WidthFixed, 75.0f);
            ImGui::TableSetupColumn("Formula", ImGuiTableColumnFlags_WidthFixed, 120.0f);
            ImGui::TableSetupColumn("Name", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("ΔHf°298 (kJ/mol)", ImGuiTableColumnFlags_WidthFixed, 125.0f);
            ImGui::TableHeadersRow();

            for (const auto* esp : db_especies) {
                std::string nm = esp->nombre;
                std::string fm = esp->formula;
                std::string nml = nm, fml = fm;
                std::transform(nml.begin(), nml.end(), nml.begin(), ::tolower);
                std::transform(fml.begin(), fml.end(), fml.begin(), ::tolower);

                if (search_str.empty() || nml.find(search_str) != std::string::npos || fml.find(search_str) != std::string::npos) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImGui::PushID(static_cast<int>(esp->id));
                    if (ImGui::SmallButton("Add Row")) {
                        for (int r = 0; r < kNumFilasXData; ++r) {
                            if (st.filas[r].formula[0] == '\0') {
                                std::snprintf(st.filas[r].formula, sizeof(st.filas[r].formula), "%s", fm.c_str());
                                st.filas[r].coef = (st.idx_tipo_diagrama == 7)
                                                       ? motor_.balancear_cantidad_elemento(fm, st.elemento_balance, st.cantidad_balance)
                                                       : 1.0;
                                st.fila_activa = r;
                                std::snprintf(st.active_species_buf, sizeof(st.active_species_buf), "%s", fm.c_str());
                                ejecutar_calculo_dia(st, motor_);
                                break;
                            }
                        }
                    }
                    ImGui::PopID();
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(fm.c_str());
                    ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(nm.c_str());
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f", esp->h_formacion);
                }
            }
            ImGui::EndTable();
        }

        if (ImGui::Button("Close", ImVec2(130.0f, 28.0f))) {
            show_select_species_modal_ = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    ImGui::PopStyleVar(2); // ItemSpacing + FramePadding
}

}} // namespace

#pragma GCC diagnostic pop
