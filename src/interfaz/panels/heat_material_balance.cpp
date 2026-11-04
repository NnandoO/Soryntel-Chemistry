#include "shared.hpp"
#include <algorithm>
#include <cmath>
#include <cstdio>
#include <sstream>

namespace cma { namespace interfaz {

namespace {

const char* kModalHMB = "Buscar Especie (HMB)";

ImVec4 color_ok()   { return cma::interfaz::AdaptiveColor(0.30f, 0.85f, 0.40f, 1.0f); }
ImVec4 color_malo() { return cma::interfaz::AdaptiveColor(0.90f, 0.40f, 0.30f, 1.0f); }
ImVec4 color_aviso(){ return cma::interfaz::AdaptiveColor(0.95f, 0.75f, 0.25f, 1.0f); }

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
PanelBalanceHoja::PanelBalanceHoja() {
    inicializar_valores_defecto();
}

const char* PanelBalanceHoja::etiqueta_energia() const {
    switch (energy_unit_) {
        case 0: return "Mcal";
        case 1: return "MJ";
        case 2: return "kWh";
        case 3: return "kJ";
        case 4: return "kcal";
        default: return "kWh";
    }
}

const char* PanelBalanceHoja::etiqueta_temperatura() const {
    return (temp_unit_ == 0) ? "°C" : "K";
}

// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::inicializar_valores_defecto() {
    cargar_preset(0);
}

void PanelBalanceHoja::cargar_preset(int idx) {
    preset_idx_ = idx;
    entradas_.clear();
    salidas_.clear();
    areas_ui_.clear();
    area_activa_ = 0;
    factor_escala_reactiva_ = 1.0;

    auto nueva_fila = [](const char* formula, double T, double P, double kmol) {
        FilaUI f;
        copiar_texto(f.formula, sizeof(f.formula), formula);
        f.temper = T;
        f.presion = P;
        f.kmol = kmol;
        return f;
    };

    if (idx == 0) {
        // ── Preset 0: Referencia HSC Heat and Material Balance Calculator ──
        // Combustión de CH4(g) con exceso de aire y circuito de agua de enfriamiento
        // (Gas Out = 1238.938 °C, Cooling Water Out = 90.000 °C [Fija], unidad = kWh)
        temp_unit_ = 0;   // °C
        energy_unit_ = 2; // kWh

        CorrienteUI gas_in;
        copiar_texto(gas_in.nombre, sizeof(gas_in.nombre), "Fuel Gas (CH4)");
        gas_in.filas.push_back(nueva_fila("CH4(g)", 25.0, 1.0, 1.000 / 22.414));
        entradas_.push_back(gas_in);

        CorrienteUI air_in;
        copiar_texto(air_in.nombre, sizeof(air_in.nombre), "Combustion Air");
        air_in.filas.push_back(nueva_fila("O2(g)", 25.0, 1.0, 3.600 / 22.414));
        air_in.filas.push_back(nueva_fila("N2(g)", 25.0, 1.0, 13.543 / 22.414));
        entradas_.push_back(air_in);

        CorrienteUI cw_in;
        copiar_texto(cw_in.nombre, sizeof(cw_in.nombre), "Cooling Water In");
        cw_in.filas.push_back(nueva_fila("H2O(l)", 25.0, 1.0, 100.000 / 18.015));
        entradas_.push_back(cw_in);

        CorrienteUI gas_out;
        copiar_texto(gas_out.nombre, sizeof(gas_out.nombre), "Gas Out");
        gas_out.temperatura_fija = false;
        gas_out.filas.push_back(nueva_fila("CO2(g)", 1238.938, 1.0, 1.000 / 22.414));
        gas_out.filas.push_back(nueva_fila("H2O(g)", 1238.938, 1.0, 2.000 / 22.414));
        gas_out.filas.push_back(nueva_fila("N2(g)", 1238.938, 1.0, 13.543 / 22.414));
        gas_out.filas.push_back(nueva_fila("O2(g)", 1238.938, 1.0, 1.600 / 22.414));
        salidas_.push_back(gas_out);

        CorrienteUI cw_out;
        copiar_texto(cw_out.nombre, sizeof(cw_out.nombre), "Cooling Water Out");
        cw_out.temperatura_fija = true; // Fija a 90 °C como en HSC
        cw_out.filas.push_back(nueva_fila("H2O(l)", 90.000, 1.0, 100.000 / 18.015));
        salidas_.push_back(cw_out);

        vista_ = Vista::OUT_Global;
        lado_salida_sel_ = true;
        corriente_sel_ = 0;
        fila_sel_ = -1; // Cabecera "Gas Out" activa (B2 = 1238.938)
        col_sel_ = 1;
        diag_entrada_ = 0;
        diag_fila_ = 0;
        diag_unidad_x_ = 2; // Nm³
        diag_acoplar_esteq_ = true;
        diag_min_ = 1.0;
        diag_max_ = 12.0;
        factor_escala_reactiva_ = 12.0; // Punto final del barrido Diagram 1..12 Nm³ en HSC

        recalcular(true);
        temp_balance();
    } else if (idx == 1) {
        // ── Preset 1: Caldera de Gas Natural (3.3 kmol CH4, humos a 350 °C) ──
        temp_unit_ = 0;
        energy_unit_ = 3; // kJ

        CorrienteUI gas;
        copiar_texto(gas.nombre, sizeof(gas.nombre), "Gas natural (CH4)");
        gas.filas.push_back(nueva_fila("CH4(g)", 25.0, 1.0, 3.300));
        entradas_.push_back(gas);

        CorrienteUI aire;
        copiar_texto(aire.nombre, sizeof(aire.nombre), "Aire de combustión");
        aire.filas.push_back(nueva_fila("O2(g)", 25.0, 1.0, 7.350));
        aire.filas.push_back(nueva_fila("N2(g)", 25.0, 1.0, 27.650));
        entradas_.push_back(aire);

        CorrienteUI humos;
        copiar_texto(humos.nombre, sizeof(humos.nombre), "Gases de escape");
        humos.filas.push_back(nueva_fila("CO2(g)", 350.0, 1.0, 3.300));
        humos.filas.push_back(nueva_fila("H2O(g)", 350.0, 1.0, 6.600));
        humos.filas.push_back(nueva_fila("N2(g)", 350.0, 1.0, 27.650));
        humos.filas.push_back(nueva_fila("O2(g)", 350.0, 1.0, 0.750));
        salidas_.push_back(humos);

        vista_ = Vista::Balance;
        lado_salida_sel_ = false;
        corriente_sel_ = 0;
        fila_sel_ = 0;
        recalcular(true);
        if (!salidas_.empty() && !area_.salidas.empty()) {
            motor_.ajustar_calor_de_area(area_);
            salidas_.front().calor = motores::energia_desde_kJ(
                area_.salidas.front().calor_kJ, unidad_energia());
            recalcular(false);
        }
    } else if (idx == 2) {
        // ── Preset 2: Tostación de Pirita (2 FeS2 + 5.5 O2 -> Fe2O3 + 4 SO2) ──
        temp_unit_ = 0;
        energy_unit_ = 1; // MJ

        CorrienteUI min_in;
        copiar_texto(min_in.nombre, sizeof(min_in.nombre), "Concentrado Pirita");
        min_in.filas.push_back(nueva_fila("FeS2", 25.0, 1.0, 2.000));
        entradas_.push_back(min_in);

        CorrienteUI aire_in;
        copiar_texto(aire_in.nombre, sizeof(aire_in.nombre), "Aire de Tostación");
        aire_in.filas.push_back(nueva_fila("O2(g)", 25.0, 1.0, 6.000));
        aire_in.filas.push_back(nueva_fila("N2(g)", 25.0, 1.0, 22.560));
        entradas_.push_back(aire_in);

        CorrienteUI calcina;
        copiar_texto(calcina.nombre, sizeof(calcina.nombre), "Calcina (Fe2O3)");
        calcina.temperatura_fija = true;
        calcina.filas.push_back(nueva_fila("Fe2O3", 850.0, 1.0, 1.000));
        salidas_.push_back(calcina);

        CorrienteUI gas_so2;
        copiar_texto(gas_so2.nombre, sizeof(gas_so2.nombre), "Gases SO2");
        gas_so2.filas.push_back(nueva_fila("SO2(g)", 850.0, 1.0, 4.000));
        gas_so2.filas.push_back(nueva_fila("O2(g)", 850.0, 1.0, 0.500));
        gas_so2.filas.push_back(nueva_fila("N2(g)", 850.0, 1.0, 22.560));
        salidas_.push_back(gas_so2);

        vista_ = Vista::OUT_Global;
        recalcular(true);
        temp_balance();
    } else {
        // ── Preset 3: Reducción de Hematita en Alto Horno (Fe2O3 + 3CO -> 2Fe + 3CO2) ──
        temp_unit_ = 0;
        energy_unit_ = 2; // kWh

        CorrienteUI carga;
        copiar_texto(carga.nombre, sizeof(carga.nombre), "Mineral Hematita");
        carga.filas.push_back(nueva_fila("Fe2O3", 25.0, 1.0, 1.000));
        entradas_.push_back(carga);

        CorrienteUI gas_red;
        copiar_texto(gas_red.nombre, sizeof(gas_red.nombre), "Gas Reductor (CO + N2)");
        gas_red.filas.push_back(nueva_fila("CO(g)", 900.0, 2.0, 3.500));
        gas_red.filas.push_back(nueva_fila("N2(g)", 900.0, 2.0, 6.500));
        entradas_.push_back(gas_red);

        CorrienteUI arrabio;
        copiar_texto(arrabio.nombre, sizeof(arrabio.nombre), "Hierro Reducido (Fe)");
        arrabio.temperatura_fija = true;
        arrabio.filas.push_back(nueva_fila("Fe", 1450.0, 1.0, 2.000));
        salidas_.push_back(arrabio);

        CorrienteUI gas_top;
        copiar_texto(gas_top.nombre, sizeof(gas_top.nombre), "Gas de Tragante");
        gas_top.filas.push_back(nueva_fila("CO2(g)", 250.0, 1.5, 3.000));
        gas_top.filas.push_back(nueva_fila("CO(g)", 250.0, 1.5, 0.500));
        gas_top.filas.push_back(nueva_fila("N2(g)", 250.0, 1.5, 6.500));
        salidas_.push_back(gas_top);

        vista_ = Vista::IN_Global;
        recalcular(true);
    }

    AreaUI a1;
    a1.nombre = "Balance Area 1";
    a1.entradas = entradas_;
    a1.salidas = salidas_;
    areas_ui_.push_back(a1);
}

// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::cambiar_area_activa(int nueva_area) {
    if (areas_ui_.empty()) {
        AreaUI a1;
        a1.nombre = "Balance Area 1";
        a1.entradas = entradas_;
        a1.salidas = salidas_;
        areas_ui_.push_back(a1);
        area_activa_ = 0;
    }
    if (nueva_area < 0 || nueva_area >= static_cast<int>(areas_ui_.size())) return;
    if (area_activa_ >= 0 && area_activa_ < static_cast<int>(areas_ui_.size())) {
        areas_ui_[static_cast<size_t>(area_activa_)].entradas = entradas_;
        areas_ui_[static_cast<size_t>(area_activa_)].salidas = salidas_;
    }
    area_activa_ = nueva_area;
    entradas_ = areas_ui_[static_cast<size_t>(area_activa_)].entradas;
    salidas_ = areas_ui_[static_cast<size_t>(area_activa_)].salidas;
    corriente_sel_ = 0;
    fila_sel_ = -1;
    recalcular(true);
}

void PanelBalanceHoja::insertar_area_balance() {
    if (areas_ui_.empty()) cambiar_area_activa(0);
    if (area_activa_ >= 0 && area_activa_ < static_cast<int>(areas_ui_.size())) {
        areas_ui_[static_cast<size_t>(area_activa_)].entradas = entradas_;
        areas_ui_[static_cast<size_t>(area_activa_)].salidas = salidas_;
    }
    AreaUI nueva;
    nueva.nombre = "Balance Area " + std::to_string(areas_ui_.size() + 1);
    CorrienteUI cin, cout;
    copiar_texto(cin.nombre, sizeof(cin.nombre), "Input Stream 1");
    FilaUI fin;
    copiar_texto(fin.formula, sizeof(fin.formula), "H2O(l)");
    fin.temper = (temp_unit_ == 0) ? 25.0 : 298.15;
    fin.presion = 1.0;
    fin.kmol = 1.0;
    cin.filas.push_back(fin);
    copiar_texto(cout.nombre, sizeof(cout.nombre), "Output Stream 1");
    cout.filas.push_back(fin);
    nueva.entradas.push_back(cin);
    nueva.salidas.push_back(cout);
    areas_ui_.push_back(nueva);
    cambiar_area_activa(static_cast<int>(areas_ui_.size()) - 1);
}

void PanelBalanceHoja::eliminar_area_balance() {
    if (areas_ui_.size() <= 1) return;
    areas_ui_.erase(areas_ui_.begin() + area_activa_);
    for (size_t i = 0; i < areas_ui_.size(); ++i) {
        areas_ui_[i].nombre = "Balance Area " + std::to_string(i + 1);
    }
    area_activa_ = std::clamp(area_activa_, 0, static_cast<int>(areas_ui_.size()) - 1);
    entradas_ = areas_ui_[static_cast<size_t>(area_activa_)].entradas;
    salidas_ = areas_ui_[static_cast<size_t>(area_activa_)].salidas;
    recalcular(true);
}

// ─────────────────────────────────────────────────────────────────────────────
motores::AreaBalance PanelBalanceHoja::area_escalada_operacion() const {
    if (std::abs(factor_escala_reactiva_ - 1.0) <= 1e-6) return area_;
    motores::AreaBalance copia = area_;
    bool hay_salida_fija = false;
    for (const auto& cs : copia.salidas) {
        if (cs.temperatura_fija) {
            hay_salida_fija = true;
            break;
        }
    }
    for (auto& ce : copia.entradas) {
        const bool es_agua_enfr = (ce.nombre.find("Water") != std::string::npos ||
                                   ce.nombre.find("Agua") != std::string::npos);
        if (hay_salida_fija && es_agua_enfr) continue;
        for (auto& f : ce.filas) f.kmol *= factor_escala_reactiva_;
    }
    for (auto& cs : copia.salidas) {
        if (hay_salida_fija && cs.temperatura_fija) continue;
        for (auto& f : cs.filas) f.kmol *= factor_escala_reactiva_;
    }
    const_cast<motores::BalanceHoja&>(motor_).actualizar(
        copia, motores::BalanceHoja::ResolvedorEspecie{});
    return copia;
}

void PanelBalanceHoja::sincronizar_motor(bool resolver) {
    auto copiar = [&](const std::vector<CorrienteUI>& ui,
                      std::vector<motores::CorrienteBalance>& dst) {
        std::vector<motores::CorrienteBalance> nuevo;
        nuevo.reserve(ui.size());
        for (size_t i = 0; i < ui.size(); ++i) {
            motores::CorrienteBalance cb;
            cb.nombre = ui[i].nombre;
            cb.calor_kJ = motores::energia_a_kJ(ui[i].calor, unidad_energia());
            cb.temperatura_fija = ui[i].temperatura_fija;
            cb.filas.reserve(ui[i].filas.size());
            for (size_t j = 0; j < ui[i].filas.size(); ++j) {
                const FilaUI& fu = ui[i].filas[j];
                motores::FilaBalance fb;
                fb.formula = fu.formula;
                fb.temperatura_K = motores::temperatura_a_K(fu.temper, unidad_temperatura());
                fb.presion_bar = fu.presion;
                fb.kmol = fu.kmol;
                if (i < dst.size() && j < dst[i].filas.size() &&
                    dst[i].filas[j].formula == fb.formula) {
                    fb.especie = dst[i].filas[j].especie;
                }
                cb.filas.push_back(std::move(fb));
            }
            nuevo.push_back(std::move(cb));
        }
        dst = std::move(nuevo);
    };

    copiar(entradas_, area_.entradas);
    copiar(salidas_, area_.salidas);

    if (resolver) {
        auto resolver_fn = [](const std::string& etiqueta) -> const nucleo::Especie* {
            return buscar_especie_por_nombre_o_formula(etiqueta);
        };
        motor_.actualizar(area_, resolver_fn);
    } else {
        motor_.actualizar(area_, motores::BalanceHoja::ResolvedorEspecie{});
    }

    const motores::AreaBalance area_op = area_escalada_operacion();
    resultado_ = motor_.evaluar(area_op);
    elem_balance_ = motor_.balance_elemental(area_op);
    diag_dirty_ = true;

    if (!areas_ui_.empty() && area_activa_ >= 0 &&
        area_activa_ < static_cast<int>(areas_ui_.size())) {
        areas_ui_[static_cast<size_t>(area_activa_)].entradas = entradas_;
        areas_ui_[static_cast<size_t>(area_activa_)].salidas = salidas_;
    }
}

void PanelBalanceHoja::recalcular(bool resolver) {
    sincronizar_motor(resolver);
}

void PanelBalanceHoja::copiar_temperaturas_desde_motor() {
    auto copiar = [&](std::vector<CorrienteUI>& ui,
                      const std::vector<motores::CorrienteBalance>& src) {
        for (size_t i = 0; i < ui.size() && i < src.size(); ++i) {
            for (size_t j = 0; j < ui[i].filas.size() && j < src[i].filas.size(); ++j) {
                ui[i].filas[j].temper = motores::temperatura_desde_K(
                    src[i].filas[j].temperatura_K, unidad_temperatura());
            }
        }
    };
    copiar(entradas_, area_.entradas);
    copiar(salidas_, area_.salidas);
}

void PanelBalanceHoja::temp_balance() {
    sincronizar_motor(true);
    motores::AreaBalance area_op = area_escalada_operacion();
    ultimo_solver_ = motor_.temperatura_equilibrio(area_op, mantener_offsets_);
    if (ultimo_solver_.factible) {
        for (size_t i = 0; i < area_.salidas.size() && i < area_op.salidas.size(); ++i) {
            for (size_t j = 0; j < area_.salidas[i].filas.size() &&
                               j < area_op.salidas[i].filas.size(); ++j) {
                area_.salidas[i].filas[j].temperatura_K =
                    area_op.salidas[i].filas[j].temperatura_K;
            }
        }
        copiar_temperaturas_desde_motor();
    }
    recalcular(false);
}

void PanelBalanceHoja::ajustar_calor() {
    sincronizar_motor(true);
    motor_.ajustar_calor_de_area(area_);
    if (!salidas_.empty() && !area_.salidas.empty()) {
        salidas_.front().calor = motores::energia_desde_kJ(
            area_.salidas.front().calor_kJ, unidad_energia());
    }
    recalcular(false);
}

void PanelBalanceHoja::agregar_corriente(bool salida) {
    auto& grupos = lista(salida);
    CorrienteUI c;
    std::snprintf(c.nombre, sizeof(c.nombre), "%s %d",
                  salida ? "Output Stream" : "Input Stream",
                  static_cast<int>(grupos.size()) + 1);
    FilaUI f;
    f.temper = (temp_unit_ == 0) ? 25.0 : 298.15;
    c.filas.push_back(f);
    grupos.push_back(c);
    lado_salida_sel_ = salida;
    corriente_sel_ = static_cast<int>(grupos.size()) - 1;
    fila_sel_ = 0;
    vista_ = salida ? Vista::OUT_Global : Vista::IN_Global;
    recalcular(true);
}

void PanelBalanceHoja::aplicar_accion_pendiente() {
    if (accion_pendiente_ == AccionPendiente::Ninguna) return;

    auto& grupos = lista(lado_salida_sel_);
    const int idx = corriente_sel_;
    const int fila = fila_sel_;

    switch (accion_pendiente_) {
        case AccionPendiente::BorrarFila:
            if (idx >= 0 && idx < static_cast<int>(grupos.size()) &&
                fila >= 0 && fila < static_cast<int>(grupos[idx].filas.size())) {
                grupos[idx].filas.erase(grupos[idx].filas.begin() + fila);
                fila_sel_ = std::min(fila, static_cast<int>(grupos[idx].filas.size()) - 1);
            }
            break;
        case AccionPendiente::DuplicarFila:
            if (idx >= 0 && idx < static_cast<int>(grupos.size()) &&
                fila >= 0 && fila < static_cast<int>(grupos[idx].filas.size())) {
                grupos[idx].filas.insert(grupos[idx].filas.begin() + fila + 1,
                                         grupos[idx].filas[static_cast<size_t>(fila)]);
                fila_sel_ = fila + 1;
            }
            break;
        case AccionPendiente::MoverArriba:
            if (idx >= 0 && idx < static_cast<int>(grupos.size()) && fila > 0 &&
                fila < static_cast<int>(grupos[idx].filas.size())) {
                std::swap(grupos[idx].filas[static_cast<size_t>(fila)],
                          grupos[idx].filas[static_cast<size_t>(fila) - 1]);
                fila_sel_ = fila - 1;
            }
            break;
        case AccionPendiente::MoverAbajo:
            if (idx >= 0 && idx < static_cast<int>(grupos.size()) && fila >= 0 &&
                fila + 1 < static_cast<int>(grupos[idx].filas.size())) {
                std::swap(grupos[idx].filas[static_cast<size_t>(fila)],
                          grupos[idx].filas[static_cast<size_t>(fila) + 1]);
                fila_sel_ = fila + 1;
            }
            break;
        case AccionPendiente::InsertarDebajo:
            if (idx >= 0 && idx < static_cast<int>(grupos.size())) {
                FilaUI nf;
                nf.temper = (temp_unit_ == 0) ? 25.0 : 298.15;
                const size_t pos = static_cast<size_t>(std::max(fila + 1, 0));
                const size_t limite = grupos[idx].filas.size();
                grupos[idx].filas.insert(grupos[idx].filas.begin() +
                                             static_cast<long>(std::min(pos, limite)),
                                         nf);
                fila_sel_ = static_cast<int>(std::min(pos, limite));
            }
            break;
        case AccionPendiente::BorrarCorriente:
            if (idx >= 0 && idx < static_cast<int>(grupos.size())) {
                grupos.erase(grupos.begin() + idx);
                corriente_sel_ = std::max(0, std::min(idx,
                    static_cast<int>(grupos.size()) - 1));
                fila_sel_ = -1;
            }
            break;
        case AccionPendiente::DuplicarCorriente:
            if (idx >= 0 && idx < static_cast<int>(grupos.size())) {
                CorrienteUI copia = grupos[idx];
                std::snprintf(copia.nombre, sizeof(copia.nombre), "%.55s (copia)",
                              grupos[idx].nombre);
                grupos.insert(grupos.begin() + idx + 1, copia);
                corriente_sel_ = idx + 1;
            }
            break;
        case AccionPendiente::Ninguna:
            break;
    }

    accion_pendiente_ = AccionPendiente::Ninguna;
    recalcular(true);
}

// ─────────────────────────────────────────────────────────────────────────────
// Barra de fórmula estilo HSC (Active species + Celda + Valor en doble precisión)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_barra_formula() {
    auto& grupos = lista(lado_salida_sel_);
    if (grupos.empty()) return;
    const int c_idx = std::clamp(corriente_sel_, 0, static_cast<int>(grupos.size()) - 1);
    CorrienteUI& corr = grupos[static_cast<size_t>(c_idx)];

    // Calcula el número de fila en la hoja global (2 = cabecera de corriente 0, etc.)
    int row_num = 2;
    for (int i = 0; i < c_idx; ++i) {
        row_num += 1 + static_cast<int>(grupos[static_cast<size_t>(i)].filas.size());
    }
    const bool es_cabecera = (fila_sel_ < 0 || fila_sel_ >= static_cast<int>(corr.filas.size()));
    if (!es_cabecera) row_num += 1 + fila_sel_;

    const char letras_col[] = {'A', 'B', 'C', 'D', 'E', 'F'};
    const char col_ch = letras_col[std::clamp(col_sel_, 0, 5)];
    char ref_celda[16];
    std::snprintf(ref_celda, sizeof(ref_celda), "%c%d", col_ch, row_num);

    const char* etiqueta_activa = es_cabecera
                                      ? corr.nombre
                                      : corr.filas[static_cast<size_t>(fila_sel_)].formula;

    ImGui::AlignTextToFramePadding();
    ImGui::TextDisabled("Active species:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(175.0f);
    char buf_activo[64];
    copiar_texto(buf_activo, sizeof(buf_activo), etiqueta_activa);
    ImGui::BeginDisabled();
    ImGui::InputText("##active_sp_box", buf_activo, sizeof(buf_activo), ImGuiInputTextFlags_ReadOnly);
    ImGui::EndDisabled();

    ImGui::SameLine();
    ImGui::SetNextItemWidth(56.0f);
    ImGui::BeginDisabled();
    ImGui::InputText("##cell_ref_box", ref_celda, sizeof(ref_celda), ImGuiInputTextFlags_ReadOnly);
    ImGui::EndDisabled();

    ImGui::SameLine();
    ImGui::SetNextItemWidth(220.0f);
    const auto& motor_grupos = lado_salida_sel_ ? area_.salidas : area_.entradas;
    const motores::CorrienteBalance* mc =
        (c_idx < static_cast<int>(motor_grupos.size()))
            ? &motor_grupos[static_cast<size_t>(c_idx)]
            : nullptr;

    if (es_cabecera) {
        double t_cab = corr.filas.empty() ? 25.0 : corr.filas.front().temper;
        if (ImGui::InputDouble("##formula_bar_val", &t_cab, 0, 0, "%.11g")) {
            for (auto& f : corr.filas) f.temper = t_cab;
            recalcular(false);
        }
    } else {
        FilaUI& f = corr.filas[static_cast<size_t>(fila_sel_)];
        const motores::FilaBalance* mf =
            (mc != nullptr && fila_sel_ >= 0 && fila_sel_ < static_cast<int>(mc->filas.size()))
                ? &mc->filas[static_cast<size_t>(fila_sel_)]
                : nullptr;
        if (col_sel_ == 2) {
            if (ImGui::InputDouble("##formula_bar_val", &f.presion, 0, 0, "%.11g"))
                recalcular(false);
        } else if (col_sel_ == 3) {
            if (ImGui::InputDouble("##formula_bar_val", &f.kmol, 0, 0, "%.11g")) {
                factor_escala_reactiva_ = 1.0;
                recalcular(false);
            }
        } else if (col_sel_ == 4) {
            double val_kg = (mf != nullptr) ? mf->masa_kg : 0.0;
            if (ImGui::InputDouble("##formula_bar_val", &val_kg, 0, 0, "%.11g")) {
                if (mf != nullptr && mf->especie != nullptr && mf->especie->masa_molar > 0.0) {
                    f.kmol = std::max(0.0, val_kg / mf->especie->masa_molar);
                    factor_escala_reactiva_ = 1.0;
                    recalcular(false);
                }
            }
        } else if (col_sel_ == 5) {
            double val_nm3 = (mf != nullptr) ? mf->volumen_nm3 : 0.0;
            if (ImGui::InputDouble("##formula_bar_val", &val_nm3, 0, 0, "%.11g")) {
                if (mf != nullptr && mf->especie != nullptr) {
                    const double vm = motores::FilaBalance::volumen_molar_nm3(mf->especie);
                    if (vm > 0.0) {
                        f.kmol = std::max(0.0, val_nm3 / vm);
                        factor_escala_reactiva_ = 1.0;
                        recalcular(false);
                    }
                }
            }
        } else {
            if (ImGui::InputDouble("##formula_bar_val", &f.temper, 0, 0, "%.11g"))
                recalcular(false);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Paneles inferiores: Element Balance & Total Balance (como en HSC)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_tabla_elementos() {
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f),
                       "Element Balance");
    ImGui::SameLine();
    if (ImGui::SmallButton(elem_bal_kg_ ? "Ver kmol" : "Ver kg")) {
        elem_bal_kg_ = !elem_bal_kg_;
    }

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollY | ImGuiTableFlags_SizingStretchProp;
    if (elem_bal_kg_) {
        if (ImGui::BeginTable("hmb_elem_table_kg", 5, flags, ImVec2(0, 0))) {
            ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 55.0f);
            ImGui::TableSetupColumn("IN (kg)");
            ImGui::TableSetupColumn("OUT (kg)");
            ImGui::TableSetupColumn("Balance");
            ImGui::TableSetupColumn("Error %", ImGuiTableColumnFlags_WidthFixed, 55.0f);
            ImGui::TableHeadersRow();

            for (const auto& b : elem_balance_) {
                const bool ok = std::abs(b.error_pct()) < 0.01;
                const double bal_kg = b.out_kg - b.in_kg;
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text("%s", b.elemento.c_str());
                ImGui::TableNextColumn(); ImGui::Text("%.3f", b.in_kg);
                ImGui::TableNextColumn(); ImGui::Text("%.3f", b.out_kg);
                ImGui::TableNextColumn();
                ImGui::TextColored(ok ? color_ok() : color_malo(), "%.3f",
                                   std::abs(bal_kg) < 5e-4 ? 0.0 : bal_kg);
                ImGui::TableNextColumn();
                ImGui::TextColored(ok ? color_ok() : color_malo(), "%.2f", b.error_pct());
            }
            ImGui::EndTable();
        }
    } else {
        // Modo por defecto HSC: Element | IN kmol | OUT kmol | Balance kmol
        if (ImGui::BeginTable("hmb_elem_table_kmol", 4, flags, ImVec2(0, 0))) {
            ImGui::TableSetupColumn("Element", ImGuiTableColumnFlags_WidthFixed, 60.0f);
            ImGui::TableSetupColumn("IN (kmol)");
            ImGui::TableSetupColumn("OUT (kmol)");
            ImGui::TableSetupColumn("Balance");
            ImGui::TableHeadersRow();

            for (const auto& b : elem_balance_) {
                const bool ok = std::abs(b.error_pct()) < 0.01;
                const double bal_kmol = b.out_kmol_at - b.in_kmol_at;
                ImGui::TableNextRow();
                ImGui::TableNextColumn(); ImGui::Text("%s", b.elemento.c_str());
                ImGui::TableNextColumn(); ImGui::Text("%.3f", b.in_kmol_at);
                ImGui::TableNextColumn(); ImGui::Text("%.3f", b.out_kmol_at);
                ImGui::TableNextColumn();
                ImGui::TextColored(ok ? color_ok() : color_malo(), "%.3f",
                                   std::abs(bal_kmol) < 5e-4 ? 0.0 : bal_kmol);
            }
            ImGui::EndTable();
        }
    }
}

void PanelBalanceHoja::dibujar_resumen_total() {
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.7f, 0.6f, 1.0f),
                       "Total Balance");
    const double ue_kj = motores::factor_energia_kJ(unidad_energia());
    const char* ue = etiqueta_energia();

    char h_hc[48], h_ht[48], h_ex[48];
    std::snprintf(h_hc, sizeof(h_hc), "Heat Content\n%s", ue);
    std::snprintf(h_ht, sizeof(h_ht), "Total H\n%s", ue);
    std::snprintf(h_ex, sizeof(h_ex), "Exergy\n%s", ue);

    const ImGuiTableFlags flags = ImGuiTableFlags_Borders | ImGuiTableFlags_RowBg |
                                  ImGuiTableFlags_ScrollY | ImGuiTableFlags_SizingStretchProp;
    if (ImGui::BeginTable("hmb_total_table", 7, flags, ImVec2(0, 0))) {
        ImGui::TableSetupColumn("BALANCE\nAREAS", ImGuiTableColumnFlags_WidthFixed, 88.0f);
        ImGui::TableSetupColumn("Amount\nkmol");
        ImGui::TableSetupColumn("Amount\nkg");
        ImGui::TableSetupColumn("Amount\nNm3");
        ImGui::TableSetupColumn(h_hc);
        ImGui::TableSetupColumn(h_ht);
        ImGui::TableSetupColumn(h_ex);
        ImGui::TableHeadersRow();

        // Filas IN y OUT del área activa
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::TextDisabled("Inputs (%d)", area_activa_ + 1);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.kmol_entrada);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.masa_entrada_kg);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.nm3_entrada);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.h_sensible_entrada_kJ / ue_kj);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.h_entrada_kJ / ue_kj);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.exergia_entrada_kJ / ue_kj);

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::TextDisabled("Outputs (%d)", area_activa_ + 1);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.kmol_salida);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.masa_salida_kg);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.nm3_salida);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.h_sensible_salida_kJ / ue_kj);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.h_salida_kJ / ue_kj);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", resultado_.exergia_salida_kJ / ue_kj);

        // Fila Balance (1) estilo HSC: OUT - IN (+ calor externo en Total H)
        const double d_kmol = resultado_.kmol_salida - resultado_.kmol_entrada;
        const double d_kg   = resultado_.masa_salida_kg - resultado_.masa_entrada_kg;
        const double d_nm3  = resultado_.nm3_salida - resultado_.nm3_entrada;
        const double d_hc   = (resultado_.h_sensible_salida_kJ - resultado_.h_sensible_entrada_kJ) / ue_kj;
        const double d_ht   = -resultado_.residual_energia_kJ / ue_kj;
        const double d_ex   = (resultado_.exergia_salida_kJ - resultado_.exergia_entrada_kJ) / ue_kj;

        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text("Balance (%d)", area_activa_ + 1);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", std::abs(d_kmol) < 5e-4 ? 0.0 : d_kmol);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_masa_ok ? color_ok() : color_malo(),
                           "%.3f", std::abs(d_kg) < 5e-4 ? 0.0 : d_kg);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", std::abs(d_nm3) < 5e-4 ? 0.0 : d_nm3);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", d_hc);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_energia_ok ? color_ok() : color_malo(),
                           "%.3f", std::abs(d_ht) < 5e-4 ? 0.0 : d_ht);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", d_ex);

        // Fila Total Balance
        ImGui::TableNextRow(ImGuiTableRowFlags_Headers);
        ImGui::TableNextColumn(); ImGui::TextColored(color_aviso(), "Total Balance");
        ImGui::TableNextColumn(); ImGui::Text("%.3f", std::abs(d_kmol) < 5e-4 ? 0.0 : d_kmol);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_masa_ok ? color_ok() : color_malo(),
                           "%.3f", std::abs(d_kg) < 5e-4 ? 0.0 : d_kg);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", std::abs(d_nm3) < 5e-4 ? 0.0 : d_nm3);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", d_hc);
        ImGui::TableNextColumn();
        ImGui::TextColored(resultado_.balance_energia_ok ? color_ok() : color_malo(),
                           "%.3f", std::abs(d_ht) < 5e-4 ? 0.0 : d_ht);
        ImGui::TableNextColumn(); ImGui::Text("%.3f", d_ex);

        ImGui::EndTable();
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Herramientas (columna derecha - réplica de Tools en HSC Bal)
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::dibujar_herramientas() {
    ImGui::TextColored(cma::interfaz::AdaptiveColor(0.8f, 0.6f, 0.8f, 1.0f), "Tools");
    ImGui::Separator();
    ImGui::Spacing();

    // Balance Area
    ImGui::Text("Balance Area");
    if (areas_ui_.empty()) cambiar_area_activa(0);
    std::string items_areas;
    for (const auto& a : areas_ui_) {
        items_areas += a.nombre;
        items_areas.push_back('\0');
    }
    items_areas.push_back('\0');
    int sel_area = area_activa_;
    ImGui::SetNextItemWidth(-1.0f);
    if (ImGui::Combo("##view_bal_area", &sel_area, items_areas.c_str())) {
        cambiar_area_activa(sel_area);
    }

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Text("Get Species from Database");
    const float avail_w = ImGui::GetContentRegionAvail().x;
    const float half_w = (avail_w - 6.0f) * 0.5f;
    if (ImGui::Button("Input##db_in", ImVec2(half_w, 24))) {
        if (entradas_.empty()) agregar_corriente(false);
        modal_es_salida_ = false;
        modal_corriente_ = std::clamp(corriente_sel_, 0, static_cast<int>(entradas_.size()) - 1);
        modal_fila_ = -1;
        ModalBuscadorDB::instancia().abrir(kModalHMB);
    }
    ImGui::SameLine();
    if (ImGui::Button("Output##db_out", ImVec2(half_w, 24))) {
        if (salidas_.empty()) agregar_corriente(true);
        modal_es_salida_ = true;
        modal_corriente_ = std::clamp(corriente_sel_, 0, static_cast<int>(salidas_.size()) - 1);
        modal_fila_ = -1;
        ModalBuscadorDB::instancia().abrir(kModalHMB);
    }

    ImGui::Spacing();
    ImGui::Separator();

    // Insert y Delete en tabla de 2 columnas proporcionales
    if (ImGui::BeginTable("hsc_ins_del_tbl", 2, ImGuiTableFlags_SizingStretchSame)) {
        ImGui::TableNextRow();
        ImGui::TableNextColumn(); ImGui::Text("Insert");
        ImGui::TableNextColumn(); ImGui::Text("Delete");

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        if (ImGui::Button("Balance Area##ins", ImVec2(-1, 23))) insertar_area_balance();
        ImGui::TableNextColumn();
        if (ImGui::Button("Balance Area##del", ImVec2(-1, 23))) eliminar_area_balance();

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        if (ImGui::Button("Stream##ins", ImVec2(-1, 23))) agregar_corriente(lado_salida_sel_);
        ImGui::TableNextColumn();
        if (ImGui::Button("Sel. Stream##del", ImVec2(-1, 23))) accion_pendiente_ = AccionPendiente::BorrarCorriente;
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Eliminar corriente seleccionada");

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        if (ImGui::Button("Row##ins", ImVec2(-1, 23))) {
            auto& grupos = lista(lado_salida_sel_);
            if (!grupos.empty()) {
                const int ci = std::clamp(corriente_sel_, 0, static_cast<int>(grupos.size()) - 1);
                FilaUI f;
                f.temper = grupos[static_cast<size_t>(ci)].filas.empty()
                               ? ((temp_unit_ == 0) ? 25.0 : 298.15)
                               : grupos[static_cast<size_t>(ci)].filas.back().temper;
                grupos[static_cast<size_t>(ci)].filas.push_back(f);
                fila_sel_ = static_cast<int>(grupos[static_cast<size_t>(ci)].filas.size()) - 1;
                recalcular(false);
            }
        }
        ImGui::TableNextColumn();
        if (ImGui::Button("Sel. Row##del", ImVec2(-1, 23))) accion_pendiente_ = AccionPendiente::BorrarFila;
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Eliminar fila/especie seleccionada");

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        if (ImGui::Button("Heat Flow##ins", ImVec2(-1, 23))) {
            auto& grupos = lista(lado_salida_sel_);
            if (!grupos.empty()) {
                const int ci = std::clamp(corriente_sel_, 0, static_cast<int>(grupos.size()) - 1);
                if (std::abs(grupos[static_cast<size_t>(ci)].calor) < 1e-9)
                    grupos[static_cast<size_t>(ci)].calor = 10.0;
                recalcular(false);
            }
        }
        ImGui::TableNextColumn();
        if (ImGui::Button("Clear Heat##del", ImVec2(-1, 23))) {
            auto& grupos = lista(lado_salida_sel_);
            if (!grupos.empty()) {
                const int ci = std::clamp(corriente_sel_, 0, static_cast<int>(grupos.size()) - 1);
                grupos[static_cast<size_t>(ci)].calor = 0.0;
                recalcular(false);
            }
        }
        if (ImGui::IsItemHovered()) ImGui::SetTooltip("Poner a cero el calor externo de esta corriente");

        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::Separator();

    // Measure Units
    ImGui::Text("Measure Units");
    ImGui::Text("Temperature:");
    const int temp_prev = temp_unit_;
    ImGui::SameLine();
    ImGui::RadioButton("°C", &temp_unit_, 0);
    ImGui::SameLine();
    ImGui::RadioButton("K", &temp_unit_, 1);
    if (temp_unit_ != temp_prev) {
        auto conv = [&](double v) {
            const double K = (temp_prev == 0) ? v + 273.15 : v;
            return (temp_unit_ == 0) ? K - 273.15 : K;
        };
        for (auto* grupos : {&entradas_, &salidas_})
            for (auto& c : *grupos)
                for (auto& f : c.filas) f.temper = conv(f.temper);
        target_temp_ = conv(target_temp_);
        gem_temp_ = conv(gem_temp_);
        calor_calculado_ = false;
        recalcular(false);
    }

    ImGui::Text("Amount:");
    ImGui::SameLine();
    int amt_u = elem_bal_kg_ ? 1 : 0;
    if (ImGui::RadioButton("kmol", &amt_u, 0)) elem_bal_kg_ = false;
    ImGui::SameLine();
    if (ImGui::RadioButton("kg", &amt_u, 1)) elem_bal_kg_ = true;

    ImGui::Text("Energy:");
    const int energy_prev = energy_unit_;
    ImGui::RadioButton("Mcal", &energy_unit_, 0); ImGui::SameLine(0, 8);
    ImGui::RadioButton("MJ", &energy_unit_, 1);   ImGui::SameLine(0, 8);
    ImGui::RadioButton("kWh", &energy_unit_, 2);
    ImGui::RadioButton("kJ", &energy_unit_, 3);   ImGui::SameLine(0, 8);
    ImGui::RadioButton("kcal", &energy_unit_, 4);
    if (energy_unit_ != energy_prev) {
        auto conv = [&](double v) {
            return motores::energia_desde_kJ(
                motores::energia_a_kJ(v, static_cast<motores::UnidadEnergia>(energy_prev)),
                unidad_energia());
        };
        for (auto* grupos : {&entradas_, &salidas_})
            for (auto& c : *grupos) c.calor = conv(c.calor);
        calor_requerido_ = conv(calor_requerido_);
        recalcular(false);
    }

    ImGui::Spacing();
    ImGui::Separator();

    // Functions
    ImGui::Text("Functions");
    ImGui::Checkbox("Predictive Typing", &enable_predictive_typing_);
    const float btn_w = (ImGui::GetContentRegionAvail().x - 6.0f) * 0.5f;
    if (ImGui::Button("Temp. Balance", ImVec2(btn_w, 25))) temp_balance();
    ImGui::SameLine();
    if (ImGui::Button("Summary", ImVec2(btn_w, 25))) copiar_informe();
    if (ImGui::Button("Ajustar Calor", ImVec2(btn_w, 25))) ajustar_calor();
    ImGui::SameLine();
    if (ImGui::Button(show_diagram_ ? "Hide Diagram" : "Diagram", ImVec2(btn_w, 25))) {
        show_diagram_ = !show_diagram_;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Render principal
// ─────────────────────────────────────────────────────────────────────────────
void PanelBalanceHoja::renderizar(diagrama_flujo::Flowsheet* flowsheet) {
    if (corriente_sel_ < 0 ||
        corriente_sel_ >= static_cast<int>(lista(lado_salida_sel_).size())) {
        corriente_sel_ = 0;
    }

    // ── Barra de herramientas superior ──
    if (ImGui::Button(tr("Abrir"))) abrir_txt();
    ImGui::SameLine();
    if (ImGui::Button(tr("Guardar"))) guardar_txt();
    ImGui::SameLine();
    if (ImGui::Button(tr("Exportar CSV"))) exportar_csv();
    if (flowsheet != nullptr) {
        ImGui::SameLine();
        if (ImGui::Button(tr("Cargar del diagrama"))) cargar_flujos(flowsheet);
    }
    ImGui::SameLine();
    ImGui::SetNextItemWidth(std::clamp(ImGui::GetContentRegionAvail().x - 90.0f, 140.0f, 300.0f));
    const char* presets_nombres[] = {
        "HSC Ref: CH4 + Cooling Water (1238.9 °C)",
        "Caldera de Gas Natural (350 °C)",
        "Tostación de Pirita FeS2 (850 °C)",
        "Alto Horno Hematita Fe2O3 + CO"
    };
    if (ImGui::Combo("##preset_hmb", &preset_idx_, presets_nombres, 4)) {
        cargar_preset(preset_idx_);
    }
    ImGui::SameLine();
    if (ImGui::Button(tr("Limpiar"))) {
        entradas_.clear();
        salidas_.clear();
        corriente_sel_ = 0;
        fila_sel_ = -1;
        vista_ = Vista::Balance;
        recalcular(false);
    }

    // ── Barra de fórmula activa estilo HSC + indicadores de estado ──
    dibujar_barra_formula();
    ImGui::SameLine(0.0f, 16.0f);
    if (resultado_.balance_masa_ok)
        ImGui::TextColored(color_ok(), "Masa: OK");
    else
        ImGui::TextColored(color_malo(), "Masa: Δ=%.3f kg", resultado_.residual_masa_kg);
    ImGui::SameLine();
    if (resultado_.balance_energia_ok)
        ImGui::TextColored(color_ok(), "Energía: OK");
    else
        ImGui::TextColored(color_malo(), "Energía: Δ=%.2f %%", resultado_.error_energia_pct);
    ImGui::Separator();

    // ── Dos paneles: hoja (izquierda) | Tools (derecha, ancho fijo) ──
    const ImVec2 disponible = ImGui::GetContentRegionAvail();
    const float espacio = ImGui::GetStyle().ItemSpacing.x;
    const float ancho_tools = std::clamp(disponible.x * 0.24f, 200.0f, 260.0f);
    const float ancho_hoja = std::max(disponible.x - ancho_tools - espacio, 240.0f);
    const float alto_panel = disponible.y;

    // ───────── Panel izquierdo ─────────
    ImGui::BeginChild("hmb_izquierda", ImVec2(ancho_hoja, alto_panel), false);
    {
        // Altura reservada para Element Balance / Total Balance (siempre visibles, como en HSC)
        const float alto_total = ImGui::GetContentRegionAvail().y;
        const float alto_aviso = ImGui::GetFrameHeightWithSpacing() + 6.0f;
        const float alto_inferior = std::clamp((alto_total - alto_aviso) * 0.32f, 130.0f, 210.0f);
        const float alto_hoja = std::max(alto_total - alto_inferior - alto_aviso - ImGui::GetStyle().ItemSpacing.y * 2, 120.0f);

        ImGui::BeginChild("hmb_hoja_tabs", ImVec2(0.0f, alto_hoja), false);
        if (ImGui::BeginTabBar("hmb_pestanas", ImGuiTabBarFlags_FittingPolicyScroll)) {
            char tab_in[32], tab_out[32], tab_in_pct[32], tab_out_pct[32];
            std::snprintf(tab_in, sizeof(tab_in), "IN%d###tab_in_g", area_activa_ + 1);
            std::snprintf(tab_out, sizeof(tab_out), "OUT%d###tab_out_g", area_activa_ + 1);
            std::snprintf(tab_in_pct, sizeof(tab_in_pct), "In%d-%%###tab_in_pct", area_activa_ + 1);
            std::snprintf(tab_out_pct, sizeof(tab_out_pct), "Out%d-%%###tab_out_pct", area_activa_ + 1);

            if (ImGui::BeginTabItem("BALANCE###tab_bal_g")) {
                vista_ = Vista::Balance;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_balance();
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem(tab_in)) {
                vista_ = Vista::IN_Global;
                lado_salida_sel_ = false;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_global_hoja(false);
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem(tab_out)) {
                vista_ = Vista::OUT_Global;
                lado_salida_sel_ = true;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_global_hoja(true);
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem("Target###tab_target_g")) {
                vista_ = Vista::Target;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_target();
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem(tab_in_pct)) {
                vista_ = Vista::InPct;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_porcentaje(false);
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            if (ImGui::BeginTabItem(tab_out_pct)) {
                vista_ = Vista::OutPct;
                ImGui::BeginChild("hmb_tab_body", ImVec2(0, 0), false);
                dibujar_pestana_porcentaje(true);
                ImGui::EndChild();
                ImGui::EndTabItem();
            }
            ImGui::EndTabBar();
        }
        ImGui::EndChild();

        // ── Paneles inferiores: Element Balance | Total Balance ──
        const float ancho_avail = ImGui::GetContentRegionAvail().x;
        const float ancho_elem = std::clamp(ancho_avail * 0.35f, 230.0f, 350.0f);
        const float ancho_total = ancho_avail - ancho_elem - espacio;
        ImGui::BeginChild("hmb_inf_elem", ImVec2(ancho_elem, alto_inferior), true);
        dibujar_tabla_elementos();
        ImGui::EndChild();
        ImGui::SameLine();
        ImGui::BeginChild("hmb_inf_total", ImVec2(ancho_total, alto_inferior), true);
        dibujar_resumen_total();
        ImGui::EndChild();

        if (!resultado_.avisos.empty()) {
            ImGui::TextColored(color_aviso(), "Avisos (%d): %s",
                               static_cast<int>(resultado_.avisos.size()),
                               resultado_.avisos.front().c_str());
            if (ImGui::IsItemHovered() && resultado_.avisos.size() > 1) {
                ImGui::BeginTooltip();
                for (const auto& a : resultado_.avisos) ImGui::BulletText("%s", a.c_str());
                ImGui::EndTooltip();
            }
        } else {
            ImGui::TextColored(color_ok(), "Sin avisos: hoja de balance consistente.");
        }
    }
    ImGui::EndChild();

    ImGui::SameLine();

    // ───────── Panel derecho (Tools) ─────────
    ImGui::BeginChild("hmb_derecha", ImVec2(ancho_tools, alto_panel), true);
    dibujar_herramientas();
    ImGui::EndChild();

    // ── Modal buscador de especies ──
    std::vector<const nucleo::Especie*> seleccionadas;
    if (ModalBuscadorDB::instancia().dibujar(kModalHMB, seleccionadas, false)) {
        if (!seleccionadas.empty() && modal_corriente_ >= 0) {
            auto& grupos = lista(modal_es_salida_);
            if (modal_corriente_ < static_cast<int>(grupos.size())) {
                CorrienteUI& c = grupos[static_cast<size_t>(modal_corriente_)];
                const std::string formula = seleccionadas[0]->formula;
                if (modal_fila_ >= 0 && modal_fila_ < static_cast<int>(c.filas.size())) {
                    copiar_texto(c.filas[static_cast<size_t>(modal_fila_)].formula,
                                 sizeof(FilaUI::formula), formula);
                } else {
                    FilaUI f;
                    copiar_texto(f.formula, sizeof(f.formula), formula);
                    f.temper = c.filas.empty()
                                   ? ((temp_unit_ == 0) ? 25.0 : 298.15)
                                   : c.filas.back().temper;
                    c.filas.push_back(f);
                    fila_sel_ = static_cast<int>(c.filas.size()) - 1;
                }
                lado_salida_sel_ = modal_es_salida_;
                corriente_sel_ = modal_corriente_;
            }
        }
        modal_corriente_ = -1;
        modal_fila_ = -1;
        recalcular(true);
    }

    // ── Ventana flotante Diagram (como en HSC Bal) ──
    if (show_diagram_) {
        ImGui::SetNextWindowSize(ImVec2(640, 430), ImGuiCond_FirstUseEver);
        if (ImGui::Begin(tr("Diagram - HSC Heat and Material Balance"), &show_diagram_)) {
            dibujar_diagrama();
        }
        ImGui::End();
    }

    aplicar_accion_pendiente();
}

}} // namespace cma::interfaz
