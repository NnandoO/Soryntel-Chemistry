#include "motores/lca.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <map>

namespace cma { namespace motores {

namespace {

// ── Factores de los consumos manuales (base horaria) ────────────────────────
// Electricidad: mix global ~0.48 kg CO2-eq/kWh (AIE/Ember 2023).
constexpr double kGwpElec     = 0.48;    // kg CO2-eq/kWh
constexpr double kApElec      = 0.0005;  // kg SO2-eq/kWh
constexpr double kEpElec      = 0.0001;  // kg PO4-eq/kWh
constexpr double kAguaElec    = 0.0015;  // m³/kWh (refrigeración)
constexpr double kEnergiaElec = 3.6;     // MJ/kWh

// Gas natural: combustión directa + CH4 fugitivo (0.5 %).
constexpr double kGwpGas      = 2.00;    // kg CO2/Nm³
constexpr double kSlipCH4     = 0.005;   // Nm³ CH4/Nm³ gas
constexpr double kGwpCH4      = 29.8;    // kg CO2-eq/kg CH4 (AR6 fósil)
constexpr double kApGas       = 0.0017;  // kg SO2-eq/Nm³
constexpr double kEpGas       = 0.0003;  // kg PO4-eq/Nm³
constexpr double kEnergiaGas  = 38.0;    // MJ/Nm³ (PCI)

// Diésel.
constexpr double kGwpDiesel     = 2.68;  // kg CO2-eq/kg
constexpr double kApDiesel      = 0.012; // kg SO2-eq/kg
constexpr double kEpDiesel      = 0.003; // kg PO4-eq/kg
constexpr double kEnergiaDiesel = 42.6;  // MJ/kg

// Residuo declarado manualmente (vertedero inerte genérico).
constexpr double kGwpResiduo = 0.01;     // kg CO2-eq/kg

std::string minusculas(std::string s) {
    for (char& c : s) {
        if (c >= 'A' && c <= 'Z') c = static_cast<char>(c - 'A' + 'a');
    }
    return s;
}

bool empieza_con(const std::string& s, const char* prefijo) {
    return s.rfind(prefijo, 0) == 0;
}

bool contiene(const std::string& s, const char* texto) {
    return s.find(texto) != std::string::npos;
}

} // namespace

// ─────────────────────────────────────────────────────────────────────────────
const std::vector<EquivalenteLCA>& LCA::catalogo() {
    static const std::vector<EquivalenteLCA> tabla = {
        // id              nombre                          grupo                     unidad  gwp    ap     ep    agua  energia  fuente
        {"co2-fossil",     "CO2 (fossil)",                 "Emissions to air",       "kg",   1.0,   0.0,   0.0,  0.0,  0.0,     "IPCC AR6 GWP100"},
        {"ch4-fossil",     "CH4 (fossil)",                 "Emissions to air",       "kg",   29.8,  0.0,   0.0,  0.0,  0.0,     "IPCC AR6 GWP100 (CH4 fósil)"},
        {"ch4-biogenic",   "CH4 (biogenic)",               "Emissions to air",       "kg",   27.0,  0.0,   0.0,  0.0,  0.0,     "IPCC AR6 GWP100 (CH4 biogénico)"},
        {"n2o",            "N2O (nitrous oxide)",          "Emissions to air",       "kg",   273.0, 0.0,   0.027, 0.0, 0.0,    "IPCC AR6; EP CML 2016"},
        {"so2",            "SO2 (sulfur dioxide)",         "Emissions to air",       "kg",   0.0,   1.0,   0.0,  0.0,  0.0,     "CML 2016 AP"},
        {"nox",            "NOx (nitrogen oxides)",        "Emissions to air",       "kg",   0.0,   0.7,   0.13, 0.0,  0.0,     "CML 2016 AP/EP (como NO2)"},
        {"nh3",            "NH3 (ammonia)",                "Emissions to air",       "kg",   0.0,   1.88,  0.35, 0.0,  0.0,     "CML 2016 AP/EP"},
        {"hcl",            "HCl (hydrogen chloride)",      "Emissions to air",       "kg",   0.0,   0.88,  0.0,  0.0,  0.0,     "CML 2016 AP"},
        {"co",             "CO (carbon monoxide)",         "Emissions to air",       "kg",   0.0,   0.0,   0.0,  0.0,  0.0,     "Inventario; sin factor en CML baseline"},
        {"pm",             "PM (particulate matter)",      "Emissions to air",       "kg",   0.0,   0.0,   0.0,  0.0,  0.0,     "Inventario; requiere factor específico"},
        {"water-intake",   "Water intake",                 "Resources",              "kg",   0.0003,0.0,   0.0,  0.001,0.0036,  "Potabilización/bombeo ~0.3 kg CO2/m³; agua consuntiva"},
        {"wastewater",     "Wastewater discharge",         "Emissions to fresh water","kg",  0.0003,0.0,   0.0005,0.0, 0.0,     "Tratamiento genérico; EP por DQO"},
        {"slag-landfill",  "Slag landfill",                "Deposited goods",        "kg",   0.01,  0.0,   0.0,  0.0,  0.0,     "Vertedero inerte genérico"},
        {"fly-ash",        "Fly ash disposal",             "Deposited goods",        "kg",   0.02,  0.0,   0.0,  0.0,  0.0,     "Gestión de cenizas volantes"},
        {"tailings",       "Tailings deposition",          "Deposited goods",        "kg",   0.005, 0.0,   0.0,  0.0,  0.0,     "Depósito de relaves"},
        {"copper-ore",     "Copper ore resource",          "Resources",              "kg",   0.05,  0.0,   0.0,  0.0,  1.2,     "Minería de cobre (screening)"},
        {"iron-ore",       "Iron ore resource",            "Resources",              "kg",   0.02,  0.0,   0.0,  0.0,  0.5,     "Minería de hierro (screening)"},
        {"limestone",      "Limestone resource",           "Resources",              "kg",   0.01,  0.0,   0.0,  0.0,  0.1,     "Minería de caliza (screening)"},
        {"product",        "Producto / coproducto",        "Products",               "kg",   0.0,   0.0,   0.0,  0.0,  0.0,     "Salida de producto (no suma impacto)"},
    };
    return tabla;
}

const EquivalenteLCA* LCA::buscar_equivalente(const std::string& id) {
    for (const auto& e : catalogo()) {
        if (e.id == id || e.nombre == id) return &e;
    }
    return nullptr;
}

// ─────────────────────────────────────────────────────────────────────────────
std::string LCA::sugerir_equivalente(const std::string& especie,
                                     const std::string& corriente,
                                     bool entrada) {
    const std::string esp = minusculas(especie);
    const std::string cor = minusculas(corriente);
    const bool gaseoso = contiene(esp, "(g)");

    std::string base = esp;
    const auto parentesis = base.find('(');
    if (parentesis != std::string::npos) base = base.substr(0, parentesis);
    while (!base.empty() && (base.back() == ' ' || base.back() == '\t')) base.pop_back();
    while (!base.empty() && (base.front() == ' ' || base.front() == '\t')) base.erase(base.begin());

    // Gases de emisión directa.
    if (base == "co2") return "co2-fossil";
    if (base == "ch4") return "ch4-fossil";
    if (base == "n2o") return "n2o";
    if (base == "so2") return "so2";
    if (base == "no" || base == "no2" || base == "nox") return "nox";
    if (base == "nh3") return "nh3";
    if (base == "hcl") return "hcl";
    if (base == "co") return "co";

    // Agua: vapor no se contabiliza; líquida según dirección.
    if (base == "h2o") {
        if (gaseoso) return "no-mapeado";
        return entrada ? "water-intake" : "wastewater";
    }

    // Componentes del aire sin factor de impacto.
    if (base == "n2" || base == "o2" || base == "ar" || base == "he") {
        return "no-mapeado";
    }

    // Sólidos identificados por nombre de corriente.
    if (contiene(cor, "escoria") || contiene(cor, "slag")) return "slag-landfill";
    if (contiene(cor, "ceniza") || contiene(cor, "ash")) return "fly-ash";
    if (contiene(cor, "relave") || contiene(cor, "tail")) return "tailings";
    if (contiene(cor, "producto") || contiene(cor, "cátodo") ||
        contiene(cor, "catodo") || contiene(cor, "cathode")) {
        return "product";
    }
    if (contiene(cor, "agua") || contiene(cor, "water")) {
        return entrada ? "water-intake" : "wastewater";
    }

    // Menas y caliza (solo como entradas).
    if (empieza_con(base, "cu")) return entrada ? "copper-ore" : "product";
    if (empieza_con(base, "fe")) return entrada ? "iron-ore" : "no-mapeado";
    if (base == "caco3" || base == "cao" || base == "ca(oh)2") {
        return entrada ? "limestone" : "no-mapeado";
    }

    return "no-mapeado";
}

// ─────────────────────────────────────────────────────────────────────────────
ResultadoLCA LCA::evaluar(const std::vector<FilaLCA>& filas,
                          double electricidad_kWh_h,
                          double gas_natural_Nm3_h,
                          double combustible_kg_h,
                          double agua_m3_h,
                          double residuo_kg_h,
                          double unidad_funcional_kg) const {
    ResultadoLCA r;
    std::map<std::string, double> contrib_gwp;

    auto cantidad_valida = [](double v) {
        return (std::isfinite(v) && v > 0.0) ? v : 0.0;
    };
    electricidad_kWh_h = cantidad_valida(electricidad_kWh_h);
    gas_natural_Nm3_h = cantidad_valida(gas_natural_Nm3_h);
    combustible_kg_h = cantidad_valida(combustible_kg_h);
    agua_m3_h = cantidad_valida(agua_m3_h);
    residuo_kg_h = cantidad_valida(residuo_kg_h);

    auto aviso = [&r](const std::string& texto) {
        if (r.avisos.size() < 64) r.avisos.push_back(texto);
    };

    for (const auto& f : filas) {
        if (!std::isfinite(f.cantidad) || f.cantidad < 0.0) {
            aviso("Cantidad inválida en " + f.corriente + " / " + f.especie);
            continue;
        }
        ++r.filas_totales;
        if (f.entrada) {
            r.masa_entrada_kg += f.cantidad;
        } else {
            r.masa_salida_kg += f.cantidad;
        }

        const EquivalenteLCA* e = buscar_equivalente(f.equivalente);
        if (e == nullptr) {
            ++r.filas_sin_mapear;
            r.masa_sin_mapear_kg += f.cantidad;
            continue;
        }
        ++r.filas_mapeadas;

        r.gwp_kg_co2eq += f.cantidad * e->gwp;
        r.ap_kg_so2eq += f.cantidad * e->ap;
        r.ep_kg_po4eq += f.cantidad * e->ep;
        r.agua_m3 += f.cantidad * e->agua;
        r.energia_mj += f.cantidad * e->energia;
        if (e->grupo == "Deposited goods") r.residuos_kg += f.cantidad;
        if (e->gwp != 0.0) contrib_gwp[e->nombre] += f.cantidad * e->gwp;
    }

    // Consumos manuales auxiliares.
    r.gwp_kg_co2eq += electricidad_kWh_h * kGwpElec;
    r.ap_kg_so2eq += electricidad_kWh_h * kApElec;
    r.ep_kg_po4eq += electricidad_kWh_h * kEpElec;
    r.agua_m3 += electricidad_kWh_h * kAguaElec;
    r.energia_mj += electricidad_kWh_h * kEnergiaElec;

    const double gwp_gas = kGwpGas + kSlipCH4 * kGwpCH4;
    r.gwp_kg_co2eq += gas_natural_Nm3_h * gwp_gas;
    r.ap_kg_so2eq += gas_natural_Nm3_h * kApGas;
    r.ep_kg_po4eq += gas_natural_Nm3_h * kEpGas;
    r.energia_mj += gas_natural_Nm3_h * kEnergiaGas;

    r.gwp_kg_co2eq += combustible_kg_h * kGwpDiesel;
    r.ap_kg_so2eq += combustible_kg_h * kApDiesel;
    r.ep_kg_po4eq += combustible_kg_h * kEpDiesel;
    r.energia_mj += combustible_kg_h * kEnergiaDiesel;

    r.agua_m3 += agua_m3_h;
    r.residuos_kg += residuo_kg_h;
    r.gwp_kg_co2eq += residuo_kg_h * kGwpResiduo;

    // Unidad funcional: normaliza todo el dataset (flujos y consumos).
    if (std::isfinite(unidad_funcional_kg) && unidad_funcional_kg > 0.0) {
        r.factor_unidad_funcional = 1.0 / unidad_funcional_kg;
        r.gwp_kg_co2eq *= r.factor_unidad_funcional;
        r.ap_kg_so2eq *= r.factor_unidad_funcional;
        r.ep_kg_po4eq *= r.factor_unidad_funcional;
        r.agua_m3 *= r.factor_unidad_funcional;
        r.energia_mj *= r.factor_unidad_funcional;
        r.residuos_kg *= r.factor_unidad_funcional;
        r.masa_entrada_kg *= r.factor_unidad_funcional;
        r.masa_salida_kg *= r.factor_unidad_funcional;
        r.masa_sin_mapear_kg *= r.factor_unidad_funcional;
    } else {
        r.factor_unidad_funcional = 1.0;
    }

    // Balance de masa del dataset de frontera.
    const double referencia_masa =
        std::max({std::abs(r.masa_entrada_kg), std::abs(r.masa_salida_kg), 1e-12});
    r.balance_masa_ok =
        std::abs(r.masa_entrada_kg - r.masa_salida_kg) / referencia_masa <= 1e-4;

    if (!r.balance_masa_ok) {
        char buf[192];
        std::snprintf(buf, sizeof(buf),
                      "Masa no balanceada: entra %.4g kg, sale %.4g kg (Δ %.3g %%)",
                      r.masa_entrada_kg, r.masa_salida_kg,
                      (r.masa_entrada_kg - r.masa_salida_kg) / referencia_masa * 100.0);
        aviso(buf);
    }
    if (r.filas_sin_mapear > 0) {
        char buf[192];
        std::snprintf(buf, sizeof(buf),
                      "%d fila(s) sin mapear (%.4g kg/h); no se cuentan en los indicadores",
                      r.filas_sin_mapear, r.masa_sin_mapear_kg);
        aviso(buf);
    }

    for (const auto& kv : contrib_gwp) {
        r.contribuciones.emplace_back(kv.first, kv.second * r.factor_unidad_funcional);
    }
    std::sort(r.contribuciones.begin(), r.contribuciones.end(),
              [](const auto& a, const auto& b) { return a.second > b.second; });

    r.ok = true;
    return r;
}

// ─────────────────────────────────────────────────────────────────────────────
// API previa (compatibilidad)
// ─────────────────────────────────────────────────────────────────────────────
double LCA::factor_emision_electricidad(double kWh, const std::string& pais) {
    double factor = 0.48;
    if (pais == "china") factor = 0.70;
    else if (pais == "india") factor = 0.70;
    else if (pais == "usa") factor = 0.45;
    else if (pais == "mexico") factor = 0.45;
    else if (pais == "alemania") factor = 0.40;
    else if (pais == "brasil") factor = 0.15;
    else if (pais == "españa" || pais == "espana") factor = 0.19;
    else if (pais == "francia") factor = 0.06;
    return kWh * factor;
}

double LCA::factor_emision_gas_natural(double m3) {
    return m3 * kGwpGas;
}

double LCA::factor_emision_combustible(double kg, const std::string& tipo) {
    double factor = 3.0;
    if (tipo == "diésel" || tipo == "diesel" || tipo == "gasoil") factor = kGwpDiesel;
    else if (tipo == "gasolina") factor = 2.31;
    else if (tipo == "carbón" || tipo == "carbon") factor = 2.86;
    else if (tipo == "biomasa") factor = 0.0;
    return kg * factor;
}

double LCA::huella_carbono_total(const ImpactoLCA& impacto) {
    return impacto.emision_CO2 + impacto.emision_CH4 * kGwpCH4 +
           impacto.emision_N2O * 273.0;
}

ImpactoLCA LCA::evaluar_proceso(double kWh, double gas_m3, double combustible_kg,
                                double materia_kg, double /*producto_kg*/,
                                double agua_m3) {
    ImpactoLCA res;
    const bool invalido = !std::isfinite(kWh) || !std::isfinite(gas_m3) ||
                          !std::isfinite(combustible_kg) || !std::isfinite(materia_kg) ||
                          !std::isfinite(agua_m3) || kWh < 0.0 || gas_m3 < 0.0 ||
                          combustible_kg < 0.0 || materia_kg < 0.0 || agua_m3 < 0.0;
    if (invalido) {
        res.ok = false;
        res.mensaje = "Entradas inválidas: se requieren valores finitos y no negativos.";
        error_ = res.mensaje;
        return res;
    }

    res.emision_CO2 = factor_emision_electricidad(kWh) +
                      factor_emision_gas_natural(gas_m3) +
                      factor_emision_combustible(combustible_kg);
    res.emision_CH4 = gas_m3 * kSlipCH4;
    res.emision_N2O = combustible_kg * 0.0001;
    res.huella_carbono = huella_carbono_total(res);
    res.consumo_energia = kWh * kEnergiaElec + gas_m3 * kEnergiaGas +
                          combustible_kg * kEnergiaDiesel;
    res.consumo_agua = agua_m3;
    res.residuos_solidos = materia_kg * 0.1;

    res.ok = true;
    char buf[192];
    std::snprintf(buf, sizeof(buf),
                  "Huella de carbono: %.2f kg CO2-eq (GWP100 AR6)", res.huella_carbono);
    res.mensaje = buf;
    error_.clear();
    return res;
}

}} // namespace cma::motores
