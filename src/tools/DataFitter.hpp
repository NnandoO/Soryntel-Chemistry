#pragma once
#include <vector>
#include <string>

namespace hsc_clone {

struct ResultadoFit {
    std::vector<double> coeficientes;
    double r_cuadrado = 0.0;
    std::string mensaje;
    bool ok = false;
};

class DataFitter {
public:
    ResultadoFit lineal(const std::vector<double>& x, const std::vector<double>& y);
    ResultadoFit polinomial(const std::vector<double>& x, const std::vector<double>& y, int grado);
};

} // namespace hsc_clone
