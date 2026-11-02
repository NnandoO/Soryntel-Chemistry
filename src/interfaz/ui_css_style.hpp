#ifndef CMA_INTERFAZ_UI_CSS_STYLE_HPP
#define CMA_INTERFAZ_UI_CSS_STYLE_HPP

#include <imgui.h>
#include <implot.h>
#include <cmath>
#include <cstdio>
#include <string>
#include <vector>

namespace cma { namespace interfaz { namespace css {

// ═══════════════════════════════════════════════════════════════════════════
// Motor de Estilos Declarativo tipo CSS3 para C++ / Dear ImGui (Global & Local)
// Soporta de forma nativa y simétrica Modo Claro (HSC Silver / Outotec Aero)
// y Modo Oscuro (HSC Carbon / Slate Dark), además de iconografía vectorial.
// ═══════════════════════════════════════════════════════════════════════════

inline bool is_light_theme() {
    return ImGui::GetStyle().Colors[ImGuiCol_WindowBg].x > 0.50f;
}

struct ThemeTokens {
    bool light = true;
    ImU32 win_bg          = IM_COL32(242, 245, 249, 255);
    ImU32 panel_bg        = IM_COL32(250, 251, 253, 255);
    ImU32 card_bg         = IM_COL32(255, 255, 255, 255);
    ImU32 ribbon_bg_top   = IM_COL32(250, 252, 255, 255);
    ImU32 ribbon_bg_bot   = IM_COL32(233, 238, 246, 255);
    ImU32 header_bg       = IM_COL32(228, 234, 242, 255);
    ImU32 border_strong   = IM_COL32(168, 178, 192, 255);
    ImU32 border_soft     = IM_COL32(204, 212, 224, 255);
    ImU32 text_primary    = IM_COL32(22, 27, 34, 255);
    ImU32 text_secondary  = IM_COL32(86, 96, 110, 255);
    ImU32 text_muted      = IM_COL32(124, 134, 148, 255);
    ImU32 accent_blue     = IM_COL32(36, 102, 184, 255);
    ImU32 accent_olive    = IM_COL32(107, 123, 55, 255);
    ImU32 accent_green    = IM_COL32(46, 142, 78, 255);
    ImU32 accent_red      = IM_COL32(198, 56, 52, 255);
    ImU32 accent_amber    = IM_COL32(212, 136, 28, 255);
};

inline ThemeTokens tokens() {
    ThemeTokens t;
    t.light = is_light_theme();
    if (t.light) {
        t.win_bg         = IM_COL32(242, 245, 249, 255);
        t.panel_bg       = IM_COL32(249, 251, 254, 255);
        t.card_bg        = IM_COL32(255, 255, 255, 255);
        t.ribbon_bg_top  = IM_COL32(252, 253, 255, 255);
        t.ribbon_bg_bot  = IM_COL32(233, 238, 246, 255);
        t.header_bg      = IM_COL32(226, 232, 241, 255);
        t.border_strong  = IM_COL32(166, 176, 190, 255);
        t.border_soft    = IM_COL32(204, 212, 223, 255);
        t.text_primary   = IM_COL32(20, 25, 32, 255);
        t.text_secondary = IM_COL32(78, 88, 104, 255);
        t.text_muted     = IM_COL32(118, 128, 142, 255);
        t.accent_blue    = IM_COL32(36, 102, 184, 255);
        t.accent_olive   = IM_COL32(107, 123, 55, 255);
        t.accent_green   = IM_COL32(44, 140, 76, 255);
        t.accent_red     = IM_COL32(198, 56, 52, 255);
        t.accent_amber   = IM_COL32(212, 136, 28, 255);
    } else {
        t.win_bg         = IM_COL32(24, 28, 35, 255);
        t.panel_bg       = IM_COL32(30, 35, 44, 255);
        t.card_bg        = IM_COL32(20, 24, 31, 255);
        t.ribbon_bg_top  = IM_COL32(38, 45, 56, 255);
        t.ribbon_bg_bot  = IM_COL32(27, 32, 41, 255);
        t.header_bg      = IM_COL32(42, 50, 63, 255);
        t.border_strong  = IM_COL32(72, 84, 102, 255);
        t.border_soft    = IM_COL32(52, 61, 76, 255);
        t.text_primary   = IM_COL32(236, 241, 248, 255);
        t.text_secondary = IM_COL32(168, 180, 196, 255);
        t.text_muted     = IM_COL32(122, 134, 150, 255);
        t.accent_blue    = IM_COL32(68, 142, 232, 255);
        t.accent_olive   = IM_COL32(128, 152, 66, 255);
        t.accent_green   = IM_COL32(62, 172, 98, 255);
        t.accent_red     = IM_COL32(224, 78, 72, 255);
        t.accent_amber   = IM_COL32(235, 162, 48, 255);
    }
    return t;
}

struct BoxStyle {
    ImU32 bg_top         = IM_COL32(248, 249, 251, 255);
    ImU32 bg_bottom      = IM_COL32(232, 235, 240, 255);
    ImU32 bg_hover_top   = IM_COL32(226, 238, 252, 255);
    ImU32 bg_hover_bot   = IM_COL32(198, 222, 248, 255);
    ImU32 bg_active_top  = IM_COL32(185, 212, 242, 255);
    ImU32 bg_active_bot  = IM_COL32(162, 195, 235, 255);
    ImU32 border_color   = IM_COL32(180, 186, 196, 255);
    ImU32 border_hover   = IM_COL32(105, 155, 215, 255);
    ImU32 text_color     = IM_COL32(28, 32, 38, 255);
    ImU32 subtext_color  = IM_COL32(95, 102, 112, 255);
    float border_radius  = 4.0f;
    float border_width   = 1.0f;

    static BoxStyle from_theme() {
        BoxStyle b;
        if (is_light_theme()) {
            b.bg_top        = IM_COL32(251, 252, 254, 255);
            b.bg_bottom     = IM_COL32(232, 237, 244, 255);
            b.bg_hover_top  = IM_COL32(232, 243, 255, 255);
            b.bg_hover_bot  = IM_COL32(204, 226, 250, 255);
            b.bg_active_top = IM_COL32(186, 214, 246, 255);
            b.bg_active_bot = IM_COL32(162, 196, 238, 255);
            b.border_color  = IM_COL32(182, 192, 206, 255);
            b.border_hover  = IM_COL32(82, 142, 212, 255);
            b.text_color    = IM_COL32(22, 27, 34, 255);
            b.subtext_color = IM_COL32(88, 98, 112, 255);
        } else {
            b.bg_top        = IM_COL32(46, 54, 66, 255);
            b.bg_bottom     = IM_COL32(34, 40, 50, 255);
            b.bg_hover_top  = IM_COL32(58, 76, 102, 255);
            b.bg_hover_bot  = IM_COL32(44, 60, 84, 255);
            b.bg_active_top = IM_COL32(48, 88, 142, 255);
            b.bg_active_bot = IM_COL32(38, 72, 120, 255);
            b.border_color  = IM_COL32(72, 84, 102, 255);
            b.border_hover  = IM_COL32(96, 158, 235, 255);
            b.text_color    = IM_COL32(236, 241, 248, 255);
            b.subtext_color = IM_COL32(165, 176, 192, 255);
        }
        return b;
    }
};

enum class RibbonIcon {
    Save,
    FolderOpen,
    NewDoc,
    InputArrow,
    OutputArrow,
    ManualInput,
    ManualOutput,
    Indicator,
    Mapping,
    Normalize,
    Help,
    ToGaBi,
    ToExcel,
    AddRow,
    SubMaterial,
    SendSim,
    BalanceScale,
    ResetDB,
    Duplicate,
    Trash,
    WindowTileH,
    WindowTileV,
    WindowCascade,
    MineralDB,
    FindMineral,
    Crystal,
    IdentifyQuestion,
    ModalBarChart,
    Liberation,
    PlusCircle,
    MinusCircle,
    Eraser,
    PlayArrow,
    Sun,
    Moon
};

// ── Hoja de Estilo Global CSS para toda la aplicación (Claro HSC / Oscuro Carbon) ──
inline void apply_global_stylesheet(bool dark_mode) {
    ImGuiStyle& style = ImGui::GetStyle();

    // Box-model & Layout CSS rules
    style.WindowPadding     = ImVec2(10.0f, 10.0f);
    style.FramePadding      = ImVec2(8.0f, 5.0f);
    style.CellPadding       = ImVec2(6.0f, 4.0f);
    style.ItemSpacing       = ImVec2(8.0f, 6.0f);
    style.ItemInnerSpacing  = ImVec2(6.0f, 5.0f);
    style.IndentSpacing     = 18.0f;
    style.ScrollbarSize     = 13.0f;
    style.GrabMinSize       = 10.0f;

    // Border-width CSS rules (1px solid borders for engineering precision)
    style.WindowBorderSize  = 1.0f;
    style.ChildBorderSize   = 1.0f;
    style.PopupBorderSize   = 1.0f;
    style.FrameBorderSize   = 1.0f;
    style.TabBorderSize     = 1.0f;

    // Border-radius CSS rules
    style.WindowRounding    = 5.0f;
    style.ChildRounding     = 4.0f;
    style.FrameRounding     = 3.5f;
    style.PopupRounding     = 4.0f;
    style.ScrollbarRounding = 6.0f;
    style.GrabRounding      = 3.0f;
    style.TabRounding       = 4.0f;

    ImVec4* colors = style.Colors;

    if (!dark_mode) {
        // ── Tema Claro HSC Chemistry / Outotec Engineering (Silver & Slate) ──
        colors[ImGuiCol_Text]                  = ImVec4(0.10f, 0.12f, 0.15f, 1.00f);
        colors[ImGuiCol_TextDisabled]          = ImVec4(0.45f, 0.49f, 0.55f, 1.00f);
        colors[ImGuiCol_WindowBg]              = ImVec4(0.95f, 0.96f, 0.97f, 1.00f);
        colors[ImGuiCol_ChildBg]               = ImVec4(0.98f, 0.99f, 1.00f, 1.00f);
        colors[ImGuiCol_PopupBg]               = ImVec4(0.99f, 0.99f, 1.00f, 0.99f);
        colors[ImGuiCol_Border]                = ImVec4(0.71f, 0.75f, 0.81f, 1.00f);
        colors[ImGuiCol_BorderShadow]          = ImVec4(0.00f, 0.00f, 0.00f, 0.04f);
        colors[ImGuiCol_FrameBg]               = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
        colors[ImGuiCol_FrameBgHovered]        = ImVec4(0.92f, 0.96f, 1.00f, 1.00f);
        colors[ImGuiCol_FrameBgActive]         = ImVec4(0.85f, 0.92f, 0.99f, 1.00f);
        colors[ImGuiCol_TitleBg]               = ImVec4(0.84f, 0.87f, 0.92f, 1.00f);
        colors[ImGuiCol_TitleBgActive]         = ImVec4(0.76f, 0.82f, 0.89f, 1.00f);
        colors[ImGuiCol_TitleBgCollapsed]      = ImVec4(0.88f, 0.90f, 0.94f, 0.85f);
        colors[ImGuiCol_MenuBarBg]             = ImVec4(0.91f, 0.93f, 0.96f, 1.00f);
        colors[ImGuiCol_ScrollbarBg]           = ImVec4(0.93f, 0.94f, 0.96f, 1.00f);
        colors[ImGuiCol_ScrollbarGrab]         = ImVec4(0.73f, 0.77f, 0.82f, 1.00f);
        colors[ImGuiCol_ScrollbarGrabHovered]  = ImVec4(0.60f, 0.65f, 0.72f, 1.00f);
        colors[ImGuiCol_ScrollbarGrabActive]   = ImVec4(0.46f, 0.53f, 0.62f, 1.00f);
        colors[ImGuiCol_CheckMark]             = ImVec4(0.15f, 0.44f, 0.78f, 1.00f);
        colors[ImGuiCol_SliderGrab]            = ImVec4(0.20f, 0.48f, 0.82f, 1.00f);
        colors[ImGuiCol_SliderGrabActive]      = ImVec4(0.13f, 0.38f, 0.70f, 1.00f);
        colors[ImGuiCol_Button]                = ImVec4(0.90f, 0.92f, 0.95f, 1.00f);
        colors[ImGuiCol_ButtonHovered]         = ImVec4(0.81f, 0.88f, 0.97f, 1.00f);
        colors[ImGuiCol_ButtonActive]          = ImVec4(0.71f, 0.82f, 0.95f, 1.00f);
        colors[ImGuiCol_Header]                = ImVec4(0.82f, 0.88f, 0.96f, 1.00f);
        colors[ImGuiCol_HeaderHovered]         = ImVec4(0.87f, 0.92f, 0.98f, 1.00f);
        colors[ImGuiCol_HeaderActive]          = ImVec4(0.75f, 0.84f, 0.95f, 1.00f);
        colors[ImGuiCol_Separator]             = ImVec4(0.76f, 0.80f, 0.85f, 1.00f);
        colors[ImGuiCol_SeparatorHovered]      = ImVec4(0.22f, 0.52f, 0.86f, 0.80f);
        colors[ImGuiCol_SeparatorActive]       = ImVec4(0.16f, 0.44f, 0.78f, 1.00f);
        colors[ImGuiCol_ResizeGrip]            = ImVec4(0.22f, 0.52f, 0.86f, 0.25f);
        colors[ImGuiCol_ResizeGripHovered]     = ImVec4(0.22f, 0.52f, 0.86f, 0.65f);
        colors[ImGuiCol_ResizeGripActive]      = ImVec4(0.16f, 0.44f, 0.78f, 0.95f);
        colors[ImGuiCol_Tab]                   = ImVec4(0.87f, 0.90f, 0.94f, 1.00f);
        colors[ImGuiCol_TabHovered]            = ImVec4(0.80f, 0.87f, 0.96f, 1.00f);
        colors[ImGuiCol_TabActive]             = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
        colors[ImGuiCol_TabUnfocused]          = ImVec4(0.85f, 0.88f, 0.92f, 1.00f);
        colors[ImGuiCol_TabUnfocusedActive]    = ImVec4(0.95f, 0.97f, 0.99f, 1.00f);
        colors[ImGuiCol_PlotLines]             = ImVec4(0.15f, 0.44f, 0.78f, 1.00f);
        colors[ImGuiCol_PlotLinesHovered]      = ImVec4(0.85f, 0.35f, 0.15f, 1.00f);
        colors[ImGuiCol_PlotHistogram]         = ImVec4(0.42f, 0.48f, 0.22f, 1.00f);
        colors[ImGuiCol_PlotHistogramHovered]  = ImVec4(0.50f, 0.58f, 0.26f, 1.00f);
        colors[ImGuiCol_TableHeaderBg]         = ImVec4(0.88f, 0.91f, 0.95f, 1.00f);
        colors[ImGuiCol_TableBorderStrong]      = ImVec4(0.66f, 0.70f, 0.77f, 1.00f);
        colors[ImGuiCol_TableBorderLight]       = ImVec4(0.81f, 0.84f, 0.89f, 1.00f);
        colors[ImGuiCol_TableRowBg]            = ImVec4(1.00f, 1.00f, 1.00f, 1.00f);
        colors[ImGuiCol_TableRowBgAlt]         = ImVec4(0.96f, 0.97f, 0.99f, 1.00f);
        colors[ImGuiCol_TextSelectedBg]        = ImVec4(0.20f, 0.50f, 0.85f, 0.28f);
        colors[ImGuiCol_DragDropTarget]        = ImVec4(0.18f, 0.48f, 0.82f, 0.90f);
        colors[ImGuiCol_NavHighlight]          = ImVec4(0.20f, 0.50f, 0.85f, 1.00f);

        ImPlotStyle& plot = ImPlot::GetStyle();
        plot.Colors[ImPlotCol_PlotBg]      = ImVec4(0.98f, 0.99f, 1.00f, 1.00f);
        plot.Colors[ImPlotCol_FrameBg]     = ImVec4(0.95f, 0.96f, 0.98f, 1.00f);
        plot.Colors[ImPlotCol_PlotBorder]  = ImVec4(0.68f, 0.72f, 0.78f, 1.00f);
        plot.Colors[ImPlotCol_LegendBg]    = ImVec4(0.98f, 0.99f, 1.00f, 0.94f);
        plot.Colors[ImPlotCol_LegendBorder]= ImVec4(0.72f, 0.76f, 0.82f, 1.00f);
        plot.Colors[ImPlotCol_LegendText]  = ImVec4(0.12f, 0.14f, 0.18f, 1.00f);
        plot.Colors[ImPlotCol_TitleText]   = ImVec4(0.10f, 0.12f, 0.16f, 1.00f);
        plot.Colors[ImPlotCol_InlayText]   = ImVec4(0.12f, 0.14f, 0.18f, 1.00f);
        plot.Colors[ImPlotCol_AxisText]    = ImVec4(0.15f, 0.18f, 0.22f, 1.00f);
        plot.Colors[ImPlotCol_AxisGrid]    = ImVec4(0.80f, 0.83f, 0.88f, 0.65f);
    } else {
        // ── Tema Oscuro HSC Carbon / Slate Engineering ──
        colors[ImGuiCol_Text]                  = ImVec4(0.92f, 0.94f, 0.97f, 1.00f);
        colors[ImGuiCol_TextDisabled]          = ImVec4(0.55f, 0.60f, 0.67f, 1.00f);
        colors[ImGuiCol_WindowBg]              = ImVec4(0.10f, 0.12f, 0.15f, 1.00f);
        colors[ImGuiCol_ChildBg]               = ImVec4(0.13f, 0.15f, 0.19f, 1.00f);
        colors[ImGuiCol_PopupBg]               = ImVec4(0.13f, 0.15f, 0.19f, 0.98f);
        colors[ImGuiCol_Border]                = ImVec4(0.25f, 0.30f, 0.37f, 1.00f);
        colors[ImGuiCol_BorderShadow]          = ImVec4(0.00f, 0.00f, 0.00f, 0.22f);
        colors[ImGuiCol_FrameBg]               = ImVec4(0.08f, 0.10f, 0.13f, 1.00f);
        colors[ImGuiCol_FrameBgHovered]        = ImVec4(0.15f, 0.20f, 0.27f, 1.00f);
        colors[ImGuiCol_FrameBgActive]         = ImVec4(0.19f, 0.26f, 0.36f, 1.00f);
        colors[ImGuiCol_TitleBg]               = ImVec4(0.08f, 0.10f, 0.13f, 1.00f);
        colors[ImGuiCol_TitleBgActive]         = ImVec4(0.13f, 0.18f, 0.25f, 1.00f);
        colors[ImGuiCol_TitleBgCollapsed]      = ImVec4(0.08f, 0.10f, 0.13f, 0.85f);
        colors[ImGuiCol_MenuBarBg]             = ImVec4(0.11f, 0.13f, 0.17f, 1.00f);
        colors[ImGuiCol_ScrollbarBg]           = ImVec4(0.09f, 0.11f, 0.14f, 1.00f);
        colors[ImGuiCol_ScrollbarGrab]         = ImVec4(0.28f, 0.33f, 0.40f, 1.00f);
        colors[ImGuiCol_ScrollbarGrabHovered]  = ImVec4(0.37f, 0.43f, 0.51f, 1.00f);
        colors[ImGuiCol_ScrollbarGrabActive]   = ImVec4(0.47f, 0.54f, 0.64f, 1.00f);
        colors[ImGuiCol_CheckMark]             = ImVec4(0.36f, 0.68f, 1.00f, 1.00f);
        colors[ImGuiCol_SliderGrab]            = ImVec4(0.32f, 0.63f, 0.96f, 1.00f);
        colors[ImGuiCol_SliderGrabActive]      = ImVec4(0.45f, 0.74f, 1.00f, 1.00f);
        colors[ImGuiCol_Button]                = ImVec4(0.17f, 0.21f, 0.27f, 1.00f);
        colors[ImGuiCol_ButtonHovered]         = ImVec4(0.24f, 0.31f, 0.41f, 1.00f);
        colors[ImGuiCol_ButtonActive]          = ImVec4(0.26f, 0.45f, 0.72f, 1.00f);
        colors[ImGuiCol_Header]                = ImVec4(0.18f, 0.25f, 0.35f, 1.00f);
        colors[ImGuiCol_HeaderHovered]         = ImVec4(0.24f, 0.33f, 0.45f, 1.00f);
        colors[ImGuiCol_HeaderActive]          = ImVec4(0.28f, 0.45f, 0.69f, 1.00f);
        colors[ImGuiCol_Separator]             = ImVec4(0.23f, 0.28f, 0.35f, 1.00f);
        colors[ImGuiCol_SeparatorHovered]      = ImVec4(0.36f, 0.68f, 1.00f, 0.78f);
        colors[ImGuiCol_SeparatorActive]       = ImVec4(0.36f, 0.68f, 1.00f, 1.00f);
        colors[ImGuiCol_ResizeGrip]            = ImVec4(0.36f, 0.68f, 1.00f, 0.22f);
        colors[ImGuiCol_ResizeGripHovered]     = ImVec4(0.36f, 0.68f, 1.00f, 0.65f);
        colors[ImGuiCol_ResizeGripActive]      = ImVec4(0.36f, 0.68f, 1.00f, 0.95f);
        colors[ImGuiCol_Tab]                   = ImVec4(0.13f, 0.16f, 0.21f, 1.00f);
        colors[ImGuiCol_TabHovered]            = ImVec4(0.23f, 0.31f, 0.42f, 1.00f);
        colors[ImGuiCol_TabActive]             = ImVec4(0.19f, 0.25f, 0.34f, 1.00f);
        colors[ImGuiCol_TabUnfocused]          = ImVec4(0.11f, 0.13f, 0.17f, 1.00f);
        colors[ImGuiCol_TabUnfocusedActive]    = ImVec4(0.15f, 0.19f, 0.25f, 1.00f);
        colors[ImGuiCol_PlotLines]             = ImVec4(0.38f, 0.70f, 1.00f, 1.00f);
        colors[ImGuiCol_PlotLinesHovered]      = ImVec4(1.00f, 0.65f, 0.25f, 1.00f);
        colors[ImGuiCol_PlotHistogram]         = ImVec4(0.52f, 0.68f, 0.28f, 1.00f);
        colors[ImGuiCol_PlotHistogramHovered]  = ImVec4(0.62f, 0.78f, 0.35f, 1.00f);
        colors[ImGuiCol_TableHeaderBg]         = ImVec4(0.16f, 0.20f, 0.26f, 1.00f);
        colors[ImGuiCol_TableBorderStrong]      = ImVec4(0.27f, 0.32f, 0.40f, 1.00f);
        colors[ImGuiCol_TableBorderLight]       = ImVec4(0.19f, 0.23f, 0.29f, 1.00f);
        colors[ImGuiCol_TableRowBg]            = ImVec4(0.11f, 0.13f, 0.17f, 1.00f);
        colors[ImGuiCol_TableRowBgAlt]         = ImVec4(0.14f, 0.17f, 0.21f, 1.00f);
        colors[ImGuiCol_TextSelectedBg]        = ImVec4(0.24f, 0.54f, 0.90f, 0.36f);
        colors[ImGuiCol_DragDropTarget]        = ImVec4(0.95f, 0.75f, 0.20f, 0.90f);
        colors[ImGuiCol_NavHighlight]          = ImVec4(0.36f, 0.68f, 1.00f, 1.00f);

        ImPlotStyle& plot = ImPlot::GetStyle();
        plot.Colors[ImPlotCol_PlotBg]      = ImVec4(0.09f, 0.11f, 0.14f, 1.00f);
        plot.Colors[ImPlotCol_FrameBg]     = ImVec4(0.13f, 0.15f, 0.19f, 1.00f);
        plot.Colors[ImPlotCol_PlotBorder]  = ImVec4(0.26f, 0.30f, 0.37f, 1.00f);
        plot.Colors[ImPlotCol_LegendBg]    = ImVec4(0.13f, 0.15f, 0.19f, 0.94f);
        plot.Colors[ImPlotCol_LegendBorder]= ImVec4(0.26f, 0.30f, 0.37f, 1.00f);
        plot.Colors[ImPlotCol_LegendText]  = ImVec4(0.90f, 0.92f, 0.95f, 1.00f);
        plot.Colors[ImPlotCol_TitleText]   = ImVec4(0.92f, 0.94f, 0.96f, 1.00f);
        plot.Colors[ImPlotCol_InlayText]   = ImVec4(0.90f, 0.92f, 0.95f, 1.00f);
        plot.Colors[ImPlotCol_AxisText]    = ImVec4(0.84f, 0.87f, 0.91f, 1.00f);
        plot.Colors[ImPlotCol_AxisGrid]    = ImVec4(0.24f, 0.28f, 0.34f, 0.55f);
    }
}

// Dibuja un contenedor con gradiente vertical y borde estilo CSS (background: linear-gradient(...); border: 1px solid ...)
inline void draw_css_box(ImDrawList* dl, ImVec2 p_min, ImVec2 p_max, const BoxStyle& st, bool hovered = false, bool active = false) {
    ImU32 top = active ? st.bg_active_top : (hovered ? st.bg_hover_top : st.bg_top);
    ImU32 bot = active ? st.bg_active_bot : (hovered ? st.bg_hover_bot : st.bg_bottom);
    ImU32 brd = (hovered || active) ? st.border_hover : st.border_color;

    if (st.border_radius > 0.5f) {
        dl->AddRectFilled(p_min, p_max, bot, st.border_radius);
        dl->AddRectFilledMultiColor(
            ImVec2(p_min.x + 1, p_min.y + 1),
            ImVec2(p_max.x - 1, p_max.y - 1),
            top, top, bot, bot);
        if (st.border_width > 0.0f) {
            dl->AddRect(p_min, p_max, brd, st.border_radius, 0, st.border_width);
        }
    } else {
        dl->AddRectFilledMultiColor(p_min, p_max, top, top, bot, bot);
        if (st.border_width > 0.0f) {
            dl->AddRect(p_min, p_max, brd, 0.0f, 0, st.border_width);
        }
    }
}

// Banner de encabezado CSS para cualquier ventana de módulo (.module-header-banner)
inline void draw_module_banner(const char* code_badge, const char* title, const char* subtitle,
                               ImU32 accent_col = IM_COL32(62, 118, 188, 255))
{
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImVec2 p0 = ImGui::GetCursorScreenPos();
    float avail_w = ImGui::GetContentRegionAvail().x;
    if (avail_w < 80.0f) avail_w = 80.0f;
    const float h = 42.0f;
    ImVec2 p1(p0.x + avail_w, p0.y + h);

    const auto tk = tokens();
    dl->AddRectFilledMultiColor(p0, p1, tk.ribbon_bg_top, tk.ribbon_bg_top, tk.ribbon_bg_bot, tk.ribbon_bg_bot);
    dl->AddRect(p0, p1, tk.border_strong, 4.0f);
    dl->AddRectFilled(p0, ImVec2(p0.x + 5.0f, p1.y), accent_col, 4.0f);

    float x_cursor = p0.x + 12.0f;
    if (code_badge && code_badge[0] != '\0') {
        ImVec2 bts = ImGui::CalcTextSize(code_badge);
        ImVec2 bp0(x_cursor, p0.y + 9.0f);
        ImVec2 bp1(x_cursor + bts.x + 14.0f, p0.y + h - 9.0f);
        dl->AddRectFilled(ImVec2(bp0.x + 1.0f, bp0.y + 1.0f), ImVec2(bp1.x + 1.0f, bp1.y + 1.0f), IM_COL32(0, 0, 0, 45), 4.0f);
        dl->AddRectFilled(bp0, bp1, accent_col, 4.0f);
        dl->AddText(ImVec2(bp0.x + 7.0f, p0.y + (h - bts.y) * 0.5f), IM_COL32(255, 255, 255, 255), code_badge);
        x_cursor = bp1.x + 10.0f;
    }

    dl->AddText(ImVec2(x_cursor, p0.y + 5.0f), tk.text_primary, title);
    if (subtitle && subtitle[0] != '\0') {
        dl->AddText(ImVec2(x_cursor, p0.y + 22.0f), tk.text_secondary, subtitle);
    }

    ImGui::Dummy(ImVec2(avail_w, h + 4.0f));
}

// ── Iconografía Vectorial Multi-Capa Estilo HSC Chemistry / Outotec ─────────
inline void draw_ribbon_icon(ImDrawList* dl, ImVec2 center, float size, RibbonIcon icon) {
    const float r = size * 0.45f;
    const bool light = is_light_theme();
    const ImU32 label_fg = light ? IM_COL32(25, 30, 38, 255) : IM_COL32(235, 240, 248, 255);

    switch (icon) {
        case RibbonIcon::NewDoc: {
            ImVec2 a(center.x - r * 0.72f, center.y - r * 0.90f);
            ImVec2 b(center.x + r * 0.72f, center.y + r * 0.90f);
            dl->AddRectFilled(ImVec2(a.x + 1.5f, a.y + 1.5f), ImVec2(b.x + 1.5f, b.y + 1.5f), IM_COL32(0, 0, 0, 40), 2.0f);
            dl->AddRectFilled(a, b, IM_COL32(248, 250, 254, 255), 2.0f);
            dl->AddRect(a, b, IM_COL32(92, 118, 156, 255), 2.0f, 0, 1.4f);
            // Estrella / destello dorado arriba-izquierda (como en New de HSC Geo)
            ImVec2 sc(a.x + 2.0f, a.y + 3.0f);
            dl->AddCircleFilled(sc, r * 0.34f, IM_COL32(248, 198, 46, 255), 12);
            dl->AddCircle(sc, r * 0.34f, IM_COL32(196, 132, 18, 255), 12, 1.1f);
            break;
        }
        case RibbonIcon::Save: {
            ImVec2 a(center.x - r * 0.85f, center.y - r * 0.85f);
            ImVec2 b(center.x + r * 0.85f, center.y + r * 0.85f);
            dl->AddRectFilled(ImVec2(a.x + 1, a.y + 1), ImVec2(b.x + 1, b.y + 1), IM_COL32(0, 0, 0, 45), 2.5f);
            dl->AddRectFilled(a, b, IM_COL32(54, 96, 162, 255), 2.5f);
            dl->AddRect(a, b, IM_COL32(32, 62, 114, 255), 2.5f);
            dl->AddRectFilled(ImVec2(center.x - r * 0.52f, a.y + 1.0f), ImVec2(center.x + r * 0.52f, center.y - r * 0.18f),
                              IM_COL32(230, 236, 245, 255), 1.0f);
            dl->AddRectFilled(ImVec2(center.x - r * 0.62f, center.y + r * 0.12f),
                              ImVec2(center.x + r * 0.62f, b.y - 1.5f),
                              IM_COL32(250, 252, 255, 255), 1.0f);
            break;
        }
        case RibbonIcon::FolderOpen: {
            ImVec2 a(center.x - r * 0.92f, center.y - r * 0.55f);
            ImVec2 b(center.x + r * 0.92f, center.y + r * 0.82f);
            dl->AddRectFilled(ImVec2(a.x, a.y - 3.5f), ImVec2(center.x - r * 0.05f, a.y + 2.0f),
                              IM_COL32(214, 158, 42, 255), 2.0f);
            dl->AddRectFilledMultiColor(a, b,
                              IM_COL32(250, 212, 92, 255), IM_COL32(250, 212, 92, 255),
                              IM_COL32(224, 168, 46, 255), IM_COL32(224, 168, 46, 255));
            dl->AddRect(a, b, IM_COL32(158, 110, 22, 255), 2.0f, 0, 1.3f);
            break;
        }
        case RibbonIcon::WindowTileH: {
            ImVec2 a(center.x - r * 0.88f, center.y - r * 0.78f);
            ImVec2 b(center.x + r * 0.88f, center.y + r * 0.78f);
            dl->AddRectFilled(a, b, IM_COL32(235, 242, 252, 255), 2.0f);
            dl->AddRectFilled(a, ImVec2(b.x, a.y + 4.0f), IM_COL32(52, 112, 188, 255), 2.0f);
            dl->AddLine(ImVec2(a.x, center.y + 1.0f), ImVec2(b.x, center.y + 1.0f), IM_COL32(52, 112, 188, 255), 1.5f);
            dl->AddRect(a, b, IM_COL32(52, 112, 188, 255), 2.0f, 0, 1.3f);
            break;
        }
        case RibbonIcon::WindowTileV: {
            ImVec2 a(center.x - r * 0.88f, center.y - r * 0.78f);
            ImVec2 b(center.x + r * 0.88f, center.y + r * 0.78f);
            dl->AddRectFilled(a, b, IM_COL32(235, 242, 252, 255), 2.0f);
            dl->AddRectFilled(a, ImVec2(b.x, a.y + 4.0f), IM_COL32(52, 112, 188, 255), 2.0f);
            dl->AddLine(ImVec2(center.x, a.y + 4.0f), ImVec2(center.x, b.y), IM_COL32(52, 112, 188, 255), 1.5f);
            dl->AddRect(a, b, IM_COL32(52, 112, 188, 255), 2.0f, 0, 1.3f);
            break;
        }
        case RibbonIcon::WindowCascade: {
            ImVec2 a1(center.x - r * 0.85f, center.y - r * 0.75f);
            ImVec2 b1(center.x + r * 0.35f, center.y + r * 0.25f);
            ImVec2 a2(center.x - r * 0.35f, center.y - r * 0.25f);
            ImVec2 b2(center.x + r * 0.85f, center.y + r * 0.75f);
            dl->AddRectFilled(a1, b1, IM_COL32(215, 228, 246, 255), 1.5f);
            dl->AddRect(a1, b1, IM_COL32(62, 112, 178, 255), 1.5f);
            dl->AddRectFilled(a2, b2, IM_COL32(242, 247, 255, 255), 1.5f);
            dl->AddRectFilled(a2, ImVec2(b2.x, a2.y + 3.5f), IM_COL32(46, 104, 182, 255), 1.5f);
            dl->AddRect(a2, b2, IM_COL32(46, 104, 182, 255), 1.5f);
            break;
        }
        case RibbonIcon::MineralDB: {
            // Cilindro rojo de base de datos mineralógica (como en HSC Geo Features)
            ImVec2 a(center.x - r * 0.75f, center.y - r * 0.75f);
            ImVec2 b(center.x + r * 0.75f, center.y + r * 0.78f);
            dl->AddRectFilledMultiColor(a, b,
                IM_COL32(218, 62, 54, 255), IM_COL32(242, 112, 102, 255),
                IM_COL32(192, 42, 36, 255), IM_COL32(158, 28, 24, 255));
            dl->AddRect(a, b, IM_COL32(115, 20, 18, 255), 2.5f, 0, 1.4f);
            dl->AddLine(ImVec2(a.x + 1, center.y - r * 0.22f), ImVec2(b.x - 1, center.y - r * 0.22f), IM_COL32(255, 215, 210, 180), 1.3f);
            dl->AddLine(ImVec2(a.x + 1, center.y + r * 0.26f), ImVec2(b.x - 1, center.y + r * 0.26f), IM_COL32(255, 215, 210, 180), 1.3f);
            break;
        }
        case RibbonIcon::FindMineral: {
            // Lupa dorada/azul sobre tabla (Find mineral)
            ImVec2 c(center.x - r * 0.18f, center.y - r * 0.18f);
            dl->AddCircleFilled(c, r * 0.56f, IM_COL32(218, 238, 255, 255), 18);
            dl->AddCircle(c, r * 0.56f, IM_COL32(212, 146, 28, 255), 18, 2.2f);
            dl->AddLine(ImVec2(c.x + r * 0.38f, c.y + r * 0.38f),
                        ImVec2(center.x + r * 0.82f, center.y + r * 0.82f),
                        IM_COL32(168, 98, 22, 255), 3.0f);
            break;
        }
        case RibbonIcon::Crystal: {
            // Cristal / rombo amarillo-ámbar (Minerals)
            ImVec2 p1(center.x, center.y - r * 0.88f);
            ImVec2 p2(center.x + r * 0.78f, center.y);
            ImVec2 p3(center.x, center.y + r * 0.88f);
            ImVec2 p4(center.x - r * 0.78f, center.y);
            dl->AddQuadFilled(p1, p2, p3, p4, IM_COL32(245, 198, 46, 255));
            dl->AddTriangleFilled(p1, p2, center, IM_COL32(255, 228, 115, 255));
            dl->AddQuad(p1, p2, p3, p4, IM_COL32(142, 98, 12, 255), 1.5f);
            break;
        }
        case RibbonIcon::IdentifyQuestion: {
            // Círculo verde con '?' (Identify mineral en HSC Geo)
            dl->AddCircleFilled(center, r * 0.84f, IM_COL32(48, 158, 68, 255), 20);
            dl->AddCircle(center, r * 0.84f, IM_COL32(24, 98, 38, 255), 20, 1.5f);
            dl->AddText(ImVec2(center.x - 3.5f, center.y - 7.0f), IM_COL32(255, 255, 255, 255), "?");
            break;
        }
        case RibbonIcon::ModalBarChart: {
            // Gráfico de barras estadístico / modal (Modal calculations)
            ImVec2 base_l(center.x - r * 0.85f, center.y + r * 0.82f);
            ImVec2 base_r(center.x + r * 0.85f, center.y + r * 0.82f);
            dl->AddRectFilled(ImVec2(center.x - r * 0.75f, center.y - r * 0.15f),
                              ImVec2(center.x - r * 0.32f, base_l.y), IM_COL32(42, 52, 68, 255));
            dl->AddRectFilled(ImVec2(center.x - r * 0.18f, center.y - r * 0.82f),
                              ImVec2(center.x + r * 0.24f, base_l.y), IM_COL32(52, 122, 198, 255));
            dl->AddRectFilled(ImVec2(center.x + r * 0.36f, center.y - r * 0.45f),
                              ImVec2(center.x + r * 0.78f, base_l.y), IM_COL32(76, 164, 88, 255));
            dl->AddLine(base_l, base_r, label_fg, 1.5f);
            break;
        }
        case RibbonIcon::Liberation: {
            // Liberación mineral (partículas minerales multifásicas verde/magenta)
            dl->AddCircleFilled(ImVec2(center.x - r * 0.28f, center.y + r * 0.15f), r * 0.52f, IM_COL32(78, 168, 74, 255), 16);
            dl->AddCircleFilled(ImVec2(center.x + r * 0.28f, center.y - r * 0.22f), r * 0.46f, IM_COL32(192, 82, 168, 255), 16);
            dl->AddCircle(ImVec2(center.x - r * 0.28f, center.y + r * 0.15f), r * 0.52f, IM_COL32(38, 96, 36, 255), 16, 1.3f);
            dl->AddCircle(ImVec2(center.x + r * 0.28f, center.y - r * 0.22f), r * 0.46f, IM_COL32(118, 42, 102, 255), 16, 1.3f);
            break;
        }
        case RibbonIcon::PlusCircle: {
            dl->AddCircleFilled(center, r * 0.80f, IM_COL32(68, 168, 62, 255), 18);
            dl->AddCircle(center, r * 0.80f, IM_COL32(38, 108, 34, 255), 18, 1.3f);
            dl->AddLine(ImVec2(center.x - r * 0.42f, center.y), ImVec2(center.x + r * 0.42f, center.y), IM_COL32(255, 255, 255, 255), 2.2f);
            dl->AddLine(ImVec2(center.x, center.y - r * 0.42f), ImVec2(center.x, center.y + r * 0.42f), IM_COL32(255, 255, 255, 255), 2.2f);
            break;
        }
        case RibbonIcon::MinusCircle: {
            dl->AddCircleFilled(center, r * 0.80f, IM_COL32(214, 62, 52, 255), 18);
            dl->AddCircle(center, r * 0.80f, IM_COL32(142, 32, 26, 255), 18, 1.3f);
            dl->AddLine(ImVec2(center.x - r * 0.44f, center.y), ImVec2(center.x + r * 0.44f, center.y), IM_COL32(255, 255, 255, 255), 2.4f);
            break;
        }
        case RibbonIcon::Eraser: {
            ImVec2 p1(center.x - r * 0.25f, center.y - r * 0.65f);
            ImVec2 p2(center.x + r * 0.75f, center.y - r * 0.15f);
            ImVec2 p3(center.x + r * 0.25f, center.y + r * 0.65f);
            ImVec2 p4(center.x - r * 0.75f, center.y + r * 0.15f);
            dl->AddQuadFilled(p1, p2, p3, p4, IM_COL32(236, 156, 168, 255));
            dl->AddQuad(p1, p2, p3, p4, IM_COL32(168, 86, 98, 255), 1.4f);
            break;
        }
        case RibbonIcon::PlayArrow: {
            ImVec2 p1(center.x - r * 0.65f, center.y - r * 0.75f);
            ImVec2 p2(center.x - r * 0.65f, center.y + r * 0.75f);
            ImVec2 p3(center.x + r * 0.80f, center.y);
            dl->AddTriangleFilled(p1, p2, p3, IM_COL32(48, 164, 78, 255));
            dl->AddTriangle(p1, p2, p3, IM_COL32(26, 108, 46, 255), 1.4f);
            break;
        }
        case RibbonIcon::Sun: {
            dl->AddCircleFilled(center, r * 0.48f, IM_COL32(246, 186, 36, 255), 16);
            dl->AddCircle(center, r * 0.74f, IM_COL32(232, 156, 24, 200), 8, 1.6f);
            break;
        }
        case RibbonIcon::Moon: {
            dl->AddCircleFilled(center, r * 0.68f, IM_COL32(118, 168, 242, 255), 18);
            dl->AddCircleFilled(ImVec2(center.x + r * 0.26f, center.y - r * 0.20f), r * 0.54f,
                                light ? IM_COL32(236, 240, 246, 255) : IM_COL32(34, 40, 50, 255), 16);
            break;
        }
        case RibbonIcon::InputArrow:
        case RibbonIcon::ManualInput:
        case RibbonIcon::Indicator: {
            const ImU32 fill = IM_COL32(102, 156, 218, 255);
            const ImU32 border = IM_COL32(44, 88, 146, 255);
            dl->AddRectFilled(ImVec2(center.x - r * 0.35f, center.y - r * 0.9f),
                              ImVec2(center.x + r * 0.35f, center.y + r * 0.05f), fill);
            ImVec2 p1(center.x - r * 0.85f, center.y + r * 0.0f);
            ImVec2 p2(center.x + r * 0.85f, center.y + r * 0.0f);
            ImVec2 p3(center.x, center.y + r * 0.95f);
            dl->AddTriangleFilled(p1, p2, p3, fill);
            dl->AddTriangle(p1, p2, p3, border, 1.2f);
            if (icon == RibbonIcon::ManualInput) {
                dl->AddText(ImVec2(center.x + r * 0.25f, center.y + r * 0.15f), label_fg, "M");
            } else if (icon == RibbonIcon::Indicator) {
                dl->AddText(ImVec2(center.x + r * 0.35f, center.y + r * 0.15f), label_fg, "I");
            }
            break;
        }
        case RibbonIcon::OutputArrow:
        case RibbonIcon::ManualOutput: {
            const ImU32 fill = IM_COL32(102, 156, 218, 255);
            const ImU32 border = IM_COL32(44, 88, 146, 255);
            ImVec2 p1(center.x - r * 0.85f, center.y - r * 0.05f);
            ImVec2 p2(center.x + r * 0.85f, center.y - r * 0.05f);
            ImVec2 p3(center.x, center.y - r * 0.95f);
            dl->AddTriangleFilled(p1, p2, p3, fill);
            dl->AddTriangle(p1, p2, p3, border, 1.2f);
            dl->AddRectFilled(ImVec2(center.x - r * 0.35f, center.y - r * 0.05f),
                              ImVec2(center.x + r * 0.35f, center.y + r * 0.85f), fill);
            if (icon == RibbonIcon::ManualOutput) {
                dl->AddText(ImVec2(center.x + r * 0.25f, center.y + r * 0.15f), label_fg, "M");
            }
            break;
        }
        case RibbonIcon::Mapping: {
            dl->AddCircle(center, r * 0.78f, IM_COL32(76, 168, 82, 255), 20, 2.6f);
            dl->AddCircle(center, r * 0.42f, IM_COL32(230, 145, 45, 255), 16, 2.2f);
            break;
        }
        case RibbonIcon::Normalize: {
            dl->AddRectFilled(ImVec2(center.x - r * 0.85f, center.y - r * 0.2f),
                              ImVec2(center.x - r * 0.35f, center.y + r * 0.85f), IM_COL32(215, 85, 65, 255));
            dl->AddRectFilled(ImVec2(center.x - r * 0.22f, center.y - r * 0.85f),
                              ImVec2(center.x + r * 0.25f, center.y + r * 0.85f), IM_COL32(85, 165, 85, 255));
            dl->AddRectFilled(ImVec2(center.x + r * 0.38f, center.y - r * 0.5f),
                              ImVec2(center.x + r * 0.85f, center.y + r * 0.85f), IM_COL32(75, 130, 205, 255));
            break;
        }
        case RibbonIcon::Help: {
            dl->AddCircleFilled(center, r * 0.85f, IM_COL32(58, 126, 204, 255), 20);
            dl->AddCircle(center, r * 0.85f, IM_COL32(32, 82, 148, 255), 20, 1.4f);
            dl->AddText(ImVec2(center.x - 3.5f, center.y - 7.0f), IM_COL32(255, 255, 255, 255), "?");
            break;
        }
        case RibbonIcon::ToGaBi: {
            dl->AddCircleFilled(center, r * 0.82f, IM_COL32(62, 152, 96, 255), 20);
            dl->AddCircle(center, r * 0.82f, IM_COL32(35, 95, 58, 255), 20, 1.5f);
            dl->AddText(ImVec2(center.x - 4.5f, center.y - 7.0f), IM_COL32(255, 255, 255, 255), "G");
            break;
        }
        case RibbonIcon::ToExcel: {
            ImVec2 a(center.x - r * 0.85f, center.y - r * 0.85f);
            ImVec2 b(center.x + r * 0.85f, center.y + r * 0.85f);
            dl->AddRectFilled(a, b, IM_COL32(33, 115, 70, 255), 2.5f);
            dl->AddText(ImVec2(center.x - 4.0f, center.y - 7.0f), IM_COL32(255, 255, 255, 255), "X");
            break;
        }
        case RibbonIcon::AddRow: {
            dl->AddCircleFilled(center, r * 0.82f, IM_COL32(76, 156, 72, 255), 20);
            dl->AddLine(ImVec2(center.x - r * 0.45f, center.y), ImVec2(center.x + r * 0.45f, center.y), IM_COL32(255, 255, 255, 255), 2.2f);
            dl->AddLine(ImVec2(center.x, center.y - r * 0.45f), ImVec2(center.x, center.y + r * 0.45f), IM_COL32(255, 255, 255, 255), 2.2f);
            break;
        }
        case RibbonIcon::SubMaterial: {
            ImVec2 a(center.x - r * 0.85f, center.y - r * 0.75f);
            ImVec2 b(center.x + r * 0.85f, center.y + r * 0.75f);
            dl->AddRectFilled(a, b, IM_COL32(107, 123, 55, 255), 2.5f);
            dl->AddText(ImVec2(center.x - 6.5f, center.y - 7.0f), IM_COL32(255, 255, 255, 255), "[]");
            break;
        }
        case RibbonIcon::SendSim: {
            ImVec2 p1(center.x - r * 0.75f, center.y - r * 0.75f);
            ImVec2 p2(center.x - r * 0.75f, center.y + r * 0.75f);
            ImVec2 p3(center.x + r * 0.85f, center.y);
            dl->AddTriangleFilled(p1, p2, p3, IM_COL32(52, 132, 210, 255));
            break;
        }
        case RibbonIcon::BalanceScale: {
            dl->AddLine(ImVec2(center.x - r * 0.8f, center.y - r * 0.2f),
                        ImVec2(center.x + r * 0.8f, center.y - r * 0.2f), IM_COL32(215, 140, 45, 255), 2.2f);
            dl->AddTriangleFilled(ImVec2(center.x - r * 0.4f, center.y + r * 0.8f),
                                  ImVec2(center.x + r * 0.4f, center.y + r * 0.8f),
                                  ImVec2(center.x, center.y - r * 0.2f), IM_COL32(105, 115, 130, 255));
            break;
        }
        case RibbonIcon::ResetDB:
        case RibbonIcon::Duplicate: {
            dl->AddRect(ImVec2(center.x - r * 0.75f, center.y - r * 0.75f),
                        ImVec2(center.x + r * 0.35f, center.y + r * 0.35f), IM_COL32(75, 115, 170, 255), 1.5f, 0, 1.8f);
            dl->AddRectFilled(ImVec2(center.x - r * 0.35f, center.y - r * 0.35f),
                              ImVec2(center.x + r * 0.75f, center.y + r * 0.75f), IM_COL32(115, 158, 215, 255), 1.5f);
            break;
        }
        case RibbonIcon::Trash: {
            dl->AddRectFilled(ImVec2(center.x - r * 0.6f, center.y - r * 0.45f),
                              ImVec2(center.x + r * 0.6f, center.y + r * 0.8f), IM_COL32(195, 72, 68, 255), 1.5f);
            dl->AddLine(ImVec2(center.x - r * 0.8f, center.y - r * 0.55f),
                        ImVec2(center.x + r * 0.8f, center.y - r * 0.55f), IM_COL32(195, 72, 68, 255), 2.2f);
            break;
        }
    }
}

// Botón vertical de cinta (Ribbon Button) con icono vectorial arriba y etiqueta centrada abajo (100% adaptable a Claro/Oscuro)
inline bool ribbon_button(const char* id, const char* label_line1, const char* label_line2,
                          RibbonIcon icon, float width = 54.0f, float height = 56.0f)
{
    ImGui::PushID(id);
    ImVec2 pos = ImGui::GetCursorScreenPos();
    ImVec2 sz(width, height);
    bool pressed = ImGui::InvisibleButton("##rb", sz);
    bool hovered = ImGui::IsItemHovered();
    bool active  = ImGui::IsItemActive();

    ImDrawList* dl = ImGui::GetWindowDrawList();
    BoxStyle st = BoxStyle::from_theme();
    st.border_radius = 3.5f;
    if (hovered || active) {
        draw_css_box(dl, pos, ImVec2(pos.x + sz.x, pos.y + sz.y), st, hovered, active);
    }

    const auto tk = tokens();
    const float icon_y = (label_line2 && label_line2[0] != '\0') ? (pos.y + 15.0f) : (pos.y + 18.0f);
    draw_ribbon_icon(dl, ImVec2(pos.x + sz.x * 0.5f, icon_y), 22.0f, icon);

    ImVec2 ts1 = ImGui::CalcTextSize(label_line1);
    const float y1 = (label_line2 && label_line2[0] != '\0') ? (pos.y + 30.0f) : (pos.y + 35.0f);
    dl->AddText(ImVec2(pos.x + (sz.x - ts1.x) * 0.5f, y1), tk.text_primary, label_line1);

    if (label_line2 && label_line2[0] != '\0') {
        ImVec2 ts2 = ImGui::CalcTextSize(label_line2);
        dl->AddText(ImVec2(pos.x + (sz.x - ts2.x) * 0.5f, pos.y + 42.0f), tk.text_secondary, label_line2);
    }

    ImGui::PopID();
    return pressed;
}

// Botón horizontal con icono a la izquierda (para paneles de navegación como "Features" en HSC Geo)
inline bool feature_nav_button(const char* id, const char* label, RibbonIcon icon, bool selected = false, float height = 30.0f) {
    ImGui::PushID(id);
    float avail_w = ImGui::GetContentRegionAvail().x;
    if (avail_w < 90.0f) avail_w = 90.0f;
    ImVec2 pos = ImGui::GetCursorScreenPos();
    ImVec2 sz(avail_w, height);
    bool pressed = ImGui::InvisibleButton("##fnav", sz);
    bool hovered = ImGui::IsItemHovered();
    bool active  = ImGui::IsItemActive();

    ImDrawList* dl = ImGui::GetWindowDrawList();
    BoxStyle st = BoxStyle::from_theme();
    st.border_radius = 3.5f;
    draw_css_box(dl, pos, ImVec2(pos.x + sz.x, pos.y + sz.y), st, hovered, active || selected);

    const auto tk = tokens();
    if (selected) {
        dl->AddRectFilled(pos, ImVec2(pos.x + 4.0f, pos.y + sz.y), tk.accent_blue, 3.5f);
    }
    draw_ribbon_icon(dl, ImVec2(pos.x + 16.0f, pos.y + sz.y * 0.5f), 19.0f, icon);
    ImVec2 ts = ImGui::CalcTextSize(label);
    dl->AddText(ImVec2(pos.x + 30.0f, pos.y + (sz.y - ts.y) * 0.5f), tk.text_primary, label);

    ImGui::PopID();
    return pressed;
}

inline void begin_ribbon_bar(const char* id, float height = 78.0f) {
    const auto tk = tokens();
    ImGui::PushStyleColor(ImGuiCol_ChildBg, tk.ribbon_bg_bot);
    ImGui::PushStyleColor(ImGuiCol_Border, tk.border_strong);
    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(8.0f, 4.0f));
    ImGui::BeginChild(id, ImVec2(0.0f, height), true, ImGuiWindowFlags_NoScrollbar);
}

inline void end_ribbon_bar() {
    ImGui::EndChild();
    ImGui::PopStyleVar();
    ImGui::PopStyleColor(2);
}

inline void draw_ribbon_group_footer(float group_start_x, float group_end_x, float bar_bottom_y, const char* group_title) {
    const auto tk = tokens();
    ImDrawList* dl = ImGui::GetWindowDrawList();
    dl->AddLine(ImVec2(group_start_x + 2.0f, bar_bottom_y - 16.0f),
                ImVec2(group_end_x - 2.0f, bar_bottom_y - 16.0f),
                tk.border_soft, 1.0f);
    ImVec2 ts = ImGui::CalcTextSize(group_title);
    float tx = group_start_x + ((group_end_x - group_start_x) - ts.x) * 0.5f;
    dl->AddText(ImVec2(tx, bar_bottom_y - 14.5f), tk.text_secondary, group_title);
    dl->AddLine(ImVec2(group_end_x + 3.0f, bar_bottom_y - 72.0f),
                ImVec2(group_end_x + 3.0f, bar_bottom_y - 4.0f),
                tk.border_soft, 1.0f);
}

// Píldora de categoría estilo CSS (.badge-pill)
inline bool category_pill(const char* label, bool selected, ImU32 accent_col = IM_COL32(107, 123, 55, 255)) {
    ImGui::PushID(label);
    ImVec2 ts = ImGui::CalcTextSize(label);
    ImVec2 sz(ts.x + 18.0f, 22.0f);
    ImVec2 pos = ImGui::GetCursorScreenPos();
    bool clicked = ImGui::InvisibleButton("##pill", sz);
    bool hovered = ImGui::IsItemHovered();

    const bool light = is_light_theme();
    ImDrawList* dl = ImGui::GetWindowDrawList();
    ImU32 bg = selected ? accent_col
                        : (hovered ? (light ? IM_COL32(222, 228, 212, 255) : IM_COL32(55, 68, 84, 255))
                                   : (light ? IM_COL32(236, 239, 243, 255) : IM_COL32(38, 44, 54, 255)));
    ImU32 fg = selected ? IM_COL32(255, 255, 255, 255)
                        : (light ? IM_COL32(45, 52, 62, 255) : IM_COL32(220, 226, 235, 255));
    ImU32 bd = selected ? accent_col
                        : (light ? IM_COL32(192, 198, 208, 255) : IM_COL32(68, 76, 90, 255));

    dl->AddRectFilled(pos, ImVec2(pos.x + sz.x, pos.y + sz.y), bg, 11.0f);
    dl->AddRect(pos, ImVec2(pos.x + sz.x, pos.y + sz.y), bd, 11.0f);
    dl->AddText(ImVec2(pos.x + 9.0f, pos.y + (sz.y - ts.y) * 0.5f), fg, label);
    ImGui::PopID();
    return clicked;
}

// Barra apilada horizontal estilo CSS (.stacked-progress-bar)
struct StackedSegment {
    std::string label;
    double pct = 0.0;
};

inline void draw_stacked_composition_bar(const char* title, const std::vector<StackedSegment>& segs, float height = 22.0f) {
    static const ImU32 kPalette[] = {
        IM_COL32(107, 123, 55, 255),
        IM_COL32(66, 126, 176, 255),
        IM_COL32(208, 124, 52, 255),
        IM_COL32(142, 92, 168, 255),
        IM_COL32(58, 158, 136, 255),
        IM_COL32(196, 78, 78, 255),
        IM_COL32(128, 148, 72, 255),
        IM_COL32(92, 112, 148, 255),
        IM_COL32(212, 162, 58, 255),
        IM_COL32(118, 122, 128, 255)
    };
    constexpr int n_pal = static_cast<int>(sizeof(kPalette) / sizeof(kPalette[0]));

    const auto tk = tokens();
    ImGui::TextColored(ImGui::ColorConvertU32ToFloat4(tk.text_secondary), "%s", title);
    ImVec2 p0 = ImGui::GetCursorScreenPos();
    float avail_w = ImGui::GetContentRegionAvail().x;
    if (avail_w < 50.0f) avail_w = 50.0f;
    ImVec2 p1(p0.x + avail_w, p0.y + height);

    ImDrawList* dl = ImGui::GetWindowDrawList();
    dl->AddRectFilled(p0, p1, tk.light ? IM_COL32(228, 231, 236, 255) : IM_COL32(32, 37, 46, 255), 4.0f);

    float cur_x = p0.x;
    for (size_t i = 0; i < segs.size(); ++i) {
        if (segs[i].pct <= 0.05) continue;
        float w = static_cast<float>(segs[i].pct / 100.0) * avail_w;
        if (cur_x + w > p1.x) w = p1.x - cur_x;
        if (w <= 0.5f) continue;

        ImU32 col = kPalette[i % n_pal];
        dl->AddRectFilled(ImVec2(cur_x, p0.y), ImVec2(cur_x + w, p1.y), col, 0.0f);
        if (w > 46.0f) {
            char buf[32];
            std::snprintf(buf, sizeof(buf), "%s %.1f%%", segs[i].label.c_str(), segs[i].pct);
            ImVec2 ts = ImGui::CalcTextSize(buf);
            if (ts.x + 6.0f < w) {
                dl->AddText(ImVec2(cur_x + (w - ts.x) * 0.5f, p0.y + (height - ts.y) * 0.5f),
                            IM_COL32(255, 255, 255, 255), buf);
            }
        }
        cur_x += w;
    }
    dl->AddRect(p0, p1, tk.border_strong, 4.0f);
    ImGui::Dummy(ImVec2(avail_w, height + 2.0f));
}

// Scoped HSC Theme que respeta el Modo Claro o Modo Oscuro activo del usuario
class ScopedHSCTheme {
public:
    ScopedHSCTheme() {
        ImGui::PushStyleVar(ImGuiStyleVar_FrameRounding, 3.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_ChildRounding, 4.0f);
        ImGui::PushStyleVar(ImGuiStyleVar_FrameBorderSize, 1.0f);
    }

    ~ScopedHSCTheme() {
        ImGui::PopStyleVar(3);
    }
};

}}} // namespace cma::interfaz::css

#endif // CMA_INTERFAZ_UI_CSS_STYLE_HPP
