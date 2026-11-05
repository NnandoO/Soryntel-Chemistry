#include "shared.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>

namespace cma { namespace interfaz {

namespace {

const char* kModalHMB = "Buscar Especie (HMB)";

ImVec4 color_ok()   { return cma::interfaz::AdaptiveColor(0.30f, 0.85f, 0.40f, 1.0f); }
ImVec4 color_aviso(){ return cma::interfaz::AdaptiveColor(0.95f, 0.75f, 0.25f, 1.0f); }
ImVec4 color_cabecera_corriente() {
    return cma::interfaz::AdaptiveColor(0.85f, 0.92f, 1.00f, 1.0f);
}

void popup_autocompletar_especie(const domain::ThermoRepository& repo, char (&formula)[64],
                                 bool& necesita_recalcular, bool& necesita_resolver) {
    if (!ImGui::BeginPopup("##autocompletar")) return;
    std::string q = formula;
    for (char& c : q) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
    }
    int mostradas = 0;
    for (const auto* esp : repo.todas_especies()) {
        if (mostradas >= 8) break;
        std::string nf = esp->nombre + " " + esp->formula;
        for (char& c : nf) {
            if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
        }
        if (!q.empty() && nf.find(q) == std::string::npos) continue;
        char etiqueta[192];
        std::snprintf(etiqueta, sizeof(etiqueta), "%s  (%s)",
                      esp->nombre.c_str(), esp->formula.c_str());
        if (ImGui::Selectable(etiqueta)) {
            copiar_texto(formula, sizeof(formula), esp->formula);
            necesita_recalcular = true;
            necesita_resolver = true;
            ImGui::CloseCurrentPopup();
        }
        ++mostradas;
    }
    if (mostradas == 0) ImGui::TextDisabled("Sin coincidencias");
    ImGui::EndPopup();
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
// Hoja unificada IN1 / OUT1 estilo HSC Heat and Material Balance Calculator
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_pestana_global_hoja(bool es_salida) {
    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());
    auto& grupos = lista(es_salida);
    const auto& motor_grupos = es_salida ? area_.salidas : area_.entradas;

    bool necesita_recalcular = false;
    bool necesita_resolver = false;

    const char* ut = etiqueta_temperatura();
    const char* ue = etiqueta_energia();
    const double ue_kj = motores::factor_energia_kJ(unidad_energia());

    char h_col_a[64], h_temp[64], h_hc[64], h_ht[64], h_ex[64];
    std::snprintf(h_col_a, sizeof(h_col_a), "%s SPECIES (%d)\nFormula",
                  es_salida ? "OUTPUT" : "INPUT", area_activa_ + 1);
    std::snprintf(h_temp, sizeof(h_temp), "Temper.\n%s", ut);
    std::snprintf(h_hc, sizeof(h_hc), "Heat Content\n%s", ue);
    std::snprintf(h_ht, sizeof(h_ht), "Total H\n%s", ue);
    std::snprintf(h_ex, sizeof(h_ex), "Exergy\n%s", ue);

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY |
                                  ImGuiTableFlags_Resizable;

    if (ImGui::BeginTable(es_salida ? "hsc_sheet_out" : "hsc_sheet_in", 10, flags, ImVec2(0, 0))) {
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 28.0f);
        ImGui::TableSetupColumn(h_col_a, ImGuiTableColumnFlags_WidthFixed, 185.0f);
        ImGui::TableSetupColumn(h_temp, ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Pressure\nbar", ImGuiTableColumnFlags_WidthFixed, 68.0f);
        ImGui::TableSetupColumn("Amount\nkmol", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Amount\nkg", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Amount\nNm³", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn(h_hc, ImGuiTableColumnFlags_WidthFixed, 92.0f);
        ImGui::TableSetupColumn(h_ht, ImGuiTableColumnFlags_WidthFixed, 92.0f);
        ImGui::TableSetupColumn(h_ex, ImGuiTableColumnFlags_WidthFixed, 84.0f);
        ImGui::TableHeadersRow();

        int num_fila_excel = 2; // Fila 1 = cabeceras (como en HSC)

        for (int c_idx = 0; c_idx < static_cast<int>(grupos.size()); ++c_idx) {
            CorrienteUI& corr = grupos[static_cast<size_t>(c_idx)];
            const motores::CorrienteBalance* mc =
                (c_idx < static_cast<int>(motor_grupos.size()))
                    ? &motor_grupos[static_cast<size_t>(c_idx)]
                    : nullptr;

            // Totales de la corriente para mostrar en su fila cabecera (como Row 2 "Gas Out")
            double sum_kmol = 0.0, sum_kg = 0.0, sum_nm3 = 0.0;
            double sum_hc_kj = 0.0, sum_ht_kj = 0.0, sum_ex_kj = 0.0;
            if (mc != nullptr) {
                for (const auto& mf : mc->filas) {
                    sum_kmol += mf.kmol;
                    sum_kg += mf.masa_kg;
                    sum_nm3 += mf.volumen_nm3;
                    sum_hc_kj += mf.h_sensible_kJ;
                    sum_ht_kj += mf.h_total_kJ;
                    sum_ex_kj += mf.exergia_total_kJ;
                }
            }
            double temp_corriente = corr.filas.empty() ? ((temp_unit_ == 0) ? 25.0 : 298.15)
                                                       : corr.filas.front().temper;

            // ── FILA CABECERA DE CORRIENTE (en negrita/color como "Gas Out" y "Cooling Water Out") ──
            ImGui::TableNextRow();
            ImGui::PushID(c_idx * 10000);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0,
                                   IM_COL32(42, 62, 88, 180));

            ImGui::TableSetColumnIndex(0);
            ImGui::TextDisabled("%d", num_fila_excel++);

            // Columna A: Nombre de la corriente + checkbox [Fijar T] en salidas
            ImGui::TableSetColumnIndex(1);
            if (es_salida) {
                if (ImGui::Checkbox("##fix_t", &corr.temperatura_fija)) {
                    necesita_recalcular = true;
                }
                if (ImGui::IsItemHovered()) {
                    ImGui::SetTooltip("Fijar temperatura de esta corriente durante Temp. Balance\n"
                                      "(ej. Cooling Water Out fija a 90 °C mientras se resuelve Gas Out).");
                }
                ImGui::SameLine();
            }
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::PushStyleColor(ImGuiCol_Text, color_cabecera_corriente());
            if (ImGui::InputText("##stream_name", corr.nombre, sizeof(corr.nombre))) {
                necesita_recalcular = true;
            }
            ImGui::PopStyleColor();
            if (ImGui::IsItemActivated()) {
                lado_salida_sel_ = es_salida;
                corriente_sel_ = c_idx;
                fila_sel_ = -1;
                col_sel_ = 0;
            }

            // Columna B: Temperatura de toda la corriente (editable como celda B2 en HSC)
            ImGui::TableSetColumnIndex(2);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##stream_temp", &temp_corriente, 0, 0, "%.3f")) {
                for (auto& f : corr.filas) f.temper = temp_corriente;
                necesita_recalcular = true;
            }
            if (ImGui::IsItemActivated()) {
                lado_salida_sel_ = es_salida;
                corriente_sel_ = c_idx;
                fila_sel_ = -1;
                col_sel_ = 1;
            }

            // Columna C: Presión (sombreada en cabecera de corriente como en HSC)
            ImGui::TableSetColumnIndex(3);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, IM_COL32(70, 68, 55, 110));
            ImGui::TextDisabled(" ");

            // Columna D: Total kmol de la corriente (editable para escalar proporcionalmente)
            ImGui::TableSetColumnIndex(4);
            ImGui::SetNextItemWidth(-1.0f);
            double edit_sum_kmol = sum_kmol;
            if (ImGui::InputDouble("##stream_kmol", &edit_sum_kmol, 0, 0, "%.3f")) {
                if (sum_kmol > 1e-12 && edit_sum_kmol >= 0.0) {
                    const double factor = edit_sum_kmol / sum_kmol;
                    for (auto& f : corr.filas) f.kmol *= factor;
                    factor_escala_reactiva_ = 1.0;
                    necesita_recalcular = true;
                }
            }
            if (ImGui::IsItemActivated()) {
                lado_salida_sel_ = es_salida;
                corriente_sel_ = c_idx;
                fila_sel_ = -1;
                col_sel_ = 3;
            }

            // Columna E: Total kg de la corriente (editable para escalar por masa total)
            ImGui::TableSetColumnIndex(5);
            ImGui::SetNextItemWidth(-1.0f);
            double edit_sum_kg = sum_kg;
            if (ImGui::InputDouble("##stream_kg", &edit_sum_kg, 0, 0, "%.3f")) {
                if (sum_kg > 1e-12 && edit_sum_kg >= 0.0) {
                    const double factor = edit_sum_kg / sum_kg;
                    for (auto& f : corr.filas) f.kmol *= factor;
                    factor_escala_reactiva_ = 1.0;
                    necesita_recalcular = true;
                }
            }
            if (ImGui::IsItemActivated()) {
                lado_salida_sel_ = es_salida;
                corriente_sel_ = c_idx;
                fila_sel_ = -1;
                col_sel_ = 4;
            }

            // Columna F: Total Nm³ de la corriente (editable para escalar por volumen total)
            ImGui::TableSetColumnIndex(6);
            ImGui::SetNextItemWidth(-1.0f);
            double edit_sum_nm3 = sum_nm3;
            if (ImGui::InputDouble("##stream_nm3", &edit_sum_nm3, 0, 0, "%.3f")) {
                if (sum_nm3 > 1e-12 && edit_sum_nm3 >= 0.0) {
                    const double factor = edit_sum_nm3 / sum_nm3;
                    for (auto& f : corr.filas) f.kmol *= factor;
                    factor_escala_reactiva_ = 1.0;
                    necesita_recalcular = true;
                }
            }
            if (ImGui::IsItemActivated()) {
                lado_salida_sel_ = es_salida;
                corriente_sel_ = c_idx;
                fila_sel_ = -1;
                col_sel_ = 5;
            }

            // Columna G: Heat Content (H sensible de la corriente)
            ImGui::TableSetColumnIndex(7);
            ImGui::TextColored(color_cabecera_corriente(), "%.2f", sum_hc_kj / ue_kj);

            // Columna H: Total H (H total de la corriente)
            ImGui::TableSetColumnIndex(8);
            ImGui::TextColored(color_cabecera_corriente(), "%.2f", sum_ht_kj / ue_kj);

            // Columna I: Exergy
            ImGui::TableSetColumnIndex(9);
            ImGui::TextColored(color_cabecera_corriente(), "%.2f", sum_ex_kj / ue_kj);

            ImGui::PopID();

            // ── FILAS DE ESPECIES DE LA CORRIENTE ──
            for (int r = 0; r < static_cast<int>(corr.filas.size()); ++r) {
                FilaUI& fila = corr.filas[static_cast<size_t>(r)];
                const motores::FilaBalance* mf =
                    (mc != nullptr && r < static_cast<int>(mc->filas.size()))
                        ? &mc->filas[static_cast<size_t>(r)]
                        : nullptr;

                ImGui::TableNextRow();
                ImGui::PushID(c_idx * 10000 + r + 1);

                if (lado_salida_sel_ == es_salida && corriente_sel_ == c_idx && fila_sel_ == r) {
                    ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(50, 75, 120, 220));
                }

                ImGui::TableSetColumnIndex(0);
                ImGui::TextDisabled("%d", num_fila_excel++);

                // Columna A: Fórmula (editable + autocompletado + buscador BD)
                ImGui::TableSetColumnIndex(1);
                ImGui::SetNextItemWidth(-30.0f);
                if (ImGui::InputText("##formula", fila.formula, sizeof(fila.formula))) {
                    if (fila.formula[0] != '\0' && enable_predictive_typing_)
                        ImGui::OpenPopup("##autocompletar");
                    necesita_recalcular = true;
                    necesita_resolver = true;
                }
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 0;
                }
                ImGui::SameLine();
                if (ImGui::Button("...", ImVec2(-1, 0))) {
                    modal_corriente_ = c_idx;
                    modal_fila_ = r;
                    modal_es_salida_ = es_salida;
                    ModalBuscadorDB::instancia().abrir(kModalHMB);
                }
                if (ImGui::BeginPopupContextItem("##ctx_fila_global")) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    if (ImGui::MenuItem("Insertar especie debajo"))
                        accion_pendiente_ = AccionPendiente::InsertarDebajo;
                    if (ImGui::MenuItem("Duplicar fila"))
                        accion_pendiente_ = AccionPendiente::DuplicarFila;
                    if (ImGui::MenuItem("Eliminar fila"))
                        accion_pendiente_ = AccionPendiente::BorrarFila;
                    ImGui::Separator();
                    if (ImGui::MenuItem("Subir")) accion_pendiente_ = AccionPendiente::MoverArriba;
                    if (ImGui::MenuItem("Bajar")) accion_pendiente_ = AccionPendiente::MoverAbajo;
                    ImGui::Separator();
                    if (ImGui::MenuItem("Eliminar corriente completa"))
                        accion_pendiente_ = AccionPendiente::BorrarCorriente;
                    ImGui::EndPopup();
                }
                popup_autocompletar_especie(repo, fila.formula, necesita_recalcular, necesita_resolver);

                // Columna B: Temperatura
                ImGui::TableSetColumnIndex(2);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##temp", &fila.temper, 0, 0, "%.3f"))
                    necesita_recalcular = true;
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 1;
                }

                // Columna C: Presión (bar)
                ImGui::TableSetColumnIndex(3);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##pres", &fila.presion, 0, 0, "%.2f"))
                    necesita_recalcular = true;
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 2;
                }

                // Columna D: Amount kmol (editable)
                ImGui::TableSetColumnIndex(4);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##kmol", &fila.kmol, 0, 0, "%.3f")) {
                    factor_escala_reactiva_ = 1.0;
                    necesita_recalcular = true;
                }
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 3;
                }

                // Columna E: Amount kg (editable - convierte automáticamente a kmol usando M_w)
                ImGui::TableSetColumnIndex(5);
                ImGui::SetNextItemWidth(-1.0f);
                double val_kg = (mf != nullptr) ? mf->masa_kg : 0.0;
                if (ImGui::InputDouble("##kg", &val_kg, 0, 0, "%.3f")) {
                    if (mf != nullptr && mf->especie != nullptr && mf->especie->masa_molar > 0.0) {
                        fila.kmol = std::max(0.0, val_kg / mf->especie->masa_molar);
                        factor_escala_reactiva_ = 1.0;
                        necesita_recalcular = true;
                    }
                }
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 4;
                }

                // Columna F: Amount Nm³ (editable - convierte automáticamente a kmol)
                ImGui::TableSetColumnIndex(6);
                ImGui::SetNextItemWidth(-1.0f);
                double val_nm3 = (mf != nullptr) ? mf->volumen_nm3 : 0.0;
                if (ImGui::InputDouble("##nm3", &val_nm3, 0, 0, "%.3f")) {
                    if (mf != nullptr && mf->especie != nullptr) {
                        const double vm = motores::FilaBalance::volumen_molar_nm3(mf->especie);
                        if (vm > 0.0) {
                            fila.kmol = std::max(0.0, val_nm3 / vm);
                            factor_escala_reactiva_ = 1.0;
                            necesita_recalcular = true;
                        }
                    }
                }
                if (ImGui::IsItemActivated()) {
                    lado_salida_sel_ = es_salida;
                    corriente_sel_ = c_idx;
                    fila_sel_ = r;
                    col_sel_ = 5;
                }

                // Columna G: Heat Content (Sensible H en HSC)
                ImGui::TableSetColumnIndex(7);
                if (mf != nullptr && mf->especie != nullptr)
                    ImGui::Text("%.2f", mf->h_sensible_kJ / ue_kj);
                else
                    ImGui::TextDisabled("--");

                // Columna H: Total H (Entalpía absoluta en HSC)
                ImGui::TableSetColumnIndex(8);
                if (mf != nullptr && mf->especie != nullptr)
                    ImGui::Text("%.2f", mf->h_total_kJ / ue_kj);
                else
                    ImGui::TextDisabled("--");

                // Columna I: Exergía total
                ImGui::TableSetColumnIndex(9);
                if (mf != nullptr && mf->especie != nullptr)
                    ImGui::Text("%.2f", mf->exergia_total_kJ / ue_kj);
                else
                    ImGui::TextDisabled("--");

                ImGui::PopID();
            }

            // Fila de flujo de calor externo si la corriente tiene calor != 0
            if (std::abs(corr.calor) > 1e-9) {
                ImGui::TableNextRow();
                ImGui::PushID(c_idx * 10000 + 9999);
                ImGui::TableSetColumnIndex(0);
                ImGui::TextDisabled("%d", num_fila_excel++);
                ImGui::TableSetColumnIndex(1);
                ImGui::TextColored(color_aviso(), "  [Heat Flow] %s", corr.nombre);
                ImGui::TableSetColumnIndex(7);
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputDouble("##q_flow_hc", &corr.calor, 0, 0, "%.2f"))
                    necesita_recalcular = true;
                ImGui::TableSetColumnIndex(8);
                ImGui::TextColored(color_aviso(), "%.2f", corr.calor);
                ImGui::PopID();
            }
        }

        ImGui::EndTable();
    }

    if (necesita_recalcular)
        recalcular(necesita_resolver);
}

// ─────────────────────────────────────────────────────────────────────────────
// Pestañas In1-% y Out1-% (Composición porcentual molar, másica, volumétrica y elemental)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_pestana_porcentaje(bool es_salida) {
    const auto& grupos = es_salida ? area_.salidas : area_.entradas;
    const double tot_kmol = es_salida ? resultado_.kmol_salida : resultado_.kmol_entrada;
    const double tot_kg   = es_salida ? resultado_.masa_salida_kg : resultado_.masa_entrada_kg;

    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.85f, 0.75f, 1.0f),
                       "Distribución Porcentual de %s (Mol-%%, Weight-%%, Volume-%%)",
                       es_salida ? "Salidas (Out-%)" : "Entradas (In-%)");

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable;
    if (ImGui::BeginTable(es_salida ? "hmb_out_pct" : "hmb_in_pct", 8, flags, ImVec2(0, 0))) {
        ImGui::TableSetupColumn("Corriente / Especie", ImGuiTableColumnFlags_WidthFixed, 190.0f);
        ImGui::TableSetupColumn("Mol-% (Corr.)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Wt-% (Corr.)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Vol-% (Corr.)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Mol-% (Total)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Wt-% (Total)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableSetupColumn("Masa (kg)", ImGuiTableColumnFlags_WidthFixed, 85.0f);
        ImGui::TableSetupColumn("Cantidad (kmol)", ImGuiTableColumnFlags_WidthFixed, 95.0f);
        ImGui::TableHeadersRow();

        for (const auto& c : grupos) {
            double c_kmol = 0.0, c_kg = 0.0, c_nm3 = 0.0;
            for (const auto& f : c.filas) {
                c_kmol += f.kmol;
                c_kg += f.masa_kg;
                c_nm3 += f.volumen_nm3;
            }
            ImGui::TableNextRow();
            ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(42, 62, 88, 170));
            ImGui::TableNextColumn();
            ImGui::TextColored(color_cabecera_corriente(), "%s", c.nombre.c_str());
            ImGui::TableNextColumn(); ImGui::Text("100.00 %%");
            ImGui::TableNextColumn(); ImGui::Text("100.00 %%");
            ImGui::TableNextColumn(); ImGui::Text("100.00 %%");
            ImGui::TableNextColumn();
            ImGui::Text("%.2f %%", tot_kmol > 1e-12 ? (c_kmol / tot_kmol * 100.0) : 0.0);
            ImGui::TableNextColumn();
            ImGui::Text("%.2f %%", tot_kg > 1e-12 ? (c_kg / tot_kg * 100.0) : 0.0);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", c_kg);
            ImGui::TableNextColumn(); ImGui::Text("%.3f", c_kmol);

            for (const auto& f : c.filas) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text("   %s", f.formula.c_str());
                ImGui::TableNextColumn();
                ImGui::Text("%.2f %%", c_kmol > 1e-12 ? (f.kmol / c_kmol * 100.0) : 0.0);
                ImGui::TableNextColumn();
                ImGui::Text("%.2f %%", c_kg > 1e-12 ? (f.masa_kg / c_kg * 100.0) : 0.0);
                ImGui::TableNextColumn();
                ImGui::Text("%.2f %%", c_nm3 > 1e-12 ? (f.volumen_nm3 / c_nm3 * 100.0) : 0.0);
                ImGui::TableNextColumn();
                ImGui::Text("%.2f %%", tot_kmol > 1e-12 ? (f.kmol / tot_kmol * 100.0) : 0.0);
                ImGui::TableNextColumn();
                ImGui::Text("%.2f %%", tot_kg > 1e-12 ? (f.masa_kg / tot_kg * 100.0) : 0.0);
                ImGui::TableNextColumn(); ImGui::Text("%.3f", f.masa_kg);
                ImGui::TableNextColumn(); ImGui::Text("%.3f", f.kmol);
            }
        }
        ImGui::EndTable();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tabla de una corriente individual (pestañas IN i / OUT i)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_tabla_corriente(std::vector<CorrienteUI>& grupos, int idx,
                                               bool es_salida) {
    if (idx < 0 || idx >= static_cast<int>(grupos.size())) return;
    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());
    CorrienteUI& g = grupos[static_cast<size_t>(idx)];

    const std::vector<motores::CorrienteBalance>& motor_grupos =
        es_salida ? area_.salidas : area_.entradas;
    const motores::CorrienteBalance* mg =
        (idx < static_cast<int>(motor_grupos.size())) ? &motor_grupos[static_cast<size_t>(idx)]
                                                      : nullptr;

    bool necesita_recalcular = false;
    bool necesita_resolver = false;

    const char* ut = etiqueta_temperatura();
    const char* ue = etiqueta_energia();

    char h_temp[64], h_hf[64], h_hs[64], h_ht[64], h_ex[64];
    std::snprintf(h_temp, sizeof(h_temp), "Temper.\n(%s)", ut);
    std::snprintf(h_hf, sizeof(h_hf), "Latent Hf\n(%s)", ue);
    std::snprintf(h_hs, sizeof(h_hs), "Heat Content\n(%s)", ue);
    std::snprintf(h_ht, sizeof(h_ht), "Total H\n(%s)", ue);
    std::snprintf(h_ex, sizeof(h_ex), "Exergy\n(%s)", ue);

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY |
                                  ImGuiTableFlags_Resizable;
    if (ImGui::BeginTable("hmb_tabla_corriente", 11, flags, ImVec2(0, 260))) {
        ImGui::TableSetupColumn("Formula", ImGuiTableColumnFlags_WidthFixed, 170.0f);
        ImGui::TableSetupColumn(h_temp, ImGuiTableColumnFlags_WidthFixed, 82.0f);
        ImGui::TableSetupColumn("Pressure\n(bar)", ImGuiTableColumnFlags_WidthFixed, 70.0f);
        ImGui::TableSetupColumn("Amount\n(kmol)", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Amount\n(kg)", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Amount\n(Nm³)", ImGuiTableColumnFlags_WidthFixed, 86.0f);
        ImGui::TableSetupColumn("Cp\n(kJ/K)", ImGuiTableColumnFlags_WidthFixed, 72.0f);
        ImGui::TableSetupColumn(h_hf, ImGuiTableColumnFlags_WidthFixed, 88.0f);
        ImGui::TableSetupColumn(h_hs, ImGuiTableColumnFlags_WidthFixed, 92.0f);
        ImGui::TableSetupColumn(h_ht, ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn(h_ex, ImGuiTableColumnFlags_WidthFixed, 84.0f);
        ImGui::TableHeadersRow();

        for (int r = 0; r < static_cast<int>(g.filas.size()); ++r) {
            FilaUI& fila = g.filas[static_cast<size_t>(r)];
            const motores::FilaBalance* mf =
                (mg != nullptr && r < static_cast<int>(mg->filas.size()))
                    ? &mg->filas[static_cast<size_t>(r)]
                    : nullptr;

            ImGui::TableNextRow();
            ImGui::PushID(r);

            if (fila_sel_ == r && corriente_sel_ == idx && lado_salida_sel_ == es_salida)
                ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(50, 75, 120, 255));

            ImGui::TableSetColumnIndex(0);
            ImGui::SetNextItemWidth(-32.0f);
            if (ImGui::InputText("##formula", fila.formula, sizeof(fila.formula))) {
                if (fila.formula[0] != '\0' && enable_predictive_typing_)
                    ImGui::OpenPopup("##autocompletar");
                necesita_recalcular = true;
                necesita_resolver = true;
            }
            if (ImGui::IsItemActivated()) {
                fila_sel_ = r;
                corriente_sel_ = idx;
                lado_salida_sel_ = es_salida;
            }
            ImGui::SameLine();
            if (ImGui::Button("...", ImVec2(-1, 0))) {
                modal_corriente_ = idx;
                modal_fila_ = r;
                modal_es_salida_ = es_salida;
                ModalBuscadorDB::instancia().abrir(kModalHMB);
            }
            if (ImGui::BeginPopupContextItem("##ctx_fila")) {
                fila_sel_ = r;
                corriente_sel_ = idx;
                lado_salida_sel_ = es_salida;
                if (ImGui::MenuItem("Insertar fila debajo"))
                    accion_pendiente_ = AccionPendiente::InsertarDebajo;
                if (ImGui::MenuItem("Duplicar fila"))
                    accion_pendiente_ = AccionPendiente::DuplicarFila;
                if (ImGui::MenuItem("Eliminar fila"))
                    accion_pendiente_ = AccionPendiente::BorrarFila;
                ImGui::Separator();
                if (ImGui::MenuItem("Subir")) accion_pendiente_ = AccionPendiente::MoverArriba;
                if (ImGui::MenuItem("Bajar")) accion_pendiente_ = AccionPendiente::MoverAbajo;
                ImGui::EndPopup();
            }
            popup_autocompletar_especie(repo, fila.formula, necesita_recalcular, necesita_resolver);

            // Temperatura
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##temp", &fila.temper, 0, 0, "%.3f"))
                necesita_recalcular = true;

            // Presión
            ImGui::TableSetColumnIndex(2);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##pres", &fila.presion, 0, 0, "%.2f"))
                necesita_recalcular = true;

            // Cantidad kmol
            ImGui::TableSetColumnIndex(3);
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputDouble("##kmol", &fila.kmol, 0, 0, "%.3f"))
                necesita_recalcular = true;

            // Cantidad kg (editable)
            ImGui::TableSetColumnIndex(4);
            ImGui::SetNextItemWidth(-1.0f);
            double v_kg = (mf != nullptr) ? mf->masa_kg : 0.0;
            if (ImGui::InputDouble("##kg", &v_kg, 0, 0, "%.3f")) {
                if (mf != nullptr && mf->especie != nullptr && mf->especie->masa_molar > 0.0) {
                    fila.kmol = std::max(0.0, v_kg / mf->especie->masa_molar);
                    necesita_recalcular = true;
                }
            }

            // Cantidad Nm³ (editable)
            ImGui::TableSetColumnIndex(5);
            ImGui::SetNextItemWidth(-1.0f);
            double v_nm3 = (mf != nullptr) ? mf->volumen_nm3 : 0.0;
            if (ImGui::InputDouble("##nm3", &v_nm3, 0, 0, "%.3f")) {
                if (mf != nullptr && mf->especie != nullptr) {
                    const double vm = motores::FilaBalance::volumen_molar_nm3(mf->especie);
                    if (vm > 0.0) {
                        fila.kmol = std::max(0.0, v_nm3 / vm);
                        necesita_recalcular = true;
                    }
                }
            }

            const double ue_kj = motores::factor_energia_kJ(unidad_energia());
            ImGui::TableSetColumnIndex(6);
            if (mf) ImGui::Text("%.2f", mf->cp_kJ_K); else ImGui::TextDisabled("--");
            ImGui::TableSetColumnIndex(7);
            if (mf) ImGui::Text("%.2f", mf->h_formacion_kJ / ue_kj); else ImGui::TextDisabled("--");
            ImGui::TableSetColumnIndex(8);
            if (mf) ImGui::Text("%.2f", mf->h_sensible_kJ / ue_kj); else ImGui::TextDisabled("--");
            ImGui::TableSetColumnIndex(9);
            if (mf) ImGui::Text("%.2f", mf->h_total_kJ / ue_kj); else ImGui::TextDisabled("--");
            ImGui::TableSetColumnIndex(10);
            if (mf) ImGui::Text("%.2f", mf->exergia_total_kJ / ue_kj); else ImGui::TextDisabled("--");

            ImGui::PopID();
        }

        // Fila de totales
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        ImGui::TableSetColumnIndex(0);
        ImGui::TextColored(color_aviso(), "TOTAL");
        if (mg != nullptr) {
            double km = 0, kg = 0, nm3 = 0, cp = 0, hf = 0, hs = 0, ht = 0, ex = 0;
            for (const auto& f : mg->filas) {
                km += f.kmol;
                kg += f.masa_kg;
                nm3 += f.volumen_nm3;
                cp += f.cp_kJ_K;
                hf += f.h_formacion_kJ;
                hs += f.h_sensible_kJ;
                ht += f.h_total_kJ;
                ex += f.exergia_total_kJ;
            }
            const double ue_kj = motores::factor_energia_kJ(unidad_energia());
            ImGui::TableSetColumnIndex(3);  ImGui::Text("%.3f", km);
            ImGui::TableSetColumnIndex(4);  ImGui::Text("%.3f", kg);
            ImGui::TableSetColumnIndex(5);  ImGui::Text("%.3f", nm3);
            ImGui::TableSetColumnIndex(6);  ImGui::Text("%.2f", cp);
            ImGui::TableSetColumnIndex(7);  ImGui::Text("%.2f", hf / ue_kj);
            ImGui::TableSetColumnIndex(8);  ImGui::Text("%.2f", hs / ue_kj);
            ImGui::TableSetColumnIndex(9);  ImGui::Text("%.2f", ht / ue_kj);
            ImGui::TableSetColumnIndex(10); ImGui::Text("%.2f", ex / ue_kj);
        }

        ImGui::EndTable();
    }

    if (necesita_recalcular)
        recalcular(necesita_resolver);
}

void PanelBalanceHoja::dibujar_pestana_corriente(int idx, bool es_salida) {
    auto& grupos = lista(es_salida);
    if (idx < 0 || idx >= static_cast<int>(grupos.size())) return;
    CorrienteUI& g = grupos[static_cast<size_t>(idx)];

    ImGui::SetNextItemWidth(220);
    ImGui::InputText("##nombre_corriente", g.nombre, sizeof(g.nombre));
    if (es_salida) {
        ImGui::SameLine();
        if (ImGui::Checkbox("T Fija en Temp. Balance", &g.temperatura_fija)) {
            recalcular(false);
        }
    }
    ImGui::SameLine();
    if (ImGui::Button("Duplicar")) {
        corriente_sel_ = idx;
        lado_salida_sel_ = es_salida;
        accion_pendiente_ = AccionPendiente::DuplicarCorriente;
    }
    ImGui::SameLine();
    if (ImGui::Button("Eliminar")) {
        corriente_sel_ = idx;
        lado_salida_sel_ = es_salida;
        accion_pendiente_ = AccionPendiente::BorrarCorriente;
    }
    ImGui::SameLine();
    ImGui::Text("Heat Flow:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(105);
    if (ImGui::InputDouble("##calor_externo", &g.calor, 0, 0, "%.4g")) recalcular(false);
    ImGui::SameLine();
    ImGui::TextDisabled("(%s) +pérdida / -aporte", etiqueta_energia());

    dibujar_tabla_corriente(grupos, idx, es_salida);

    if (ImGui::Button("+ Especie (base de datos)")) {
        modal_corriente_ = idx;
        modal_fila_ = -1;
        modal_es_salida_ = es_salida;
        ModalBuscadorDB::instancia().abrir(kModalHMB);
    }
    ImGui::SameLine();
    if (ImGui::Button("+ Fila vacía")) {
        FilaUI f;
        f.temper = (temp_unit_ == 0) ? 25.0 : 298.15;
        g.filas.push_back(f);
        fila_sel_ = static_cast<int>(g.filas.size()) - 1;
        recalcular(false);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pestaña Target (Solvers de Temperatura de Producto, Precalentamiento, Calor,
// Reacción Teórica y Escalado Industrial)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_pestana_target() {
    const double ue_kj = motores::factor_energia_kJ(unidad_energia());

    // 1. Solver Temp. Balance (Producto / Llama Adiabática)
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.85f, 1.0f, 1.0f),
                       "1. Temp. Balance - Temperatura de Salida / Llama Adiabática (ΔH = 0)");
    ImGui::TextWrapped(
        "Calcula la temperatura de los productos para un balance térmico nulo (ΔH = 0). "
        "Puedes marcar corrientes auxiliares (ej. Cooling Water Out) como 'T Fija' para "
        "resolver únicamente la temperatura de la corriente principal (ej. Gas Out).");

    if (!salidas_.empty()) {
        for (size_t i = 0; i < salidas_.size(); ++i) {
            ImGui::PushID(static_cast<int>(i));
            if (ImGui::Checkbox(salidas_[i].nombre, &salidas_[i].temperatura_fija)) {
                recalcular(false);
            }
            ImGui::SameLine();
            ImGui::TextDisabled(salidas_[i].temperatura_fija ? "(T Fija)" : "(Resolver T)");
            if (i + 1 < salidas_.size()) ImGui::SameLine(0.0f, 18.0f);
            ImGui::PopID();
        }
    }
    ImGui::Checkbox("Conservar diferencias de temperatura (ΔT) entre salidas libres",
                    &mantener_offsets_);
    if (ImGui::Button("Ejecutar Temp. Balance (ΔH = 0)", ImVec2(280, 28))) temp_balance();

    if (!ultimo_solver_.mensaje.empty()) {
        ImGui::SameLine();
        if (ultimo_solver_.factible) {
            ImGui::TextColored(color_ok(), "T producto = %.3f %s (%d iter)",
                               motores::temperatura_desde_K(ultimo_solver_.temperatura_K,
                                                            unidad_temperatura()),
                               etiqueta_temperatura(), ultimo_solver_.iteraciones);
        } else {
            ImGui::TextColored(color_aviso(), "%s", ultimo_solver_.mensaje.c_str());
        }
    }

    // 2. Solver de Temperatura Óptima de Precalentamiento de Materias Primas
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.85f, 1.0f, 1.0f),
                       "2. Temperatura Óptima de Precalentamiento de Materias Primas");
    ImGui::TextWrapped(
        "Encuentra la temperatura de precalentamiento de una corriente de entrada para "
        "compensar los requerimientos eléctricos/térmicos del reactor y lograr operación autotérmica (ΔH = 0).");

    if (!entradas_.empty()) {
        std::string items_in;
        for (size_t i = 0; i < entradas_.size(); ++i) {
            items_in += entradas_[i].nombre;
            items_in.push_back('\0');
        }
        items_in.push_back('\0');
        preheat_stream_idx_ = std::clamp(preheat_stream_idx_, 0,
                                         static_cast<int>(entradas_.size()) - 1);
        ImGui::SetNextItemWidth(220);
        ImGui::Combo("Corriente a precalentar", &preheat_stream_idx_, items_in.c_str());
        ImGui::SameLine();
        if (ImGui::Button("Calcular T Precalentamiento (ΔH = 0)")) {
            sincronizar_motor(true);
            ultimo_preheat_ = motor_.temperatura_precalentamiento(area_, preheat_stream_idx_);
            if (ultimo_preheat_.factible) copiar_temperaturas_desde_motor();
            recalcular(false);
        }
        if (!ultimo_preheat_.mensaje.empty()) {
            if (ultimo_preheat_.factible) {
                ImGui::TextColored(color_ok(), "T precalentamiento óptima (%s): %.2f %s",
                                   entradas_[static_cast<size_t>(preheat_stream_idx_)].nombre,
                                   motores::temperatura_desde_K(ultimo_preheat_.temperatura_K,
                                                                unidad_temperatura()),
                                   etiqueta_temperatura());
            } else {
                ImGui::TextColored(color_aviso(), "%s", ultimo_preheat_.mensaje.c_str());
            }
        }
    }

    // 3. Calor requerido para T objetivo
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.85f, 1.0f, 1.0f),
                       "3. Calor Requerido para una Temperatura de Salida Fijada");
    ImGui::SetNextItemWidth(120);
    ImGui::InputDouble("Temperatura objetivo", &target_temp_, 0, 0, "%.2f");
    ImGui::SameLine();
    ImGui::TextDisabled("(%s)", etiqueta_temperatura());
    ImGui::SameLine();
    if (ImGui::Button("Calcular Calor Requerido")) {
        sincronizar_motor(true);
        const double T_K = motores::temperatura_a_K(target_temp_, unidad_temperatura());
        calor_requerido_ = motor_.calor_requerido(area_, T_K, mantener_offsets_);
        calor_calculado_ = true;
    }
    if (calor_calculado_) {
        const double mostrar = calor_requerido_ / ue_kj;
        if (calor_requerido_ > 0.0) {
            ImGui::TextColored(color_aviso(),
                               "Se deben RETIRAR %.3f %s (enfriamiento) para mantener %.2f %s.",
                               mostrar, etiqueta_energia(), target_temp_, etiqueta_temperatura());
        } else {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.5f, 0.7f, 1.0f, 1.0f),
                               "Se deben APORTAR %.3f %s (calefacción eléctrica/externa) para alcanzar %.2f %s.",
                               -mostrar, etiqueta_energia(), target_temp_, etiqueta_temperatura());
        }
        if (!salidas_.empty()) {
            ImGui::SameLine();
            if (ImGui::Button("Aplicar al balance")) {
                const double delta = calor_requerido_ - resultado_.calor_externo_kJ;
                salidas_.front().calor += motores::energia_desde_kJ(delta, unidad_energia());
                recalcular(false);
                calor_calculado_ = false;
            }
        }
    }

    // 4. Balance Teórico desde Ecuación de Reacción, Equilibrio de Gibbs (GEM) y Escalado Industrial
    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.7f, 0.85f, 1.0f, 1.0f),
                       "4. Balance Teórico (Reacción o Equilibrio de Gibbs) y Escalado Industrial");

    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());
    const auto& todas_rxn = repo.todas_reacciones();
    if (!todas_rxn.empty()) {
        rxn_sel_idx_ = std::clamp(rxn_sel_idx_, 0, static_cast<int>(todas_rxn.size()) - 1);
        std::string items_rxn;
        for (const auto* r : todas_rxn) {
            items_rxn += r->nombre;
            items_rxn.push_back('\0');
        }
        items_rxn.push_back('\0');
        ImGui::SetNextItemWidth(340);
        ImGui::Combo("Reacción química", &rxn_sel_idx_, items_rxn.c_str());
        ImGui::SetNextItemWidth(90);
        ImGui::InputDouble("Base (kmol)", &rxn_kmol_base_, 0, 0, "%.3f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(90);
        ImGui::InputDouble("Conversión %", &rxn_conversion_pct_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(90);
        ImGui::InputDouble("Exceso O2 %", &rxn_exceso_o2_pct_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::Checkbox("Aire (N2)", &rxn_incluir_aire_n2_);

        if (ImGui::Button("Generar Hoja desde Reacción")) {
            const nucleo::Reaccion* rxn = todas_rxn[static_cast<size_t>(rxn_sel_idx_)];
            motor_.poblar_desde_reaccion(
                area_, *rxn, repo.mapa_especies(),
                rxn_kmol_base_, rxn_conversion_pct_ / 100.0, rxn_exceso_o2_pct_,
                rxn_incluir_aire_n2_, nucleo::STP_TEMP, nucleo::STP_TEMP);
            entradas_.clear();
            salidas_.clear();
            factor_escala_reactiva_ = 1.0;
            auto copiar_desde_motor = [&](const std::vector<motores::CorrienteBalance>& src,
                                          std::vector<CorrienteUI>& dst) {
                for (const auto& mc : src) {
                    CorrienteUI cu;
                    copiar_texto(cu.nombre, sizeof(cu.nombre), mc.nombre);
                    cu.calor = motores::energia_desde_kJ(mc.calor_kJ, unidad_energia());
                    cu.temperatura_fija = mc.temperatura_fija;
                    for (const auto& mf : mc.filas) {
                        FilaUI fu;
                        copiar_texto(fu.formula, sizeof(fu.formula), mf.formula);
                        fu.temper = motores::temperatura_desde_K(mf.temperatura_K,
                                                                 unidad_temperatura());
                        fu.presion = mf.presion_bar;
                        fu.kmol = mf.kmol;
                        cu.filas.push_back(fu);
                    }
                    dst.push_back(cu);
                }
            };
            copiar_desde_motor(area_.entradas, entradas_);
            copiar_desde_motor(area_.salidas, salidas_);
            recalcular(true);
        }
    }

    ImGui::Spacing();
    ImGui::SetNextItemWidth(110);
    ImGui::InputDouble("T Equilibrio (GEM)", &gem_temp_, 0, 0, "%.1f");
    ImGui::SameLine();
    ImGui::TextDisabled("(%s)", etiqueta_temperatura());
    ImGui::SameLine();
    ImGui::SetNextItemWidth(90);
    ImGui::InputDouble("P (bar)##gem", &gem_pres_bar_, 0, 0, "%.2f");
    ImGui::SameLine();
    if (ImGui::Button("Calcular Salida por Equilibrio de Gibbs (GEM)")) {
        calcular_equilibrio_gibbs();
    }
    if (!gem_mensaje_.empty()) {
        ImGui::TextColored(gem_ok_ ? color_ok() : color_aviso(), "%s", gem_mensaje_.c_str());
    }

    ImGui::Spacing();
    ImGui::SetNextItemWidth(100);
    ImGui::InputDouble("Factor de Escala", &scale_factor_, 0, 0, "%.3g");
    ImGui::SameLine();
    if (ImGui::Button("Escalar Proceso (× Factor)")) {
        if (scale_factor_ > 0.0) {
            for (auto* g : {&entradas_, &salidas_}) {
                for (auto& c : *g) {
                    c.calor *= scale_factor_;
                    for (auto& f : c.filas) f.kmol *= scale_factor_;
                }
            }
            factor_escala_reactiva_ = 1.0;
            recalcular(false);
        }
    }
    ImGui::SameLine();
    ImGui::SetNextItemWidth(110);
    ImGui::InputDouble("Masa objetivo (kg)", &scale_target_kg_, 0, 0, "%.1f");
    ImGui::SameLine();
    if (ImGui::Button("Escalar a Masa Total")) {
        if (resultado_.masa_salida_kg > 1e-9 && scale_target_kg_ > 0.0) {
            const double factor = scale_target_kg_ / resultado_.masa_salida_kg;
            for (auto* g : {&entradas_, &salidas_}) {
                for (auto& c : *g) {
                    c.calor *= factor;
                    for (auto& f : c.filas) f.kmol *= factor;
                }
            }
            factor_escala_reactiva_ = 1.0;
            recalcular(false);
        }
    }
}

void PanelBalanceHoja::calcular_equilibrio_gibbs() {
    sincronizar_motor(true);
    std::map<int64_t, double> alimentacion;
    std::set<int64_t> candidatos_set;

    bool hay_salida_fija = false;
    for (const auto& cs : area_.salidas) {
        if (cs.temperatura_fija) {
            hay_salida_fija = true;
            break;
        }
    }

    for (const auto& ce : area_.entradas) {
        const bool es_agua_enfr = (ce.nombre.find("Water") != std::string::npos ||
                                   ce.nombre.find("Agua") != std::string::npos);
        if (hay_salida_fija && es_agua_enfr) continue;
        for (const auto& f : ce.filas) {
            if (f.especie != nullptr && f.kmol > 0.0) {
                alimentacion[f.especie->id] += f.kmol;
                candidatos_set.insert(f.especie->id);
            }
        }
    }
    for (const auto& cs : area_.salidas) {
        if (hay_salida_fija && cs.temperatura_fija) continue;
        for (const auto& f : cs.filas) {
            if (f.especie != nullptr) candidatos_set.insert(f.especie->id);
        }
    }

    if (alimentacion.empty()) {
        gem_ok_ = false;
        gem_mensaje_ = "Agrega especies con cantidad > 0 en las entradas para calcular el equilibrio.";
        return;
    }

    std::vector<int64_t> candidatos(candidatos_set.begin(), candidatos_set.end());
    const double T_K = std::max(motores::temperatura_a_K(gem_temp_, unidad_temperatura()), 298.15);
    const double P_atm = std::max(gem_pres_bar_ / 1.01325, 0.01);

    const domain::ThermoRepository& repo =
        repo_ ? *repo_ : static_cast<const domain::ThermoRepository&>(base_datos::GestorDB::instancia());
    motores::MinimizadorGibbs minimizador;
    const auto& db = repo.mapa_especies();
    const auto res = minimizador.minimizar(alimentacion, T_K, P_atm, db, candidatos);
    if (!res.convergio || res.flujo_molar.empty()) {
        gem_ok_ = false;
        gem_mensaje_ = "El minimizador de Gibbs no convergió con las especies actuales.";
        return;
    }

    // Encuentra la primera corriente de salida libre (no fija) o crea una
    int out_idx = -1;
    for (size_t i = 0; i < salidas_.size(); ++i) {
        if (!salidas_[i].temperatura_fija) {
            out_idx = static_cast<int>(i);
            break;
        }
    }
    if (out_idx < 0) {
        CorrienteUI nueva_out;
        copiar_texto(nueva_out.nombre, sizeof(nueva_out.nombre), "Equilibrium Products");
        salidas_.insert(salidas_.begin(), nueva_out);
        out_idx = 0;
    }

    CorrienteUI& corr_out = salidas_[static_cast<size_t>(out_idx)];
    const double T_disp = motores::temperatura_desde_K(T_K, unidad_temperatura());

    // Actualiza cantidades de las especies existentes o reconstruye la corriente
    std::map<int64_t, bool> asignados;
    for (size_t r = 0; r < corr_out.filas.size(); ++r) {
        const auto* esp = repo.especie_por_nombre(corr_out.filas[r].formula);
        if (!esp) esp = buscar_especie_por_nombre_o_formula(corr_out.filas[r].formula);
        if (esp != nullptr && res.flujo_molar.count(esp->id)) {
            corr_out.filas[r].kmol = res.flujo_molar.at(esp->id);
            corr_out.filas[r].temper = T_disp;
            corr_out.filas[r].presion = gem_pres_bar_;
            asignados[esp->id] = true;
        }
    }
    for (const auto& [sid, moles] : res.flujo_molar) {
        if (asignados[sid] || moles < 1e-7) continue;
        auto it = db.find(sid);
        if (it == db.end()) continue;
        FilaUI nf;
        copiar_texto(nf.formula, sizeof(nf.formula), it->second.formula);
        nf.temper = T_disp;
        nf.presion = gem_pres_bar_;
        nf.kmol = moles;
        corr_out.filas.push_back(nf);
    }

    recalcular(true);
    gem_ok_ = true;
    char msg[192];
    std::snprintf(msg, sizeof(msg),
                  "Equilibrio de Gibbs (GEM) aplicado a '%s' (T = %.1f %s, G = %.2f MJ, %d iter).",
                  corr_out.nombre, T_disp, etiqueta_temperatura(),
                  res.energia_gibbs_total / 1000.0, res.iteraciones);
    gem_mensaje_ = msg;
}

}} // namespace cma::interfaz
