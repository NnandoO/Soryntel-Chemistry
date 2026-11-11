#include "interfaz/aplicacion.hpp"
#include "interfaz/ui_css_style.hpp"
#include "utils/i18n.h"

#include <GLFW/glfw3.h>
#include <imgui.h>
#include <imgui_impl_glfw.h>
#include <imgui_impl_opengl3.h>
#include <implot.h>
#include <implot3d.h>

#include <cstdio>
#include <ctime>
#include <fstream>
#include <sstream>
#include <iomanip>
#include <filesystem>
#include <set>
#include <algorithm>
#include "adapters/desktop/process.hpp"
#include "application/SimulationService.hpp"

namespace cma { namespace interfaz {
void Aplicacion::renderizar_modulos() {
    // Ventanas flotantes
    if (mostrar_tabla_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Tabla Periódica"), &mostrar_tabla_);
        css::draw_module_banner("PT", "Periodic Table & Element Properties", "Atomic weights, oxidation states, and IUPAC thermodynamic reference data", IM_COL32(36, 98, 178, 255));
        panel_tabla_.renderizar();
        ImGui::End();
    }

    if (mostrar_calculadora_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Calculadora Termodinámica"), &mostrar_calculadora_);
        css::draw_module_banner("H/S", "H, S, C and G Thermodynamic Calculator", "Shomate / NASA / Maier-Kelley polynomial evaluation across temperature ranges", IM_COL32(42, 128, 76, 255));
        auto& gestor = base_datos::GestorDB::instancia();
        auto db = gestor.mapa_especies();
        panel_calculadora_.renderizar(db);
        ImGui::End();
    }

    if (mostrar_electroquimica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Electroquímica"), &mostrar_electroquimica_);
        css::draw_module_banner("CELL", "Electrochemical Cell Equilibria", "Nernst potentials, half-cell reactions, and Faraday electrolysis balances", IM_COL32(142, 68, 173, 255));
        panel_electroquimica_.renderizar();
        ImGui::End();
    }

    if (mostrar_cinetica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Cinética"), &mostrar_cinetica_);
        css::draw_module_banner("KIN", "Chemical Reaction Kinetics", "Arrhenius rate laws, batch/CSTR/PFR integration, and conversion profiles", IM_COL32(204, 102, 24, 255));
        panel_cinetica_.renderizar();
        ImGui::End();
    }

    if (mostrar_equilibrio_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(600,500), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(1100,750), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("7. Cálculos de Equilibrio (GEM)"), &mostrar_equilibrio_);
        css::draw_module_banner("GEM", "Gibbs Energy Minimization (Equilibrium)", "Multiphase constrained Gibbs free energy solver with elemental conservation", IM_COL32(36, 98, 178, 255));
        auto& gestor = base_datos::GestorDB::instancia();
        const auto& db = gestor.mapa_especies();
        panel_equilibrio_.renderizar(db);
        ImGui::End();
    }

    if (mostrar_sim_dinamica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Simulación Dinámica"), &mostrar_sim_dinamica_);
        css::draw_module_banner("DYN", "Dynamic Process Simulation", "Time-dependent holdup integration, PID control loops, and transient response", IM_COL32(42, 128, 76, 255));
        panel_sim_dinamica_.renderizar();
        ImGui::End();
    }

    if (mostrar_estabilidad_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(760, 520), ImVec2(3000, 3000));
        ImGui::SetNextWindowSize(ImVec2(1180, 760), ImGuiCond_FirstUseEver);
        if (review_panel_ == "tpp") {
            const ImVec2 vp = ImGui::GetMainViewport()->Size;
            ImGui::SetNextWindowPos(ImVec2(10.0f, 35.0f), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(std::max(vp.x - 20.0f, 800.0f), std::max(vp.y - 50.0f, 560.0f)), ImGuiCond_Always);
        }
        ImGui::Begin("Tpp Predominance Diagrams", &mostrar_estabilidad_, ImGuiWindowFlags_MenuBar);
        panel_estabilidad_.renderizar();
        ImGui::End();
    }

    if (mostrar_lpp_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(760, 520), ImVec2(3000, 3000));
        ImGui::SetNextWindowSize(ImVec2(1180, 760), ImGuiCond_FirstUseEver);
        if (review_panel_ == "lpp") {
            const ImVec2 vp = ImGui::GetMainViewport()->Size;
            ImGui::SetNextWindowPos(ImVec2(10.0f, 35.0f), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(std::max(vp.x - 20.0f, 800.0f), std::max(vp.y - 50.0f, 560.0f)), ImGuiCond_Always);
        }
        ImGui::Begin("Lpp Phase Stability Diagram", &mostrar_lpp_, ImGuiWindowFlags_MenuBar);
        panel_lpp_.renderizar();
        ImGui::End();
    }

    if (mostrar_reconciliacion_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(500,400), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(850,550), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("HSC Sim 8 - Mass Balance"), &mostrar_reconciliacion_);
        css::draw_module_banner("MAS", "HSC Sim Mass Balance & Data Reconciliation", "Weighted least-squares mineral and elemental mass balance reconciliation", IM_COL32(42, 128, 76, 255));
        panel_reconciliacion_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_balance_hoja_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(760,420), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(1360,820), ImGuiCond_FirstUseEver);
        if (review_panel_ == "balance") {
            const ImVec2 vp = ImGui::GetMainViewport()->Size;
            ImGui::SetNextWindowPos(ImVec2(10.0f, 35.0f), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(std::max(vp.x - 20.0f, 600.0f), std::max(vp.y - 50.0f, 400.0f)), ImGuiCond_Always);
        }
        ImGui::Begin(tr("5. Balances de Calor y Materiales (HSC)"), &mostrar_balance_hoja_);
        panel_balance_hoja_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_exergia_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Análisis Exergético"), &mostrar_exergia_);
        css::draw_module_banner("EXG", "Exergy & Second-Law Analysis", "Physical and chemical exergy, Szargut reference environment, and irreversibility", IM_COL32(204, 102, 24, 255));
        panel_exergia_.renderizar();
        ImGui::End();
    }

    if (mostrar_pourbaix_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(780, 540), ImVec2(3000, 3000));
        ImGui::SetNextWindowSize(ImVec2(1220, 780), ImGuiCond_FirstUseEver);
        if (review_panel_ == "pourbaix" || review_panel_ == "eph") {
            const ImVec2 vp = ImGui::GetMainViewport()->Size;
            ImGui::SetNextWindowPos(ImVec2(10.0f, 35.0f), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(std::max(vp.x - 20.0f, 800.0f), std::max(vp.y - 50.0f, 560.0f)), ImGuiCond_Always);
        }
        if (ImGui::Begin("Eh-pH Diagram", &mostrar_pourbaix_, ImGuiWindowFlags_MenuBar)) {
            panel_pourbaix_.renderizar();
        }
        ImGui::End();
    }

    if (mostrar_diagramas_termo_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Diagramas H, S, Cp, G"), &mostrar_diagramas_termo_);
        css::draw_module_banner("DIA", "Ellingham & Thermodynamic Diagrams (H, S, Cp, G)", "Comparative species & reaction property plots as a function of temperature", IM_COL32(186, 52, 46, 255));
        panel_diagramas_termo_.renderizar();
        ImGui::End();
    }

    if (mostrar_agua_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        if (review_panel_=="water") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(1100,700),ImGuiCond_Always); }
        ImGui::Begin(tr("Tablas de Vapor (Agua)"), &mostrar_agua_);
        css::draw_module_banner("H2O", "IAPWS-IF97 Steam & Water Tables", "High-precision thermodynamic and transport properties of water and steam", IM_COL32(36, 98, 178, 255));
        panel_agua_.renderizar();
        ImGui::End();
    }

    if (mostrar_soluciones_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(800,650), ImGuiCond_FirstUseEver);
        if (review_panel_=="aqueous") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(950,700),ImGuiCond_Always); }
        ImGui::Begin(tr("Soluciones Acuosas"), &mostrar_soluciones_);
        css::draw_module_banner("AQU", "Aqueous Electrolyte Solutions", "Debye-Hückel, Davies, B-dot, and Pitzer activity models for hydrometallurgy", IM_COL32(42, 128, 76, 255));
        if (ImGui::BeginTabBar("aqueous_models")) {
            if (ImGui::BeginTabItem("Avanzado USGS / Pitzer")) { panel_advanced_aqueous_.render(); ImGui::EndTabItem(); }
            if (ImGui::BeginTabItem("Equilibrio MINTEQ")) { panel_native_aqueous_.render(); ImGui::EndTabItem(); }
            if (ImGui::BeginTabItem("Hoja heredada (aproximada)")) { panel_soluciones_.renderizar(); ImGui::EndTabItem(); }
            ImGui::EndTabBar();
        }
        ImGui::End();
    }
    if (mostrar_mineral_processing_) {
        ImGui::SetNextWindowSize(ImVec2(900,680),ImGuiCond_FirstUseEver);
        if (review_panel_=="mineral") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(1100,700),ImGuiCond_Always); }
        if (ImGui::Begin("Procesamiento mineral",&mostrar_mineral_processing_)) {
            css::draw_module_banner("MIN", "Mineral Processing & Comminution", "Bond work index, Rosin-Rammler PSD, Tromp partition curves, and flotation recovery", IM_COL32(118, 146, 60, 255));
            panel_mineral_processing_.render();
        }
        ImGui::End();
    }
    if (mostrar_catalog_audit_) {
        ImGui::SetNextWindowSize(ImVec2(900,520),ImGuiCond_FirstUseEver);
        if (review_panel_=="catalog") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(1100,700),ImGuiCond_Always); }
        if (ImGui::Begin("Auditoría del catálogo",&mostrar_catalog_audit_)) {
            css::draw_module_banner("DB", "Thermodynamic Catalog Audit", "Database integrity verification, temperature range coverage, and source provenance", IM_COL32(36, 98, 178, 255));
            panel_catalog_audit_.render();
        }
        ImGui::End();
    }

    if (mostrar_perdida_calor_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(600,450), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(950,620), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Pérdida de Calor"), &mostrar_perdida_calor_);
        css::draw_module_banner("HLI", "Heat Loss & Refractory Insulation Calculator", "Multilayer conduction, convection, and thermal radiation heat transfer", IM_COL32(186, 52, 46, 255));
        panel_perdida_calor_.renderizar();
        ImGui::End();
    }

    if (mostrar_convertidor_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Convertidor de Especies"), &mostrar_convertidor_);
        css::draw_module_banner("CON", "Species <-> Elemental Assay Converter", "Stoichiometric mineralogy reconstruction from elemental XRF/ICP assays", IM_COL32(118, 146, 60, 255));
        panel_convertidor_.renderizar();
        ImGui::End();
    }

    if (mostrar_unidades_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Conversor de Unidades"), &mostrar_unidades_);
        css::draw_module_banner("MEA", "Engineering Unit Converter (Measure)", "Dimensional analysis and conversion across SI, Imperial, and metallurgical units", IM_COL32(36, 98, 178, 255));
        panel_unidades_.renderizar();
        ImGui::End();
    }

    if (mostrar_estimacion_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Estimación Joback-Reid"), &mostrar_estimacion_);
        css::draw_module_banner("JOB", "Joback-Reid Group Contribution Estimator", "Critical properties, normal boiling point, Hf, Gf, and Cp ideal gas estimation", IM_COL32(142, 68, 173, 255));
        panel_estimacion_.renderizar();
        ImGui::End();
    }

    if (mostrar_benson_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Estimación Benson"), &mostrar_benson_);
        css::draw_module_banner("BEN", "Benson Thermochemical Group Additivity", "Second-order group additivity for gas-phase enthalpy, entropy, and heat capacity", IM_COL32(142, 68, 173, 255));
        panel_benson_.renderizar();
        ImGui::End();
    }

    if (mostrar_lca_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(900,600), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(1280,800), ImGuiCond_FirstUseEver);
        if (review_panel_ == "lca") {
            ImGui::SetNextWindowPos(ImVec2(12, 52), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(1250, 720), ImGuiCond_Always);
        }
        ImGui::Begin(tr("LCA Evaluation"), &mostrar_lca_);
        panel_lca_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_datos_ann_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(700,500), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(1250,800), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Dat - Data (Redes Neuronales)"), &mostrar_datos_ann_);
        css::draw_module_banner("DAT", "HSC Data & Neural Network Modeling", "Multilayer perceptron training, response surfaces, and multivariate process modeling", IM_COL32(36, 98, 178, 255));
        panel_datos_ann_.renderizar();
        ImGui::End();
    }

    if (mostrar_grafico3d_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(600,450), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Gráfico 3D"), &mostrar_grafico3d_);
        css::draw_module_banner("3D", "3D Thermodynamic Surface Visualization", "Interactive 3D surfaces for G(T,P), phase stability, and response landscapes", IM_COL32(42, 128, 76, 255));
        panel_grafico3d_.renderizar();
        ImGui::End();
    }

    if (mostrar_reaction_solver_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(500,400), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(900,600), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("4. Ecuaciones de Reacción"), &mostrar_reaction_solver_);
        css::draw_module_banner("REA", "Reaction Equations (H, S, G, K, log K, E)", "Automatic stoichiometric matrix balancing and temperature-tabulated reaction properties", IM_COL32(36, 98, 178, 255));
        panel_reaction_solver_.renderizar();
        ImGui::End();
    }

    if (ImGui::BeginPopupModal("Confirmar Nueva", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::Text(tr("¿Está seguro? Se perderán todos los datos no guardados."));
        ImGui::Spacing();
        if (ImGui::Button(tr("Sí, crear nueva"), ImVec2(140, 24))) {
            nueva_simulacion();
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(100, 24))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    // Popups de abrir y guardar nativos ahora (eliminados los ImGui)

    if (mostrar_api_externa_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(350,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,350), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("API Externa"), &mostrar_api_externa_);
        css::draw_module_banner("API", "External Thermodynamic Database Connector", "NIST WebBook, Materials Project, and custom JSON/CSV species import", IM_COL32(36, 98, 178, 255));
        panel_api_externa_.renderizar();
        ImGui::End();
    }

    if (mostrar_statistical_analysis_) {
        ImGui::SetNextWindowSize(ImVec2(1200, 800), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("24. Análisis Estadístico de Datos"), &mostrar_statistical_analysis_);
        css::draw_module_banner("STA", "Multivariate Statistical Analysis & PCA", "Correlation matrices, Principal Component Analysis, and regression diagnostics", IM_COL32(204, 102, 24, 255));
        panel_stat_.renderizar();
        ImGui::End();
    }

    // Ventanas de marcadores de posición (placeholders) para los 24 módulos
    if (mostrar_lpp_diagrams_) {
        ImGui::SetNextWindowSize(ImVec2(400,300), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("12. Diagramas Lpp-Estabilidad"), &mostrar_lpp_diagrams_);
        css::draw_module_banner("Lpp", "Lpp Partial Pressure Diagrams", "Kellogg predominance diagrams", IM_COL32(36, 98, 178, 255));
        ImGui::Text(tr("Módulo Lpp Diagrams - En desarrollo"));
        ImGui::Separator();
        ImGui::TextWrapped(tr("Este módulo calculará y graficará los diagramas de estabilidad Lpp de presión parcial para sistemas multicomponentes."));
        ImGui::End();
    }
    if (mostrar_add_in_functions_) {
        ImGui::SetNextWindowSize(ImVec2(800, 400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("19. Funciones AddIn"), &mostrar_add_in_functions_);
        css::draw_module_banner("XLS", "Spreadsheet Add-In Functions", "Direct formula evaluation (=H_T, =G_T, =Cp_T, =MW, =logK) for process sheets", IM_COL32(42, 128, 76, 255));
        panel_addin_.renderizar();
        ImGui::End();
    }
    if (mostrar_geo_mineralogical_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(900, 580), ImVec2(3000, 3000));
        ImGui::SetNextWindowSize(ImVec2(1240, 740), ImGuiCond_FirstUseEver);
        if (review_panel_ == "geo") {
            ImGui::SetNextWindowPos(ImVec2(12, 52), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(1250, 720), ImGuiCond_Always);
        }
        ImGui::Begin(tr("Modal Calculations - HSC Geo 8.0"), &mostrar_geo_mineralogical_);
        panel_geo_.renderizar();
        ImGui::End();
    }
    if (mostrar_map_gps_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 600), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("22. Stock de Materiales Map-GPS"), &mostrar_map_gps_);
        css::draw_module_banner("MAP", "Geospatial Material Stock & Deposit Tracker", "GPS coordinates, tonnage inventory, and ore grade distribution", IM_COL32(36, 98, 178, 255));
        panel_map_gps_.renderizar();
        ImGui::End();
    }

    if (mostrar_material_db_) {
        if (review_panel_ == "material") {
            ImGui::SetNextWindowPos(ImVec2(12, 52), ImGuiCond_Always);
            ImGui::SetNextWindowSize(ImVec2(1250, 720), ImGuiCond_Always);
        }
        panel_material_db_.renderizar(&mostrar_material_db_);
    }

    if (panel_sim_tools_.visible) {
        motores::BalanceMasa motor_bm_sim(repository_);
        panel_sim_tools_.renderizar(*flowsheet_, motor_bm_sim, sim_ultimo_balance_masa_);
    }
    
    if (mostrar_data_fit_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 700), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("23. FIT - Ajuste Numérico de Datos"), &mostrar_data_fit_);
        css::draw_module_banner("FIT", "Non-Linear Curve & Cp Polynomial Fitting", "Least-squares fitting of Shomate/Maier-Kelley thermodynamic coefficients", IM_COL32(142, 68, 173, 255));
        panel_fit_.renderizar();
        ImGui::End();
    }

    if (mostrar_process_simulation_status_) {
        ImGui::SetNextWindowSize(ImVec2(600, 450), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("1. Sim - Simulación de Procesos"), &mostrar_process_simulation_status_);
        css::draw_module_banner("SIM", "HSC Sim Flowsheet Convergence & Diagnostics", "Topological tear-stream analysis and Wegstein recycle acceleration", IM_COL32(42, 128, 76, 255));

        auto& fs = flowsheet_;
        int total_nodos = (int)fs->conteo_nodos();
        int total_flujos = (int)fs->conteo_flujos();
        int total_conexiones = 0;
        bool tiene_ciclo = fs->detectar_ciclo();

        // Contar conexiones y clasificar nodos
        int n_fuentes = 0, n_sumideros = 0, n_reactores = 0, n_equipos = 0;
        for (const auto& nodo : fs->nodos()) {
            total_conexiones += (int)nodo.ids_flujos_salida.size();
            switch (nodo.tipo) {
                case nucleo::TipoNodo::Fuente: n_fuentes++; break;
                case nucleo::TipoNodo::Sumidero: n_sumideros++; break;
                case nucleo::TipoNodo::Reactor:
                case nucleo::TipoNodo::ReactorGibbs:
                case nucleo::TipoNodo::ReactorEsteq:
                case nucleo::TipoNodo::ReactorCinetica:
                    n_reactores++; break;
                default:
                    if (nodo.tipo != nucleo::TipoNodo::CajaTexto &&
                        nodo.tipo != nucleo::TipoNodo::Rectangulo &&
                        nodo.tipo != nucleo::TipoNodo::Elipse &&
                        nodo.tipo != nucleo::TipoNodo::FlujoEnergia)
                        n_equipos++;
                    break;
            }
        }

        // ── Resumen del flowsheet ──
        ImGui::Text("%s", tr("Estado del Flowsheet"));
        ImGui::Separator();

        if (ImGui::BeginTable("##sim_status", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("Nodos totales");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", total_nodos);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("  • Fuentes");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", n_fuentes);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("  • Sumideros");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", n_sumideros);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("  • Reactores");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", n_reactores);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("  • Otros equipos");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", n_equipos);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("Flujos (streams)");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", total_flujos);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("Conexiones");
            ImGui::TableSetColumnIndex(1); ImGui::Text("%d", total_conexiones);
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0); ImGui::Text("¿Tiene ciclos?");
            ImGui::TableSetColumnIndex(1);
            if (tiene_ciclo) {
                ImGui::TextColored(ImVec4(0.9f, 0.5f, 0.2f, 1.0f), "Sí (reciclo)");
            } else {
                ImGui::TextColored(ImVec4(0.3f, 0.8f, 0.3f, 1.0f), "No");
            }
            ImGui::EndTable();
        }

        ImGui::Spacing();

        // ── Botón resolver convergencia ──
        if (ImGui::Button(tr("Resolver convergencia (Wegstein)"))) {
            auto resultado = fs->resolver_convergencia(repository_, 500, 1e-6);
            sim_ultimo_resultado_ = resultado;
            sim_resuelto_ = true;
        }

        ImGui::SameLine();
        ImGui::TextDisabled("(?)");
        if (ImGui::IsItemHovered())
            ImGui::SetTooltip("%s", tr("Resuelve reciclos iterando con el método de Wegstein.\nMax 500 iteraciones, tolerancia 1e-6."));

        if (sim_resuelto_) {
            ImGui::Separator();
            ImGui::Text("%s", tr("Resultado de la última simulación"));
            if (ImGui::BeginTable("##sim_result", 2, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::Text("Estado");
                ImGui::TableSetColumnIndex(1);
                if (sim_ultimo_resultado_.convergio)
                    ImGui::TextColored(ImVec4(0.3f, 0.8f, 0.3f, 1.0f), "Convergió ✓");
                else
                    ImGui::TextColored(ImVec4(0.9f, 0.3f, 0.3f, 1.0f), "No convergió ✗");
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::Text("Iteraciones");
                ImGui::TableSetColumnIndex(1); ImGui::Text("%d", sim_ultimo_resultado_.iteraciones);
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0); ImGui::Text("Error final");
                ImGui::TableSetColumnIndex(1); ImGui::Text("%.2e", sim_ultimo_resultado_.error_final);
                if (!sim_ultimo_resultado_.mensaje.empty()) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0); ImGui::Text("Mensaje");
                    ImGui::TableSetColumnIndex(1); ImGui::TextWrapped("%s", sim_ultimo_resultado_.mensaje.c_str());
                }
                ImGui::EndTable();
            }
        }

        ImGui::Spacing();

        // ── Lista de nodos con estado ──
        if (total_nodos > 0 && ImGui::CollapsingHeader(tr("Nodos del flowsheet"))) {
            if (ImGui::BeginTable("##sim_nodos", 4,
                    ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY | ImGuiTableFlags_Resizable)) {
                ImGui::TableSetupColumn("ID", ImGuiTableColumnFlags_WidthFixed, 50);
                ImGui::TableSetupColumn("Nombre", ImGuiTableColumnFlags_WidthFixed, 150);
                ImGui::TableSetupColumn("Tipo", ImGuiTableColumnFlags_WidthFixed, 120);
                ImGui::TableSetupColumn("Estado");
                ImGui::TableHeadersRow();

                auto orden = fs->orden_topologico();
                for (const auto& nodo : fs->nodos()) {
                    ImGui::TableNextRow();
                    ImGui::TableSetColumnIndex(0);
                    ImGui::Text("%lld", (long long)nodo.id);
                    ImGui::TableSetColumnIndex(1);
                    ImGui::TextUnformatted(nodo.nombre.c_str());
                    ImGui::TableSetColumnIndex(2);
                    ImGui::TextUnformatted(nucleo::nombre_tipo_nodo(nodo.tipo));
                    ImGui::TableSetColumnIndex(3);
                    // Marca como "listo" si está en el orden topológico
                    bool en_orden = std::find(orden.begin(), orden.end(), nodo.id) != orden.end();
                    if (en_orden)
                        ImGui::TextColored(ImVec4(0.3f, 0.8f, 0.3f, 1.0f), "OK");
                    else
                        ImGui::TextDisabled("—");
                }
                ImGui::EndTable();
            }
        }

        if (total_nodos == 0 && total_flujos == 0) {
            ImGui::Separator();
            ImGui::TextWrapped("%s", tr("El flowsheet está vacío. Agrega equipos desde la caja de herramientas."));
        }

        ImGui::End();
    }

    // ── Popup: Confirmar salida con cambios sin guardar ──
    if (mostrar_confirmar_salir_) {
        ImGui::OpenPopup("Confirmar Salida");
        mostrar_confirmar_salir_ = false;
    }
    if (ImGui::BeginPopupModal("Confirmar Salida", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::Text("%s", tr("Hay cambios sin guardar."));
        ImGui::Text("%s", tr("¿Desea guardar antes de salir?"));
        ImGui::Spacing();
        if (ImGui::Button(tr("Guardar y salir"), ImVec2(160, 0))) {
            guardar_proyecto();
            // Solo salir si el guardado fue exitoso (no hay cambios pendientes)
            if (!hay_cambios_sin_guardar_) {
                ejecutando_ = false;
                ImGui::CloseCurrentPopup();
            }
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Salir sin guardar"), ImVec2(160, 0))) {
            hay_cambios_sin_guardar_ = false;
            ejecutando_ = false;
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Cancelar"), ImVec2(120, 0))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Menú principal
// ═══════════════════════════════════════════════════════════════════════════



}} // namespace
