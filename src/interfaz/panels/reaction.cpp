#include "shared.hpp"
namespace cma { namespace interfaz {
// Panel de ReactionSolver (Analizador y Solucionador de Reacciones)
// ═══════════════════════════════════════════════════════════════════════════

// Split por '+' respetando paréntesis: "Ag(+a) + e-" -> ["Ag(+a)", "e-"]
// (con split naíf, "Ag(+a)" quedaría destruido en "Ag(" y "a)")
static std::vector<std::string> split_reaction_terms(const std::string& s) {
    std::vector<std::string> parts;
    std::string cur;
    int depth = 0;
    for (char c : s) {
        if (c == '(') ++depth;
        else if (c == ')' && depth > 0) --depth;
        if (c == '+' && depth == 0) {
            parts.push_back(cur);
            cur.clear();
        } else {
            cur += c;
        }
    }
    parts.push_back(cur);
    return parts;
}

static std::pair<double, std::string> parse_reaction_term(const std::string& term) {
    std::string t = trim_str(term);
    if (t.empty()) return {0.0, ""};

    size_t i = 0;
    while (i < t.length() && (std::isdigit(t[i]) || t[i] == '.' || t[i] == '/')) {
        i++;
    }

    double coeff = 1.0;
    std::string formula = t;

    if (i > 0) {
        std::string num_part = t.substr(0, i);
        std::string rest = trim_str(t.substr(i));
        if (!rest.empty()) {
            formula = rest;
            size_t slash = num_part.find('/');
            if (slash != std::string::npos) {
                try {
                    double num = std::stod(num_part.substr(0, slash));
                    double den = std::stod(num_part.substr(slash + 1));
                    if (den != 0) coeff = num / den;
                } catch (...) {
                    coeff = 1.0;
                }
            } else {
                try {
                    coeff = std::stod(num_part);
                } catch (...) {
                    coeff = 1.0;
                }
            }
        }
    }

    return {coeff, formula};
}

bool parse_chemical_equation(const std::string& eq_str, cma::nucleo::Reaccion& rxn, std::string& error_msg) {
    rxn.estequiometria.clear();

    std::string clean_eq = trim_str(eq_str);
    if (clean_eq.empty()) {
        error_msg = "La ecuación está vacía.";
        return false;
    }

    std::string arrow = "->";
    size_t arrow_pos = clean_eq.find(arrow);
    if (arrow_pos == std::string::npos) {
        arrow = "=";
        arrow_pos = clean_eq.find(arrow);
        if (arrow_pos == std::string::npos) {
            error_msg = "No se encontró el separador '->' o '='.";
            return false;
        }
    }

    std::string reactants_str = clean_eq.substr(0, arrow_pos);
    std::string products_str = clean_eq.substr(arrow_pos + arrow.length());

    auto reactants = split_reaction_terms(reactants_str);
    for (const auto& r : reactants) {
        std::string term = trim_str(r);
        if (term.empty()) continue;
        auto [coeff, formula] = parse_reaction_term(term);
        if (formula.empty()) continue;

        const auto* esp = buscar_especie_por_nombre_o_formula(formula);
        if (!esp) {
            error_msg = "Especie no encontrada en la base de datos: " + formula;
            return false;
        }

        rxn.estequiometria[esp->id] -= coeff;
    }

    auto products = split_reaction_terms(products_str);
    for (const auto& p : products) {
        std::string term = trim_str(p);
        if (term.empty()) continue;
        auto [coeff, formula] = parse_reaction_term(term);
        if (formula.empty()) continue;

        const auto* esp = buscar_especie_por_nombre_o_formula(formula);
        if (!esp) {
            error_msg = "Especie no encontrada en la base de datos: " + formula;
            return false;
        }

        rxn.estequiometria[esp->id] += coeff;
    }

    if (rxn.estequiometria.empty()) {
        error_msg = "Ecuación vacía o inválida.";
        return false;
    }

    return true;
}

// Funciones auxiliares para termodinámica con Cp real
static double calc_dH_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dH = 0.0;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it != db.end()) {
            dH += coeff * it->second.entalpia_abs(T_K);
        }
    }
    return dH;
}

static double calc_dS_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dS = 0.0;
    for (auto& [sid, coeff] : rxn.estequiometria) {
        auto it = db.find(sid);
        if (it != db.end()) {
            dS += coeff * it->second.entropia_abs(T_K);
        }
    }
    return dS;
}

static double calc_dG_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dH = calc_dH_T(rxn, T_K, db);
    double dS = calc_dS_T(rxn, T_K, db);
    return dH - T_K * dS / 1000.0;
}

// Log10 K calculado directamente de dG para evitar overflow de exp()
static double calc_logK_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double dG = calc_dG_T(rxn, T_K, db); // kJ/mol
    return -dG * 1000.0 / (cma::nucleo::R_GAS * T_K * 2.302585092994046); // ln(10)
}

static double calc_Keq_T(const cma::nucleo::Reaccion& rxn, double T_K, const std::map<int64_t, cma::nucleo::Especie>& db) {
    double logK = calc_logK_T(rxn, T_K, db);
    if (logK >  300.0) return 1.0e+300; // saturar para no imprimir "inf"
    if (logK < -300.0) return 1.0e-300;
    return std::pow(10.0, logK);
}

// ── Detección de semirreacción electroquímica (presencia de e-) ────────────
static bool info_electroquimica(const cma::nucleo::Reaccion& rxn, double& n_electrones) {
    auto& gestor = cma::base_datos::GestorDB::instancia();
    n_electrones = 0.0;
    for (auto& [sp_id, coeff] : rxn.estequiometria) {
        const auto* sp = gestor.especie_por_id(sp_id);
        if (sp && (sp->formula == "e-" || sp->formula == "e")) {
            n_electrones = std::abs(coeff);
            return n_electrones > 0.0;
        }
    }
    return false;
}

// Potencial estándar vs SHE:  ΔG = -n·F·E  →  E° = -ΔG/(n·F)   [F = 96.4853 kJ/(V·mol)]
static double calc_E0_she(double dG_kJ, double n_e) {
    if (n_e <= 0.0) return 0.0;
    return -dG_kJ / (n_e * 96.4853);
}

// Carga formal a partir del tag de fase almacenado en la BD: "(+a)"→+1, "(-2a)"→-2...
static double carga_de_tag(const std::string& formula) {
    size_t open = formula.rfind('(');
    if (open == std::string::npos) return 0.0;
    size_t close = formula.find(')', open);
    if (close == std::string::npos) return 0.0;
    std::string tag = formula.substr(open + 1, close - open - 1);
    if (tag.empty() || (tag[0] != '+' && tag[0] != '-')) return 0.0;
    int sign = (tag[0] == '+') ? 1 : -1;
    double val = 0.0;
    for (size_t i = 1; i < tag.size() && std::isdigit((unsigned char)tag[i]); ++i)
        val = val * 10.0 + (tag[i] - '0');
    if (val == 0.0) val = 1.0;
    return sign * val;
}

// Verificación del balance elemental (y de carga) como hace HSC al calcular
static bool verificar_balance_reaccion(const cma::nucleo::Reaccion& rxn, std::string& detalle) {
    auto& gestor = cma::base_datos::GestorDB::instancia();
    std::map<std::string, double> tot_el;
    double carga = 0.0;
    for (auto& [sp_id, coeff] : rxn.estequiometria) {
        const auto* sp = gestor.especie_por_id(sp_id);
        if (!sp) continue;
        std::string err;
        auto el = parse_formula_elements(sp->formula, err);
        for (auto& [e, n] : el) {
            if (e != "e-") tot_el[e] += coeff * n;
        }
        if (sp->formula == "e-" || sp->formula == "e") carga -= coeff;
        else carga += coeff * carga_de_tag(sp->formula);
    }
    bool ok = true;
    std::ostringstream os;
    for (auto& [e, v] : tot_el) {
        if (std::abs(v) > 1e-6) {
            ok = false;
            os << e << (v > 0 ? ": +" : ": ") << v << "  ";
        }
    }
    if (std::abs(carga) > 1e-6) {
        ok = false;
        os << "charge: " << (carga > 0 ? "+" : "") << carga;
    }
    detalle = os.str();
    return ok;
}

// ── Element y charge parser para balance de reacciones ──
std::map<std::string, int> parse_formula_elements(const std::string& formula, std::string& err) {
    std::map<std::string, int> counts;
    std::vector<std::map<std::string, int>> stack;
    stack.push_back({});
    
    size_t i = 0;
    size_t len = formula.length();
    
    std::string clean_form;
    for (size_t k = 0; k < len; ++k) {
        if (formula[k] == '(') {
            size_t close = formula.find(')', k);
            if (close != std::string::npos) {
                std::string tag = formula.substr(k + 1, close - k - 1);
                if (tag == "g" || tag == "s" || tag == "l" || tag == "a" || tag == "+a" || tag == "-a" || tag.find("a") != std::string::npos) {
                    k = close;
                    continue;
                }
            }
        }
        clean_form += formula[k];
    }
    
    len = clean_form.length();
    i = 0;
    while (i < len) {
        if (clean_form[i] == '(') {
            stack.push_back({});
            i++;
        } else if (clean_form[i] == ')') {
            i++;
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            
            if (stack.size() < 2) {
                err = "Mismatched parentheses";
                return {};
            }
            auto top = stack.back();
            stack.pop_back();
            for (auto& [el, cnt] : top) {
                stack.back()[el] += cnt * val;
            }
        } else if (std::isupper(clean_form[i])) {
            std::string el;
            el += clean_form[i];
            i++;
            if (i < len && std::islower(clean_form[i])) {
                el += clean_form[i];
                i++;
            }
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            stack.back()[el] += val;
        } else if (clean_form[i] == '+' || clean_form[i] == '-') {
            std::string el = "e-";
            int sign = (clean_form[i] == '+') ? 1 : -1;
            i++;
            int val = 0;
            while (i < len && std::isdigit(clean_form[i])) {
                val = val * 10 + (clean_form[i] - '0');
                i++;
            }
            if (val == 0) val = 1;
            stack.back()[el] += sign * val;
        } else {
            i++;
        }
    }
    
    if (stack.size() != 1) {
        err = "Mismatched parentheses";
        return {};
    }
    return stack.back();
}

static bool balance_reaction_algebraic(const std::string& input, std::string& output, std::string& error_msg,
                                       double smallest_coeff = 1.0) {
    std::string clean = trim_str(input);
    std::string arrow = "->";
    size_t arrow_pos = clean.find(arrow);
    if (arrow_pos == std::string::npos) {
        arrow = "=";
        arrow_pos = clean.find(arrow);
        if (arrow_pos == std::string::npos) {
            error_msg = "Separador '=' o '->' no encontrado.";
            return false;
        }
    }

    std::string react_str = clean.substr(0, arrow_pos);
    std::string prod_str = clean.substr(arrow_pos + arrow.length());

    auto react_parts = split_reaction_terms(react_str);
    auto prod_parts = split_reaction_terms(prod_str);
    
    struct SpeciesTerm {
        std::string raw_formula;
        std::string state_suffix;
        std::map<std::string, int> elements;
        bool is_reactant;
    };
    
    std::vector<SpeciesTerm> terms;
    std::set<std::string> all_elements;
    
    auto process_part = [&](const std::string& part, bool is_react) {
        std::string term = trim_str(part);
        if (term.empty()) return;
        auto [coeff, formula] = parse_reaction_term(term);
        
        std::string base_formula = formula;
        std::string suffix;
        size_t par = formula.find('(');
        if (par != std::string::npos) {
            base_formula = formula.substr(0, par);
            suffix = formula.substr(par);
        }
        
        std::string err;
        auto el_map = parse_formula_elements(formula, err);
        if (!err.empty()) {
            error_msg = "Error parseando " + formula + ": " + err;
            return;
        }
        
        SpeciesTerm st;
        st.raw_formula = base_formula;
        st.state_suffix = suffix;
        st.elements = el_map;
        st.is_reactant = is_react;
        terms.push_back(st);
        
        for (auto& [el, cnt] : el_map) {
            all_elements.insert(el);
        }
    };
    
    for (auto& r : react_parts) process_part(r, true);
    for (auto& p : prod_parts) process_part(p, false);
    
    if (!error_msg.empty()) return false;
    if (terms.empty()) {
        error_msg = "Ecuación vacía.";
        return false;
    }
    
    int num_elements = (int)all_elements.size();
    int num_species = (int)terms.size();
    
    std::vector<std::string> elements_list(all_elements.begin(), all_elements.end());
    
    Eigen::MatrixXd A = Eigen::MatrixXd::Zero(num_elements, num_species);
    for (int i = 0; i < num_elements; ++i) {
        std::string el = elements_list[i];
        for (int j = 0; j < num_species; ++j) {
            auto& st = terms[j];
            auto it = st.elements.find(el);
            if (it != st.elements.end()) {
                A(i, j) = st.is_reactant ? it->second : -it->second;
            }
        }
    }
    
    Eigen::JacobiSVD<Eigen::MatrixXd> svd(A, Eigen::ComputeFullV);
    Eigen::VectorXd null_space_vector = svd.matrixV().col(num_species - 1);
    
    if (null_space_vector(0) < 0.0) {
        null_space_vector = -null_space_vector;
    }
    
    double min_val = 1e9;
    for (int i = 0; i < num_species; ++i) {
        if (null_space_vector(i) > 1e-4 && null_space_vector(i) < min_val) {
            min_val = null_space_vector(i);
        }
    }

    // Normalizar: coeficiente más pequeño = 1
    if (min_val > 1e-4 && min_val < 1e9) {
        null_space_vector /= min_val;
    }

    // "Smallest Coefficient" de HSC: si es 1, preferir coeficientes enteros;
    // si no, escalar para que el coeficiente menor sea exactamente ese valor.
    if (smallest_coeff == 1.0) {
        double multiplier = 1.0;
        for (int m = 1; m <= 20; ++m) {
            bool all_int = true;
            for (int i = 0; i < num_species; ++i) {
                double val = null_space_vector(i) * m;
                double diff = std::abs(val - std::round(val));
                if (diff > 0.05) {
                    all_int = false;
                    break;
                }
            }
            if (all_int) {
                multiplier = m;
                break;
            }
        }
        null_space_vector *= multiplier;
    } else {
        null_space_vector *= smallest_coeff;
    }

    std::string balanced_reactants;
    std::string balanced_products;

    for (int j = 0; j < num_species; ++j) {
        double coeff = std::round(null_space_vector(j) * 10000.0) / 10000.0;
        if (coeff <= 0.0) coeff = (smallest_coeff > 0.0) ? smallest_coeff : 1.0;

        std::string coeff_str;
        if (std::abs(coeff - 1.0) > 1e-9) {
            char buf[32];
            if (coeff == std::round(coeff))
                snprintf(buf, sizeof(buf), "%d ", (int)coeff);
            else
                snprintf(buf, sizeof(buf), "%.4g ", coeff);
            coeff_str = buf;
        }
        
        std::string sp_str = coeff_str + terms[j].raw_formula + terms[j].state_suffix;
        
        if (terms[j].is_reactant) {
            if (!balanced_reactants.empty()) balanced_reactants += " + ";
            balanced_reactants += sp_str;
        } else {
            if (!balanced_products.empty()) balanced_products += " + ";
            balanced_products += sp_str;
        }
    }
    
    output = balanced_reactants + " = " + balanced_products;
    return true;
}

void PanelReactionSolver::balancear_ecuacion() {
    std::string balanced;
    std::string err;
    if (balance_reaction_algebraic(ecuacion_, balanced, err, smallest_coefficient_)) {
        strncpy(ecuacion_, balanced.c_str(), sizeof(ecuacion_) - 1);
        ecuacion_[sizeof(ecuacion_) - 1] = '\0';
        error_parsing_ = false;
        error_msg_.clear();
    } else {
        error_parsing_ = true;
        error_msg_ = err;
    }
}

void PanelReactionSolver::guardar_tab_actual() {
    auto& t = tabs_[tab_actual_];
    t.ecuacion = ecuacion_;
    t.desde = temp_desde_;
    t.hasta = temp_hasta_;
    t.paso  = temp_paso_;
}

void PanelReactionSolver::cargar_tab(int idx) {
    if (idx < 0 || idx >= 3 || idx == tab_actual_) return;
    guardar_tab_actual();
    tab_actual_ = idx;
    auto& t = tabs_[idx];
    strncpy(ecuacion_, t.ecuacion.c_str(), sizeof(ecuacion_) - 1);
    ecuacion_[sizeof(ecuacion_) - 1] = '\0';
    temp_desde_ = t.desde;
    temp_hasta_ = t.hasta;
    temp_paso_  = t.paso;
    calculado_ = false;
    error_parsing_ = false;
    error_msg_.clear();
    balance_ok_ = true;
    balance_detalle_.clear();
}

// Valor del eje Y para un modo dado (reacción)
static double valor_eje_rxn(int mode, const cma::nucleo::Reaccion& rxn, double T_K,
                            double n_electrones, const std::map<int64_t, cma::nucleo::Especie>& db) {
    switch (mode) {
        case 0:  return calc_logK_T(rxn, T_K, db);
        case 1:  return calc_dH_T(rxn, T_K, db);
        case 2:  return calc_dS_T(rxn, T_K, db);
        case 3:  return calc_dG_T(rxn, T_K, db);
        case 4:  return calc_E0_she(calc_dG_T(rxn, T_K, db), n_electrones);
        default: return 0.0;
    }
}

// Valor del eje Y para un modo dado (fórmula / especie)
static double valor_eje_especie(int mode, const cma::nucleo::Especie& e, double T_K) {
    switch (mode) {
        case 0:  return e.cp(T_K);
        case 1:  return e.entalpia_abs(T_K);
        case 2:  return e.entropia_abs(T_K);
        case 3:  return e.energia_gibbs(T_K);
        default: return 0.0;
    }
}

void PanelReactionSolver::dibujar_grafico(const ImVec2& tam, bool para_reaccion) {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    static const char* ylabels_rxn[5] = {"Log K", "dH (kJ)", "dS (J/K)", "dG (kJ)", "E° vs SHE (V)"};
    static const char* titulos_rxn[5] = {"Equilibrium constant (Log)", "Reaction Enthalpy",
                                         "Reaction Entropy", "Reaction Gibbs Energy",
                                         "Electrode Potential vs SHE"};
    static const char* ylabels_esp[4] = {"Cp (J/(mol*K))", "H (kJ/mol)", "S (J/(mol*K))", "G (kJ/mol)"};
    static const char* titulos_esp[4] = {"Heat Capacity", "Enthalpy", "Entropy", "Gibbs Energy"};

    int mode = y_axis_mode_;
    if (para_reaccion && mode > 4) mode = 0;
    if (!para_reaccion && mode > 3) mode = 0;

    const char* titulo = para_reaccion ? titulos_rxn[mode] : titulos_esp[mode];
    const char* ylabel = para_reaccion ? ylabels_rxn[mode] : ylabels_esp[mode];

    if (!ImPlot::BeginPlot(titulo, tam)) return;
    ImPlot::SetupAxes("T (°C)", ylabel);

    // Solo se dibujan curvas del tipo actual (los ejes Y son incompatibles entre tipos)
    size_t last = curvas_.size();
    bool hay_compat = false;
    for (size_t i = 0; i < curvas_.size(); ++i)
        if (curvas_[i].para_reaccion == para_reaccion) { last = i; hay_compat = true; }
    size_t first = plot_all_data_ ? 0 : last; // Plot All Data off -> solo la última compatible

    if (hay_compat) {
        for (size_t i = first; i < curvas_.size(); ++i) {
            auto& c = curvas_[i];
            if (!c.visible || c.para_reaccion != para_reaccion) continue;
            std::vector<double> ys;
            ys.reserve(c.xs.size());
            for (double tc : c.xs) {
                double tk = tc + 273.15;
                if (c.para_reaccion) {
                    double n_e = 0.0;
                    info_electroquimica(c.rxn, n_e);
                    ys.push_back(valor_eje_rxn(mode, c.rxn, tk, n_e, db));
                } else {
                    const auto* sp = gestor.especie_por_id(c.especie_id);
                    ys.push_back(sp ? valor_eje_especie(mode, *sp, tk) : 0.0);
                }
            }
            ImPlot::PlotLine(c.etiqueta.c_str(), c.xs.data(), ys.data(), (int)c.xs.size());
        }
    }
    ImPlot::EndPlot();
}

void PanelReactionSolver::renderizar() {
    // Alias locales a miembros (el cuerpo heredado usa estos nombres)
    auto& rxn_actual    = rxn_actual_;
    auto& error_msg     = error_msg_;
    bool& error_parsing = error_parsing_;

    if (!tabs_inicializados_) {
        tabs_[0] = {ecuacion_, temp_desde_, temp_hasta_, temp_paso_};
        tabs_[1] = {"", 0.0, 1200.0, 25.0};
        tabs_[2] = {"", 0.0, 1200.0, 25.0};
        tabs_inicializados_ = true;
    }

    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();

    bool es_reaccion = (strchr(ecuacion_, '=') != nullptr || strstr(ecuacion_, "->") != nullptr);
    const cma::nucleo::Especie* esp_actual = nullptr;
    if (!es_reaccion) {
        esp_actual = buscar_especie_por_nombre_o_formula(ecuacion_);
    }

    // Detección de semirreacción electroquímica (e- presente)
    double n_electrones = 0.0;
    bool es_electroquimica = calculado_ && es_reaccion && info_electroquimica(rxn_actual, n_electrones);

    // Clamp del selector de eje Y según el modo actual (evita índices fuera de rango)
    int max_mode = es_reaccion ? (es_electroquimica ? 4 : 3) : 3;
    if (y_axis_mode_ > max_mode) y_axis_mode_ = 0;

    // ── Layout de Dos Columnas ──
    ImGui::Columns(2, "reaction_solver_split", true);
    static bool init_w = true;
    if (init_w) {
        ImGui::SetColumnWidth(0, ImGui::GetWindowWidth() * 0.60f);
        ImGui::SetColumnWidth(1, ImGui::GetWindowWidth() * 0.40f);
        init_w = false;
    }

    // ════════════════════════════════════════════════════════════════════════
    // Columna Izquierda: Especificaciones y Grid
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("left_solver_pane");
    {
        // ── Toolbar ──
        ImGui::PushStyleVar(ImGuiStyleVar_FramePadding, ImVec2(6, 4));
        if (ImGui::Button(tr("Exportar CSV"))) {
            if (calculado_) {
                FILE* zen = adapters::dialog_pipe("zenity --file-selection --save --confirm-overwrite --file-filter='*.csv' --title='Export Reaction Data' 2>/dev/null");
                if (zen) {
                    char buf[512] = {0};
                    if (fgets(buf, sizeof(buf), zen)) {
                        std::string path(buf);
                        if (!path.empty() && path.back() == '\n') path.pop_back();
                        if (!path.empty()) {
                            if (path.find(".csv") == std::string::npos) path += ".csv";
                            FILE* fp = fopen(path.c_str(), "w");
                            if (fp) {
                                if (es_reaccion) {
                                    fprintf(fp, "Reaction: %s\n", ecuacion_);
                                    double n_e = 0.0;
                                    bool electro = info_electroquimica(rxn_actual, n_e);
                                    if (electro) {
                                        fprintf(fp, "T (°C),dH (kJ),dS (J/K),dG (kJ),K,Log K,E vs SHE (V)\n");
                                    } else {
                                        fprintf(fp, "T (°C),dH (kJ),dS (J/K),dG (kJ),K,Log K\n");
                                    }
                                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                                        double tk = tc + 273.15;
                                        double dH = calc_dH_T(rxn_actual, tk, db);
                                        double dS = calc_dS_T(rxn_actual, tk, db);
                                        double dG = calc_dG_T(rxn_actual, tk, db);
                                        double K    = calc_Keq_T(rxn_actual, tk, db);
                                        double logK = calc_logK_T(rxn_actual, tk, db);
                                        if (electro) {
                                            double E0 = calc_E0_she(dG, n_e);
                                            fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.6e,%.4f,%.4f\n", tc, dH, dS, dG, K, logK, E0);
                                        } else {
                                            fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.6e,%.4f\n", tc, dH, dS, dG, K, logK);
                                        }
                                    }
                                } else if (esp_actual) {
                                    fprintf(fp, "Species: %s\n", esp_actual->nombre.c_str());
                                    fprintf(fp, "T (°C),Cp (J/mol·K),H (kJ/mol),S (J/mol·K),G (kJ/mol)\n");
                                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                                        double tk = tc + 273.15;
                                        fprintf(fp, "%.1f,%.4f,%.4f,%.4f,%.4f\n", tc,
                                            esp_actual->cp(tk),
                                            esp_actual->entalpia_abs(tk),
                                            esp_actual->entropia_abs(tk),
                                            esp_actual->energia_gibbs(tk));
                                    }
                                }
                                fclose(fp);
                            }
                        }
                    }
                    fclose(zen);
                }
            }
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Export thermodynamic data to CSV file");
        ImGui::SameLine();
        if (ImGui::Button(tr("Limpiar"))) {
            ecuacion_[0] = '\0';
            calculado_ = false;
            error_parsing = false;
        }
        ImGui::SameLine();
        ImGui::TextDisabled(tr("|"));
        ImGui::SameLine();
        if (ImGui::Button(tr("WGS"))) {
            strncpy(ecuacion_, "CO + H2O = CO2 + H2", sizeof(ecuacion_)-1);
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Water-Gas Shift Reaction");
        ImGui::SameLine();
        if (ImGui::Button(tr("Combustion CH4"))) {
            strncpy(ecuacion_, "CH4 + 2 O2 = CO2 + 2 H2O", sizeof(ecuacion_)-1);
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("CO2(g)"))) {
            strncpy(ecuacion_, "CO2", sizeof(ecuacion_)-1);
            calculado_ = false;
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Show CO2 thermodynamic properties");
        ImGui::PopStyleVar();
        ImGui::Separator();
        ImGui::Spacing();

        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.6f, 0.7f, 0.9f, 1.0f), "Specifications");
        ImGui::Separator();
        
        ImGui::Text(tr("Reaction Equation or Chemical Formula:"));
        ImGui::SetNextItemWidth(-150);
        ImGui::InputText("##eq_input", ecuacion_, sizeof(ecuacion_));
        ImGui::SameLine();
        if (ImGui::Button(tr("Examinar DB##rxn"), ImVec2(140, 0))) {
            ModalBuscadorDB::instancia().abrir("Buscar Especie (Reacción)");
        }
        std::vector<const nucleo::Especie*> sel_rxn;
        if (ModalBuscadorDB::instancia().dibujar("Buscar Especie (Reacción)", sel_rxn, true)) {
            for (auto* sp : sel_rxn) {
                std::string eq = ecuacion_;
                if (!eq.empty() && eq.back() != ' ' && eq.back() != '+') eq += " + ";
                eq += sp->formula;
                strncpy(ecuacion_, eq.c_str(), sizeof(ecuacion_) - 1);
            }
        }
        
        ImGui::SetNextItemWidth(100);
        if (ImGui::InputDouble("Smallest Coefficient", &smallest_coefficient_, 0.5, 1.0, "%.4g")) {
            if (smallest_coefficient_ <= 0.0) smallest_coefficient_ = 1.0;
        }
        ImGui::SameLine();
        if (ImGui::Button(tr("Balance"), ImVec2(80, 24))) {
            balancear_ecuacion();
        }
        
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("From (°C)", &temp_desde_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("To (°C)", &temp_hasta_, 0, 0, "%.1f");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(80);
        ImGui::InputDouble("Step (°C)", &temp_paso_, 0, 0, "%.1f");
        
        ImGui::SameLine();
        if (ImGui::Button(tr("Calcular"), ImVec2(90, 24))) {
            if (es_reaccion) {
                error_parsing = !parse_chemical_equation(ecuacion_, rxn_actual, error_msg);
                calculado_ = !error_parsing;
                if (calculado_) {
                    // HSC verifica el balance elemental de la ecuación
                    balance_ok_ = verificar_balance_reaccion(rxn_actual, balance_detalle_);
                }
            } else {
                if (esp_actual) {
                    error_parsing = false;
                    calculado_ = true;
                    balance_ok_ = true;
                    balance_detalle_.clear();
                } else {
                    error_msg = "Species not found in database: " + std::string(ecuacion_);
                    error_parsing = true;
                    calculado_ = false;
                }
            }

            // Collect Data de HSC: cada cálculo añade una curva al chart
            if (calculado_) {
                if (!collect_data_) curvas_.clear();
                Curva c;
                c.para_reaccion = es_reaccion;
                char tag = es_reaccion ? 'R' : 'S';
                std::string eq_lbl = es_reaccion ? std::string(ecuacion_) : esp_actual->formula;
                c.etiqueta = std::string(1, tag) + std::to_string(tab_actual_ + 1) + ": " + eq_lbl;
                if (es_reaccion) c.rxn = rxn_actual;
                else c.especie_id = esp_actual->id;
                double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) c.xs.push_back(tc);
                if (c.xs.empty()) c.xs.push_back(temp_desde_);
                curvas_.push_back(std::move(c));
            }
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Calculate (recalculates and collects curve)");

        if (error_parsing) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.4f, 0.4f, 1.0f), "Error: %s", error_msg.c_str());
        } else if (calculado_ && es_reaccion && !balance_ok_) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(1.0f, 0.6f, 0.1f, 1.0f),
                "Warning: elemental/charge balance failed (%s). Use 'Balance' to fix the equation.", balance_detalle_.c_str());
        } else if (calculado_ && es_reaccion) {
            ImGui::TextColored(cma::interfaz::AdaptiveColor(0.4f, 0.85f, 0.4f, 1.0f), "Elemental balance: OK");
        }
        
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        
        // Tabla de datos termodinámicos
        if (calculado_) {
            if (es_reaccion) {
                ImGui::Text(tr("Reaction Equation: %s"), ecuacion_);
                int cols = es_electroquimica ? 7 : 6;

                if (ImGui::BeginTable("reaction_grid", cols, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 180))) {
                    ImGui::TableSetupColumn("T");
                    ImGui::TableSetupColumn("dH");
                    ImGui::TableSetupColumn("dS");
                    ImGui::TableSetupColumn("dG");
                    ImGui::TableSetupColumn("K");
                    ImGui::TableSetupColumn("Log K");
                    if (es_electroquimica) {
                        ImGui::TableSetupColumn("E");
                    }
                    ImGui::TableHeadersRow();

                    // Fila de unidades como en HSC
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/K");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");
                    if (es_electroquimica) {
                        ImGui::TableNextColumn(); ImGui::TextDisabled("V vs SHE");
                    }

                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        double dH = calc_dH_T(rxn_actual, tk, db);
                        double dS = calc_dS_T(rxn_actual, tk, db);
                        double dG = calc_dG_T(rxn_actual, tk, db);
                        double K    = calc_Keq_T(rxn_actual, tk, db);
                        double logK = calc_logK_T(rxn_actual, tk, db);

                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tc);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dH);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dS);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", dG);
                        ImGui::TableNextColumn(); ImGui::Text("%.3e", K);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", logK);
                        if (es_electroquimica) {
                            double E0 = calc_E0_she(dG, n_electrones);
                            ImGui::TableNextColumn(); ImGui::Text("%.4f", E0);
                        }
                    }
                    ImGui::EndTable();
                }
                ImGui::Spacing();
            } else if (esp_actual) {
                ImGui::Text(tr("Formula: %s"), esp_actual->nombre.c_str());
                if (ImGui::BeginTable("formula_grid", 6, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg | ImGuiTableFlags_ScrollY, ImVec2(0, 180))) {
                    ImGui::TableSetupColumn("T");
                    ImGui::TableSetupColumn("Cp");
                    ImGui::TableSetupColumn("H");
                    ImGui::TableSetupColumn("S");
                    ImGui::TableSetupColumn("G");
                    ImGui::TableSetupColumn("Reference");
                    ImGui::TableHeadersRow();

                    // Fila de unidades como en HSC
                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::TextDisabled("°C");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/(mol*K)");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ/mol");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("J/(mol*K)");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("kJ/mol");
                    ImGui::TableNextColumn(); ImGui::TextDisabled("-");

                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        double cp = esp_actual->cp(tk);
                        double H = esp_actual->entalpia_abs(tk);
                        double S = esp_actual->entropia_abs(tk);
                        double G = esp_actual->energia_gibbs(tk);

                        ImGui::TableNextRow();
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", tc);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", cp);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", H);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", S);
                        ImGui::TableNextColumn(); ImGui::Text("%.3f", G);
                        ImGui::TableNextColumn(); ImGui::TextDisabled("CMA DB");
                    }
                    ImGui::EndTable();
                }

                ImGui::Spacing();
                ImGui::Separator();
                ImGui::Spacing();

                // Species Data: cantidad de especie (Input Amounts) como en HSC
                ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.5f, 1.0f), "Species Data");
                if (ImGui::BeginTable("species_data_grid", 8, ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg)) {
                    ImGui::TableSetupColumn("Fórmula");
                    ImGui::TableSetupColumn("M g/mol");
                    ImGui::TableSetupColumn("Conc. wt-%");
                    ImGui::TableSetupColumn("Input mol", ImGuiTableColumnFlags_WidthFixed, 110.0f);
                    ImGui::TableSetupColumn("g");
                    ImGui::TableSetupColumn("Volume");
                    ImGui::TableSetupColumn("Unit");
                    ImGui::TableSetupColumn("Extrap. From T(K)");
                    ImGui::TableHeadersRow();

                    double g = especie_mol_ * esp_actual->masa_molar;
                    bool es_gas = (esp_actual->fase_std == cma::nucleo::Fase::Gas);
                    double vol_nl = especie_mol_ * 22.414; // N litros (0 °C, 1 atm)

                    ImGui::TableNextRow();
                    ImGui::TableNextColumn(); ImGui::Text("%s", esp_actual->formula.c_str());
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", esp_actual->masa_molar);
                    ImGui::TableNextColumn(); ImGui::Text("100.000");
                    ImGui::TableNextColumn();
                    ImGui::SetNextItemWidth(100.0f);
                    if (ImGui::InputDouble("##input_mol_species", &especie_mol_, 0.1, 1.0, "%.3f")) {
                        if (especie_mol_ <= 0.0) especie_mol_ = 1.0;
                    }
                    ImGui::TableNextColumn(); ImGui::Text("%.3f", g);
                    ImGui::TableNextColumn();
                    if (es_gas) ImGui::Text("%.3f", vol_nl); else ImGui::TextDisabled("-");
                    ImGui::TableNextColumn();
                    if (es_gas) ImGui::Text("Nl"); else ImGui::TextDisabled("-");
                    ImGui::TableNextColumn(); ImGui::Text("298.15");

                    ImGui::EndTable();
                }
                ImGui::Spacing();
            }
        }

        // ── Tabs estilo HSC: R1-R3 para reacciones, S1-S3 para fórmulas ──
        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();
        {
            char letra = es_reaccion ? 'R' : 'S';
            for (int i = 0; i < 3; ++i) {
                ImGui::PushID(7000 + i);
                char lbl[8];
                snprintf(lbl, sizeof(lbl), "%c%d", letra, i + 1);
                if (ImGui::Selectable(lbl, tab_actual_ == i, ImGuiSelectableFlags_None, ImVec2(36, 0))) {
                    cargar_tab(i);
                }
                if (ImGui::IsItemHovered()) {
                    ImGui::SetTooltip("%s", tabs_[i].ecuacion.empty() ? "(empty)" : tabs_[i].ecuacion.c_str());
                }
                ImGui::PopID();
                if (i < 2) ImGui::SameLine();
            }
        }
    }
    ImGui::EndChild();

    ImGui::NextColumn();

    // ════════════════════════════════════════════════════════════════════════
    // Columna Derecha: Gráfico
    // ════════════════════════════════════════════════════════════════════════
    ImGui::BeginChild("right_solver_pane");
    {
        ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.6f, 0.8f, 1.0f), "Chart");
        ImGui::Separator();

        ImGui::Text(tr("Y - Axis"));
        ImGui::SetNextItemWidth(-1);
        if (es_reaccion) {
            if (es_electroquimica) {
                ImGui::Combo("##y_axis_rxn", &y_axis_mode_, "Log K\0dH (kJ)\0dS (J/K)\0dG (kJ)\0E vs SHE (V)\0");
            } else {
                ImGui::Combo("##y_axis_rxn", &y_axis_mode_, "Log K\0dH (kJ)\0dS (J/K)\0dG (kJ)\0");
            }
        } else {
            ImGui::Combo("##y_axis_form", &y_axis_mode_, "Cp (J/mol*K)\0H (kJ/mol)\0S (J/mol*K)\0G (kJ/mol)\0");
        }

        ImGui::Checkbox(tr("Collect Data"), &collect_data_);
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("When on, each Calculate adds a new curve to the chart");
        ImGui::Checkbox(tr("Plot All Data"), &plot_all_data_);
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("When off, only the most recent curve is plotted");

        ImGui::Spacing();
        if (ImGui::Button(tr("Open in new Window"), ImVec2(-1, 0)) && !curvas_.empty()) {
            ventana_grafico_ = true;
        }
        if (ImGui::Button(tr("Copy to Clipboard"), ImVec2(-1, 0))) {
            if (calculado_) {
                std::ostringstream clip;
                if (es_reaccion) {
                    clip << "Reaction: " << ecuacion_ << "\n";
                    if (es_electroquimica)
                        clip << "T (°C)\tdH (kJ)\tdS (J/K)\tdG (kJ)\tK\tLog K\tE vs SHE (V)\n";
                    else
                        clip << "T (°C)\tdH (kJ)\tdS (J/K)\tdG (kJ)\tK\tLog K\n";
                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        clip << tc << "\t" << calc_dH_T(rxn_actual, tk, db)
                             << "\t" << calc_dS_T(rxn_actual, tk, db)
                             << "\t" << calc_dG_T(rxn_actual, tk, db)
                             << "\t" << calc_Keq_T(rxn_actual, tk, db)
                             << "\t" << calc_logK_T(rxn_actual, tk, db);
                        if (es_electroquimica)
                            clip << "\t" << calc_E0_she(calc_dG_T(rxn_actual, tk, db), n_electrones);
                        clip << "\n";
                    }
                } else if (esp_actual) {
                    clip << "Formula: " << esp_actual->formula << "\n";
                    clip << "T (°C)\tCp (J/mol*K)\tH (kJ/mol)\tS (J/mol*K)\tG (kJ/mol)\n";
                    double step = (temp_paso_ <= 0.0) ? 25.0 : temp_paso_;
                    for (double tc = temp_desde_; tc <= temp_hasta_; tc += step) {
                        double tk = tc + 273.15;
                        clip << tc << "\t" << esp_actual->cp(tk)
                             << "\t" << esp_actual->entalpia_abs(tk)
                             << "\t" << esp_actual->entropia_abs(tk)
                             << "\t" << esp_actual->energia_gibbs(tk) << "\n";
                    }
                }
                ImGui::SetClipboardText(clip.str().c_str());
            }
        }

        ImGui::Spacing();
        ImGui::Separator();
        ImGui::Spacing();

        if (!curvas_.empty()) {
            dibujar_grafico(ImVec2(-1, 240), es_reaccion);

            // Leyenda con checkboxes por curva (como en HSC)
            ImGui::Spacing();
            for (size_t i = 0; i < curvas_.size(); ++i) {
                if (curvas_[i].para_reaccion != es_reaccion) continue;
                ImGui::PushID((int)i + 7100);
                ImGui::Checkbox(curvas_[i].etiqueta.c_str(), &curvas_[i].visible);
                ImGui::PopID();
            }
            if (ImGui::SmallButton(tr("Clear Curves"))) {
                curvas_.clear();
            }
        } else {
            ImGui::TextDisabled(tr("Run calculations to generate plots."));
        }
    }
    ImGui::EndChild();

    ImGui::Columns(1);

    // ── Ventana de gráfico desacoplada ("Open in new Window") ──
    if (ventana_grafico_) {
        ImGui::SetNextWindowSize(ImVec2(720, 520), ImGuiCond_FirstUseEver);
        if (ImGui::Begin("Reaction Equations - Chart##detached", &ventana_grafico_)) {
            dibujar_grafico(ImVec2(-1, -40), es_reaccion);
            if (ImGui::SmallButton(tr("Close"))) {
                ventana_grafico_ = false;
            }
        }
        ImGui::End();
    }
}

// ═══════════════════════════════════════════════════════════════════════════

}} // namespace
