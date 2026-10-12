#pragma once
#include "nucleo/tipos.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include <Eigen/Dense>
#include <Eigen/Sparse>
#include <optional>

namespace cma { namespace motores {

struct ResultadoBalanceMasa {
    bool     convergio = false;
    std::vector<nucleo::Flujo> flujos_salida;
    std::string mensaje;
    std::map<int64_t, double> balance_global; // especie_id -> balance neto
    int      iteraciones = 0;  // iteraciones usadas en lazos de recirculación
    double   error_final = 0.0; // error de convergencia final (lazos de reciclo)
};

class BalanceMasa {
public:
    BalanceMasa() = default;

    ResultadoBalanceMasa resolver(const diagrama_flujo::Flowsheet& fs);

    // Operaciones unitarias individuales
    nucleo::Flujo mezclar(const std::vector<nucleo::Flujo>& entradas, int64_t id_salida = -1) const;
    std::vector<nucleo::Flujo> dividir(const nucleo::Flujo& entrada, const std::vector<double>& fracciones, int64_t base_id = 0) const;
    nucleo::Flujo reactor(const std::vector<nucleo::Flujo>& entradas, const nucleo::Reaccion& rxn, double conversion, double T, double P, int64_t id_salida = -1) const;
    nucleo::Flujo separador(const nucleo::Flujo& entrada, const std::map<int64_t, double>& frac_remocion, int64_t id_salida = -1) const;
    nucleo::Flujo intercambiador_calor(const nucleo::Flujo& entrada, double T_salida, double caida_presion = 0.0, int64_t id_salida = -1) const;
    nucleo::Flujo compresor(const nucleo::Flujo& entrada, double relacion_presion, double eficiencia = 0.85, int64_t id_salida = -1) const;
    nucleo::Flujo valvula(const nucleo::Flujo& entrada, double P_salida, int64_t id_salida = -1) const;

private:
    Eigen::VectorXd resolver_sistema_lineal(const Eigen::SparseMatrix<double>& A, const Eigen::VectorXd& b) const;
    nucleo::Flujo combinar_entradas(const std::vector<nucleo::Flujo>& entradas) const;
    // Procesa un nodo y actualiza el mapa de flujos de trabajo (lanza std::exception en errores)
    void procesar_nodo(const nucleo::Nodo& nodo,
                       std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const;
    // Fracción de una especie que va a la SALIDA 1 de unidades de separación (param "split_<id>").
    double fraccion_hacia_salida1(const nucleo::Nodo& nodo, int64_t sid, double por_defecto) const;
    // Separa la(s) entrada(s) en 2 corrientes por especie, con default según el tipo de unidad.
    void procesar_separacion(const nucleo::Nodo& nodo,
                             const std::vector<nucleo::Flujo>& entradas,
                             std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const;
    // Unidades tipo reactor/reacción (hornos, tostador, convertidor): combina entradas,
    // aplica reacción si hay "reaccion_id" y divide en las salidas conectadas.
    void procesar_reactor_generico(const nucleo::Nodo& nodo,
                                   const std::vector<nucleo::Flujo>& entradas,
                                   std::map<int64_t, nucleo::Flujo>& flujos_trabajo) const;
};

}} // namespace
