#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"

#include "interfaz/panels/panel_material_db.hpp"
#include "interfaz/ui_css_style.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <imgui.h>
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstring>

namespace cma { namespace interfaz {

namespace {

const char* kCategoriasHSC[] = {
    "All",
    "Blends",
    "Concentrates",
    "Fluxes",
    "Fuels",
    "Sludges",
    "Steels",
    "Plastics",
    "Slags"
};

constexpr int kNumCategoriasHSC = static_cast<int>(sizeof(kCategoriasHSC) / sizeof(kCategoriasHSC[0]));

bool contiene_icase(const std::string& texto, const char* patron) {
    if (!patron || patron[0] == '\0') return true;
    std::string t = texto;
    std::string p = patron;
    std::transform(t.begin(), t.end(), t.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    std::transform(p.begin(), p.end(), p.begin(), [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return t.find(p) != std::string::npos;
}

} // namespace

PanelMaterialDB::PanelMaterialDB(const domain::ThermoRepository& repository)
    : repo_(repository), db_(repository)
{
    ref_celda_activa_ = "A7";
    buffer_formula_bar_[0] = '\0';
}

void PanelMaterialDB::dibujar_barra_herramientas(cma::application::modules::RegistroMaterial& mat) {
    const auto especies = db_.resolver_especies(mat);
    const auto elementos = db_.resolver_elementos(especies);

    css::begin_ribbon_bar("##MatDBRibbonBar", 78.0f);
    const float bar_bottom_y = ImGui::GetWindowPos().y + ImGui::GetWindowHeight();

    // ── GRUPO 1: Database ──
    float g1_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("rb_save_db", "Save", "DB", css::RibbonIcon::Save, 50.0f, 54.0f)) {
        if (db_.guardar_catalogo_tsv(ruta_archivo_io_)) {
            mensaje_estado_ = std::string("Base de datos guardada en ") + ruta_archivo_io_;
            estado_ok_ = true;
        } else {
            mensaje_estado_ = "Error al guardar la base de datos.";
            estado_ok_ = false;
        }
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_load_db", "Load", "DB", css::RibbonIcon::FolderOpen, 50.0f, 54.0f)) {
        if (db_.cargar_catalogo_tsv(ruta_archivo_io_)) {
            idx_material_ = 0;
            mensaje_estado_ = std::string("Base de datos cargada desde ") + ruta_archivo_io_;
            estado_ok_ = true;
        } else {
            mensaje_estado_ = std::string("No se encontró ") + ruta_archivo_io_;
            estado_ok_ = false;
        }
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_reset_db", "Reset", "HSC", css::RibbonIcon::ResetDB, 50.0f, 54.0f)) {
        db_.restaurar_predeterminados();
        idx_material_ = 0;
        mensaje_estado_ = "Catálogo predeterminado HSC Material Database restaurado.";
        estado_ok_ = true;
    }
    float g1_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g1_start, g1_end, bar_bottom_y, "Database");

    ImGui::SameLine(0.0f, 10.0f);

    // ── GRUPO 2: Composition & Sub-Materials ──
    float g2_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("rb_add_row", "+ Add", "Species", css::RibbonIcon::AddRow, 54.0f, 54.0f)) {
        mat.composicion.push_back({"SiO2", 1.000, "Added species"});
        mensaje_estado_ = "Nueva especie añadida en Composition.";
        estado_ok_ = true;
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_add_sub", "+ [Sub]", "Material", css::RibbonIcon::SubMaterial, 58.0f, 54.0f)) {
        std::string sub_ref = (mat.nombre != "Concentrate") ? "[Concentrate]" : "[Silica Flux]";
        mat.composicion.push_back({sub_ref, 10.000, "Sub-material blend component"});
        mensaje_estado_ = "Sub-material " + sub_ref + " añadido a Composition.";
        estado_ok_ = true;
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_norm", "Normalize", "100 wt-%", css::RibbonIcon::Normalize, 62.0f, 54.0f)) {
        cma::application::modules::MaterialDB::normalizar_composicion(mat);
        mensaje_estado_ = "Composición normalizada exactamente a 100.000 wt-%.";
        estado_ok_ = true;
    }
    float g2_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g2_start, g2_end, bar_bottom_y, "Composition");

    ImGui::SameLine(0.0f, 10.0f);

    // ── GRUPO 3: HSC Sim & Export ──
    float g3_start = ImGui::GetCursorScreenPos().x;
    if (css::ribbon_button("rb_to_sim", "To HSC", "Sim Feed", css::RibbonIcon::SendSim, 58.0f, 54.0f)) {
        inyectar_en_flowsheet(mat, especies, elementos);
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_to_bal", "To Heat", "Balance", css::RibbonIcon::BalanceScale, 58.0f, 54.0f)) {
        if (al_enviar_a_balance) {
            al_enviar_a_balance(mat, caudal_sim_t_h_ * 1000.0, temp_sim_c_);
            mensaje_estado_ = "Material '" + mat.nombre + "' enviado a Heat & Material Balances.";
            estado_ok_ = true;
        }
    }
    ImGui::SameLine(0.0f, 2.0f);
    if (css::ribbon_button("rb_to_excel", "To Excel", "(CSV)", css::RibbonIcon::ToExcel, 56.0f, 54.0f)) {
        if (db_.exportar_csv_material(ruta_csv_export_, mat)) {
            mensaje_estado_ = std::string("Hoja exportada a ") + ruta_csv_export_;
            estado_ok_ = true;
        } else {
            mensaje_estado_ = std::string("Error al exportar ") + ruta_csv_export_;
            estado_ok_ = false;
        }
    }
    float g3_end = ImGui::GetItemRectMax().x;
    css::draw_ribbon_group_footer(g3_start, g3_end, bar_bottom_y, "HSC Sim & Export");

    ImGui::SameLine(0.0f, 14.0f);

    // Selector rápido de [Sub-Material] + Resumen del material activo a la derecha del Ribbon
    ImGui::BeginGroup();
    ImGui::TextColored(ImVec4(0.32f, 0.42f, 0.16f, 1.0f), "%s  [%s]", mat.nombre.c_str(), mat.categoria.c_str());
    const auto& cat = db_.catalogo();
    std::vector<std::string> nombres_sub;
    for (const auto& m : cat) {
        if (m.nombre != mat.nombre) nombres_sub.push_back(m.nombre);
    }
    if (!nombres_sub.empty()) {
        if (idx_submaterial_combo_ >= static_cast<int>(nombres_sub.size())) idx_submaterial_combo_ = 0;
        ImGui::SetNextItemWidth(160.0f);
        if (ImGui::BeginCombo("##SubMatCombo", nombres_sub[idx_submaterial_combo_].c_str())) {
            for (int i = 0; i < static_cast<int>(nombres_sub.size()); ++i) {
                if (ImGui::Selectable(nombres_sub[i].c_str(), i == idx_submaterial_combo_)) {
                    idx_submaterial_combo_ = i;
                }
            }
            ImGui::EndCombo();
        }
        ImGui::SameLine(0.0f, 4.0f);
        if (ImGui::SmallButton("Insertar [Sub-Mat]")) {
            std::string bracketed = "[" + nombres_sub[idx_submaterial_combo_] + "]";
            mat.composicion.push_back({bracketed, 10.000, "Sub-material blend"});
        }
    }
    ImGui::EndGroup();

    css::end_ribbon_bar();
}

void PanelMaterialDB::dibujar_barra_formulas(
    cma::application::modules::RegistroMaterial& mat,
    const std::vector<cma::application::modules::EspecieResuelta>& especies,
    const std::vector<cma::application::modules::ElementoResuelto>& elementos)
{
    const auto tk = css::tokens();
    ImGui::PushStyleColor(ImGuiCol_ChildBg, tk.header_bg);
    ImGui::PushStyleColor(ImGuiCol_Text, tk.text_primary);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(6.0f, 4.0f));
    ImGui::BeginChild("##HSCFormulaBar", ImVec2(0, 30), true, ImGuiWindowFlags_NoScrollbar);

    // Cuadro de referencia de celda activa (ej. "A7")
    ImGui::PushStyleColor(ImGuiCol_FrameBg, tk.card_bg);
    ImGui::SetNextItemWidth(62.0f);
    char ref_buf[16];
    std::snprintf(ref_buf, sizeof(ref_buf), "%s", ref_celda_activa_.c_str());
    ImGui::InputText("##CellRefBox", ref_buf, sizeof(ref_buf), ImGuiInputTextFlags_ReadOnly);
    ImGui::SameLine(0.0f, 6.0f);

    // Botones clásicos [x] [✓] [fx] de la barra de fórmulas de HSC
    ImGui::PushStyleColor(ImGuiCol_Button, tk.panel_bg);
    ImGui::PushStyleColor(ImGuiCol_ButtonHovered, tk.border_soft);
    if (ImGui::SmallButton(" x ")) {
        buffer_formula_bar_[0] = '\0';
    }
    ImGui::SameLine(0.0f, 3.0f);
    if (ImGui::SmallButton(" v ")) {
        if (fila_sel_ >= 0 && fila_sel_ < static_cast<int>(mat.composicion.size())) {
            if (col_sel_ == 0) {
                mat.composicion[fila_sel_].nombre = buffer_formula_bar_;
            } else if (col_sel_ == 1) {
                try { mat.composicion[fila_sel_].wt_pct = std::stod(buffer_formula_bar_); } catch (...) {}
            } else if (col_sel_ == 6) {
                mat.composicion[fila_sel_].referencia = buffer_formula_bar_;
            }
        }
    }
    ImGui::SameLine(0.0f, 5.0f);
    ImGui::TextColored(ImVec4(0.16f, 0.34f, 0.62f, 1.0f), "fx");
    ImGui::PopStyleColor(2);
    ImGui::SameLine(0.0f, 8.0f);

    ImGui::SetNextItemWidth(-1.0f);
    if (ImGui::InputText("##FormulaContent", buffer_formula_bar_, sizeof(buffer_formula_bar_),
                         ImGuiInputTextFlags_EnterReturnsTrue)) {
        if (fila_sel_ >= 0 && fila_sel_ < static_cast<int>(mat.composicion.size())) {
            if (col_sel_ == 0) {
                mat.composicion[fila_sel_].nombre = buffer_formula_bar_;
            } else if (col_sel_ == 1) {
                try { mat.composicion[fila_sel_].wt_pct = std::stod(buffer_formula_bar_); } catch (...) {}
            } else if (col_sel_ == 6) {
                mat.composicion[fila_sel_].referencia = buffer_formula_bar_;
            }
        }
    }

    ImGui::PopStyleColor(); // FrameBg
    ImGui::EndChild();
    ImGui::PopStyleVar();
    ImGui::PopStyleColor(2);

    (void)especies;
    (void)elementos;
}

void PanelMaterialDB::dibujar_panel_izquierdo() {
    ImGui::BeginChild("##MatDBLeftSidebar", ImVec2(275.0f, 0.0f), true);

    ImGui::TextColored(ImVec4(0.32f, 0.44f, 0.16f, 1.0f), "Material Catalog (%zu)", db_.catalogo().size());
    ImGui::Separator();

    // Píldoras CSS de filtro por categoría
    for (int c = 0; c < kNumCategoriasHSC; ++c) {
        if (c > 0 && (c % 3) != 0) ImGui::SameLine(0.0f, 4.0f);
        if (css::category_pill(kCategoriasHSC[c], idx_categoria_filtro_ == c)) {
            idx_categoria_filtro_ = c;
        }
    }

    ImGui::Spacing();
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputTextWithHint("##SearchMat", "Filter by name, category or code...", filtro_busqueda_, sizeof(filtro_busqueda_));

    ImGui::Spacing();
    ImGui::BeginChild("##ListaMateriales", ImVec2(0.0f, -195.0f), true);
    auto& catalogo = db_.catalogo();
    for (int i = 0; i < static_cast<int>(catalogo.size()); ++i) {
        const auto& m = catalogo[i];
        if (idx_categoria_filtro_ > 0 && m.categoria != kCategoriasHSC[idx_categoria_filtro_]) {
            continue;
        }
        if (!contiene_icase(m.nombre, filtro_busqueda_) &&
            !contiene_icase(m.categoria, filtro_busqueda_) &&
            !contiene_icase(m.codigo, filtro_busqueda_)) {
            continue;
        }

        const bool seleccionado = (i == idx_material_);
        ImGui::PushID(i);
        std::string etiqueta = m.nombre + "\n  " + m.categoria + " · " + m.codigo;
        if (ImGui::Selectable(etiqueta.c_str(), seleccionado, 0, ImVec2(0.0f, 34.0f))) {
            idx_material_ = i;
            ref_celda_activa_ = "A7";
            buffer_formula_bar_[0] = '\0';
        }
        ImGui::PopID();
    }
    ImGui::EndChild();

    ImGui::Separator();
    ImGui::TextColored(ImVec4(0.25f, 0.32f, 0.42f, 1.0f), "Create / Manage Material");
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::InputTextWithHint("##NewMatName", "New material name", nuevo_mat_nombre_, sizeof(nuevo_mat_nombre_));
    ImGui::SetNextItemWidth(-1.0f);
    ImGui::Combo("##NewMatCat", &nuevo_mat_cat_idx_, kCategoriasHSC + 1, kNumCategoriasHSC - 1);

    if (ImGui::Button("+ Create Material", ImVec2(-1.0f, 0.0f))) {
        if (nuevo_mat_nombre_[0] != '\0') {
            cma::application::modules::RegistroMaterial nuevo;
            nuevo.nombre = nuevo_mat_nombre_;
            nuevo.categoria = kCategoriasHSC[nuevo_mat_cat_idx_ + 1];
            nuevo.codigo = nuevo_mat_codigo_;
            nuevo.descripcion = "User-defined raw material for HSC Sim";
            nuevo.composicion = {
                {"[Concentrate]", 50.000, "Primary concentrate"},
                {"SiO2",          30.000, "Silica"},
                {"Fe2O3",         15.000, "Iron oxide"},
                {"H2O",            5.000, "Moisture"}
            };
            nuevo.propiedades_fisicas = {
                {"Bulk Density", 1600.0, "kg/m3", "Default", false},
                {"Moisture Content", 5.0, "wt-%", "Default", false}
            };
            db_.agregar_o_actualizar(nuevo);
            idx_material_ = static_cast<int>(db_.catalogo().size()) - 1;
            mensaje_estado_ = "Material '" + nuevo.nombre + "' creado.";
            estado_ok_ = true;
        }
    }

    const float half_w = (ImGui::GetContentRegionAvail().x - 6.0f) * 0.5f;
    if (ImGui::Button("Duplicate", ImVec2(half_w, 0.0f))) {
        if (idx_material_ >= 0 && idx_material_ < static_cast<int>(db_.catalogo().size())) {
            auto copia = db_.catalogo()[idx_material_];
            copia.nombre += " (Copy)";
            db_.agregar_o_actualizar(copia);
            idx_material_ = static_cast<int>(db_.catalogo().size()) - 1;
            mensaje_estado_ = "Material duplicado como '" + copia.nombre + "'.";
            estado_ok_ = true;
        }
    }
    ImGui::SameLine(0.0f, 6.0f);
    ImGui::BeginDisabled(db_.catalogo().size() <= 1);
    if (ImGui::Button("Delete", ImVec2(half_w, 0.0f))) {
        std::string borrado = db_.catalogo()[idx_material_].nombre;
        db_.eliminar(borrado);
        if (idx_material_ >= static_cast<int>(db_.catalogo().size())) {
            idx_material_ = static_cast<int>(db_.catalogo().size()) - 1;
        }
        mensaje_estado_ = "Material '" + borrado + "' eliminado.";
        estado_ok_ = true;
    }
    ImGui::EndDisabled();

    ImGui::EndChild();
}

void PanelMaterialDB::dibujar_hoja_composicion(
    cma::application::modules::RegistroMaterial& mat,
    const std::vector<cma::application::modules::EspecieResuelta>& especies,
    const std::vector<cma::application::modules::ElementoResuelto>& elementos)
{
    const double total_comp = cma::application::modules::MaterialDB::suma_composicion(mat);
    const double total_esp  = cma::application::modules::MaterialDB::suma_especies(especies);
    const double total_elem = cma::application::modules::MaterialDB::suma_elementos(elementos);

    // Colores idénticos a la hoja HSC Chemistry Material Database (media_1793653450161_312dac09.png)
    const ImU32 col_hdr_excel   = IM_COL32(230, 232, 236, 255); // Encabezado gris claro A..H
    const ImU32 col_row_num     = IM_COL32(235, 236, 239, 255); // Numeración de filas 1..16
    const ImU32 col_olive_band  = IM_COL32(107, 123, 55, 255);  // Verde oliva HSC para filas 1 y 2
    const ImU32 col_edit_white  = IM_COL32(255, 255, 255, 255); // Celdas editables blancas (Cols A, B, G, H)
    const ImU32 col_calc_gray   = IM_COL32(206, 208, 211, 255); // Celdas calculadas grises (Cols C, D, E, F)
    const ImU32 col_active_cell = IM_COL32(215, 234, 254, 255); // Celda seleccionada

    ImGui::PushStyleColor(ImGuiCol_TableBorderStrong, IM_COL32(142, 148, 158, 255));
    ImGui::PushStyleColor(ImGuiCol_TableBorderLight,  IM_COL32(178, 184, 192, 255));
    ImGui::PushStyleColor(ImGuiCol_HeaderHovered,     IM_COL32(190, 205, 150, 255));
    ImGui::PushStyleVar(ImGuiStyleVar_CellPadding, ImVec2(6.0f, 3.5f));

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders |
                                  ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_Resizable |
                                  ImGuiTableFlags_ScrollY;

    const size_t max_data_rows = std::max<size_t>({14, mat.composicion.size() + 2, especies.size(), elementos.size()});

    if (ImGui::BeginTable("##HSCSpreadsheetGrid", 9, flags, ImVec2(0.0f, -94.0f))) {
        ImGui::TableSetupScrollFreeze(0, 3); // Congelar encabezado A..H + Fila 1 + Fila 2 (Total)
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 30.0f);
        ImGui::TableSetupColumn("A", ImGuiTableColumnFlags_WidthFixed, 132.0f);
        ImGui::TableSetupColumn("B", ImGuiTableColumnFlags_WidthFixed, 76.0f);
        ImGui::TableSetupColumn("C", ImGuiTableColumnFlags_WidthFixed, 92.0f);
        ImGui::TableSetupColumn("D", ImGuiTableColumnFlags_WidthFixed, 76.0f);
        ImGui::TableSetupColumn("E", ImGuiTableColumnFlags_WidthFixed, 82.0f);
        ImGui::TableSetupColumn("F", ImGuiTableColumnFlags_WidthFixed, 76.0f);
        ImGui::TableSetupColumn("G", ImGuiTableColumnFlags_WidthStretch, 1.2f);
        ImGui::TableSetupColumn("H", ImGuiTableColumnFlags_WidthFixed, 68.0f);

        // Encabezado de columnas A..H estilo hoja de cálculo
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        const char* letras_col[9] = {"", "A", "B", "C", "D", "E", "F", "G", "H"};
        for (int c = 0; c < 9; ++c) {
            ImGui::TableSetColumnIndex(c);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_hdr_excel);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(30, 30, 30, 255));
            const float avail = ImGui::GetContentRegionAvail().x;
            const float tw = ImGui::CalcTextSize(letras_col[c]).x;
            if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + (avail - tw) * 0.5f);
            ImGui::TextUnformatted(letras_col[c]);
            ImGui::PopStyleColor();
        }

        // ── FILA 1: Títulos de bloque en franja verde oliva ──────────────────
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_row_num);
        ImGui::TextColored(ImVec4(0.15f, 0.15f, 0.15f, 1.0f), " 1");

        const char* titulos_fila1[8] = {
            "Composition", "wt-%", "Species", "wt-%", "Elements", "wt-%", "Reference", ""
        };
        for (int c = 0; c < 8; ++c) {
            ImGui::TableSetColumnIndex(c + 1);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_olive_band);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
            if (c == 1 || c == 3 || c == 5) {
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(titulos_fila1[c]).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
            }
            ImGui::TextUnformatted(titulos_fila1[c]);
            ImGui::PopStyleColor();
        }

        // ── FILA 2: Totales en franja verde oliva ────────────────────────────
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_row_num);
        ImGui::TextColored(ImVec4(0.15f, 0.15f, 0.15f, 1.0f), " 2");

        for (int c = 0; c < 8; ++c) {
            ImGui::TableSetColumnIndex(c + 1);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_olive_band);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
            char buf[32] = "";
            if (c == 0) {
                std::snprintf(buf, sizeof(buf), "Total");
                ImGui::TextUnformatted(buf);
            } else if (c == 1) {
                std::snprintf(buf, sizeof(buf), "%.3f", total_comp);
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(buf).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
                ImGui::TextUnformatted(buf);
            } else if (c == 3) {
                std::snprintf(buf, sizeof(buf), "%.3f", total_esp);
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(buf).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
                ImGui::TextUnformatted(buf);
            } else if (c == 5) {
                std::snprintf(buf, sizeof(buf), "%.3f", total_elem);
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(buf).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
                ImGui::TextUnformatted(buf);
            } else {
                ImGui::TextUnformatted("");
            }
            ImGui::PopStyleColor();
        }

        // ── FILAS 3..N: Datos de Composition (A-B), Species (C-D), Elements (E-F), Reference (G) ──
        int idx_eliminar = -1;
        for (size_t r = 0; r < max_data_rows; ++r) {
            const int num_fila_excel = static_cast<int>(r) + 3;
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(r));

            // Columna 0: Número de fila (3..16+)
            ImGui::TableSetColumnIndex(0);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_row_num);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(35, 35, 35, 255));
            ImGui::Text("%2d", num_fila_excel);
            ImGui::PopStyleColor();

            // ── Columna A: Composition (Nombre de [Sub-Material] o Especie) ──
            ImGui::TableSetColumnIndex(1);
            const bool sel_a = (fila_sel_ == static_cast<int>(r) && col_sel_ == 0);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, sel_a ? col_active_cell : col_edit_white);
            if (r < mat.composicion.size()) {
                char buf_nom[128];
                std::snprintf(buf_nom, sizeof(buf_nom), "%s", mat.composicion[r].nombre.c_str());
                ImGui::PushStyleColor(ImGuiCol_FrameBg, IM_COL32(0, 0, 0, 0));
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(15, 15, 15, 255));
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputText("##CompName", buf_nom, sizeof(buf_nom))) {
                    mat.composicion[r].nombre = buf_nom;
                }
                if (ImGui::IsItemActivated() || ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 0;
                    ref_celda_activa_ = "A" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%s", mat.composicion[r].nombre.c_str());
                }
                ImGui::PopStyleColor(2);
            } else {
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(20, 20, 20, 255));
                if (ImGui::Selectable("##EmptyA", sel_a, 0, ImVec2(0, 18))) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 0;
                    ref_celda_activa_ = "A" + std::to_string(num_fila_excel);
                    buffer_formula_bar_[0] = '\0';
                }
                ImGui::PopStyleColor();
            }

            // ── Columna B: Composition wt-% (Editable) ──
            ImGui::TableSetColumnIndex(2);
            const bool sel_b = (fila_sel_ == static_cast<int>(r) && col_sel_ == 1);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, sel_b ? col_active_cell : col_edit_white);
            if (r < mat.composicion.size()) {
                ImGui::PushStyleColor(ImGuiCol_FrameBg, IM_COL32(0, 0, 0, 0));
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(15, 15, 15, 255));
                ImGui::SetNextItemWidth(-1.0f);
                double val = mat.composicion[r].wt_pct;
                if (ImGui::InputDouble("##CompWt", &val, 0.0, 0.0, "%.3f")) {
                    mat.composicion[r].wt_pct = std::max(0.0, val);
                }
                if (ImGui::IsItemActivated() || ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 1;
                    ref_celda_activa_ = "B" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%.3f", mat.composicion[r].wt_pct);
                }
                ImGui::PopStyleColor(2);
            } else {
                if (ImGui::Selectable("##EmptyB", sel_b, 0, ImVec2(0, 18))) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 1;
                    ref_celda_activa_ = "B" + std::to_string(num_fila_excel);
                    buffer_formula_bar_[0] = '\0';
                }
            }

            // ── Columna C: Species (Calculada automáticamente, fondo gris) ──
            ImGui::TableSetColumnIndex(3);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_calc_gray);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(20, 20, 20, 255));
            if (r < especies.size()) {
                ImGui::TextUnformatted(especies[r].formula.c_str());
                if (ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 2;
                    ref_celda_activa_ = "C" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%s", especies[r].formula.c_str());
                }
            } else {
                ImGui::TextUnformatted("");
            }
            ImGui::PopStyleColor();

            // ── Columna D: Species wt-% (Calculada automáticamente, alineada a la derecha) ──
            ImGui::TableSetColumnIndex(4);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_calc_gray);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(20, 20, 20, 255));
            if (r < especies.size()) {
                char buf[32];
                std::snprintf(buf, sizeof(buf), "%.3f", especies[r].wt_pct);
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(buf).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
                ImGui::TextUnformatted(buf);
                if (ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 3;
                    ref_celda_activa_ = "D" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%.3f", especies[r].wt_pct);
                }
            } else {
                ImGui::TextUnformatted("");
            }
            ImGui::PopStyleColor();

            // ── Columna E: Elements (Calculada automáticamente, fondo gris) ──
            ImGui::TableSetColumnIndex(5);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_calc_gray);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(20, 20, 20, 255));
            if (r < elementos.size()) {
                ImGui::TextUnformatted(elementos[r].simbolo.c_str());
                if (ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 4;
                    ref_celda_activa_ = "E" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%s", elementos[r].simbolo.c_str());
                }
            } else {
                ImGui::TextUnformatted("");
            }
            ImGui::PopStyleColor();

            // ── Columna F: Elements wt-% (Calculada automáticamente, alineada a la derecha) ──
            ImGui::TableSetColumnIndex(6);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_calc_gray);
            ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(20, 20, 20, 255));
            if (r < elementos.size()) {
                char buf[32];
                std::snprintf(buf, sizeof(buf), "%.3f", elementos[r].wt_pct);
                const float avail = ImGui::GetContentRegionAvail().x;
                const float tw = ImGui::CalcTextSize(buf).x;
                if (avail > tw) ImGui::SetCursorPosX(ImGui::GetCursorPosX() + avail - tw);
                ImGui::TextUnformatted(buf);
                if (ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 5;
                    ref_celda_activa_ = "F" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%.3f", elementos[r].wt_pct);
                }
            } else {
                ImGui::TextUnformatted("");
            }
            ImGui::PopStyleColor();

            // ── Columna G: Reference (Editable, blanca) ──
            ImGui::TableSetColumnIndex(7);
            const bool sel_g = (fila_sel_ == static_cast<int>(r) && col_sel_ == 6);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, sel_g ? col_active_cell : col_edit_white);
            if (r < mat.composicion.size()) {
                char buf_ref[128];
                std::snprintf(buf_ref, sizeof(buf_ref), "%s", mat.composicion[r].referencia.c_str());
                ImGui::PushStyleColor(ImGuiCol_FrameBg, IM_COL32(0, 0, 0, 0));
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(45, 45, 45, 255));
                ImGui::SetNextItemWidth(-1.0f);
                if (ImGui::InputText("##CompRef", buf_ref, sizeof(buf_ref))) {
                    mat.composicion[r].referencia = buf_ref;
                }
                if (ImGui::IsItemActivated() || ImGui::IsItemClicked()) {
                    fila_sel_ = static_cast<int>(r);
                    col_sel_ = 6;
                    ref_celda_activa_ = "G" + std::to_string(num_fila_excel);
                    std::snprintf(buffer_formula_bar_, sizeof(buffer_formula_bar_), "%s", mat.composicion[r].referencia.c_str());
                }
                ImGui::PopStyleColor(2);
            } else if (r < especies.size() && !especies[r].referencia.empty()) {
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(90, 90, 90, 255));
                ImGui::TextUnformatted(especies[r].referencia.c_str());
                ImGui::PopStyleColor();
            } else {
                ImGui::TextUnformatted("");
            }

            // ── Columna H: Acción rápida (Abrir sub-material o eliminar fila) ──
            ImGui::TableSetColumnIndex(8);
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, col_edit_white);
            if (r < mat.composicion.size()) {
                if (mat.composicion[r].es_submaterial()) {
                    const std::string sub = mat.composicion[r].nombre_submaterial();
                    ImGui::PushStyleColor(ImGuiCol_Button, IM_COL32(107, 123, 55, 220));
                    ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
                    if (ImGui::SmallButton("Go")) {
                        for (int k = 0; k < static_cast<int>(db_.catalogo().size()); ++k) {
                            if (db_.catalogo()[k].nombre == sub) {
                                idx_material_ = k;
                                break;
                            }
                        }
                    }
                    ImGui::PopStyleColor(2);
                    ImGui::SameLine();
                }
                ImGui::PushStyleColor(ImGuiCol_Button, IM_COL32(195, 72, 68, 215));
                ImGui::PushStyleColor(ImGuiCol_Text, IM_COL32(255, 255, 255, 255));
                if (ImGui::SmallButton("X")) {
                    idx_eliminar = static_cast<int>(r);
                }
                ImGui::PopStyleColor(2);
            }

            ImGui::PopID();
        }

        if (idx_eliminar >= 0 && idx_eliminar < static_cast<int>(mat.composicion.size())) {
            mat.composicion.erase(mat.composicion.begin() + idx_eliminar);
        }

        ImGui::EndTable();
    }

    ImGui::PopStyleVar();
    ImGui::PopStyleColor(3);

    // Barras visuales apiladas tipo CSS (Species Breakdown + Elemental Breakdown)
    ImGui::Columns(2, "##StackedBarsCols", false);
    {
        std::vector<css::StackedSegment> seg_esp;
        for (const auto& s : especies) seg_esp.push_back({s.formula, s.wt_pct});
        css::draw_stacked_composition_bar("Species Distribution (wt-%)", seg_esp, 20.0f);
    }
    ImGui::NextColumn();
    {
        std::vector<css::StackedSegment> seg_el;
        for (const auto& e : elementos) seg_el.push_back({e.simbolo, e.wt_pct});
        css::draw_stacked_composition_bar("Elemental Assay Distribution (wt-%)", seg_el, 20.0f);
    }
    ImGui::Columns(1);

    // Barra inferior para agregar rápidamente una especie química directa
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Quick Add Species:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(110.0f);
    ImGui::InputTextWithHint("##AddSpFormula", "e.g. CuFeS2", nueva_especie_buf_, sizeof(nueva_especie_buf_));
    ImGui::SameLine();
    ImGui::SetNextItemWidth(85.0f);
    ImGui::InputDouble("##AddSpWt", &nuevo_wt_pct_, 0.0, 0.0, "%.3f wt-%%");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(160.0f);
    ImGui::InputTextWithHint("##AddSpRef", "Reference / Mineral", nueva_ref_buf_, sizeof(nueva_ref_buf_));
    ImGui::SameLine();
    if (ImGui::Button("Insert into Composition")) {
        if (nueva_especie_buf_[0] != '\0') {
            mat.composicion.push_back({nueva_especie_buf_, std::max(0.0, nuevo_wt_pct_), nueva_ref_buf_});
            nueva_especie_buf_[0] = '\0';
            nueva_ref_buf_[0] = '\0';
        }
    }
}

void PanelMaterialDB::dibujar_pestana_propiedades(cma::application::modules::RegistroMaterial& mat) {
    ImGui::Columns(2, "##ColsProps", true);

    // Columna 1: Metadatos + Propiedades físicas + Análisis
    ImGui::TextColored(ImVec4(0.32f, 0.44f, 0.16f, 1.0f), "Material Metadata & Physical Properties");
    char cod_buf[64], desc_buf[256], ref_buf[256];
    std::snprintf(cod_buf, sizeof(cod_buf), "%s", mat.codigo.c_str());
    std::snprintf(desc_buf, sizeof(desc_buf), "%s", mat.descripcion.c_str());
    std::snprintf(ref_buf, sizeof(ref_buf), "%s", mat.referencia.c_str());

    if (ImGui::InputText("Code", cod_buf, sizeof(cod_buf))) mat.codigo = cod_buf;
    if (ImGui::InputText("Description", desc_buf, sizeof(desc_buf))) mat.descripcion = desc_buf;
    if (ImGui::InputText("Reference", ref_buf, sizeof(ref_buf))) mat.referencia = ref_buf;

    ImGui::Spacing();
    if (ImGui::BeginTable("##TblPhysProps", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Physical Property", ImGuiTableColumnFlags_WidthStretch, 1.4f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 75.0f);
        ImGui::TableSetupColumn("Reference", ImGuiTableColumnFlags_WidthStretch, 1.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < mat.propiedades_fisicas.size(); ++i) {
            auto& p = mat.propiedades_fisicas[i];
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(i));
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted(p.nombre.c_str());
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##pv", &p.valor, 0.0, 0.0, "%.3f");
            ImGui::TableSetColumnIndex(2);
            ImGui::TextUnformatted(p.unidad.c_str());
            ImGui::TableSetColumnIndex(3);
            char rbuf[128];
            std::snprintf(rbuf, sizeof(rbuf), "%s", p.referencia.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##pr", rbuf, sizeof(rbuf))) p.referencia = rbuf;
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    if (ImGui::Button("+ Add Physical Property")) {
        mat.propiedades_fisicas.push_back({"Thermal Conductivity", 1.25, "W/(m.K)", "Measured", false});
    }

    ImGui::Spacing();
    ImGui::TextColored(ImVec4(0.32f, 0.44f, 0.16f, 1.0f), "Chemical & Proximate Analyses (Assays)");
    if (ImGui::BeginTable("##TblAnalyses", 4, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Analysis / Assay", ImGuiTableColumnFlags_WidthStretch, 1.4f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 75.0f);
        ImGui::TableSetupColumn("Method / Ref", ImGuiTableColumnFlags_WidthStretch, 1.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < mat.analisis.size(); ++i) {
            auto& a = mat.analisis[i];
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(i + 500));
            ImGui::TableSetColumnIndex(0);
            char nbuf[128];
            std::snprintf(nbuf, sizeof(nbuf), "%s", a.nombre.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##an", nbuf, sizeof(nbuf))) a.nombre = nbuf;
            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##av", &a.valor, 0.0, 0.0, "%.3f");
            ImGui::TableSetColumnIndex(2);
            ImGui::TextUnformatted(a.unidad.c_str());
            ImGui::TableSetColumnIndex(3);
            ImGui::TextUnformatted(a.referencia.c_str());
            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    ImGui::NextColumn();

    // Columna 2: Propiedades definidas por el usuario (User-Defined Properties)
    ImGui::TextColored(ImVec4(0.16f, 0.36f, 0.62f, 1.0f), "User-Defined Properties");
    ImGui::TextWrapped("Create custom properties (Bond Work Index, smelter penalties, precious metal credits, "
                       "emission factors, etc.) stored alongside the material for HSC Sim process models.");
    ImGui::Spacing();

    int idx_borrar_usr = -1;
    if (ImGui::BeginTable("##TblUserProps", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("User Property", ImGuiTableColumnFlags_WidthStretch, 1.3f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthFixed, 85.0f);
        ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 75.0f);
        ImGui::TableSetupColumn("Reference", ImGuiTableColumnFlags_WidthStretch, 1.0f);
        ImGui::TableSetupColumn("Del", ImGuiTableColumnFlags_WidthFixed, 34.0f);
        ImGui::TableHeadersRow();

        for (size_t i = 0; i < mat.propiedades_usuario.size(); ++i) {
            auto& u = mat.propiedades_usuario[i];
            ImGui::TableNextRow();
            ImGui::PushID(static_cast<int>(i + 1000));
            ImGui::TableSetColumnIndex(0);
            char nbuf[128];
            std::snprintf(nbuf, sizeof(nbuf), "%s", u.nombre.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##un", nbuf, sizeof(nbuf))) u.nombre = nbuf;

            ImGui::TableSetColumnIndex(1);
            ImGui::SetNextItemWidth(-1.0f);
            ImGui::InputDouble("##uv", &u.valor, 0.0, 0.0, "%.3f");

            ImGui::TableSetColumnIndex(2);
            char ubuf[64];
            std::snprintf(ubuf, sizeof(ubuf), "%s", u.unidad.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##uu", ubuf, sizeof(ubuf))) u.unidad = ubuf;

            ImGui::TableSetColumnIndex(3);
            char rbuf[128];
            std::snprintf(rbuf, sizeof(rbuf), "%s", u.referencia.c_str());
            ImGui::SetNextItemWidth(-1.0f);
            if (ImGui::InputText("##ur", rbuf, sizeof(rbuf))) u.referencia = rbuf;

            ImGui::TableSetColumnIndex(4);
            if (ImGui::SmallButton("X")) idx_borrar_usr = static_cast<int>(i);
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    if (idx_borrar_usr >= 0 && idx_borrar_usr < static_cast<int>(mat.propiedades_usuario.size())) {
        mat.propiedades_usuario.erase(mat.propiedades_usuario.begin() + idx_borrar_usr);
    }

    ImGui::Separator();
    ImGui::TextColored(ImVec4(0.25f, 0.32f, 0.42f, 1.0f), "New User-Defined Property");
    ImGui::InputText("Property Name", prop_usr_nombre_, sizeof(prop_usr_nombre_));
    ImGui::InputDouble("Numeric Value", &prop_usr_valor_, 0.1, 1.0, "%.3f");
    ImGui::InputText("Unit", prop_usr_unidad_, sizeof(prop_usr_unidad_));
    ImGui::InputText("Reference / Note", prop_usr_ref_, sizeof(prop_usr_ref_));
    if (ImGui::Button("+ Add User-Defined Property")) {
        if (prop_usr_nombre_[0] != '\0') {
            mat.propiedades_usuario.push_back({
                prop_usr_nombre_, prop_usr_valor_, prop_usr_unidad_, prop_usr_ref_, true
            });
            mensaje_estado_ = std::string("Propiedad personalizada '") + prop_usr_nombre_ + "' agregada.";
            estado_ok_ = true;
        }
    }

    ImGui::Columns(1);
}

void PanelMaterialDB::inyectar_en_flowsheet(
    const cma::application::modules::RegistroMaterial& mat,
    const std::vector<cma::application::modules::EspecieResuelta>& especies,
    const std::vector<cma::application::modules::ElementoResuelto>& elementos)
{
    if (!flowsheet_) {
        mensaje_estado_ = "No hay un Flowsheet activo disponible.";
        estado_ok_ = false;
        return;
    }

    nucleo::Flujo* flujo_dest = nullptr;
    auto& mapa = flowsheet_->mapa_flujos();
    std::vector<int64_t> ids;
    for (const auto& [id, f] : mapa) {
        if (!f.es_energia) ids.push_back(id);
    }

    if (idx_flujo_destino_ == 0 || ids.empty()) {
        const int64_t id_fuente = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Feed: " + mat.nombre);
        const int64_t id_sumidero = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "To Process: " + mat.nombre);
        if (auto* nf = flowsheet_->nodo_por_id(id_fuente)) {
            nf->canvas_x = 80.0f;
            nf->canvas_y = 120.0f;
        }
        if (auto* ns = flowsheet_->nodo_por_id(id_sumidero)) {
            ns->canvas_x = 320.0f;
            ns->canvas_y = 120.0f;
        }
        const int64_t id_flujo = flowsheet_->agregar_flujo(mat.nombre);
        flowsheet_->conectar(id_fuente, id_sumidero, id_flujo);
        flujo_dest = flowsheet_->flujo_por_id(id_flujo);
    } else {
        const int idx = std::clamp(idx_flujo_destino_ - 1, 0, static_cast<int>(ids.size()) - 1);
        flujo_dest = flowsheet_->flujo_por_id(ids[idx]);
    }

    if (!flujo_dest) {
        mensaje_estado_ = "No se pudo obtener la corriente de destino.";
        estado_ok_ = false;
        return;
    }

    flujo_dest->nombre = mat.nombre;
    flujo_dest->temperatura = temp_sim_c_ + nucleo::CERO_KELVIN;
    flujo_dest->flujo_molar.clear();
    flujo_dest->flujo_solido.clear();
    flujo_dest->flujo_liquido.clear();
    flujo_dest->flujo_gas.clear();
    flujo_dest->particulas.clear();
    flujo_dest->agua_kg_h = 0.0;
    flujo_dest->mb_assays.clear();

    const double masa_total_kg_h = std::max(0.0, caudal_sim_t_h_) * 1000.0;
    double d80_um = 75.0;
    for (const auto& p : mat.propiedades_fisicas) {
        if (contiene_icase(p.nombre, "d80") || contiene_icase(p.nombre, "particle")) {
            if (p.valor > 0.0) d80_um = p.valor;
        }
    }

    double agua_kg_h = 0.0;
    double solidos_kg_h = 0.0;

    for (const auto& esp : especies) {
        const double masa_esp_kg_h = masa_total_kg_h * (esp.wt_pct / 100.0);
        if (masa_esp_kg_h <= 0.0) continue;

        if (esp.formula == "H2O" || esp.formula == "H2O(l)") {
            agua_kg_h += masa_esp_kg_h;
        } else {
            solidos_kg_h += masa_esp_kg_h;
            flujo_dest->particulas.push_back({esp.formula, d80_um, masa_esp_kg_h});
        }

        if (const auto* sp_db = repo_.especie_por_nombre(esp.formula)) {
            const double mw_kg_kmol = (sp_db->masa_molar > 1e-6) ? sp_db->masa_molar : 50.0;
            const double kmol_s = (masa_esp_kg_h / 3600.0) / mw_kg_kmol;
            flujo_dest->flujo_molar[sp_db->id] += kmol_s;
            if (esp.formula == "H2O" || esp.formula == "H2O(l)") {
                flujo_dest->flujo_liquido[sp_db->id] += kmol_s;
            } else {
                flujo_dest->flujo_solido[sp_db->id] += kmol_s;
            }
        }
    }

    flujo_dest->agua_kg_h = agua_kg_h;
    flujo_dest->measured_flow = solidos_kg_h / 1000.0;
    flujo_dest->measured_water = agua_kg_h / 1000.0;
    flujo_dest->fase = (agua_kg_h > 0.0 && solidos_kg_h > 0.0) ? nucleo::Fase::Mixto : nucleo::Fase::Solido;

    for (const auto& el : elementos) {
        flujo_dest->mb_assays[el.simbolo] = el.wt_pct / 100.0;
    }

    mensaje_estado_ = "Material '" + mat.nombre + "' inyectado en HSC-Sim Flowsheet (" +
                      std::to_string(static_cast<int>(caudal_sim_t_h_)) + " t/h).";
    estado_ok_ = true;
}

void PanelMaterialDB::dibujar_pestana_integracion_sim(
    const cma::application::modules::RegistroMaterial& mat,
    const std::vector<cma::application::modules::EspecieResuelta>& especies,
    const std::vector<cma::application::modules::ElementoResuelto>& elementos)
{
    ImGui::TextColored(ImVec4(0.32f, 0.44f, 0.16f, 1.0f),
                       "Direct Raw Material Feed Integration with HSC Sim & Heat Balance (Bal)");
    ImGui::TextWrapped("Materials stored in the database (concentrates, fluxes, fuels, sludges, steels, "
                       "plastics, e-waste and slags) can be injected directly as raw material feed streams "
                       "into HSC Sim process models or Heat and Material Balances.");
    ImGui::Spacing();

    ImGui::SetNextItemWidth(180.0f);
    ImGui::InputDouble("Feed Mass Flow Rate (t/h)", &caudal_sim_t_h_, 5.0, 25.0, "%.2f t/h");
    ImGui::SetNextItemWidth(180.0f);
    ImGui::InputDouble("Feed Temperature (°C)", &temp_sim_c_, 5.0, 25.0, "%.1f C");

    std::vector<std::string> opciones_flujo = {"[Create new Feed Stream in HSC-Sim]"};
    if (flowsheet_) {
        for (const auto& [id, f] : flowsheet_->mapa_flujos()) {
            if (!f.es_energia) {
                opciones_flujo.push_back("Stream #" + std::to_string(id) + ": " + f.nombre);
            }
        }
    }
    if (idx_flujo_destino_ >= static_cast<int>(opciones_flujo.size())) {
        idx_flujo_destino_ = 0;
    }

    ImGui::SetNextItemWidth(340.0f);
    if (ImGui::BeginCombo("Target Stream (HSC-Sim)", opciones_flujo[idx_flujo_destino_].c_str())) {
        for (int i = 0; i < static_cast<int>(opciones_flujo.size()); ++i) {
            bool sel = (i == idx_flujo_destino_);
            if (ImGui::Selectable(opciones_flujo[i].c_str(), sel)) {
                idx_flujo_destino_ = i;
            }
        }
        ImGui::EndCombo();
    }

    ImGui::Spacing();
    if (ImGui::Button("Send Material to HSC-Sim Flowsheet", ImVec2(280.0f, 32.0f))) {
        inyectar_en_flowsheet(mat, especies, elementos);
    }
    ImGui::SameLine();
    if (ImGui::Button("Send to Heat & Material Balances (Bal)", ImVec2(290.0f, 32.0f))) {
        if (al_enviar_a_balance) {
            al_enviar_a_balance(mat, caudal_sim_t_h_ * 1000.0, temp_sim_c_);
            mensaje_estado_ = "Material '" + mat.nombre + "' enviado a Heat and Material Balances.";
            estado_ok_ = true;
        }
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::TextDisabled("Preview of Species & Elemental Mass Flows for %.2f t/h (%.0f kg/h):",
                        caudal_sim_t_h_, caudal_sim_t_h_ * 1000.0);

    if (ImGui::BeginTable("##PreviewSimFlows", 5, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Species", ImGuiTableColumnFlags_WidthFixed, 110.0f);
        ImGui::TableSetupColumn("wt-%", ImGuiTableColumnFlags_WidthFixed, 85.0f);
        ImGui::TableSetupColumn("Mass Flow (kg/h)", ImGuiTableColumnFlags_WidthFixed, 130.0f);
        ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 90.0f);
        ImGui::TableSetupColumn("Element Flow (kg/h)", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableHeadersRow();

        const size_t n_rows = std::max(especies.size(), elementos.size());
        const double total_kg_h = caudal_sim_t_h_ * 1000.0;
        for (size_t i = 0; i < n_rows; ++i) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            if (i < especies.size()) ImGui::TextUnformatted(especies[i].formula.c_str());
            ImGui::TableSetColumnIndex(1);
            if (i < especies.size()) ImGui::Text("%.3f", especies[i].wt_pct);
            ImGui::TableSetColumnIndex(2);
            if (i < especies.size()) ImGui::Text("%.2f", total_kg_h * especies[i].wt_pct / 100.0);
            ImGui::TableSetColumnIndex(3);
            if (i < elementos.size()) ImGui::TextUnformatted(elementos[i].simbolo.c_str());
            ImGui::TableSetColumnIndex(4);
            if (i < elementos.size()) ImGui::Text("%.2f (%.3f wt-%%)", total_kg_h * elementos[i].wt_pct / 100.0, elementos[i].wt_pct);
        }
        ImGui::EndTable();
    }
}

void PanelMaterialDB::renderizar(bool* abierto) {
    css::ScopedHSCTheme hsc_theme;

    ImGui::SetNextWindowSize(ImVec2(1180.0f, 720.0f), ImGuiCond_FirstUseEver);
    if (!ImGui::Begin("HSC Material Database (Mat) - Concentrates, Fuels, Sludges, Steels & Plastics", abierto)) {
        ImGui::End();
        return;
    }

    auto& catalogo = db_.catalogo();
    if (catalogo.empty()) {
        db_.restaurar_predeterminados();
    }
    idx_material_ = std::clamp(idx_material_, 0, static_cast<int>(catalogo.size()) - 1);

    dibujar_panel_izquierdo();
    ImGui::SameLine();

    ImGui::BeginChild("##MatDBRightMain", ImVec2(0.0f, 0.0f), false);
    auto& mat_actual = db_.catalogo()[idx_material_];
    const auto especies_resueltas = db_.resolver_especies(mat_actual);
    const auto elementos_resueltos = db_.resolver_elementos(especies_resueltas);

    dibujar_barra_herramientas(mat_actual);
    dibujar_barra_formulas(mat_actual, especies_resueltas, elementos_resueltos);

    if (!mensaje_estado_.empty()) {
        ImVec4 col = estado_ok_ ? ImVec4(0.18f, 0.52f, 0.22f, 1.0f) : ImVec4(0.78f, 0.22f, 0.20f, 1.0f);
        ImGui::TextColored(col, "%s", mensaje_estado_.c_str());
    }

    if (ImGui::BeginTabBar("##MatDBTabs")) {
        if (ImGui::BeginTabItem("Composition Sheet (Composition / Species / Elements)")) {
            dibujar_hoja_composicion(mat_actual, especies_resueltas, elementos_resueltos);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Physical Properties, Analyses & User-Defined")) {
            dibujar_pestana_propiedades(mat_actual);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("HSC-Sim & Balance Feed Integration")) {
            dibujar_pestana_integracion_sim(mat_actual, especies_resueltas, elementos_resueltos);
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }

    ImGui::EndChild();
    ImGui::End();
}

}} // namespace cma::interfaz

#pragma GCC diagnostic pop
