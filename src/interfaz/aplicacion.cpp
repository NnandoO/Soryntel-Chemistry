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
void Aplicacion::scroll_cb(GLFWwindow* w, double dx, double dy) {}

// ═══════════════════════════════════════════════════════════════════════════
// Aplicacion
// ═══════════════════════════════════════════════════════════════════════════

Aplicacion::Aplicacion() {
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
    if (!iniciar_ventana()) return false;
    if (!iniciar_imgui()) return false;

    try {
        auto& gestor = base_datos::GestorDB::instancia();
        gestor.cargar_desde_csv("data/minteq.csv");
    } catch (const std::exception& e) {
        fprintf(stderr, "Error inicializando base de datos: %s\n", e.what());
        return false;
    }

    cargar_ejemplo();

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
    std::string font_path = "assets/fonts/OpenSans-Regular.ttf";
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

void Aplicacion::renderizar() {
    manejar_atajos_globales();
    menu_principal();

    ImVec2 tam_vp = ImGui::GetMainViewport()->Size;
    float menu_h = ImGui::GetFrameHeight();
    float stat_h = ImGui::GetFrameHeight();

    // Layout con splitters manuales (sin dependencia de ventana ImGui)
    const float izq_w = 46.0f;
    if (!layout_inicializado_ || tam_vp.x <= 0 || tam_vp.y <= 0) {
        layout_der_w_ = tam_vp.x * 0.32f;
        layout_inf_h_ = tam_vp.y * 0.28f;
        layout_esp_ratio_ = 0.55f;
        layout_inicializado_ = true;
    }
    float& der_w = layout_der_w_;
    float& inf_h = layout_inf_h_;
    float& esp_ratio = layout_esp_ratio_;

    float min_panel = 80.0f;
    float disp_h = tam_vp.y - menu_h - inf_h - stat_h;

    // ── Splitters manuales mediante mouse tracking directo ──
    auto& io = ImGui::GetIO();
    ImVec2 mp = io.MousePos;

    auto punto_en_rect = [](ImVec2 p, float x1, float y1, float x2, float y2) {
        return p.x >= x1 && p.x < x2 && p.y >= y1 && p.y < y2;
    };

    // ── Lienzo (Flowsheet Area) ──
    ImGui::SetNextWindowPos(ImVec2(izq_w, menu_h));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x - izq_w - der_w, disp_h));
    ImGui::Begin(tr("Lienzo"), nullptr, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoBringToFrontOnFocus);
    ImVec2 lienzo_tam = ImGui::GetContentRegionAvail();
    lienzo_.persistir_herramienta = persist_tool_;
    lienzo_.renderizar(lienzo_tam);
    ImGui::End();

    // ── Panel izquierdo: Barra de Herramientas CAD ──
    ImGui::SetNextWindowPos(ImVec2(0, menu_h));
    ImGui::SetNextWindowSize(ImVec2(izq_w, disp_h));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(4, 8));
    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(4, 6));
    ImGui::Begin(tr("CAD Tools"), nullptr, ImGuiWindowFlags_NoCollapse | ImGuiWindowFlags_NoTitleBar | ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoScrollbar);
    {
        auto render_tool_button = [&](const char* label_id, const char* tooltip, Lienzo::ModoDibujo target_mode, std::function<void(ImDrawList*, ImVec2)> draw_icon) {
            bool active = (lienzo_.modo_dibujo == target_mode);
            if (active) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImGui::GetStyle().Colors[ImGuiCol_ButtonActive]);
            } else {
                ImGui::PushStyleColor(ImGuiCol_Button, ImGui::GetStyle().Colors[ImGuiCol_Button]);
            }
            ImVec2 pos = ImGui::GetCursorScreenPos();
            if (ImGui::Button(label_id, ImVec2(38, 38))) {
                lienzo_.modo_dibujo = target_mode;
            }
            ImGui::PopStyleColor();
            
            ImDrawList* draw = ImGui::GetWindowDrawList();
            ImVec2 center(pos.x + 19.0f, pos.y + 19.0f);
            draw_icon(draw, center);

            if (ImGui::IsItemHovered()) {
                ImGui::SetTooltip("%s", tooltip);
            }
            ImGui::Spacing();
        };

        // Pointer Tool
        render_tool_button("##ptr", "Pointer Tool (Seleccionar y Mover)", Lienzo::ModoDibujo::Seleccion, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p0(center.x - 7, center.y - 10);
            ImVec2 p1(center.x - 7, center.y + 8);
            ImVec2 p2(center.x - 2, center.y + 3);
            ImVec2 p3(center.x + 3, center.y + 8);
            ImVec2 p4(center.x + 5, center.y + 6);
            ImVec2 p5(center.x, center.y + 1);
            ImVec2 p6(center.x + 5, center.y - 1);
            
            dl->AddTriangleFilled(p0, p1, p2, IM_COL32(30, 30, 35, 255));
            dl->AddTriangleFilled(p0, p2, p6, IM_COL32(30, 30, 35, 255));
            dl->AddLine(p2, p3, IM_COL32(30, 30, 35, 255), 2.5f);
            dl->AddTriangle(p0, p1, p6, IM_COL32(255, 255, 255, 240), 1.0f);
        });

        // Stream Tool
        render_tool_button("##stream", "Stream Tool (Conectar Nodos/Corrientes)", Lienzo::ModoDibujo::DibujarFlujo, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p_start(center.x - 10, center.y + 8);
            ImVec2 cp1(center.x - 4, center.y - 8);
            ImVec2 cp2(center.x + 2, center.y + 8);
            ImVec2 p_end(center.x + 10, center.y - 6);
            
            dl->AddBezierCubic(p_start, cp1, cp2, p_end, IM_COL32(0, 150, 255, 255), 2.5f);
            
            ImVec2 dir(1.0f, -0.6f);
            float len = sqrt(dir.x*dir.x + dir.y*dir.y);
            dir.x /= len; dir.y /= len;
            ImVec2 perp(-dir.y, dir.x);
            dl->AddTriangleFilled(p_end,
                ImVec2(p_end.x - dir.x * 6 + perp.x * 3.5f, p_end.y - dir.y * 6 + perp.y * 3.5f),
                ImVec2(p_end.x - dir.x * 6 - perp.x * 3.5f, p_end.y - dir.y * 6 - perp.y * 3.5f),
                IM_COL32(0, 150, 255, 255));
        });

        // Rectangle Tool
        render_tool_button("##rect", "Rectangle Tool (Dibujar Caja de Fondo)", Lienzo::ModoDibujo::Rectangulo, [](ImDrawList* dl, ImVec2 center) {
            ImVec2 p0(center.x - 10, center.y - 8);
            ImVec2 p1(center.x + 10, center.y + 8);
            dl->AddRectFilled(p0, p1, IM_COL32(80, 120, 220, 50), 2.0f);
            dl->AddRect(p0, p1, IM_COL32(80, 120, 220, 255), 2.0f, 0, 1.5f);
        });

        // Ellipse Tool
        render_tool_button("##ellipse", "Ellipse Tool (Dibujar Círculo/Elipse)", Lienzo::ModoDibujo::Elipse, [](ImDrawList* dl, ImVec2 center) {
            dl->AddEllipseFilled(center, ImVec2(11, 8), IM_COL32(180, 80, 220, 50));
            dl->AddEllipse(center, ImVec2(11, 8), IM_COL32(180, 80, 220, 255), 0, 0, 1.5f);
        });

        // Text Tool
        render_tool_button("##text", "Text Tool (Dibujar Caja de Texto)", Lienzo::ModoDibujo::Texto, [](ImDrawList* dl, ImVec2 center) {
            dl->AddLine(ImVec2(center.x - 9, center.y - 8), ImVec2(center.x + 9, center.y - 8), IM_COL32(230, 80, 50, 255), 2.5f);
            dl->AddLine(ImVec2(center.x, center.y - 8), ImVec2(center.x, center.y + 9), IM_COL32(230, 80, 50, 255), 2.5f);
            dl->AddLine(ImVec2(center.x - 4, center.y + 9), ImVec2(center.x + 4, center.y + 9), IM_COL32(230, 80, 50, 255), 1.5f);
        });
        
        ImGui::Separator();
        ImGui::Spacing();

        // Fit View
        ImVec2 pos_f = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##fit", ImVec2(38, 38))) {
            lienzo_.encajar_todo();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_f.x + 19.0f, pos_f.y + 19.0f);
            dl->AddCircle(center, 5.5f, IM_COL32(100, 180, 120, 255), 0, 1.5f);
            float d = 9.0f;
            dl->AddLine(ImVec2(center.x - d, center.y - d), ImVec2(center.x - 4, center.y - 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x + d, center.y - d), ImVec2(center.x + 4, center.y - 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x - d, center.y + d), ImVec2(center.x - 4, center.y + 4), IM_COL32(100, 180, 120, 255), 1.5f);
            dl->AddLine(ImVec2(center.x + d, center.y + d), ImVec2(center.x + 4, center.y + 4), IM_COL32(100, 180, 120, 255), 1.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Encajar todo (Fit Flowsheet)");

        ImGui::Spacing();

        // Zoom In
        ImVec2 pos_z_in = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##z_in", ImVec2(38, 38))) {
            lienzo_.zoom = std::min(lienzo_.zoom * 1.2f, 5.0f);
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_z_in.x + 19.0f, pos_z_in.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 8, center.y), ImVec2(center.x + 8, center.y), IM_COL32(80, 200, 120, 255), 2.5f);
            dl->AddLine(ImVec2(center.x, center.y - 8), ImVec2(center.x, center.y + 8), IM_COL32(80, 200, 120, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Acercar Zoom");

        ImGui::Spacing();

        // Zoom Out
        ImVec2 pos_z_out = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##z_out", ImVec2(38, 38))) {
            lienzo_.zoom = std::max(lienzo_.zoom / 1.2f, 0.1f);
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_z_out.x + 19.0f, pos_z_out.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 8, center.y), ImVec2(center.x + 8, center.y), IM_COL32(220, 80, 80, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Alejar Zoom");

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        // Undo Action
        ImVec2 pos_u = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##undo_btn", ImVec2(38, 38))) {
            deshacer_ultimo();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_u.x + 19.0f, pos_u.y + 19.0f);
            dl->PathArcTo(ImVec2(center.x + 2, center.y + 2), 6.0f, 3.14159f * 1.0f, 3.14159f * 1.8f);
            dl->PathStroke(IM_COL32(200, 180, 80, 255), false, 2.0f);
            dl->AddTriangleFilled(
                ImVec2(center.x - 4, center.y + 2),
                ImVec2(center.x - 1, center.y - 1),
                ImVec2(center.x - 1, center.y + 5),
                IM_COL32(200, 180, 80, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Deshacer última acción (Ctrl+Z)");

        ImGui::Spacing();

        // Delete Selected
        ImVec2 pos_d = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##del_btn", ImVec2(38, 38))) {
            if (lienzo_.id_nodo_seleccionado > 0) {
                flowsheet_->eliminar_nodo(lienzo_.id_nodo_seleccionado);
                lienzo_.id_nodo_seleccionado = -1;
            } else if (lienzo_.id_flujo_seleccionado > 0) {
                flowsheet_->eliminar_flujo(lienzo_.id_flujo_seleccionado);
                lienzo_.id_flujo_seleccionado = -1;
            } else if (lienzo_.id_anotacion_seleccionada > 0) {
                flowsheet_->eliminar_anotacion(lienzo_.id_anotacion_seleccionada);
                lienzo_.id_anotacion_seleccionada = -1;
            }
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_d.x + 19.0f, pos_d.y + 19.0f);
            dl->AddLine(ImVec2(center.x - 7, center.y - 7), ImVec2(center.x + 7, center.y + 7), IM_COL32(230, 70, 70, 255), 2.5f);
            dl->AddLine(ImVec2(center.x + 7, center.y - 7), ImVec2(center.x - 7, center.y + 7), IM_COL32(230, 70, 70, 255), 2.5f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Eliminar seleccionado (Delete)");

        ImGui::Spacing();

        // Clear Canvas
        ImVec2 pos_c = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##clear_btn", ImVec2(38, 38))) {
            ImGui::OpenPopup("Confirmar Nueva");
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 center(pos_c.x + 19.0f, pos_c.y + 19.0f);
            dl->AddRect(ImVec2(center.x - 6, center.y - 8), ImVec2(center.x + 6, center.y + 8), IM_COL32(200, 200, 220, 255), 1.5f);
            dl->AddLine(ImVec2(center.x - 3, center.y - 3), ImVec2(center.x + 3, center.y - 3), IM_COL32(200, 200, 220, 255), 1.0f);
            dl->AddLine(ImVec2(center.x - 3, center.y + 1), ImVec2(center.x + 3, center.y + 1), IM_COL32(200, 200, 220, 255), 1.0f);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Limpiar lienzo completo (Nuevo)");

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        // ── Controls ──
        ImGui::TextColored(ImVec4(0.5f, 0.85f, 0.6f, 1.0f), "RUN");
        ImGui::Spacing();

        // Run Simulation
        ImVec2 pos_run = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##run_btn", ImVec2(38, 38))) {
            ejecutar_simulacion();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 c(pos_run.x + 19.0f, pos_run.y + 19.0f);
            dl->AddTriangleFilled(ImVec2(c.x-6, c.y-8), ImVec2(c.x-6, c.y+8), ImVec2(c.x+8, c.y), IM_COL32(80, 220, 100, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Ejecutar Simulación (Run)"));

        ImGui::Spacing();

        // Reset Simulation
        ImVec2 pos_rst = ImGui::GetCursorScreenPos();
        if (ImGui::Button("##rst_btn", ImVec2(38, 38))) {
            reiniciar_simulacion();
        }
        {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            ImVec2 c(pos_rst.x + 19.0f, pos_rst.y + 19.0f);
            dl->AddRectFilled(ImVec2(c.x-6, c.y-6), ImVec2(c.x+6, c.y+6), IM_COL32(220, 80, 80, 255));
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Reiniciar Simulación (Reset)"));
    }
    ImGui::End();
    ImGui::PopStyleVar(2);

    // ── Panel inferior ──
    float inf_y = tam_vp.y - inf_h - stat_h;
    ImGui::SetNextWindowPos(ImVec2(0, inf_y));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x, inf_h));
    ImGui::Begin(tr("Resultados"), nullptr, ImGuiWindowFlags_NoCollapse);
    panel_resultados_.renderizar();
    ImGui::End();

    // Splitter inferior (resultados / lienzo)
    {   bool& drag_b = layout_drag_b_;
        float r_x1=0, r_y1=inf_y, r_x2=tam_vp.x, r_y2=inf_y+8;
        bool hover_b = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_b && io.MouseClicked[0]) drag_b = true;
        if (drag_b) { inf_h -= io.MouseDelta.y; if (!io.MouseDown[0]) drag_b = false; }
        inf_h = std::max(60.0f, std::min(inf_h, tam_vp.y * 0.6f));
        if (hover_b || drag_b) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeNS);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_b ? IM_COL32(26, 115, 232, 200) : hover_b ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(0,inf_y), ImVec2(tam_vp.x,inf_y+8), col);
        if (hover_b || drag_b) {
            dl->AddRectFilled(ImVec2(tam_vp.x/2 - 20, inf_y+3), ImVec2(tam_vp.x/2 + 20, inf_y+5), IM_COL32(255, 255, 255, 180));
        }
    }

    disp_h = tam_vp.y - menu_h - inf_h - stat_h;
    float esp_h = disp_h * esp_ratio;

    // ── Panel derecho superior: Propiedades ──
    ImGui::SetNextWindowPos(ImVec2(tam_vp.x - der_w, menu_h));
    ImGui::SetNextWindowSize(ImVec2(der_w, esp_h));
    ImGui::Begin(tr("Propiedades"), nullptr, ImGuiWindowFlags_NoCollapse);
    panel_propiedades_.renderizar();
    ImGui::End();

    // Splitter derecho (propiedades / caja de herramientas)
    {   bool& drag_r = layout_drag_r_;
        float r_x1=tam_vp.x-der_w, r_y1=menu_h+esp_h-4, r_x2=tam_vp.x, r_y2=menu_h+esp_h+4;
        bool hover_r = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_r && io.MouseClicked[0]) drag_r = true;
        if (drag_r) { esp_ratio += io.MouseDelta.y / disp_h; if (!io.MouseDown[0]) drag_r = false; }
        esp_ratio = std::max(0.15f, std::min(esp_ratio, 0.85f));
        esp_h = disp_h * esp_ratio;
        if (hover_r || drag_r) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeNS);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_r ? IM_COL32(26, 115, 232, 200) : hover_r ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(tam_vp.x-der_w,menu_h+esp_h-4), ImVec2(tam_vp.x,menu_h+esp_h+4), col);
        if (hover_r || drag_r) {
            dl->AddRectFilled(ImVec2(tam_vp.x-der_w/2 - 20, menu_h+esp_h-1), ImVec2(tam_vp.x-der_w/2 + 20, menu_h+esp_h+1), IM_COL32(255, 255, 255, 180));
        }
    }

    // ── Panel derecho inferior: Caja de Herramientas y Paleta de Equipos (Unit Icons) ──
    float prop_y = menu_h + esp_h;
    ImGui::SetNextWindowPos(ImVec2(tam_vp.x - der_w, prop_y));
    ImGui::SetNextWindowSize(ImVec2(der_w, disp_h - esp_h));
    ImGui::Begin(tr("Caja de Herramientas"), nullptr, ImGuiWindowFlags_NoCollapse);
    {
        if (ImGui::BeginTabBar("HSCRightBottomTabs")) {
            if (ImGui::BeginTabItem(tr("Unit Icons"))) {
                ImGui::BeginChild("UnitIconsScroll", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);
                
                std::vector<nucleo::TipoNodo> types = {
                    nucleo::TipoNodo::Fuente,
                    nucleo::TipoNodo::Sumidero,
                    nucleo::TipoNodo::Mezclador,
                    nucleo::TipoNodo::Divisor,
                    nucleo::TipoNodo::Reactor,
                    nucleo::TipoNodo::Espesador,
                    nucleo::TipoNodo::Filtro,
                    nucleo::TipoNodo::CeldaFlotacion,
                    nucleo::TipoNodo::MolinoSAG,
                    nucleo::TipoNodo::MolinoBolas,
                    nucleo::TipoNodo::Hidrociclon,
                    nucleo::TipoNodo::TrituradoraGiratoria,
                    nucleo::TipoNodo::Zaranda,
                    nucleo::TipoNodo::IntercambiadorCalor,
                    nucleo::TipoNodo::ColumnaDestilacion,
                    nucleo::TipoNodo::Compresor,
                    nucleo::TipoNodo::Válvula,
                    nucleo::TipoNodo::Separador
                };

                float col_width = 75.0f;
                int cols = std::max(1, (int)(ImGui::GetContentRegionAvail().x / col_width));
                if (ImGui::BeginTable("unit_icons_grid", cols)) {
                    for (size_t i = 0; i < types.size(); ++i) {
                        ImGui::TableNextColumn();
                        
                        nucleo::TipoNodo t = types[i];
                        ImGui::PushID((int)t);
                        
                        ImVec2 p = ImGui::GetCursorScreenPos();
                        ImVec2 btn_size(60, 60);

                        auto* draw_list = ImGui::GetWindowDrawList();
                        bool is_hover = ImGui::IsMouseHoveringRect(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y));
                        ImU32 col_bg = is_hover ? IM_COL32(180, 200, 240, 60) : IM_COL32(230, 230, 235, 180);
                        draw_list->AddRectFilled(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y), col_bg, 4.0f);
                        draw_list->AddRect(p, ImVec2(p.x + btn_size.x, p.y + btn_size.y), IM_COL32(180, 180, 190, 200), 4.0f);

                        // Dibujar vista previa del equipo en miniatura vectorial
                        Lienzo::dibujar_modelo(draw_list, ImVec2(p.x + 30, p.y + 30), 30.0f, t);

                        if (ImGui::InvisibleButton("##btn", btn_size)) {
                            lienzo_.guardar_estado();
                            auto id = flowsheet_->agregar_nodo(t);
                            auto* n = flowsheet_->nodo_por_id(id);
                            if (n) {
                                n->nombre = std::string(nucleo::nombre_tipo_nodo(t)) + " #" + std::to_string(id);
                                n->canvas_x = 50.0f + (contador_nodos_ % 5) * 160.0f;
                                n->canvas_y = 50.0f + (contador_nodos_ / 5) * 90.0f;
                                n->pagina = lienzo_.pagina_actual;
                                contador_nodos_++;
                            }
                            panel_propiedades_.seleccionar_nodo(id);
                            lienzo_.id_nodo_seleccionado = id;
                            lienzo_.id_flujo_seleccionado = -1;
                            lienzo_.id_anotacion_seleccionada = -1;
                            hay_cambios_sin_guardar_ = true;
                        }

                        if (ImGui::BeginDragDropSource(ImGuiDragDropFlags_SourceNoDisableHover)) {
                            ImGui::SetDragDropPayload("tipo_nodo", &t, sizeof(t));
                            ImGui::Text(tr("%s"), nucleo::nombre_tipo_nodo(t));
                            ImGui::EndDragDropSource();
                        }

                        ImGui::TextWrapped(tr("%s"), nucleo::nombre_tipo_nodo(t));
                        
                        ImGui::PopID();
                    }
                    ImGui::EndTable();
                }
                ImGui::EndChild();
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Especie"))) {
                panel_especies_.renderizar();
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Árbol de proceso"))) {
                ImGui::Text(tr("Elementos del Proceso:"));
                ImGui::Separator();
                if (ImGui::TreeNode(tr("Unidades (Nodos)"))) {
                    for (auto& n : flowsheet_->nodos()) {
                        bool sel = (lienzo_.id_nodo_seleccionado == n.id);
                        if (ImGui::Selectable((n.nombre + "##node").c_str(), sel)) {
                            panel_propiedades_.seleccionar_nodo(n.id);
                            lienzo_.id_nodo_seleccionado = n.id;
                            lienzo_.id_flujo_seleccionado = -1;
                        }
                    }
                    ImGui::TreePop();
                }
                if (ImGui::TreeNode(tr("Corrientes (Flujos)"))) {
                    for (auto& [fid, f] : flowsheet_->mapa_flujos()) {
                        bool sel = (lienzo_.id_flujo_seleccionado == fid);
                        std::string name = f.nombre.empty() ? ("Flujo " + std::to_string(fid)) : f.nombre;
                        if (ImGui::Selectable((name + "##stream").c_str(), sel)) {
                            panel_propiedades_.seleccionar_flujo(fid);
                            lienzo_.id_flujo_seleccionado = fid;
                            lienzo_.id_nodo_seleccionado = -1;
                        }
                    }
                    ImGui::TreePop();
                }
                ImGui::EndTabItem();
            }

            if (ImGui::BeginTabItem(tr("Log Viewer"))) {
                ImGui::Text(tr("Registro de Simulación:"));
                ImGui::Separator();
                ImGui::TextDisabled(tr("Simulación lista."));
                ImGui::TextDisabled(tr("Haga clic en 'Play' en la barra superior."));
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
    }
    ImGui::End();

    // Splitter derecho vertical (derecha / lienzo)
    {   bool& drag_rv = layout_drag_rv_;
        float r_x1=tam_vp.x-der_w-8, r_y1=menu_h, r_x2=tam_vp.x-der_w, r_y2=menu_h+disp_h;
        bool hover_rv = punto_en_rect(mp, r_x1, r_y1, r_x2, r_y2);
        if (hover_rv && io.MouseClicked[0]) drag_rv = true;
        if (drag_rv) { der_w -= io.MouseDelta.x; if (!io.MouseDown[0]) drag_rv = false; }
        der_w = std::max(120.0f, std::min(der_w, tam_vp.x - izq_w - min_panel));
        if (hover_rv || drag_rv) ImGui::SetMouseCursor(ImGuiMouseCursor_ResizeEW);
        auto* dl = ImGui::GetForegroundDrawList();
        ImU32 col = drag_rv ? IM_COL32(26, 115, 232, 200) : hover_rv ? IM_COL32(150, 150, 150, 150) : IM_COL32(0, 0, 0, 0);
        dl->AddRectFilled(ImVec2(tam_vp.x-der_w-8,menu_h), ImVec2(tam_vp.x-der_w,menu_h+disp_h), col);
        if (hover_rv || drag_rv) {
            dl->AddRectFilled(ImVec2(tam_vp.x-der_w-5, menu_h+disp_h/2 - 20), ImVec2(tam_vp.x-der_w-3, menu_h+disp_h/2 + 20), IM_COL32(255, 255, 255, 180));
        }
    }

    // ── Lienzo central ──
    // ── Barra de estado ──
    ImGui::SetNextWindowPos(ImVec2(0, tam_vp.y - stat_h));
    ImGui::SetNextWindowSize(ImVec2(tam_vp.x, stat_h));
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(6, 2));
    ImGui::Begin("###statusbar", nullptr,
        ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse);
    {
        // Pestañas de página al estilo Excel/HSC Sim
        for (int p = 0; p < 3; p++) {
            char tab_label[16];
            snprintf(tab_label, sizeof(tab_label), "Page %d", p + 1);
            bool is_active = (lienzo_.pagina_actual == p);
            if (is_active) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.28f, 0.45f, 1.0f));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
            } else {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.12f, 0.12f, 0.15f, 0.5f));
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.6f, 0.6f, 0.7f, 1.0f));
            }
            if (ImGui::Button(tab_label, ImVec2(65, 20))) {
                lienzo_.pagina_actual = p;
            }
            ImGui::PopStyleColor(2);
            ImGui::SameLine();
        }

        ImGui::TextColored(ImVec4(0.4f, 0.4f, 0.5f, 1), "|  © 2026 Outotec Oyj | °C, bar, kWh");
        
        ImGui::SameLine();
        if (hay_cambios_sin_guardar_) {
            ImGui::TextColored(ImVec4(1.0f, 0.8f, 0.2f, 1), " ●");
            if (ImGui::IsItemHovered()) ImGui::SetTooltip("Cambios sin guardar");
        }

        ImGui::SameLine();
        ImGui::SetCursorPosX(420.0f);
        ImGui::TextColored(ImVec4(0.4f,0.7f,0.4f,1), "| Nodos: %zu  |  Flujos: %zu", flowsheet_->conteo_nodos(), flowsheet_->conteo_flujos());

        ImGui::SameLine();
        if (sim_resuelto_) {
            ImGui::TextColored(ImVec4(0.3f, 0.9f, 0.3f, 1), "  | Sim: %s", sim_ultimo_resultado_.convergio ? "OK" : "Error");
            if (ImGui::IsItemHovered()) ImGui::SetTooltip("%s", sim_ultimo_resultado_.mensaje.c_str());
        }

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 520);
        ImGui::TextColored(ImVec4(0.8f,0.8f,0.8f,1), "Mouse: %d, %d", (int)lienzo_.coord_raton.x, (int)lienzo_.coord_raton.y);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 390);
        ImGui::Checkbox(tr("Persist Tool"), &persist_tool_);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 280);
        ImGui::Checkbox(tr("Ajustar a grilla"), &lienzo_.ajustar_grilla);

        ImGui::SameLine();
        ImGui::SetCursorPosX(tam_vp.x - 140);
        ImGui::SetNextItemWidth(70);
        float z_pct = lienzo_.zoom * 100.0f;
        if (ImGui::SliderFloat("##zoom_slider", &z_pct, 10.0f, 200.0f, "%.0f%%")) {
            lienzo_.zoom = z_pct / 100.0f;
        }
    }
    ImGui::End();
    ImGui::PopStyleVar();

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
        ImGui::Begin(tr("Tablas de Vapor (Agua)"), &mostrar_agua_);
        panel_agua_.renderizar();
        ImGui::End();
    }

    if (mostrar_soluciones_) {
        ImGui::SetNextWindowSizeConstraints(ImVec2(300,200), ImVec2(3000,3000));
        ImGui::SetNextWindowSize(ImVec2(500,400), ImGuiCond_FirstUseEver);
        ImGui::Begin(tr("Soluciones Acuosas"), &mostrar_soluciones_);
        panel_soluciones_.renderizar();
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
            auto resultado = fs->resolver_convergencia(500, 1e-6);
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
        if (ImGui::MenuItem(tr("Eliminar"), "Supr"))
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

void Aplicacion::cargar_ejemplo(int ejemplo) {
    nueva_simulacion();
    auto& gestor = base_datos::GestorDB::instancia();

    switch (ejemplo) {
        case 1: cargar_ejemplo_1(); break;
        case 2: cargar_ejemplo_2(); break;
        case 3: cargar_ejemplo_3(); break;
        case 4: cargar_ejemplo_4(); break;
        default: cargar_ejemplo_1(); break;
    }
}

void Aplicacion::cargar_ejemplo_1() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 1: Reformado de Metano (SMR) ──
    // CH₄ + H₂O → CO + 3H₂  (Reformador)
    // CO + H₂O → CO₂ + H₂   (WGS)
    // Mezcla final → gas de síntesis

    auto id_fuente = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "CH₄ + Vapor");
    auto* fuente = flowsheet_->nodo_por_id(id_fuente);
    if (fuente) {
        fuente->canvas_x = 50; fuente->canvas_y = 50;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        auto* h2o = gestor.especie_por_nombre("Agua");
        if (ch4) fuente->params["flujo_" + std::to_string(ch4->id)] = 1.0;
        if (h2o) fuente->params["flujo_" + std::to_string(h2o->id)] = 2.0;
        fuente->params["T"] = 500.0;
        fuente->params["P"] = 20.0;
    }

    auto id_reactor = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Reformador SMR");
    auto* reactor = flowsheet_->nodo_por_id(id_reactor);
    if (reactor) {
        reactor->canvas_x = 280; reactor->canvas_y = 50;
        reactor->params["conversion"] = 0.75;
        reactor->params["temperatura"] = 1073.0;
        reactor->params["presion"] = 20.0;
        reactor->params["reaccion_id"] = 1.0;
    }

    auto id_fuente2 = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Aire (O₂)");
    auto* fuente2 = flowsheet_->nodo_por_id(id_fuente2);
    if (fuente2) {
        fuente2->canvas_x = 50; fuente2->canvas_y = 250;
        auto* o2 = gestor.especie_por_nombre("Oxígeno");
        if (o2) fuente2->params["flujo_" + std::to_string(o2->id)] = 0.5;
    }

    auto id_wgs = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "WGS");
    auto* wgs = flowsheet_->nodo_por_id(id_wgs);
    if (wgs) {
        wgs->canvas_x = 500; wgs->canvas_y = 50;
        wgs->params["conversion"] = 0.90;
        wgs->params["temperatura"] = 623.0;
        wgs->params["presion"] = 20.0;
        wgs->params["reaccion_id"] = 2.0;
    }

    auto id_mezcla = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezclador");
    auto* mezcla = flowsheet_->nodo_por_id(id_mezcla);
    if (mezcla) {
        mezcla->canvas_x = 700; mezcla->canvas_y = 150;
    }

    auto id_sum = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas Síntesis");
    auto* sum = flowsheet_->nodo_por_id(id_sum);
    if (sum) {
        sum->canvas_x = 880; sum->canvas_y = 150;
    }

    auto id_calor = flowsheet_->agregar_flujo_energia("Calor SMR");
    auto* flujo_calor = flowsheet_->flujo_por_id(id_calor);
    if (flujo_calor) flujo_calor->energia = 500.0;

    flowsheet_->conectar(id_fuente, id_reactor);
    flowsheet_->conectar(id_reactor, id_wgs);
    flowsheet_->conectar(id_wgs, id_mezcla);
    flowsheet_->conectar(id_fuente2, id_mezcla);
    flowsheet_->conectar(id_mezcla, id_sum);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(0, 0);
    panel_propiedades_.seleccionar_nodo(id_fuente);
    panel_resultados_.poner_resultado_masa("Ejemplo 1: Reformado de Metano (SMR) cargado. Presiona F5 o Módulos→SteadyStateEngine", true);
}

void Aplicacion::cargar_ejemplo_4() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 4: SMR con lazo de recirculación de CH₄ ──
    // Fuente (CH₄+H₂O) → Mezclador → Reactor SMR → Separador → Sumidero
    //                                       │  (CH₄ sin convertir se recicla)
    //                                       └──▶ Mezclador
    // El separador retira todo el CH₄ (remover_ = 1.0) y lo devuelve al reactor.
    // Solución esperada (converge con Ctrl+R / F5):
    //   Alimentación al reactor: CH₄ = 1.3333, H₂O = 2.0  (3.3333 kmol/s)
    //   Reciclo: CH₄ = 0.3333 kmol/s   Producto: H₂ = 3.0, CO = 1.0, H₂O = 1.0 (5.0 kmol/s)

    auto id_fuente = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "CH₄ + Vapor");
    auto* fuente = flowsheet_->nodo_por_id(id_fuente);
    if (fuente) {
        fuente->canvas_x = 50; fuente->canvas_y = 150;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        auto* h2o = gestor.especie_por_nombre("Agua");
        if (ch4) fuente->params["flujo_" + std::to_string(ch4->id)] = 1.0;
        if (h2o) fuente->params["flujo_" + std::to_string(h2o->id)] = 2.0;
        fuente->params["temperatura"] = 500.0;
        fuente->params["presion"] = 20.0;
    }

    auto id_mezclador = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezclador");
    auto* mezclador = flowsheet_->nodo_por_id(id_mezclador);
    if (mezclador) mezclador->canvas_x = 240; mezclador->canvas_y = 150;

    auto id_reactor = flowsheet_->agregar_nodo(nucleo::TipoNodo::Reactor, "Reformador SMR");
    auto* reactor = flowsheet_->nodo_por_id(id_reactor);
    if (reactor) {
        reactor->canvas_x = 430; reactor->canvas_y = 150;
        reactor->params["conversion"] = 0.75;
        reactor->params["temperatura"] = 1073.0;
        reactor->params["presion"] = 20.0;
        reactor->params["reaccion_id"] = 1.0;
    }

    auto id_sep = flowsheet_->agregar_nodo(nucleo::TipoNodo::Separador, "Separador CH₄");
    auto* sep = flowsheet_->nodo_por_id(id_sep);
    if (sep) {
        sep->canvas_x = 640; sep->canvas_y = 150;
        auto* ch4 = gestor.especie_por_nombre("Metano");
        if (ch4) sep->params["remover_" + std::to_string(ch4->id)] = 1.0;
    }

    auto id_sum = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas de Síntesis");
    auto* sum = flowsheet_->nodo_por_id(id_sum);
    if (sum) sum->canvas_x = 850; sum->canvas_y = 150;

    flowsheet_->conectar(id_fuente, id_mezclador);
    flowsheet_->conectar(id_mezclador, id_reactor);
    flowsheet_->conectar(id_reactor, id_sep);
    flowsheet_->conectar(id_sep, id_sum);       // salida 0 = producto (sin CH₄)
    flowsheet_->conectar(id_sep, id_mezclador); // salida 1 = reciclo de CH₄

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-30, -50);
    panel_propiedades_.seleccionar_nodo(id_fuente);
    panel_resultados_.poner_resultado_masa(
        "Ejemplo 4: SMR con reciclo de CH₄ cargado. Presiona Ctrl+R (Resolver Recirculaciones) o F5.", true);
}

void Aplicacion::cargar_ejemplo_2() {
    auto& gestor = base_datos::GestorDB::instancia();
    // ── Ejemplo 2: Flotación de Cobre ──
    // Alimentación (mineral con CuFeS₂, FeS₂, SiO₂) → Molienda → Flotación → Concentrado + Relave

    auto id_feed = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Cu");
    auto* feed = flowsheet_->nodo_por_id(id_feed);
    if (feed) {
        feed->canvas_x = 50; feed->canvas_y = 150;
        feed->params["ley_Cu"] = 1.2;
        feed->params["ley_Fe"] = 25.0;
        feed->params["ley_S"] = 28.0;
        feed->params["ley_SiO2"] = 45.0;
        auto* pyrite = gestor.especie_por_nombre("Pirita");
        auto* cuarzo = gestor.especie_por_nombre("Cuarzo");
        if (pyrite) feed->params["flujo_" + std::to_string(pyrite->id)] = 60.0;
        if (cuarzo) feed->params["flujo_" + std::to_string(cuarzo->id)] = 40.0;
    }

    auto id_molienda = flowsheet_->agregar_nodo(nucleo::TipoNodo::MolinoBolas, "Molienda");
    auto* molienda = flowsheet_->nodo_por_id(id_molienda);
    if (molienda) {
        molienda->canvas_x = 250; molienda->canvas_y = 150;
        molienda->params["potencia"] = 1500.0;
        molienda->params["P80"] = 75.0;
    }

    auto id_flotacion = flowsheet_->agregar_nodo(nucleo::TipoNodo::CeldaFlotacion, "Flotación Rougher");
    auto* flotacion = flowsheet_->nodo_por_id(id_flotacion);
    if (flotacion) {
        flotacion->canvas_x = 450; flotacion->canvas_y = 150;
        flotacion->params["recuperacion"] = 0.90;
        flotacion->params["ley_concentrado"] = 28.0;
    }

    auto id_conc = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Concentrado Cu");
    auto* conc = flowsheet_->nodo_por_id(id_conc);
    if (conc) conc->canvas_x = 650; conc->canvas_y = 80;

    auto id_relave = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Relave");
    auto* relave = flowsheet_->nodo_por_id(id_relave);
    if (relave) relave->canvas_x = 650; relave->canvas_y = 280;

    flowsheet_->conectar(id_feed, id_molienda);
    flowsheet_->conectar(id_molienda, id_flotacion);
    flowsheet_->conectar(id_flotacion, id_conc);
    flowsheet_->conectar(id_flotacion, id_relave);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-50, -50);
    panel_resultados_.poner_resultado_masa("Ejemplo 2: Flotación de Cobre cargado. Usa Módulos→SpeciesConverter para mineralogía", true);
}

void Aplicacion::cargar_ejemplo_3() {
    auto& gestor = base_datos::GestorDB::instancia();

    // ── Ejemplo 3: Alto Horno de Hierro (simplificado) ──
    // Mineral + Coque → Reducción → Arrabio + Escoria + Gas

    auto id_mineral = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Mineral Fe₂O₃");
    auto* mineral = flowsheet_->nodo_por_id(id_mineral);
    if (mineral) {
        mineral->canvas_x = 50; mineral->canvas_y = 50;
        auto* hem = gestor.especie_por_nombre("Hematita");
        auto* qz = gestor.especie_por_nombre("Cuarzo");
        if (hem) mineral->params["flujo_" + std::to_string(hem->id)] = 80.0;
        if (qz) mineral->params["flujo_" + std::to_string(qz->id)] = 15.0;
        mineral->params["T"] = 298.0;
    }

    auto id_coque = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Coque (C)");
    auto* coque = flowsheet_->nodo_por_id(id_coque);
    if (coque) {
        coque->canvas_x = 50; coque->canvas_y = 250;
        auto* coq = gestor.especie_por_nombre("Negro de carbón");
        if (coq) coque->params["flujo_" + std::to_string(coq->id)] = 40.0;
        coque->params["T"] = 298.0;
    }

    auto id_aire = flowsheet_->agregar_nodo(nucleo::TipoNodo::Fuente, "Aire caliente");
    auto* aire = flowsheet_->nodo_por_id(id_aire);
    if (aire) {
        aire->canvas_x = 50; aire->canvas_y = 400;
        auto* o2 = gestor.especie_por_nombre("Oxígeno");
        auto* n2 = gestor.especie_por_nombre("Nitrógeno");
        if (o2) aire->params["flujo_" + std::to_string(o2->id)] = 50.0;
        if (n2) aire->params["flujo_" + std::to_string(n2->id)] = 188.0;
        aire->params["T"] = 1200.0;
    }

    auto id_mezcla = flowsheet_->agregar_nodo(nucleo::TipoNodo::Mezclador, "Mezcla carga");
    auto* mezcla = flowsheet_->nodo_por_id(id_mezcla);
    if (mezcla) {
        mezcla->canvas_x = 280; mezcla->canvas_y = 150;
    }

    auto id_horno = flowsheet_->agregar_nodo(nucleo::TipoNodo::HornoFlash, "Alto Horno");
    auto* horno = flowsheet_->nodo_por_id(id_horno);
    if (horno) {
        horno->canvas_x = 480; horno->canvas_y = 150;
        horno->params["temperatura"] = 1873.0;
        horno->params["presion"] = 3.0;
        horno->params["conversion"] = 0.92;
    }

    auto id_arrabio = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Arrabio");
    auto* arrabio = flowsheet_->nodo_por_id(id_arrabio);
    if (arrabio) arrabio->canvas_x = 700; arrabio->canvas_y = 50;

    auto id_escoria = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Escoria");
    auto* escoria = flowsheet_->nodo_por_id(id_escoria);
    if (escoria) escoria->canvas_x = 700; escoria->canvas_y = 180;

    auto id_gas = flowsheet_->agregar_nodo(nucleo::TipoNodo::Sumidero, "Gas salida");
    auto* gas = flowsheet_->nodo_por_id(id_gas);
    if (gas) gas->canvas_x = 700; gas->canvas_y = 310;

    flowsheet_->conectar(id_mineral, id_mezcla);
    flowsheet_->conectar(id_coque, id_mezcla);
    flowsheet_->conectar(id_mezcla, id_horno);
    flowsheet_->conectar(id_aire, id_horno);
    flowsheet_->conectar(id_horno, id_arrabio);
    flowsheet_->conectar(id_horno, id_escoria);
    flowsheet_->conectar(id_horno, id_gas);

    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.desplazamiento = ImVec2(-80, -30);
    panel_resultados_.poner_resultado_masa("Ejemplo 3: Alto Horno cargado. Presiona F5 para simular balance de masa", true);
}

void Aplicacion::nueva_simulacion() {
    flowsheet_ = std::make_unique<diagrama_flujo::Flowsheet>();
    lienzo_.establecer_flowsheet(flowsheet_.get());
    panel_herramientas_.establecer_flowsheet(flowsheet_.get());
    panel_resultados_.limpiar();
    panel_propiedades_ = PanelPropiedades();
    panel_propiedades_.establecer_flowsheet(flowsheet_.get());
    panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
    panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
    panel_geo_.establecer_flowsheet(flowsheet_.get());
    panel_map_gps_.establecer_flowsheet(flowsheet_.get());
    panel_fit_.establecer_flowsheet(flowsheet_.get());
    panel_stat_.establecer_flowsheet(flowsheet_.get());
    panel_lca_.reset();
    panel_reconciliacion_.reset();
    lienzo_.id_nodo_seleccionado = -1;
    lienzo_.id_flujo_seleccionado = -1;
    lienzo_.id_anotacion_seleccionada = -1;
    lienzo_.zoom = 1.0f;
    lienzo_.desplazamiento = ImVec2(0.0f, 0.0f);
    lienzo_.pagina_actual = 0;
    lienzo_.limpiar_historial();
    pila_undo_acciones_.clear();
    ruta_archivo_actual_.clear();
    hay_cambios_sin_guardar_ = false;
    panel_resultados_.poner_resultado_masa("Nuevo lienzo creado.", true);
}

void Aplicacion::guardar_proyecto() {
    std::string ruta = ruta_archivo_actual_;
    if (ruta.empty()) {
        guardar_proyecto_como();
        return;
    }
    std::string json = flowsheet_->a_json();
    std::ofstream f(ruta, std::ios::binary);
    if (f.is_open()) {
        f.write(json.c_str(), (std::streamsize)json.size());
        f.close();
        if (f.good()) {
            ruta_archivo_actual_ = ruta;
            hay_cambios_sin_guardar_ = false;
            panel_resultados_.poner_resultado_masa("Proyecto guardado: " + ruta, true);
        } else {
            panel_resultados_.poner_resultado_masa("Error al escribir: " + ruta, false);
        }
    } else {
        panel_resultados_.poner_resultado_masa("Error al guardar: " + ruta, false);
    }
}

void Aplicacion::guardar_proyecto_como() {
    std::string cmd = "zenity --file-selection --save --confirm-overwrite --title=\"Guardar Proyecto CMA Como\" --file-filter=\"Proyectos CMA | *.cma\" 2>/dev/null";
    hsc_clone::ExternalAPI api;
    std::string filepath = api.ejecutarComando(cmd);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    
    if (!filepath.empty()) {
        if (filepath.size() < 4 || filepath.substr(filepath.size() - 4) != ".cma") {
            filepath += ".cma";
        }
        std::string json = flowsheet_->a_json();
        std::ofstream f(filepath, std::ios::binary);
        if (f.is_open()) {
            f.write(json.c_str(), (std::streamsize)json.size());
            f.close();
            if (f.good()) {
                ruta_archivo_actual_ = filepath;
                hay_cambios_sin_guardar_ = false;
                panel_resultados_.poner_resultado_masa("Proyecto guardado: " + filepath, true);
            } else {
                panel_resultados_.poner_resultado_masa("Error al escribir: " + filepath, false);
            }
        } else {
            panel_resultados_.poner_resultado_masa("Error al guardar: " + filepath, false);
        }
    }
}

void Aplicacion::cargar_proyecto() {
    std::string cmd = "zenity --file-selection --title=\"Abrir Proyecto CMA\" --file-filter=\"Proyectos CMA | *.cma\" 2>/dev/null";
    hsc_clone::ExternalAPI api;
    std::string filepath = api.ejecutarComando(cmd);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    
    if (!filepath.empty()) {
        std::ifstream fp(filepath, std::ios::binary);
        if (fp.is_open()) {
            std::string json_content((std::istreambuf_iterator<char>(fp)),
                                     std::istreambuf_iterator<char>());
            fp.close();
            
            std::string err;
            if (flowsheet_->desde_json(json_content, err)) {
                ruta_archivo_actual_ = filepath;
                hay_cambios_sin_guardar_ = false;
                panel_resultados_.poner_resultado_masa("Proyecto cargado: " + filepath, true);
                
                lienzo_.id_nodo_seleccionado = -1;
                lienzo_.id_flujo_seleccionado = -1;
                lienzo_.id_anotacion_seleccionada = -1;
                panel_propiedades_ = PanelPropiedades();
                panel_propiedades_.establecer_flowsheet(flowsheet_.get());
                panel_propiedades_.al_cambio_flowsheet = [this]() { hay_cambios_sin_guardar_ = true; };
                panel_propiedades_.al_antes_de_modificar = [this]() { lienzo_.guardar_estado(); };
                panel_geo_.establecer_flowsheet(flowsheet_.get());
                panel_map_gps_.establecer_flowsheet(flowsheet_.get());
                panel_fit_.establecer_flowsheet(flowsheet_.get());
                panel_stat_.establecer_flowsheet(flowsheet_.get());
                panel_lca_.reset();
                lienzo_.encajar_todo();
                lienzo_.limpiar_historial();
                pila_undo_acciones_.clear();
            } else {
                panel_resultados_.poner_resultado_masa("Error al cargar proyecto: " + err, false);
            }
        } else {
            panel_resultados_.poner_resultado_masa("Error al abrir archivo.", false);
        }
    }
}

void Aplicacion::reiniciar_simulacion() {
    panel_resultados_.limpiar();
    
    // Clear computed flows and properties from streams
    for (auto& st : flowsheet_->flujos()) {
        st.flujo_molar.clear();
        st.flujo_solido.clear();
        st.flujo_liquido.clear();
        st.flujo_gas.clear();
        st.fraccion_solido = 0.0;
        st.fraccion_liquido = 0.0;
        st.fraccion_gas = 0.0;
        st.entalpia = 0.0;
        st.energia = 0.0;
    }
    
    // Clear calculated node params
    for (auto& n : flowsheet_->nodos()) {
        n.params.erase("Q_calculated");
        n.params.erase("W_calculated");
        n.params.erase("T_out_calculated");
        n.params.erase("P_out_calculated");
        n.params.erase("Ex_loss");
    }
}

void Aplicacion::ejecutar_convergencia() {
    auto res = flowsheet_->resolver_convergencia(500, 1e-6, nullptr);
    panel_resultados_.poner_resultado_masa(res.mensaje, res.convergio);
    if (res.convergio) {
        char buf[128];
        snprintf(buf, sizeof(buf), "Recirculaciones resueltas. Iteraciones: %d, Error final: %.2e",
                 res.iteraciones, res.error_final);
        panel_resultados_.poner_resultado_energia(buf, true);
        // Muestra el reporte completo (masa + energía) con los valores convergidos
        ejecutar_simulacion();
    }
}

void Aplicacion::agregar_anotacion() {
    lienzo_.guardar_estado();
    int64_t aid = flowsheet_->agregar_anotacion("Nueva anotación");
    auto* a = flowsheet_->anotacion_por_id(aid);
    if (a) a->pagina = lienzo_.pagina_actual;
    pila_undo_acciones_.push_back({UndoAction::Tipo::Anotacion, aid});
    panel_propiedades_.seleccionar_anotacion(aid);
    lienzo_.id_anotacion_seleccionada = aid;
    hay_cambios_sin_guardar_ = true;
}

void Aplicacion::agregar_tabla_flujo() {
    lienzo_.guardar_estado();
    flowsheet_->agregar_tabla_flujo();
    hay_cambios_sin_guardar_ = true;
}

void Aplicacion::abrir_archivo() { cargar_proyecto(); }
void Aplicacion::guardar_archivo() { guardar_proyecto(); }

/// Helper: find origin node name for a flow ID
static std::string origen_del_flujo(const cma::diagrama_flujo::Flowsheet& fs, int64_t fid) {
    for (auto& n : fs.nodos())
        for (auto& sfid : n.ids_flujos_salida)
            if (sfid == fid) return n.nombre;
    return "?";
}
/// Helper: find destination node name for a flow ID
static std::string destino_del_flujo(const cma::diagrama_flujo::Flowsheet& fs, int64_t fid) {
    for (auto& n : fs.nodos())
        for (auto& efid : n.ids_flujos_entrada)
            if (efid == fid) return n.nombre;
    return "?";
}

void Aplicacion::ejecutar_simulacion() {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    panel_resultados_.limpiar();

    // ── Validación previa ──
    bool tiene_fuente = false;
    bool tiene_sumidero = false;
    int  n_fuentes = 0, n_sumideros = 0;
    for (const auto& nodo : flowsheet_->nodos()) {
        if (nodo.tipo == nucleo::TipoNodo::Fuente) { tiene_fuente = true; n_fuentes++; }
        if (nodo.tipo == nucleo::TipoNodo::Sumidero) { tiene_sumidero = true; n_sumideros++; }
    }
    if (!tiene_fuente || !tiene_sumidero) {
        std::string err = "No se puede simular: el flowsheet necesita al menos ";
        if (!tiene_fuente && !tiene_sumidero) err += "una Fuente y un Sumidero.";
        else if (!tiene_fuente) err += "una Fuente de entrada.";
        else err += "un Sumidero de salida.";
        panel_resultados_.poner_resultado_masa(err, false);
        sim_resuelto_ = true;
        sim_ultimo_resultado_.convergio = false;
        sim_ultimo_resultado_.mensaje = err;
        return;
    }
    if (flowsheet_->conteo_flujos() == 0) {
        std::string err = "No se puede simular: no hay flujos de conexión. Conecte los equipos.";
        panel_resultados_.poner_resultado_masa(err, false);
        sim_resuelto_ = true;
        sim_ultimo_resultado_.convergio = false;
        sim_ultimo_resultado_.mensaje = err;
        return;
    }

    // ── Balance de masa (SteadyStateEngine) ──
    bool masa_ok = false;
    std::string masa_msg;
    try {
        hsc_clone::SteadyStateEngine sse;
        auto res_masa = sse.ejecutar(*flowsheet_);
        masa_ok = res_masa.convergio;
        masa_msg = res_masa.mensaje;

        // Write solved stream properties back to flowsheet
        if (res_masa.convergio) {
            for (auto& f_res : res_masa.flujos_salida) {
                auto* f_orig = flowsheet_->flujo_por_id(f_res.id);
                if (f_orig) {
                    f_orig->flujo_molar = f_res.flujo_molar;
                    f_orig->temperatura = f_res.temperatura;
                    f_orig->presion = f_res.presion;
                    f_orig->entalpia = f_res.entalpia;
                    f_orig->energia = f_res.energia;
                    f_orig->fase = f_res.fase;
                    f_orig->es_energia = f_res.es_energia;
                    f_orig->flujo_solido = f_res.flujo_solido;
                    f_orig->flujo_liquido = f_res.flujo_liquido;
                    f_orig->flujo_gas = f_res.flujo_gas;
                    f_orig->fraccion_solido = f_res.fraccion_solido;
                    f_orig->fraccion_liquido = f_res.fraccion_liquido;
                    f_orig->fraccion_gas = f_res.fraccion_gas;
                }
            }
        }

        std::string msg = res_masa.convergio
            ? "[OK] Balance de masa convergido."
            : "[ERROR] " + res_masa.mensaje;

        // Resumen de flujos
        msg += "\n-- Flujos del flowsheet --\n";
        int nflujo = 0;
        for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
            if (flujo.es_energia) continue;
            nflujo++;
            double total_kmol = 0.0;
            for (auto& [_, n] : flujo.flujo_molar) total_kmol += n;
            char buf[256];
            snprintf(buf, sizeof(buf), "  #%d: %s -> %s  (%.3f kmol/s  T=%.0fK)",
                     nflujo,
                     origen_del_flujo(*flowsheet_, fid).c_str(),
                     destino_del_flujo(*flowsheet_, fid).c_str(),
                     total_kmol, flujo.temperatura);
            msg += buf;
            msg += "\n";
        }

        // Tabla de especies por flujo (hasta 4 flujos, máx 4 especies)
        msg += "\n-- Especies por flujo (kmol/s) --\n";
        int cnt = 0;
        for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
            if (flujo.es_energia) continue;
            if (++cnt > 4) { msg += "  ...\n"; break; }
            char buf[256];
            snprintf(buf, sizeof(buf), "  #%d (%s):", cnt, origen_del_flujo(*flowsheet_, fid).c_str());
            msg += buf;
            int sp_cnt = 0;
            double total = 0.0;
            for (auto& [sid, n] : flujo.flujo_molar) total += n;
            for (auto& [sid, n] : flujo.flujo_molar) {
                if (++sp_cnt > 4) { msg += " ..."; break; }
                auto* sp = gestor.especie_por_id(sid);
                char buf2[128];
                double fracc = total > 0 ? n / total : 0.0;
                if (n < 1e-4) continue;
                snprintf(buf2, sizeof(buf2), " %s=%.4f(%d%%)",
                         sp ? sp->nombre.c_str() : "?", n, (int)(fracc * 100 + 0.5));
                msg += buf2;
            }
            msg += "\n";
        }

        msg += "\n" + std::to_string(flowsheet_->conteo_flujos()) + " flujos materiales totales.\n";

        panel_resultados_.poner_resultado_masa(msg, res_masa.convergio);
    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_masa(std::string("Error (masa): ") + e.what(), false);
    }

    // ── Balance de energía (HeatMaterialBalance) ──
    try {
        hsc_clone::HeatMaterialBalance hmb;
        double H_fuentes = 0.0;
        double H_sumideros = 0.0;
        double Q_neto_equipos = 0.0;
        bool equipos_ok = true;
        std::string en_msg;

        for (auto& nodo : flowsheet_->nodos()) {
            std::vector<nucleo::Flujo> entradas, salidas;
            auto& mapa_f = flowsheet_->mapa_flujos();
            for (auto& fid : nodo.ids_flujos_entrada)
                if (auto it = mapa_f.find(fid); it != mapa_f.end())
                    entradas.push_back(it->second);
            for (auto& fid : nodo.ids_flujos_salida)
                if (auto it = mapa_f.find(fid); it != mapa_f.end())
                    salidas.push_back(it->second);

            auto res_nodo = hmb.calcular(nodo, entradas, salidas, db);
            
            if (nodo.tipo == nucleo::TipoNodo::Fuente) {
                H_fuentes += res_nodo.entalpia_salida;
            } else if (nodo.tipo == nucleo::TipoNodo::Sumidero) {
                H_sumideros += res_nodo.entalpia_entrada;
            } else {
                Q_neto_equipos += res_nodo.calor;
                if (!res_nodo.balanceado) equipos_ok = false;
            }

            char buf[256];
            snprintf(buf, sizeof(buf), "  %s: H_in=%.1f  H_out=%.1f  dH=%+.1f kJ/s",
                     nodo.nombre.c_str(),
                     res_nodo.entalpia_entrada,
                     res_nodo.entalpia_salida,
                     res_nodo.calor);
            en_msg += buf;
            en_msg += "\n";
        }

        char sum[256];
        snprintf(sum, sizeof(sum),
                 "-- Balance energetico Global --\n"
                 "Entalpia de Reactivos (Fuentes): %+.1f kJ/s\n"
                 "Entalpia de Productos (Sumideros): %+.1f kJ/s\n"
                 "Calor Neto Requerido por Proceso: %+.1f kJ/s\n"
                 "Estado termico de equipos: %s\n\n"
                 "-- Detalle por equipo --\n",
                 H_fuentes, H_sumideros, Q_neto_equipos,
                 equipos_ok ? "Balanceado (Autotérmico)" : "Requiere calefacción/refrigeración externa");
        en_msg = sum + en_msg;

        panel_resultados_.poner_resultado_energia(en_msg, true);
        
        // ── BALANCE DE EXERGÍA ──
        try {
            motores::BalanceExergia motor_exergia;
            auto res_ex = motor_exergia.calcular_flowsheet(*flowsheet_, db);
            panel_resultados_.poner_resultado_exergia(res_ex, true, "Balance exergético calculado exitosamente.");
        } catch (const std::exception& e) {
            motores::ResultadoBalanceExergia res_vacio;
            panel_resultados_.poner_resultado_exergia(res_vacio, false, std::string("Error (exergía): ") + e.what());
        }

    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_energia(std::string("Error (energía): ") + e.what(), false);
    }

    // Guardar estado de simulación para la barra de estado
    sim_resuelto_ = true;
    sim_ultimo_resultado_.convergio = masa_ok;
    sim_ultimo_resultado_.mensaje = masa_ok
        ? ("Simulación convergió. " + std::to_string(flowsheet_->conteo_flujos()) + " flujos.")
        : ("Error: " + masa_msg);
}

void Aplicacion::exportar_resultados_csv() {
    std::string cmd = "zenity --file-selection --save --confirm-overwrite --title=\"Exportar Resultados CSV\" --file-filter=\"CSV | *.csv\" 2>/dev/null";
    hsc_clone::ExternalAPI api;
    std::string filepath = api.ejecutarComando(cmd);
    if (!filepath.empty() && filepath.back() == '\n') filepath.pop_back();
    if (filepath.empty()) return;
    if (filepath.size() < 4 || filepath.substr(filepath.size() - 4) != ".csv")
        filepath += ".csv";

    auto& gestor = base_datos::GestorDB::instancia();

    std::ostringstream ss;
    ss << "Flujo_ID,Nombre,Origen,Destino,Temperatura(K),Presion(atm),Flujo_Total(kmol/s),Especie,Flujo_Especie(kmol/s),Fase\n";

    for (auto& [fid, flujo] : flowsheet_->mapa_flujos()) {
        if (flujo.es_energia) {
            ss << fid << "," << flujo.nombre << ",,,"
               << flujo.temperatura << "," << flujo.presion << ","
               << flujo.energia << ",(Energia kW),,\n";
            continue;
        }
        double total = flujo.flujo_total();
        std::string origen = origen_del_flujo(*flowsheet_, fid);
        std::string destino = destino_del_flujo(*flowsheet_, fid);

        if (flujo.flujo_molar.empty()) {
            ss << fid << "," << flujo.nombre << "," << origen << "," << destino
               << "," << flujo.temperatura << "," << flujo.presion << ","
               << total << ",(vacio),,\n";
        } else {
            for (auto& [sid, n] : flujo.flujo_molar) {
                const auto* esp = gestor.especie_por_id(sid);
                std::string nombre_esp = esp ? esp->nombre : ("ID_" + std::to_string(sid));
                ss << fid << "," << flujo.nombre << "," << origen << "," << destino
                   << "," << flujo.temperatura << "," << flujo.presion << ","
                   << total << "," << nombre_esp << "," << n << ","
                   << nucleo::nombre_fase(flujo.fase) << "\n";
            }
        }
    }

    std::ofstream f(filepath, std::ios::binary);
    if (f.is_open()) {
        f << ss.rdbuf();
        f.close();
        panel_resultados_.poner_resultado_masa("Resultados exportados: " + filepath, true);
    } else {
        panel_resultados_.poner_resultado_masa("Error al exportar CSV: " + filepath, false);
    }
}

void Aplicacion::ejecutar_gibbs() {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    try {
        hsc_clone::GibbsMinimizer gibbs;
        std::map<int64_t, double> alimentacion;
        for (auto& nodo : flowsheet_->nodos()) {
            if (nodo.tipo == nucleo::TipoNodo::Fuente)
                for (auto& [k, v] : nodo.params)
                    if (k.rfind("flujo_", 0) == 0) {
                        try {
                            size_t pos;
                            int64_t sid = std::stoll(k.substr(6), &pos);
                            if (sid > 0 && pos == k.size() - 6)
                                alimentacion[sid] += v;
                        } catch (...) {}
                    }
        }
        if (alimentacion.empty()) {
            auto* h2o = gestor.especie_por_nombre("Agua");
            auto* co2 = gestor.especie_por_nombre("Dióxido de Carbono");
            if (h2o) alimentacion[h2o->id] = 1.0;
            if (co2) alimentacion[co2->id] = 1.0;
        }
        auto res = gibbs.minimizar(alimentacion, 1000.0, 1.0, db);
        panel_resultados_.poner_resultado_gibbs(res.mensaje, res.convergio, res.flujo_molar, db);
    } catch (const std::exception& e) {
        panel_resultados_.poner_resultado_gibbs(std::string("Error: ") + e.what(), false, {}, db);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Paneles
// ═══════════════════════════════════════════════════════════════════════════



// ═══════════════════════════════════════════════════════════════════════════
// Atajos de teclado globales
// ═══════════════════════════════════════════════════════════════════════════

void Aplicacion::manejar_atajos_globales() {
    auto& io = ImGui::GetIO();
    if (io.WantTextInput) return;

    bool ctrl = io.KeyCtrl;
    bool shift = io.KeyShift;

    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_N)) nueva_simulacion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_E)) cargar_ejemplo();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_O)) cargar_proyecto();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_S)) guardar_proyecto();
    if (ctrl && shift && ImGui::IsKeyPressed(ImGuiKey_S)) guardar_proyecto_como();

    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_C)) copiar_seleccion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_V)) pegar_seleccion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_Z)) lienzo_.deshacer();
    if ((ctrl && shift && ImGui::IsKeyPressed(ImGuiKey_Z)) ||
        (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_Y))) lienzo_.rehacer();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_D)) duplicar_seleccion();

    if (ImGui::IsKeyPressed(ImGuiKey_F5)) ejecutar_simulacion();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_R)) ejecutar_convergencia();
    if (ctrl && !shift && ImGui::IsKeyPressed(ImGuiKey_G)) ejecutar_gibbs();

    if (ImGui::IsKeyPressed(ImGuiKey_Delete) || ImGui::IsKeyPressed(ImGuiKey_KeypadDecimal)) {
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
}

void Aplicacion::copiar_seleccion() {
    if (lienzo_.id_nodo_seleccionado > 0) {
        auto* n = flowsheet_->nodo_por_id(lienzo_.id_nodo_seleccionado);
        if (n) {
            portapapeles_ = NodoClipboard{n->tipo, n->nombre, n->params};
            portapapeles_anotacion_ = std::nullopt;
        }
    } else if (lienzo_.id_anotacion_seleccionada > 0) {
        diagrama_flujo::Anotacion* found = nullptr;
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == lienzo_.id_anotacion_seleccionada) {
                found = &a;
                break;
            }
        }
        if (found) {
            portapapeles_anotacion_ = AnotacionClipboard{found->tipo, found->w, found->h, found->texto, found->color_fondo, found->color_borde, found->grosor_borde};
            portapapeles_ = std::nullopt;
        }
    }
}

void Aplicacion::pegar_seleccion() {
    if (portapapeles_) {
        lienzo_.guardar_estado();
        auto& clip = *portapapeles_;
        auto id = flowsheet_->agregar_nodo(clip.tipo);
        auto* n = flowsheet_->nodo_por_id(id);
        if (!n) return;
        n->nombre = clip.nombre + " (copia)";
        n->params = clip.params;
        n->canvas_x = std::min(n->canvas_x + 30.0f, 1200.0f);
        n->canvas_y = std::min(n->canvas_y + 30.0f, 800.0f);
        n->pagina = lienzo_.pagina_actual;
        pila_undo_acciones_.push_back({UndoAction::Tipo::Nodo, id});
        panel_propiedades_.seleccionar_nodo(id);
        lienzo_.id_nodo_seleccionado = id;
        hay_cambios_sin_guardar_ = true;
    } else if (portapapeles_anotacion_) {
        lienzo_.guardar_estado();
        auto& clip = *portapapeles_anotacion_;
        int64_t aid = flowsheet_->agregar_anotacion(clip.texto);
        for (auto& a : flowsheet_->anotaciones()) {
            if (a.id == aid) {
                a.tipo = clip.tipo;
                a.w = clip.w;
                a.h = clip.h;
                a.color_fondo = clip.color_fondo;
                a.color_borde = clip.color_borde;
                a.grosor_borde = clip.grosor_borde;
                a.pagina = lienzo_.pagina_actual;
                // Offset slightly
                a.x = std::min(a.x + 30.0f, 1200.0f);
                a.y = std::min(a.y + 30.0f, 800.0f);
                break;
            }
        }
        pila_undo_acciones_.push_back({UndoAction::Tipo::Anotacion, aid});
        panel_propiedades_.seleccionar_anotacion(aid);
        lienzo_.id_anotacion_seleccionada = aid;
        hay_cambios_sin_guardar_ = true;
    }
}

void Aplicacion::duplicar_seleccion() {
    if (lienzo_.id_nodo_seleccionado > 0) {
        auto* n = flowsheet_->nodo_por_id(lienzo_.id_nodo_seleccionado);
        if (!n) return;
        copiar_seleccion();
        pegar_seleccion();
    } else if (lienzo_.id_anotacion_seleccionada > 0) {
        copiar_seleccion();
        pegar_seleccion();
    }
}

void Aplicacion::deshacer_ultimo() {
    if (pila_undo_acciones_.empty()) return;
    auto act = pila_undo_acciones_.back();
    pila_undo_acciones_.pop_back();
    if (act.tipo == UndoAction::Tipo::Nodo) {
        flowsheet_->eliminar_nodo(act.id);
        lienzo_.id_nodo_seleccionado = -1;
    } else {
        flowsheet_->eliminar_anotacion(act.id);
        lienzo_.id_anotacion_seleccionada = -1;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Cierre
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
