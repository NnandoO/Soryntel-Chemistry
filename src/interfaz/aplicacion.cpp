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
      panel_reaction_solver_(repository), panel_pourbaix_(repository) {
    instancia_global = this;
    flowsheet_ = std::make_unique<diagrama_flujo::Flowsheet>();

    lienzo_.establecer_flowsheet(flowsheet_.get());
    panel_propiedades_.establecer_flowsheet(flowsheet_.get());
    panel_herramientas_.establecer_flowsheet(flowsheet_.get());

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

    lienzo_.establecer_callback_cambio([this]() { hay_cambios_sin_guardar_ = true; });

    panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
    panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
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
        ImGui::GetIO().IniFilename=nullptr;
    }
    if (smoke_test_) {
        mostrar_agua_ = mostrar_statistical_analysis_ = mostrar_data_fit_ = true;
        mostrar_convertidor_ = mostrar_benson_ = mostrar_equilibrio_ = true;
        mostrar_pourbaix_ = mostrar_add_in_functions_ = mostrar_balance_hoja_ = true;
        mostrar_soluciones_ = mostrar_mineral_processing_ = mostrar_catalog_audit_ = true;
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
        0x2070, 0x209F, // Superscripts and Subscripts
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

    configurar_estilo_oscuro();

    ImGui_ImplGlfw_InitForOpenGL(ventana_, true);
    ImGui_ImplOpenGL3_Init("#version 330");

    return true;
}

void Aplicacion::configurar_estilo_oscuro() {
    ImGuiStyle& style = ImGui::GetStyle();
    // Material Design 3 Styling
    style.WindowPadding = ImVec2(16.0f, 16.0f);
    style.FramePadding = ImVec2(12.0f, 8.0f);
    style.ItemSpacing = ImVec2(12.0f, 12.0f);
    style.ItemInnerSpacing = ImVec2(8.0f, 8.0f);
    
    // Rounded Corners (Pill shape for buttons/frames)
    style.WindowRounding = 12.0f;
    style.FrameRounding = 8.0f;
    style.PopupRounding = 8.0f;
    style.GrabRounding = 8.0f;
    style.ScrollbarRounding = 12.0f;
    style.TabRounding = 8.0f;
    
    // No borders
    style.WindowBorderSize = 0.0f;
    style.FrameBorderSize = 0.0f;
    style.PopupBorderSize = 0.0f;
    style.ChildBorderSize = 0.0f;
    style.TabBorderSize = 0.0f;

    ImVec4* colors = style.Colors;
    colors[ImGuiCol_Text]                 = ImVec4(0.91f, 0.92f, 0.93f, 1.00f);
    colors[ImGuiCol_TextDisabled]         = ImVec4(0.60f, 0.63f, 0.66f, 1.00f);
    colors[ImGuiCol_WindowBg]             = ImVec4(0.13f, 0.13f, 0.14f, 1.00f); // #202124
    colors[ImGuiCol_ChildBg]              = ImVec4(0.16f, 0.16f, 0.18f, 1.00f); // #292a2d
    colors[ImGuiCol_PopupBg]              = ImVec4(0.16f, 0.16f, 0.18f, 0.98f);
    colors[ImGuiCol_Border]               = ImVec4(0.35f, 0.35f, 0.38f, 0.50f);
    colors[ImGuiCol_BorderShadow]         = ImVec4(0.00f, 0.00f, 0.00f, 0.00f);
    colors[ImGuiCol_FrameBg]              = ImVec4(0.20f, 0.20f, 0.22f, 1.00f); // Input fields
    colors[ImGuiCol_FrameBgHovered]       = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_FrameBgActive]        = ImVec4(0.30f, 0.32f, 0.35f, 1.00f);
    colors[ImGuiCol_TitleBg]              = ImVec4(0.13f, 0.13f, 0.14f, 1.00f);
    colors[ImGuiCol_TitleBgActive]        = ImVec4(0.13f, 0.13f, 0.14f, 1.00f);
    colors[ImGuiCol_TitleBgCollapsed]     = ImVec4(0.13f, 0.13f, 0.14f, 0.75f);
    colors[ImGuiCol_MenuBarBg]            = ImVec4(0.13f, 0.13f, 0.14f, 1.00f);
    colors[ImGuiCol_ScrollbarBg]          = ImVec4(0.13f, 0.13f, 0.14f, 1.00f);
    colors[ImGuiCol_ScrollbarGrab]        = ImVec4(0.35f, 0.37f, 0.40f, 1.00f);
    colors[ImGuiCol_ScrollbarGrabHovered] = ImVec4(0.45f, 0.47f, 0.50f, 1.00f);
    colors[ImGuiCol_ScrollbarGrabActive]  = ImVec4(0.55f, 0.57f, 0.60f, 1.00f);
    colors[ImGuiCol_CheckMark]            = ImVec4(0.54f, 0.71f, 0.97f, 1.00f); // #8ab4f8
    colors[ImGuiCol_SliderGrab]           = ImVec4(0.54f, 0.71f, 0.97f, 1.00f);
    colors[ImGuiCol_SliderGrabActive]     = ImVec4(0.64f, 0.81f, 1.00f, 1.00f);
    colors[ImGuiCol_Button]               = ImVec4(0.20f, 0.21f, 0.24f, 1.00f);
    colors[ImGuiCol_ButtonHovered]        = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_ButtonActive]         = ImVec4(0.54f, 0.71f, 0.97f, 0.40f);
    colors[ImGuiCol_Header]               = ImVec4(0.20f, 0.21f, 0.24f, 1.00f);
    colors[ImGuiCol_HeaderHovered]        = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_HeaderActive]         = ImVec4(0.54f, 0.71f, 0.97f, 0.40f);
    colors[ImGuiCol_Separator]            = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_SeparatorHovered]     = ImVec4(0.54f, 0.71f, 0.97f, 0.50f);
    colors[ImGuiCol_SeparatorActive]      = ImVec4(0.54f, 0.71f, 0.97f, 1.00f);
    colors[ImGuiCol_ResizeGrip]           = ImVec4(0.54f, 0.71f, 0.97f, 0.20f);
    colors[ImGuiCol_ResizeGripHovered]    = ImVec4(0.54f, 0.71f, 0.97f, 0.60f);
    colors[ImGuiCol_ResizeGripActive]     = ImVec4(0.54f, 0.71f, 0.97f, 0.90f);
    colors[ImGuiCol_Tab]                  = ImVec4(0.16f, 0.16f, 0.18f, 1.00f);
    colors[ImGuiCol_TabHovered]           = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_TabActive]            = ImVec4(0.25f, 0.26f, 0.29f, 1.00f);
    colors[ImGuiCol_TabUnfocused]         = ImVec4(0.13f, 0.13f, 0.14f, 1.00f);
    colors[ImGuiCol_TabUnfocusedActive]   = ImVec4(0.16f, 0.16f, 0.18f, 1.00f);
    colors[ImGuiCol_PlotLines]            = ImVec4(0.54f, 0.71f, 0.97f, 1.00f);
    colors[ImGuiCol_PlotLinesHovered]     = ImVec4(0.00f, 0.80f, 1.00f, 1.00f);
    colors[ImGuiCol_PlotHistogram]        = ImVec4(0.10f, 0.80f, 0.40f, 1.00f);
    colors[ImGuiCol_PlotHistogramHovered] = ImVec4(0.20f, 0.90f, 0.50f, 1.00f);
    colors[ImGuiCol_TableHeaderBg]        = ImVec4(0.14f, 0.15f, 0.19f, 1.00f);
    colors[ImGuiCol_TableBorderStrong]     = ImVec4(0.25f, 0.25f, 0.30f, 1.00f);
    colors[ImGuiCol_TableBorderLight]      = ImVec4(0.18f, 0.18f, 0.22f, 1.00f);
    colors[ImGuiCol_TextSelectedBg]       = ImVec4(0.18f, 0.40f, 0.70f, 0.40f);
    colors[ImGuiCol_DragDropTarget]       = ImVec4(1.00f, 0.80f, 0.20f, 0.90f);
    colors[ImGuiCol_NavHighlight]         = ImVec4(0.18f, 0.60f, 0.96f, 1.00f);
    modo_oscuro_ = true;
}


void Aplicacion::configurar_estilo_claro() {
    ImGuiStyle& style = ImGui::GetStyle();
    // Material Design 3 Styling
    style.WindowPadding = ImVec2(16.0f, 16.0f);
    style.FramePadding = ImVec2(12.0f, 8.0f);
    style.ItemSpacing = ImVec2(12.0f, 12.0f);
    style.ItemInnerSpacing = ImVec2(8.0f, 8.0f);
    
    // Rounded Corners (Pill shape for buttons/frames)
    style.WindowRounding = 12.0f;
    style.FrameRounding = 8.0f;
    style.PopupRounding = 8.0f;
    style.GrabRounding = 8.0f;
    style.ScrollbarRounding = 12.0f;
    style.TabRounding = 8.0f;

    // No borders
    style.WindowBorderSize = 0.0f;
    style.FrameBorderSize = 0.0f;
    style.PopupBorderSize = 0.0f;
    style.ChildBorderSize = 0.0f;
    style.TabBorderSize = 0.0f;

    ImVec4* colors = style.Colors;
    colors[ImGuiCol_Text]                 = ImVec4(0.13f, 0.13f, 0.14f, 1.00f); // #202124
    colors[ImGuiCol_TextDisabled]         = ImVec4(0.47f, 0.50f, 0.53f, 1.00f);
    colors[ImGuiCol_WindowBg]             = ImVec4(1.00f, 1.00f, 1.00f, 1.00f); // #ffffff
    colors[ImGuiCol_ChildBg]              = ImVec4(0.95f, 0.95f, 0.96f, 1.00f); // #f1f3f4
    colors[ImGuiCol_PopupBg]              = ImVec4(1.00f, 1.00f, 1.00f, 0.98f);
    colors[ImGuiCol_Border]               = ImVec4(0.85f, 0.85f, 0.87f, 0.50f);
    colors[ImGuiCol_FrameBg]              = ImVec4(0.95f, 0.95f, 0.96f, 1.00f); // Input fields
    colors[ImGuiCol_FrameBgHovered]       = ImVec4(0.90f, 0.90f, 0.92f, 1.00f);
    colors[ImGuiCol_FrameBgActive]        = ImVec4(0.85f, 0.85f, 0.87f, 1.00f);
    colors[ImGuiCol_TitleBg]              = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
    colors[ImGuiCol_TitleBgActive]        = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
    colors[ImGuiCol_TitleBgCollapsed]     = ImVec4(1.00f, 1.00f, 1.00f, 0.75f);
    colors[ImGuiCol_MenuBarBg]            = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
    colors[ImGuiCol_ScrollbarBg]          = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
    colors[ImGuiCol_ScrollbarGrab]        = ImVec4(0.80f, 0.80f, 0.82f, 1.00f);
    colors[ImGuiCol_ScrollbarGrabHovered] = ImVec4(0.70f, 0.70f, 0.72f, 1.00f);
    colors[ImGuiCol_ScrollbarGrabActive]  = ImVec4(0.60f, 0.60f, 0.62f, 1.00f);
    colors[ImGuiCol_CheckMark]            = ImVec4(0.10f, 0.45f, 0.91f, 1.00f); // #1a73e8
    colors[ImGuiCol_SliderGrab]           = ImVec4(0.10f, 0.45f, 0.91f, 1.00f);
    colors[ImGuiCol_SliderGrabActive]     = ImVec4(0.10f, 0.35f, 0.81f, 1.00f);
    colors[ImGuiCol_Button]               = ImVec4(0.95f, 0.95f, 0.96f, 1.00f);
    colors[ImGuiCol_ButtonHovered]        = ImVec4(0.90f, 0.90f, 0.92f, 1.00f);
    colors[ImGuiCol_ButtonActive]         = ImVec4(0.10f, 0.45f, 0.91f, 0.20f);
    colors[ImGuiCol_Header]               = ImVec4(0.95f, 0.95f, 0.96f, 1.00f);
    colors[ImGuiCol_HeaderHovered]        = ImVec4(0.90f, 0.90f, 0.92f, 1.00f);
    colors[ImGuiCol_HeaderActive]         = ImVec4(0.10f, 0.45f, 0.91f, 0.20f);
    colors[ImGuiCol_Separator]            = ImVec4(0.90f, 0.90f, 0.92f, 1.00f);
    colors[ImGuiCol_PlotLines]            = ImVec4(0.25f, 0.45f, 0.70f, 1.00f);
    colors[ImGuiCol_PlotHistogram]        = ImVec4(0.35f, 0.55f, 0.35f, 1.00f);
    colors[ImGuiCol_TextSelectedBg]       = ImVec4(0.25f, 0.40f, 0.60f, 0.40f);
    colors[ImGuiCol_DragDropTarget]       = ImVec4(0.80f, 0.60f, 0.10f, 0.70f);
    colors[ImGuiCol_Tab]                  = ImVec4(0.85f, 0.85f, 0.90f, 1.00f);
    colors[ImGuiCol_TabHovered]           = ImVec4(0.70f, 0.75f, 0.85f, 1.00f);
    colors[ImGuiCol_TabActive]            = ImVec4(0.95f, 0.95f, 0.97f, 1.00f);
    colors[ImGuiCol_TabUnfocused]         = ImVec4(0.85f, 0.85f, 0.90f, 1.00f);
    colors[ImGuiCol_TabUnfocusedActive]   = ImVec4(0.90f, 0.90f, 0.95f, 1.00f);
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
