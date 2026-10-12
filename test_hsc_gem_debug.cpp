// Test del motor GEM (minimización de Gibbs) + integración con la BD
#include "base_datos/gestor_db.hpp"
#include "motores/minimizador_gibbs.hpp"
#include "nucleo/tabla_periodica.hpp"
#include <cmath>
#include <cstdio>
#include <set>

using namespace cma;

static int64_t id_de(const std::string& formula) {
    auto& gestor = base_datos::GestorDB::instancia();
    for (const auto* e : gestor.todas_especies())
        if (e->formula == formula) return e->id;
    return -1;
}

static int fallos = 0;
#define CHECK(cond, msg) do { if (!(cond)) { ++fallos; printf("[FAIL] %s\n", msg); } \
                              else { printf("[ OK ] %s\n", msg); } } while (0)

static double error_masa(const std::map<int64_t,double>& feed,
                         const std::map<int64_t,double>& res,
                         const std::map<int64_t, nucleo::Especie>& db) {
    auto& tp = nucleo::TablaPeriodica::instancia();
    std::map<std::string,double> ein, eout;
    for (auto& [sid, n] : feed) {
        auto it = db.find(sid); if (it == db.end()) continue;
        for (auto& ca : tp.analizar_formula(it->second.formula)) ein[ca.elem] += n * ca.cuenta;
    }
    for (auto& [sid, n] : res) {
        auto it = db.find(sid); if (it == db.end()) continue;
        for (auto& ca : tp.analizar_formula(it->second.formula)) eout[ca.elem] += n * ca.cuenta;
    }
    double err = 0.0;
    for (auto& [el, v] : ein) {
        double rel = std::abs(eout[el] - v) / std::max(std::abs(v), 1e-12);
        err = std::max(err, rel);
    }
    return err;
}

int main() {
    auto& gestor = base_datos::GestorDB::instancia();
    auto db = gestor.mapa_especies();
    motores::MinimizadorGibbs gm;

    // ── 1) Combustión CH4: 1 CH4 + 2 O2, gases C/H/O a 1000 K ─────────
    {
        std::vector<int64_t> pos;
        std::map<int64_t,double> feed;
        for (const char* f : {"CH4","O2","CO2","CO","H2O(g)","H2","H2O","NO","N2"})
            { int64_t id = id_de(f); if (id >= 0) pos.push_back(id); }
        feed[id_de("CH4")] = 1.0;
        feed[id_de("O2")]  = 2.0;

        auto r = gm.minimizar(feed, 1000.0, 1.0, db, pos);
        printf("       CH4+2O2 @1000K: conv=%d iters=%d | CO2=%.4f H2Og=%.4f CO=%.4e H2=%.4e O2=%.4e\n",
            r.convergio, r.iteraciones,
            r.flujo_molar[id_de("CO2")], r.flujo_molar[id_de("H2O(g)")],
            r.flujo_molar[id_de("CO")], r.flujo_molar[id_de("H2")], r.flujo_molar[id_de("O2")]);
        CHECK(r.convergio, "combustión CH4 converge a 1000 K");
        CHECK(std::abs(r.flujo_molar[id_de("CO2")] - 1.0) < 0.05, "CO2 ≈ 1 kmol");
        CHECK(std::abs(r.flujo_molar[id_de("H2O(g)")] - 2.0) < 0.05, "H2O ≈ 2 kmol");
        CHECK(r.flujo_molar[id_de("O2")] < 1e-3, "O2 residual ≈ 0");
        CHECK(error_masa(feed, r.flujo_molar, db) < 1e-6, "balance de masa C/H/O");
    }

    // ── 2) Sistema HSC de la imagen: Na2SO4 + 2 C + N2 (barrido T) ─────
    {
        std::vector<int64_t> pos;
        std::set<std::string> elems = {"Na","S","O","C","N"};
        auto& tp = nucleo::TablaPeriodica::instancia();
        for (auto& [sid, sp] : db) {
            auto atomos = tp.analizar_formula(sp.formula);
            if (atomos.empty()) continue;
            bool valid = true;
            for (auto& a : atomos) if (!elems.count(a.elem)) { valid = false; break; }
            if (valid) pos.push_back(sid);
        }
        printf("       especies subsistema {Na,S,O,C,N}: %zu\n", pos.size());
        CHECK(pos.size() > 20, "auto-incluye especies por elementos (HSC)");

        std::map<int64_t,double> feed;
        int64_t ns_id = id_de("Na2SO4(s)") >= 0 ? id_de("Na2SO4(s)") : id_de("Na2SO4");
        feed[ns_id] = 1.0;
        feed[id_de("C")] = 2.0;
        feed[id_de("N2")] = 1.0;

        std::map<int64_t,double> warm;
        int exitos = 0, tot_iters = 0;
        double nso4_400 = -1, nso4_900 = -1;
        for (double T = 400.0; T <= 1000.0; T += 100.0) {
            auto r = gm.minimizar(feed, T, 1.0, db, pos, &warm);
            printf("       T=%.0fK conv=%d iters=%d\n", T, r.convergio, r.iteraciones);
            if (r.convergio) { ++exitos; tot_iters += r.iteraciones; }
            if (r.convergio && error_masa(feed, r.flujo_molar, db) > 1e-5) {
                ++fallos; printf("[FAIL] masa no se conserva a T=%.0f K\n", T);
            }
            if (T == 400.0) nso4_400 = r.flujo_molar[ns_id];
            if (T == 900.0) nso4_900 = r.flujo_molar[ns_id];
        }
        printf("       barrido: %d/7 conv, iters med=%.1f, Na2SO4: %.3f(400K) -> %.3f(900K)\n",
               exitos, tot_iters/7.0, nso4_400, nso4_900);
        CHECK(exitos == 7, "converge en todo el barrido T (warm start)");
        if (nso4_400 >= 0 && nso4_900 >= 0)
            CHECK(nso4_900 < nso4_400, "Na2SO4 se descompone con T");
    }

    // ── 3) Modo celda: electroneutralidad con iones acuosos ────────────
    {
        int64_t fe2 = id_de("Fe(+2a)"), so4 = id_de("SO4(-2a)"), h2o = id_de("H2O");
        int64_t fe3 = id_de("Fe(+3a)"), h_a = id_de("H(+a)"), oh = id_de("OH(-a)");
        if (fe2 < 0 || so4 < 0 || h2o < 0) {
            printf("[SKIP] faltan iones acuosos\n");
        } else {
            std::vector<int64_t> pos;
            for (int64_t id : {fe2, fe3, so4, h_a, oh, h2o, id_de("O2"), id_de("H2")})
                if (id >= 0) pos.push_back(id);
            std::map<int64_t,double> feed;
            feed[fe2] = 1.0; feed[so4] = 1.0; feed[h2o] = 50.0;

            auto r = gm.minimizar(feed, 298.15, 1.0, db, pos, nullptr, true);
            CHECK(r.convergio, "modo celda (con e-) converge");
            double q = 0.0;
            for (auto& [sid, n] : r.flujo_molar) {
                auto it = db.find(sid); if (it == db.end()) continue;
                const std::string& f = it->second.formula;
                if (f == "e-") q -= n;
                size_t op = f.rfind('(');
                if (op != std::string::npos) {
                    size_t cl = f.find(')', op);
                    std::string tag = f.substr(op+1, cl-op-1);
                    if (!tag.empty() && (tag[0]=='+'||tag[0]=='-')) {
                        double v = 0; int s = (tag[0]=='+')?1:-1;
                        for (size_t k=1;k<tag.size()&&isdigit(tag[k]);++k) v=v*10+(tag[k]-'0');
                        q += n * s * (v?v:1.0);
                    }
                }
            }
            printf("       carga neta equilibrio = %.3e\n", q);
            CHECK(std::abs(q) < 1e-6, "electroneutralidad se conserva");
        }
    }

    // ── 4) Límite de especies ───────────────────────────────────────────
    {
        std::vector<int64_t> muchas;
        for (auto& [sid, sp] : db) muchas.push_back(sid);
        std::map<int64_t,double> feed;
        feed[muchas[0]] = 1.0;
        auto r = gm.minimizar(feed, 1000.0, 1.0, db, muchas);
        CHECK(!r.convergio && r.mensaje.find("Demasiadas especies") != std::string::npos,
              "límite de especies controlado");
    }

    printf("\n%s (%d fallos)\n", fallos ? "FALLO" : "TODO OK", fallos);
    return fallos ? 1 : 0;
}
