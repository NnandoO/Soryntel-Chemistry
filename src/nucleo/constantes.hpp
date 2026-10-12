#pragma once
namespace cma { namespace nucleo {

constexpr double R_GAS       = 8.314462618;   // J/(mol·K)
constexpr double R_GAS_ATM   = 0.08205736608; // L·atm/(mol·K)
constexpr double R_GAS_CAL   = 1.9872036;     // cal/(mol·K)
constexpr double STP_TEMP    = 298.15;        // K
constexpr double STP_PRES    = 1.0;           // atm
constexpr double KELVIN      = 273.15;        // 0°C en K
constexpr double FARADAY     = 96485.33212;   // C/mol
constexpr double AVOGADRO    = 6.02214076e23; // mol⁻¹
constexpr double BOLTZMANN   = 1.380649e-23;  // J/K
constexpr double PLANCK      = 6.62607015e-34;// J·s
constexpr double ATM_TO_PA   = 101325.0;      // Pa
constexpr double CAL_TO_J    = 4.184;         // J/cal
constexpr double BAR_TO_ATM  = 0.986923;      // atm/bar

}} // namespace cma::nucleo
