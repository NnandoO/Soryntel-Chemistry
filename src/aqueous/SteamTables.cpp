#include "SteamTables.hpp"

namespace hsc_clone {

PropiedadesAgua SteamTables::propiedades(double T, double P) {
    auto raw = motor_.propiedades(T, P);
    return {raw.T, raw.P, raw.h, raw.s, raw.v, raw.cp, raw.cv, raw.fase};
}

}
