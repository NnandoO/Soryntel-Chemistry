#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de Exergía
// ═══════════════════════════════════════════════════════════════════════════
void PanelExergia::renderizar() {
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
    if (ImGui::Button(tr("Calcular exergía"), ImVec2(-1,30))) {
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
}

// ═══════════════════════════════════════════════════════════════════════════
// Panel de Diagramas Termoquímicos y Ellingham (HSC H, S, Cp, G & Ellingham)
// ═══════════════════════════════════════════════════════════════════════════

namespace {

const char* nombre_base_normalizacion(domain::EllinghamNormalizationBasis b) {
    switch (b) {
    case domain::EllinghamNormalizationBasis::PerMoleO2: return "por 1 mol O2(g)";
    case domain::EllinghamNormalizationBasis::PerMoleS2: return "por 1 mol S2(g)";
    case domain::EllinghamNormalizationBasis::PerMoleCl2: return "por 1 mol Cl2(g)";
    case domain::EllinghamNormalizationBasis::PerMoleCO2: return "por 1 mol CO2(g)";
    case domain::EllinghamNormalizationBasis::PerMoleSO3: return "por 1 mol SO3(g)";
    case domain::EllinghamNormalizationBasis::PerMoleH2O: return "por 1 mol H2O(g)";
    case domain::EllinghamNormalizationBasis::PerMoleReaction: return "por mol de reacción";
    }
    return "por 1 mol O2(g)";
}

std::string etiqueta_eje_y(domain::PropertyDiagramMode modo,
                           domain::PropertyDiagramType tipo,
                           domain::EllinghamNormalizationBasis basis,
                           bool en_kcal,
                           bool usar_dHf) {
    const char* u_e = en_kcal ? "kcal" : "kJ";
    const char* u_s = en_kcal ? "cal/(mol·K)" : "J/(mol·K)";
    const char* suf = (modo == domain::PropertyDiagramMode::EllinghamReactions)
                          ? nombre_base_normalizacion(basis)
                          : "por mol";
    const bool rxn = (modo == domain::PropertyDiagramMode::EllinghamReactions);
    if (!rxn && usar_dHf && tipo == domain::PropertyDiagramType::Enthalpy_H) {
        return std::string("ΔHf°(T) [") + u_e + " " + suf + "]";
    }
    switch (tipo) {
    case domain::PropertyDiagramType::Enthalpy_H:
        return std::string(rxn ? "ΔH°(T) [" : "H°(T) [") + u_e + " " + suf + "]";
    case domain::PropertyDiagramType::HeatContent_dH298:
        return std::string(rxn ? "ΔH°(T) - ΔH°(298) [" : "H°(T) - H°(298) [") + u_e + " " + suf + "]";
    case domain::PropertyDiagramType::Entropy_S:
        return std::string(rxn ? "ΔS°(T) [" : "S°(T) [") + u_s + "]";
    case domain::PropertyDiagramType::HeatCapacity_Cp:
        return std::string(rxn ? "ΔCp°(T) [" : "Cp°(T) [") + u_s + "]";
    case domain::PropertyDiagramType::Gibbs_G:
        return std::string("G°(T) = H° - T·S° [") + u_e + " " + suf + "]";
    case domain::PropertyDiagramType::DeltaG_Ellingham:
        return std::string(rxn ? "ΔG°(T) Ellingham [" : "ΔGf°(T) [") + u_e + " " + suf + "]";
    case domain::PropertyDiagramType::Log10K:
        return rxn ? "log10 K(T) [-]" : "log10 Kf(T) [-]";
    }
    return std::string("ΔG°(T) [") + u_e + "]";
}

double evaluar_punto_curva(const domain::CurvaDiagramaTermo& c,
                           size_t idx,
                           domain::PropertyDiagramMode modo,
                           domain::PropertyDiagramType tipo,
                           bool en_kcal,
                           bool especie_usar_dHf) {
    if (idx >= c.points.size()) return 0.0;
    if (modo == domain::PropertyDiagramMode::SpeciesProperties && especie_usar_dHf &&
        tipo == domain::PropertyDiagramType::Enthalpy_H) {
        return c.points[idx].dHf * (en_kcal ? (1.0 / 4.184) : 1.0);
    }
    return c.evaluar_propiedad(tipo, idx, en_kcal);
}

} // namespace

void PanelDiagramasTermo::cargar_familia_preset(domain::EllinghamPresetFamily familia) {
    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());
    familia_preset_ = static_cast<int>(familia);
    modo_ = domain::PropertyDiagramMode::EllinghamReactions;
    curvas_eliminadas_.clear();
    visibilidad_curvas_.clear();

    if (T_max_ <= T_min_ + 10.0) {
        T_min_ = 298.15;
        T_max_ = 2273.15;
    }
    resultado_ = motor_.generar_familia_ellingham(
        repo, familia, T_min_, T_max_, std::max(num_pasos_, 20), usar_envolvente_fases_);
    base_norm_ = resultado_.basis;
    resultado_.diagram_type = tipo_propiedad_;

    for (const auto& c : resultado_.curves) {
        visibilidad_curvas_[c.label] = c.visible;
    }
    for (const auto& eq : ecuaciones_personalizadas_) {
        auto curva = motor_.calcular_curva_ecuacion_texto(
            repo, eq.ecuacion, base_norm_, T_min_, T_max_, std::max(num_pasos_, 20), usar_envolvente_fases_);
        if (!curva.points.empty()) {
            curva.visible = eq.visible;
            resultado_.curves.push_back(std::move(curva));
        }
    }

    actualizar_intersecciones();
    nomo_resultado_ = motor_.evaluar_escala_nomografica(repo, nomo_T_K_, nomo_dG_kJ_);
    calculado_ = !resultado_.curves.empty();
    inicializado_ = true;
}

void PanelDiagramasTermo::actualizar_intersecciones() {
    resultado_.diagram_type = tipo_propiedad_;
    resultado_.intersections = motor_.encontrar_intersecciones(resultado_.curves, tipo_propiedad_);
}

void PanelDiagramasTermo::recalcular() {
    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());

    if (T_min_ < 100.0) T_min_ = 100.0;
    if (T_max_ <= T_min_ + 10.0) T_max_ = T_min_ + 100.0;
    num_pasos_ = std::clamp(num_pasos_, 20, 600);

    for (const auto& c : resultado_.curves) {
        visibilidad_curvas_[c.label] = c.visible;
    }

    if (modo_ == domain::PropertyDiagramMode::EllinghamReactions) {
        auto familia = static_cast<domain::EllinghamPresetFamily>(std::clamp(familia_preset_, 0, 4));
        auto base_res = motor_.generar_familia_ellingham(
            repo, familia, T_min_, T_max_, num_pasos_, usar_envolvente_fases_);

        resultado_.mode = domain::PropertyDiagramMode::EllinghamReactions;
        resultado_.diagram_type = tipo_propiedad_;
        resultado_.basis = base_norm_;
        resultado_.T_min_K = T_min_;
        resultado_.T_max_K = T_max_;
        resultado_.curves.clear();
        resultado_.iso_pO2_lines = std::move(base_res.iso_pO2_lines);
        resultado_.iso_CO_CO2_lines = std::move(base_res.iso_CO_CO2_lines);
        resultado_.iso_H2_H2O_lines = std::move(base_res.iso_H2_H2O_lines);
        resultado_.summary_log = base_res.summary_log;

        for (auto& c : base_res.curves) {
            if (std::find(curvas_eliminadas_.begin(), curvas_eliminadas_.end(), c.label) !=
                curvas_eliminadas_.end()) {
                continue;
            }
            if (base_norm_ != base_res.basis && !c.equation_normalized.empty()) {
                auto recalc_c = motor_.calcular_curva_ecuacion_texto(
                    repo, c.label, base_norm_, T_min_, T_max_, num_pasos_, usar_envolvente_fases_);
                if (!recalc_c.points.empty()) {
                    c = std::move(recalc_c);
                }
            }
            auto it_vis = visibilidad_curvas_.find(c.label);
            if (it_vis != visibilidad_curvas_.end()) {
                c.visible = it_vis->second;
            }
            resultado_.curves.push_back(std::move(c));
        }

        for (const auto& eq : ecuaciones_personalizadas_) {
            auto curva = motor_.calcular_curva_ecuacion_texto(
                repo, eq.ecuacion, base_norm_, T_min_, T_max_, num_pasos_, usar_envolvente_fases_);
            if (!curva.points.empty()) {
                curva.visible = eq.visible;
                resultado_.curves.push_back(std::move(curva));
            }
        }

        if (resultado_.iso_pO2_lines.empty()) {
            motor_.generar_lineas_isopotenciales(
                repo, T_min_, T_max_,
                resultado_.iso_pO2_lines, resultado_.iso_CO_CO2_lines, resultado_.iso_H2_H2O_lines);
        }
    } else {
        if (especies_activas_.empty()) {
            especies_activas_ = {
                {"Fe", 1.0, true},
                {"FeO", 1.0, true},
                {"Fe3O4", 1.0, true},
                {"Fe2O3", 1.0, true},
                {"CO(g)", 1.0, true},
                {"CO2(g)", 1.0, true},
            };
        }
        resultado_.mode = domain::PropertyDiagramMode::SpeciesProperties;
        resultado_.diagram_type = tipo_propiedad_;
        resultado_.basis = domain::EllinghamNormalizationBasis::PerMoleReaction;
        resultado_.T_min_K = T_min_;
        resultado_.T_max_K = T_max_;
        resultado_.curves.clear();

        for (const auto& item : especies_activas_) {
            auto curva = motor_.calcular_curva_especie(
                repo, item.formula, T_min_, T_max_, num_pasos_, usar_envolvente_fases_, item.multiplicador);
            if (!curva.points.empty()) {
                curva.visible = item.activa;
                resultado_.curves.push_back(std::move(curva));
            }
        }
    }

    actualizar_intersecciones();
    nomo_resultado_ = motor_.evaluar_escala_nomografica(repo, nomo_T_K_, nomo_dG_kJ_);
    calculado_ = !resultado_.curves.empty();
    inicializado_ = true;
}

void PanelDiagramasTermo::renderizar() {
    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());

    if (!inicializado_) {
        cargar_familia_preset(domain::EllinghamPresetFamily::MetallurgicalOxides);
    }

    // ── Barra Superior: Selector de Modo y Familias Preconfiguradas de Ellingham ──
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(8.0f, 5.0f));
    int modo_int = (modo_ == domain::PropertyDiagramMode::EllinghamReactions) ? 1 : 0;
    ImGui::TextColored(AdaptiveColor(0.2f, 0.55f, 0.9f, 1.0f), "Modo de Trabajo:");
    ImGui::SameLine();
    if (ImGui::RadioButton("Diagramas de Reacción y Ellingham (ΔG°, ΔH°, ΔS°, ΔCp°, log10 K)", modo_int == 1)) {
        modo_ = domain::PropertyDiagramMode::EllinghamReactions;
        tipo_propiedad_ = domain::PropertyDiagramType::DeltaG_Ellingham;
        especie_usar_dHf_ = false;
        recalcular();
    }
    ImGui::SameLine();
    if (ImGui::RadioButton("Propiedades de Especies y Polimorfos (H°, H°-H°298, S°, Cp°, G°, ΔGf°, ΔHf°)", modo_int == 0)) {
        modo_ = domain::PropertyDiagramMode::SpeciesProperties;
        recalcular();
    }

    if (modo_ == domain::PropertyDiagramMode::EllinghamReactions) {
        ImGui::TextColored(AdaptiveColor(0.85f, 0.45f, 0.15f, 1.0f), "Familias HSC Ellingham (1 Clic):");
        ImGui::SameLine();
        struct PresetBtn {
            const char* label;
            domain::EllinghamPresetFamily fam;
        };
        static const PresetBtn kPresets[] = {
            {"1. Óxidos y Reductores C/CO/H2 (1 mol O2)", domain::EllinghamPresetFamily::MetallurgicalOxides},
            {"2. Tostación de Sulfuros (1 mol S2)", domain::EllinghamPresetFamily::SulfideRoasting},
            {"3. Cloración Metalúrgica (1 mol Cl2)", domain::EllinghamPresetFamily::ChlorinationMetallurgy},
            {"4. Calcinación de Carbonatos (1 mol CO2)", domain::EllinghamPresetFamily::CarbonateCalcination},
            {"5. Descomposición de Sulfatos (1 mol SO3)", domain::EllinghamPresetFamily::SulfateDecomposition},
        };
        for (int i = 0; i < 5; ++i) {
            if (i > 0) ImGui::SameLine();
            const bool activo = (familia_preset_ == i);
            if (activo) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.46f, 0.78f, 0.90f));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
            }
            if (ImGui::SmallButton(kPresets[i].label)) {
                cargar_familia_preset(kPresets[i].fam);
                mensaje_estado_ = std::string("Familia cargada: ") + kPresets[i].label;
            }
            if (activo) {
                ImGui::PopStyleColor(2);
            }
        }
    }
    ImGui::PopStyleVar();
    ImGui::Separator();

    // ── Layout Principal en 2 Columnas ──
    ImGui::Columns(2, "EllinghamWorkstationCols", true);
    ImGui::SetColumnWidth(0, 415.0f);

    // ═══════════════════════════════════════════════════════════════════════
    // COLUMNA IZQUIERDA: Propiedad, Unidades, Escalas Nomográficas y Curvas
    // ═══════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("LeftControlsEllingham", ImVec2(0, 0), true);

    ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f), "1. Propiedad Termodinámica y Normalización");
    if (modo_ == domain::PropertyDiagramMode::EllinghamReactions) {
        int prop_idx = 0;
        switch (tipo_propiedad_) {
        case domain::PropertyDiagramType::DeltaG_Ellingham: prop_idx = 0; break;
        case domain::PropertyDiagramType::Enthalpy_H:       prop_idx = 1; break;
        case domain::PropertyDiagramType::HeatContent_dH298:prop_idx = 2; break;
        case domain::PropertyDiagramType::Entropy_S:        prop_idx = 3; break;
        case domain::PropertyDiagramType::HeatCapacity_Cp:  prop_idx = 4; break;
        case domain::PropertyDiagramType::Log10K:           prop_idx = 5; break;
        case domain::PropertyDiagramType::Gibbs_G:          prop_idx = 0; break;
        }
        ImGui::SetNextItemWidth(-1);
        const char* items_rxn =
            "ΔG°(T) — Energía Libre de Gibbs (Diagrama de Ellingham)\0"
            "ΔH°(T) — Entalpía de Reacción\0"
            "ΔH°(T) - ΔH°(298 K) — Cambio de Contenido Calórico\0"
            "ΔS°(T) — Entropía de Reacción\0"
            "ΔCp°(T) — Capacidad Calorífica de Reacción\0"
            "log10 K(T) — Constante de Equilibrio Termodinámico\0";
        if (ImGui::Combo("##PropRxn", &prop_idx, items_rxn)) {
            static const domain::PropertyDiagramType kMapRxn[] = {
                domain::PropertyDiagramType::DeltaG_Ellingham,
                domain::PropertyDiagramType::Enthalpy_H,
                domain::PropertyDiagramType::HeatContent_dH298,
                domain::PropertyDiagramType::Entropy_S,
                domain::PropertyDiagramType::HeatCapacity_Cp,
                domain::PropertyDiagramType::Log10K,
            };
            tipo_propiedad_ = kMapRxn[prop_idx];
            actualizar_intersecciones();
        }

        int basis_idx = static_cast<int>(base_norm_);
        ImGui::SetNextItemWidth(-1);
        const char* items_basis =
            "Base: Por 1 mol O2(g) (Óxidos / Reducción)\0"
            "Base: Por 1 mol S2(g) (Sulfuros / Tostación)\0"
            "Base: Por 1 mol Cl2(g) (Cloración Metalúrgica)\0"
            "Base: Por 1 mol CO2(g) (Calcinación Carbonatos)\0"
            "Base: Por 1 mol SO3(g) (Descomposición Sulfatos)\0"
            "Base: Por 1 mol H2O(g) (Hidratación / Vapor)\0"
            "Base: Por 1 mol de reacción escrita (Sin reescalar)\0";
        if (ImGui::Combo("##BasisNorm", &basis_idx, items_basis)) {
            base_norm_ = static_cast<domain::EllinghamNormalizationBasis>(basis_idx);
            recalcular();
        }
    } else {
        int esp_prop_idx = 0;
        if (especie_usar_dHf_ && tipo_propiedad_ == domain::PropertyDiagramType::Enthalpy_H) {
            esp_prop_idx = 6;
        } else {
            switch (tipo_propiedad_) {
            case domain::PropertyDiagramType::Enthalpy_H:        esp_prop_idx = 0; break;
            case domain::PropertyDiagramType::HeatContent_dH298: esp_prop_idx = 1; break;
            case domain::PropertyDiagramType::Entropy_S:         esp_prop_idx = 2; break;
            case domain::PropertyDiagramType::HeatCapacity_Cp:   esp_prop_idx = 3; break;
            case domain::PropertyDiagramType::Gibbs_G:           esp_prop_idx = 4; break;
            case domain::PropertyDiagramType::DeltaG_Ellingham:  esp_prop_idx = 5; break;
            case domain::PropertyDiagramType::Log10K:            esp_prop_idx = 7; break;
            }
        }
        ImGui::SetNextItemWidth(-1);
        const char* items_esp =
            "H°(T) — Entalpía Estándar Absoluta\0"
            "H°(T) - H°(298.15 K) — Entalpía Sensible\0"
            "S°(T) — Entropía Estándar Absoluta\0"
            "Cp°(T) — Capacidad Calorífica Molar\0"
            "G°(T) = H°(T) - T·S°(T) — Función de Gibbs\0"
            "ΔGf°(T) — Energía Libre de Formación desde Elementos\0"
            "ΔHf°(T) — Entalpía de Formación desde Elementos\0"
            "log10 Kf(T) — Constante de Formación\0";
        if (ImGui::Combo("##PropEsp", &esp_prop_idx, items_esp)) {
            especie_usar_dHf_ = (esp_prop_idx == 6);
            static const domain::PropertyDiagramType kMapEsp[] = {
                domain::PropertyDiagramType::Enthalpy_H,
                domain::PropertyDiagramType::HeatContent_dH298,
                domain::PropertyDiagramType::Entropy_S,
                domain::PropertyDiagramType::HeatCapacity_Cp,
                domain::PropertyDiagramType::Gibbs_G,
                domain::PropertyDiagramType::DeltaG_Ellingham,
                domain::PropertyDiagramType::Enthalpy_H,
                domain::PropertyDiagramType::Log10K,
            };
            tipo_propiedad_ = kMapEsp[esp_prop_idx];
            actualizar_intersecciones();
        }
    }

    if (ImGui::Checkbox("Envolvente de Polimorfos (s -> l -> g automático)", &usar_envolvente_fases_)) {
        recalcular();
    }

    ImGui::Separator();
    ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f), "2. Rango Térmico, Unidades y Abanicos Nomográficos");

    bool rango_cambiado = false;
    ImGui::PushItemWidth(105.0f);
    rango_cambiado |= ImGui::InputDouble("T mín (K)", &T_min_, 0.0, 0.0, "%.1f");
    ImGui::SameLine();
    rango_cambiado |= ImGui::InputDouble("T máx (K)", &T_max_, 0.0, 0.0, "%.1f");
    ImGui::PopItemWidth();
    ImGui::SetNextItemWidth(120.0f);
    rango_cambiado |= ImGui::SliderInt("Pasos T", &num_pasos_, 40, 300);
    ImGui::SameLine();
    if (ImGui::Button("Recalcular", ImVec2(-1, 0)) || rango_cambiado) {
        recalcular();
    }

    ImGui::Text("Eje T:");
    ImGui::SameLine();
    if (ImGui::RadioButton("Kelvin (K)", !temp_en_celsius_)) temp_en_celsius_ = false;
    ImGui::SameLine();
    if (ImGui::RadioButton("Celsius (°C)", temp_en_celsius_)) temp_en_celsius_ = true;

    ImGui::Text("Energía:");
    ImGui::SameLine();
    if (ImGui::RadioButton("kJ / J", !energia_en_kcal_)) energia_en_kcal_ = false;
    ImGui::SameLine();
    if (ImGui::RadioButton("kcal / cal", energia_en_kcal_)) energia_en_kcal_ = true;

    if (modo_ == domain::PropertyDiagramMode::EllinghamReactions) {
        ImGui::Checkbox("Mostrar abanico log10(pO2)", &mostrar_iso_pO2_);
        ImGui::Checkbox("Mostrar abanico log10(pCO / pCO2)", &mostrar_iso_CO_CO2_);
        ImGui::Checkbox("Mostrar abanico log10(pH2 / pH2O)", &mostrar_iso_H2_H2O_);
    }
    ImGui::Checkbox("Marcar intersecciones (T inv.) y transiciones (Tm / Tb)", &mostrar_marcadores_);

    ImGui::Separator();

    // ── Constructor de Reacciones o Selector de Especies ──
    if (modo_ == domain::PropertyDiagramMode::EllinghamReactions) {
        ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f), "3. Constructor de Reacciones y Curvas Activas");
        ImGui::SetNextItemWidth(-105.0f);
        bool enter_eq = ImGui::InputTextWithHint(
            "##NuevaEcuacion", "Ej. 2 Fe + O2(g) = 2 FeO", nueva_ecuacion_buf_, sizeof(nueva_ecuacion_buf_),
            ImGuiInputTextFlags_EnterReturnsTrue);
        ImGui::SameLine();
        if ((ImGui::Button("+ Ecuación", ImVec2(-1, 0)) || enter_eq) && std::strlen(nueva_ecuacion_buf_) > 0) {
            std::string eq_str = trim_str(nueva_ecuacion_buf_);
            auto nueva_c = motor_.calcular_curva_ecuacion_texto(
                repo, eq_str, base_norm_, T_min_, T_max_, num_pasos_, usar_envolvente_fases_);
            if (!nueva_c.points.empty()) {
                ecuaciones_personalizadas_.push_back({eq_str, true});
                resultado_.curves.push_back(std::move(nueva_c));
                actualizar_intersecciones();
                mensaje_estado_ = "Curva agregada: " + eq_str;
            } else {
                mensaje_estado_ = "No se pudo evaluar la reacción: revise fórmulas/fases.";
            }
        }

        // Botones rápidos de reductores clásicos
        ImGui::TextDisabled("Reductores rápidos:");
        ImGui::SameLine();
        const char* kQuickEqs[] = {
            "2 C + O2(g) = 2 CO(g)",
            "2 CO(g) + O2(g) = 2 CO2(g)",
            "2 H2(g) + O2(g) = 2 H2O(g)",
        };
        const char* kQuickLabels[] = {"+ Boudouard/C->CO", "+ CO->CO2", "+ H2->H2O"};
        for (int q = 0; q < 3; ++q) {
            if (q > 0) ImGui::SameLine();
            if (ImGui::SmallButton(kQuickLabels[q])) {
                copiar_texto(nueva_ecuacion_buf_, sizeof(nueva_ecuacion_buf_), kQuickEqs[q]);
                auto nueva_c = motor_.calcular_curva_ecuacion_texto(
                    repo, kQuickEqs[q], base_norm_, T_min_, T_max_, num_pasos_, usar_envolvente_fases_);
                if (!nueva_c.points.empty()) {
                    ecuaciones_personalizadas_.push_back({kQuickEqs[q], true});
                    resultado_.curves.push_back(std::move(nueva_c));
                    actualizar_intersecciones();
                }
            }
        }

        if (!mensaje_estado_.empty()) {
            ImGui::TextColored(AdaptiveColor(0.3f, 0.75f, 0.4f, 1.0f), "%s", mensaje_estado_.c_str());
        }

        if (ImGui::SmallButton("Mostrar Todas")) {
            for (auto& c : resultado_.curves) c.visible = true;
            for (auto& eq : ecuaciones_personalizadas_) eq.visible = true;
            actualizar_intersecciones();
        }
        ImGui::SameLine();
        if (ImGui::SmallButton("Ocultar Todas")) {
            for (auto& c : resultado_.curves) c.visible = false;
            for (auto& eq : ecuaciones_personalizadas_) eq.visible = false;
            actualizar_intersecciones();
        }
        ImGui::SameLine();
        if (ImGui::SmallButton("Restaurar Familia")) {
            ecuaciones_personalizadas_.clear();
            cargar_familia_preset(static_cast<domain::EllinghamPresetFamily>(std::clamp(familia_preset_, 0, 4)));
        }

        if (ImGui::BeginTable("TablaCurvasEllingham", 4,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0, -1))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Ver", ImGuiTableColumnFlags_WidthFixed, 34.0f);
            ImGui::TableSetupColumn("Reacción / Ecuación Normalizada", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Tr.", ImGuiTableColumnFlags_WidthFixed, 32.0f);
            ImGui::TableSetupColumn("X", ImGuiTableColumnFlags_WidthFixed, 28.0f);
            ImGui::TableHeadersRow();

            int idx_borrar = -1;
            for (int i = 0; i < static_cast<int>(resultado_.curves.size()); ++i) {
                auto& c = resultado_.curves[static_cast<size_t>(i)];
                ImGui::PushID(i);
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                if (ImGui::Checkbox("##vis", &c.visible)) {
                    visibilidad_curvas_[c.label] = c.visible;
                    for (auto& eq : ecuaciones_personalizadas_) {
                        if (eq.ecuacion == c.label) eq.visible = c.visible;
                    }
                    actualizar_intersecciones();
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::TextUnformatted(c.label.c_str());
                if (ImGui::IsItemHovered() && !c.equation_normalized.empty()) {
                    ImGui::SetTooltip("Normalizada: %s (factor = %.3g)", c.equation_normalized.c_str(), c.norm_factor);
                }
                ImGui::TableSetColumnIndex(2);
                ImGui::Text("%zu", c.phase_transitions.size());
                ImGui::TableSetColumnIndex(3);
                if (ImGui::SmallButton("X")) {
                    idx_borrar = i;
                }
                ImGui::PopID();
            }
            if (idx_borrar >= 0 && idx_borrar < static_cast<int>(resultado_.curves.size())) {
                const std::string lbl = resultado_.curves[static_cast<size_t>(idx_borrar)].label;
                curvas_eliminadas_.push_back(lbl);
                ecuaciones_personalizadas_.erase(
                    std::remove_if(ecuaciones_personalizadas_.begin(), ecuaciones_personalizadas_.end(),
                                   [&](const EcuacionPersonalizada& e) { return e.ecuacion == lbl; }),
                    ecuaciones_personalizadas_.end());
                resultado_.curves.erase(resultado_.curves.begin() + idx_borrar);
                actualizar_intersecciones();
            }
            ImGui::EndTable();
        }
    } else {
        ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f), "3. Selector de Especies y Multiplicadores");
        ImGui::PushItemWidth(135.0f);
        ImGui::InputTextWithHint("##EspFormula", "Fórmula (ej. Fe2O3)", nueva_especie_buf_, sizeof(nueva_especie_buf_));
        ImGui::SameLine();
        ImGui::SetNextItemWidth(75.0f);
        ImGui::InputDouble("Mol##Mult", &nuevo_mult_especie_, 0.0, 0.0, "%.2f");
        ImGui::PopItemWidth();
        ImGui::SameLine();
        if (ImGui::Button("+ Agregar", ImVec2(-1, 0)) && std::strlen(nueva_especie_buf_) > 0) {
            std::string f_str = trim_str(nueva_especie_buf_);
            double m = (std::abs(nuevo_mult_especie_) > 1e-6) ? nuevo_mult_especie_ : 1.0;
            especies_activas_.push_back({f_str, m, true});
            recalcular();
        }

        if (ImGui::Button("Examinar Base de Datos Termoquímica...", ImVec2(-1, 0))) {
            show_select_species_modal_ = true;
            ImGui::OpenPopup("Catálogo de Especies HSC");
        }

        if (ImGui::BeginTable("TablaEspeciesDiagrama", 5,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0, -1))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Ver", ImGuiTableColumnFlags_WidthFixed, 34.0f);
            ImGui::TableSetupColumn("Especie", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("Mult.", ImGuiTableColumnFlags_WidthFixed, 68.0f);
            ImGui::TableSetupColumn("Tr.", ImGuiTableColumnFlags_WidthFixed, 32.0f);
            ImGui::TableSetupColumn("X", ImGuiTableColumnFlags_WidthFixed, 28.0f);
            ImGui::TableHeadersRow();

            int esp_borrar = -1;
            for (int i = 0; i < static_cast<int>(especies_activas_.size()); ++i) {
                auto& item = especies_activas_[static_cast<size_t>(i)];
                ImGui::PushID(i);
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                if (ImGui::Checkbox("##act", &item.activa)) {
                    if (static_cast<size_t>(i) < resultado_.curves.size()) {
                        resultado_.curves[static_cast<size_t>(i)].visible = item.activa;
                        actualizar_intersecciones();
                    } else {
                        recalcular();
                    }
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::TextUnformatted(item.formula.c_str());
                ImGui::TableSetColumnIndex(2);
                ImGui::SetNextItemWidth(-1);
                if (ImGui::InputDouble("##m", &item.multiplicador, 0.0, 0.0, "%.2f")) {
                    if (std::abs(item.multiplicador) < 1e-6) item.multiplicador = 1.0;
                    recalcular();
                }
                ImGui::TableSetColumnIndex(3);
                size_t n_tr = (static_cast<size_t>(i) < resultado_.curves.size())
                                  ? resultado_.curves[static_cast<size_t>(i)].phase_transitions.size()
                                  : 0;
                ImGui::Text("%zu", n_tr);
                ImGui::TableSetColumnIndex(4);
                if (ImGui::SmallButton("X")) {
                    esp_borrar = i;
                }
                ImGui::PopID();
            }
            if (esp_borrar >= 0 && esp_borrar < static_cast<int>(especies_activas_.size())) {
                especies_activas_.erase(especies_activas_.begin() + esp_borrar);
                recalcular();
            }
            ImGui::EndTable();
        }
    }

    ImGui::EndChild();

    ImGui::NextColumn();

    // ═══════════════════════════════════════════════════════════════════════
    // COLUMNA DERECHA: 4 Pestañas de Visualización
    // ═══════════════════════════════════════════════════════════════════════
    if (ImGui::BeginTabBar("TabsDiagramasTermo")) {
        // ───────────────────────────────────────────────────────────────────
        // PESTAÑA 1: Gráfico Termoquímico / Ellingham (ImPlot)
        // ───────────────────────────────────────────────────────────────────
        if (ImGui::BeginTabItem("1. Gráfico Termoquímico / Ellingham")) {
            const double offset_T = temp_en_celsius_ ? -273.15 : 0.0;
            const double factor_E = energia_en_kcal_ ? (1.0 / 4.184) : 1.0;
            const std::string x_label = temp_en_celsius_ ? "Temperatura (°C)" : "Temperatura (K)";
            const std::string y_label = etiqueta_eje_y(
                modo_, tipo_propiedad_, base_norm_, energia_en_kcal_, especie_usar_dHf_);

            bool cursor_en_grafico = false;
            double cursor_T_K = 1000.0;
            double cursor_Y_plot = 0.0;
            double cursor_dG_kJ = 0.0;

            if (ImPlot::BeginPlot("##EllinghamMainPlot", ImVec2(-1, -58.0f))) {
                ImPlot::SetupAxes(x_label.c_str(), y_label.c_str());
                ImPlot::SetupAxisLimits(ImAxis_X1, T_min_ + offset_T, T_max_ + offset_T, ImGuiCond_Once);
                ImPlot::SetupLegend(ImPlotLocation_NorthWest, ImPlotLegendFlags_Outside);

                // 1) Abanicos Isopotenciales de Richardson-Jeffes (en modo Ellingham Delta G°)
                const bool mostrar_abanicos =
                    (modo_ == domain::PropertyDiagramMode::EllinghamReactions &&
                     tipo_propiedad_ == domain::PropertyDiagramType::DeltaG_Ellingham);

                auto dibujar_abanico = [&](const std::vector<domain::LineaIsoPotencial>& lineas,
                                           const ImVec4& color,
                                           const char* prefijo_id) {
                    std::vector<double> xs;
                    std::vector<double> ys;
                    for (size_t idx = 0; idx < lineas.size(); ++idx) {
                        const auto& l = lineas[idx];
                        if (l.T_K.size() < 2 || l.T_K.size() != l.delta_G_kJ.size()) continue;
                        xs.resize(l.T_K.size());
                        ys.resize(l.delta_G_kJ.size());
                        for (size_t k = 0; k < l.T_K.size(); ++k) {
                            xs[k] = l.T_K[k] + offset_T;
                            ys[k] = l.delta_G_kJ[k] * factor_E;
                        }
                        ImPlot::SetNextLineStyle(color, 1.0f);
                        std::string plot_id = std::string("##") + prefijo_id + "_" + std::to_string(idx);
                        ImPlot::PlotLine(plot_id.c_str(), xs.data(), ys.data(), static_cast<int>(xs.size()));
                        ImPlot::PlotText(l.label.c_str(), xs.back(), ys.back());
                    }
                };

                if (mostrar_abanicos) {
                    if (mostrar_iso_pO2_) {
                        dibujar_abanico(resultado_.iso_pO2_lines, ImVec4(0.45f, 0.62f, 0.85f, 0.42f), "iso_pO2");
                    }
                    if (mostrar_iso_CO_CO2_) {
                        dibujar_abanico(resultado_.iso_CO_CO2_lines, ImVec4(0.88f, 0.62f, 0.25f, 0.45f), "iso_CO");
                    }
                    if (mostrar_iso_H2_H2O_) {
                        dibujar_abanico(resultado_.iso_H2_H2O_lines, ImVec4(0.25f, 0.78f, 0.72f, 0.45f), "iso_H2");
                    }
                }

                // 2) Curvas Termoquímicas / Ellingham Visibles
                std::vector<double> tr_x;
                std::vector<double> tr_y;
                for (const auto& c : resultado_.curves) {
                    if (!c.visible || c.points.empty()) continue;
                    std::vector<double> xs(c.points.size());
                    std::vector<double> ys(c.points.size());
                    for (size_t k = 0; k < c.points.size(); ++k) {
                        xs[k] = c.points[k].T + offset_T;
                        ys[k] = evaluar_punto_curva(
                            c, k, modo_, tipo_propiedad_, energia_en_kcal_, especie_usar_dHf_);
                    }
                    ImPlot::SetNextLineStyle(IMPLOT_AUTO_COL, 2.25f);
                    ImPlot::PlotLine(c.label.c_str(), xs.data(), ys.data(), static_cast<int>(xs.size()));

                    if (mostrar_marcadores_ && !c.phase_transitions.empty()) {
                        for (const auto& tr : c.phase_transitions) {
                            if (tr.T_K < T_min_ || tr.T_K > T_max_) continue;
                            // Interpolar valor Y en T_K de transición
                            size_t best_k = 0;
                            double min_dT = 1e18;
                            for (size_t k = 0; k < c.points.size(); ++k) {
                                double dT = std::abs(c.points[k].T - tr.T_K);
                                if (dT < min_dT) {
                                    min_dT = dT;
                                    best_k = k;
                                }
                            }
                            tr_x.push_back(tr.T_K + offset_T);
                            tr_y.push_back(evaluar_punto_curva(
                                c, best_k, modo_, tipo_propiedad_, energia_en_kcal_, especie_usar_dHf_));
                        }
                    }
                }

                // 3) Marcadores de Transición de Fase (Tm / Tb) e Intersecciones (T de inversión)
                if (mostrar_marcadores_) {
                    if (!tr_x.empty()) {
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Diamond, 6.0f,
                                                   ImVec4(0.95f, 0.75f, 0.15f, 1.0f), 1.5f,
                                                   ImVec4(0.2f, 0.2f, 0.2f, 1.0f));
                        ImPlot::PlotScatter("Transiciones Fase (Tm/Tb)", tr_x.data(), tr_y.data(),
                                            static_cast<int>(tr_x.size()));
                    }
                    if (!resultado_.intersections.empty() && !especie_usar_dHf_) {
                        std::vector<double> ix;
                        std::vector<double> iy;
                        const bool es_unidades_energia =
                            (tipo_propiedad_ != domain::PropertyDiagramType::Log10K);
                        const double f_int = es_unidades_energia ? factor_E : 1.0;
                        for (const auto& inter : resultado_.intersections) {
                            ix.push_back(inter.T_K + offset_T);
                            iy.push_back(inter.value_Y * f_int);
                        }
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 6.5f,
                                                   ImVec4(0.92f, 0.22f, 0.25f, 1.0f), 1.8f,
                                                   ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
                        ImPlot::PlotScatter("Inversiones / Intersecciones", ix.data(), iy.data(),
                                            static_cast<int>(ix.size()));
                    }
                }

                if (ImPlot::IsPlotHovered()) {
                    ImPlotPoint mp = ImPlot::GetPlotMousePos();
                    cursor_en_grafico = true;
                    cursor_T_K = std::max(100.0, mp.x - offset_T);
                    cursor_Y_plot = mp.y;
                    cursor_dG_kJ = energia_en_kcal_ ? (mp.y * 4.184) : mp.y;
                }

                ImPlot::EndPlot();
            }

            // Barra de lectura nomográfica en tiempo real bajo el gráfico
            if (cursor_en_grafico &&
                modo_ == domain::PropertyDiagramMode::EllinghamReactions &&
                tipo_propiedad_ == domain::PropertyDiagramType::DeltaG_Ellingham) {
                auto nomo_cur = motor_.evaluar_escala_nomografica(repo, cursor_T_K, cursor_dG_kJ);
                ImGui::TextColored(
                    AdaptiveColor(0.15f, 0.65f, 0.95f, 1.0f),
                    "Nomógrafo Richardson-Jeffes en Cursor:  T = %.1f K (%.1f °C)  |  ΔG° = %.2f %s",
                    cursor_T_K, cursor_T_K - 273.15, cursor_Y_plot,
                    energia_en_kcal_ ? "kcal/mol O2" : "kJ/mol O2");
                ImGui::Text(
                    "   log10(pO2) = %.2f (pO2 = %.3e atm)   |   pCO/pCO2 = %.3e (log10 = %.2f)   |   pH2/pH2O = %.3e (log10 = %.2f)",
                    nomo_cur.log10_pO2, nomo_cur.pO2_atm,
                    nomo_cur.ratio_CO_CO2, nomo_cur.log10_CO_CO2,
                    nomo_cur.ratio_H2_H2O, nomo_cur.log10_H2_H2O);
            } else if (cursor_en_grafico) {
                ImGui::Text("Cursor: T = %.2f K (%.2f °C)  |  Valor = %.4f  |  Intersecciones activas: %zu",
                            cursor_T_K, cursor_T_K - 273.15, cursor_Y_plot, resultado_.intersections.size());
            } else {
                ImGui::TextDisabled(
                    "Pase el cursor sobre el gráfico de Ellingham para leer en tiempo real las escalas nomográficas pO2, pCO/pCO2 y pH2/pH2O.");
                ImGui::Text("Curvas activas: %zu  |  Intersecciones (T de inversión) detectadas: %zu",
                            resultado_.curves.size(), resultado_.intersections.size());
            }
            ImGui::EndTabItem();
        }

        // ───────────────────────────────────────────────────────────────────
        // PESTAÑA 2: Intersecciones e Inversión Carbotérmica / Metalotérmica
        // ───────────────────────────────────────────────────────────────────
        if (ImGui::BeginTabItem("2. Intersecciones e Inversión Carbotérmica / Metalotérmica")) {
            ImGui::SetNextItemWidth(300.0f);
            ImGui::InputTextWithHint("##FiltroInter", "Filtrar por especie o reacción (ej. Fe, CO, Cr2O3)...",
                                     filtro_intersecciones_, sizeof(filtro_intersecciones_));
            ImGui::SameLine();
            ImGui::Text("Total detectadas: %zu", resultado_.intersections.size());

            std::string f_str = filtro_intersecciones_;
            std::transform(f_str.begin(), f_str.end(), f_str.begin(), ::tolower);

            if (ImGui::BeginTable("TablaInterseccionesEllingham", 9,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                      ImGuiTableFlags_Resizable | ImGuiTableFlags_ScrollY,
                                  ImVec2(0, -1))) {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("Curva A", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Curva B", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("T (K)", ImGuiTableColumnFlags_WidthFixed, 72.0f);
                ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 72.0f);
                ImGui::TableSetupColumn("Valor Y", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                ImGui::TableSetupColumn("log10(pO2)", ImGuiTableColumnFlags_WidthFixed, 82.0f);
                ImGui::TableSetupColumn("pCO / pCO2", ImGuiTableColumnFlags_WidthFixed, 92.0f);
                ImGui::TableSetupColumn("pH2 / pH2O", ImGuiTableColumnFlags_WidthFixed, 92.0f);
                ImGui::TableSetupColumn("Interpretación Termodinámica", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();

                const double f_val =
                    (energia_en_kcal_ && tipo_propiedad_ != domain::PropertyDiagramType::Log10K)
                        ? (1.0 / 4.184)
                        : 1.0;

                for (const auto& inter : resultado_.intersections) {
                    if (!f_str.empty()) {
                        std::string ca = inter.curve_a;
                        std::string cb = inter.curve_b;
                        std::string ip = inter.interpretation;
                        std::transform(ca.begin(), ca.end(), ca.begin(), ::tolower);
                        std::transform(cb.begin(), cb.end(), cb.begin(), ::tolower);
                        std::transform(ip.begin(), ip.end(), ip.begin(), ::tolower);
                        if (ca.find(f_str) == std::string::npos &&
                            cb.find(f_str) == std::string::npos &&
                            ip.find(f_str) == std::string::npos) {
                            continue;
                        }
                    }

                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(inter.curve_a.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(inter.curve_b.c_str());
                    ImGui::TableSetColumnIndex(2); ImGui::Text("%.1f", inter.T_K);
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.1f", inter.T_C);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%.2f", inter.value_Y * f_val);
                    ImGui::TableSetColumnIndex(5); ImGui::Text("%.2f", inter.log10_pO2);
                    ImGui::TableSetColumnIndex(6); ImGui::Text("%.2e", std::pow(10.0, std::clamp(inter.log10_CO_CO2, -99.0, 99.0)));
                    ImGui::TableSetColumnIndex(7); ImGui::Text("%.2e", std::pow(10.0, std::clamp(inter.log10_H2_H2O, -99.0, 99.0)));
                    ImGui::TableSetColumnIndex(8); ImGui::TextUnformatted(inter.interpretation.c_str());
                }
                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }

        // ───────────────────────────────────────────────────────────────────
        // PESTAÑA 3: Nomógrafo Richardson-Jeffes y Transiciones de Fase
        // ───────────────────────────────────────────────────────────────────
        if (ImGui::BeginTabItem("3. Nomógrafo Richardson-Jeffes y Transiciones de Fase")) {
            ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f),
                               "Calculadora Nomográfica de Equilibrio de Gases Reductores (Richardson-Jeffes)");
            bool nomo_changed = false;
            ImGui::PushItemWidth(160.0f);
            nomo_changed |= ImGui::InputDouble("Temperatura T (K)", &nomo_T_K_, 25.0, 100.0, "%.2f");
            ImGui::SameLine();
            ImGui::Text("(%.2f °C)", nomo_T_K_ - 273.15);
            ImGui::SameLine();
            nomo_changed |= ImGui::InputDouble("Potencial ΔG° (kJ/mol O2)", &nomo_dG_kJ_, 10.0, 50.0, "%.2f");
            ImGui::PopItemWidth();

            if (nomo_changed) {
                nomo_T_K_ = std::clamp(nomo_T_K_, 100.0, 5000.0);
                nomo_resultado_ = motor_.evaluar_escala_nomografica(repo, nomo_T_K_, nomo_dG_kJ_);
            }

            if (ImGui::BeginTable("TablaNomografoRJ", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                ImGui::TableSetupColumn("Escala Nomográfica", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Escala Logarítmica (log10)", ImGuiTableColumnFlags_WidthFixed, 170.0f);
                ImGui::TableSetupColumn("Razón / Presión de Equilibrio", ImGuiTableColumnFlags_WidthFixed, 190.0f);
                ImGui::TableSetupColumn("Fracción Gas Binario (% vol)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted("Presión Parcial de Oxígeno pO2(g)");
                ImGui::TableSetColumnIndex(1); ImGui::Text("log10(pO2) = %.3f", nomo_resultado_.log10_pO2);
                ImGui::TableSetColumnIndex(2); ImGui::Text("%.4e atm", nomo_resultado_.pO2_atm);
                ImGui::TableSetColumnIndex(3); ImGui::Text("Potencial químico μ(O2) = %.2f kJ/mol", nomo_resultado_.delta_G_kJ_per_O2);

                const double r_co = nomo_resultado_.ratio_CO_CO2;
                const double pct_co = (r_co / (1.0 + r_co)) * 100.0;
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted("Atmósfera Reductora CO / CO2 (2 CO + O2 = 2 CO2)");
                ImGui::TableSetColumnIndex(1); ImGui::Text("log10(pCO/pCO2) = %.3f", nomo_resultado_.log10_CO_CO2);
                ImGui::TableSetColumnIndex(2); ImGui::Text("pCO / pCO2 = %.4e", r_co);
                ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f%% CO  /  %.2f%% CO2", pct_co, 100.0 - pct_co);

                const double r_h2 = nomo_resultado_.ratio_H2_H2O;
                const double pct_h2 = (r_h2 / (1.0 + r_h2)) * 100.0;
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted("Atmósfera Reductora H2 / H2O (2 H2 + O2 = 2 H2O)");
                ImGui::TableSetColumnIndex(1); ImGui::Text("log10(pH2/pH2O) = %.3f", nomo_resultado_.log10_H2_H2O);
                ImGui::TableSetColumnIndex(2); ImGui::Text("pH2 / pH2O = %.4e", r_h2);
                ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f%% H2  /  %.2f%% H2O(g)", pct_h2, 100.0 - pct_h2);

                ImGui::EndTable();
            }

            ImGui::Spacing();
            ImGui::Separator();
            ImGui::TextColored(AdaptiveColor(0.2f, 0.6f, 0.9f, 1.0f),
                               "Transiciones de Fase y Cambios de Pendiente (Tm, Tb, Transformaciones Alotrópicas)");

            if (ImGui::BeginTable("TablaTransicionesFase", 7,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                  ImVec2(0, -1))) {
                ImGui::TableSetupScrollFreeze(0, 1);
                ImGui::TableSetupColumn("Curva / Reacción", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("Fase Inicial", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                ImGui::TableSetupColumn("Fase Final", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                ImGui::TableSetupColumn("T tr (K)", ImGuiTableColumnFlags_WidthFixed, 85.0f);
                ImGui::TableSetupColumn("T tr (°C)", ImGuiTableColumnFlags_WidthFixed, 85.0f);
                ImGui::TableSetupColumn("ΔH tr (kJ/mol)", ImGuiTableColumnFlags_WidthFixed, 110.0f);
                ImGui::TableSetupColumn("ΔS tr (J/(mol·K))", ImGuiTableColumnFlags_WidthFixed, 120.0f);
                ImGui::TableHeadersRow();

                for (const auto& c : resultado_.curves) {
                    if (!c.visible) continue;
                    for (const auto& tr : c.phase_transitions) {
                        ImGui::TableNextRow();
                        ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(c.label.c_str());
                        ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(tr.from_phase.c_str());
                        ImGui::TableSetColumnIndex(2); ImGui::TextUnformatted(tr.to_phase.c_str());
                        ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f", tr.T_K);
                        ImGui::TableSetColumnIndex(4); ImGui::Text("%.2f", tr.T_K - 273.15);
                        ImGui::TableSetColumnIndex(5); ImGui::Text("%.3f", tr.delta_H_kJ);
                        ImGui::TableSetColumnIndex(6); ImGui::Text("%.3f", tr.delta_S_J_K);
                    }
                }
                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }

        // ───────────────────────────────────────────────────────────────────
        // PESTAÑA 4: Tabla de Datos y Exportación CSV
        // ───────────────────────────────────────────────────────────────────
        if (ImGui::BeginTabItem("4. Tabla de Datos y Exportación CSV")) {
            auto asegurar_csv = [&]() {
                if (csv_exportado_.empty()) {
                    csv_exportado_ = motor_.exportar_csv(
                        resultado_.curves, resultado_.intersections, tipo_propiedad_,
                        temp_en_celsius_, energia_en_kcal_);
                }
            };
            if (ImGui::Button("Generar / Actualizar CSV")) {
                csv_exportado_.clear();
                asegurar_csv();
                mensaje_estado_ = "CSV generado correctamente.";
            }
            ImGui::SameLine();
            if (ImGui::Button("Copiar CSV al Portapapeles")) {
                asegurar_csv();
                ImGui::SetClipboardText(csv_exportado_.c_str());
                mensaje_estado_ = "CSV copiado al portapapeles.";
            }
            ImGui::SameLine();
            ImGui::SetNextItemWidth(220.0f);
            ImGui::InputText("##RutaCSV", ruta_csv_buf_, sizeof(ruta_csv_buf_));
            ImGui::SameLine();
            if (ImGui::Button("Guardar Archivo CSV")) {
                asegurar_csv();
                std::ofstream ofs(ruta_csv_buf_);
                if (ofs) {
                    ofs << csv_exportado_;
                    mensaje_estado_ = std::string("Archivo guardado: ") + ruta_csv_buf_;
                } else {
                    mensaje_estado_ = "Error al abrir archivo para escritura.";
                }
            }

            if (!resultado_.curves.empty()) {
                curva_tabla_idx_ = std::clamp(curva_tabla_idx_, 0, static_cast<int>(resultado_.curves.size()) - 1);
                ImGui::SetNextItemWidth(380.0f);
                if (ImGui::BeginCombo("Inspeccionar Curva",
                                      resultado_.curves[static_cast<size_t>(curva_tabla_idx_)].label.c_str())) {
                    for (int i = 0; i < static_cast<int>(resultado_.curves.size()); ++i) {
                        if (ImGui::Selectable(resultado_.curves[static_cast<size_t>(i)].label.c_str(), curva_tabla_idx_ == i)) {
                            curva_tabla_idx_ = i;
                        }
                    }
                    ImGui::EndCombo();
                }

                const auto& c_sel = resultado_.curves[static_cast<size_t>(curva_tabla_idx_)];
                const double f_e = energia_en_kcal_ ? (1.0 / 4.184) : 1.0;

                if (ImGui::BeginTable("TablaNumericaCurva", 9,
                                      ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                      ImVec2(0, -1))) {
                    ImGui::TableSetupScrollFreeze(0, 1);
                    ImGui::TableSetupColumn("T (K)", ImGuiTableColumnFlags_WidthFixed, 72.0f);
                    ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthFixed, 72.0f);
                    ImGui::TableSetupColumn(energia_en_kcal_ ? "H°/ΔH° (kcal)" : "H°/ΔH° (kJ)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                    ImGui::TableSetupColumn(energia_en_kcal_ ? "dH298 (kcal)" : "dH298 (kJ)", ImGuiTableColumnFlags_WidthFixed, 90.0f);
                    ImGui::TableSetupColumn(energia_en_kcal_ ? "S°/ΔS° (cal/K)" : "S°/ΔS° (J/K)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                    ImGui::TableSetupColumn(energia_en_kcal_ ? "Cp° (cal/K)" : "Cp° (J/K)", ImGuiTableColumnFlags_WidthFixed, 88.0f);
                    ImGui::TableSetupColumn(energia_en_kcal_ ? "ΔG° (kcal)" : "ΔG° (kJ)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
                    ImGui::TableSetupColumn("log10 K", ImGuiTableColumnFlags_WidthFixed, 85.0f);
                    ImGui::TableSetupColumn("Fase / Estado", ImGuiTableColumnFlags_WidthStretch);
                    ImGui::TableHeadersRow();

                    for (const auto& pt : c_sel.points) {
                        ImGui::TableNextRow();
                        ImGui::TableSetColumnIndex(0); ImGui::Text("%.2f", pt.T);
                        ImGui::TableSetColumnIndex(1); ImGui::Text("%.2f", pt.T - 273.15);
                        ImGui::TableSetColumnIndex(2); ImGui::Text("%.3f", pt.H * f_e);
                        ImGui::TableSetColumnIndex(3); ImGui::Text("%.3f", pt.dH298 * f_e);
                        ImGui::TableSetColumnIndex(4); ImGui::Text("%.3f", pt.S * f_e);
                        ImGui::TableSetColumnIndex(5); ImGui::Text("%.3f", pt.Cp * f_e);
                        ImGui::TableSetColumnIndex(6); ImGui::Text("%.3f", pt.dGf * f_e);
                        ImGui::TableSetColumnIndex(7); ImGui::Text("%.3f", pt.log10K);
                        ImGui::TableSetColumnIndex(8); ImGui::TextUnformatted(pt.active_phase.c_str());
                    }
                    ImGui::EndTable();
                }
            }
            ImGui::EndTabItem();
        }

        ImGui::EndTabBar();
    }

    ImGui::Columns(1);

    // ── Modal para explorar y agregar especies del catálogo ──
    ImGui::SetNextWindowSize(ImVec2(640, 480), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Catálogo de Especies HSC", &show_select_species_modal_)) {
        ImGui::InputTextWithHint("##SearchDBEsp", "Filtrar por fórmula o nombre...", search_buffer_, sizeof(search_buffer_));
        std::string s_q = search_buffer_;
        std::transform(s_q.begin(), s_q.end(), s_q.begin(), ::tolower);

        auto db_especies = repo.todas_especies();
        if (ImGui::BeginTable("DBModalTableEsp", 4,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0, -38.0f))) {
            ImGui::TableSetupScrollFreeze(0, 1);
            ImGui::TableSetupColumn("Acción", ImGuiTableColumnFlags_WidthFixed, 68.0f);
            ImGui::TableSetupColumn("Fórmula", ImGuiTableColumnFlags_WidthFixed, 130.0f);
            ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthStretch);
            ImGui::TableSetupColumn("H°298 (kJ/mol)", ImGuiTableColumnFlags_WidthFixed, 110.0f);
            ImGui::TableHeadersRow();

            int shown = 0;
            for (const auto* esp : db_especies) {
                if (!esp) continue;
                std::string nm = esp->nombre;
                std::string fm = esp->formula;
                if (!s_q.empty()) {
                    std::string nml = nm;
                    std::string fml = fm;
                    std::transform(nml.begin(), nml.end(), nml.begin(), ::tolower);
                    std::transform(fml.begin(), fml.end(), fml.begin(), ::tolower);
                    if (nml.find(s_q) == std::string::npos && fml.find(s_q) == std::string::npos) {
                        continue;
                    }
                }
                ImGui::PushID(static_cast<int>(esp->id));
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                if (ImGui::SmallButton("+ Añadir")) {
                    especies_activas_.push_back({esp->formula, 1.0, true});
                    recalcular();
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::TextUnformatted(fm.c_str());
                ImGui::TableSetColumnIndex(2);
                ImGui::TextUnformatted(nm.c_str());
                ImGui::TableSetColumnIndex(3);
                ImGui::Text("%.2f", esp->h_formacion);
                ImGui::PopID();
                if (++shown >= 250 && s_q.empty()) break;
            }
            ImGui::EndTable();
        }
        if (ImGui::Button("Cerrar", ImVec2(120, 0))) {
            show_select_species_modal_ = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
