#pragma once

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace cma {
namespace motores {

enum class ModoBalanceMetalurgico {
    Solidos = 0,
    SoloEnsayos = 1,
    SolidosAgua = 2
};

struct CorrienteBalanceMetalurgico {
    int64_t id = -1;
    std::string nombre;
    double solido_t_h = 0.0;
    double liquido_t_h = 0.0;
    double sd_solido_t_h = 0.0;
    double sd_liquido_t_h = 0.0;
    bool seleccionada = true;
    bool medida = true;
    bool fija = false;
    std::map<std::string, double> ensayos; // elemento -> fracción másica seca (0..1)
};

struct UnidadBalanceMetalurgico {
    int64_t id = -1;
    std::string nombre;
    std::vector<int64_t> entradas;
    std::vector<int64_t> salidas;
    bool seleccionada = true;
};

struct ConfigBalanceMetalurgico {
    ModoBalanceMetalurgico modo = ModoBalanceMetalurgico::Solidos;
    std::vector<CorrienteBalanceMetalurgico> corrientes;
    std::vector<UnidadBalanceMetalurgico> unidades;
};

struct CierreUnidadBalance {
    int64_t id = -1;
    std::string nombre;
    double solido_entrada_t_h = 0.0;
    double solido_salida_t_h = 0.0;
    double liquido_entrada_t_h = 0.0;
    double liquido_salida_t_h = 0.0;
    double error_solido_t_h = 0.0;
    double error_liquido_t_h = 0.0;
};

struct ResultadoBalanceMetalurgico {
    bool ok = false;
    std::string mensaje;
    std::map<int64_t, double> solido_reconciliado_t_h;
    std::map<int64_t, double> liquido_reconciliado_t_h;
    std::map<int64_t, double> residuo_solido_t_h;
    std::map<int64_t, double> residuo_liquido_t_h;
    std::map<int64_t, double> z_solido;
    std::map<int64_t, double> z_liquido;
    std::map<int64_t, bool> outlier_solido;
    std::map<int64_t, bool> outlier_liquido;
    std::vector<CierreUnidadBalance> cierres;
    std::vector<std::string> avisos;
    double chi2 = 0.0;
    int grados_libertad = 0;
    double q_ajuste = 0.0;
    int variables = 0;
    int variables_medidas = 0;
    int restricciones = 0;
    int iteraciones = 0;
};

class BalanceMetalurgico {
public:
    ResultadoBalanceMetalurgico resolver(const ConfigBalanceMetalurgico& config) const;
};

} // namespace motores
} // namespace cma
