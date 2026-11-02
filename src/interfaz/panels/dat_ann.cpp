#include "shared.hpp"
#include "motores/red_neuronal.hpp"
#include "adapters/desktop/process.hpp"
#include <implot.h>
#include <sstream>
#include <fstream>
#include <algorithm>
#include <cmath>
#include <cfloat>
#include <cctype>
#include <random>

namespace cma { namespace interfaz {
// ═══════════════════════════════════════════════════════════════════════════
// Dat - Data: entrenamiento de redes neuronales (FFNN / PSO / RNN aleatoria)
// Interfaz basada en el módulo Data de HSC Chemistry
// ═══════════════════════════════════════════════════════════════════════════

// Letra(s) de columna estilo Excel: 0→A, 25→Z, 26→AA, 27→AB...
static void nombre_columna_excel(int idx, char* out, size_t cap) {
    std::string s;
    int n = idx + 1;   // 1-based
    while (n > 0) {
        int rem = (n - 1) % 26;
        s.insert(s.begin(), char('A' + rem));
        n = (n - 1) / 26;
    }
    snprintf(out, cap, "%s", s.c_str());
}

PanelDatosANN::~PanelDatosANN() {
    cancelar_.store(true);
    if (hilo_.joinable()) hilo_.join();
}

void PanelDatosANN::cargar_ejemplo_hsc() {
    // Replica el benchmark de HSC Data:
    // Y = X1^2 + 2*X2^2 + 3*X3^2 + 4*X4^2 + 5*X5^2 + 6*X6^2 + 7*X7^2 + 8*X8^2
    // con 8 entradas X1..X8 en [-1000, 1000] y 200 muestras.
    datos_.clear();
    ynn_.clear();
    error_.clear();
    std::mt19937 rng(42);
    std::uniform_real_distribution<double> dist(-1000.0, 1000.0);

    const int n_rows = 200;
    datos_.reserve(n_rows);
    for (int r = 0; r < n_rows; ++r) {
        std::vector<double> fila(9, 0.0);
        double y = 0.0;
        for (int k = 0; k < 8; ++k) {
            double xk = dist(rng);
            fila[k] = xk;
            y += (double)(k + 1) * xk * xk;
        }
        fila[8] = y;
        datos_.push_back(std::move(fila));
    }
    regla_ = {0, 0, 0, 0, 0, 0, 0, 0, 1};
    strncpy(capas_buf_, "16;16", sizeof(capas_buf_) - 1);
    capas_buf_[sizeof(capas_buf_) - 1] = '\0';
    tasa_ = 0.02;
    decaimiento_ = 1e-6;
    momento_ = 0.1;
    act_oculta_ = 0; // TanH
    act_salida_ = 3; // Linear
    particion_val_ = 20;
    batch_size_ = 1;
    rmse_objetivo_ = 1e-5;
    max_epocas_ = 600;
    aplicar_datos();
}

void PanelDatosANN::aplicar_datos() {
    int nx = (int)std::count(regla_.begin(), regla_.end(), 0);
    int ny = (int)std::count(regla_.begin(), regla_.end(), 1);
    if (nx <= 0 || ny <= 0) {
        error_ = "Define al menos una columna X y una columna Y en la fila RULE.";
        return;
    }
    std::vector<int> ocultas;
    for (auto& tok : split_str(capas_buf_, ";")) {
        std::string t = trim_str(tok);
        if (t.empty()) continue;
        try { ocultas.push_back(std::max(1, std::stoi(t))); } catch (...) {}
    }
    motores::ConfigANN cfg;
    cfg.ocultas = ocultas.empty() ? std::vector<int>{16, 16} : ocultas;
    cfg.act_oculta = act_oculta_;
    cfg.act_salida = act_salida_;
    cfg.tasa = tasa_;
    cfg.decaimiento = decaimiento_;
    cfg.momento = momento_;
    cfg.batch_size = std::max(1, batch_size_);
    cfg.tipo = tipo_entrenamiento_;
    cfg.particulas_pso = std::max(10, particulas_pso_);
    red_.inicializar(nx, cfg, ny);
    error_.clear();
}

void PanelDatosANN::importar_csv(const std::string& ruta) {
    std::ifstream f(ruta);
    if (!f) { error_ = "No se pudo abrir: " + ruta; return; }
    std::vector<std::vector<double>> nuevo;
    std::vector<int> regla_csv;
    std::string linea;
    while (std::getline(f, linea)) {
        // limpiar BOM y \r
        if (!linea.empty() && (linea[0] == '\xEF' || linea[0] == '\r')) {
            size_t p = linea.find_first_not_of("\xEF\xBB\xBF\r");
            if (p == std::string::npos) continue;
            linea = linea.substr(p);
        }
        std::string l = trim_str(linea);
        if (l.empty()) continue;
        auto celdas = split_str(l, linea.find(';') != std::string::npos ? ";" : ",");
        // Detectar fila RULE exportada previamente
        if (!celdas.empty() && trim_str(celdas[0]) == "RULE") {
            regla_csv.clear();
            for (size_t k = 1; k < celdas.size(); ++k) {
                std::string tok = trim_str(celdas[k]);
                if (tok == "X") regla_csv.push_back(0);
                else if (tok == "Y") regla_csv.push_back(1);
            }
            continue;
        }
        std::vector<double> fila;
        bool numerica = true;
        for (auto& c : celdas) {
            std::string t = trim_str(c);
            if (t.empty()) { fila.push_back(0.0); continue; }
            try { fila.push_back(std::stod(t)); }
            catch (...) { numerica = false; break; }
        }
        if (numerica && !fila.empty()) nuevo.push_back(fila);
    }
    if (nuevo.empty()) { error_ = "El archivo no contiene filas numéricas."; return; }
    // Normalizar: todas las filas al mismo nº de columnas (evita accesos fuera de rango)
    size_t ncols = 0;
    for (auto& f_row : nuevo) ncols = std::max(ncols, f_row.size());
    for (auto& f_row : nuevo) f_row.resize(ncols, 0.0);
    datos_ = std::move(nuevo);
    if (!regla_csv.empty() && regla_csv.size() <= ncols) {
        regla_ = regla_csv;
        regla_.resize(ncols, 1);
    } else {
        // Regla por defecto: todas X excepto la última que es Y
        regla_.assign(datos_[0].size(), 0);
        if (!regla_.empty()) regla_.back() = 1;
    }
    ynn_.clear();
    error_.clear();
}

void PanelDatosANN::exportar_csv(const std::string& ruta) {
    FILE* f = fopen(ruta.c_str(), "w");
    if (!f) { error_ = "No se pudo escribir: " + ruta; return; }
    bool con_ynn = !ynn_.empty() && ynn_.size() == datos_.size();
    fprintf(f, "RULE,");
    for (size_t c = 0; c < regla_.size(); ++c)
        fprintf(f, "%s%s", c ? "," : "", regla_[c] == 0 ? "X" : "Y");
    if (con_ynn) for (size_t c = 0; c < regla_.size(); ++c)
        if (regla_[c] == 1) fprintf(f, ",YNN");
    fprintf(f, "\n");
    for (size_t r = 0; r < datos_.size(); ++r) {
        fprintf(f, "%zu", r + 1);
        for (double v : datos_[r]) fprintf(f, ",%.6g", v);
        if (con_ynn) {
            int k2 = 0;
            for (size_t c = 0; c < regla_.size(); ++c) {
                if (regla_[c] != 1) continue;
                double v = (k2 < (int)ynn_[r].size()) ? ynn_[r][k2] : 0.0;
                fprintf(f, ",%.6g", v);
                ++k2;
            }
        }
        fprintf(f, "\n");
    }
    fclose(f);
}

void PanelDatosANN::calcular_ynn() {
    if (!red_.lista() || datos_.empty()) return;
    ynn_.assign(datos_.size(), {});
    for (size_t r = 0; r < datos_.size(); ++r) {
        std::vector<double> x;
        for (size_t c = 0; c < regla_.size() && c < datos_[r].size(); ++c)
            if (regla_[c] == 0) x.push_back(datos_[r][c]);
        if ((int)x.size() != red_.n_entrada()) { ynn_[r] = {}; continue; }
        ynn_[r] = red_.predecir(x);
    }
}

void PanelDatosANN::lanzar_entrenamiento() {
    // Construir X, Y según la regla
    std::vector<std::vector<double>> X, Y;
    std::vector<size_t> cols_x, cols_y;
    for (size_t c = 0; c < regla_.size(); ++c)
        (regla_[c] == 0 ? cols_x : cols_y).push_back(c);
    if (cols_x.empty() || cols_y.empty()) {
        error_ = "Define al menos una columna X y una columna Y en la fila RULE.";
        return;
    }
    for (auto& fila : datos_) {
        std::vector<double> x, y;
        for (size_t c : cols_x) x.push_back(fila[c]);
        for (size_t c : cols_y) y.push_back(fila[c]);
        X.push_back(std::move(x));
        Y.push_back(std::move(y));
    }
    if (X.size() < 4) { error_ = "Se necesitan al menos 4 filas de datos."; return; }

    // Parsear capas ocultas "16;16"
    std::vector<int> ocultas;
    for (auto& tok : split_str(capas_buf_, ";")) {
        std::string t = trim_str(tok);
        if (t.empty()) continue;
        try { ocultas.push_back(std::max(1, std::stoi(t))); } catch (...) {}
    }

    // Partición entrenamiento/validación (n >= 4 garantizado por la validación previa)
    int n = (int)X.size();
    int n_val = std::clamp((int)((double)n * particion_val_ / 100.0), 1, n - 2);
    std::vector<int> idx_ent, idx_val;
    for (int i = 0; i < n; ++i)
        (i < n - n_val ? idx_ent : idx_val).push_back(i);

    motores::ConfigANN cfg;
    cfg.ocultas = ocultas.empty() ? std::vector<int>{8} : ocultas;
    cfg.act_oculta = act_oculta_;
    cfg.act_salida = act_salida_;
    cfg.tasa = tasa_;
    cfg.decaimiento = decaimiento_;
    cfg.momento = momento_;
    cfg.batch_size = std::max(1, batch_size_);
    cfg.tipo = tipo_entrenamiento_;
    cfg.particulas_pso = std::max(10, particulas_pso_);

    int nx = (int)cols_x.size(), ny = (int)cols_y.size();
    red_.inicializar(nx, cfg, ny);

    error_.clear();
    calculando_.store(true);
    listo_.store(false);
    cancelar_.store(false);
    if (hilo_.joinable()) hilo_.join();

    auto Xc = X, Yc = Y;
    auto ent = idx_ent, val = idx_val;
    double obj = rmse_objetivo_;
    int maxep = max_epocas_;
    hilo_ = std::thread([this, Xc, Yc, ent, val, obj, maxep]() {
        auto r = red_.entrenar(Xc, Yc, ent, val, obj, maxep,
            [this](float p) { progreso_.store(p); }, cancelar_);
        resultado_ = std::move(r);
        listo_.store(true);
    });
}

void PanelDatosANN::dibujar_red(const ImVec2& tam) {
    if (!red_.lista()) { ImGui::TextDisabled(tr("Presiona 'Apply Data' o 'Learn' para visualizar la arquitectura de la red.")); return; }
    auto topo = red_.topologia();
    if (topo.size() < 2) return;

    ImVec2 canvas_sz(std::max(tam.x, 120.0f), std::max(tam.y, 120.0f));
    ImVec2 p0 = ImGui::GetCursorScreenPos();
    ImVec2 p1(p0.x + canvas_sz.x, p0.y + canvas_sz.y);
    ImGui::InvisibleButton("##nn_canvas", canvas_sz);

    ImDrawList* draw = ImGui::GetWindowDrawList();
    // Fondo gris claro estilo HSC View NN Picture
    draw->AddRectFilled(p0, p1, IM_COL32(230, 230, 230, 255), 4.0f);
    draw->AddRect(p0, p1, IM_COL32(160, 160, 160, 255), 4.0f);

    int L = (int)topo.size();
    float pad_x = 46.0f;
    float pad_y = 26.0f;
    float usable_w = std::max(40.0f, canvas_sz.x - 2.0f * pad_x);
    float usable_h = std::max(40.0f, canvas_sz.y - 2.0f * pad_y);
    float paso_x = usable_w / (float)std::max(L - 1, 1);

    // En HSC, cada capa distribuye sus neuronas uniformemente a lo largo del alto disponible
    auto pos_nodo = [&](int capa, int i) -> ImVec2 {
        int n = topo[capa];
        float x = p0.x + pad_x + (float)capa * paso_x;
        float y = (n <= 1)
            ? (p0.y + canvas_sz.y * 0.5f)
            : (p0.y + pad_y + (float)i * (usable_h / (float)(n - 1)));
        return ImVec2(x, y);
    };

    // Conexiones en verde oscuro estilo HSC
    ImU32 col_linea = IM_COL32(15, 115, 15, 175);
    long total_lineas = 0;
    for (int l = 0; l + 1 < L; ++l) total_lineas += (long)topo[l] * topo[l + 1];
    bool dibujar_todas = total_lineas <= 4000;

    for (int l = 0; l + 1 < L; ++l) {
        for (int i = 0; i < topo[l]; ++i) {
            int paso = dibujar_todas ? 1 : std::max(1, topo[l] / 12);
            if (i % paso != 0) continue;
            for (int j = 0; j < topo[l + 1]; ++j) {
                int paso2 = dibujar_todas ? 1 : std::max(1, topo[l + 1] / 12);
                if (j % paso2 != 0) continue;
                draw->AddLine(pos_nodo(l, i), pos_nodo(l + 1, j), col_linea, 1.15f);
            }
        }
    }

    // Nodos: entrada (salmón/rosa), oculta (dorado/naranja), salida (cian claro) como en HSC
    for (int l = 0; l < L; ++l) {
        ImU32 col = (l == 0) ? IM_COL32(236, 112, 108, 255)      // entrada (salmón)
                  : (l == L - 1) ? IM_COL32(155, 220, 215, 255)  // salida (cian claro)
                  : IM_COL32(218, 155, 22, 255);                 // oculta (dorado)
        float radio = (topo[l] > 24) ? 5.5f : 8.0f;
        for (int i = 0; i < topo[l]; ++i) {
            ImVec2 c = pos_nodo(l, i);
            draw->AddCircleFilled(c, radio, col);
            draw->AddCircle(c, radio, IM_COL32(45, 45, 45, 255), 0, 1.2f);
        }
    }
}

void PanelDatosANN::renderizar() {
    // ── Resultado del hilo de entrenamiento ──
    if (listo_.load()) {
        if (hilo_.joinable()) hilo_.join();
        resultado_ui_ = resultado_;   // copia estable: la UI solo lee resultado_ui_
        calculando_.store(false);
        listo_.store(false);
        calcular_ynn();  // llena YNN y habilita los gráficos
    }

    // Cargar el ejemplo inicial la primera vez (benchmark HSC de 8 entradas)
    if (datos_.empty() && !ejemplo_cargado_) {
        cargar_ejemplo_hsc();
        ejemplo_cargado_ = true;
    }

    // ── Toolbar estilo cinta HSC (Common | Training | Neural Network | Tools) ──
    if (ImGui::Button(tr("New"))) {
        ImGui::OpenPopup("popup_new_ann");
    }
    if (ImGui::BeginPopup("popup_new_ann")) {
        if (ImGui::MenuItem(tr("Hoja vacía (3 columnas)"))) {
            datos_.assign(8, std::vector<double>(3, 0.0));
            regla_ = {0, 0, 1};
            ynn_.clear();
            resultado_ = {}; resultado_ui_ = {}; error_.clear();
        }
        if (ImGui::MenuItem(tr("Benchmark HSC (Y = Σ k·Xk², 8 entradas)"))) {
            cargar_ejemplo_hsc();
        }
        ImGui::EndPopup();
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Open"))) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Import Data CSV", "*.csv", false);
        if (!path.empty()) importar_csv(path);
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Save")) && !datos_.empty()) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Save Data CSV", "*.csv", true);
        if (!path.empty()) {
            if (path.find(".csv") == std::string::npos) path += ".csv";
            exportar_csv(path);
        }
    }
    ImGui::SameLine();
    ImGui::TextDisabled("|"); ImGui::SameLine();

    bool ocupado = calculando_.load();
    if (ocupado) ImGui::BeginDisabled();
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.22f, 0.55f, 0.22f, 1.0f));
    if (ImGui::Button(tr("Apply Data"))) aplicar_datos();
    ImGui::PopStyleColor();
    if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Valida las columnas X/Y e inicializa la estructura de la red"));
    ImGui::SameLine();
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.28f, 0.62f, 0.18f, 1.0f));
    if (ImGui::Button(tr("Learn"))) lanzar_entrenamiento();
    ImGui::PopStyleColor();
    if (ocupado) ImGui::EndDisabled();
    if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Entrenar la red neuronal (FFNN / PSO / RNN)"));
    ImGui::SameLine();
    if (!ocupado) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Stop"))) cancelar_.store(true);
    if (!ocupado) ImGui::EndDisabled();

    ImGui::SameLine();
    ImGui::TextDisabled("|"); ImGui::SameLine();

    if (!red_.lista() || ocupado) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Calculate NN"))) calcular_ynn();
    if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Calcula las predicciones YNN para todas las filas de la hoja"));
    if (!red_.lista() || ocupado) ImGui::EndDisabled();
    ImGui::SameLine();
    if (ImGui::Button(tr("View NN Picture"))) {
        if (!red_.lista()) aplicar_datos();
        ver_red_ = true;
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("View NN XML File"))) {
        if (!red_.lista()) aplicar_datos();
        ver_xml_ = true;
    }
    ImGui::SameLine();
    if (!red_.lista() || ocupado) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Export"))) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Export Neural Network", "*.json", true);
        if (!path.empty()) {
            if (path.find(".json") == std::string::npos && path.find(".xml") == std::string::npos)
                path += ".json";
            FILE* f = fopen(path.c_str(), "w");
            if (f) {
                if (path.size() >= 4 && path.substr(path.size() - 4) == ".xml")
                    fprintf(f, "%s", red_.a_xml().c_str());
                else
                    fprintf(f, "%s", red_.a_json().c_str());
                fclose(f);
            }
        }
    }
    if (!red_.lista() || ocupado) ImGui::EndDisabled();
    ImGui::SameLine();
    if (ocupado) ImGui::BeginDisabled();
    if (ImGui::Button(tr("Import NN"))) {
        adapters::DesktopFileDialog dlg;
        std::string path = dlg.select("Import Neural Network", "*.json", false);
        if (!path.empty()) {
            std::ifstream f(path);
            if (f) {
                std::string content((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
                if (red_.desde_json(content)) {
                    calcular_ynn();
                    error_.clear();
                } else {
                    error_ = "Archivo de red neuronal inválido.";
                }
            }
        }
    }
    if (ocupado) ImGui::EndDisabled();

    // ── Fila 2 de la cinta: selector de arquitectura (FFNN / PSO / RNN) + Toggles Tools ──
    const char* tipos[] = {"FFNN (Feedforward Backprop)", "PSO (Particle Swarm Optimizer)", "RNN (Random Neural Network)"};
    ImGui::SetNextItemWidth(220);
    if (ImGui::Combo("##tipo_ann", &tipo_entrenamiento_, tipos, 3)) {
        prop_activa_ = 9;
    }
    ImGui::SameLine();
    ImGui::TextDisabled("| Tools:"); ImGui::SameLine();
    ImGui::Checkbox(tr("Properties"), &ver_propiedades_);
    ImGui::SameLine();
    ImGui::Checkbox(tr("Performance"), &ver_rendimiento_);
    ImGui::SameLine();
    ImGui::Checkbox(tr("Parity Chart"), &ver_paridad_);
    ImGui::SameLine();
    ImGui::Checkbox(tr("Compare Values"), &ver_comparar_);

    if (ocupado) {
        ImGui::SameLine();
        ImGui::ProgressBar(progreso_.load(), ImVec2(140, 18));
    }

    if (!error_.empty())
        ImGui::TextColored(cma::interfaz::AdaptiveColor(1, 0.4f, 0.4f, 1), "%s", error_.c_str());

    ImGui::Separator();

    // ── Layout (como HSC): [Properties | Hoja] arriba + [Gráficos] abajo ──
    bool hay_graficos = red_.lista() && !ynn_.empty()
        && ynn_.size() == datos_.size()
        && !resultado_ui_.epoca_hist.empty();
    float zona_graficos_h = 0.0f;
    if (hay_graficos && (ver_paridad_ || ver_rendimiento_ || ver_comparar_)) {
        zona_graficos_h = 255.0f;
    }
    float area_h = ImGui::GetContentRegionAvail().y - zona_graficos_h;
    area_h = std::max(area_h, 80.0f);

    if (ver_propiedades_) {
        ImGui::Columns(2, "dat_split", false);
        static bool init_w = true;
        if (init_w) { ImGui::SetColumnWidth(0, 275.0f); init_w = false; }

        // ══ Properties (como el panel izquierdo de HSC) ══
        ImGui::BeginChild("dat_props", ImVec2(0, area_h), true);
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.55f, 0.6f, 0.75f, 1), "Properties");
        ImGui::Separator();
        float prop_scroll_h = std::max(60.0f, ImGui::GetContentRegionAvail().y - 95.0f);
        ImGui::BeginChild("dat_props_inner", ImVec2(0, prop_scroll_h), false);

        if (ImGui::CollapsingHeader("Hyperparameters", ImGuiTreeNodeFlags_DefaultOpen)) {
            ImGui::PushItemWidth(-95);
            if (ImGui::InputDouble(tr("Learning Rate"), &tasa_, 0, 0, "%.4g")) prop_activa_ = 0;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 0;
            if (ImGui::InputDouble(tr("LR Decay"), &decaimiento_, 0, 0, "%.1e")) prop_activa_ = 1;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 1;
            if (ImGui::InputDouble(tr("Momentum"), &momento_, 0, 0, "%.3g")) prop_activa_ = 2;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 2;
            const char* acts[] = {"TanH", "Sigmoid", "ReLU", "Linear"};
            if (ImGui::Combo(tr("Hidden Act."), &act_oculta_, acts, 4)) prop_activa_ = 3;
            if (ImGui::IsItemHovered()) prop_activa_ = 3;
            if (ImGui::Combo(tr("Output Act."), &act_salida_, acts, 4)) prop_activa_ = 3;
            if (tipo_entrenamiento_ == 1) {
                if (ImGui::InputInt(tr("PSO Swarm"), &particulas_pso_))
                    particulas_pso_ = std::clamp(particulas_pso_, 10, 500);
            }
            ImGui::PopItemWidth();
        }
        if (ImGui::CollapsingHeader("Structure", ImGuiTreeNodeFlags_DefaultOpen)) {
            int nx = (int)std::count(regla_.begin(), regla_.end(), 0);
            int ny = (int)std::count(regla_.begin(), regla_.end(), 1);
            ImGui::Text(tr("Inputs (X): %d"), nx);
            ImGui::PushItemWidth(-95);
            if (ImGui::InputText(tr("Hidden Layers"), capas_buf_, sizeof(capas_buf_))) prop_activa_ = 4;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 4;
            ImGui::PopItemWidth();
            ImGui::Text(tr("Outputs (Y): %d"), ny);
        }
        if (ImGui::CollapsingHeader("Data", ImGuiTreeNodeFlags_DefaultOpen)) {
            ImGui::PushItemWidth(-95);
            if (ImGui::InputInt(tr("Validation %"), &particion_val_)) prop_activa_ = 5;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 5;
            if (particion_val_ < 1) particion_val_ = 1;
            if (particion_val_ > 80) particion_val_ = 80;
            ImGui::Text(tr("Training %%: %d"), 100 - particion_val_);
            if (ImGui::InputInt(tr("Batch Size"), &batch_size_)) prop_activa_ = 6;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 6;
            if (batch_size_ < 1) batch_size_ = 1;
            ImGui::PopItemWidth();
        }
        if (ImGui::CollapsingHeader("Stopping Condition", ImGuiTreeNodeFlags_DefaultOpen)) {
            ImGui::PushItemWidth(-95);
            if (ImGui::InputDouble(tr("Stopping MSE"), &rmse_objetivo_, 0, 0, "%.1e")) prop_activa_ = 7;
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 7;
            int maxep = max_epocas_;
            if (ImGui::InputInt(tr("Max Rounds"), &maxep)) {
                max_epocas_ = std::clamp(maxep, 10, 100000000);
                prop_activa_ = 8;
            }
            if (ImGui::IsItemActive() || ImGui::IsItemHovered()) prop_activa_ = 8;
            ImGui::PopItemWidth();
        }
        if (red_.lista() && resultado_ui_.epocas > 0) {
            ImGui::Separator();
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.35f, 0.85f, 0.35f, 1), tr("Rounds: %d"), resultado_ui_.epocas);
            ImGui::Text(tr("MSE Train: %.3e"), resultado_ui_.mse_ent);
            ImGui::Text(tr("MSE Val:   %.3e"), resultado_ui_.mse_val);
        }
        ImGui::EndChild();

        // ── Caja de ayuda contextual (como en la esquina inferior izquierda de HSC Data) ──
        ImGui::Separator();
        static const char* prop_titulos[] = {
            "Learning Rate",
            "Learning Rate Decay",
            "Momentum",
            "Activation Function",
            "Hidden Layers",
            "Validation Data Partition",
            "Batch Size",
            "Stopping MSE",
            "Max Iteration Rounds",
            "Training Method"
        };
        static const char* prop_ayudas[] = {
            "Step size used to update neural network weights during backpropagation (e.g. 0.02).",
            "Rate at which learning rate decreases over iterations: lr / (1 + decay * step).",
            "Fraction of previous weight update added to current step to accelerate convergence (e.g. 0.1).",
            "Transfer function for hidden/output neurons: TanH, Sigmoid, ReLU, or Linear.",
            "Defines hidden layer structure. Example: 4;6 (Two hidden layers, first layer has 4 neurons and last 6).",
            "Percentage of data rows reserved for validation (default 20% validation, 80% training).",
            "Number of training samples processed before updating weights (1 = online SGD, as in HSC).",
            "Target Mean Squared Error (MSE) on normalized data to stop training automatically.",
            "Maximum number of training epochs/iterations before stopping.",
            "Selects FFNN (Backpropagation), PSO (Particle Swarm Optimizer), or RNN (Random Neural Network)."
        };
        int pidx = std::clamp(prop_activa_, 0, 9);
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.75f, 0.82f, 0.95f, 1.0f), "%s", prop_titulos[pidx]);
        ImGui::TextWrapped("%s", prop_ayudas[pidx]);
        ImGui::EndChild();

        ImGui::NextColumn();
    }

    // ══ Hoja de cálculo con fila RULE y pestañas Data | Info ══
    ImGui::BeginChild("dat_sheet", ImVec2(0, area_h), true);
    for (auto& f : datos_) f.resize(regla_.size(), 0.0);

    if (tab_hoja_ == 0) {
        int ncols = (int)regla_.size();
        bool con_ynn = !ynn_.empty() && ynn_.size() == datos_.size();
        int n_ynn = 0;
        if (con_ynn) for (int r : regla_) if (r == 1) n_ynn++;
        int nfilas = (int)datos_.size();

        celda_fila_ = std::clamp(celda_fila_, 0, std::max(0, nfilas - 1));
        celda_colo_ = std::clamp(celda_colo_, 0, std::max(0, ncols - 1));

        const ImU32 COL_SEL      = IM_COL32(68, 114, 196, 255);
        const ImU32 COL_HDR_SEL  = IM_COL32(68, 114, 196, 70);
        const ImU32 COL_FILA_TIN = IM_COL32(68, 114, 196, 26);
        const ImU32 gris_hdr     = ImGui::GetColorU32(ImGuiCol_TableHeaderBg);

        // ── Barra de nombre + fórmula (como la barra de Excel en HSC) ──
        {
            if (celda_fila_ != nombre_fila_prev_ || celda_colo_ != nombre_colo_prev_) {
                char letra[8];
                nombre_columna_excel(celda_colo_, letra, sizeof(letra));
                snprintf(nombre_celda_buf_, sizeof(nombre_celda_buf_), "%s%d", letra, celda_fila_ + 1);
                nombre_fila_prev_ = celda_fila_;
                nombre_colo_prev_ = celda_colo_;
            }
            ImGui::SetNextItemWidth(52);
            if (ImGui::InputText("##namebox", nombre_celda_buf_, sizeof(nombre_celda_buf_),
                                 ImGuiInputTextFlags_EnterReturnsTrue | ImGuiInputTextFlags_AutoSelectAll)) {
                int colo = 0, fila = 0;
                bool ok_letras = false, ok_digitos = false;
                const char* p = nombre_celda_buf_;
                while (*p && std::isalpha((unsigned char)*p)) {
                    colo = colo * 26 + (std::toupper((unsigned char)*p) - 'A' + 1);
                    ++p; ok_letras = true;
                }
                while (*p && std::isdigit((unsigned char)*p)) {
                    fila = fila * 10 + (*p - '0');
                    ++p; ok_digitos = true;
                }
                if (ok_letras && ok_digitos && *p == '\0' &&
                    colo >= 1 && colo <= ncols && fila >= 1 && fila <= nfilas) {
                    celda_colo_ = colo - 1;
                    celda_fila_ = fila - 1;
                    foco_fila_ = celda_fila_;
                    foco_colo_ = celda_colo_;
                }
                nombre_fila_prev_ = -9999;
                nombre_colo_prev_ = -9999;
            }
            if (ImGui::IsItemHovered()) ImGui::SetTooltip(tr("Referencia de celda (ej. B5) + Enter para ir"));
            ImGui::SameLine();
            ImGui::TextDisabled("fx =");
            ImGui::SameLine();
            if (nfilas > 0 && ncols > 0) {
                double val_bar = datos_[celda_fila_][celda_colo_];
                ImGui::SetNextItemWidth(-1);
                ImGui::InputDouble("##formula_bar", &val_bar, 0, 0, "%.6g",
                                   ImGuiInputTextFlags_AutoSelectAll);
                if (ImGui::IsItemEdited())
                    datos_[celda_fila_][celda_colo_] = val_bar;
            }
        }

        ImGuiTableFlags flags = ImGuiTableFlags_Borders
            | ImGuiTableFlags_ScrollY | ImGuiTableFlags_ScrollX | ImGuiTableFlags_Resizable | ImGuiTableFlags_SizingFixedFit;
        bool celda_rect_ok = false;
        ImVec2 celda_min(0, 0), celda_max(0, 0);
        if (ImGui::BeginTable("dat_grid", 1 + ncols + (con_ynn ? n_ynn : 0), flags,
            ImVec2(0, ImGui::GetContentRegionAvail().y - ImGui::GetFrameHeightWithSpacing() * 1.4f))) {
            ImGui::TableSetupScrollFreeze(1, 2);
            ImGui::TableSetupColumn("A (RULE)", ImGuiTableColumnFlags_WidthFixed, 58);
            for (int c = 0; c < ncols; ++c) {
                char lbl[8]; nombre_columna_excel(c + 1, lbl, sizeof(lbl));
                ImGui::TableSetupColumn(lbl, ImGuiTableColumnFlags_WidthFixed, 92);
            }
            if (con_ynn)
                for (int k = 0; k < n_ynn; ++k) {
                    char lbl[8]; nombre_columna_excel(ncols + 1 + k, lbl, sizeof(lbl));
                    ImGui::TableSetupColumn(lbl, ImGuiTableColumnFlags_WidthFixed, 98);
                }

            // ── Encabezado con letras estilo Excel ──
            ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
            ImGui::TableNextColumn();
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, gris_hdr);
            ImGui::TableHeader("A");
            for (int c = 0; c < ncols; ++c) {
                char lbl[8]; nombre_columna_excel(c + 1, lbl, sizeof(lbl));
                ImGui::TableNextColumn();
                ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg,
                                      (c == celda_colo_) ? COL_HDR_SEL : gris_hdr);
                ImGui::TableHeader(lbl);
            }
            if (con_ynn)
                for (int k = 0; k < n_ynn; ++k) {
                    char lbl[8]; nombre_columna_excel(ncols + 1 + k, lbl, sizeof(lbl));
                    ImGui::TableNextColumn();
                    ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, gris_hdr);
                    ImGui::TableHeader(lbl);
                }

            // ── Fila 1: RULE | X | X ... | Y | YNN (clic para alternar X → Y) ──
            ImGui::TableNextRow();
            ImGui::TableNextColumn();
            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, gris_hdr);
            ImGui::TextUnformatted("1  RULE");
            for (int c = 0; c < ncols; ++c) {
                ImGui::TableNextColumn();
                ImGui::PushID(5000 + c);
                ImGui::PushStyleColor(ImGuiCol_Button, regla_[c] == 0
                    ? ImVec4(0.20f, 0.45f, 0.28f, 1.0f) : ImVec4(0.75f, 0.50f, 0.15f, 1.0f));
                if (ImGui::SmallButton(regla_[c] == 0 ? "   X   " : "   Y   ")) {
                    regla_[c] = 1 - regla_[c];
                    int nx = (int)std::count(regla_.begin(), regla_.end(), 0);
                    int ny = (int)std::count(regla_.begin(), regla_.end(), 1);
                    if (nx == 0) regla_[c] = 0;
                    if (ny == 0) regla_[c] = 1;
                    ynn_.clear();
                }
                ImGui::PopStyleColor();
                ImGui::PopID();
            }
            if (con_ynn)
                for (int k = 0; k < n_ynn; ++k) {
                    ImGui::TableNextColumn();
                    ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, gris_hdr);
                    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.3f, 0.7f, 0.95f, 1.0f), "YNN");
                }

            // ── Filas de datos estilo Excel ──
            const ImGuiInputTextFlags flags_celda = ImGuiInputTextFlags_AutoSelectAll;
            for (int r = 0; r < nfilas; ++r) {
                ImGui::TableNextRow();
                ImGui::TableNextColumn();
                ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg,
                                      (r == celda_fila_) ? COL_HDR_SEL : gris_hdr);
                ImGui::Text("%d", r + 4); // numeración de fila como HSC (datos desde fila 4)
                for (int c = 0; c < ncols; ++c) {
                    ImGui::TableNextColumn();
                    if (r == celda_fila_)
                        ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, COL_FILA_TIN);
                    ImGui::PushID(r);
                    ImGui::PushID(c);
                    if (r == foco_fila_ && c == foco_colo_)
                        ImGui::SetKeyboardFocusHere();
                    ImGui::SetNextItemWidth(-FLT_MIN);
                    ImGui::InputDouble("##cell", &datos_[r][c], 0, 0, "%.6g", flags_celda);
                    if (ImGui::IsItemActivated()) {
                        celda_fila_ = r; celda_colo_ = c;
                        foco_fila_ = -1; foco_colo_ = -1;
                    }
                    if (ImGui::IsItemDeactivated()) {
                        ImGuiIO& io = ImGui::GetIO();
                        if (ImGui::IsKeyPressed(ImGuiKey_Enter) || ImGui::IsKeyPressed(ImGuiKey_KeypadEnter)) {
                            foco_fila_ = io.KeyShift ? std::max(0, r - 1)
                                                     : std::min(nfilas - 1, r + 1);
                            foco_colo_ = c;
                        } else if (ImGui::IsKeyPressed(ImGuiKey_Tab)) {
                            foco_fila_ = r;
                            foco_colo_ = io.KeyShift ? std::max(0, c - 1)
                                                     : std::min(ncols - 1, c + 1);
                        }
                    }
                    ImGui::PopID();
                    ImGui::PopID();
                    if (r == celda_fila_ && c == celda_colo_) {
                        celda_rect_ok = true;
                        celda_min = ImGui::GetItemRectMin();
                        celda_max = ImGui::GetItemRectMax();
                    }
                }
                if (con_ynn) {
                    int k2 = 0;
                    for (int c = 0; c < ncols; ++c) {
                        if (regla_[c] != 1) continue;
                        ImGui::TableNextColumn();
                        if (r == celda_fila_)
                            ImGui::TableSetBgColor(ImGuiTableBgTarget_CellBg, COL_FILA_TIN);
                        double v = (k2 < (int)ynn_[r].size()) ? ynn_[r][k2] : 0.0;
                        ImGui::Text("%.6g", v);
                        ++k2;
                    }
                }
            }
            ImGui::EndTable();
        }

        if (celda_rect_ok) {
            ImDrawList* dl = ImGui::GetWindowDrawList();
            dl->AddRect(celda_min, celda_max, COL_SEL, 0.0f, 0, 2.0f);
            dl->AddRectFilled(ImVec2(celda_max.x - 5.0f, celda_max.y - 5.0f), celda_max, COL_SEL);
        }
    } else {
        // ── Pestaña Info: Estadísticas descriptivas de las columnas y resumen de la red ──
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.75f, 0.95f, 1.0f), "Dataset & Network Summary (Info)");
        ImGui::Separator();
        int ncols = (int)regla_.size();
        int nfilas = (int)datos_.size();
        ImGui::Text("Total Rows: %d  |  Training Rows: %d  |  Validation Rows: %d",
                    nfilas,
                    nfilas - std::clamp((int)((double)nfilas * particion_val_ / 100.0), 1, std::max(1, nfilas - 2)),
                    std::clamp((int)((double)nfilas * particion_val_ / 100.0), 1, std::max(1, nfilas - 2)));
        ImGui::Spacing();
        if (ImGui::BeginTable("dat_info_stats", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY,
                              ImVec2(0, ImGui::GetContentRegionAvail().y - ImGui::GetFrameHeightWithSpacing() * 1.6f))) {
            ImGui::TableSetupColumn("Col");
            ImGui::TableSetupColumn("Role");
            ImGui::TableSetupColumn("Min");
            ImGui::TableSetupColumn("Max");
            ImGui::TableSetupColumn("Mean");
            ImGui::TableSetupColumn("Std Dev");
            ImGui::TableHeadersRow();
            for (int c = 0; c < ncols; ++c) {
                double mn = 1e300, mx = -1e300, sum = 0.0, sum2 = 0.0;
                for (int r = 0; r < nfilas; ++r) {
                    double v = datos_[r][c];
                    mn = std::min(mn, v);
                    mx = std::max(mx, v);
                    sum += v;
                }
                double mean = nfilas > 0 ? sum / nfilas : 0.0;
                for (int r = 0; r < nfilas; ++r) {
                    double d = datos_[r][c] - mean;
                    sum2 += d * d;
                }
                double sd = nfilas > 1 ? std::sqrt(sum2 / (nfilas - 1)) : 0.0;
                char lbl[8]; nombre_columna_excel(c + 1, lbl, sizeof(lbl));
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text("%s", lbl);
                ImGui::TableNextColumn(); ImGui::Text("%s", regla_[c] == 0 ? "Input (X)" : "Output (Y)");
                ImGui::TableNextColumn(); ImGui::Text("%.5g", nfilas ? mn : 0.0);
                ImGui::TableNextColumn(); ImGui::Text("%.5g", nfilas ? mx : 0.0);
                ImGui::TableNextColumn(); ImGui::Text("%.5g", mean);
                ImGui::TableNextColumn(); ImGui::Text("%.5g", sd);
            }
            ImGui::EndTable();
        }
    }

    // ── Barra inferior de la hoja: pestañas Data | Info + botones de filas/columnas ──
    if (ImGui::SmallButton(tab_hoja_ == 0 ? "[Data]" : " Data ")) tab_hoja_ = 0;
    ImGui::SameLine();
    if (ImGui::SmallButton(tab_hoja_ == 1 ? "[Info]" : " Info ")) tab_hoja_ = 1;
    ImGui::SameLine();
    ImGui::TextDisabled("|");
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("+ Fila"))) {
        datos_.push_back(std::vector<double>(regla_.size(), 0.0));
        ynn_.clear();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("- Fila")) && datos_.size() > 1) {
        datos_.pop_back();
        ynn_.clear();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("+ Col X"))) {
        for (auto& f : datos_) f.push_back(0.0);
        auto it = std::find(regla_.begin(), regla_.end(), 1);
        regla_.insert(it == regla_.end() ? regla_.end() : it, 0);
        ynn_.clear();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("+ Col Y"))) {
        for (auto& f : datos_) f.push_back(0.0);
        regla_.push_back(1);
        ynn_.clear();
    }
    ImGui::SameLine();
    if (ImGui::SmallButton(tr("- Col")) && regla_.size() > 2) {
        for (auto& f : datos_) if (!f.empty()) f.pop_back();
        regla_.pop_back();
        ynn_.clear();
    }
    ImGui::EndChild();

    if (ver_propiedades_) {
        ImGui::Columns(1);
    }

    // ══ Zona inferior: 3 gráficos estilo HSC (Parity Chart | Performance MSE | Compare Values) ══
    if (hay_graficos && (ver_paridad_ || ver_rendimiento_ || ver_comparar_)) {
        int n_vis = (ver_paridad_ ? 1 : 0) + (ver_rendimiento_ ? 1 : 0) + (ver_comparar_ ? 1 : 0);
        float alto = 195.0f;
        float w_panel = (ImGui::GetContentRegionAvail().x - 8.0f * (float)(n_vis - 1)) / (float)std::max(1, n_vis);
        float child_h = alto + 50.0f;

        // ── 1. Parity Chart (escala normalizada [-0.9, 0.9] como en HSC) ──
        if (ver_paridad_) {
            ImGui::BeginChild("paridad", ImVec2(w_panel, child_h), true);
            ImGui::TextDisabled(tr("Parity Chart"));
            std::vector<double> pe_x, pe_y, pv_x, pv_y;
            int n = (int)std::min(datos_.size(), ynn_.size());
            int n_val = (n >= 4) ? std::clamp((int)((double)n * particion_val_ / 100.0), 1, n - 2) : 1;
            int col_y = -1;
            for (int c = 0; c < (int)regla_.size(); ++c) if (regla_[c] == 1) { col_y = c; break; }
            if (col_y >= 0) {
                for (int r = 0; r < n; ++r) {
                    if (ynn_[r].empty()) continue;
                    // Normalizar Y real e Y predicho a [-0.9, 0.9] como muestra HSC en Parity Chart
                    auto y_true_norm = red_.normalizar_y({datos_[r][col_y]});
                    auto y_pred_norm = red_.normalizar_y({ynn_[r][0]});
                    double vx = y_true_norm.empty() ? datos_[r][col_y] : y_true_norm[0];
                    double vy = y_pred_norm.empty() ? ynn_[r][0] : y_pred_norm[0];
                    if (r < n - n_val) { pe_x.push_back(vx); pe_y.push_back(vy); }
                    else               { pv_x.push_back(vx); pv_y.push_back(vy); }
                }
                if (ImPlot::BeginPlot("##parity", ImVec2(-1, alto), ImPlotFlags_NoLegend)) {
                    ImPlot::SetupAxes("", "");
                    ImPlot::SetupAxisLimits(ImAxis_X1, -1.0, 1.0, ImPlotCond_Once);
                    ImPlot::SetupAxisLimits(ImAxis_Y1, -1.0, 1.0, ImPlotCond_Once);
                    ImPlot::SetNextLineStyle(ImVec4(0.45f, 0.45f, 0.45f, 0.8f), 1.2f);
                    double d[2] = {-0.95, 0.95};
                    ImPlot::PlotLine("y=x", d, d, 2);
                    if (mostrar_validacion_ && !pv_x.empty()) {
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 2.5f, ImVec4(0.20f, 0.42f, 0.85f, 0.85f), -1, ImVec4(0.20f, 0.42f, 0.85f, 0.85f));
                        ImPlot::PlotScatter(tr("Validation"), pv_x.data(), pv_y.data(), (int)pv_x.size());
                    }
                    if (mostrar_entrenamiento_ && !pe_x.empty()) {
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 2.5f, ImVec4(0.75f, 0.22f, 0.20f, 0.85f), -1, ImVec4(0.75f, 0.22f, 0.20f, 0.85f));
                        ImPlot::PlotScatter(tr("Training"), pe_x.data(), pe_y.data(), (int)pe_x.size());
                    }
                    ImPlot::EndPlot();
                }
                ImGui::Checkbox(tr("Validation"), &mostrar_validacion_);
                ImGui::SameLine();
                ImGui::Checkbox(tr("Training"), &mostrar_entrenamiento_);
            }
            ImGui::EndChild();
            if (ver_rendimiento_ || ver_comparar_) ImGui::SameLine();
        }

        // ── 2. Performance MSE (escala logarítmica con línea vertical punteada al final) ──
        if (ver_rendimiento_) {
            ImGui::BeginChild("rend", ImVec2(w_panel, child_h), true);
            ImGui::TextDisabled(tr("Performance MSE"));
            if (ImPlot::BeginPlot("##perf", ImVec2(-1, alto), ImPlotFlags_NoLegend)) {
                ImPlot::SetupAxes("", "");
                ImPlot::SetupAxisScale(ImAxis_Y1, ImPlotScale_Log10);
                auto& xs = resultado_ui_.epoca_hist;
                const auto& src_ent = !resultado_ui_.mse_ent_hist.empty() ? resultado_ui_.mse_ent_hist : resultado_ui_.rmse_ent_hist;
                const auto& src_val = !resultado_ui_.mse_val_hist.empty() ? resultado_ui_.mse_val_hist : resultado_ui_.rmse_val_hist;
                std::vector<double> ent_safe(src_ent.size()), val_safe(src_val.size());
                for (size_t k = 0; k < src_ent.size(); ++k) ent_safe[k] = std::max(src_ent[k], 1e-12);
                for (size_t k = 0; k < src_val.size(); ++k) val_safe[k] = std::max(src_val[k], 1e-12);

                if (mostrar_validacion_ && !xs.empty() && val_safe.size() == xs.size()) {
                    ImPlot::SetNextLineStyle(ImVec4(0.20f, 0.42f, 0.85f, 1.0f), 1.6f);
                    ImPlot::PlotLine(tr("Validation"), xs.data(), val_safe.data(), (int)xs.size());
                }
                if (mostrar_entrenamiento_ && !xs.empty() && ent_safe.size() == xs.size()) {
                    ImPlot::SetNextLineStyle(ImVec4(0.78f, 0.22f, 0.20f, 1.0f), 1.6f);
                    ImPlot::PlotLine(tr("Training"), xs.data(), ent_safe.data(), (int)xs.size());
                }
                if (!xs.empty()) {
                    double x_end = xs.back();
                    ImPlot::SetNextLineStyle(ImVec4(0.35f, 0.35f, 0.35f, 0.75f), 1.0f);
                    ImPlot::PlotInfLines("##end_ep", &x_end, 1);
                }
                ImPlot::EndPlot();
            }
            ImGui::Checkbox(tr("Validation##p"), &mostrar_validacion_);
            ImGui::SameLine();
            ImGui::Checkbox(tr("Training##p"), &mostrar_entrenamiento_);
            ImGui::EndChild();
            if (ver_comparar_) ImGui::SameLine();
        }

        // ── 3. Compare Values (Original en rojo ordenado vs NN Value en puntos azules, como HSC) ──
        if (ver_comparar_) {
            ImGui::BeginChild("comp", ImVec2(w_panel, child_h), true);
            ImGui::TextDisabled(tr("Compare Values"));
            int col_y = -1;
            for (int c = 0; c < (int)regla_.size(); ++c) if (regla_[c] == 1) { col_y = c; break; }
            if (col_y >= 0) {
                int n_filas = (int)std::min(datos_.size(), ynn_.size());
                std::vector<int> orden(n_filas);
                for (int i = 0; i < n_filas; ++i) orden[i] = i;
                std::sort(orden.begin(), orden.end(), [&](int a, int b) {
                    return datos_[a][col_y] < datos_[b][col_y];
                });
                std::vector<double> xs, orig, nn;
                for (int k = 0; k < (int)orden.size(); ++k) {
                    int r = orden[k];
                    if (ynn_[r].empty()) continue;
                    xs.push_back((double)k);
                    orig.push_back(datos_[r][col_y]);
                    nn.push_back(ynn_[r][0]);
                }
                if (ImPlot::BeginPlot("##comp", ImVec2(-1, alto), ImPlotFlags_NoLegend)) {
                    ImPlot::SetupAxes("", "");
                    if (mostrar_nn_comp_ && !xs.empty()) {
                        ImPlot::SetNextMarkerStyle(ImPlotMarker_Circle, 2.2f, ImVec4(0.18f, 0.42f, 0.85f, 0.85f), -1, ImVec4(0.18f, 0.42f, 0.85f, 0.85f));
                        ImPlot::PlotScatter(tr("NN Value"), xs.data(), nn.data(), (int)nn.size());
                    }
                    if (mostrar_orig_comp_ && !xs.empty()) {
                        ImPlot::SetNextLineStyle(ImVec4(0.78f, 0.22f, 0.18f, 1.0f), 2.2f);
                        ImPlot::PlotLine(tr("Original"), xs.data(), orig.data(), (int)xs.size());
                    }
                    ImPlot::EndPlot();
                }
                ImGui::Checkbox(tr("NN Value"), &mostrar_nn_comp_);
                ImGui::SameLine();
                ImGui::Checkbox(tr("Original"), &mostrar_orig_comp_);
            }
            ImGui::EndChild();
        }
    }

    // ══ Ventana View NN Picture (dibujo de la red estilo HSC) ══
    if (ver_red_) {
        ImGui::SetNextWindowSize(ImVec2(580, 440), ImGuiCond_FirstUseEver);
        const char* titulo_red = (tipo_entrenamiento_ == 0) ? "Feedforward Neural Network FFNN###nn_pic"
                               : (tipo_entrenamiento_ == 1) ? "Particle Swarm Optimized FFNN###nn_pic"
                               : "Random Neural Network RNN###nn_pic";
        if (ImGui::Begin(titulo_red, &ver_red_)) {
            dibujar_red(ImGui::GetContentRegionAvail());
        }
        ImGui::End();
    }

    // ══ Ventana View NN XML File (inspección XML de pesos y estructura) ══
    if (ver_xml_) {
        ImGui::SetNextWindowSize(ImVec2(640, 480), ImGuiCond_FirstUseEver);
        if (ImGui::Begin("Neural Network XML File###nn_xml", &ver_xml_)) {
            std::string xml_str = red_.lista() ? red_.a_xml() : "<!-- Press 'Apply Data' or 'Learn' first -->";
            if (ImGui::Button(tr("Copiar XML al portapapeles"))) {
                ImGui::SetClipboardText(xml_str.c_str());
            }
            ImGui::SameLine();
            if (ImGui::Button(tr("Guardar XML..."))) {
                adapters::DesktopFileDialog dlg;
                std::string path = dlg.select("Save Neural Network XML", "*.xml", true);
                if (!path.empty()) {
                    if (path.find(".xml") == std::string::npos) path += ".xml";
                    FILE* f = fopen(path.c_str(), "w");
                    if (f) { fprintf(f, "%s", xml_str.c_str()); fclose(f); }
                }
            }
            ImGui::Separator();
            ImGui::InputTextMultiline("##xml_viewer", const_cast<char*>(xml_str.c_str()), xml_str.size() + 1,
                                      ImVec2(-1, -1), ImGuiInputTextFlags_ReadOnly);
        }
        ImGui::End();
    }
}

}} // namespace
