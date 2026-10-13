#pragma once
#include <cmath>
#include <string>
#include <vector>
namespace cma::application {
struct FlotationFit {
    bool ok=false;
    double rate_per_min=0, recovery_rmse=0;
    size_t samples=0;
    std::string message;
};
// Fixed, independently measured R-infinity. Log-linear fit through origin:
// -ln(1-R/Rinf)=k*t. This is not an unweighted least squares fit in R-space.
inline FlotationFit fit_flotation(const std::vector<std::vector<double>>& data,double limit) {
    FlotationFit result;
    if (!std::isfinite(limit) || limit<=0 || limit>1 || data.size()<3) {
        result.message="Se requieren >=3 muestras y R infinito medido en (0,1]."; return result;
    }
    double numerator=0,denominator=0;
    for (const auto& row : data) {
        if (row.size()!=2 || !std::isfinite(row[0]) || !std::isfinite(row[1]) || row[0]<0 ||
            row[1]<0 || row[1]>=limit || (row[0]==0 && row[1]!=0)) {
            result.message="CSV requiere dos columnas: tiempo [min] >=0 y recuperación fraccional 0<=R<R infinito."; return result;
        }
        numerator+=row[0]*(-std::log1p(-row[1]/limit)); denominator+=row[0]*row[0];
    }
    if (!std::isfinite(numerator) || !std::isfinite(denominator) || denominator<=0) {
        result.message="Tiempos sin información o fuera de rango numérico."; return result;
    }
    result.rate_per_min=numerator/denominator;
    double squared_error=0;
    for (const auto& row : data) {
        const double residual=limit*(-std::expm1(-result.rate_per_min*row[0]))-row[1];
        squared_error+=residual*residual;
    }
    result.recovery_rmse=std::sqrt(squared_error/static_cast<double>(data.size()));
    result.samples=data.size(); result.ok=true;
    result.message="Ajuste log-lineal con R infinito fijado. Verifique residuos y valide con muestras independientes.";
    return result;
}
}
