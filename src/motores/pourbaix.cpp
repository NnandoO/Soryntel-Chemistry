// ══════════════════════════════════════════════════════════════════════════════
//  pourbaix.cpp — Implementación científica de diagramas Eh-pH (HSC EpH)
// ══════════════════════════════════════════════════════════════════════════════
#include "pourbaix.hpp"

#include "nucleo/tipos.hpp"

#include <algorithm>
#include <cctype>
#include <cmath>
#include <fstream>
#include <iomanip>
#include <map>
#include <set>
#include <sstream>

namespace cma {
namespace motores {

static constexpr double R_KJ = 8.314462618e-3; // kJ/(mol·K)
static constexpr double F_KJV = 96.48533212;   // kJ/(mol·V)
static constexpr double LN10 = 2.302585092994046;

// Entropías molares estándar S°₂₉₈ [J/(mol·K)] de elementos por átomo
static const std::map<std::string, double>
    S_ELEMENTAL_298 = {{"H", 65.342},  // ½ S°(H2,g) = 130.684 / 2
                       {"O", 102.576}, // ½ S°(O2,g) = 205.152 / 2
                       {"C", 5.740},    {"N", 95.805},  {"S", 32.054},  {"P", 41.090},
                       {"Cl", 111.540}, // ½ S°(Cl2,g) = 223.08 / 2
                       {"Fe", 27.280},  {"Cu", 33.150}, {"Zn", 41.630}, {"Al", 28.300},
                       {"Mn", 32.010},  {"Cr", 23.770}, {"Ni", 29.870}, {"Co", 30.040},
                       {"Ti", 30.720},  {"V", 28.910},  {"Mo", 28.660}, {"W", 32.640},
                       {"Pb", 64.810},  {"Sn", 51.180}, {"Ag", 42.550}, {"Au", 47.400},
                       {"Pt", 41.630},  {"Hg", 75.900}, {"Cd", 51.760}, {"As", 35.100},
                       {"Sb", 45.690},  {"Bi", 56.740}, {"U", 50.200},  {"Mg", 32.670},
                       {"Ca", 41.590},  {"Na", 51.300}, {"K", 64.680},  {"Li", 29.120},
                       {"Ba", 62.480},  {"Sr", 55.000}, {"Si", 18.810}};

struct RefPourbaixSpecies {
    const char* elem;
    const char* formula;
    const char* formula_limpia;
    const char* nombre;
    const char* estado; // "s", "l", "aq"
    double dG298;       // kJ/mol (convención NBS/HSC: ΔGf°(H+)=0, ΔGf°(e-)=0)
    double dH298;       // kJ/mol
    int carga;
    int n_M;
    int n_H;
    int n_O;
    const char* lig_elem;
    int n_L;
};

// Catálogo de respaldo NBS / Pourbaix / HSC Chemistry para garantizar que todos los
// iones hidrolizados, óxidos, hidróxidos y complejos hidrometalúrgicos clave estén presentes
static const RefPourbaixSpecies kRefCatalog[] = {
    // ── Sistema Fe - S - C - H2O ──
    {"Fe", "Fe", "Fe", "Hierro metálico (α)", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Fe", "Fe(+2a)", "Fe²⁺", "Hierro(II) acuoso", "aq", -78.87, -89.10, +2, 1, 0, 0, "", 0},
    {"Fe", "Fe(+3a)", "Fe³⁺", "Hierro(III) acuoso", "aq", -4.60, -48.50, +3, 1, 0, 0, "", 0},
    {"Fe", "FeOH(+a)", "FeOH⁺", "Hidroxi-hierro(II)", "aq", -277.40, -324.70, +1, 1, 1, 1, "", 0},
    {"Fe", "FeOH(+2a)", "FeOH²⁺", "Hidroxi-hierro(III)", "aq", -229.40, -290.80, +2, 1, 1, 1, "", 0},
    {"Fe", "HFeO2(-a)", "HFeO₂⁻", "Biferrito acuoso", "aq", -377.70, -453.50, -1, 1, 1, 2, "", 0},
    {"Fe", "FeO4(-2a)", "FeO₄²⁻", "Ferrato(VI) acuoso", "aq", -322.20, -454.00, -2, 1, 0, 4, "", 0},
    {"Fe", "Fe3O4", "Fe₃O₄", "Magnetita", "s", -1015.40, -1118.40, 0, 3, 0, 4, "", 0},
    {"Fe", "Fe2O3", "Fe₂O₃", "Hematita", "s", -742.20, -824.20, 0, 2, 0, 3, "", 0},
    {"Fe", "Fe(OH)2", "Fe(OH)₂", "Hidróxido ferroso", "s", -486.50, -569.00, 0, 1, 2, 2, "", 0},
    {"Fe", "Fe(OH)3", "Fe(OH)₃", "Hidróxido férrico", "s", -696.50, -823.00, 0, 1, 3, 3, "", 0},
    {"Fe", "FeS", "FeS", " Pirrotina / Troilita", "s", -100.40, -100.00, 0, 1, 0, 0, "S", 1},
    {"Fe", "FeS2", "FeS₂", "Pirita", "s", -166.90, -178.20, 0, 1, 0, 0, "S", 2},
    {"Fe", "FeCO3", "FeCO₃", "Siderita", "s", -666.70, -740.60, 0, 1, 0, 3, "C", 1},

    // ── Sistema Cu - S - Cl - N - C - H2O ──
    {"Cu", "Cu", "Cu", "Cobre metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Cu", "Cu(+a)", "Cu⁺", "Cobre(I) acuoso", "aq", +49.98, +71.67, +1, 1, 0, 0, "", 0},
    {"Cu", "Cu(+2a)", "Cu²⁺", "Cobre(II) acuoso", "aq", +65.49, +64.77, +2, 1, 0, 0, "", 0},
    {"Cu", "HCuO2(-a)", "HCuO₂⁻", "Bicuprito acuoso", "aq", -258.50, -343.00, -1, 1, 1, 2, "", 0},
    {"Cu", "CuO2(-2a)", "CuO₂²⁻", "Cuprito acuoso", "aq", -183.60, -280.00, -2, 1, 0, 2, "", 0},
    {"Cu", "Cu2O", "Cu₂O", "Cuprita", "s", -146.00, -168.60, 0, 2, 0, 1, "", 0},
    {"Cu", "CuO", "CuO", "Tenorita", "s", -129.70, -157.30, 0, 1, 0, 1, "", 0},
    {"Cu", "Cu(OH)2", "Cu(OH)₂", "Hidróxido cúprico", "s", -359.00, -449.80, 0, 1, 2, 2, "", 0},
    {"Cu", "Cu2S", "Cu₂S", "Calcosina", "s", -86.20, -79.50, 0, 2, 0, 0, "S", 1},
    {"Cu", "CuS", "CuS", "Covelina", "s", -53.60, -53.10, 0, 1, 0, 0, "S", 1},
    {"Cu", "CuCl2(-a)", "CuCl₂⁻", "Diclorocuprato(I)", "aq", -240.10, -260.00, -1, 1, 0, 0, "Cl", 2},
    {"Cu", "Cu(NH3)2(+a)", "Cu(NH₃)₂⁺", "Diaminocobre(I)", "aq", -64.50, -95.00, +1, 1, 6, 0, "N", 2},
    {"Cu", "Cu(NH3)4(+2a)", "Cu(NH₃)₄²⁺", "Tetraminocobre(II)", "aq", -111.10, -220.00, +2, 1, 12, 0, "N", 4},

    // ── Sistema Zn - H2O ──
    {"Zn", "Zn", "Zn", "Zinc metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Zn", "Zn(+2a)", "Zn²⁺", "Zinc(II) acuoso", "aq", -147.06, -153.89, +2, 1, 0, 0, "", 0},
    {"Zn", "ZnOH(+a)", "ZnOH⁺", "Hidroxi-zinc(II)", "aq", -330.10, -367.00, +1, 1, 1, 1, "", 0},
    {"Zn", "HZnO2(-a)", "HZnO₂⁻", "Bizincato acuoso", "aq", -457.10, -530.00, -1, 1, 1, 2, "", 0},
    {"Zn", "ZnO2(-2a)", "ZnO₂²⁻", "Zincato acuoso", "aq", -384.20, -460.00, -2, 1, 0, 2, "", 0},
    {"Zn", "ZnO", "ZnO", "Zincita", "s", -320.48, -350.46, 0, 1, 0, 1, "", 0},
    {"Zn", "Zn(OH)2", "Zn(OH)₂", "Hidróxido de zinc", "s", -553.50, -641.90, 0, 1, 2, 2, "", 0},
    {"Zn", "ZnS", "ZnS", "Esfalerita", "s", -201.29, -205.98, 0, 1, 0, 0, "S", 1},

    // ── Sistema Mn - H2O ──
    {"Mn", "Mn", "Mn", "Manganeso metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Mn", "Mn(+2a)", "Mn²⁺", "Manganeso(II) acuoso", "aq", -228.10, -220.75, +2, 1, 0, 0, "", 0},
    {"Mn", "Mn(+3a)", "Mn³⁺", "Manganeso(III) acuoso", "aq", -85.00, -100.00, +3, 1, 0, 0, "", 0},
    {"Mn", "HMnO2(-a)", "HMnO₂⁻", "Bimanganito acuoso", "aq", -505.80, -580.00, -1, 1, 1, 2, "", 0},
    {"Mn", "MnO4(-2a)", "MnO₄²⁻", "Manganato(VI) acuoso", "aq", -500.70, -653.00, -2, 1, 0, 4, "", 0},
    {"Mn", "MnO4(-a)", "MnO₄⁻", "Permanganato(VII) acuoso", "aq", -447.20, -541.40, -1, 1, 0, 4, "", 0},
    {"Mn", "Mn(OH)2", "Mn(OH)₂", "Pirocroíta", "s", -615.00, -695.40, 0, 1, 2, 2, "", 0},
    {"Mn", "Mn3O4", "Mn₃O₄", "Hausmannita", "s", -1283.20, -1387.80, 0, 3, 0, 4, "", 0},
    {"Mn", "Mn2O3", "Mn₂O₃", "Bixbyíta", "s", -881.10, -959.00, 0, 2, 0, 3, "", 0},
    {"Mn", "MnO2", "MnO₂", "Pirolusita", "s", -465.10, -520.03, 0, 1, 0, 2, "", 0},

    // ── Sistema Ni - H2O ──
    {"Ni", "Ni", "Ni", "Níquel metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Ni", "Ni(+2a)", "Ni²⁺", "Níquel(II) acuoso", "aq", -45.60, -54.00, +2, 1, 0, 0, "", 0},
    {"Ni", "HNiO2(-a)", "HNiO₂⁻", "Biniquelito acuoso", "aq", -349.20, -430.00, -1, 1, 1, 2, "", 0},
    {"Ni", "Ni(OH)2", "Ni(OH)₂", "Teofrastita", "s", -447.20, -529.70, 0, 1, 2, 2, "", 0},
    {"Ni", "Ni3O4", "Ni₃O₄", "Óxido de níquel(II,III)", "s", -711.90, -815.00, 0, 3, 0, 4, "", 0},
    {"Ni", "NiO2", "NiO₂", "Peróxido de níquel(IV)", "s", -215.10, -270.00, 0, 1, 0, 2, "", 0},

    // ── Sistema Cr - H2O ──
    {"Cr", "Cr", "Cr", "Cromo metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Cr", "Cr(+2a)", "Cr²⁺", "Cromo(II) acuoso", "aq", -176.10, -143.50, +2, 1, 0, 0, "", 0},
    {"Cr", "Cr(+3a)", "Cr³⁺", "Cromo(III) acuoso", "aq", -215.50, -256.00, +3, 1, 0, 0, "", 0},
    {"Cr", "CrOH(+2a)", "CrOH²⁺", "Hidroxi-cromo(III)", "aq", -430.90, -495.00, +2, 1, 1, 1, "", 0},
    {"Cr", "CrO2(-a)", "CrO₂⁻", "Cromito acuoso", "aq", -535.90, -610.00, -1, 1, 0, 2, "", 0},
    {"Cr", "HCrO4(-a)", "HCrO₄⁻", "Bicromato acuoso", "aq", -764.80, -878.20, -1, 1, 1, 4, "", 0},
    {"Cr", "CrO4(-2a)", "CrO₄²⁻", "Cromato(VI) acuoso", "aq", -727.80, -881.20, -2, 1, 0, 4, "", 0},
    {"Cr", "Cr2O3", "Cr₂O₃", "Eskolaíta", "s", -1058.10, -1139.70, 0, 2, 0, 3, "", 0},

    // ── Sistema Al - H2O ──
    {"Al", "Al", "Al", "Aluminio metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Al", "Al(+3a)", "Al³⁺", "Aluminio(III) acuoso", "aq", -489.40, -538.40, +3, 1, 0, 0, "", 0},
    {"Al", "AlOH(+2a)", "AlOH²⁺", "Hidroxi-aluminio(III)", "aq", -698.30, -767.00, +2, 1, 1, 1, "", 0},
    {"Al", "AlO2(-a)", "AlO₂⁻", "Aluminato acuoso", "aq", -830.90, -930.90, -1, 1, 0, 2, "", 0},
    {"Al", "Al(OH)3", "Al(OH)₃", "Gibbsita", "s", -1154.90, -1293.10, 0, 1, 3, 3, "", 0},
    {"Al", "Al2O3", "Al₂O₃", "Corindón", "s", -1582.30, -1675.70, 0, 2, 0, 3, "", 0},

    // ── Sistema Au - Cl - H2O ──
    {"Au", "Au", "Au", "Oro metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Au", "Au(+a)", "Au⁺", "Oro(I) acuoso", "aq", +163.20, +199.00, +1, 1, 0, 0, "", 0},
    {"Au", "Au(+3a)", "Au³⁺", "Oro(III) acuoso", "aq", +433.50, +409.00, +3, 1, 0, 0, "", 0},
    {"Au", "H2AuO3(-a)", "H₂AuO₃⁻", "Aurato acuoso", "aq", -218.00, -310.00, -1, 1, 2, 3, "", 0},
    {"Au", "Au(OH)3", "Au(OH)₃", "Hidróxido áurico", "s", -317.00, -424.70, 0, 1, 3, 3, "", 0},
    {"Au", "AuCl2(-a)", "AuCl₂⁻", "Dicloroaurato(I)", "aq", -151.00, -175.00, -1, 1, 0, 0, "Cl", 2},
    {"Au", "AuCl4(-a)", "AuCl₄⁻", "Tetracloroaurato(III)", "aq", -235.20, -322.20, -1, 1, 0, 0, "Cl", 4},

    // ── Sistema Ag - Cl - H2O ──
    {"Ag", "Ag", "Ag", "Plata metálica", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Ag", "Ag(+a)", "Ag⁺", "Plata(I) acuosa", "aq", +77.11, +105.58, +1, 1, 0, 0, "", 0},
    {"Ag", "AgO(-a)", "AgO⁻", "Argentato(I) acuoso", "aq", -22.90, -75.00, -1, 1, 0, 1, "", 0},
    {"Ag", "Ag2O", "Ag₂O", "Óxido de plata(I)", "s", -11.20, -31.05, 0, 2, 0, 1, "", 0},
    {"Ag", "AgO", "AgO", "Óxido de plata(I,III)", "s", +14.20, -12.10, 0, 1, 0, 1, "", 0},
    {"Ag", "AgCl", "AgCl", "Clorargirita", "s", -109.79, -127.01, 0, 1, 0, 0, "Cl", 1},

    // ── Sistema Pb - S - C - H2O ──
    {"Pb", "Pb", "Pb", "Plomo metálico", "s", 0.00, 0.00, 0, 1, 0, 0, "", 0},
    {"Pb", "Pb(+2a)", "Pb²⁺", "Plomo(II) acuoso", "aq", -24.43, -1.70, +2, 1, 0, 0, "", 0},
    {"Pb", "HPbO2(-a)", "HPbO₂⁻", "Biplumbito acuoso", "aq", -338.90, -415.00, -1, 1, 1, 2, "", 0},
    {"Pb", "PbO", "PbO", "Litargirio", "s", -188.95, -219.00, 0, 1, 0, 1, "", 0},
    {"Pb", "Pb3O4", "Pb₃O₄", "Minio", "s", -601.20, -718.40, 0, 3, 0, 4, "", 0},
    {"Pb", "PbO2", "PbO₂", "Plattnerita", "s", -217.33, -277.40, 0, 1, 0, 2, "", 0},
    {"Pb", "PbS", "PbS", "Galena", "s", -98.70, -100.40, 0, 1, 0, 0, "S", 1},
    {"Pb", "PbCO3", "PbCO₃", "Cerusita", "s", -625.50, -699.10, 0, 1, 0, 3, "C", 1}};

static domain::PourbaixCorrosionState clasificar_corrosion(const std::string& estado, int n_M, int n_H,
                                                           int n_O, const std::map<std::string, int>& ligs) {
    if (estado == "g")
        return domain::PourbaixCorrosionState::GasPhase;
    if (estado == "aq")
        return domain::PourbaixCorrosionState::Corrosion;
    bool has_lig = false;
    for (const auto& [el, cnt] : ligs) {
        if (cnt > 0) {
            has_lig = true;
            break;
        }
    }
    if (n_M > 0 && n_H == 0 && n_O == 0 && !has_lig) {
        return domain::PourbaixCorrosionState::Immunity;
    }
    return domain::PourbaixCorrosionState::Passivation;
}

static bool parsear_formula_extendida(const std::string& formula, const std::string& elem_M,
                                      const std::vector<domain::PourbaixLigandSpec>& ligandos, int& n_M,
                                      int& n_H, int& n_O, std::map<std::string, int>& n_ligands, int& carga,
                                      std::string& estado, std::string& formula_limpia) {
    n_M = n_H = n_O = carga = 0;
    n_ligands.clear();
    estado = "s";

    std::string f = formula;
    auto trim = [](std::string& s) {
        while (!s.empty() && s.back() == ' ')
            s.pop_back();
    };
    trim(f);

    if (f.size() > 3) {
        size_t pos_par = f.rfind('(');
        if (pos_par != std::string::npos && f.back() == ')') {
            std::string suf = f.substr(pos_par + 1, f.size() - pos_par - 2);
            if (suf == "s" || suf == "cr" || suf == "α" || suf == "β" || suf == "γ") {
                estado = "s";
                f = f.substr(0, pos_par);
            } else if (suf == "l") {
                estado = "l";
                f = f.substr(0, pos_par);
            } else if (suf == "g") {
                estado = "g";
                f = f.substr(0, pos_par);
            } else if (suf == "a" || suf == "aq" || suf == "ia") {
                estado = "aq";
                f = f.substr(0, pos_par);
            } else if (!suf.empty() && (suf[0] == '+' || suf[0] == '-')) {
                estado = "aq";
                bool neg = (suf[0] == '-');
                std::string num_str;
                for (size_t i = 1; i < suf.size(); ++i) {
                    if (std::isdigit(static_cast<unsigned char>(suf[i])))
                        num_str += suf[i];
                }
                int mag = num_str.empty() ? 1 : std::stoi(num_str);
                carga = neg ? -mag : mag;
                f = f.substr(0, pos_par);
            }
        }
    }

    formula_limpia = f;
    if (carga != 0) {
        formula_limpia += (carga > 0 ? "+" : "-");
        if (std::abs(carga) > 1)
            formula_limpia += std::to_string(std::abs(carga));
    }

    std::map<std::string, int> conteo;
    auto parse_seg = [&](const std::string& seg, int mult) {
        size_t i = 0;
        while (i < seg.size()) {
            if (std::isupper(static_cast<unsigned char>(seg[i]))) {
                std::string el(1, seg[i++]);
                while (i < seg.size() && std::islower(static_cast<unsigned char>(seg[i]))) {
                    el += seg[i++];
                }
                std::string num;
                while (i < seg.size() && std::isdigit(static_cast<unsigned char>(seg[i]))) {
                    num += seg[i++];
                }
                int cnt = num.empty() ? 1 : std::stoi(num);
                conteo[el] += cnt * mult;
            } else {
                ++i;
            }
        }
    };

    size_t i = 0;
    std::string base;
    while (i < f.size()) {
        if (f[i] == '(') {
            parse_seg(base, 1);
            base.clear();
            size_t close = f.find(')', i);
            if (close == std::string::npos)
                return false;
            std::string inner = f.substr(i + 1, close - i - 1);
            i = close + 1;
            std::string num;
            while (i < f.size() && std::isdigit(static_cast<unsigned char>(f[i]))) {
                num += f[i++];
            }
            int mult = num.empty() ? 1 : std::stoi(num);
            parse_seg(inner, mult);
        } else if (f[i] == '*' || f[i] == '\xC2' || f[i] == '.') {
            return false; // Excluir hidratos complejos no estandarizados
        } else {
            base += f[i++];
        }
    }
    parse_seg(base, 1);

    for (const auto& [el, cnt] : conteo) {
        if (el == elem_M) {
            n_M += cnt;
        } else if (el == "H") {
            n_H += cnt;
        } else if (el == "O") {
            n_O += cnt;
        } else {
            bool allowed_lig = false;
            for (const auto& lig : ligandos) {
                if (lig.enabled && lig.element == el) {
                    allowed_lig = true;
                    n_ligands[el] += cnt;
                    break;
                }
            }
            if (!allowed_lig)
                return false;
        }
    }
    return n_M > 0;
}

CalculadorPourbaix::CalculadorPourbaix(const domain::ThermoRepository* repo) : repo_(repo) {}

void CalculadorPourbaix::establecer_elemento(const std::string& elem) {
    if (elemento_ != elem) {
        elemento_ = elem;
        especies_cargadas_ = false;
    }
}

void CalculadorPourbaix::establecer_temperatura(double T_K) {
    T_K_ = std::clamp(T_K, 273.15, 623.15);
}

void CalculadorPourbaix::establecer_presion(double P_bar) {
    P_bar_ = std::clamp(P_bar, 0.01, 1000.0);
}

void CalculadorPourbaix::establecer_concentracion(double mol_L) {
    conc_ = std::clamp(mol_L, 1e-12, 10.0);
    for (auto& sp : especies_) {
        if (sp.estado == "aq")
            sp.molalidad = conc_;
    }
}

void CalculadorPourbaix::establecer_rango_pH(double pH_min, double pH_max) {
    pH_min_ = pH_min;
    pH_max_ = std::max(pH_min + 1.0, pH_max);
}

void CalculadorPourbaix::establecer_rango_Eh(double Eh_min, double Eh_max) {
    Eh_min_ = Eh_min;
    Eh_max_ = std::max(Eh_min + 0.2, Eh_max);
}

void CalculadorPourbaix::establecer_resolucion(int n_pH, int n_Eh) {
    n_pH_ = std::clamp(n_pH, 20, 250);
    n_Eh_ = std::clamp(n_Eh, 20, 250);
}

void CalculadorPourbaix::establecer_escala_potencial(domain::PourbaixPotentialScale escala) {
    escala_ = escala;
}

void CalculadorPourbaix::limpiar_ligandos() {
    ligandos_.clear();
    especies_cargadas_ = false;
}

void CalculadorPourbaix::agregar_o_actualizar_ligando(const std::string& elem_ligando, double molalidad,
                                                      bool activo) {
    for (auto& l : ligandos_) {
        if (l.element == elem_ligando) {
            l.molality = std::clamp(molalidad, 1e-12, 10.0);
            l.enabled = activo;
            especies_cargadas_ = false;
            return;
        }
    }
    ligandos_.push_back({elem_ligando, std::clamp(molalidad, 1e-12, 10.0), activo});
    especies_cargadas_ = false;
}

void CalculadorPourbaix::establecer_especie_activa(size_t idx, bool activa) {
    if (idx < especies_.size())
        especies_[idx].activa = activa;
}

void CalculadorPourbaix::establecer_molalidad_especie(size_t idx, double molalidad) {
    if (idx < especies_.size()) {
        especies_[idx].molalidad = std::clamp(molalidad, 1e-12, 10.0);
    }
}

std::string CalculadorPourbaix::calcular_hash_sistema() const {
    std::ostringstream os;
    os << elemento_ << "|";
    for (const auto& l : ligandos_) {
        if (l.enabled)
            os << l.element << ":" << l.molality << ",";
    }
    return os.str();
}

double CalculadorPourbaix::dG0_agua_liquida(double T_K) const {
    // ΔGf°(H2O, l) = ΔHf°298 - T*ΔSf°298 + ΔCp*(T - 298.15 - T*ln(T/298.15))
    // A 298.15 K: ΔHf° = -285.83 kJ/mol, ΔSf° = 69.95 - (130.684 + 102.576) = -163.31 J/(mol·K)
    // -> ΔGf°(298.15 K) = -237.14 kJ/mol
    const double dH298 = -285.830;
    const double dS298 = -0.16331; // kJ/(mol·K)
    const double dCp = 0.0320;     // kJ/(mol·K)
    return dH298 - T_K * dS298 + dCp * ((T_K - 298.15) - T_K * std::log(T_K / 298.15));
}

void CalculadorPourbaix::cargar_especies_disponibles() {
    especies_.clear();
    especies_ligandos_.clear();
    std::set<std::string> vistos;

    auto eval_dG_T = [&](double dG298, double dH298) -> double {
        double dS298 = (dH298 - dG298) / 298.15;
        return dH298 - T_K_ * dS298;
    };

    // 1. Cargar primero el catálogo de referencia NBS/HSC validado para el sistema
    for (const auto& ref : kRefCatalog) {
        if (elemento_ != ref.elem)
            continue;
        std::map<std::string, int> lig_map;
        if (ref.n_L > 0 && ref.lig_elem[0] != '\0') {
            bool lig_ok = false;
            for (const auto& l : ligandos_) {
                if (l.enabled && l.element == ref.lig_elem) {
                    lig_ok = true;
                    lig_map[ref.lig_elem] = ref.n_L;
                    break;
                }
            }
            if (!lig_ok)
                continue;
        }

        EspeciePourbaix ep;
        ep.id = static_cast<int64_t>(especies_.size() + 1);
        ep.formula = ref.formula;
        ep.formula_limpia = ref.formula_limpia;
        ep.nombre = ref.nombre;
        ep.estado = ref.estado;
        ep.dG0 = eval_dG_T(ref.dG298, ref.dH298);
        ep.carga = ref.carga;
        ep.n_M = ref.n_M;
        ep.n_H = ref.n_H;
        ep.n_O = ref.n_O;
        ep.molalidad = (ep.estado == "aq") ? conc_ : 1.0;
        ep.activa = true;
        ep.estado_corrosion = clasificar_corrosion(ep.estado, ep.n_M, ep.n_H, ep.n_O, lig_map);

        std::string clave = ep.formula + "_" + ep.estado;
        vistos.insert(clave);
        vistos.insert(ep.formula_limpia + "_" + ep.estado);
        especies_.push_back(ep);
        especies_ligandos_.push_back(lig_map);
    }

    // 2. Complementar con especies desde el repositorio termodinámico inyectado
    if (repo_) {
        const auto& todas = repo_->todas_especies();
        for (const auto* sp : todas) {
            if (!sp || sp->id > 445 || std::abs(sp->cp_c) > 1.0)
                continue;
            int n_M = 0, n_H = 0, n_O = 0, carga = 0;
            std::map<std::string, int> n_ligs;
            std::string estado, f_limpia;
            if (!parsear_formula_extendida(sp->formula, elemento_, ligandos_, n_M, n_H, n_O, n_ligs, carga,
                                           estado, f_limpia)) {
                continue;
            }
            if (sp->fase_std == nucleo::Fase::Gas || estado == "g")
                continue;
            if (sp->fase_std == nucleo::Fase::Liquido || estado == "l") {
                if (elemento_ != "Hg" || n_O > 0 || n_H > 0)
                    continue;
            }
            // Filtrar duplicados combinatorios neutros "(a)" de óxidos/sales en datos_especies.hpp
            if (estado == "aq" && carga == 0)
                continue;

            std::string clave = f_limpia + "_" + estado;
            if (vistos.count(clave) || vistos.count(sp->formula + "_" + estado))
                continue;
            vistos.insert(clave);

            // Cálculo de ΔGf°(T) con corrección electroquímica +z·½S°(H2,g)
            auto it_M = S_ELEMENTAL_298.find(elemento_);
            double s_M = (it_M != S_ELEMENTAL_298.end()) ? it_M->second : 35.0;
            double s_elem = n_M * s_M + n_H * S_ELEMENTAL_298.at("H") + n_O * S_ELEMENTAL_298.at("O") -
                            carga * S_ELEMENTAL_298.at("H");
            for (const auto& [el_l, cnt_l] : n_ligs) {
                auto it_L = S_ELEMENTAL_298.find(el_l);
                s_elem += cnt_l * ((it_L != S_ELEMENTAL_298.end()) ? it_L->second : 35.0);
            }
            double dH_T = sp->entalpia_abs(T_K_);
            double s_sp_T = sp->entropia_abs(T_K_);
            double dG_T = dH_T - T_K_ * (s_sp_T - s_elem) / 1000.0;
            if (n_M == 1 && n_H == 0 && n_O == 0 && n_ligs.empty() && carga == 0 && estado == "s") {
                dG_T = 0.0;
            }

            EspeciePourbaix ep;
            ep.id = sp->id;
            ep.formula = sp->formula;
            ep.formula_limpia = f_limpia;
            ep.nombre = sp->nombre;
            ep.estado = estado;
            ep.dG0 = dG_T;
            ep.carga = carga;
            ep.n_M = n_M;
            ep.n_H = n_H;
            ep.n_O = n_O;
            ep.molalidad = (estado == "aq") ? conc_ : 1.0;
            ep.activa = true;
            ep.estado_corrosion = clasificar_corrosion(estado, n_M, n_H, n_O, n_ligs);

            especies_.push_back(ep);
            especies_ligandos_.push_back(n_ligs);
        }
    }

    especies_cargadas_ = true;
    ultimo_hash_sistema_ = calcular_hash_sistema();
}

double CalculadorPourbaix::potencial_ligando(const std::string& elem_L, double molalidad, double pH,
                                             double Eh_SHE) const {
    struct Carrier {
        int n_L, n_H, n_O, z;
        double dG298, dH298;
    };
    static const std::map<std::string, std::vector<Carrier>> kCarriers = {
        {"S",
         {
             {1, 2, 0, 0, -27.83, -39.70},    // H2S(aq)
             {1, 1, 0, -1, +12.08, -17.60},   // HS-(aq)
             {1, 0, 0, -2, +85.80, +33.10},   // S2-(aq)
             {1, 1, 4, -1, -755.91, -887.34}, // HSO4-(aq)
             {1, 0, 4, -2, -744.53, -909.27}  // SO4^2-(aq)
         }},
        {"Cl",
         {
             {1, 0, 0, -1, -131.23, -167.16}, // Cl-(aq)
             {1, 1, 1, 0, -79.90, -120.90},   // HClO(aq)
             {1, 0, 1, -1, -36.80, -107.10}   // ClO-(aq)
         }},
        {"C",
         {
             {1, 2, 3, 0, -623.11, -699.65},  // H2CO3(aq)
             {1, 1, 3, -1, -586.77, -691.99}, // HCO3-(aq)
             {1, 0, 3, -2, -527.81, -677.14}  // CO3^2-(aq)
         }},
        {"N",
         {
             {1, 4, 0, +1, -79.31, -132.51}, // NH4+(aq)
             {1, 3, 0, 0, -26.50, -80.29}    // NH3(aq)
         }}};

    auto it = kCarriers.find(elem_L);
    if (it == kCarriers.end())
        return 0.0;

    const double dG_H2O = dG0_agua_liquida(T_K_);
    double best_mu = 1e30;
    for (const auto& c : it->second) {
        double dS = (c.dH298 - c.dG298) / 298.15;
        double dG_T = c.dH298 - T_K_ * dS;
        double nu_H2O = c.n_O;
        double nu_H = 2.0 * c.n_O - c.n_H;
        double nu_e = nu_H + c.z;
        double phi = (dG_T + R_KJ * T_K_ * std::log(std::max(1e-12, molalidad)) - nu_H2O * dG_H2O -
                      nu_H * R_KJ * T_K_ * LN10 * pH - nu_e * F_KJV * Eh_SHE) /
                     c.n_L;
        if (phi < best_mu)
            best_mu = phi;
    }
    return best_mu;
}

double CalculadorPourbaix::potencial_transformado_especie(size_t idx, double pH, double Eh_SHE,
                                                          double dG_H2O) const {
    const auto& sp = especies_[idx];
    const auto& n_ligs = especies_ligandos_[idx];

    double mu_sp = sp.dG0;
    if (sp.estado == "aq") {
        mu_sp += R_KJ * T_K_ * std::log(std::max(1e-12, sp.molalidad));
    }

    double mu_lig_total = 0.0;
    for (const auto& [el_L, cnt_L] : n_ligs) {
        if (cnt_L <= 0)
            continue;
        double m_L = 1e-3;
        for (const auto& l : ligandos_) {
            if (l.element == el_L) {
                m_L = l.molality;
                break;
            }
        }
        mu_lig_total += cnt_L * potencial_ligando(el_L, m_L, pH, Eh_SHE);
    }

    const double nu_H2O = static_cast<double>(sp.n_O);
    const double nu_H = 2.0 * sp.n_O - sp.n_H;
    const double nu_e = nu_H + sp.carga;

    return (mu_sp - mu_lig_total - nu_H2O * dG_H2O - nu_H * R_KJ * T_K_ * LN10 * pH - nu_e * F_KJV * Eh_SHE) /
           static_cast<double>(sp.n_M);
}

void CalculadorPourbaix::calcular() {
    if (!especies_cargadas_ || calcular_hash_sistema() != ultimo_hash_sistema_) {
        cargar_especies_disponibles();
    } else {
        // Actualizar ΔGf°(T) si cambió la temperatura manteniendo activaciones/molalidades
        for (auto& sp : especies_) {
            for (const auto& ref : kRefCatalog) {
                if (elemento_ == ref.elem && sp.formula == ref.formula && sp.estado == ref.estado) {
                    double dS298 = (ref.dH298 - ref.dG298) / 298.15;
                    sp.dG0 = ref.dH298 - T_K_ * dS298;
                    break;
                }
            }
        }
    }

    lineas_.clear();
    poligonos_.clear();
    puntos_triples_.clear();
    segmentos_frontera_.clear();

    const double dG_H2O = dG0_agua_liquida(T_K_);
    construir_lineas_y_recortes(dG_H2O);
    construir_malla_y_poligonos(dG_H2O);
}

void CalculadorPourbaix::construir_lineas_y_recortes(double dG_H2O) {
    const size_t N = especies_.size();
    const double offset_V = offset_escala_V();

    auto es_par_dominante = [&](size_t i, size_t j, double pH, double Eh_SHE) -> bool {
        const double phi_i = potencial_transformado_especie(i, pH, Eh_SHE, dG_H2O);
        const double phi_j = potencial_transformado_especie(j, pH, Eh_SHE, dG_H2O);
        const double phi_pair = std::max(phi_i, phi_j);
        for (size_t k = 0; k < N; ++k) {
            if (k == i || k == j || !especies_[k].activa)
                continue;
            const double phi_k = potencial_transformado_especie(k, pH, Eh_SHE, dG_H2O);
            if (phi_k + 1.5e-2 < phi_pair)
                return false;
        }
        return true;
    };

    for (size_t i = 0; i < N; ++i) {
        if (!especies_[i].activa)
            continue;
        for (size_t j = i + 1; j < N; ++j) {
            if (!especies_[j].activa)
                continue;
            const auto& A = especies_[i];
            const auto& B = especies_[j];

            const double c_A = 1.0 / A.n_M;
            const double c_B = 1.0 / B.n_M;
            const double n_H2O = c_B * B.n_O - c_A * A.n_O;
            const double n_H = c_A * A.n_H - c_B * B.n_H - 2.0 * n_H2O;
            const double n_e = c_A * A.carga - c_B * B.carga + n_H;

            double dG_rxn = c_B * B.dG0 - c_A * A.dG0 + n_H2O * dG_H2O;
            double ln_Q_conc = 0.0;
            if (B.estado == "aq")
                ln_Q_conc += c_B * std::log(std::max(1e-12, B.molalidad));
            if (A.estado == "aq")
                ln_Q_conc -= c_A * std::log(std::max(1e-12, A.molalidad));

            LineaPourbaix lp;
            lp.sp_ox = A.formula_limpia;
            lp.sp_red = B.formula_limpia;
            lp.n_e = static_cast<int>(std::round(std::abs(n_e)));
            lp.n_H = static_cast<int>(std::round(n_H));
            lp.pH_min = pH_min_;
            lp.pH_max = pH_max_;

            if (std::abs(n_e) < 1e-4) {
                if (std::abs(n_H) < 1e-4)
                    continue;
                lp.es_vertical = true;
                double logK = -dG_rxn / (R_KJ * T_K_ * LN10);
                double corr = ln_Q_conc / LN10;
                lp.pH_eq = (logK - corr) / (-n_H);
                if (lp.pH_eq < pH_min_ - 1.0 || lp.pH_eq > pH_max_ + 1.0)
                    continue;

                // Recortar segmento vertical donde A y B son las fases predominantes
                const int M_steps = 160;
                bool in_seg = false;
                double eh_start = 0.0, eh_last = 0.0;
                for (int s = 0; s <= M_steps; ++s) {
                    double Eh_disp = Eh_min_ + (Eh_max_ - Eh_min_) * s / M_steps;
                    double Eh_SHE = Eh_disp + offset_V;
                    bool dom = es_par_dominante(i, j, lp.pH_eq, Eh_SHE);
                    if (dom && !in_seg) {
                        in_seg = true;
                        eh_start = Eh_disp;
                        eh_last = Eh_disp;
                    } else if (dom && in_seg) {
                        eh_last = Eh_disp;
                    } else if (!dom && in_seg) {
                        in_seg = false;
                        if (std::abs(eh_last - eh_start) > 1e-3) {
                            lp.segmentos_recortados.push_back({lp.pH_eq, eh_start, lp.pH_eq, eh_last});
                        }
                    }
                }
                if (in_seg && std::abs(eh_last - eh_start) > 1e-3) {
                    lp.segmentos_recortados.push_back({lp.pH_eq, eh_start, lp.pH_eq, eh_last});
                }
            } else {
                double E0_std_SHE = -dG_rxn / (n_e * F_KJV);
                double corr_conc = -(R_KJ * T_K_ / (n_e * F_KJV)) * ln_Q_conc;
                lp.E0 = E0_std_SHE + corr_conc - offset_V;
                lp.dEdpH = -(n_H / n_e) * (R_KJ * T_K_ * LN10 / F_KJV);
                lp.es_horizontal = (std::abs(n_H) < 1e-4);

                // Recortar segmento Nernst inclinado/horizontal donde A y B codominan
                const int M_steps = 160;
                bool in_seg = false;
                double pH_start = 0.0, Eh_start = 0.0;
                double pH_last = 0.0, Eh_last = 0.0;
                for (int s = 0; s <= M_steps; ++s) {
                    double pH = pH_min_ + (pH_max_ - pH_min_) * s / M_steps;
                    double Eh_disp = lp.E0 + lp.dEdpH * pH;
                    double Eh_SHE = Eh_disp + offset_V;
                    bool inside_box = (Eh_disp >= Eh_min_ - 0.1 && Eh_disp <= Eh_max_ + 0.1);
                    bool dom = inside_box && es_par_dominante(i, j, pH, Eh_SHE);
                    if (dom && !in_seg) {
                        in_seg = true;
                        pH_start = pH;
                        Eh_start = Eh_disp;
                        pH_last = pH;
                        Eh_last = Eh_disp;
                    } else if (dom && in_seg) {
                        pH_last = pH;
                        Eh_last = Eh_disp;
                    } else if (!dom && in_seg) {
                        in_seg = false;
                        if (std::hypot(pH_last - pH_start, Eh_last - Eh_start) > 1e-3) {
                            lp.segmentos_recortados.push_back({pH_start, Eh_start, pH_last, Eh_last});
                        }
                    }
                }
                if (in_seg && std::hypot(pH_last - pH_start, Eh_last - Eh_start) > 1e-3) {
                    lp.segmentos_recortados.push_back({pH_start, Eh_start, pH_last, Eh_last});
                }
            }

            if (lp.segmentos_recortados.empty())
                continue;

            std::ostringstream eq;
            eq << A.formula_limpia;
            if (n_H > 1e-3)
                eq << " + " << std::abs(lp.n_H) << " H+";
            if (n_e > 1e-3)
                eq << " + " << lp.n_e << " e-";
            eq << " = " << B.formula_limpia;
            if (n_H < -1e-3)
                eq << " + " << std::abs(lp.n_H) << " H+";
            if (n_e < -1e-3)
                eq << " + " << lp.n_e << " e-";
            if (std::abs(n_H2O) > 1e-3) {
                eq << (n_H2O > 0 ? " + " : " - ") << static_cast<int>(std::round(std::abs(n_H2O))) << " H2O";
            }
            lp.ecuacion = eq.str();
            lineas_.push_back(std::move(lp));
        }
    }
}

void CalculadorPourbaix::construir_malla_y_poligonos(double dG_H2O) {
    const int n_pH = n_pH_;
    const int n_Eh = n_Eh_;
    const double offset_V = offset_escala_V();

    dom_.pH_grid.resize(static_cast<size_t>(n_pH));
    dom_.Eh_grid.resize(static_cast<size_t>(n_Eh));
    for (int j = 0; j < n_pH; ++j) {
        dom_.pH_grid[static_cast<size_t>(j)] = pH_min_ + (pH_max_ - pH_min_) * j / (n_pH - 1);
    }
    for (int i = 0; i < n_Eh; ++i) {
        dom_.Eh_grid[static_cast<size_t>(i)] = Eh_min_ + (Eh_max_ - Eh_min_) * i / (n_Eh - 1);
    }

    dom_.especie_dom.assign(static_cast<size_t>(n_Eh), std::vector<int>(static_cast<size_t>(n_pH), -1));
    dom_.corrosion_grid.assign(static_cast<size_t>(n_Eh), std::vector<int>(static_cast<size_t>(n_pH), 2));

    struct Accum {
        double sum_pH = 0.0;
        double sum_Eh = 0.0;
        int count = 0;
        std::vector<std::pair<double, double>> boundary_pts;
    };
    std::map<int, Accum> accum_map;

    for (int ie = 0; ie < n_Eh; ++ie) {
        const double Eh_disp = dom_.Eh_grid[static_cast<size_t>(ie)];
        const double Eh_SHE = Eh_disp + offset_V;
        for (int ip = 0; ip < n_pH; ++ip) {
            const double pH = dom_.pH_grid[static_cast<size_t>(ip)];
            double min_phi = 1e30;
            int best_idx = -1;
            for (size_t k = 0; k < especies_.size(); ++k) {
                if (!especies_[k].activa)
                    continue;
                const double phi = potencial_transformado_especie(k, pH, Eh_SHE, dG_H2O);
                if (phi < min_phi) {
                    min_phi = phi;
                    best_idx = static_cast<int>(k);
                }
            }
            dom_.especie_dom[static_cast<size_t>(ie)][static_cast<size_t>(ip)] = best_idx;
            if (best_idx >= 0) {
                const auto& sp = especies_[static_cast<size_t>(best_idx)];
                dom_.corrosion_grid[static_cast<size_t>(ie)][static_cast<size_t>(ip)] =
                    static_cast<int>(sp.estado_corrosion);
                auto& acc = accum_map[best_idx];
                acc.sum_pH += pH;
                acc.sum_Eh += Eh_disp;
                acc.count += 1;
            }
        }
    }

    // Extraer segmentos de frontera de celda y puntos triples invariantes
    const double dpH = (pH_max_ - pH_min_) / (n_pH - 1);
    const double dEh = (Eh_max_ - Eh_min_) / (n_Eh - 1);
    std::set<std::string> seen_triples;

    for (size_t ie = 0; ie + 1 < static_cast<size_t>(n_Eh); ++ie) {
        const double eh0 = dom_.Eh_grid[ie];
        const double eh1 = dom_.Eh_grid[ie + 1];
        const double ehm = 0.5 * (eh0 + eh1);

        for (size_t ip = 0; ip + 1 < static_cast<size_t>(n_pH); ++ip) {
            const double ph0 = dom_.pH_grid[ip];
            const double ph1 = dom_.pH_grid[ip + 1];
            const double phm = 0.5 * (ph0 + ph1);

            int c00 = dom_.especie_dom[ie][ip];
            int c10 = dom_.especie_dom[ie][ip + 1];
            int c01 = dom_.especie_dom[ie + 1][ip];
            int c11 = dom_.especie_dom[ie + 1][ip + 1];

            if (c00 != c10) {
                segmentos_frontera_.push_back({phm, eh0, phm, ehm});
                if (c00 >= 0)
                    accum_map[c00].boundary_pts.push_back({phm, eh0});
                if (c10 >= 0)
                    accum_map[c10].boundary_pts.push_back({phm, eh0});
            }
            if (c00 != c01) {
                segmentos_frontera_.push_back({ph0, ehm, phm, ehm});
                if (c00 >= 0)
                    accum_map[c00].boundary_pts.push_back({ph0, ehm});
                if (c01 >= 0)
                    accum_map[c01].boundary_pts.push_back({ph0, ehm});
            }

            std::set<int> uniq = {c00, c10, c01, c11};
            uniq.erase(-1);
            if (uniq.size() >= 3) {
                auto it = uniq.begin();
                int s1 = *it++;
                int s2 = *it++;
                int s3 = *it++;
                std::string key = std::to_string(s1) + "_" + std::to_string(s2) + "_" + std::to_string(s3);
                if (!seen_triples.count(key)) {
                    seen_triples.insert(key);
                    puntos_triples_.push_back({phm, ehm, especies_[static_cast<size_t>(s1)].formula_limpia,
                                               especies_[static_cast<size_t>(s2)].formula_limpia,
                                               especies_[static_cast<size_t>(s3)].formula_limpia});
                }
            }
        }
    }

    const double cell_area = dpH * dEh;
    for (auto& [idx, acc] : accum_map) {
        if (acc.count <= 0)
            continue;
        const auto& sp = especies_[static_cast<size_t>(idx)];
        domain::PourbaixPolygon poly;
        poly.species_index = idx;
        poly.formula = sp.formula;
        poly.clean_formula = sp.formula_limpia;
        poly.estado = sp.estado;
        poly.corrosion_state = sp.estado_corrosion;
        poly.centroid_pH = acc.sum_pH / acc.count;
        poly.centroid_Eh = acc.sum_Eh / acc.count;
        poly.area = acc.count * cell_area;
        poly.vertices = std::move(acc.boundary_pts);
        poligonos_.push_back(std::move(poly));
    }
}

double CalculadorPourbaix::offset_escala_V() const {
    return domain::pourbaix_scale_offset_V(escala_);
}

double CalculadorPourbaix::eh_agua_oxidacion(double pH, double pO2) const {
    // O2(g) + 4H+ + 4e- = 2H2O(l)
    const double dG_H2O = dG0_agua_liquida(T_K_);
    const double E0_O2 = -2.0 * dG_H2O / (4.0 * F_KJV); // +1.229 V a 298.15 K
    const double slope = -(R_KJ * T_K_ * LN10) / F_KJV;
    const double corr_pO2 = (R_KJ * T_K_ / (4.0 * F_KJV)) * std::log(std::max(1e-10, pO2));
    return (E0_O2 + corr_pO2 + slope * pH) - offset_escala_V();
}

double CalculadorPourbaix::eh_agua_reduccion(double pH, double pH2) const {
    // 2H+ + 2e- = H2(g), E°(SHE) = 0.000 V
    const double slope = -(R_KJ * T_K_ * LN10) / F_KJV;
    const double corr_pH2 = -(R_KJ * T_K_ / (2.0 * F_KJV)) * std::log(std::max(1e-10, pH2));
    return (corr_pH2 + slope * pH) - offset_escala_V();
}

double CalculadorPourbaix::pendiente_nernst() const {
    return -(R_KJ * T_K_ * LN10) / F_KJV;
}

domain::PourbaixPointEvaluation CalculadorPourbaix::evaluar_punto(double pH, double Eh_display_V) const {
    domain::PourbaixPointEvaluation out;
    out.pH = pH;
    out.Eh_display_V = Eh_display_V;
    out.Eh_SHE_V = Eh_display_V + offset_escala_V();
    const double nernst = (R_KJ * T_K_ * LN10) / F_KJV;
    out.pe = (nernst > 1e-9) ? (out.Eh_SHE_V / nernst) : 0.0;

    out.eh_water_upper_V = eh_agua_oxidacion(pH, 1.0);
    out.eh_water_lower_V = eh_agua_reduccion(pH, 1.0);
    out.o2_evolution = (Eh_display_V > out.eh_water_upper_V);
    out.h2_evolution = (Eh_display_V < out.eh_water_lower_V);
    out.water_stable = (!out.o2_evolution && !out.h2_evolution);

    const double dG_H2O = dG0_agua_liquida(T_K_);
    double best_phi = 1e30, second_phi = 1e30;
    int best_idx = -1, second_idx = -1;

    for (size_t k = 0; k < especies_.size(); ++k) {
        if (!especies_[k].activa)
            continue;
        double phi = potencial_transformado_especie(k, pH, out.Eh_SHE_V, dG_H2O);
        if (phi < best_phi) {
            second_phi = best_phi;
            second_idx = best_idx;
            best_phi = phi;
            best_idx = static_cast<int>(k);
        } else if (phi < second_phi) {
            second_phi = phi;
            second_idx = static_cast<int>(k);
        }
    }

    if (best_idx >= 0) {
        const auto& sp = especies_[static_cast<size_t>(best_idx)];
        out.valid = true;
        out.predominant_idx = best_idx;
        out.predominant_formula = sp.formula;
        out.predominant_clean = sp.formula_limpia;
        out.predominant_state = sp.estado;
        out.corrosion_state = sp.estado_corrosion;
        out.stability_margin_kJ = (second_idx >= 0) ? (second_phi - best_phi) : 0.0;
        if (second_idx >= 0) {
            out.runner_up_formula = especies_[static_cast<size_t>(second_idx)].formula_limpia;
        }
    }
    return out;
}

void CalculadorPourbaix::cargar_preset_hsc(int preset_id) {
    limpiar_ligandos();
    establecer_escala_potencial(domain::PourbaixPotentialScale::SHE);
    establecer_rango_pH(-1.0, 15.0);
    establecer_rango_Eh(-1.6, 2.0);
    switch (preset_id) {
    case 0: // Fe - H2O (Corrosión clásica de hierro, 25 °C)
        establecer_elemento("Fe");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-6);
        break;
    case 1: // Cu - H2O (Hidrometalurgia del cobre, 25 °C)
        establecer_elemento("Cu");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-4);
        break;
    case 2: // Cu - S - H2O (Lixiviación de sulfuros Cu2S / CuS, 25 °C)
        establecer_elemento("Cu");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-4);
        agregar_o_actualizar_ligando("S", 1e-2, true);
        break;
    case 3: // Fe - S - C - H2O ( Aguas ácidas de mina / Pirita / Siderita)
        establecer_elemento("Fe");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-4);
        agregar_o_actualizar_ligando("S", 1e-2, true);
        agregar_o_actualizar_ligando("C", 1e-3, true);
        break;
    case 4: // Au - Cl - H2O (Cloruración aurífera, 25 °C)
        establecer_elemento("Au");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-5);
        agregar_o_actualizar_ligando("Cl", 1.0, true);
        establecer_rango_Eh(-0.6, 1.8);
        break;
    case 5: // Mn - H2O (Pirolusita / Permanganato, 25 °C)
        establecer_elemento("Mn");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-4);
        break;
    default:
        establecer_elemento("Zn");
        establecer_temperatura(298.15);
        establecer_concentracion(1e-4);
        break;
    }
    cargar_especies_disponibles();
    calcular();
}

std::string CalculadorPourbaix::exportar_csv() const {
    std::ostringstream os;
    os << std::fixed << std::setprecision(4);
    os << "# HSC Eh-pH (Pourbaix) Diagram Export\n";
    os << "# Element," << elemento_ << ",T_K," << T_K_ << ",Molality_M," << std::scientific << conc_
       << std::fixed << ",Scale," << domain::pourbaix_scale_label(escala_) << "\n\n";

    os << "[SPECIES]\n";
    os << "Formula,CleanFormula,Phase,Charge,dG0_kJ_mol,Molality,Active,CorrosionZone\n";
    for (const auto& sp : especies_) {
        os << "\"" << sp.formula << "\",\"" << sp.formula_limpia << "\"," << sp.estado << "," << sp.carga
           << "," << sp.dG0 << "," << std::scientific << sp.molalidad << std::fixed << ","
           << (sp.activa ? 1 : 0) << ",\"" << domain::pourbaix_corrosion_label(sp.estado_corrosion) << "\"\n";
    }

    os << "\n[NERNST_BOUNDARIES]\n";
    os << "Sp_Ox,Sp_Red,E0_V,dEh_dpH,pH_eq,IsVertical,Equation\n";
    for (const auto& l : lineas_) {
        os << "\"" << l.sp_ox << "\",\"" << l.sp_red << "\"," << l.E0 << "," << l.dEdpH << "," << l.pH_eq
           << "," << (l.es_vertical ? 1 : 0) << ",\"" << l.ecuacion << "\"\n";
    }

    os << "\n[TRIPLE_POINTS]\n";
    os << "pH,Eh_V,Species_1,Species_2,Species_3\n";
    for (const auto& tp : puntos_triples_) {
        os << tp.pH << "," << tp.Eh_V << ",\"" << tp.sp1 << "\",\"" << tp.sp2 << "\",\"" << tp.sp3 << "\"\n";
    }
    return os.str();
}

bool CalculadorPourbaix::guardar_csv(const std::string& filepath) const {
    std::ofstream f(filepath);
    if (!f.is_open())
        return false;
    f << exportar_csv();
    return f.good();
}

} // namespace motores
} // namespace cma
