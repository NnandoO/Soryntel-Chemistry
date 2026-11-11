#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
#pragma GCC diagnostic ignored "-Wformat-truncation"
#include "shared.hpp"
#include "interfaz/panels/panel_sim_tools.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <fstream>
#include <set>

namespace cma { namespace interfaz {

namespace {

struct ModeloUnidadInfo {
    const char* nombre;
    const char* modo;
    const char* codigo;
    const char* version;
    const char* descripcion;
    nucleo::TipoNodo tipo_nodo;
    int categoria; // 0=Species, 1=Particles, 2=Others, 3=Imported
};

static const ModeloUnidadInfo kModelosBiblioteca[] = {
    {"Elemental Distribution", "Elemental Distribution", "1", "1.9", "Distribución elemental y de fases para unidades pirometalúrgicas y separadores generales.", nucleo::TipoNodo::Separador, 0},
    {"Reactions (Hydro)", "Reactions", "2", "1.9", "Modelo hidrometalúrgico basado en reacciones acuosas, enfriamiento/calentamiento y balance entálpico.", nucleo::TipoNodo::IntercambiadorCalor, 0},
    {"Chemical Reactions", "Reactions", "3", "2.1", "Reactor estequiométrico y cinético multifásico con conversión por etapas y balance de energía.", nucleo::TipoNodo::TanqueLixiviacion, 0},
    {"SimEquil (Gibbs)", "Equilibrium", "4", "2.0", "Reactor de equilibrio termodinámico por minimización directa de energía libre de Gibbs.", nucleo::TipoNodo::ReactorGibbs, 0},
    {"Solvent Extraction (SX)", "Reactions", "5", "1.8", "Mezclador-decantador de extracción por solventes e intercambio iónico selectivo.", nucleo::TipoNodo::SXMixerSettler, 0},
    {"Electrowinning Cell", "Reactions", "6", "1.8", "Celda de electroobtención catódica con eficiencia de corriente de Faraday.", nucleo::TipoNodo::CeldaElectroobtencion, 0},
    {"Comminution (Bond Ball Mill)", "Particles", "10", "2.2", "Molienda de minerales con ley energética de Bond (Wi) y distribución granulométrica PSD.", nucleo::TipoNodo::MolinoBolas, 1},
    {"Jaw / Cone Crusher", "Particles", "11", "2.0", "Trituración primaria y secundaria controlada por abertura CSS en milímetros.", nucleo::TipoNodo::TrituradoraMandibula, 1},
    {"Hydrocyclone (Plitt)", "Particles", "12", "2.1", "Clasificación centrífuga por tamaño de corte d50c, bypass de agua y nitidez de separación.", nucleo::TipoNodo::Hidrociclon, 1},
    {"Flotation Bank", "Particles", "13", "2.3", "Banco de celdas de flotación cinética de primer orden con recuperación de agua y arrastre.", nucleo::TipoNodo::CeldaFlotacion, 1},
    {"Thickener / Dewatering", "Particles", "14", "1.9", "Espesador continuo sólido-líquido con control de % de sólidos en descarga (underflow).", nucleo::TipoNodo::Espesador, 1},
    {"Stream Mixer", "General", "20", "1.5", "Mezclador adiabático o con carga térmica Q (kW) ponderado por capacidad calorífica Cp.", nucleo::TipoNodo::Mezclador, 2},
    {"Stream Splitter", "General", "21", "1.5", "Divisor de corrientes con fracciones másicas/molares configurables por rama.", nucleo::TipoNodo::Divisor, 2},
    {"Compressor / Turbine", "General", "22", "1.6", "Compresión o expansión politrópica/isoentrópica con eficiencia mecánica configurable.", nucleo::TipoNodo::Compresor, 2},
    {"External Excel / DLL Unit", "Imported", "30", "1.0", "Unidad externa acoplada mediante tabla parametrizada o submodelo importado.", nucleo::TipoNodo::ReactorEsteq, 3}
};

struct VariableStreamTableInfo {
    const char* clave;
    const char* nombre_mostrar;
    const char* unidad;
    bool es_particulas;
};

static const VariableStreamTableInfo kCatalogoVariablesStream[] = {
    // Minerals Processing (particles) — Fig. 9
    {"Total Solids Flow",    "Total Solids Flow",    "t/h",   true},
    {"Total Gas Flow",       "Total Gas Flow",       "t/h",   true},
    {"Total Liquid Flow",    "Total Liquid Flow",    "t/h",   true},
    {"Pulp Mass Flow",       "Pulp Mass Flow",       "t/h",   true},
    {"Pulp Volumetric Flow", "Pulp Volumetric Flow", "m3/h",  true},
    {"Solids SG",            "Solids SG",            "g/cm3", true},
    {"Liquid SG",            "Liquid SG",            "g/cm3", true},
    {"Pulp SG",              "Pulp SG",              "g/cm3", true},
    {"% Solids",             "% Solids",             "wt-%",  true},
    {"Solids Recovery",      "Solids Recovery",      "Rec-%", true},
    {"Liquid Recovery",      "Liquid Recovery",      "Rec-%", true},
    {"Cu Assay",             "Cu",                   "wt-%",  true},
    {"Fe Assay",             "Fe",                   "wt-%",  true},
    {"S Assay",              "S",                    "wt-%",  true},
    {"P80 Size",             "P80 Particle Size",    "um",    true},
    // Reactions / Thermochemical — Fig. 24-25
    {"Temperature",          "Temperature",          "°C",      false},
    {"Pressure",             "Pressure",             "bar",     false},
    {"Total Mass Flow",      "Amount",               "t/h",     false},
    {"Gas Volume Flow",      "Gas Flow",             "Nm3/h",   false},
    {"Enthalpy Flow",        "Enthalpy",             "kW",      false},
    {"Heat Content Flow",    "Heat Content Flow",    "kW",      false},
    {"Thermal Energy Flow",  "Thermal Energy Flow",  "kW",      false},
    {"Exergy Flow",          "Exergy",               "kW",      false},
    {"Heat Capacity",        "Heat Capacity",        "kJ/kgK",  false}
};

double evaluar_variable_corriente(const std::string& clave,
                                  const nucleo::Flujo& f,
                                  const domain::ThermoRepository& db,
                                  double ref_solids_t_h = 0.0,
                                  double ref_liquid_t_h = 0.0) {
    if (clave == "Total Solids Flow") return f.masa_solidos_t_h(db);
    if (clave == "Total Gas Flow") return f.masa_gas_t_h(db);
    if (clave == "Total Liquid Flow") return f.masa_liquido_t_h(db);
    if (clave == "Pulp Mass Flow") return f.masa_solidos_t_h(db) + f.masa_liquido_t_h(db);
    if (clave == "Pulp Volumetric Flow") return f.volumen_total_m3_h(db);
    if (clave == "Solids SG") return (f.masa_solidos_t_h(db) > 1e-9) ? 2.75 : 0.0;
    if (clave == "Liquid SG") return (f.masa_liquido_t_h(db) > 1e-9) ? 1.00 : 0.0;
    if (clave == "Pulp SG") return f.densidad_pulpa_g_cm3(db);
    if (clave == "% Solids") return f.porcentaje_solidos_wt(db);
    if (clave == "Solids Recovery") {
        double s = f.masa_solidos_t_h(db);
        return (ref_solids_t_h > 1e-9) ? std::min(100.0, (s / ref_solids_t_h) * 100.0) : (s > 0.0 ? 100.0 : 0.0);
    }
    if (clave == "Liquid Recovery") {
        double l = f.masa_liquido_t_h(db);
        return (ref_liquid_t_h > 1e-9) ? std::min(100.0, (l / ref_liquid_t_h) * 100.0) : (l > 0.0 ? 100.0 : 0.0);
    }
    if (clave == "Cu Assay") {
        auto it = f.mb_assays.find("Cu");
        return (it != f.mb_assays.end()) ? it->second : 0.0;
    }
    if (clave == "Fe Assay") {
        auto it = f.mb_assays.find("Fe");
        return (it != f.mb_assays.end()) ? it->second : (f.masa_solidos_t_h(db) > 0.0 ? 63.53 : 0.0);
    }
    if (clave == "S Assay") {
        auto it = f.mb_assays.find("S");
        return (it != f.mb_assays.end()) ? it->second : (f.masa_solidos_t_h(db) > 0.0 ? 36.47 : 0.0);
    }
    if (clave == "P80 Size") return f.p80_um();
    if (clave == "Temperature") return f.temperatura - 273.15;
    if (clave == "Pressure") return f.presion;
    if (clave == "Total Mass Flow") return f.flujo_masico_total(db) * 3.6;
    if (clave == "Gas Volume Flow") return f.volumen_gas_Nm3_h(db);
    if (clave == "Enthalpy Flow" || clave == "Thermal Energy Flow") return f.entalpia;
    if (clave == "Heat Content Flow") return f.calor_contenido_Hc_kW(db);
    if (clave == "Exergy Flow") return f.exergia_flujo_kW(db);
    if (clave == "Heat Capacity") return f.cp_especifico_kJ_kgK(db);
    return 0.0;
}

const VariableStreamTableInfo* buscar_info_variable(const std::string& clave) {
    for (const auto& v : kCatalogoVariablesStream) {
        if (clave == v.clave) return &v;
    }
    return nullptr;
}

} // namespace

PanelSimCommonTools::PanelSimCommonTools(const domain::ThermoRepository& db)
    : db_(db) {}

void PanelSimCommonTools::cargar_ejemplo_hydro(diagrama_flujo::Flowsheet& fs,
                                               motores::BalanceMasa& motor_bm,
                                               motores::ResultadoBalanceMasa& ultimo_res) {
    fs.limpiar();
    fs.info_proceso.project_name   = "Hydro_example";
    fs.info_proceso.project_path   = "Hydro_example.Sim";
    fs.info_proceso.process_name   = "Hydro Cooling & Leaching";
    fs.info_proceso.technology     = "Hydrometallurgy";
    fs.info_proceso.sub_technology = "Sulfuric Acid Leaching";
    fs.info_proceso.plant_type     = "Industrial Plant";
    fs.info_proceso.products       = "Hot Water, Pregnant Leach Solution (PLS)";

    auto buscar_id = [&](const char* formula) -> int64_t {
        if (const auto* sp = db_.especie_por_nombre(formula)) return sp->id;
        return -1;
    };
    const int64_t id_O2  = buscar_id("O2(g)") > 0 ? buscar_id("O2(g)") : buscar_id("O2");
    const int64_t id_N2  = buscar_id("N2(g)") > 0 ? buscar_id("N2(g)") : buscar_id("N2");

    // ── Unidad 1: Cooler (IntercambiadorCalor, calienta 197.38 t/h de agua de 25 °C a 60 °C, Fig. 24-25) ──
    int64_t n_cooler = fs.agregar_nodo(nucleo::TipoNodo::IntercambiadorCalor, "Cooler");
    if (auto* nc = fs.nodo_por_id(n_cooler)) {
        nc->canvas_x = 240.0f;
        nc->canvas_y = 140.0f;
        nc->params["T_salida"] = 273.15 + 60.0;
        nc->params["caida_presion"] = 0.0;
    }
    int64_t f_cold = fs.agregar_flujo("Cold Water");
    int64_t f_hot  = fs.agregar_flujo("Hot Water");
    if (auto* fc = fs.flujo_por_id(f_cold)) {
        fc->temperatura = 273.15 + 25.0;
        fc->presion     = 1.0;
        fc->agua_kg_h   = 197380.0; // 197.38 t/h
        fc->nodo_origen  = -1;
        fc->nodo_destino = n_cooler;
    }
    if (auto* fh = fs.flujo_por_id(f_hot)) {
        fh->temperatura  = 273.15 + 60.0;
        fh->presion      = 1.0;
        fh->nodo_origen  = n_cooler;
        fh->nodo_destino = -1;
    }
    if (auto* nc = fs.nodo_por_id(n_cooler)) {
        nc->ids_flujos_entrada.push_back(f_cold);
        nc->ids_flujos_salida.push_back(f_hot);
    }

    // ── Unidad 2: Leaching (TanqueLixiviacion: Air + FeS + H2SO4 -> Offgas + Solution a 70 °C, Fig. 25) ──
    int64_t n_leach = fs.agregar_nodo(nucleo::TipoNodo::TanqueLixiviacion, "Leaching");
    if (auto* nl = fs.nodo_por_id(n_leach)) {
        nl->canvas_x = 240.0f;
        nl->canvas_y = 320.0f;
        nl->params["temperatura"] = 273.15 + 70.0;
        nl->params["presion"]     = 1.0;
        if (id_O2 > 0) nl->params["split_" + std::to_string(id_O2)] = 0.0; // gas va a salida 0 (Offgas)
        if (id_N2 > 0) nl->params["split_" + std::to_string(id_N2)] = 0.0;
    }
    int64_t f_air   = fs.agregar_flujo("Air");
    int64_t f_fes   = fs.agregar_flujo("FeS");
    int64_t f_h2so4 = fs.agregar_flujo("H2SO4");
    int64_t f_off   = fs.agregar_flujo("Offgas");
    int64_t f_sol   = fs.agregar_flujo("Solution");

    if (auto* fa = fs.flujo_por_id(f_air)) {
        fa->temperatura = 273.15 + 25.0;
        fa->presion     = 1.0;
        // 14.06 t/h aire (~11 071.85 Nm3/h): 21% O2, 79% N2
        const double kmol_s_tot = (11071.85 / 22414.0) / 3600.0;
        if (id_O2 > 0) fa->flujo_molar[id_O2] = kmol_s_tot * 0.21;
        if (id_N2 > 0) fa->flujo_molar[id_N2] = kmol_s_tot * 0.79;
        fa->nodo_origen  = -1;
        fa->nodo_destino = n_leach;
    }
    if (auto* ff = fs.flujo_por_id(f_fes)) {
        ff->temperatura = 273.15 + 25.0;
        ff->presion     = 1.0;
        ff->agua_kg_h   = 50000.0; // 50.00 t/h líquido
        ff->particulas.push_back({"FeS", 75.0, 10000.0}); // 10.00 t/h sólidos
        ff->nodo_origen  = -1;
        ff->nodo_destino = n_leach;
    }
    if (auto* fh2 = fs.flujo_por_id(f_h2so4)) {
        fh2->temperatura  = 273.15 + 25.0;
        fh2->presion      = 1.0;
        fh2->agua_kg_h    = 10100.0; // 10.10 t/h solución ácida
        fh2->nodo_origen  = -1;
        fh2->nodo_destino = n_leach;
    }
    if (auto* fo = fs.flujo_por_id(f_off)) {
        fo->temperatura  = 273.15 + 70.0;
        fo->presion      = 1.0;
        fo->nodo_origen  = n_leach;
        fo->nodo_destino = -1;
    }
    if (auto* fso = fs.flujo_por_id(f_sol)) {
        fso->temperatura  = 273.15 + 70.0;
        fso->presion      = 1.0;
        fso->nodo_origen  = n_leach;
        fso->nodo_destino = -1;
    }
    if (auto* nl = fs.nodo_por_id(n_leach)) {
        nl->ids_flujos_entrada = {f_air, f_fes, f_h2so4};
        nl->ids_flujos_salida  = {f_off, f_sol};
    }

    // Ejecutar simulación para poblar todas las corrientes y el reporte
    ultimo_res = motor_bm.resolver(fs);
    for (const auto& f_res : ultimo_res.flujos_salida) {
        if (auto* f_ptr = fs.flujo_por_id(f_res.id)) {
            int64_t orig = f_ptr->nodo_origen;
            int64_t dest = f_ptr->nodo_destino;
            *f_ptr = f_res;
            f_ptr->nodo_origen = orig;
            f_ptr->nodo_destino = dest;
        }
    }
    // Asegurar temperatura de salida en Leaching (70 °C) y recalcular entalpía
    for (int64_t fid : {f_off, f_sol}) {
        if (auto* fp = fs.flujo_por_id(fid)) {
            fp->temperatura = 273.15 + 70.0;
            fp->entalpia = (fp->agua_kg_h / 3600.0) * (-15879.0 + 4.184 * (fp->temperatura - nucleo::STP_TEMP))
                         + fp->calor_contenido_Hc_kW(db_);
        }
    }
    hist_mb_pct_.push_back(ultimo_res.error_balance_masa_pct);
    hist_eb_pct_.push_back(ultimo_res.error_balance_entalpia_pct);
}

void PanelSimCommonTools::renderizar(diagrama_flujo::Flowsheet& fs,
                                     motores::BalanceMasa& motor_bm,
                                     motores::ResultadoBalanceMasa& ultimo_res) {
    if (!visible) return;

    ImGui::SetNextWindowSize(ImVec2(1180, 760), ImGuiCond_FirstUseEver);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(14.0f, 12.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(8.0f, 5.0f));
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(10.0f, 8.0f));

    if (!ImGui::Begin("HSC Sim — Common Tools (Unit Models, Stream Tables, Reports, Scenarios & Convergence)",
                      &visible, ImGuiWindowFlags_NoCollapse)) {
        ImGui::End();
        ImGui::PopStyleVar(3);
        return;
    }

    // Barra superior de acceso rápido
    ImGui::TextColored(AdaptiveColor(0.22f, 0.58f, 0.96f, 1.0f), "[Sim] Proyecto: %s",
                       fs.info_proceso.project_name.c_str());
    ImGui::SameLine();
    ImGui::TextDisabled("(%zu unidades, %zu corrientes)", fs.conteo_nodos(), fs.conteo_flujos());
    ImGui::SameLine(std::max(320.0f, ImGui::GetWindowWidth() - 430.0f));
    if (ImGui::Button("Cargar Hydro_example.Sim (Fig. 24-28)")) {
        cargar_ejemplo_hydro(fs, motor_bm, ultimo_res);
    }
    ImGui::SameLine();
    if (ImGui::Button("Calcular Sim")) {
        ultimo_res = motor_bm.resolver(fs, max_iter_conv_, tol_conv_);
        for (const auto& f_res : ultimo_res.flujos_salida) {
            if (auto* fp = fs.flujo_por_id(f_res.id)) {
                int64_t o = fp->nodo_origen, d = fp->nodo_destino;
                *fp = f_res;
                fp->nodo_origen = o;
                fp->nodo_destino = d;
            }
        }
        hist_mb_pct_.push_back(ultimo_res.error_balance_masa_pct);
        hist_eb_pct_.push_back(ultimo_res.error_balance_entalpia_pct);
    }

    ImGui::Separator();

    if (ImGui::BeginTabBar("##SimCommonToolsTabs", ImGuiTabBarFlags_None)) {
        auto flags_tab = [&](int idx) -> ImGuiTabItemFlags {
            return (pestana_solicitada_ == idx) ? ImGuiTabItemFlags_SetSelected : ImGuiTabItemFlags_None;
        };

        if (ImGui::BeginTabItem("1. Select Unit Models (40.2.2)", nullptr, flags_tab(0))) {
            renderizar_select_unit_models(fs, motor_bm, ultimo_res);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("2. Stream Table Editor (40.1.4)", nullptr, flags_tab(1))) {
            renderizar_stream_table_editor(fs);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("3. Reports — Unit & Stream Balance (40.2.1)", nullptr, flags_tab(2))) {
            renderizar_reports(fs, motor_bm, ultimo_res);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("4. Scenario & Monte Carlo Editor (40.2.3-4)", nullptr, flags_tab(3))) {
            renderizar_scenario_monte_carlo(fs, motor_bm, ultimo_res);
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("5. Convergence & Calculation Order (40.2.5)", nullptr, flags_tab(4))) {
            renderizar_convergence_diagnostics(fs, motor_bm, ultimo_res);
            ImGui::EndTabItem();
        }

        pestana_solicitada_ = -1;
        ImGui::EndTabBar();
    }

    ImGui::End();
    ImGui::PopStyleVar(3);
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Select Unit Models (Sec. 40.2.2, Fig. 27-28)
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimCommonTools::renderizar_select_unit_models(diagrama_flujo::Flowsheet& fs,
                                                        motores::BalanceMasa& motor_bm,
                                                        motores::ResultadoBalanceMasa& ultimo_res) {
    auto& nodos = fs.nodos();
    const int num_modelos = static_cast<int>(sizeof(kModelosBiblioteca) / sizeof(kModelosBiblioteca[0]));
    modelo_lib_idx_ = std::clamp(modelo_lib_idx_, 0, num_modelos - 1);

    ImGui::Columns(3, "##ColsSelectUnitModels", true);
    ImGui::SetColumnWidth(0, 360.0f);
    ImGui::SetColumnWidth(1, 380.0f);

    // ── Columna 1: Process Unit Operations (Fig. 27 izquierda) ──
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Process Unit Operations");
    ImGui::Spacing();
    if (nodos.empty()) {
        ImGui::TextWrapped("No hay unidades en el diagrama. Pulse 'Cargar Hydro_example.Sim' o dibuje unidades en el lienzo.");
        if (ImGui::Button("+ Cargar Ejemplo Hydro_example.Sim")) {
            cargar_ejemplo_hydro(fs, motor_bm, ultimo_res);
        }
    } else {
        nodo_sel_idx_ = std::clamp(nodo_sel_idx_, 0, static_cast<int>(nodos.size()) - 1);
        if (ImGui::BeginTable("##TblProcessUnits", 2,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0, 460.0f))) {
            ImGui::TableSetupColumn("Unit Name", ImGuiTableColumnFlags_WidthStretch, 0.45f);
            ImGui::TableSetupColumn("Assigned Model", ImGuiTableColumnFlags_WidthStretch, 0.55f);
            ImGui::TableHeadersRow();
            for (int i = 0; i < static_cast<int>(nodos.size()); ++i) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                bool sel = (nodo_sel_idx_ == i);
                std::string lbl = nodos[i].nombre + "##unit_" + std::to_string(nodos[i].id);
                if (ImGui::Selectable(lbl.c_str(), sel, ImGuiSelectableFlags_SpanAllColumns)) {
                    nodo_sel_idx_ = i;
                }
                ImGui::TableSetColumnIndex(1);
                ImGui::TextUnformatted(nucleo::nombre_tipo_nodo(nodos[i].tipo));
            }
            ImGui::EndTable();
        }
    }

    ImGui::NextColumn();

    // ── Columna 2: Model Library (Species | Particles | Others | Imported, Fig. 27 centro) ──
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Model Library");
    ImGui::Spacing();
    const char* cats[] = {"Species", "Particles", "Others", "Imported"};
    for (int c = 0; c < 4; ++c) {
        if (c > 0) ImGui::SameLine();
        bool act = (cat_lib_idx_ == c);
        if (act) ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.52f, 0.88f, 1.0f));
        if (ImGui::Button(cats[c], ImVec2(82.0f, 0.0f))) cat_lib_idx_ = c;
        if (act) ImGui::PopStyleColor();
    }
    ImGui::Spacing();
    if (ImGui::BeginTable("##TblModelLib", 2,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                          ImVec2(0, 420.0f))) {
        ImGui::TableSetupColumn("Model Name", ImGuiTableColumnFlags_WidthStretch, 0.68f);
        ImGui::TableSetupColumn("Version", ImGuiTableColumnFlags_WidthStretch, 0.32f);
        ImGui::TableHeadersRow();
        for (int m = 0; m < num_modelos; ++m) {
            if (kModelosBiblioteca[m].categoria != cat_lib_idx_) continue;
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            bool sel = (modelo_lib_idx_ == m);
            if (ImGui::Selectable(kModelosBiblioteca[m].nombre, sel, ImGuiSelectableFlags_SpanAllColumns)) {
                modelo_lib_idx_ = m;
            }
            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted(kModelosBiblioteca[m].version);
        }
        ImGui::EndTable();
    }

    ImGui::NextColumn();

    // ── Columna 3: Model Info (Fig. 27 derecha) ──
    const auto& info = kModelosBiblioteca[modelo_lib_idx_];
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Model Info");
    ImGui::Spacing();
    if (ImGui::BeginTable("##TblModelInfo", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Property", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthStretch);
        auto fila = [](const char* k, const char* v) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("%s", k);
            ImGui::TableSetColumnIndex(1); ImGui::TextWrapped("%s", v);
        };
        fila("Model Name", info.nombre);
        fila("Mode",       info.modo);
        fila("Type Code",  info.codigo);
        fila("Version",    info.version);
        fila("Engine Map", nucleo::nombre_tipo_nodo(info.tipo_nodo));
        ImGui::EndTable();
    }
    ImGui::Spacing();
    ImGui::TextDisabled("Description:");
    ImGui::BeginChild("##DescModelBox", ImVec2(0, 110.0f), true);
    ImGui::TextWrapped("%s", info.descripcion);
    ImGui::EndChild();

    ImGui::Spacing();
    if (!nodos.empty()) {
        auto& u = nodos[nodo_sel_idx_];
        if (ImGui::Button("Asignar Modelo a Unidad Seleccionada", ImVec2(-1.0f, 34.0f))) {
            u.tipo = info.tipo_nodo;
        }
    }
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Stream Table Editor (Sec. 40.1.4, Fig. 9-10)
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimCommonTools::renderizar_stream_table_editor(diagrama_flujo::Flowsheet& fs) {
    auto& cfg = fs.config_tabla_flujos;
    const int n_cat = static_cast<int>(sizeof(kCatalogoVariablesStream) / sizeof(kCatalogoVariablesStream[0]));
    var_sel_idx_ = std::clamp(var_sel_idx_, 0, n_cat - 1);

    // Barra de herramientas superior (Fig. 9)
    if (ImGui::Button("+ Add / Update Stream Table on Flowsheet")) {
        if (fs.tablas_flujo().empty()) fs.agregar_tabla_flujo();
        auto& t = fs.tablas_flujo().front();
        t.visible = true;
        t.ids_flujos_mostrados.clear();
        for (const auto& [fid, f] : fs.mapa_flujos()) {
            if (f.es_energia) continue;
            bool inc = flujos_en_tabla_.count(fid) ? flujos_en_tabla_[fid] : true;
            if (inc) t.ids_flujos_mostrados.push_back(fid);
        }
        t.w = std::max(340.0f, 180.0f + 115.0f * static_cast<float>(t.ids_flujos_mostrados.size()));
        t.h = std::max(130.0f, 44.0f + 22.0f * static_cast<float>(cfg.variables_seleccionadas.size()));
    }
    ImGui::SameLine();
    if (ImGui::Button("Clear Table Set")) cfg.variables_seleccionadas.clear();
    ImGui::SameLine();
    ImGui::SetNextItemWidth(210.0f);
    const char* modos_vista[] = {"Name, value and unit", "Value and unit", "Value only"};
    ImGui::Combo("Table View Mode", &cfg.modo_vista, modos_vista, 3);
    ImGui::Separator();

    ImGui::Columns(3, "##ColsStreamTableEditor", true);
    ImGui::SetColumnWidth(0, 320.0f);
    ImGui::SetColumnWidth(1, 450.0f);

    // ── Columna 1: Variable List (Fig. 9 izquierda) ──
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Variable List (Double-click to add)");
    ImGui::BeginChild("##VarListChild", ImVec2(0, 500.0f), true);
    auto render_grupo = [&](const char* titulo, bool part_flag) {
        if (ImGui::TreeNodeEx(titulo, ImGuiTreeNodeFlags_DefaultOpen)) {
            for (int i = 0; i < n_cat; ++i) {
                if (kCatalogoVariablesStream[i].es_particulas != part_flag) continue;
                bool ya_esta = std::find(cfg.variables_seleccionadas.begin(),
                                         cfg.variables_seleccionadas.end(),
                                         kCatalogoVariablesStream[i].clave) != cfg.variables_seleccionadas.end();
                std::string label = std::string(ya_esta ? "[x] " : "[ ] ") +
                                    kCatalogoVariablesStream[i].nombre_mostrar + " (" +
                                    kCatalogoVariablesStream[i].unidad + ")";
                if (ImGui::Selectable(label.c_str(), var_sel_idx_ == i, ImGuiSelectableFlags_AllowDoubleClick)) {
                    var_sel_idx_ = i;
                    if (ImGui::IsMouseDoubleClicked(0) && !ya_esta)
                        cfg.variables_seleccionadas.push_back(kCatalogoVariablesStream[i].clave);
                }
            }
            ImGui::TreePop();
        }
    };
    render_grupo("Minerals Processing (particles)", true);
    render_grupo("Reactions & Thermochemical", false);
    ImGui::EndChild();
    if (ImGui::Button("-> Agregar Variable Seleccionada", ImVec2(-1.0f, 0.0f))) {
        const char* k = kCatalogoVariablesStream[var_sel_idx_].clave;
        if (std::find(cfg.variables_seleccionadas.begin(), cfg.variables_seleccionadas.end(), k) ==
            cfg.variables_seleccionadas.end()) {
            cfg.variables_seleccionadas.push_back(k);
        }
    }

    ImGui::NextColumn();

    // ── Columna 2: Selected Variables — Table Set 1 (Fig. 9 centro) ──
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Selected Variables (%s)",
                       cfg.nombre_set.c_str());
    const nucleo::Flujo* flujo_muestra = !fs.mapa_flujos().empty() ? &fs.mapa_flujos().begin()->second : nullptr;

    if (ImGui::BeginTable("##TblPreviewStreamSet", 4,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                          ImVec2(0, 500.0f))) {
        ImGui::TableSetupColumn("#", ImGuiTableColumnFlags_WidthFixed, 28.0f);
        ImGui::TableSetupColumn("A (Variable Name)", ImGuiTableColumnFlags_WidthStretch, 0.52f);
        ImGui::TableSetupColumn("B (Value)", ImGuiTableColumnFlags_WidthStretch, 0.26f);
        ImGui::TableSetupColumn("C (Unit)", ImGuiTableColumnFlags_WidthStretch, 0.22f);
        ImGui::TableHeadersRow();

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("1");
        ImGui::TableSetColumnIndex(1); ImGui::TextColored(AdaptiveColor(0.3f, 0.7f, 1.0f, 1.0f), "<Stream Title>");
        ImGui::TableSetColumnIndex(2); ImGui::TextDisabled("%s", flujo_muestra ? flujo_muestra->nombre.c_str() : "-");
        ImGui::TableSetColumnIndex(3); ImGui::TextDisabled("-");

        int del_idx = -1;
        for (int r = 0; r < static_cast<int>(cfg.variables_seleccionadas.size()); ++r) {
            const auto& k = cfg.variables_seleccionadas[r];
            const auto* inf = buscar_info_variable(k);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("%d", r + 2);
            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted(inf ? inf->nombre_mostrar : k.c_str());
            ImGui::SameLine();
            ImGui::PushID(r);
            if (ImGui::SmallButton("x")) del_idx = r;
            ImGui::PopID();
            ImGui::TableSetColumnIndex(2);
            double v = flujo_muestra ? evaluar_variable_corriente(k, *flujo_muestra, db_) : 0.0;
            ImGui::Text("%.2f", v);
            ImGui::TableSetColumnIndex(3);
            ImGui::TextUnformatted(inf ? inf->unidad : "");
        }
        if (del_idx >= 0) cfg.variables_seleccionadas.erase(cfg.variables_seleccionadas.begin() + del_idx);
        ImGui::EndTable();
    }

    ImGui::NextColumn();

    // ── Columna 3: Variable Properties & Configuration to Streams (Fig. 9 derecha) ──
    const auto& vinfo = kCatalogoVariablesStream[var_sel_idx_];
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Variable Properties");
    if (ImGui::BeginTable("##TblVarProps", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Field", ImGuiTableColumnFlags_WidthFixed, 120.0f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("Display Name");
        ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(vinfo.nombre_mostrar);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("Unit");
        ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(vinfo.unidad);
        ImGui::TableNextRow(); ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("Header Visibility");
        ImGui::TableSetColumnIndex(1); ImGui::Checkbox("##hdr_vis", &cfg.mostrar_encabezado);
        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Configuration to Streams");
    if (ImGui::BeginTable("##TblCfgStreams", 3,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                          ImVec2(0, 320.0f))) {
        ImGui::TableSetupColumn("On", ImGuiTableColumnFlags_WidthFixed, 32.0f);
        ImGui::TableSetupColumn("Stream", ImGuiTableColumnFlags_WidthStretch, 0.6f);
        ImGui::TableSetupColumn("Table Set", ImGuiTableColumnFlags_WidthStretch, 0.4f);
        ImGui::TableHeadersRow();
        for (const auto& [fid, f] : fs.mapa_flujos()) {
            if (f.es_energia) continue;
            if (!flujos_en_tabla_.count(fid)) flujos_en_tabla_[fid] = true;
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::PushID(static_cast<int>(fid));
            ImGui::Checkbox("##inc_s", &flujos_en_tabla_[fid]);
            ImGui::PopID();
            ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(f.nombre.c_str());
            ImGui::TableSetColumnIndex(2); ImGui::TextDisabled("%s", flujos_en_tabla_[fid] ? cfg.nombre_set.c_str() : "<None>");
        }
        ImGui::EndTable();
    }
    ImGui::Columns(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Flowsheet Reports: Unit Balance & Stream Balance (Sec. 40.2.1, Fig. 24-26)
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimCommonTools::renderizar_reports(diagrama_flujo::Flowsheet& fs,
                                             motores::BalanceMasa& /*motor_bm*/,
                                             motores::ResultadoBalanceMasa& /*ultimo_res*/) {
    ImGui::Checkbox("Model Inputs", &mostrar_inputs_);
    ImGui::SameLine();
    ImGui::Checkbox("Model Outputs", &mostrar_outputs_);
    ImGui::SameLine();
    ImGui::Checkbox("Intermediates", &mostrar_intermedios_);
    ImGui::SameLine();
    ImGui::Checkbox("Mass in t/h (else kg/s)", &usar_t_h_);
    ImGui::SameLine();
    if (ImGui::Button("Export Report CSV")) {
        std::ofstream ofs("flowsheet_report.csv");
        if (ofs.is_open()) {
            ofs << "Unit/Stream,Type,Temperature_C,Pressure_bar,MassFlow_t_h,GasFlow_Nm3_h,LiquidFlow_t_h,SolidsFlow_t_h,Enthalpy_kW,Exergy_kW\n";
            for (const auto& [fid, f] : fs.mapa_flujos()) {
                if (f.es_energia) continue;
                int tc = fs.clasificar_conexion_flujo(fid);
                const char* ts = (tc == 0) ? "Input" : (tc == 2 ? "Output" : "Intermediate");
                ofs << f.nombre << "," << ts << "," << (f.temperatura - 273.15) << "," << f.presion << ","
                    << (f.flujo_masico_total(db_) * 3.6) << "," << f.volumen_gas_Nm3_h(db_) << ","
                    << f.masa_liquido_t_h(db_) << "," << f.masa_solidos_t_h(db_) << ","
                    << f.entalpia << "," << f.exergia_flujo_kW(db_) << "\n";
            }
            estado_exportacion_ = "Exportado a flowsheet_report.csv";
        }
    }
    if (!estado_exportacion_.empty()) {
        ImGui::SameLine();
        ImGui::TextColored(AdaptiveColor(0.2f, 0.8f, 0.4f, 1.0f), "%s", estado_exportacion_.c_str());
    }
    ImGui::Separator();

    const double factor_m = usar_t_h_ ? 3.6 : 1.0;
    const char* u_masa = usar_t_h_ ? "t/h" : "kg/s";

    ImGui::BeginChild("##ReportContentArea", ImVec2(0, -38.0f), true);
    if (hoja_reporte_ == 0) {
        ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f),
                           "Unit Balance - General (%s)", fs.info_proceso.project_path.c_str());
        ImGui::Spacing();

        for (const auto& n : fs.nodos()) {
            if (n.tipo == nucleo::TipoNodo::Fuente || n.tipo == nucleo::TipoNodo::Sumidero) continue;
            ImGui::PushID(static_cast<int>(n.id));
            ImGui::TextColored(AdaptiveColor(0.95f, 0.75f, 0.25f, 1.0f), "[Unit] %s (%s)",
                               n.nombre.c_str(), nucleo::nombre_tipo_nodo(n.tipo));
            if (ImGui::BeginTable("##TblUnitBal", 10,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_SizingStretchProp)) {
                ImGui::TableSetupColumn("Streams ID", ImGuiTableColumnFlags_WidthStretch, 1.2f);
                ImGui::TableSetupColumn("Type", ImGuiTableColumnFlags_WidthStretch, 0.7f);
                ImGui::TableSetupColumn("Temperature (°C)", ImGuiTableColumnFlags_WidthStretch, 0.9f);
                ImGui::TableSetupColumn("Pressure (bar)", ImGuiTableColumnFlags_WidthStretch, 0.8f);
                ImGui::TableSetupColumn("Mass Flow", ImGuiTableColumnFlags_WidthStretch, 0.9f);
                ImGui::TableSetupColumn("Gas Flow (Nm3/h)", ImGuiTableColumnFlags_WidthStretch, 1.0f);
                ImGui::TableSetupColumn("Liquid Flow", ImGuiTableColumnFlags_WidthStretch, 0.9f);
                ImGui::TableSetupColumn("Solids Flow", ImGuiTableColumnFlags_WidthStretch, 0.9f);
                ImGui::TableSetupColumn("Enthalpy Flow (kW)", ImGuiTableColumnFlags_WidthStretch, 1.1f);
                ImGui::TableSetupColumn("Exergy Flow (kW)", ImGuiTableColumnFlags_WidthStretch, 1.0f);
                ImGui::TableHeadersRow();

                double sum_m = 0.0, sum_g = 0.0, sum_l = 0.0, sum_s = 0.0, sum_h = 0.0, sum_ex = 0.0;
                auto render_fila_flujo = [&](int64_t fid, bool es_entrada) {
                    const auto* f = fs.flujo_por_id(fid);
                    if (!f || f->es_energia) return;
                    const double sgn = es_entrada ? -1.0 : 1.0; // BALANCE = Output - Input (Fig. 25)
                    const double m  = f->flujo_masico_total(db_) * factor_m;
                    const double g  = f->volumen_gas_Nm3_h(db_);
                    const double l  = (usar_t_h_ ? 1.0 : 1.0 / 3.6) * f->masa_liquido_t_h(db_);
                    const double s  = (usar_t_h_ ? 1.0 : 1.0 / 3.6) * f->masa_solidos_t_h(db_);
                    const double h  = f->entalpia;
                    const double ex = f->exergia_flujo_kW(db_);
                    sum_m += sgn * m; sum_g += sgn * g; sum_l += sgn * l;
                    sum_s += sgn * s; sum_h += sgn * h; sum_ex += sgn * ex;

                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImVec4 col_nom = es_entrada ? AdaptiveColor(0.25f, 0.62f, 0.98f, 1.0f) : AdaptiveColor(0.95f, 0.35f, 0.35f, 1.0f);
                    ImGui::TextColored(col_nom, "%s", f->nombre.c_str());
                    ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(es_entrada ? "Input" : "Output");
                    ImGui::TableSetColumnIndex(2); ImGui::Text("%.2f", f->temperatura - 273.15);
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f", f->presion);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%.2f", m);
                    ImGui::TableSetColumnIndex(5); ImGui::Text("%.2f", g);
                    ImGui::TableSetColumnIndex(6); ImGui::Text("%.2f", l);
                    ImGui::TableSetColumnIndex(7); ImGui::Text("%.2f", s);
                    ImGui::TableSetColumnIndex(8); ImGui::Text("%.2f", h);
                    ImGui::TableSetColumnIndex(9); ImGui::Text("%.2f", ex);
                };

                for (int64_t fid : n.ids_flujos_entrada) render_fila_flujo(fid, true);
                for (int64_t fid : n.ids_flujos_salida)  render_fila_flujo(fid, false);

                ImGui::TableNextRow();
                ImGui::TableSetBgColor(ImGuiTableBgTarget_RowBg0, IM_COL32(60, 65, 78, 140));
                ImGui::TableSetColumnIndex(1); ImGui::TextColored(AdaptiveColor(1.0f, 0.9f, 0.4f, 1.0f), "BALANCE:");
                ImGui::TableSetColumnIndex(4);
                ImGui::TextColored(std::abs(sum_m) > 0.05 ? AdaptiveColor(1.0f, 0.35f, 0.35f, 1.0f) : AdaptiveColor(0.4f, 0.95f, 0.5f, 1.0f), "%.2f", sum_m);
                ImGui::TableSetColumnIndex(5); ImGui::Text("%.2f", sum_g);
                ImGui::TableSetColumnIndex(6); ImGui::Text("%.2f", sum_l);
                ImGui::TableSetColumnIndex(7); ImGui::Text("%.2f", sum_s);
                ImGui::TableSetColumnIndex(8);
                ImGui::TextColored(std::abs(sum_h) > 1.0 ? AdaptiveColor(1.0f, 0.35f, 0.35f, 1.0f) : AdaptiveColor(0.4f, 0.95f, 0.5f, 1.0f), "%.2f", sum_h);
                ImGui::TableSetColumnIndex(9); ImGui::Text("%.2f", sum_ex);
                ImGui::EndTable();
            }
            ImGui::Spacing();
            ImGui::PopID();
        }
    } else if (hoja_reporte_ == 1) {
        ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f),
                           "Stream Balance - Hydro (Reactions Variable List 1 streams)");
        ImGui::Spacing();

        std::vector<const nucleo::Flujo*> flujos_in, flujos_out, flujos_int;
        for (const auto& [fid, f] : fs.mapa_flujos()) {
            if (f.es_energia) continue;
            int tc = fs.clasificar_conexion_flujo(fid);
            if (tc == 0 && mostrar_inputs_) flujos_in.push_back(&f);
            else if (tc == 2 && mostrar_outputs_) flujos_out.push_back(&f);
            else if (tc == 1 && mostrar_intermedios_) flujos_int.push_back(&f);
        }

        const int n_cols = 6 + static_cast<int>(flujos_in.size() + flujos_out.size() + flujos_int.size());
        if (ImGui::BeginTable("##TblStreamBalHydro", n_cols,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollX | ImGuiTableFlags_ScrollY)) {
            ImGui::TableSetupColumn("Code", ImGuiTableColumnFlags_WidthFixed, 48.0f);
            ImGui::TableSetupColumn("Property", ImGuiTableColumnFlags_WidthFixed, 145.0f);
            ImGui::TableSetupColumn("Unit", ImGuiTableColumnFlags_WidthFixed, 60.0f);
            ImGui::TableSetupColumn("BALANCE:", ImGuiTableColumnFlags_WidthFixed, 95.0f);
            ImGui::TableSetupColumn("INPUT:", ImGuiTableColumnFlags_WidthFixed, 95.0f);
            for (const auto* f : flujos_in) ImGui::TableSetupColumn(f->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 95.0f);
            ImGui::TableSetupColumn("OUTPUT:", ImGuiTableColumnFlags_WidthFixed, 95.0f);
            for (const auto* f : flujos_out) ImGui::TableSetupColumn(f->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 95.0f);
            for (const auto* f : flujos_int) ImGui::TableSetupColumn(f->nombre.c_str(), ImGuiTableColumnFlags_WidthFixed, 95.0f);
            ImGui::TableHeadersRow();

            struct DefFilaSB { const char* cod; const char* nom; const char* uni; bool es_intensiva; int id_prop; };
            const DefFilaSB filas[] = {
                {"T", "Temperature", "°C", true, 0}, {"Pr", "Pressure", "bar", true, 1},
                {"A", "Amount", u_masa, false, 2},   {"H", "Enthalpy", "kW", false, 3},
                {"V", "Volume", "m3/h", false, 4},   {"Ht", "Thermal Energy Flow", "kW", false, 3},
                {"Hc", "Heat Content Flow", "kW", false, 5}, {"Ex", "Exergy", "kW", false, 6},
                {"Cp", "Heat Capacity", "kJ/kgK", true, 7},  {"P1g", "Gas Phase", u_masa, false, 8},
                {"P2a", "Water Phase", u_masa, false, 9},    {"P3s", "Solids Phase", u_masa, false, 10},
                {"Sppd", "Pressure Drop", "bar", true, 11},  {"Spd", "Delay", "s", true, 12}
            };

            auto eval_prop = [&](const nucleo::Flujo& f, int id_prop) -> double {
                switch (id_prop) {
                    case 0: return f.temperatura - 273.15;
                    case 1: return f.presion;
                    case 2: return f.flujo_masico_total(db_) * factor_m;
                    case 3: return f.entalpia;
                    case 4: return f.volumen_total_m3_h(db_);
                    case 5: return f.calor_contenido_Hc_kW(db_);
                    case 6: return f.exergia_flujo_kW(db_);
                    case 7: return f.cp_especifico_kJ_kgK(db_);
                    case 8: return f.masa_gas_t_h(db_) * (usar_t_h_ ? 1.0 : 1.0 / 3.6);
                    case 9: return f.masa_liquido_t_h(db_) * (usar_t_h_ ? 1.0 : 1.0 / 3.6);
                    case 10: return f.masa_solidos_t_h(db_) * (usar_t_h_ ? 1.0 : 1.0 / 3.6);
                    default: return 0.0;
                }
            };

            for (const auto& rf : filas) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::TextDisabled("%s", rf.cod);
                ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(rf.nom);
                ImGui::TableSetColumnIndex(2); ImGui::TextDisabled("%s", rf.uni);

                double sum_in = 0.0, sum_out = 0.0;
                for (const auto* f : flujos_in)  sum_in  += eval_prop(*f, rf.id_prop);
                for (const auto* f : flujos_out) sum_out += eval_prop(*f, rf.id_prop);
                if (rf.es_intensiva) {
                    if (!flujos_in.empty())  sum_in  /= static_cast<double>(flujos_in.size());
                    if (!flujos_out.empty()) sum_out /= static_cast<double>(flujos_out.size());
                }
                const double bal = sum_out - sum_in;

                ImGui::TableSetColumnIndex(3);
                if (!rf.es_intensiva || rf.id_prop == 7) ImGui::Text("%.2f", bal);
                else ImGui::TextDisabled("-");

                ImGui::TableSetColumnIndex(4);
                if (!rf.es_intensiva) ImGui::Text("%.2f", sum_in);
                else ImGui::TextDisabled("-");

                int col = 5;
                for (const auto* f : flujos_in)  { ImGui::TableSetColumnIndex(col++); ImGui::Text("%.2f", eval_prop(*f, rf.id_prop)); }
                ImGui::TableSetColumnIndex(col++);
                if (!rf.es_intensiva) ImGui::Text("%.2f", sum_out);
                else ImGui::TextDisabled("-");
                for (const auto* f : flujos_out) { ImGui::TableSetColumnIndex(col++); ImGui::Text("%.2f", eval_prop(*f, rf.id_prop)); }
                for (const auto* f : flujos_int) { ImGui::TableSetColumnIndex(col++); ImGui::Text("%.2f", eval_prop(*f, rf.id_prop)); }
            }
            ImGui::EndTable();
        }
    } else {
        ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Streams - Properties & Phase Breakdown");
        if (ImGui::BeginTable("##TblStreamsProps", 9,
                              ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY)) {
            ImGui::TableSetupColumn("Stream", ImGuiTableColumnFlags_WidthStretch, 1.2f);
            ImGui::TableSetupColumn("Role", ImGuiTableColumnFlags_WidthStretch, 0.8f);
            ImGui::TableSetupColumn("T (°C)", ImGuiTableColumnFlags_WidthStretch, 0.7f);
            ImGui::TableSetupColumn("P (bar)", ImGuiTableColumnFlags_WidthStretch, 0.7f);
            ImGui::TableSetupColumn("Total (t/h)", ImGuiTableColumnFlags_WidthStretch, 0.9f);
            ImGui::TableSetupColumn("% Solids", ImGuiTableColumnFlags_WidthStretch, 0.8f);
            ImGui::TableSetupColumn("Pulp SG", ImGuiTableColumnFlags_WidthStretch, 0.8f);
            ImGui::TableSetupColumn("Hc (kW)", ImGuiTableColumnFlags_WidthStretch, 0.9f);
            ImGui::TableSetupColumn("Cp (kJ/kgK)", ImGuiTableColumnFlags_WidthStretch, 0.9f);
            ImGui::TableHeadersRow();
            for (const auto& [fid, f] : fs.mapa_flujos()) {
                if (f.es_energia) continue;
                int tc = fs.clasificar_conexion_flujo(fid);
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(f.nombre.c_str());
                ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(tc == 0 ? "Input (Blue)" : (tc == 2 ? "Output (Red)" : "Internal (Black)"));
                ImGui::TableSetColumnIndex(2); ImGui::Text("%.2f", f.temperatura - 273.15);
                ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f", f.presion);
                ImGui::TableSetColumnIndex(4); ImGui::Text("%.2f", f.flujo_masico_total(db_) * 3.6);
                ImGui::TableSetColumnIndex(5); ImGui::Text("%.2f", f.porcentaje_solidos_wt(db_));
                ImGui::TableSetColumnIndex(6); ImGui::Text("%.3f", f.densidad_pulpa_g_cm3(db_));
                ImGui::TableSetColumnIndex(7); ImGui::Text("%.2f", f.calor_contenido_Hc_kW(db_));
                ImGui::TableSetColumnIndex(8); ImGui::Text("%.3f", f.cp_especifico_kJ_kgK(db_));
            }
            ImGui::EndTable();
        }
    }
    ImGui::EndChild();

    const char* hojas[] = {"Unit Balance - General", "Stream Balance - Hydro", "Streams - Properties"};
    for (int h = 0; h < 3; ++h) {
        if (h > 0) ImGui::SameLine();
        bool act = (hoja_reporte_ == h);
        if (act) ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.52f, 0.88f, 1.0f));
        if (ImGui::Button(hojas[h], ImVec2(210.0f, 26.0f))) hoja_reporte_ = h;
        if (act) ImGui::PopStyleColor();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Static Scenario Editor & Monte Carlo Simulation (Sec. 40.2.3-40.2.4, Fig. 29-34)
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimCommonTools::renderizar_scenario_monte_carlo(diagrama_flujo::Flowsheet& fs,
                                                          motores::BalanceMasa& motor_bm,
                                                          motores::ResultadoBalanceMasa& /*ultimo_res*/) {
    if (vars_escenario_.empty() && !fs.mapa_flujos().empty()) {
        const auto& f0 = fs.mapa_flujos().begin()->second;
        vars_escenario_.push_back({f0.id, true, f0.nombre, "temperatura", 288.15, 353.15, f0.temperatura, 1});
        if (f0.agua_kg_h > 0.0)
            vars_escenario_.push_back({f0.id, true, f0.nombre, "agua_kg_h", f0.agua_kg_h * 0.7, f0.agua_kg_h * 1.3, f0.agua_kg_h, 0});
    }

    ImGui::Checkbox("Monte Carlo Simulation Mode (Fig. 32)", &modo_monte_carlo_);
    ImGui::SameLine();
    ImGui::SetNextItemWidth(110.0f);
    ImGui::InputInt("Cases / Samples", &num_casos_mc_);
    num_casos_mc_ = std::clamp(num_casos_mc_, 2, 2000);
    ImGui::SameLine();
    if (ImGui::Button("+ Add SET Variable") && !fs.mapa_flujos().empty()) {
        const auto& f0 = fs.mapa_flujos().begin()->second;
        vars_escenario_.push_back({f0.id, true, f0.nombre, "temperatura", 293.15, 363.15, f0.temperatura, 0});
    }
    ImGui::SameLine();
    if (ImGui::Button("Run Scenarios / Monte Carlo")) {
        resultados_escenario_.clear();
        std::vector<application::modules::VariableDecision> dec_vars;
        for (size_t i = 0; i < vars_escenario_.size(); ++i) {
            application::modules::VariableDecision vd;
            vd.nombre = vars_escenario_[i].nombre_objeto + "." + vars_escenario_[i].parametro + "#" + std::to_string(i);
            vd.min = vars_escenario_[i].min_val;
            vd.max = vars_escenario_[i].max_val;
            vd.valor_inicial = vars_escenario_[i].val_inicial;
            vd.activa = true;
            vd.distribucion = static_cast<application::modules::DistribucionMonteCarlo>(vars_escenario_[i].distribucion);
            dec_vars.push_back(vd);
        }

        application::modules::SimOptimization opt;
        int caso_counter = 0;
        auto func_eval = [&](const std::map<std::string, double>& m) -> double {
            diagrama_flujo::Flowsheet copia = fs;
            std::vector<double> vals_set;
            for (size_t i = 0; i < vars_escenario_.size(); ++i) {
                std::string k = dec_vars[i].nombre;
                double v = m.count(k) ? m.at(k) : vars_escenario_[i].val_inicial;
                vals_set.push_back(v);
                if (vars_escenario_[i].es_flujo) {
                    if (auto* fl = copia.flujo_por_id(vars_escenario_[i].id_objeto)) {
                        if (vars_escenario_[i].parametro == "temperatura") fl->temperatura = v;
                        else if (vars_escenario_[i].parametro == "presion") fl->presion = v;
                        else if (vars_escenario_[i].parametro == "agua_kg_h") fl->agua_kg_h = v;
                    }
                } else if (auto* nd = copia.nodo_por_id(vars_escenario_[i].id_objeto)) {
                    nd->params[vars_escenario_[i].parametro] = v;
                }
            }
            auto r_bm = motor_bm.resolver(copia, 80, 1e-5);
            double t_out_C = !r_bm.flujos_salida.empty() ? (r_bm.flujos_salida.back().temperatura - 273.15) : 25.0;
            resultados_escenario_.push_back({++caso_counter, vals_set, r_bm.masa_salida_kg_s * 3.6,
                                             r_bm.entalpia_salida_kW - r_bm.entalpia_entrada_kW, t_out_C, r_bm.error_balance_masa_pct});
            return std::abs(r_bm.entalpia_salida_kW - r_bm.entalpia_entrada_kW);
        };

        if (modo_monte_carlo_ && !dec_vars.empty()) {
            ultimo_mc_ = opt.optimizar(dec_vars, func_eval, application::modules::MetodoOptimizacion::MonteCarlo, num_casos_mc_, semilla_mc_);
        } else if (!dec_vars.empty()) {
            for (int c = 0; c < num_casos_mc_; ++c) {
                double alpha = (num_casos_mc_ > 1) ? static_cast<double>(c) / (num_casos_mc_ - 1) : 0.0;
                std::map<std::string, double> mp;
                for (size_t i = 0; i < dec_vars.size(); ++i)
                    mp[dec_vars[i].nombre] = dec_vars[i].min + alpha * (dec_vars[i].max - dec_vars[i].min);
                func_eval(mp);
            }
        }
    }
    ImGui::Separator();

    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "SET Variables & Probability Distributions");
    if (ImGui::BeginTable("##TblScenarioSetVars", 6,
                          ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                          ImVec2(0, 165.0f))) {
        ImGui::TableSetupColumn("Stream / Unit", ImGuiTableColumnFlags_WidthStretch, 1.2f);
        ImGui::TableSetupColumn("Parameter", ImGuiTableColumnFlags_WidthStretch, 1.0f);
        ImGui::TableSetupColumn("Min Value", ImGuiTableColumnFlags_WidthStretch, 0.8f);
        ImGui::TableSetupColumn("Max Value", ImGuiTableColumnFlags_WidthStretch, 0.8f);
        ImGui::TableSetupColumn("Initial / Center", ImGuiTableColumnFlags_WidthStretch, 0.8f);
        ImGui::TableSetupColumn("MC Distribution (Fig. 33)", ImGuiTableColumnFlags_WidthStretch, 1.2f);
        ImGui::TableHeadersRow();
        const char* dist_names[] = {"Uniform", "Normal", "Laplace", "Trapezoid"};
        for (int i = 0; i < static_cast<int>(vars_escenario_.size()); ++i) {
            auto& v = vars_escenario_[i];
            ImGui::PushID(i);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(v.nombre_objeto.c_str());
            ImGui::TableSetColumnIndex(1); ImGui::TextUnformatted(v.parametro.c_str());
            ImGui::TableSetColumnIndex(2); ImGui::InputDouble("##min", &v.min_val, 0, 0, "%.2f");
            ImGui::TableSetColumnIndex(3); ImGui::InputDouble("##max", &v.max_val, 0, 0, "%.2f");
            ImGui::TableSetColumnIndex(4); ImGui::InputDouble("##ini", &v.val_inicial, 0, 0, "%.2f");
            ImGui::TableSetColumnIndex(5); ImGui::Combo("##dist", &v.distribucion, dist_names, 4);
            ImGui::PopID();
        }
        ImGui::EndTable();
    }

    if (ImGui::BeginTabBar("##ScenarioSubTabs")) {
        if (ImGui::BeginTabItem("Case Results Table (Fig. 30)")) {
            if (ImGui::BeginTable("##TblScenarioResults", 5,
                                  ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                                  ImVec2(0, 320.0f))) {
                ImGui::TableSetupColumn("Case #", ImGuiTableColumnFlags_WidthFixed, 65.0f);
                ImGui::TableSetupColumn("SET[0] Value", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("GET: Mass Out (t/h)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("GET: Net Enthalpy (kW)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableSetupColumn("GET: Outlet Temp (°C)", ImGuiTableColumnFlags_WidthStretch);
                ImGui::TableHeadersRow();
                for (const auto& r : resultados_escenario_) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::Text("Case %d", r.caso);
                    ImGui::TableSetColumnIndex(1); ImGui::Text("%.2f", r.valores_set.empty() ? 0.0 : r.valores_set[0]);
                    ImGui::TableSetColumnIndex(2); ImGui::Text("%.2f", r.masa_out_t_h);
                    ImGui::TableSetColumnIndex(3); ImGui::Text("%.2f", r.entalpia_neta_kW);
                    ImGui::TableSetColumnIndex(4); ImGui::Text("%.2f", r.temp_salida_C);
                }
                ImGui::EndTable();
            }
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Distribution Charts (Fig. 33)")) {
            if (!ultimo_mc_.muestras_variables.empty()) {
                const auto& kv = *ultimo_mc_.muestras_variables.begin();
                if (ImPlot::BeginPlot("Monte Carlo Input Distribution Histogram", ImVec2(-1, 310.0f))) {
                    ImPlot::SetupAxes("Sampled Value", "Frequency");
                    ImPlot::PlotHistogram(kv.first.c_str(), kv.second.data(), static_cast<int>(kv.second.size()), 20);
                    ImPlot::EndPlot();
                }
            } else {
                ImGui::TextDisabled("Ejecute 'Run Scenarios / Monte Carlo' para visualizar los histogramas de distribución.");
            }
            ImGui::EndTabItem();
        }
        if (ImGui::BeginTabItem("Simulation Results (Fig. 31 & 34)")) {
            if (!resultados_escenario_.empty()) {
                std::vector<double> xs, ys;
                for (const auto& r : resultados_escenario_) {
                    xs.push_back(static_cast<double>(r.caso));
                    ys.push_back(r.entalpia_neta_kW);
                }
                if (ImPlot::BeginPlot("GET Variable: Net Enthalpy Flow (kW) across Cases", ImVec2(-1, 310.0f))) {
                    ImPlot::SetupAxes("Case / Iteration", "Net Enthalpy Flow (kW)");
                    ImPlot::PlotLine("Net Enthalpy (kW)", xs.data(), ys.data(), static_cast<int>(xs.size()));
                    ImPlot::EndPlot();
                }
            } else {
                ImGui::TextDisabled("Ejecute 'Run Scenarios / Monte Carlo' para graficar los resultados.");
            }
            ImGui::EndTabItem();
        }
        ImGui::EndTabBar();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. Convergence Monitor, Clear Internal Streams & Calculation Order (Fig. 35-40)
// ═══════════════════════════════════════════════════════════════════════════
void PanelSimCommonTools::renderizar_convergence_diagnostics(diagrama_flujo::Flowsheet& fs,
                                                             motores::BalanceMasa& motor_bm,
                                                             motores::ResultadoBalanceMasa& ultimo_res) {
    if (ImGui::Button("Run Until Convergence")) {
        ultimo_res = motor_bm.resolver(fs, max_iter_conv_, tol_conv_);
        for (const auto& f_res : ultimo_res.flujos_salida) {
            if (auto* fp = fs.flujo_por_id(f_res.id)) {
                int64_t o = fp->nodo_origen, d = fp->nodo_destino;
                *fp = f_res;
                fp->nodo_origen = o;
                fp->nodo_destino = d;
            }
        }
        hist_mb_pct_.push_back(ultimo_res.error_balance_masa_pct);
        hist_eb_pct_.push_back(ultimo_res.error_balance_entalpia_pct);
        mensaje_accion_conv_ = ultimo_res.mensaje;
    }
    ImGui::SameLine();
    if (ImGui::Button("Clear Internal Streams (Fig. 37)")) {
        size_t n_limp = fs.limpiar_corrientes_internas();
        mensaje_accion_conv_ = "Se reiniciaron " + std::to_string(n_limp) +
                               " corrientes internas/salidas; las corrientes de alimentación (Input) se conservaron.";
    }
    ImGui::SameLine();
    if (ImGui::Button("Empty All Tanks")) {
        size_t n_vac = fs.vaciar_tanques();
        mensaje_accion_conv_ = "Se vaciaron " + std::to_string(n_vac) + " tanques/acumuladores del diagrama.";
    }
    if (!mensaje_accion_conv_.empty()) {
        ImGui::TextColored(AdaptiveColor(0.3f, 0.85f, 0.45f, 1.0f), "[OK] %s", mensaje_accion_conv_.c_str());
    }
    ImGui::Separator();

    ImGui::Columns(2, "##ColsConvOrder", true);

    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Sim Model Convergence Monitor (Fig. 36)");
    if (ImGui::BeginTable("##TblConvMetrics", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
        ImGui::TableSetupColumn("Global Indicator (HSC Sim Sec. 40.2.4)", ImGuiTableColumnFlags_WidthStretch, 0.62f);
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthStretch, 0.38f);
        ImGui::TableHeadersRow();

        auto fila_m = [](const char* k, const char* fmt, double v) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::TextUnformatted(k);
            ImGui::TableSetColumnIndex(1); ImGui::Text(fmt, v);
        };
        fila_m("Input Mass Flow (t/h)", "%.3f t/h", ultimo_res.masa_entrada_kg_s * 3.6);
        fila_m("Output Mass Flow (t/h)", "%.3f t/h", ultimo_res.masa_salida_kg_s * 3.6);
        fila_m("Global Mass Balance Error (MB %)", "%.4f %%", ultimo_res.error_balance_masa_pct);
        fila_m("Input Enthalpy Flow (kW)", "%.2f kW", ultimo_res.entalpia_entrada_kW);
        fila_m("Output Enthalpy Flow (kW)", "%.2f kW", ultimo_res.entalpia_salida_kW);
        fila_m("Global Enthalpy Balance Error (EB %)", "%.4f %%", ultimo_res.error_balance_entalpia_pct);
        ImGui::EndTable();
    }

    ImGui::Spacing();
    if (!hist_mb_pct_.empty() && ImPlot::BeginPlot("Global Mass & Enthalpy Error History (%)", ImVec2(-1, 210.0f))) {
        std::vector<double> xs(hist_mb_pct_.size());
        for (size_t i = 0; i < xs.size(); ++i) xs[i] = static_cast<double>(i + 1);
        ImPlot::SetupAxes("Calculation Run", "Error (%)");
        ImPlot::PlotLine("Mass Error MB (%)", xs.data(), hist_mb_pct_.data(), static_cast<int>(xs.size()));
        ImPlot::PlotLine("Enthalpy Error EB (%)", xs.data(), hist_eb_pct_.data(), static_cast<int>(xs.size()));
        ImPlot::EndPlot();
    }

    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f), "Diagnostics & Suggestions (Fig. 38)");
    ImGui::BeginChild("##DiagChild", ImVec2(0, 140.0f), true);
    int issues = 0;
    for (const auto& n : fs.nodos()) {
        if (n.tipo != nucleo::TipoNodo::Fuente && n.ids_flujos_entrada.empty()) {
            ImGui::TextColored(AdaptiveColor(0.95f, 0.6f, 0.2f, 1.0f),
                               "[Warning] Unidad '%s' no tiene corrientes de entrada conectadas.", n.nombre.c_str());
            ++issues;
        }
        if (n.tipo != nucleo::TipoNodo::Sumidero && n.ids_flujos_salida.empty()) {
            ImGui::TextColored(AdaptiveColor(0.95f, 0.6f, 0.2f, 1.0f),
                               "[Warning] Unidad '%s' no tiene corrientes de salida conectadas.", n.nombre.c_str());
            ++issues;
        }
    }
    if (issues == 0) {
        ImGui::TextColored(AdaptiveColor(0.3f, 0.85f, 0.45f, 1.0f),
                           "0 errores estructurales detectados. Todas las unidades y corrientes están correctamente conectadas.");
    }
    ImGui::EndChild();

    ImGui::NextColumn();

    ImGui::TextColored(AdaptiveColor(0.20f, 0.60f, 0.95f, 1.0f),
                       "Process Calculation Order & Stream Colors (Fig. 39-40)");
    ImGui::TextDisabled("Codificación de colores (Sec. 40.1.2e / Fig. 40):");
    ImGui::TextColored(AdaptiveColor(0.20f, 0.55f, 0.98f, 1.0f), "[Azul] Corriente de Entrada (Input)");
    ImGui::SameLine();
    ImGui::TextColored(AdaptiveColor(0.75f, 0.78f, 0.82f, 1.0f), "[Negro] Interna");
    ImGui::SameLine();
    ImGui::TextColored(AdaptiveColor(0.95f, 0.30f, 0.30f, 1.0f), "[Rojo] Salida (Output)");

    ImGui::BeginChild("##CalcOrderTree", ImVec2(0, 440.0f), true);
    auto orden = fs.orden_topologico();
    int paso = 1;
    for (int64_t nid : orden) {
        const auto* n = fs.nodo_por_id(nid);
        if (!n) continue;
        std::string hdr = std::to_string(paso++) + ". " + n->nombre + " [" + nucleo::nombre_tipo_nodo(n->tipo) + "]";
        if (ImGui::TreeNodeEx(hdr.c_str(), ImGuiTreeNodeFlags_DefaultOpen)) {
            for (int64_t fid : n->ids_flujos_entrada) {
                if (const auto* f = fs.flujo_por_id(fid)) {
                    int tc = fs.clasificar_conexion_flujo(fid);
                    ImVec4 col = (tc == 0) ? AdaptiveColor(0.22f, 0.58f, 0.98f, 1.0f) : AdaptiveColor(0.75f, 0.78f, 0.82f, 1.0f);
                    ImGui::TextColored(col, "  <- In: %s (%.2f t/h, %.1f °C)",
                                       f->nombre.c_str(), f->flujo_masico_total(db_) * 3.6, f->temperatura - 273.15);
                }
            }
            for (int64_t fid : n->ids_flujos_salida) {
                if (const auto* f = fs.flujo_por_id(fid)) {
                    int tc = fs.clasificar_conexion_flujo(fid);
                    ImVec4 col = (tc == 2) ? AdaptiveColor(0.95f, 0.32f, 0.32f, 1.0f) : AdaptiveColor(0.75f, 0.78f, 0.82f, 1.0f);
                    ImGui::TextColored(col, "  -> Out: %s (%.2f t/h, %.1f °C)",
                                       f->nombre.c_str(), f->flujo_masico_total(db_) * 3.6, f->temperatura - 273.15);
                }
            }
            ImGui::TreePop();
        }
    }
    ImGui::EndChild();
    ImGui::Columns(1);
}

}} // namespace cma::interfaz
#pragma GCC diagnostic pop
