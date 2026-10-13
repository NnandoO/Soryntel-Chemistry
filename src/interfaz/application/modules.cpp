#include "interfaz/aplicacion.hpp"
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
        panel_tabla_.renderizar();
        ImGui::End();
    }

    if (mostrar_calculadora_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Calculadora Termodinámica"), &mostrar_calculadora_);
        auto& gestor = base_datos::GestorDB::instancia();
        auto db = gestor.mapa_especies();
        panel_calculadora_.renderizar(db);
        ImGui::End();
    }

    if (mostrar_electroquimica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Electroquímica"), &mostrar_electroquimica_);
        panel_electroquimica_.renderizar();
        ImGui::End();
    }

    if (mostrar_cinetica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Cinética"), &mostrar_cinetica_);
        panel_cinetica_.renderizar();
        ImGui::End();
    }

    if (mostrar_equilibrio_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Equilibrio de Fases"), &mostrar_equilibrio_);
        auto& gestor = base_datos::GestorDB::instancia();
        const auto& db = gestor.mapa_especies();
        panel_equilibrio_.renderizar(db);
        ImGui::End();
    }

    if (mostrar_sim_dinamica_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Simulación Dinámica"), &mostrar_sim_dinamica_);
        panel_sim_dinamica_.renderizar();
        ImGui::End();
    }

    if (mostrar_estabilidad_) {
        ImGui::SetNextWindowSize(ImVec2(900, 600), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Diagrama de Estabilidad"), &mostrar_estabilidad_);
        panel_estabilidad_.renderizar();
        ImGui::End();
    }

    if (mostrar_lpp_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 700), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Lpp Diagram - Stability Diagrams"), &mostrar_lpp_);
        panel_lpp_.renderizar();
        ImGui::End();
    }

    if (mostrar_reconciliacion_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(500,400), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(850,550), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("HSC Sim 8 - Mass Balance"), &mostrar_reconciliacion_);
        panel_reconciliacion_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_balance_hoja_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Balance (Hoja de Cálculo)"), &mostrar_balance_hoja_);
        panel_balance_hoja_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_exergia_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Análisis Exergético"), &mostrar_exergia_);
        panel_exergia_.renderizar();
        ImGui::End();
    }

    if (mostrar_pourbaix_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(800, 600), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Diagramas Eh-pH (Pourbaix)"), &mostrar_pourbaix_)) {
            panel_pourbaix_.renderizar();
        }
        ImGui::End();
    }

    if (mostrar_diagramas_termo_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Diagramas H, S, Cp, G"), &mostrar_diagramas_termo_);
        panel_diagramas_termo_.renderizar();
        ImGui::End();
    }



    if (mostrar_estabilidad_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Diagrama de Estabilidad"), &mostrar_estabilidad_);
        panel_estabilidad_.renderizar();
        ImGui::End();
    }

    if (mostrar_agua_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        if (review_panel_=="water") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(1100,700),ImGuiCond_Always); }
        ImGui::Begin(tr("Tablas de Vapor (Agua)"), &mostrar_agua_);
        panel_agua_.renderizar();
        ImGui::End();
    }

    if (mostrar_soluciones_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(800,650), ImGuiCond_FirstUseEver);
        if (review_panel_=="aqueous") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(950,700),ImGuiCond_Always); }
        ImGui::Begin(tr("Soluciones Acuosas"), &mostrar_soluciones_);
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
        if (ImGui::Begin("Procesamiento mineral",&mostrar_mineral_processing_)) panel_mineral_processing_.render();
        ImGui::End();
    }
    if (mostrar_catalog_audit_) {
        ImGui::SetNextWindowSize(ImVec2(900,520),ImGuiCond_FirstUseEver);
        if (review_panel_=="catalog") { ImGui::SetNextWindowPos(ImVec2(15,60),ImGuiCond_Always); ImGui::SetNextWindowSize(ImVec2(1100,700),ImGuiCond_Always); }
        if (ImGui::Begin("Auditoría del catálogo",&mostrar_catalog_audit_)) panel_catalog_audit_.render();
        ImGui::End();
    }

    if (mostrar_perdida_calor_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(600,450), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(950,620), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Pérdida de Calor"), &mostrar_perdida_calor_);
        panel_perdida_calor_.renderizar();
        ImGui::End();
    }

    if (mostrar_convertidor_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Convertidor de Especies"), &mostrar_convertidor_);
        panel_convertidor_.renderizar();
        ImGui::End();
    }

    if (mostrar_unidades_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Conversor de Unidades"), &mostrar_unidades_);
        panel_unidades_.renderizar();
        ImGui::End();
    }

    if (mostrar_estimacion_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Estimación Joback-Reid"), &mostrar_estimacion_);
        panel_estimacion_.renderizar();
        ImGui::End();
    }

    if (mostrar_benson_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Estimación Benson"), &mostrar_benson_);
        panel_benson_.renderizar();
        ImGui::End();
    }

    if (mostrar_lca_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(500,400), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(850,550), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Análisis de Ciclo de Vida (LCA Evaluation)"), &mostrar_lca_);
        panel_lca_.renderizar(flowsheet_.get());
        ImGui::End();
    }

    if (mostrar_datos_ann_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Datos / ANN"), &mostrar_datos_ann_);
        panel_datos_ann_.renderizar();
        ImGui::End();
    }

    if (mostrar_grafico3d_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(600,450), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Gráfico 3D"), &mostrar_grafico3d_);
        panel_grafico3d_.renderizar();
        ImGui::End();
    }

    if (mostrar_reaction_solver_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(500,400), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(900,600), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("4. Ecuaciones de Reacción"), &mostrar_reaction_solver_);
        panel_reaction_solver_.renderizar();
        ImGui::End();
    }

    if (mostrar_equilibrium_calc_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(600,500), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(1000,700), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("7. Cálculos de Equilibrio"), &mostrar_equilibrium_calc_);
        panel_equilibrium_calc_.renderizar();
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
        panel_api_externa_.renderizar();
        ImGui::End();
    }

    if (mostrar_statistical_analysis_) {
        ImGui::SetNextWindowSize(ImVec2(1200, 800), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("24. Análisis Estadístico de Datos"), &mostrar_statistical_analysis_);
        panel_stat_.renderizar();
        ImGui::End();
    }

    // Ventanas de marcadores de posición (placeholders) para los 24 módulos
    if (mostrar_lpp_diagrams_) {
        ImGui::SetNextWindowSize(ImVec2(400,300), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("12. Diagramas Lpp-Estabilidad"), &mostrar_lpp_diagrams_);
        ImGui::Text(tr("Módulo Lpp Diagrams - En desarrollo"));
        ImGui::Separator();
        ImGui::TextWrapped(tr("Este módulo calculará y graficará los diagramas de estabilidad Lpp de presión parcial para sistemas multicomponentes."));
        ImGui::End();
    }
    if (mostrar_add_in_functions_) {
        ImGui::SetNextWindowSize(ImVec2(800, 400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("19. Funciones AddIn"), &mostrar_add_in_functions_);
        panel_addin_.renderizar();
        ImGui::End();
    }
    if (mostrar_geo_mineralogical_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 700), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("21. Cálculos Geo-Mineralógicos"), &mostrar_geo_mineralogical_);
        panel_geo_.renderizar();
        ImGui::End();
    }
    if (mostrar_map_gps_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 600), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("22. Stock de Materiales Map-GPS"), &mostrar_map_gps_);
        panel_map_gps_.renderizar();
        ImGui::End();
    }
    
    if (mostrar_data_fit_) {
        ImGui::SetNextWindowSize(ImVec2(1000, 700), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("23. FIT - Ajuste Numérico de Datos"), &mostrar_data_fit_);
        panel_fit_.renderizar();
        ImGui::End();
    }

    if (mostrar_process_simulation_status_) {
        ImGui::SetNextWindowSize(ImVec2(600, 450), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("1. Sim - Simulación de Procesos"), &mostrar_process_simulation_status_);

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
