#include "estimador_propiedades.hpp"
#include <cmath>
#include <algorithm>
#include <sstream>
#include <cctype>

namespace cma { namespace motores {

std::vector<EstimadorPropiedades::GrupoJoback> EstimadorPropiedades::grupos_joback() {
    return {
        {"-CH3",   -76.45, 128.30, 16.04, -0.0335, 0.000208, -0.000000174, 23.58, -5.10},
        {"-CH2-",  -20.64, 39.40,  -4.78,  0.1075, -0.000053,  0.000000036, 22.88, 11.27},
        {">CH-",    29.89, -22.31,  2.89,  0.0740, -0.000047, -0.000000007, 22.88, 12.72},
        {">C<",    82.23, -97.80, -12.89, 0.0930, -0.000085,  0.000000033, 22.88, 24.14},
        {"=CH2",   10.47, 102.27,  8.58,  0.0022,  0.000025,  0.000000002, 18.18, -4.32},
        {"=CH-",   27.14, 58.46,  5.68,  0.0150, -0.000001, -0.000000009, 24.96, 8.73},
        {"=C<",    78.94, 13.55,  1.85,  0.0500, -0.000040,  0.000000015, 24.96, 20.84},
        {"-OH",  -208.04, 122.09,  8.45,  0.0590, -0.000029,  0.000000000, 93.06, 44.44},
        {"-O-",  -132.22, 31.78,  4.31,  0.0890, -0.000053,  0.000000015, 22.42, 23.41},
        {">C=O", -133.22, 61.30,  9.94,  0.0490, -0.000028,  0.000000005, 76.75, 31.67},
        {"-CHO", -162.03, 131.06, 12.26, 0.0570, -0.000033,  0.000000000, 81.10, 33.02},
        {"-COOH",-426.72, 145.88, 18.07, 0.1170, -0.000075,  0.000000014, 169.78, 82.47},
        {"-NH2",  -22.02, 121.44, 9.91,  0.0640, -0.000033,  0.000000000, 54.34, 22.44},
        {">NH",   53.47, 36.11,  0.44,  0.0900, -0.000053,  0.000000017, 54.34, 37.64},
        {">N-",  106.72, -25.71, -8.59, 0.1100, -0.000073,  0.000000036, 54.34, 54.87},
        {"-Cl",   -71.55, 68.46, 10.01, -0.0096, 0.000047, -0.000000026, 38.13, 30.63},
        {"-F",   -215.02, 49.90, 7.13, -0.0130, 0.000100, -0.000000067, 11.91, 1.26},
        {"-Br",   -29.63, 97.67, 11.54, -0.0062, 0.000035, -0.000000019, 52.40, 45.23},
        {"-I",    30.33, 113.29, 12.85, -0.0039, 0.000025, -0.000000013, 69.20, 56.53},
    };
}

PropiedadesEstimadas EstimadorPropiedades::estimar_joback(
    const std::string& formula_smiles, const std::string& nombre)
{
    PropiedadesEstimadas res;
    auto grupos = grupos_joback();

    double nC = 0, nH = 0, nO = 0, nN = 0, nCl = 0, nF = 0, nBr = 0, nI = 0;

    // Parseo simple de fórmula molecular
    std::string f = formula_smiles;
    for (size_t i = 0; i < f.size(); ++i) {
        if (std::isalpha(f[i])) {
            std::string element;
            element += f[i];
            if (i + 1 < f.size() && std::islower(f[i+1])) {
                element += f[i+1];
                i++;
            }
            std::string num;
            while (i + 1 < f.size() && std::isdigit(f[i+1])) {
                num += f[i+1];
                i++;
            }
            int n = num.empty() ? 1 : std::stoi(num);
            if (element == "C") nC += n;
            else if (element == "H") nH += n;
            else if (element == "O") nO += n;
            else if (element == "N") nN += n;
            else if (element == "Cl") nCl += n;
            else if (element == "F") nF += n;
            else if (element == "Br") nBr += n;
            else if (element == "I") nI += n;
        }
    }

    if (nC == 0) {
        res.ok = false;
        res.mensaje = "Fórmula inválida o sin átomos de Carbono.";
        return res;
    }

    // Mapa de conteo de grupos Joback: indice de grupo -> cantidad
    std::map<int, int> grupo_counts;

    bool contains_OH = (formula_smiles.find("OH") != std::string::npos);
    bool contains_COOH = (formula_smiles.find("COOH") != std::string::npos);
    bool contains_CHO = (formula_smiles.find("CHO") != std::string::npos);
    bool contains_NH2 = (formula_smiles.find("NH2") != std::string::npos);

    // Identificación de grupos heurística para compuestos comunes
    if (nC == 6 && nH == 6 && nO == 0 && nN == 0) {
        // Benceno: 6 grupos =CH- (aromáticos)
        grupo_counts[5] = 6;
    }
    else if (contains_COOH || (nO == 2 && nH == 2 * nC)) {
        // Ácido carboxílico lineal
        grupo_counts[11] = 1; // -COOH
        if (nC > 1) {
            grupo_counts[0] = 1; // -CH3 terminal
            grupo_counts[1] = nC - 2; // -CH2- intermedios
        }
    }
    else if (contains_OH || (nO == 1 && nH == 2 * nC + 2)) {
        // Alcohol lineal
        grupo_counts[7] = 1; // -OH
        grupo_counts[0] = 1; // -CH3
        grupo_counts[1] = nC - 1; // -CH2-
    }
    else if (nO == 1 && nH == 2 * nC && contains_CHO) {
        // Aldehído lineal
        grupo_counts[10] = 1; // -CHO
        if (nC > 1) {
            grupo_counts[0] = 1; // -CH3
            grupo_counts[1] = nC - 2; // -CH2-
        }
    }
    else if (nO == 1 && nH == 2 * nC) {
        // Cetona lineal
        grupo_counts[9] = 1; // >C=O
        grupo_counts[0] = 2; // dos -CH3
        grupo_counts[1] = nC - 3; // -CH2-
    }
    else if (contains_NH2 || (nN == 1 && nH == 2 * nC + 3)) {
        // Amina primaria lineal
        grupo_counts[12] = 1; // -NH2
        grupo_counts[0] = 1; // -CH3
        grupo_counts[1] = nC - 1; // -CH2-
    }
    else if (nO == 0 && nN == 0 && nH == 2 * nC + 2) {
        // Alcano saturado
        if (nC == 1) {
            grupo_counts[0] = 1; // Metano aproximado como 1 -CH3
        } else {
            grupo_counts[0] = 2; // dos -CH3
            grupo_counts[1] = nC - 2; // -CH2-
        }
    }
    else if (nO == 0 && nN == 0 && nH == 2 * nC) {
        // Alqueno lineal
        if (nC == 2) {
            grupo_counts[4] = 2; // dos =CH2
        } else {
            grupo_counts[4] = 1; // un =CH2
            grupo_counts[5] = 1; // un =CH-
            grupo_counts[1] = nC - 2; // -CH2-
        }
    }
    else {
        // Por defecto, asumir alcano sustituido / genérico
        grupo_counts[0] = 2;
        grupo_counts[1] = std::max(0.0, nC - 2);
    }

    // Agregar halógenos si existen
    if (nCl > 0) {
        grupo_counts[15] = nCl;
        grupo_counts[1] = std::max(0.0, grupo_counts[1] - nCl);
    }
    if (nF > 0) {
        grupo_counts[16] = nF;
        grupo_counts[1] = std::max(0.0, grupo_counts[1] - nF);
    }
    if (nBr > 0) {
        grupo_counts[17] = nBr;
        grupo_counts[1] = std::max(0.0, grupo_counts[1] - nBr);
    }
    if (nI > 0) {
        grupo_counts[18] = nI;
        grupo_counts[1] = std::max(0.0, grupo_counts[1] - nI);
    }

    double sum_Hf = 0.0, sum_Sf = 0.0, sum_Cp_a = 0.0, sum_Cp_b = 0.0, sum_Cp_c = 0.0, sum_Cp_d = 0.0, sum_Tb = 0.0, sum_Tf = 0.0;
    
    for (auto& [g_idx, count] : grupo_counts) {
        if (g_idx >= 0 && g_idx < (int)grupos.size() && count > 0) {
            auto& g = grupos[g_idx];
            sum_Hf += count * g.contrib_Hf;
            sum_Sf += count * g.contrib_Sf;
            sum_Cp_a += count * g.contrib_Cp_a;
            sum_Cp_b += count * g.contrib_Cp_b;
            sum_Cp_c += count * g.contrib_Cp_c;
            sum_Cp_d += count * g.contrib_Cp_d;
            sum_Tb += count * g.contrib_Tb;
            sum_Tf += count * g.contrib_Tf;
        }
    }

    // Fórmulas termodinámicas de Joback-Reid
    res.h_formacion = 68.29 + sum_Hf;
    res.s_formacion = 53.38 + sum_Sf;
    res.T_ebullicion = 198.2 + sum_Tb;
    res.T_fusion = 122.5 + sum_Tf;
    res.cp_a = sum_Cp_a - 37.93;
    res.cp_b = sum_Cp_b + 0.210;
    res.cp_c = sum_Cp_c - 3.91e-4;
    res.cp_d = sum_Cp_d + 2.06e-7;
    res.masa_molar = nC * 12.011 + nH * 1.008 + nO * 15.999 + nN * 14.007 + nCl * 35.45 + nF * 18.998 + nBr * 79.904 + nI * 126.904;
    res.ok = true;

    // Generar mensaje detallado de grupos detectados
    std::string msg = "Joback: ";
    for (auto& [g_idx, count] : grupo_counts) {
        if (count > 0) {
            msg += std::to_string(count) + "x" + grupos[g_idx].nombre + " ";
        }
    }
    res.mensaje = msg;

    return res;
}

PropiedadesEstimadas EstimadorPropiedades::estimar_benson(
    const std::string& formula, const std::string& tipo)
{
    return estimar_joback(formula, tipo);
}

double EstimadorPropiedades::estimar_cp_gas(double M, double T, int tipo) {
    // Correlación de Rowlinson-Bondi
    double R = 8.314;
    return R * (3.5 + 2.0 * std::exp(-T / 500.0)); // aproximación
}

double EstimadorPropiedades::estimar_T_ebullicion(
    const std::map<std::string, int>& grupos)
{
    double Tb = 273.15; // punto de partida (agua)
    for (auto& [g, n] : grupos) {
        if (g == "-CH3") Tb += n * 23.58;
        else if (g == "-CH2-") Tb += n * 22.88;
        else if (g == "-OH") Tb += n * 93.06;
        else if (g == "-COOH") Tb += n * 169.78;
        else if (g == "-NH2") Tb += n * 54.34;
        else if (g == "-Cl") Tb += n * 38.13;
    }
    return Tb;
}

double EstimadorPropiedades::estimar_Hf_benson(
    const std::map<std::string, int>& grupos, double correccion)
{
    double Hf = 0;
    for (auto& [g, n] : grupos) {
        if (g == "-CH3") Hf += n * (-76.45);
        else if (g == "-CH2-") Hf += n * (-20.64);
        else if (g == "-OH") Hf += n * (-208.04);
        else if (g == "-COOH") Hf += n * (-426.72);
        else if (g == "-NH2") Hf += n * (-22.02);
        else if (g == "-Cl") Hf += n * (-71.55);
    }
    return Hf + correccion;
}

double EstimadorPropiedades::estimar_S_benson(const std::string& formula, double M) {
    // Estimación simple basada en masa molar
    return 100.0 + 50.0 * std::log(M / 16.0);
}

}} // namespace

#include "estimador_hsc_impl.cpp"
