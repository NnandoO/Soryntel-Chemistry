# CoolProp IF97

Unmodified upstream header, MIT license.

- Repository: https://github.com/CoolProp/IF97
- Commit: `0be7b51f35c47e59e6f91f4f0f47108bf997e50c`
- Formulation: IAPWS R7-97(2012), https://iapws.org/technical-guidance/release/IF97-Rev
- Default SI units at this dependency boundary: K, Pa, J/kg.
- CMA converts to MPa and kJ/kg only in `src/motores/agua.cpp`.

Verification uses the official IAPWS tabulated verification points, not values
generated from this implementation. Keep the license with redistributions.
