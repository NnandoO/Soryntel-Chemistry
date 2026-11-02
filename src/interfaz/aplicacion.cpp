#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wold-style-cast"
#pragma GCC diagnostic ignored "-Wsign-conversion"
#pragma GCC diagnostic ignored "-Wfloat-conversion"
#pragma GCC diagnostic ignored "-Wshadow"
#pragma GCC diagnostic ignored "-Wconversion"
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

static Aplicacion* instancia_global = nullptr;

// ═══════════════════════════════════════════════════════════════════════════
// Callbacks globales
// ═══════════════════════════════════════════════════════════════════════════

void glfw_error_cb(int error, const char* desc) {
    fprintf(stderr, "GLFW Error %d: %s\n", error, desc);
}

void glfw_redimensionar_cb(GLFWwindow* w, int ancho, int alto) {
    if (auto* app = (Aplicacion*)glfwGetWindowUserPointer(w)) {
        app->ancho_ = ancho;
        app->alto_ = alto;
    }
}

void glfw_cerrar_ventana_cb(GLFWwindow* w) {
    auto* app = (Aplicacion*)glfwGetWindowUserPointer(w);
    if (app && app->hay_cambios_sin_guardar_) {
        // Cancelar el cierre y mostrar diálogo de confirmación
        glfwSetWindowShouldClose(w, GLFW_FALSE);
        app->mostrar_confirmar_salir_ = true;
    }
}

void Aplicacion::error_glfw_cb(int error, const char* desc) { glfw_error_cb(error, desc); }
void Aplicacion::redimensionar_cb(GLFWwindow* w, int ancho, int alto) { glfw_redimensionar_cb(w, ancho, alto); }
void Aplicacion::scroll_cb(GLFWwindow*, double, double) {}

// ═══════════════════════════════════════════════════════════════════════════
// Aplicacion
// ═══════════════════════════════════════════════════════════════════════════

Aplicacion::Aplicacion(base_datos::GestorDB& repository, application::FileDialog& file_dialog,
                       const domain::AqueousDatabase& aqueous_database,const domain::AqueousEngine& aqueous_engine)
    : repository_(repository), file_dialog_(file_dialog), panel_native_aqueous_(aqueous_database),
      panel_advanced_aqueous_(aqueous_engine), panel_mineral_processing_(file_dialog), panel_catalog_audit_(repository,file_dialog), panel_estimacion_(repository),
      panel_reaction_solver_(repository), panel_pourbaix_(repository), panel_material_db_(repository) {
    instancia_global = this;
    flowsheet_ = std::make_unique<diagrama_flujo::Flowsheet>();

    lienzo_.establecer_flowsheet(flowsheet_.get());
    panel_propiedades_.establecer_flowsheet(flowsheet_.get());
    panel_herramientas_.establecer_flowsheet(flowsheet_.get());
    panel_material_db_.establecer_flowsheet(flowsheet_.get());
    panel_material_db_.al_enviar_a_balance = [this](const cma::application::modules::RegistroMaterial&, double, double) {
        mostrar_balance_hoja_ = true;
    };

    panel_herramientas_.al_agregar_nodo = [this](nucleo::TipoNodo tipo) {
        lienzo_.guardar_estado();
        auto id = flowsheet_->agregar_nodo(tipo);
        auto* n = flowsheet_->nodo_por_id(id);
        if (n) {
            n->nombre = std::string(nucleo::nombre_tipo_nodo(tipo)) + " #" + std::to_string(id);
            n->canvas_x = 50.0f + (contador_nodos_ % 5) * 160.0f;
            n->canvas_y = 50.0f + (contador_nodos_ / 5) * 90.0f;
            n->pagina = lienzo_.pagina_actual;
            contador_nodos_++;
        }
        panel_propiedades_.seleccionar_nodo(id);
        lienzo_.id_nodo_seleccionado = id;
        hay_cambios_sin_guardar_ = true;
    };

    panel_herramientas_.al_abrir_tabla          = [this]() { mostrar_tabla_ = true; };
    panel_herramientas_.al_abrir_calculadora    = [this]() { mostrar_calculadora_ = true; };
    panel_herramientas_.al_abrir_electroquimica = [this]() { mostrar_electroquimica_ = true; };
    panel_herramientas_.al_abrir_cinetica       = [this]() { mostrar_cinetica_ = true; };
    panel_herramientas_.al_abrir_equilibrio     = [this]() { mostrar_equilibrio_ = true; };
    panel_herramientas_.al_seleccionar_nodo     = [this](int64_t id) {
        panel_propiedades_.seleccionar_nodo(id);
        lienzo_.id_nodo_seleccionado = id;
    };
    panel_herramientas_.al_abrir_anotacion      = [this]() { agregar_anotacion(); };
    panel_herramientas_.al_agregar_tabla_flujo  = [this]() { agregar_tabla_flujo(); };
    panel_herramientas_.al_ejecutar_simulacion  = [this]() { ejecutar_simulacion(); };
    panel_herramientas_.al_ejecutar_convergencia = [this]() { ejecutar_convergencia(); };

    lienzo_.al_seleccionar_nodo = [this](int64_t id) {
        panel_propiedades_.seleccionar_nodo(id);
        lienzo_.id_nodo_seleccionado = id;
        lienzo_.id_flujo_seleccionado = -1;
        lienzo_.id_anotacion_seleccionada = -1;
    };

    lienzo_.al_seleccionar_flujo = [this](int64_t id) {
        panel_propiedades_.seleccionar_flujo(id);
        lienzo_.id_flujo_seleccionado = id;
        lienzo_.id_nodo_seleccionado = -1;
        lienzo_.id_anotacion_seleccionada = -1;
    };

    lienzo_.al_seleccionar_anotacion = [this](int64_t id) {
        panel_propiedades_.seleccionar_anotacion(id);
        lienzo_.id_anotacion_seleccionada = id;
        lienzo_.id_nodo_seleccionado = -1;
        lienzo_.id_flujo_seleccionado = -1;
    };

    lienzo_.al_crear_anotacion = [this](int64_t id) {
        pila_undo_acciones_.push_back({UndoAction::Tipo::Anotacion, id});
        hay_cambios_sin_guardar_ = true;
    };

    lienzo_.al_solicitar_agregar_nodo = [this](nucleo::TipoNodo tipo) {
        panel_herramientas_.al_agregar_nodo(tipo);
    };

    lienzo_.al_doble_clic_nodo = [this](int64_t id) {
        panel_propiedades_.abrir_editor_unidad(id);
    };

    lienzo_.establecer_callback_cambio([this]() { hay_cambios_sin_guardar_ = true; });

    panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
    panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
    panel_propiedades_.al_ejecutar_simulacion = [this]() { ejecutar_simulacion(); };
}

Aplicacion::~Aplicacion() {
    cerrar();
    instancia_global = nullptr;
}

// ═══════════════════════════════════════════════════════════════════════════
// Inicialización
// ═══════════════════════════════════════════════════════════════════════════

bool Aplicacion::iniciar(int argc, char** argv) {
    for (int i = 1; i < argc; ++i) {
        const std::string argument(argv[i]);
        if (argument=="--smoke-test") smoke_test_=true;
        if (argument.rfind("--review-panel=",0)==0) review_panel_=argument.substr(15);
    }
    if (!iniciar_ventana()) return false;
    if (!iniciar_imgui()) return false;

    cargar_ejemplo();
    if (!review_panel_.empty()) {
        mostrar_agua_=review_panel_=="water";
        mostrar_soluciones_=review_panel_=="aqueous";
        mostrar_mineral_processing_=review_panel_=="mineral";
        mostrar_catalog_audit_=review_panel_=="catalog";
        mostrar_balance_hoja_=review_panel_=="balance";
        mostrar_material_db_=review_panel_=="material";
        mostrar_lca_=review_panel_=="lca";
        mostrar_geo_mineralogical_=review_panel_=="geo";
        ImGui::GetIO().IniFilename=nullptr;
    }
    if (smoke_test_) {
        mostrar_agua_ = mostrar_statistical_analysis_ = mostrar_data_fit_ = true;
        mostrar_convertidor_ = mostrar_benson_ = mostrar_equilibrio_ = true;
        mostrar_pourbaix_ = mostrar_add_in_functions_ = mostrar_balance_hoja_ = true;
        mostrar_soluciones_ = mostrar_mineral_processing_ = mostrar_catalog_audit_ = true;
        mostrar_material_db_ = mostrar_geo_mineralogical_ = true;
        ImGui::GetIO().IniFilename = nullptr;
    }

    ejecutando_ = true;
    return true;
}

bool Aplicacion::iniciar_ventana() {
    glfwSetErrorCallback(glfw_error_cb);
    if (!glfwInit()) return false;

    glfwWindowHint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfwWindowHint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfwWindowHint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);
#ifdef __APPLE__
    glfwWindowHint(GLFW_OPENGL_FORWARD_COMPAT, GL_TRUE);
#endif

    ventana_ = glfwCreateWindow(ancho_, alto_, "CMA Chemistry v2.0", nullptr, nullptr);
    if (!ventana_) { glfwTerminate(); return false; }

    glfwMakeContextCurrent(ventana_);
    glfwSwapInterval(1);
    glfwSetWindowUserPointer(ventana_, this);
    glfwSetWindowSizeCallback(ventana_, glfw_redimensionar_cb);
    glfwSetWindowCloseCallback(ventana_, glfw_cerrar_ventana_cb);

    return true;
}

bool Aplicacion::iniciar_imgui() {
    IMGUI_CHECKVERSION();
    ImGui::CreateContext();
    ImPlot::CreateContext();
    ImPlot3D::CreateContext();

    ImGuiIO& io = ImGui::GetIO();
    io.ConfigFlags |= ImGuiConfigFlags_NavEnableKeyboard;
    io.ConfigDebugHighlightIdConflicts = false;
    io.IniFilename = "cmachemistry.ini";
    
    // Configurar fuente Roboto con subíndices y superíndices
    static const ImWchar ranges[] = {
        0x0020, 0x00FF, // Basic Latin + Latin Supplement
        0x0370, 0x03FF, // Greek (Δ, α, β, μ, σ...)
        0x2010, 0x2027, // General Punctuation (—, –, …, ·)
        0x2070, 0x209F, // Superscripts and Subscripts
        0x2190, 0x2193, // Arrows (←, →)
        0x2212, 0x2212, // Minus sign (−)
        0,
    };
    // Buscar la fuente en posibles directorios
    std::string font_path = adapters::resource_path("assets/fonts/OpenSans-Regular.ttf");
    FILE* f = fopen(font_path.c_str(), "rb");
    if (!f) {
        font_path = "../assets/fonts/OpenSans-Regular.ttf";
        f = fopen(font_path.c_str(), "rb");
    }
    
    if (f) {
        fclose(f);
        ImFontConfig config;
        config.MergeMode = false;
        io.FontDefault = io.Fonts->AddFontFromFileTTF(font_path.c_str(), 16.0f, &config, ranges);
    }
    
    if (!io.FontDefault) {
        io.FontDefault = io.Fonts->AddFontDefault();
    }

    configurar_estilo_claro();

    ImGui_ImplGlfw_InitForOpenGL(ventana_, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    return true;
}

void Aplicacion::configurar_estilo_oscuro() {
    css::apply_global_stylesheet(true);
    modo_oscuro_ = true;
}

void Aplicacion::configurar_estilo_claro() {
    css::apply_global_stylesheet(false);
    modo_oscuro_ = false;
}

void Aplicacion::alternar_modo() {
    if (modo_oscuro_)
        configurar_estilo_claro();
    else
        configurar_estilo_oscuro();
}

// ═══════════════════════════════════════════════════════════════════════════
// Bucle principal
// ═══════════════════════════════════════════════════════════════════════════

void Aplicacion::ejecutar() {
    std::string titulo_previo;
    int smoke_frames = 0;
    while (ejecutando_ && !glfwWindowShouldClose(ventana_)) {
        glfwPollEvents();
        frame_glfw();
        renderizar();

        // Actualizar título de ventana con estado de cambios
        {
            std::string titulo = "CMA Chemistry v2.0";
            if (!ruta_archivo_actual_.empty()) {
                auto pos = ruta_archivo_actual_.find_last_of('/');
                titulo += " - " + (pos != std::string::npos
                    ? ruta_archivo_actual_.substr(pos + 1)
                    : ruta_archivo_actual_);
            }
            if (hay_cambios_sin_guardar_) titulo += " *";
            if (titulo != titulo_previo) {
                glfwSetWindowTitle(ventana_, titulo.c_str());
                titulo_previo = titulo;
            }
        }

        ImGui::Render();
        glViewport(0, 0, ancho_, alto_);
        ImVec4 bg = ImGui::GetStyle().Colors[ImGuiCol_WindowBg];
        glClearColor(bg.x, bg.y, bg.z, bg.w);
        glClear(GL_COLOR_BUFFER_BIT);
        ImGui_ImplOpenGL3_RenderDrawData(ImGui::GetDrawData());
        glfwSwapBuffers(ventana_);
        if (smoke_test_ && ++smoke_frames >= 5) ejecutando_ = false;

        if (glfwGetKey(ventana_, GLFW_KEY_ESCAPE) == GLFW_PRESS) {
            if (hay_cambios_sin_guardar_) {
                mostrar_confirmar_salir_ = true;
            } else {
                ejecutando_ = false;
            }
        }
    }
}

void Aplicacion::frame_glfw() {
    ImGui_ImplOpenGL3_NewFrame();
    ImGui_ImplGlfw_NewFrame();
    ImGui::NewFrame();
}

// ═══════════════════════════════════════════════════════════════════════════
// Renderizado principal
// ═══════════════════════════════════════════════════════════════════════════

void Aplicacion::cerrar() {
    ImGui_ImplOpenGL3_Shutdown();
    ImGui_ImplGlfw_Shutdown();
    ImPlot3D::DestroyContext();
    ImPlot::DestroyContext();
    ImGui::DestroyContext();
    if (ventana_) { glfwDestroyWindow(ventana_); ventana_ = nullptr; }
    glfwTerminate();
}


}} // namespace

#pragma GCC diagnostic pop
