#include "HeatLossCalc.hpp"

#include <algorithm>
#include <cctype>
#include <fstream>
#include <iomanip>
#include <sstream>

namespace cma::application::modules {

namespace {

std::string lower_trim(const std::string& s) {
    size_t b = 0;
    while (b < s.size() && std::isspace(static_cast<unsigned char>(s[b])))
        ++b;
    size_t e = s.size();
    while (e > b && std::isspace(static_cast<unsigned char>(s[e - 1])))
        --e;
    std::string out = s.substr(b, e - b);
    std::transform(out.begin(), out.end(), out.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return out;
}

cma::domain::ThermalSheet make_default_wall_sheet() {
    using namespace cma::domain;
    ThermalSheet s;
    s.name = "Wall";
    s.geometry = ThermalGeometryType::Cylinder;
    s.orientation = ConvectionOrientation::VerticalSurface;
    s.height_or_length_m = 1.0;
    s.inner_diameter_or_width_m = 0.5;
    s.total_heat_flow_kw = 13.471;
    s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

    // Col 1: Inner Surface (Molten metal bath)
    {
        ThermalColumn c;
        c.name = "Hot Face";
        c.type = ThermalColumnType::Surface;
        c.material = "Molten metal (p = 1 bar)";
        c.surface_material = "Ideal black surface";
        c.temp_left_c = 1100.0;
        c.temp_right_c = 1087.0;
        c.thickness_cm = 0.0;
        c.convection_hc_w_m2k = 100.0;
        c.radiation_hr_w_m2k = 579.216;
        c.emissivity = 1.0;
        c.resistance_c_kw = 0.937;
        c.fix_temp_left = true;
        c.fix_convection = true;
        s.columns.push_back(c);
    }
    // Col 2: Solid Layer - Forsterite refractory
    {
        ThermalColumn c;
        c.name = "Hot Face Brick";
        c.type = ThermalColumnType::SolidLayer;
        c.material = "Forsterite (Mg2SiO4)";
        c.temp_left_c = 1087.0;
        c.temp_right_c = 444.0;
        c.thickness_cm = 50.0;
        c.grid_points = 10;
        c.conductivity_w_mk = 3.849;
        c.resistance_c_kw = 47.729;
        c.fix_conductivity = false;
        s.columns.push_back(c);
    }
    // Col 3: Contact Layer 1
    {
        ThermalColumn c;
        c.name = "Mortar Gap 1";
        c.type = ThermalColumnType::ContactLayer;
        c.material = "Contact layer N/A";
        c.temp_left_c = 444.0;
        c.temp_right_c = 175.0;
        c.resistance_c_kw = 20.0;
        c.fix_resistance = true;
        s.columns.push_back(c);
    }
    // Col 4: Solid Layer - Stainless Steel AISI 347
    {
        ThermalColumn c;
        c.name = "Anchor Plate";
        c.type = ThermalColumnType::SolidLayer;
        c.material = "Stainless steel (AISI 347)";
        c.temp_left_c = 175.0;
        c.temp_right_c = 162.0;
        c.thickness_cm = 8.0;
        c.grid_points = 10;
        c.conductivity_w_mk = 16.444;
        c.resistance_c_kw = 0.981;
        c.fix_conductivity = true;
        s.columns.push_back(c);
    }
    // Col 5: Contact Layer 2
    {
        ThermalColumn c;
        c.name = "Expansion Gap";
        c.type = ThermalColumnType::ContactLayer;
        c.material = "Contact layer N/A";
        c.temp_left_c = 162.0;
        c.temp_right_c = 95.0;
        c.resistance_c_kw = 5.0;
        c.fix_resistance = true;
        s.columns.push_back(c);
    }
    // Col 6: Solid Layer - Carbon Steel AISI 1010
    {
        ThermalColumn c;
        c.name = "Furnace Shell";
        c.type = ThermalColumnType::SolidLayer;
        c.material = "Carbon steel AISI 1010";
        c.temp_left_c = 95.0;
        c.temp_right_c = 87.0;
        c.thickness_cm = 20.0;
        c.grid_points = 10;
        c.conductivity_w_mk = 60.588;
        c.resistance_c_kw = 0.567;
        c.fix_conductivity = true;
        s.columns.push_back(c);
    }
    // Col 7: Outer Surface (Ambient Air)
    {
        ThermalColumn c;
        c.name = "Ambient Air";
        c.type = ThermalColumnType::Surface;
        c.material = "Air (gas) (p = 1 bar)";
        c.surface_material = "Ideal black surface";
        c.temp_left_c = 87.0;
        c.temp_right_c = 25.0;
        c.thickness_cm = 0.0;
        c.convection_hc_w_m2k = 25.487;
        c.radiation_hr_w_m2k = 8.153;
        c.emissivity = 1.0;
        c.resistance_c_kw = 4.593;
        c.fix_temp_right = true;
        c.fix_convection = true;
        s.columns.push_back(c);
    }
    return s;
}

cma::domain::ThermalSheet make_default_roof_sheet() {
    using namespace cma::domain;
    ThermalSheet s;
    s.name = "Roof";
    s.geometry = ThermalGeometryType::PlaneWall;
    s.orientation = ConvectionOrientation::HorizontalHotUp;
    s.height_or_length_m = 2.0;
    s.inner_diameter_or_width_m = 2.0;
    s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

    ThermalColumn c1;
    c1.name = "Freeboard Gas";
    c1.type = ThermalColumnType::Surface;
    c1.material = "Flue gas (SO2/CO2/H2O)";
    c1.surface_material = "Refractory brick (rough)";
    c1.temp_left_c = 1150.0;
    c1.temp_right_c = 1130.0;
    c1.convection_hc_w_m2k = 35.0;
    c1.emissivity = 0.85;
    c1.fix_temp_left = true;
    s.columns.push_back(c1);

    ThermalColumn c2;
    c2.name = "High-Alumina Arch";
    c2.type = ThermalColumnType::SolidLayer;
    c2.material = "High-alumina brick (70% Al2O3)";
    c2.temp_left_c = 1130.0;
    c2.temp_right_c = 520.0;
    c2.thickness_cm = 30.0;
    c2.grid_points = 12;
    s.columns.push_back(c2);

    ThermalColumn c3;
    c3.name = "Ceramic Fiber";
    c3.type = ThermalColumnType::SolidLayer;
    c3.material = "Ceramic fiber blanket (128 kg/m3)";
    c3.temp_left_c = 520.0;
    c3.temp_right_c = 110.0;
    c3.thickness_cm = 10.0;
    c3.grid_points = 10;
    s.columns.push_back(c3);

    ThermalColumn c4;
    c4.name = "Roof Casing";
    c4.type = ThermalColumnType::SolidLayer;
    c4.material = "Carbon steel AISI 1010";
    c4.temp_left_c = 110.0;
    c4.temp_right_c = 108.0;
    c4.thickness_cm = 1.5;
    c4.grid_points = 6;
    s.columns.push_back(c4);

    ThermalColumn c5;
    c5.name = "Roof Ambient";
    c5.type = ThermalColumnType::Surface;
    c5.material = "Air (gas) (p = 1 bar)";
    c5.surface_material = "Oxidized carbon steel";
    c5.temp_left_c = 108.0;
    c5.temp_right_c = 25.0;
    c5.emissivity = 0.80;
    c5.fix_temp_right = true;
    s.columns.push_back(c5);

    return s;
}

cma::domain::ThermalSheet make_default_bottom_sheet() {
    using namespace cma::domain;
    ThermalSheet s;
    s.name = "Bottom";
    s.geometry = ThermalGeometryType::PlaneWall;
    s.orientation = ConvectionOrientation::HorizontalHotDown;
    s.height_or_length_m = 2.0;
    s.inner_diameter_or_width_m = 2.0;
    s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

    ThermalColumn c1;
    c1.name = "Molten Matte Bath";
    c1.type = ThermalColumnType::Surface;
    c1.material = "Molten metal (p = 1 bar)";
    c1.surface_material = "Refractory brick (rough)";
    c1.temp_left_c = 1180.0;
    c1.temp_right_c = 1170.0;
    c1.convection_hc_w_m2k = 180.0;
    c1.fix_temp_left = true;
    c1.fix_convection = true;
    s.columns.push_back(c1);

    ThermalColumn c2;
    c2.name = "Magnesite-Chrome Hearth";
    c2.type = ThermalColumnType::SolidLayer;
    c2.material = "Magnesite-chrome brick (MgO-Cr2O3)";
    c2.temp_left_c = 1170.0;
    c2.temp_right_c = 620.0;
    c2.thickness_cm = 45.0;
    c2.grid_points = 12;
    s.columns.push_back(c2);

    ThermalColumn c3;
    c3.name = "Fireclay Safety Lining";
    c3.type = ThermalColumnType::SolidLayer;
    c3.material = "Fireclay brick (Chamotte)";
    c3.temp_left_c = 620.0;
    c3.temp_right_c = 140.0;
    c3.thickness_cm = 23.0;
    c3.grid_points = 10;
    s.columns.push_back(c3);

    ThermalColumn c4;
    c4.name = "Bottom Steel Plate";
    c4.type = ThermalColumnType::SolidLayer;
    c4.material = "Carbon steel AISI 1010";
    c4.temp_left_c = 140.0;
    c4.temp_right_c = 136.0;
    c4.thickness_cm = 2.5;
    c4.grid_points = 6;
    s.columns.push_back(c4);

    ThermalColumn c5;
    c5.name = "Under-Hearth Air";
    c5.type = ThermalColumnType::Surface;
    c5.material = "Air (gas) (p = 1 bar)";
    c5.surface_material = "Oxidized carbon steel";
    c5.temp_left_c = 136.0;
    c5.temp_right_c = 25.0;
    c5.emissivity = 0.80;
    c5.fix_temp_right = true;
    s.columns.push_back(c5);

    return s;
}

} // namespace

// ═══════════════════════════════════════════════════════════════════════════
// DefaultThermalPropertyCatalog
// ═══════════════════════════════════════════════════════════════════════════

DefaultThermalPropertyCatalog::DefaultThermalPropertyCatalog() {
    restore_defaults();
}

void DefaultThermalPropertyCatalog::restore_defaults() {
    conduction_.clear();
    convection_.clear();
    radiation_.clear();

    // 1. Conduction Database (Refractories, Insulating, Steels & Metals, Slags & Skulls, Carbon & SiC)
    conduction_ = {{"Forsterite (Mg2SiO4)", "Refractories", 2850.0, 1750.0, 20.0, 4.95, -0.00145, 0.0, 0.0,
                    "HSC Conduction DB / Schacht", false},
                   {"Magnesite-chrome brick (MgO-Cr2O3)", "Refractories", 3050.0, 1780.0, 20.0, 4.20,
                    -0.00085, 0.0, 0.0, "Harbison-Walker Refractories", false},
                   {"Magnesia brick (95% MgO)", "Refractories", 2980.0, 1850.0, 20.0, 9.80, -0.00380, 0.0,
                    0.0, "NIST / Perry's", false},
                   {"High-alumina brick (70% Al2O3)", "Refractories", 2620.0, 1760.0, 20.0, 2.35, -0.00032,
                    0.0, 0.0, "ASTM C202 Refractory Data", false},
                   {"High-alumina brick (90% Corundum)", "Refractories", 2950.0, 1840.0, 20.0, 3.40, -0.00075,
                    0.0, 0.0, "RHI Magnesita Reference", false},
                   {"Fireclay brick (Chamotte)", "Refractories", 2150.0, 1400.0, 20.0, 1.05, 0.00035, 0.0,
                    0.0, "DIN 51049 Fireclay", false},
                   {"Silica brick (96% SiO2)", "Refractories", 1860.0, 1650.0, 20.0, 1.15, 0.00062, 0.0, 0.0,
                    "Coke Oven & Glass Arch Ref.", false},
                   {"Dense castable refractory (Al2O3-SiO2)", "Refractories", 2400.0, 1600.0, 20.0, 1.65,
                    0.00018, 0.0, 0.0, "Monolithic Lining Handbook", false},
                   {"Silicon carbide brick (SiC nitride-bonded)", "Carbon & SiC", 2680.0, 1650.0, 20.0, 19.50,
                    -0.00350, 0.0, 0.0, "Carborundum Blast Furnace", false},
                   {"Graphite block (electro-graphite)", "Carbon & SiC", 1720.0, 2500.0, 20.0, 115.0, -0.0420,
                    0.0, 0.0, "Submerged Arc Furnace Hearth", false},
                   {"Carbon ramming paste (anthracite)", "Carbon & SiC", 1580.0, 2000.0, 20.0, 11.50, 0.0018,
                    0.0, 0.0, "Ferroalloy Furnace Lining", false},
                   {"Insulating firebrick IFB-23 (JM 23)", "Insulating", 480.0, 1260.0, 20.0, 0.14, 0.00016,
                    0.0, 0.0, "Morgan Thermal Ceramics", false},
                   {"Insulating firebrick IFB-26 (JM 26)", "Insulating", 770.0, 1430.0, 20.0, 0.23, 0.00019,
                    0.0, 0.0, "Morgan Thermal Ceramics", false},
                   {"Ceramic fiber blanket (128 kg/m3)", "Insulating", 128.0, 1260.0, 20.0, 0.045, 0.00012,
                    1.2e-7, 0.0, "Kaowool / Fiberfrax", false},
                   {"Calcium silicate board (Promat/Skamol)", "Insulating", 245.0, 1000.0, 20.0, 0.072,
                    0.00011, 0.0, 0.0, "Backup Insulation Standard", false},
                   {"Mineral wool board (Rockwool)", "Insulating", 140.0, 750.0, 20.0, 0.038, 0.00014, 0.0,
                    0.0, "EN 14303 Industrial Wool", false},
                   {"Microporous silica panel (WDS)", "Insulating", 220.0, 1000.0, 20.0, 0.022, 0.00003, 0.0,
                    0.0, "Ultra-low k Ladle Backup", false},
                   {"Carbon steel AISI 1010", "Steels & Metals", 7850.0, 650.0, -50.0, 63.90, -0.0365, 0.0,
                    0.0, "ASM Metals Handbook Vol. 1", false},
                   {"Boiler plate steel (ASTM A516 Gr.70)", "Steels & Metals", 7850.0, 550.0, -50.0, 52.00,
                    -0.0280, 0.0, 0.0, "ASME Section II Part D", false},
                   {"Stainless steel (AISI 347)", "Steels & Metals", 7960.0, 870.0, -100.0, 14.20, 0.0133,
                    0.0, 0.0, "HSC Conduction DB / AISI", false},
                   {"Stainless steel (AISI 316L)", "Steels & Metals", 7990.0, 870.0, -100.0, 13.80, 0.0145,
                    0.0, 0.0, "ASM Metals Handbook", false},
                   {"Copper (C11000 water-cooled element)", "Steels & Metals", 8940.0, 450.0, -100.0, 398.0,
                    -0.0680, 0.0, 0.0, "Outotec Cooling Element DB", false},
                   {"Cast iron (Grey flake)", "Steels & Metals", 7200.0, 700.0, 0.0, 50.00, -0.0220, 0.0, 0.0,
                    "Tapblock Jacket Cast Iron", false},
                   {"Aluminum cladding (Alloy 3003)", "Steels & Metals", 2730.0, 400.0, -100.0, 190.0, 0.0400,
                    0.0, 0.0, "Pipe Insulation Jacket", false},
                   {"Frozen fayalite slag skull (FeO-SiO2)", "Slags & Skulls", 3550.0, 1175.0, 20.0, 1.85,
                    0.00045, 0.0, 0.0, "Copper Smelter Freeze Lining", false},
                   {"Frozen magnetite accretion (Fe3O4)", "Slags & Skulls", 4900.0, 1590.0, 20.0, 3.20,
                    -0.00050, 0.0, 0.0, "Flash Smelter Protective Skull", false},
                   {"Ilmenite titania freeze skull", "Slags & Skulls", 4200.0, 1480.0, 20.0, 2.40, 0.00020,
                    0.0, 0.0, "Ilmenite Smelting Furnace", false},
                   {"Structural concrete (Portland)", "Misc", 2300.0, 350.0, -20.0, 1.40, -0.00030, 0.0, 0.0,
                    "Foundation Pedestal", false},
                   {"Acid-resistant rubber lining", "Misc", 1180.0, 110.0, -20.0, 0.16, 0.00010, 0.0, 0.0,
                    "Hydrometallurgical Autoclave", false}};

    // 2. Convection Database (Gases, Liquids, Molten Melts)
    convection_ = {{"Air (gas) (p = 1 bar)", "Gases", 1.0, 5.0, 28.0, 0.0262, 1.58e-5, 0.71, 0.0034, true,
                    "Dry atmospheric air (300 K base)"},
                   {"Flue gas (SO2/CO2/H2O)", "Gases", 1.0, 18.0, 65.0, 0.0320, 1.85e-5, 0.74, 0.0010, true,
                    "Smelter offgas stream"},
                   {"Steam (superheated 1 bar)", "Gases", 1.0, 12.0, 85.0, 0.0248, 2.02e-5, 1.00, 0.0025,
                    true, "IAPWS Steam Vapor"},
                   {"Nitrogen (N2 gas)", "Gases", 1.0, 5.2, 30.0, 0.0259, 1.56e-5, 0.71, 0.0034, true,
                    "Inert purge gas"},
                   {"Hydrogen (H2 reducing gas)", "Gases", 1.0, 38.0, 190.0, 0.1820, 1.08e-4, 0.70, 0.0034,
                    true, "Direct reduction shaft gas"},
                   {"Cooling water (forced jacket)", "Liquids", 3.0, 450.0, 3800.0, 0.615, 8.0e-7, 5.40,
                    0.0003, false, "Water cooling channels (1-3 m/s)"},
                   {"Boiling water / Steam drum", "Liquids", 40.0, 6500.0, 15000.0, 0.670, 2.9e-7, 1.75,
                    0.0008, false, "WHB membrane wall nucleate boiling"},
                   {"Molten metal (p = 1 bar)", "Molten Melts", 1.0, 100.0, 850.0, 32.0, 4.5e-7, 0.05,
                    0.00012, false, "Molten copper / blister bath"},
                   {"Molten copper matte (Cu-Fe-S)", "Molten Melts", 1.0, 220.0, 1200.0, 14.5, 6.2e-7, 0.18,
                    0.00015, false, " agitated matte bath (1180-1250 C)"},
                   {"Molten fayalite slag (FeO-SiO2)", "Molten Melts", 1.0, 140.0, 550.0, 2.10, 8.5e-5, 45.0,
                    0.00010, false, "Slag bath natural/lance convection"}};

    // 3. Surface Radiation Database
    radiation_ = {
        {"Ideal black surface", "Ideal", 1.00, 0.0, "Blackbody reference (eps = 1.0)", false},
        {"Refractory brick (rough)", "Refractories", 0.85, -0.00005, "Fireclay / magnesia / alumina face",
         false},
        {"Alumina white refractory", "Refractories", 0.72, -0.00012, "High-purity Al2O3 lining", false},
        {"Silicon carbide / Graphite", "Refractories", 0.90, 0.00002, "SiC / Carbon lining", false},
        {"Oxidized carbon steel", "Oxidized Metals", 0.80, 0.00004, "Unpainted structural furnace shell",
         false},
        {"Heavilyrusted / scaled steel", "Oxidized Metals", 0.88, 0.00002, "Weathered rotary kiln shell",
         false},
        {"Oxidized stainless steel (304/347)", "Oxidized Metals", 0.45, 0.00025,
         "Heat-tinted stainless plate", false},
        {"Polished stainless steel", "Polished Metals", 0.16, 0.00008, "Bright radiation shield", false},
        {"Polished aluminum cladding", "Polished Metals", 0.09, 0.00004, "Low-emissivity pipe jacketing",
         false},
        {"Oxidized copper block", "Oxidized Metals", 0.65, 0.00010, "Copper cooling element face", false},
        {"Heat-resistant aluminum paint", "Paints & Coatings", 0.35, 0.00010,
         "Silicone-Al furnace shell coating", false},
        {"High-emissivity black coating", "Paints & Coatings", 0.94, 0.0,
         "Industrial high-epsilon shell paint", false}};
}

const cma::domain::ConductionMaterialEntry*
DefaultThermalPropertyCatalog::find_conduction(const std::string& name) const {
    const std::string key = lower_trim(name);
    for (const auto& m : conduction_) {
        if (lower_trim(m.name) == key)
            return &m;
    }
    return nullptr;
}

const cma::domain::ConvectionFluidEntry*
DefaultThermalPropertyCatalog::find_convection(const std::string& name) const {
    const std::string key = lower_trim(name);
    for (const auto& f : convection_) {
        if (lower_trim(f.name) == key)
            return &f;
    }
    return nullptr;
}

const cma::domain::SurfaceRadiationEntry*
DefaultThermalPropertyCatalog::find_surface(const std::string& name) const {
    const std::string key = lower_trim(name);
    for (const auto& r : radiation_) {
        if (lower_trim(r.name) == key)
            return &r;
    }
    return nullptr;
}

void DefaultThermalPropertyCatalog::add_or_update_conduction(
    const cma::domain::ConductionMaterialEntry& entry) {
    const std::string key = lower_trim(entry.name);
    for (auto& m : conduction_) {
        if (lower_trim(m.name) == key) {
            m = entry;
            return;
        }
    }
    conduction_.push_back(entry);
}

void DefaultThermalPropertyCatalog::add_or_update_surface(const cma::domain::SurfaceRadiationEntry& entry) {
    const std::string key = lower_trim(entry.name);
    for (auto& s : radiation_) {
        if (lower_trim(s.name) == key) {
            s = entry;
            return;
        }
    }
    radiation_.push_back(entry);
}

// ═══════════════════════════════════════════════════════════════════════════
// HeatLossCalc Application Service
// ═══════════════════════════════════════════════════════════════════════════

HeatLossCalc::HeatLossCalc() {
    load_preset(0);
}

cma::motores::CapaAislante HeatLossCalc::toCapaAislante(const Capa& c) {
    cma::motores::CapaAislante ca;
    ca.material = c.material;
    ca.espesor = c.espesor;
    ca.k = c.k;
    return ca;
}

ResultadoPerdida HeatLossCalc::plano(double T_int, double T_ext, double area,
                                     const std::vector<Capa>& capas) {
    std::vector<cma::motores::CapaAislante> motor_capas;
    motor_capas.reserve(capas.size());
    for (const auto& c : capas)
        motor_capas.push_back(toCapaAislante(c));

    auto motor_res = motor_.pared_plana(T_int, T_ext, area, motor_capas);

    ResultadoPerdida res;
    res.flujo_calor = motor_res.flujo_calor;
    res.resistencia_total = motor_res.resistencia_total;
    res.mensaje = motor_res.mensaje;
    res.ok = motor_res.ok;
    return res;
}

void HeatLossCalc::set_active_sheet_index(int idx) {
    if (sheets_.empty()) {
        sheets_.push_back(make_default_wall_sheet());
    }
    active_sheet_ = std::clamp(idx, 0, static_cast<int>(sheets_.size()) - 1);
}

cma::domain::ThermalSheet& HeatLossCalc::active_sheet() {
    if (sheets_.empty()) {
        sheets_.push_back(make_default_wall_sheet());
        active_sheet_ = 0;
    }
    active_sheet_ = std::clamp(active_sheet_, 0, static_cast<int>(sheets_.size()) - 1);
    return sheets_[static_cast<size_t>(active_sheet_)];
}

const cma::domain::ThermalSheet& HeatLossCalc::active_sheet() const {
    return sheets_[static_cast<size_t>(std::clamp(active_sheet_, 0, static_cast<int>(sheets_.size()) - 1))];
}

void HeatLossCalc::add_sheet(const std::string& name) {
    auto s = make_default_wall_sheet();
    s.name = name.empty() ? ("Zone " + std::to_string(sheets_.size() + 1)) : name;
    motor_.resolver_hoja(s, &catalog_);
    sheets_.push_back(std::move(s));
    active_sheet_ = static_cast<int>(sheets_.size()) - 1;
}

void HeatLossCalc::duplicate_active_sheet() {
    if (sheets_.empty())
        return;
    auto copy = active_sheet();
    copy.name += " (Copy)";
    sheets_.push_back(std::move(copy));
    active_sheet_ = static_cast<int>(sheets_.size()) - 1;
}

bool HeatLossCalc::remove_sheet(int idx) {
    if (sheets_.size() <= 1 || idx < 0 || static_cast<size_t>(idx) >= sheets_.size()) {
        return false;
    }
    sheets_.erase(sheets_.begin() + idx);
    active_sheet_ = std::clamp(active_sheet_, 0, static_cast<int>(sheets_.size()) - 1);
    return true;
}

void HeatLossCalc::load_preset(int preset_id) {
    using namespace cma::domain;
    sheets_.clear();
    active_sheet_ = 0;

    if (preset_id == 0) {
        // Preset 0: HSC Reference Smelting Furnace (Wall, Roof, Bottom)
        sheets_.push_back(make_default_wall_sheet());
        sheets_.push_back(make_default_roof_sheet());
        sheets_.push_back(make_default_bottom_sheet());
    } else if (preset_id == 1) {
        // Preset 1: Water-Cooled Copper Freeze Lining (Slag Bath -> Frozen Skull -> Ramming -> Cu Cooler ->
        // Cooling Water)
        ThermalSheet s;
        s.name = "Slag Zone Freeze Lining";
        s.geometry = ThermalGeometryType::PlaneWall;
        s.orientation = ConvectionOrientation::VerticalSurface;
        s.height_or_length_m = 1.5;
        s.inner_diameter_or_width_m = 4.0;
        s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

        ThermalColumn c1;
        c1.name = "Molten Slag Bath";
        c1.type = ThermalColumnType::Surface;
        c1.material = "Molten fayalite slag (FeO-SiO2)";
        c1.surface_material = "Refractory brick (rough)";
        c1.temp_left_c = 1260.0;
        c1.temp_right_c = 1175.0;
        c1.convection_hc_w_m2k = 280.0;
        c1.fix_temp_left = true;
        c1.fix_convection = true;
        s.columns.push_back(c1);

        ThermalColumn c2;
        c2.name = "Frozen Slag Skull";
        c2.type = ThermalColumnType::SolidLayer;
        c2.material = "Frozen fayalite slag skull (FeO-SiO2)";
        c2.temp_left_c = 1175.0;
        c2.temp_right_c = 420.0;
        c2.thickness_cm = 6.5;
        c2.grid_points = 12;
        s.columns.push_back(c2);

        ThermalColumn c3;
        c3.name = "SiC Castable Face";
        c3.type = ThermalColumnType::SolidLayer;
        c3.material = "Silicon carbide brick (SiC nitride-bonded)";
        c3.temp_left_c = 420.0;
        c3.temp_right_c = 180.0;
        c3.thickness_cm = 12.0;
        c3.grid_points = 10;
        s.columns.push_back(c3);

        ThermalColumn c4;
        c4.name = "Copper Waffle Block";
        c4.type = ThermalColumnType::SolidLayer;
        c4.material = "Copper (C11000 water-cooled element)";
        c4.temp_left_c = 180.0;
        c4.temp_right_c = 65.0;
        c4.thickness_cm = 15.0;
        c4.grid_points = 10;
        s.columns.push_back(c4);

        ThermalColumn c5;
        c5.name = "Cooling Water Channel";
        c5.type = ThermalColumnType::Surface;
        c5.material = "Cooling water (forced jacket)";
        c5.surface_material = "Oxidized copper block";
        c5.temp_left_c = 65.0;
        c5.temp_right_c = 35.0;
        c5.convection_hc_w_m2k = 3200.0;
        c5.fix_temp_right = true;
        c5.fix_convection = true;
        s.columns.push_back(c5);

        sheets_.push_back(s);
        sheets_.push_back(make_default_roof_sheet());
        sheets_.push_back(make_default_bottom_sheet());
    } else if (preset_id == 2) {
        // Preset 2: Rotary Kiln Cylindrical Refractory + Backup Insulation
        ThermalSheet s;
        s.name = "Rotary Kiln Hot Zone";
        s.geometry = ThermalGeometryType::HorizontalCylinder;
        s.orientation = ConvectionOrientation::HorizontalCylinder;
        s.height_or_length_m = 12.0;
        s.inner_diameter_or_width_m = 3.6;
        s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

        ThermalColumn c1;
        c1.name = "Kiln Gas & Bed";
        c1.type = ThermalColumnType::Surface;
        c1.material = "Flue gas (SO2/CO2/H2O)";
        c1.surface_material = "Refractory brick (rough)";
        c1.temp_left_c = 1320.0;
        c1.temp_right_c = 1295.0;
        c1.convection_hc_w_m2k = 45.0;
        c1.fix_temp_left = true;
        s.columns.push_back(c1);

        ThermalColumn c2;
        c2.name = "Mag-Chrome Working Lining";
        c2.type = ThermalColumnType::SolidLayer;
        c2.material = "Magnesite-chrome brick (MgO-Cr2O3)";
        c2.thickness_cm = 22.0;
        c2.grid_points = 12;
        s.columns.push_back(c2);

        ThermalColumn c3;
        c3.name = "Insulating Firebrick IFB-26";
        c3.type = ThermalColumnType::SolidLayer;
        c3.material = "Insulating firebrick IFB-26 (JM 26)";
        c3.thickness_cm = 7.5;
        c3.grid_points = 10;
        s.columns.push_back(c3);

        ThermalColumn c4;
        c4.name = "Kiln Tyre Shell";
        c4.type = ThermalColumnType::SolidLayer;
        c4.material = "Boiler plate steel (ASTM A516 Gr.70)";
        c4.thickness_cm = 3.2;
        c4.grid_points = 6;
        s.columns.push_back(c4);

        ThermalColumn c5;
        c5.name = "Outdoor Wind (3 m/s)";
        c5.type = ThermalColumnType::Surface;
        c5.material = "Air (gas) (p = 1 bar)";
        c5.surface_material = "Heavilyrusted / scaled steel";
        c5.temp_right_c = 20.0;
        c5.velocity_m_s = 3.0;
        c5.fix_temp_right = true;
        s.columns.push_back(c5);

        sheets_.push_back(s);
    } else {
        // Preset 3: Insulated Steam / Offgas Pipeline
        ThermalSheet s;
        s.name = "Steam Pipeline DN300";
        s.geometry = ThermalGeometryType::HorizontalCylinder;
        s.orientation = ConvectionOrientation::HorizontalCylinder;
        s.height_or_length_m = 50.0;
        s.inner_diameter_or_width_m = 0.30;
        s.solver_mode = HeatLossSolverMode::HeatFlowTwoTemps;

        ThermalColumn c1;
        c1.name = "Superheated Steam";
        c1.type = ThermalColumnType::Surface;
        c1.material = "Steam (superheated 1 bar)";
        c1.surface_material = "Oxidized carbon steel";
        c1.temp_left_c = 420.0;
        c1.convection_hc_w_m2k = 180.0;
        c1.fix_temp_left = true;
        c1.fix_convection = true;
        s.columns.push_back(c1);

        ThermalColumn c2;
        c2.name = "Steel Pipe Wall";
        c2.type = ThermalColumnType::SolidLayer;
        c2.material = "Carbon steel AISI 1010";
        c2.thickness_cm = 1.2;
        c2.grid_points = 6;
        s.columns.push_back(c2);

        ThermalColumn c3;
        c3.name = "Calcium Silicate Shell";
        c3.type = ThermalColumnType::SolidLayer;
        c3.material = "Calcium silicate board (Promat/Skamol)";
        c3.thickness_cm = 6.0;
        c3.grid_points = 10;
        s.columns.push_back(c3);

        ThermalColumn c4;
        c4.name = "Mineral Wool Mat";
        c4.type = ThermalColumnType::SolidLayer;
        c4.material = "Mineral wool board (Rockwool)";
        c4.thickness_cm = 5.0;
        c4.grid_points = 10;
        s.columns.push_back(c4);

        ThermalColumn c5;
        c5.name = "Al Cladding & Air";
        c5.type = ThermalColumnType::Surface;
        c5.material = "Air (gas) (p = 1 bar)";
        c5.surface_material = "Polished aluminum cladding";
        c5.temp_right_c = 22.0;
        c5.velocity_m_s = 1.5;
        c5.fix_temp_right = true;
        s.columns.push_back(c5);

        sheets_.push_back(s);
    }

    solve_all_sheets();
}

bool HeatLossCalc::solve_active_sheet() {
    if (sheets_.empty())
        return false;
    return motor_.resolver_hoja(active_sheet(), &catalog_);
}

bool HeatLossCalc::solve_all_sheets() {
    bool all_ok = true;
    for (auto& s : sheets_) {
        all_ok = motor_.resolver_hoja(s, &catalog_) && all_ok;
    }
    return all_ok;
}

cma::domain::TargetThicknessResult
HeatLossCalc::optimize_layer_thickness(const cma::domain::TargetThicknessRequest& req) {
    return motor_.optimizar_espesor_capa(active_sheet(), req, &catalog_);
}

cma::domain::GasParticleRadiationResult
HeatLossCalc::calculate_gas_radiation(const cma::domain::GasParticleRadiationInput& input) const {
    return cma::motores::PerdidaCalor::calcular_radiacion_gas_particulas(input);
}

double HeatLossCalc::evaluate_surface_convection(const std::string& fluid_name, double t_surf_c,
                                                 double t_fluid_c, double length_m, double velocity_m_s,
                                                 cma::domain::ConvectionOrientation orient) const {
    const auto* fluid = catalog_.find_convection(fluid_name);
    return cma::motores::PerdidaCalor::calcular_conveccion_superficie(fluid, t_surf_c, t_fluid_c, length_m,
                                                                      velocity_m_s, orient);
}

double HeatLossCalc::total_reactor_heat_flow_kw() const {
    double sum = 0.0;
    for (const auto& s : sheets_)
        sum += s.total_heat_flow_kw;
    return sum;
}

double HeatLossCalc::total_reactor_mass_kg() const {
    double sum = 0.0;
    for (const auto& s : sheets_)
        sum += s.total_mass_kg;
    return sum;
}

double HeatLossCalc::total_outer_area_m2() const {
    double sum = 0.0;
    for (const auto& s : sheets_)
        sum += s.outer_area_m2;
    return sum;
}

bool HeatLossCalc::export_active_profile_csv(const std::string& path) const {
    std::ofstream out(path);
    if (!out.is_open())
        return false;
    const auto& s = active_sheet();
    out << "# HSC Heat Loss Profile Export - CMA Chemistry\n";
    out << "# Sheet," << s.name << ",TotalHeatFlow_kW," << s.total_heat_flow_kw
        << ",TotalResistance_C_per_kW," << s.total_resistance_c_kw << ",OuterShellTemp_C,"
        << s.outer_shell_temp_c << "\n";
    out << "Point,Distance_mm,Temperature_C,Conductivity_W_mK,HeatFlux_kW_m2,MaxLimit_C,Column,Material\n";
    out << std::fixed << std::setprecision(4);
    for (size_t i = 0; i < s.profile.size(); ++i) {
        const auto& pt = s.profile[i];
        out << (i + 1) << "," << pt.distance_mm << "," << pt.temperature_c << "," << pt.conductivity_w_mk
            << "," << pt.heat_flux_kw_m2 << "," << pt.max_limit_c << ",\"" << pt.column_name << "\",\""
            << pt.material_name << "\"\n";
    }
    return true;
}

bool HeatLossCalc::save_workbook_tsv(const std::string& path) const {
    std::ofstream out(path);
    if (!out.is_open())
        return false;
    out << "CMA_HEAT_LOSS_WORKBOOK_V1\n";
    out << std::fixed << std::setprecision(6);
    for (const auto& s : sheets_) {
        out << "SHEET\t" << s.name << "\t" << static_cast<int>(s.geometry) << "\t"
            << static_cast<int>(s.orientation) << "\t" << s.height_or_length_m << "\t"
            << s.inner_diameter_or_width_m << "\t" << s.total_heat_flow_kw << "\t"
            << static_cast<int>(s.solver_mode) << "\n";
        for (const auto& c : s.columns) {
            out << "COL\t" << c.name << "\t" << static_cast<int>(c.type) << "\t" << c.material << "\t"
                << c.surface_material << "\t" << c.temp_left_c << "\t" << c.temp_right_c << "\t"
                << c.thickness_cm << "\t" << c.grid_points << "\t" << c.conductivity_w_mk << "\t"
                << c.convection_hc_w_m2k << "\t" << c.radiation_hr_w_m2k << "\t" << c.emissivity << "\t"
                << c.velocity_m_s << "\t" << c.resistance_c_kw << "\t" << (c.fix_temp_left ? 1 : 0) << "\t"
                << (c.fix_temp_right ? 1 : 0) << "\t" << (c.fix_conductivity ? 1 : 0) << "\t"
                << (c.fix_convection ? 1 : 0) << "\t" << (c.fix_radiation ? 1 : 0) << "\t"
                << (c.fix_resistance ? 1 : 0) << "\n";
        }
        out << "END_SHEET\n";
    }
    return true;
}

bool HeatLossCalc::load_workbook_tsv(const std::string& path) {
    std::ifstream in(path);
    if (!in.is_open())
        return false;
    std::string header;
    if (!std::getline(in, header) || header.find("CMA_HEAT_LOSS_WORKBOOK_V1") == std::string::npos) {
        return false;
    }

    auto split_tabs = [](const std::string& s) {
        std::vector<std::string> parts;
        std::stringstream ss(s);
        std::string item;
        while (std::getline(ss, item, '\t'))
            parts.push_back(item);
        return parts;
    };

    std::vector<cma::domain::ThermalSheet> loaded;
    cma::domain::ThermalSheet cur;
    bool in_sheet = false;
    std::string line;

    while (std::getline(in, line)) {
        if (!line.empty() && line.back() == '\r')
            line.pop_back();
        if (line.empty())
            continue;
        auto cols = split_tabs(line);
        if (cols.empty())
            continue;

        if (cols[0] == "SHEET" && cols.size() >= 8) {
            cur = cma::domain::ThermalSheet{};
            cur.name = cols[1];
            cur.geometry = static_cast<cma::domain::ThermalGeometryType>(std::stoi(cols[2]));
            cur.orientation = static_cast<cma::domain::ConvectionOrientation>(std::stoi(cols[3]));
            cur.height_or_length_m = std::stod(cols[4]);
            cur.inner_diameter_or_width_m = std::stod(cols[5]);
            cur.total_heat_flow_kw = std::stod(cols[6]);
            cur.solver_mode = static_cast<cma::domain::HeatLossSolverMode>(std::stoi(cols[7]));
            in_sheet = true;
        } else if (cols[0] == "END_SHEET" && in_sheet) {
            loaded.push_back(cur);
            in_sheet = false;
        } else if (in_sheet && cols[0] == "COL" && cols.size() >= 21) {
            cma::domain::ThermalColumn c;
            c.name = cols[1];
            c.type = static_cast<cma::domain::ThermalColumnType>(std::stoi(cols[2]));
            c.material = cols[3];
            c.surface_material = cols[4];
            c.temp_left_c = std::stod(cols[5]);
            c.temp_right_c = std::stod(cols[6]);
            c.thickness_cm = std::stod(cols[7]);
            c.grid_points = std::stoi(cols[8]);
            c.conductivity_w_mk = std::stod(cols[9]);
            c.convection_hc_w_m2k = std::stod(cols[10]);
            c.radiation_hr_w_m2k = std::stod(cols[11]);
            c.emissivity = std::stod(cols[12]);
            c.velocity_m_s = std::stod(cols[13]);
            c.resistance_c_kw = std::stod(cols[14]);
            c.fix_temp_left = (std::stoi(cols[15]) != 0);
            c.fix_temp_right = (std::stoi(cols[16]) != 0);
            c.fix_conductivity = (std::stoi(cols[17]) != 0);
            c.fix_convection = (std::stoi(cols[18]) != 0);
            c.fix_radiation = (std::stoi(cols[19]) != 0);
            c.fix_resistance = (std::stoi(cols[20]) != 0);
            cur.columns.push_back(c);
        }
    }

    if (loaded.empty())
        return false;
    sheets_ = std::move(loaded);
    active_sheet_ = 0;
    solve_all_sheets();
    return true;
}

} // namespace cma::application::modules
