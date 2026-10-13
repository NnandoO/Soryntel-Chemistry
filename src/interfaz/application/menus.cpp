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
void Aplicacion::menu_principal() {
    if (ImGui::BeginMainMenuBar()) {
        menu_archivo();
        menu_editar();
        menu_simulacion();
        menu_herramientas();
        menu_modulos();
        if (ImGui::BeginMenu(tr("Ver"))) {
            if (ImGui::MenuItem(modo_oscuro_ ? tr("Modo Claro") : tr("Modo Oscuro")))
                alternar_modo();
            
            ImGui::Separator();
            if (ImGui::BeginMenu(tr("Idioma"))) {
                if (ImGui::MenuItem(tr("Español"), nullptr, utils::I18n::instancia().idioma_actual() == utils::I18n::Idioma::ESPANOL)) {
                    utils::I18n::instancia().cambiar_idioma(utils::I18n::Idioma::ESPANOL);
                }
                if (ImGui::MenuItem(tr("Inglés"), nullptr, utils::I18n::instancia().idioma_actual() == utils::I18n::Idioma::INGLES)) {
                    utils::I18n::instancia().cambiar_idioma(utils::I18n::Idioma::INGLES);
                }
                ImGui::EndMenu();
            }
            ImGui::EndMenu();
        }
        menu_ayuda();
        ImGui::EndMainMenuBar();
    }
}

void Aplicacion::menu_archivo() {
    if (ImGui::BeginMenu(tr("Archivo"))) {
        if (ImGui::MenuItem(tr("Nueva simulación"), "Ctrl+N")) {
            nueva_simulacion();
        }

        if (ImGui::BeginMenu(tr("Cargar ejemplo##sub"))) {
            if (ImGui::MenuItem(tr("SMR - Reformado de Metano"), "Ctrl+E")) {
                cargar_ejemplo(1);
            }
            if (ImGui::MenuItem(tr("Flotación de Cobre"))) {
                cargar_ejemplo(2);
            }
            if (ImGui::MenuItem(tr("Alto Horno de Hierro"))) {
                cargar_ejemplo(3);
            }
            if (ImGui::MenuItem(tr("SMR con Reciclo de CH₄ (Separador)"))) {
                cargar_ejemplo(4);
            }
            ImGui::EndMenu();
        }
        if (ImGui::MenuItem(tr("Abrir proyecto (.cma)"), "Ctrl+O"))
            cargar_proyecto();
        if (ImGui::MenuItem(tr("Guardar proyecto (.cma)"), "Ctrl+S", false, hay_cambios_sin_guardar_))
            guardar_proyecto();
        if (ImGui::MenuItem(tr("Guardar como..."), "Ctrl+Shift+S"))
            guardar_proyecto_como();
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Salir"), "Alt+F4"))
            ejecutando_ = false;
        ImGui::EndMenu();
    }
}

void Aplicacion::menu_editar() {
    if (ImGui::BeginMenu(tr("Editar"))) {
        if (ImGui::MenuItem(tr("Deshacer"), "Ctrl+Z", false, lienzo_.hay_deshacer()))
            lienzo_.deshacer();
        if (ImGui::MenuItem(tr("Rehacer"), "Ctrl+Y", false, lienzo_.hay_rehacer()))
            lienzo_.rehacer();
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Copiar"), "Ctrl+C"))
            copiar_seleccion();
        if (ImGui::MenuItem(tr("Pegar"), "Ctrl+V"))
            pegar_seleccion();
        if (ImGui::MenuItem(tr("Duplicar"), "Ctrl+D"))
            duplicar_seleccion();
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Eliminar"), "Supr")) {
            if (lienzo_.id_nodo_seleccionado > 0) {
                lienzo_.guardar_estado();
                flowsheet_->eliminar_nodo(lienzo_.id_nodo_seleccionado);
                lienzo_.id_nodo_seleccionado = -1;
                hay_cambios_sin_guardar_ = true;
            } else if (lienzo_.id_flujo_seleccionado > 0) {
                lienzo_.guardar_estado();
                flowsheet_->eliminar_flujo(lienzo_.id_flujo_seleccionado);
                lienzo_.id_flujo_seleccionado = -1;
                hay_cambios_sin_guardar_ = true;
            } else if (lienzo_.id_anotacion_seleccionada > 0) {
                lienzo_.guardar_estado();
                flowsheet_->eliminar_anotacion(lienzo_.id_anotacion_seleccionada);
                lienzo_.id_anotacion_seleccionada = -1;
                hay_cambios_sin_guardar_ = true;
            }
        }
        ImGui::EndMenu();
    }
}

void Aplicacion::menu_simulacion() {
    if (ImGui::BeginMenu(tr("Simulación"))) {
        if (ImGui::MenuItem(tr("Ejecutar"), "F5"))
            ejecutar_simulacion();
        if (ImGui::MenuItem(tr("Resolver Recirculaciones"), "Ctrl+R"))
            ejecutar_convergencia();
        if (ImGui::MenuItem(tr("Minimizar Gibbs"), "Ctrl+G"))
            ejecutar_gibbs();
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Exportar resultados a CSV")))
            exportar_resultados_csv();
        ImGui::Separator();
        if (ImGui::MenuItem(tr("Encajar Vista"), "F"))
            lienzo_.encajar_todo();
        if (ImGui::MenuItem(tr("Limpiar resultados")))
            panel_resultados_.limpiar();
        ImGui::EndMenu();
    }
}

void Aplicacion::menu_herramientas() {
    if (ImGui::BeginMenu(tr("Herramientas"))) {
        ImGui::MenuItem(tr("Calculadora termodinámica"), nullptr, &mostrar_calculadora_);
        ImGui::MenuItem(tr("Electroquímica"), nullptr, &mostrar_electroquimica_);
        ImGui::MenuItem(tr("Cinética química"), nullptr, &mostrar_cinetica_);
        ImGui::EndMenu();
    }
}

void Aplicacion::menu_modulos() {
    if (ImGui::BeginMenu(tr("Módulos"))) {
        ImGui::MenuItem(tr("1. Sim - Simulación de Procesos"), nullptr, &mostrar_process_simulation_status_);
        ImGui::MenuItem(tr("2. Evaluación LCA"), nullptr, &mostrar_lca_);
        ImGui::MenuItem(tr("3. Balance de Masas"), nullptr, &mostrar_reconciliacion_);
        ImGui::MenuItem(tr("4. Ecuaciones de Reacción"), nullptr, &mostrar_reaction_solver_);
        ImGui::MenuItem(tr("5. Balances Térmicos y Materiales"), nullptr, &mostrar_balance_hoja_);
        ImGui::MenuItem(tr("6. Calculadora de Pérdida de Calor"), nullptr, &mostrar_perdida_calor_);
        ImGui::MenuItem(tr("7. Cálculos de Equilibrio"), nullptr, &mostrar_equilibrio_);
        ImGui::MenuItem(tr("8. Balance de Exergía"), nullptr, &mostrar_exergia_);
        ImGui::MenuItem(tr("9. Diagramas Eh-pH (Pourbaix)"), nullptr, &mostrar_pourbaix_);

        ImGui::MenuItem(tr("10. Diagramas H, S, Cp y Ellingham"), nullptr, &mostrar_diagramas_termo_);
        ImGui::MenuItem(tr("11. Diagramas Tpp-Estabilidad"), nullptr, &mostrar_estabilidad_);
        ImGui::MenuItem(tr("12. Diagramas Lpp-Estabilidad"), nullptr, &mostrar_lpp_);

        ImGui::MenuItem(tr("13. Tablas de Vapor (Agua)"), nullptr, &mostrar_agua_);
        ImGui::MenuItem(tr("14. Estimaciones H, S, Cp"), nullptr, &mostrar_estimacion_);
        ImGui::MenuItem(tr("15. Estimación Benson"), nullptr, &mostrar_benson_);
        ImGui::MenuItem(tr("16. Convertidor de Especies"), nullptr, &mostrar_convertidor_);
        ImGui::MenuItem(tr("17. Tabla Periódica"), nullptr, &mostrar_tabla_);
        ImGui::MenuItem(tr("18. Unidades de Medida"), nullptr, &mostrar_unidades_);
        ImGui::MenuItem(tr("19. Funciones AddIn"), nullptr, &mostrar_add_in_functions_);
        ImGui::MenuItem(tr("20. Soluciones Acuosas"), nullptr, &mostrar_soluciones_);
        ImGui::MenuItem(tr("21. Cálculos Geo-Mineralógicos"), nullptr, &mostrar_geo_mineralogical_);
        ImGui::MenuItem(tr("22. Stock de Materiales Map-GPS"), nullptr, &mostrar_map_gps_);
        ImGui::MenuItem(tr("23. FIT - Ajuste Numérico de Datos"), nullptr, &mostrar_data_fit_);
        ImGui::MenuItem(tr("24. Análisis Estadístico de Datos"), nullptr, &mostrar_statistical_analysis_);
        ImGui::MenuItem("25. Procesamiento mineral", nullptr, &mostrar_mineral_processing_);
        ImGui::MenuItem("26. Auditoría del catálogo", nullptr, &mostrar_catalog_audit_);
        
        ImGui::EndMenu();
    }
}

void Aplicacion::menu_ayuda() {
    if (ImGui::BeginMenu(tr("Ayuda"))) {
        if (ImGui::MenuItem(tr("Acerca de CMA Chemistry")))
            ImGui::OpenPopup("Acerca de");
        ImGui::EndMenu();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Acciones de simulación
// ═══════════════════════════════════════════════════════════════════════════


}} // namespace
