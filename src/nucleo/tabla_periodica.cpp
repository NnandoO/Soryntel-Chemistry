#include "nucleo/tabla_periodica.hpp"
#include <algorithm>
#include <regex>
#include <cctype>

namespace cma { namespace nucleo {

TablaPeriodica& TablaPeriodica::instancia() {
    static TablaPeriodica tp;
    return tp;
}

TablaPeriodica::TablaPeriodica() {
    // Formato: Z, simbolo, nombre_es, config_electronica, masa_atomica, electroneg, 
    //          T_fusion, T_ebullicion, densidad, afinidad_e, energia_ioniz, periodo, grupo
    struct ElemData {
        int Z; const char* sim; const char* nom; const char* conf;
        double M, EN, Tfus, Teb, dens, AE, EI;
        int per, gru;
    };

    const ElemData datos[] = {
        {1,"H","Hidrógeno","1s¹",1.008,2.20,14.01,20.28,0.00009,72.8,1312,1,1},
        {2,"He","Helio","1s²",4.003,0.0,0.95,4.22,0.00018,0.0,2372,1,18},
        {3,"Li","Litio","[He]2s¹",6.941,0.98,453.65,1615,0.534,59.6,520,2,1},
        {4,"Be","Berilio","[He]2s²",9.012,1.57,1560,2742,1.85,0.0,899,2,2},
        {5,"B","Boro","[He]2s²2p¹",10.811,2.04,2349,4200,2.34,26.7,801,2,13},
        {6,"C","Carbono","[He]2s²2p²",12.011,2.55,3823,4098,2.267,153.9,1086,2,14},
        {7,"N","Nitrógeno","[He]2s²2p³",14.007,3.04,63.15,77.36,0.00125,7.0,1402,2,15},
        {8,"O","Oxígeno","[He]2s²2p⁴",15.999,3.44,54.36,90.20,0.00143,141.0,1314,2,16},
        {9,"F","Flúor","[He]2s²2p⁵",18.998,3.98,53.53,85.03,0.00170,328.0,1681,2,17},
        {10,"Ne","Neón","[He]2s²2p⁶",20.180,0.0,24.56,27.07,0.00090,0.0,2081,2,18},
        {11,"Na","Sodio","[Ne]3s¹",22.990,0.93,370.87,1156,0.968,52.8,496,3,1},
        {12,"Mg","Magnesio","[Ne]3s²",24.305,1.31,923,1363,1.738,0.0,738,3,2},
        {13,"Al","Aluminio","[Ne]3s²3p¹",26.982,1.61,933.47,2792,2.70,42.5,578,3,13},
        {14,"Si","Silicio","[Ne]3s²3p²",28.086,1.90,1687,3538,2.33,133.6,787,3,14},
        {15,"P","Fósforo","[Ne]3s²3p³",30.974,2.19,317.25,553.6,1.823,72.0,1012,3,15},
        {16,"S","Azufre","[Ne]3s²3p⁴",32.065,2.58,388.36,717.8,2.07,200.0,1000,3,16},
        {17,"Cl","Cloro","[Ne]3s²3p⁵",35.453,3.16,171.65,239.11,0.00321,349.0,1251,3,17},
        {18,"Ar","Argón","[Ne]3s²3p⁶",39.948,0.0,83.80,87.30,0.00178,0.0,1521,3,18},
        {19,"K","Potasio","[Ar]4s¹",39.098,0.82,336.53,1032,0.856,48.4,419,4,1},
        {20,"Ca","Calcio","[Ar]4s²",40.078,1.00,1115,1757,1.55,2.37,590,4,2},
        {21,"Sc","Escandio","[Ar]3d¹4s²",44.956,1.36,1814,3109,2.99,18.1,631,4,3},
        {22,"Ti","Titanio","[Ar]3d²4s²",47.867,1.54,1941,3560,4.51,7.6,658,4,4},
        {23,"V","Vanadio","[Ar]3d³4s²",50.942,1.63,2183,3680,6.11,50.6,650,4,5},
        {24,"Cr","Cromo","[Ar]3d⁵4s¹",51.996,1.66,2180,2944,7.19,64.3,653,4,6},
        {25,"Mn","Manganeso","[Ar]3d⁵4s²",54.938,1.55,1519,2334,7.21,0.0,717,4,7},
        {26,"Fe","Hierro","[Ar]3d⁶4s²",55.845,1.83,1811,3134,7.87,15.7,759,4,8},
        {27,"Co","Cobalto","[Ar]3d⁷4s²",58.933,1.88,1768,3200,8.86,63.7,758,4,9},
        {28,"Ni","Níquel","[Ar]3d⁸4s²",58.693,1.91,1728,3186,8.91,112.0,737,4,10},
        {29,"Cu","Cobre","[Ar]3d¹⁰4s¹",63.546,1.90,1357.77,2835,8.96,118.4,745,4,11},
        {30,"Zn","Zinc","[Ar]3d¹⁰4s²",65.380,1.65,692.68,1180,7.13,0.0,906,4,12},
        {31,"Ga","Galio","[Ar]3d¹⁰4s²4p¹",69.723,1.81,302.91,2477,5.91,28.9,579,4,13},
        {32,"Ge","Germanio","[Ar]3d¹⁰4s²4p²",72.630,2.01,1211.4,3106,5.32,119.0,762,4,14},
        {33,"As","Arsénico","[Ar]3d¹⁰4s²4p³",74.922,2.18,1090,887,5.73,78.0,947,4,15},
        {34,"Se","Selenio","[Ar]3d¹⁰4s²4p⁴",78.971,2.55,494,958,4.81,195.0,941,4,16},
        {35,"Br","Bromo","[Ar]3d¹⁰4s²4p⁵",79.904,2.96,265.8,332.0,3.12,324.6,1140,4,17},
        {36,"Kr","Kriptón","[Ar]3d¹⁰4s²4p⁶",83.798,3.00,115.79,119.93,0.00375,0.0,1351,4,18},
        {37,"Rb","Rubidio","[Kr]5s¹",85.468,0.82,312.45,961,1.53,46.9,403,5,1},
        {38,"Sr","Estroncio","[Kr]5s²",87.620,0.95,1050,1655,2.64,5.03,549,5,2},
        {39,"Y","Itrio","[Kr]4d¹5s²",88.906,1.22,1799,3609,4.47,29.6,600,5,3},
        {40,"Zr","Circonio","[Kr]4d²5s²",91.224,1.33,2128,4682,6.52,41.1,640,5,4},
        {41,"Nb","Niobio","[Kr]4d⁴5s¹",92.906,1.60,2750,5017,8.57,86.1,652,5,5},
        {42,"Mo","Molibdeno","[Kr]4d⁵5s¹",95.950,2.16,2896,4912,10.28,71.9,684,5,6},
        {43,"Tc","Tecnecio","[Kr]4d⁵5s²",98.0,1.90,2430,4538,11.5,53.0,702,5,7},
        {44,"Ru","Rutenio","[Kr]4d⁷5s¹",101.070,2.20,2607,4423,12.37,101.3,710,5,8},
        {45,"Rh","Rodio","[Kr]4d⁸5s¹",102.906,2.28,2237,3968,12.45,109.7,720,5,9},
        {46,"Pd","Paladio","[Kr]4d¹⁰",106.420,2.20,1828.05,3236,12.02,53.7,804,5,10},
        {47,"Ag","Plata","[Kr]4d¹⁰5s¹",107.868,1.93,1234.93,2435,10.49,125.6,731,5,11},
        {48,"Cd","Cadmio","[Kr]4d¹⁰5s²",112.414,1.69,594.22,1040,8.69,0.0,868,5,12},
        {49,"In","Indio","[Kr]4d¹⁰5s²5p¹",114.818,1.78,429.75,2345,7.31,28.9,558,5,13},
        {50,"Sn","Estaño","[Kr]4d¹⁰5s²5p²",118.710,1.96,505.08,2875,7.31,107.3,709,5,14},
        {51,"Sb","Antimonio","[Kr]4d¹⁰5s²5p³",121.760,2.05,903.78,1860,6.68,101.1,831,5,15},
        {52,"Te","Telurio","[Kr]4d¹⁰5s²5p⁴",127.600,2.10,722.66,1261,6.24,190.2,869,5,16},
        {53,"I","Yodo","[Kr]4d¹⁰5s²5p⁵",126.904,2.66,386.85,457.4,4.93,295.2,1008,5,17},
        {54,"Xe","Xenón","[Kr]4d¹⁰5s²5p⁶",131.293,2.60,161.36,165.03,0.00589,0.0,1170,5,18},
        {55,"Cs","Cesio","[Xe]6s¹",132.905,0.79,301.59,944,1.93,45.5,376,6,1},
        {56,"Ba","Bario","[Xe]6s²",137.327,0.89,1000,2170,3.51,13.95,503,6,2},
        {57,"La","Lantano","[Xe]5d¹6s²",138.905,1.10,1193,3737,6.15,48.0,538,6,3},
        {58,"Ce","Cerio","[Xe]4f¹5d¹6s²",140.116,1.12,1071,3697,6.77,50.0,534,6,0},
        {59,"Pr","Praseodimio","[Xe]4f³6s²",140.908,1.13,1204,3793,6.77,50.0,527,6,0},
        {60,"Nd","Neodimio","[Xe]4f⁴6s²",144.243,1.14,1289,3347,7.01,50.0,533,6,0},
        {61,"Pm","Prometio","[Xe]4f⁵6s²",145.0,1.13,1315,3273,7.26,50.0,540,6,0},
        {62,"Sm","Samario","[Xe]4f⁶6s²",150.362,1.17,1345,2067,7.52,50.0,544,6,0},
        {63,"Eu","Europio","[Xe]4f⁷6s²",151.964,1.20,1099,1802,5.24,50.0,547,6,0},
        {64,"Gd","Gadolinio","[Xe]4f⁷5d¹6s²",157.250,1.20,1585,3546,7.90,50.0,593,6,0},
        {65,"Tb","Terbio","[Xe]4f⁹6s²",158.925,1.20,1629,3503,8.23,50.0,565,6,0},
        {66,"Dy","Disprosio","[Xe]4f¹⁰6s²",162.500,1.22,1680,2840,8.55,50.0,573,6,0},
        {67,"Ho","Holmio","[Xe]4f¹¹6s²",164.930,1.23,1734,2993,8.80,50.0,581,6,0},
        {68,"Er","Erbio","[Xe]4f¹²6s²",167.259,1.24,1802,3141,9.07,50.0,589,6,0},
        {69,"Tm","Tulio","[Xe]4f¹³6s²",168.934,1.25,1818,2223,9.32,50.0,597,6,0},
        {70,"Yb","Iterbio","[Xe]4f¹⁴6s²",173.054,1.10,1097,1469,6.57,50.0,603,6,0},
        {71,"Lu","Lutecio","[Xe]4f¹⁴5d¹6s²",174.967,1.27,1925,3675,9.84,50.0,524,6,0},
        {72,"Hf","Hafnio","[Xe]4f¹⁴5d²6s²",178.490,1.30,2506,4876,13.31,0.0,659,6,4},
        {73,"Ta","Tántalo","[Xe]4f¹⁴5d³6s²",180.948,1.50,3290,5731,16.65,31.0,761,6,5},
        {74,"W","Wolframio","[Xe]4f¹⁴5d⁴6s²",183.840,2.36,3695,5828,19.25,78.6,770,6,6},
        {75,"Re","Renio","[Xe]4f¹⁴5d⁵6s²",186.207,1.90,3459,5869,21.02,14.5,760,6,7},
        {76,"Os","Osmio","[Xe]4f¹⁴5d⁶6s²",190.230,2.20,3306,5285,22.59,106.1,840,6,8},
        {77,"Ir","Iridio","[Xe]4f¹⁴5d⁷6s²",192.217,2.20,2719,4701,22.56,151.0,880,6,9},
        {78,"Pt","Platino","[Xe]4f¹⁴5d⁹6s¹",195.084,2.28,2041.4,4098,21.45,205.0,870,6,10},
        {79,"Au","Oro","[Xe]4f¹⁴5d¹⁰6s¹",196.967,2.54,1337.33,3129,19.32,222.8,890,6,11},
        {80,"Hg","Mercurio","[Xe]4f¹⁴5d¹⁰6s²",200.592,2.00,234.32,629.88,13.53,0.0,1007,6,12},
        {81,"Tl","Talio","[Xe]4f¹⁴5d¹⁰6s²6p¹",204.383,1.62,577,1746,11.85,19.2,589,6,13},
        {82,"Pb","Plomo","[Xe]4f¹⁴5d¹⁰6s²6p²",207.200,2.33,600.61,2022,11.34,35.1,716,6,14},
        {83,"Bi","Bismuto","[Xe]4f¹⁴5d¹⁰6s²6p³",208.980,2.02,544.55,1837,9.78,91.2,703,6,15},
        {84,"Po","Polonio","[Xe]4f¹⁴5d¹⁰6s²6p⁴",209.0,2.00,527,1235,9.32,136.0,812,6,16},
        {85,"At","Ástato","[Xe]4f¹⁴5d¹⁰6s²6p⁵",210.0,2.20,575,610,0.0,270.0,920,6,17},
        {86,"Rn","Radón","[Xe]4f¹⁴5d¹⁰6s²6p⁶",222.0,0.0,202,211.3,0.00973,0.0,1037,6,18},
        {87,"Fr","Francio","[Rn]7s¹",223.0,0.70,300,950,0.0,0.0,380,7,1},
        {88,"Ra","Radio","[Rn]7s²",226.0,0.90,973,2010,5.5,0.0,509,7,2},
        {89,"Ac","Actinio","[Rn]6d¹7s²",227.0,1.10,1323,3473,10.07,0.0,499,7,3},
        {90,"Th","Torio","[Rn]6d²7s²",232.038,1.30,2023,5061,11.72,0.0,587,7,0},
        {91,"Pa","Protactinio","[Rn]5f²6d¹7s²",231.036,1.50,1845,4300,15.37,0.0,568,7,0},
        {92,"U","Uranio","[Rn]5f³6d¹7s²",238.029,1.38,1405.3,4404,18.95,0.0,598,7,0},
        {93,"Np","Neptunio","[Rn]5f⁴6d¹7s²",237.0,1.36,917,4273,20.25,0.0,605,7,0},
        {94,"Pu","Plutonio","[Rn]5f⁶7s²",244.0,1.28,912.5,3505,19.84,0.0,585,7,0},
        {95,"Am","Americio","[Rn]5f⁷7s²",243.0,1.30,1449,2880,13.67,0.0,578,7,0},
        {96,"Cm","Curio","[Rn]5f⁷6d¹7s²",247.0,1.30,1613,3383,13.51,0.0,581,7,0},
        {97,"Bk","Berkelio","[Rn]5f⁹7s²",247.0,1.30,1259,2900,14.78,0.0,601,7,0},
        {98,"Cf","Californio","[Rn]5f¹⁰7s²",251.0,1.30,1173,1743,15.1,0.0,608,7,0},
        {99,"Es","Einstenio","[Rn]5f¹¹7s²",252.0,1.30,1133,0,0.0,0.0,619,7,0},
        {100,"Fm","Fermio","[Rn]5f¹²7s²",257.0,1.30,1800,0,0.0,0.0,627,7,0},
        {101,"Md","Mendelevio","[Rn]5f¹³7s²",258.0,1.30,1100,0,0.0,0.0,635,7,0},
        {102,"No","Nobelio","[Rn]5f¹⁴7s²",259.0,1.30,1100,0,0.0,0.0,642,7,0},
        {103,"Lr","Lawrencio","[Rn]5f¹⁴7s²7p¹",266.0,1.30,1900,0,0.0,0.0,470,7,0},
        {104,"Rf","Rutherfordio","[Rn]5f¹⁴6d²7s²",267.0,0.0,2400,5800,0.0,0.0,580,7,4},
        {105,"Db","Dubnio","[Rn]5f¹⁴6d³7s²",268.0,0.0,0,0,0.0,0.0,0,7,5},
        {106,"Sg","Seaborgio","[Rn]5f¹⁴6d⁴7s²",269.0,0.0,0,0,0.0,0.0,0,7,6},
        {107,"Bh","Bohrio","[Rn]5f¹⁴6d⁵7s²",270.0,0.0,0,0,0.0,0.0,0,7,7},
        {108,"Hs","Hassio","[Rn]5f¹⁴6d⁶7s²",269.0,0.0,0,0,0.0,0.0,0,7,8},
        {109,"Mt","Meitnerio","[Rn]5f¹⁴6d⁷7s²",278.0,0.0,0,0,0.0,0.0,0,7,9},
        {110,"Ds","Darmstatio","[Rn]5f¹⁴6d⁹7s¹",281.0,0.0,0,0,0.0,0.0,0,7,10},
        {111,"Rg","Roentgenio","[Rn]5f¹⁴6d¹⁰7s¹",282.0,0.0,0,0,0.0,0.0,0,7,11},
        {112,"Cn","Copernicio","[Rn]5f¹⁴6d¹⁰7s²",285.0,0.0,0,0,0.0,0.0,0,7,12},
        {113,"Nh","Nihonio","[Rn]5f¹⁴6d¹⁰7s²7p¹",286.0,0.0,0,0,0.0,0.0,0,7,13},
        {114,"Fl","Flerovio","[Rn]5f¹⁴6d¹⁰7s²7p²",289.0,0.0,0,0,0.0,0.0,0,7,14},
        {115,"Mc","Moscovio","[Rn]5f¹⁴6d¹⁰7s²7p³",290.0,0.0,0,0,0.0,0.0,0,7,15},
        {116,"Lv","Livermorio","[Rn]5f¹⁴6d¹⁰7s²7p⁴",293.0,0.0,0,0,0.0,0.0,0,7,16},
        {117,"Ts","Tenesino","[Rn]5f¹⁴6d¹⁰7s²7p⁵",294.0,0.0,0,0,0.0,0.0,0,7,17},
        {118,"Og","Oganesón","[Rn]5f¹⁴6d¹⁰7s²7p⁶",294.0,0.0,0,0,0.0,0.0,0,7,18},
    };

    for (auto& d : datos) {
        Elemento e;
        e.numero_atomico = d.Z;
        e.simbolo = d.sim;
        e.nombre_es = d.nom;
        e.config_electronica = d.conf;
        e.masa_atomica = d.M;
        e.electronegatividad = d.EN;
        e.punto_fusion = d.Tfus;
        e.punto_ebullicion = d.Teb;
        e.densidad = d.dens;
        e.afinidad_electronica = d.AE;
        e.energia_ionizacion = d.EI;
        e.periodo = d.per;
        e.grupo = d.gru;
        elementos_.push_back(e);
    }

    for (size_t i = 0; i < elementos_.size(); ++i) {
        simb_a_idx_[elementos_[i].simbolo] = (int)i;
        num_a_idx_[elementos_[i].numero_atomico] = (int)i;
    }
}

const Elemento* TablaPeriodica::por_simbolo(const std::string& simb) const {
    auto it = simb_a_idx_.find(simb);
    return it != simb_a_idx_.end() ? &elementos_[it->second] : nullptr;
}

const Elemento* TablaPeriodica::por_numero(int z) const {
    auto it = num_a_idx_.find(z);
    return it != num_a_idx_.end() ? &elementos_[it->second] : nullptr;
}

const Elemento* TablaPeriodica::por_nombre(const std::string& nombre) const {
    for (auto& e : elementos_) {
        if (e.nombre_es == nombre) return &e;
    }
    return nullptr;
}

std::vector<TablaPeriodica::CompAtom> TablaPeriodica::analizar_formula(const std::string& formula) const {
    int pos = 0;
    auto parse_group = [&](auto& self, int& p) -> std::vector<CompAtom> {
        std::vector<CompAtom> group;
        while (p < formula.size() && formula[p] != ')') {
            if (formula[p] == '(') {
                p++; // saltar '('
                auto inner = self(self, p);
                if (p < formula.size() && formula[p] == ')') p++; // saltar ')'
                
                int mult = 0;
                while (p < formula.size() && std::isdigit(formula[p])) {
                    mult = mult * 10 + (formula[p] - '0');
                    p++;
                }
                if (mult == 0) mult = 1;
                
                for (auto& ca : inner) {
                    ca.cuenta *= mult;
                    bool found = false;
                    for (auto& g_ca : group) {
                        if (g_ca.elem == ca.elem) { g_ca.cuenta += ca.cuenta; found = true; break; }
                    }
                    if (!found) group.push_back(ca);
                }
            } else if (std::isupper(formula[p])) {
                std::string el(1, formula[p++]);
                if (p < formula.size() && std::islower(formula[p])) el += formula[p++];
                
                int cnt = 0;
                while (p < formula.size() && std::isdigit(formula[p])) {
                    cnt = cnt * 10 + (formula[p] - '0');
                    p++;
                }
                if (cnt == 0) cnt = 1;
                
                bool found = false;
                for (auto& ca : group) {
                    if (ca.elem == el) { ca.cuenta += cnt; found = true; break; }
                }
                if (!found) group.push_back({el, cnt});
            } else {
                p++; // ignorar otros caracteres (e.g., cargas +, -)
            }
        }
        return group;
    };
    
    return parse_group(parse_group, pos);
}

}} // namespace
