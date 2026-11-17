//! Module 15: Excel Add-Ins (Thermochemical & Mineralogical Spreadsheet Functions)
//!
//! Implements all 69 functions registered by HSC6 `AddIns/HSC6.XLL`, the 5 Mineralogy
//! functions from HSC 8-10 (`MineralCode`, `MineralFormula`, `MineralName`, `MineralC`,
//! `MineralProperty`), the 5 HSC-Sim particle functions (`StreamX`..`MineralX`),
//! the `StreamEQ` array equilibrium solver, the legacy `HSC5.xla` formula migrator,
//! and multi-suite interoperability (Microsoft Excel, LibreOffice Calc, ONLYOFFICE, WPS).

use crate::equilibrium::gem_solver::solve_single_point;
use crate::equilibrium::model::{
    EquilibriumAmountUnit, EquilibriumParams, EquilibriumPhase, EquilibriumSpeciesInput,
    EquilibriumSystem, SolutionModel,
};
use crate::minerals_data::{MineralRawRow, MINERALS_RAW};
use crate::spreadsheet_export::{ExportCell, ExportCellStyle, ExportCellValue, ExportSheet};
use crate::{evaluate_species_si, ChemicalSpecies, Phase, SpeciesCatalogPort, SpeciesRepository};
use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_JOULE, T_REF_K};
use soryntel_solver::{balance_reaction, parse_formula_composition, parse_input};
use std::collections::BTreeMap;

const NORMAL_MOLAR_VOL_NM3_PER_KMOL: f64 = 22.413996;

/// Session-wide measurement units controlled via `=UNITS(T; E)` or "Change measurement units...".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddInUnits {
    pub temp_unit: String,     // "C", "K", "F"
    pub energy_unit: String,   // "MJ", "kJ", "kcal", "Mcal", "kWh", "J"
    pub pressure_unit: String, // "bar", "atm", "Pa", "kPa", "MPa", "mmHg"
}

impl Default for AddInUnits {
    fn default() -> Self {
        Self {
            temp_unit: "C".to_string(),
            energy_unit: "MJ".to_string(),
            pressure_unit: "bar".to_string(),
        }
    }
}

impl AddInUnits {
    pub fn temp_to_kelvin(&self, t: f64) -> f64 {
        match self.temp_unit.trim().to_ascii_uppercase().as_str() {
            "K" => t,
            "F" | "°F" => (t - 32.0) * (5.0 / 9.0) + 273.15,
            _ => t + 273.15, // "C" default
        }
    }

    pub fn kelvin_to_temp(&self, t_k: f64) -> f64 {
        match self.temp_unit.trim().to_ascii_uppercase().as_str() {
            "K" => t_k,
            "F" | "°F" => (t_k - 273.15) * 1.8 + 32.0,
            _ => t_k - 273.15,
        }
    }

    /// Factor converting internal kJ/mol (= MJ/kmol) to `energy_unit`/kmol.
    pub fn energy_from_mj_kmol_factor(&self) -> f64 {
        match self.energy_unit.trim().to_ascii_lowercase().as_str() {
            "kj" => 1000.0,
            "j" => 1_000_000.0,
            "mcal" => 1.0 / CAL_TO_JOULE,
            "kcal" => 1000.0 / CAL_TO_JOULE,
            "cal" => 1_000_000.0 / CAL_TO_JOULE,
            "kwh" => 1.0 / 3.6,
            _ => 1.0, // "MJ" default in AddInSample.xls
        }
    }

    /// Factor converting internal J/(mol·K) (= MJ/(Mmol·K)) to active entropy/Cp unit.
    pub fn entropy_from_j_molk_factor(&self) -> f64 {
        self.energy_from_mj_kmol_factor()
    }

    pub fn pressure_to_bar(&self, p: f64) -> f64 {
        match self.pressure_unit.trim().to_ascii_lowercase().as_str() {
            "atm" => p * 1.01325,
            "pa" => p * 1e-5,
            "kpa" => p * 1e-2,
            "mpa" => p * 10.0,
            "mmhg" | "torr" => p * 1.01325 / 760.0,
            _ => p, // "bar"
        }
    }
}

/// Origin / availability badge for catalogued functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddInOrigin {
    Hsc6Xll,      // 69 official functions registered by HSC6.XLL
    Hsc10Xll,     // 3 unit-query functions added in HSC10-Excel.xll (Table 1)
    MineralAddIn, // 11 mineral functions (HSC 8-10 Section 30.5 & Fig. 7)
    HscSimOnly,   // 5 mineral/particle flowsheet functions (Chapter 27.5 / 57)
}

impl AddInOrigin {
    pub fn label(self) -> &'static str {
        match self {
            Self::Hsc6Xll => "HSC6.XLL",
            Self::Hsc10Xll => "HSC10.XLL",
            Self::MineralAddIn => "Mineral Add-In",
            Self::HscSimOnly => "HSC-Sim Only",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddInFunctionMeta {
    pub index: usize,
    pub name: &'static str,
    pub xll_sig: &'static str,
    pub c_export: &'static str,
    pub group: &'static str,
    pub syntax: &'static str,
    pub description: &'static str,
    pub example: &'static str,
    pub origin: AddInOrigin,
}

macro_rules! fn_meta {
    ($idx:expr, $name:expr, $sig:expr, $cexp:expr, $grp:expr, $syn:expr, $desc:expr, $ex:expr, $orig:expr) => {
        AddInFunctionMeta {
            index: $idx,
            name: $name,
            xll_sig: $sig,
            c_export: $cexp,
            group: $grp,
            syntax: $syn,
            description: $desc,
            example: $ex,
            origin: $orig,
        }
    };
}

/// Complete catalog: 69 HSC6.XLL + 3 HSC10.XLL + 11 Mineralogy + 5 HSC-Sim = 88 functions.
pub static ADDIN_FUNCTION_CATALOG: &[AddInFunctionMeta] = &[
    // 1..3 General
    fn_meta!(1, "Species", "RIJ", "MySPECIES", "General", "SPECIES(DBNo;Position)", "Species formula in given database (1=Own,2=Main) and position", "=SPECIES(2;200)", AddInOrigin::Hsc6Xll),
    fn_meta!(2, "BAL", "RCR", "MyBAL", "General", "BAL(Equation;[Multiplier])", "Balanced reaction equation, optionally scaled by smallest coefficient multiplier", "=BAL(\"C+O2(g)=CO(g)\";0.5)", AddInOrigin::Hsc6Xll),
    fn_meta!(3, "Units", "RCC!", "MyUnits", "General", "UNITS(T;E)", "Changes active temperature (C/K) and energy (MJ/kJ/kcal/Mcal/kWh) units", "=UNITS(\"C\";\"MJ\")", AddInOrigin::Hsc6Xll),
    // 4..12 Species & Reaction
    fn_meta!(4, "H", "RCR", "MyH", "Species / Reaction", "H(SpeciesOrEq;T)", "Enthalpy (per kmol) of species or reaction equation at T", "=H(\"CO2(g)\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(5, "HCM", "RCR", "MyHCM", "Species / Reaction", "HCM(Species;T)", "Enthalpy (per Nm3) of species at given temperature", "=HCM(\"N2(g)\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(6, "HNM3", "RCR", "MyHNM3", "Species / Reaction", "HNM3(Species;T)", "Enthalpy (per Nm3) of species at given temperature", "=HNM3(\"N2(g)\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(7, "HKG", "RCR", "MyHKG", "Species / Reaction", "HKG(SpeciesOrEq;T)", "Enthalpy (per kg) of species or reaction equation at T", "=HKG(\"FeS\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(8, "S", "RCR", "MyS", "Species / Reaction", "S(SpeciesOrEq;T)", "Entropy (per Mmol equivalent) at given temperature", "=S(\"H2O(l)\";25)", AddInOrigin::Hsc6Xll),
    fn_meta!(9, "G", "RCR", "MyG", "Species / Reaction", "G(SpeciesOrEq;T)", "Gibbs energy (per kmol) of species or reaction equation at T", "=G(\"He(g)\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(10, "K", "RCR", "MyK", "Species / Reaction", "K(Equation;T)", "Equilibrium constant K of reaction equation at T", "=K(\"2H2(g)+O2(g)=2H2O(g)\";100)", AddInOrigin::Hsc6Xll),
    fn_meta!(11, "HLAT", "RCR", "MyHLAT", "Species / Reaction", "HLAT(Species;T)", "Sensible enthalpy excluding phase transformations from 25 C", "=HLAT(\"H2O(l)\";25)", AddInOrigin::Hsc6Xll),
    fn_meta!(12, "CP", "RCR", "MyCP", "Species / Reaction", "CP(SpeciesOrEq;T)", "Heat capacity (per Mmol equivalent) at given temperature", "=CP(\"CH4(g)\";100)", AddInOrigin::Hsc6Xll),
    // 13..20 Reverse Iteration (Species)
    fn_meta!(13, "TatH", "RCR", "MyTatH", "Iteration (Reverse)", "TATH(Species;H)", "Temperature of species at given enthalpy (per kmol)", "=TATH(\"CO2(g)\";-390.59)", AddInOrigin::Hsc6Xll),
    fn_meta!(14, "TatHNM3", "RCR", "MyTatHNM3", "Iteration (Reverse)", "TATHNM3(Species;H)", "Temperature of species at given enthalpy (per Nm3)", "=TATHNM3(\"N2(g)\";0.0959)", AddInOrigin::Hsc6Xll),
    fn_meta!(15, "TatHCM", "RCR", "MyTatHCM", "Iteration (Reverse)", "TATHCM(Species;H)", "Temperature of species at given enthalpy (per Nm3)", "=TATHCM(\"N2(g)\";0.0959)", AddInOrigin::Hsc6Xll),
    fn_meta!(16, "TatHKG", "RCR", "MyTatHKG", "Iteration (Reverse)", "TATHKG(Species;H)", "Temperature of species at given enthalpy (per kg)", "=TATHKG(\"FeS\";-1.1043)", AddInOrigin::Hsc6Xll),
    fn_meta!(17, "TatHLAT", "RCR", "MyTatHLAT", "Iteration (Reverse)", "TATHLAT(Species;H)", "Temperature of species at given enthalpy excluding phase transformations", "=TATHLAT(\"H2O(l)\";0)", AddInOrigin::Hsc6Xll),
    fn_meta!(18, "TatS", "RCR", "MyTatS", "Iteration (Reverse)", "TATS(Species;S)", "Temperature of species at given entropy", "=TATS(\"H2O(l)\";70)", AddInOrigin::Hsc6Xll),
    fn_meta!(19, "TatCP", "RCR", "MyTatCP", "Iteration (Reverse)", "TATCP(Species;CP)", "Temperature of species at given heat capacity", "=TATCP(\"CH4(g)\";39)", AddInOrigin::Hsc6Xll),
    fn_meta!(20, "TatG", "RCR", "MyTatG", "Iteration (Reverse)", "TATG(Species;G)", "Temperature of species at given Gibbs energy", "=TATG(\"He(g)\";-47.25)", AddInOrigin::Hsc6Xll),
    // 21..40 Temp. independent & Cp
    fn_meta!(21, "MW", "RC", "MyMW", "Temp. Independent", "MW(Species)", "Molecular weight of given species [kg/kmol]", "=MW(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(22, "DE", "RC", "MyDE", "Temp. Independent", "DE(Species)", "Density of given species [kg/l]", "=DE(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(23, "REF", "RC", "MyREF", "Temp. Independent", "REF(Species)", "Literature reference of given species", "=REF(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(24, "BP", "RC", "MyBP", "Temp. Independent", "BP(Species)", "Boiling point of given species [active T unit]", "=BP(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(25, "MP", "RC", "MyMP", "Temp. Independent", "MP(Species)", "Melting point of given species [active T unit]", "=MP(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(26, "Struct", "RC", "MyStruct", "Temp. Independent", "STRUCT(Species)", "Structural formula of given species", "=STRUCT(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(27, "ChName", "RC", "MyChName", "Temp. Independent", "CHNAME(Species)", "Chemical name of given species", "=CHNAME(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(28, "CoName", "RC", "MyCoName", "Temp. Independent", "CONAME(Species)", "Common name of given species", "=CONAME(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(29, "CAN", "RC", "MyCAN", "Temp. Independent", "CAN(Species)", "CAS / Chemical Abstract Number of given species", "=CAN(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(30, "Phase", "RC", "MyPhase", "Temp. Independent", "PHASE(Species)", "Phase type (s, l, g, a, ia) of given species", "=PHASE(\"C\")", AddInOrigin::Hsc6Xll),
    fn_meta!(31, "RGBColor", "RC", "MyRGBColor", "Temp. Independent", "RGBCOLOR(Species)", "RGB color code integer of given species", "=RGBCOLOR(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(32, "Solu", "RC", "MySolu", "Temp. Independent", "SOLU(Species)", "Aqueous solubility metadata string (g/100ml)", "=SOLU(\"NaCl\")", AddInOrigin::Hsc6Xll),
    fn_meta!(33, "CPA", "RC", "MyCPA", "Cp-Function", "CPA(Species)", "A coefficient in Kelley Cp-function", "=CPA(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(34, "CPB", "RC", "MyCPB", "Cp-Function", "CPB(Species)", "B coefficient in Kelley Cp-function", "=CPB(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(35, "CPC", "RC", "MyCPC", "Cp-Function", "CPC(Species)", "C coefficient in Kelley Cp-function", "=CPC(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(36, "CPD", "RC", "MyCPD", "Cp-Function", "CPD(Species)", "D coefficient in Kelley Cp-function", "=CPD(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(37, "CPFUNCTION", "RC", "MyCPFUNCTION", "Cp-Function", "CPFUNCTION(Species)", "Formatted polynomial expression A + B*10^(-3)*T + ...", "=CPFUNCTION(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(38, "TMin", "RC", "MyTMin", "Cp-Function", "TMIN(Species)", "Lower temperature limit of Cp-function [K]", "=TMIN(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(39, "TMax", "RC", "MyTMax", "Cp-Function", "TMAX(Species)", "Upper temperature limit of Cp-function [K]", "=TMAX(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    fn_meta!(40, "Rel", "RC", "MyRel", "Temp. Independent", "REL(Species)", "Reliability class (1..4) of species data", "=REL(\"H2O(l)\")", AddInOrigin::Hsc6Xll),
    // 41..42 Percentage
    fn_meta!(41, "MolP", "RCC", "MyMolP", "Percentage", "MOLP(Species1;Species2)", "Species 1 content in Species 2 in mol-%", "=MOLP(\"ZnO\";\"ZnO*Fe2O3\")", AddInOrigin::Hsc6Xll),
    fn_meta!(42, "WTP", "RCC", "MyWTP", "Percentage", "WTP(Species1;Species2)", "Species 1 content in Species 2 in wt-%", "=WTP(\"ZnO\";\"ZnO*Fe2O3\")", AddInOrigin::Hsc6Xll),
    // 43..57 Stream/Flow + DensityA
    fn_meta!(43, "FlowH", "RRRR", "MyFlowH", "Stream / Flow", "FLOWH(Species;Amount;T)", "Total enthalpy of flow (kmol) at T", "=FLOWH(D34:D36;E34:E36;F34)", AddInOrigin::Hsc6Xll),
    fn_meta!(44, "FlowHLAT", "RRRR", "MyFlowHLAT", "Stream / Flow", "FLOWHLAT(Species;Amount;T)", "Sensible enthalpy excluding phase transitions of flow", "=FLOWHLAT(D43:D45;E43:E45;F43)", AddInOrigin::Hsc6Xll),
    fn_meta!(45, "FlowHKG", "RRRR", "MyFlowHKG", "Stream / Flow", "FLOWHKG(Species;Amount;T)", "Total enthalpy of flow (kg) at T", "=FLOWHKG(D37:D39;E37:E39;F37)", AddInOrigin::Hsc6Xll),
    fn_meta!(46, "FlowHNM3", "RRRR", "MyFlowHNM3", "Stream / Flow", "FLOWHNM3(Species;Amount;T)", "Total enthalpy of flow (Nm3) at T", "=FLOWHNM3(D40:D42;E40:E42;F40)", AddInOrigin::Hsc6Xll),
    fn_meta!(47, "FlowS", "RRRR", "MyFlowS", "Stream / Flow", "FLOWS(Species;Amount;T)", "Total entropy of flow at T", "=FLOWS(D46:D48;E46:E48;F46)", AddInOrigin::Hsc6Xll),
    fn_meta!(48, "FlowCP", "RRRR", "MyFlowCP", "Stream / Flow", "FLOWCP(Species;Amount;T)", "Total heat capacity of flow at T", "=FLOWCP(D49:D51;E49:E51;F49)", AddInOrigin::Hsc6Xll),
    fn_meta!(49, "DensityA", "RRRR", "MyDensityA", "Stream / Flow", "DENSITYA(Species;WtFrac;T)", "Density of aqueous electrolyte solution [kg/m3]", "=DENSITYA(D55:D56;E55:E56;F55)", AddInOrigin::Hsc6Xll),
    fn_meta!(50, "FlowG", "RRRR", "MyFlowG", "Stream / Flow", "FLOWG(Species;Amount;T)", "Total Gibbs energy of flow (kmol) at T", "=FLOWG(D52:D54;E52:E54;F52)", AddInOrigin::Hsc6Xll),
    fn_meta!(51, "StreamH", "RRRR", "MyStreamH", "Stream / Flow", "STREAMH(Species;Amount;T)", "Total enthalpy of stream (kmol) at T", "=STREAMH(D34:D36;E34:E36;F34)", AddInOrigin::Hsc6Xll),
    fn_meta!(52, "StreamHLAT", "RRRR", "MyStreamHLAT", "Stream / Flow", "STREAMHLAT(Species;Amount;T)", "Sensible enthalpy excluding phase transitions of stream", "=STREAMHLAT(D43:D45;E43:E45;F43)", AddInOrigin::Hsc6Xll),
    fn_meta!(53, "StreamHKG", "RRRR", "MyStreamHKG", "Stream / Flow", "STREAMHKG(Species;Amount;T)", "Total enthalpy of stream (kg) at T", "=STREAMHKG(D37:D39;E37:E39;F37)", AddInOrigin::Hsc6Xll),
    fn_meta!(54, "StreamHNM3", "RRRR", "MyStreamHNM3", "Stream / Flow", "STREAMHNM3(Species;Amount;T)", "Total enthalpy of stream (Nm3) at T", "=STREAMHNM3(D40:D42;E40:E42;F40)", AddInOrigin::Hsc6Xll),
    fn_meta!(55, "StreamS", "RRRR", "MyStreamS", "Stream / Flow", "STREAMS(Species;Amount;T)", "Total entropy of stream at T", "=STREAMS(D46:D48;E46:E48;F46)", AddInOrigin::Hsc6Xll),
    fn_meta!(56, "StreamCP", "RRRR", "MyStreamCP", "Stream / Flow", "STREAMCP(Species;Amount;T)", "Total heat capacity of stream at T", "=STREAMCP(D49:D51;E49:E51;F49)", AddInOrigin::Hsc6Xll),
    fn_meta!(57, "StreamG", "RRRR", "MyStreamG", "Stream / Flow", "STREAMG(Species;Amount;T)", "Total Gibbs energy of stream (kmol) at T", "=STREAMG(D52:D54;E52:E54;F52)", AddInOrigin::Hsc6Xll),
    // 58..68 Stream/Flow Reverse Iteration
    fn_meta!(58, "StreamTH", "RRRRRR", "MyStreamTH", "Stream Reverse", "STREAMTH(Species;Amount;H;Tmin;Tmax)", "Temperature of stream (kmol) at given H in [Tmin, Tmax]", "=STREAMTH(D59:D61;E59:E61;F59;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(59, "StreamTHKG", "RRRRRR", "MyStreamTHKG", "Stream Reverse", "STREAMTHKG(Species;Amount;H;Tmin;Tmax)", "Temperature of stream (kg) at given H in [Tmin, Tmax]", "=STREAMTHKG(D62:D64;E62:E64;F62;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(60, "StreamTHLAT", "RRRRRR", "MyStreamTHLAT", "Stream Reverse", "STREAMTHLAT(Species;Amount;H;Tmin;Tmax)", "Temperature of stream (kmol) at given latent-free H", "=STREAMTHLAT(D68:D70;E68:E70;F68;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(61, "StreamTHNM3", "RRRRRR", "MyStreamTHNM3", "Stream Reverse", "STREAMTHNM3(Species;Amount;H;Tmin;Tmax)", "Temperature of stream (Nm3) at given H in [Tmin, Tmax]", "=STREAMTHNM3(D65:D67;E65:E67;F65;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(62, "StreamTS", "RRRRRR", "MyStreamTS", "Stream Reverse", "STREAMTS(Species;Amount;S;Tmin;Tmax)", "Temperature of stream at given entropy S in [Tmin, Tmax]", "=STREAMTS(D71:D73;E71:E73;F71;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(63, "StreamTCP", "RRRRRR", "MyStreamTCP", "Stream Reverse", "STREAMTCP(Species;Amount;CP;Tmin;Tmax)", "Temperature of stream at given Cp in [Tmin, Tmax]", "=STREAMTCP(D74:D76;E74:E76;F74;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(64, "StreamTG", "RRRRRR", "MyStreamTG", "Stream Reverse", "STREAMTG(Species;Amount;G;Tmin;Tmax)", "Temperature of stream (kmol) at given G in [Tmin, Tmax]", "=STREAMTG(D77:D79;E77:E79;F77;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(65, "FlowTH", "RRRRRR", "MyFlowTH", "Stream Reverse", "FLOWTH(Species;Amount;H;Tmin;Tmax)", "Temperature of flow at given H in [Tmin, Tmax]", "=FLOWTH(D59:D61;E59:E61;F59;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(66, "FlowTS", "RRRRRR", "MyFlowTS", "Stream Reverse", "FLOWTS(Species;Amount;S;Tmin;Tmax)", "Temperature of flow at given S in [Tmin, Tmax]", "=FLOWTS(D71:D73;E71:E73;F71;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(67, "FlowTCP", "RRRRRR", "MyFlowTCP", "Stream Reverse", "FLOWTCP(Species;Amount;CP;Tmin;Tmax)", "Temperature of flow at given Cp in [Tmin, Tmax]", "=FLOWTCP(D74:D76;E74:E76;F74;0;1000)", AddInOrigin::Hsc6Xll),
    fn_meta!(68, "FlowTG", "RRRRRR", "MyFlowTG", "Stream Reverse", "FLOWTG(Species;Amount;G;Tmin;Tmax)", "Temperature of flow at given G in [Tmin, Tmax]", "=FLOWTG(D77:D79;E77:E79;F77;0;1000)", AddInOrigin::Hsc6Xll),
    // 69 Equilibrium Array
    fn_meta!(69, "StreamEQ", "RRRRR", "MyStreamEQ", "Equilibrium Array", "{=StreamEQ(SpeciesRange;AmountRange;T;P)}", "Multiphase Gibbs equilibrium vector across §PHASE blocks", "{=StreamEQ(A4:A15;B4:B15;C1;C2)}", AddInOrigin::Hsc6Xll),
    // 70..80 Mineralogy Add-In (HSC 8-10 Section 30.5 & Fig. 7)
    fn_meta!(70, "MineralCode", "XLL", "MineralCode", "Mineralogy", "MineralCode(MineralNameOrID)", "Standard HSC mineral code (1-4 letters)", "=MineralCode(\"Pyrite\")", AddInOrigin::MineralAddIn),
    fn_meta!(71, "MineralFormula", "XLL", "MineralFormula", "Mineralogy", "MineralFormula(MineralNameOrCode)", "General chemical formula of mineral", "=MineralFormula(\"Galena\")", AddInOrigin::MineralAddIn),
    fn_meta!(72, "MineralName", "XLL", "MineralName", "Mineralogy", "MineralName(CodeOrID)", "Full mineral name matching mineral code/ID/location", "=MineralName(\"Py\")", AddInOrigin::MineralAddIn),
    fn_meta!(73, "MineralC", "XLL", "MineralC", "Mineralogy", "MineralC(Mineral;Element)", "Elemental wt-% in mineral (e.g. Fe, S, Cu, Al)", "=MineralC(\"Pyrite\";\"S\")", AddInOrigin::MineralAddIn),
    fn_meta!(74, "MineralProperty", "XLL", "MineralProperty", "Mineralogy", "MineralProperty(Mineral;Property)", "Mineral property (Code,Formula,Name,Group,Density,Hardness,Color,Luster,Location,ID,Elements,Database,Reference, or Element)", "=MineralProperty(\"Pyrite\";\"Density\")", AddInOrigin::MineralAddIn),
    fn_meta!(75, "MineralColor", "XLL", "MineralColor", "Mineralogy", "MineralColor(Mineral)", "Visual color description of given mineral", "=MineralColor(\"Pyrite\")", AddInOrigin::MineralAddIn),
    fn_meta!(76, "MineralDensity", "XLL", "MineralDensity", "Mineralogy", "MineralDensity(Mineral)", "Bulk density [g/cm3] of given mineral", "=MineralDensity(\"Pyrite\")", AddInOrigin::MineralAddIn),
    fn_meta!(77, "MineralElementWt", "XLL", "MineralElementWt", "Mineralogy", "MineralElementWt(Mineral;Element)", "Weight-% of element in given mineral (HSC10 alias of MineralC)", "=MineralElementWt(\"Pyrite\";\"S\")", AddInOrigin::MineralAddIn),
    fn_meta!(78, "MineralHardness", "XLL", "MineralHardness", "Mineralogy", "MineralHardness(Mineral)", "Mohs hardness of given mineral", "=MineralHardness(\"Pyrite\")", AddInOrigin::MineralAddIn),
    fn_meta!(79, "MineralLuster", "XLL", "MineralLuster", "Mineralogy", "MineralLuster(Mineral)", "Optical luster description of given mineral", "=MineralLuster(\"Pyrite\")", AddInOrigin::MineralAddIn),
    fn_meta!(80, "MineralSG", "XLL", "MineralSG", "Mineralogy", "MineralSG(Mineral)", "Specific gravity (dimensionless relative to H2O) of given mineral", "=MineralSG(\"Pyrite\")", AddInOrigin::MineralAddIn),
    // 81..83 HSC10 Table 1 Active Unit Queries
    fn_meta!(81, "EnergyU", "R!", "MyEnergyU", "General", "EnergyU()", "Returns active measurement energy unit (MJ, kJ, kcal, Mcal, kWh, J)", "=EnergyU()", AddInOrigin::Hsc10Xll),
    fn_meta!(82, "TempU", "R!", "MyTempU", "General", "TempU()", "Returns active measurement temperature unit (C, K, F)", "=TempU()", AddInOrigin::Hsc10Xll),
    fn_meta!(83, "PresU", "R!", "MyPresU", "General", "PresU()", "Returns active measurement pressure unit (bar, atm, Pa, kPa, MPa, mmHg)", "=PresU()", AddInOrigin::Hsc10Xll),
    // 84..88 HSC-Sim Only (Chapter 27.5 / 57)
    fn_meta!(84, "StreamX", "SIM", "StreamX", "HSC-Sim Mineral", "StreamX(StreamName;ParticlesRange;Variable)", "Assay variable of a particle stream in HSC-Sim", "=StreamX(\"ROM\";$E$56:$E$97;\"SiO2\")", AddInOrigin::HscSimOnly),
    fn_meta!(85, "RecoveryX", "SIM", "RecoveryX", "HSC-Sim Mineral", "RecoveryX(Variable;StreamName;FeedName)", "Element/mineral recovery (%) between stream and feed", "=RecoveryX(\"Fe\";\"Zn Conc\";\"ROM\")", AddInOrigin::HscSimOnly),
    fn_meta!(86, "ParticleRecX", "SIM", "ParticleRecX", "HSC-Sim Mineral", "ParticleRecX(ParticleID;ModelRef;VarsTable)", "Particle recovery in mineral flotation/gravity model", "=ParticleRecX(A56;1;Model!B22:F34)", AddInOrigin::HscSimOnly),
    fn_meta!(87, "FractionX", "SIM", "FractionX", "HSC-Sim Mineral", "FractionX(FractionNo;Variable)", "Size-fraction property inside HSC-Sim", "=FractionX(1;\"Label\")", AddInOrigin::HscSimOnly),
    fn_meta!(88, "MineralX", "SIM", "MineralX", "HSC-Sim Mineral", "MineralX(MineralNo;Variable)", "Mineral-level stream property in HSC-Sim", "=MineralX(1;\"Cu\")", AddInOrigin::HscSimOnly),
];

/// Typed cell evaluation result compatible with Excel error conventions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellEvalValue {
    Number(f64),
    Text(String),
    Error(String),
    Array(Vec<CellEvalValue>),
}

impl CellEvalValue {
    pub fn as_display_string(&self, decimals: usize) -> String {
        match self {
            Self::Number(v) => format_cell_number(*v, decimals),
            Self::Text(s) => s.clone(),
            Self::Error(e) => e.clone(),
            Self::Array(arr) => arr
                .first()
                .map(|v| v.as_display_string(decimals))
                .unwrap_or_default(),
        }
    }

    pub fn as_multiline_string(&self, decimals: usize) -> String {
        match self {
            Self::Array(arr) => arr
                .iter()
                .map(|v| v.as_display_string(decimals))
                .collect::<Vec<_>>()
                .join("\n"),
            other => other.as_display_string(decimals),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(v) if v.is_finite() => Some(*v),
            Self::Text(s) => s.trim().parse::<f64>().ok(),
            _ => None,
        }
    }
}

pub fn format_cell_number(v: f64, decimals: usize) -> String {
    if !v.is_finite() {
        return "#NUM!".to_string();
    }
    if v != 0.0 && (v.abs() >= 1e10 || v.abs() < 1e-4) {
        format!("{v:.4E}")
    } else if decimals == 8 {
        format!("{v:.8}")
    } else {
        let s = format!("{v:.*}", decimals);
        if s.contains('.') {
            let trimmed = s.trim_end_matches('0').trim_end_matches('.');
            if trimmed == "-0" { "0".to_string() } else { trimmed.to_string() }
        } else {
            s
        }
    }
}

/// Resolved mineral record combining `Minerals.txt` (3,812 records) + Fig. 7 physical/locality attributes.
#[derive(Debug, Clone)]
pub struct MineralRecord {
    pub id: u32,
    pub name: String,
    pub code: String,
    pub formula: String,
    pub group: String,
    pub elements_str: String,
    pub density: Option<f64>,
    pub hardness: Option<f64>,
    pub color: String,
    pub luster: String,
    pub location: String,
    pub database: String,
    pub reference: String,
    pub comp_wt_pct: BTreeMap<String, f64>,
}

fn parse_comp_string(raw: &str) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for part in raw.split(';') {
        let part = part.trim();
        if let Some((el, val)) = part.split_once('=') {
            if let Ok(pct) = val.trim().parse::<f64>() {
                out.insert(el.trim().to_string(), pct);
            }
        }
    }
    out
}

fn physical_defaults_for_mineral(name_or_code: &str) -> (Option<f64>, Option<f64>, &'static str, &'static str) {
    match name_or_code.trim().to_ascii_lowercase().as_str() {
        "pyrite" | "py" => (Some(5.01), Some(6.5), "yellow, pale brass", "Metallic"),
        "galena" | "gn" => (Some(7.58), Some(2.5), "lead gray", "Metallic"),
        "chalcopyrite" | "ccp" => (Some(4.19), Some(3.8), "brass yellow", "Metallic"),
        "sphalerite" | "sp" => (Some(4.08), Some(3.8), "brown, black, yellow", "Resinous / Adamantine"),
        "quartz" | "qtz" => (Some(2.65), Some(7.0), "colorless, white", "Vitreous"),
        "k-feldspar" | "kfs" => (Some(2.56), Some(6.0), "pink, white", "Vitreous"),
        "hematite" | "hem" => (Some(5.26), Some(6.0), "steel gray, reddish brown", "Metallic / Earthy"),
        "magnetite" | "mag" => (Some(5.18), Some(6.0), "black", "Metallic"),
        "bornite" | "bn" => (Some(5.07), Some(3.0), "bronze brown, iridescent", "Metallic"),
        "covellite" | "cv" => (Some(4.68), Some(1.8), "indigo blue", "Submetallic"),
        "chalcocite" | "cc" => (Some(5.65), Some(2.8), "dark lead gray", "Metallic"),
        "pyrrhotite" | "po" => (Some(4.61), Some(4.0), "bronze brown", "Metallic"),
        "calcite" | "cal" => (Some(2.71), Some(3.0), "white, colorless", "Vitreous"),
        _ => (None, None, "-", "-"),
    }
}

/// Overlay localities shown in Fig. 7 (`Py\Joki` and `ID:12616` Pyhäsalmi) plus `Minerals.txt`.
pub fn lookup_mineral(query: &str) -> Option<MineralRecord> {
    let q = query.trim();
    if q.is_empty() {
        return None;
    }

    // Specific locality / ID samples documented in Fig. 7 (HSC 8-10 Mineralogy Add-In)
    if q.eq_ignore_ascii_case("Py\\Joki") || q.eq_ignore_ascii_case("Py\\Jokisivu") || q.eq_ignore_ascii_case("ID:5400") {
        let mut comp = BTreeMap::new();
        comp.insert("Cu".into(), 0.01);
        comp.insert("Zn".into(), 0.02);
        comp.insert("Fe".into(), 46.45);
        comp.insert("S".into(), 53.29);
        return Some(MineralRecord {
            id: 5400,
            name: "Pyrite".into(),
            code: "Py".into(),
            formula: "FeS2".into(),
            group: "Pyrite".into(),
            elements_str: "\\Fe\\S\\".into(),
            density: Some(5.01),
            hardness: Some(6.5),
            color: "yellow, pale brass".into(),
            luster: "Metallic".into(),
            location: "Jokisivu".into(),
            database: "HSC".into(),
            reference: "Luukkonen, -94".into(),
            comp_wt_pct: comp,
        });
    }
    if q.eq_ignore_ascii_case("ID:12616") || q.eq_ignore_ascii_case("12616") || q.eq_ignore_ascii_case("Pyhäsalmi") {
        let mut comp = BTreeMap::new();
        comp.insert("Cu".into(), 0.05);
        comp.insert("Zn".into(), 0.00);
        comp.insert("Fe".into(), 46.69);
        comp.insert("S".into(), 53.47);
        return Some(MineralRecord {
            id: 12616,
            name: "Pyrite".into(),
            code: "Py".into(),
            formula: "FeS2".into(),
            group: "Pyrite".into(),
            elements_str: "\\Fe\\S\\".into(),
            density: Some(5.01),
            hardness: Some(6.5),
            color: "yellow, pale brass".into(),
            luster: "Metallic".into(),
            location: "Pyhäsalmi".into(),
            database: "HSC".into(),
            reference: "3595".into(),
            comp_wt_pct: comp,
        });
    }
    if q.eq_ignore_ascii_case("ID:54") {
        return from_raw_mineral(MINERALS_RAW.iter().find(|m| m.code == "Py")?);
    }

    // Handle optional "NameOrCode\Location" syntax
    let (base_q, loc_q) = if let Some((a, b)) = q.split_once('\\') {
        (a.trim(), Some(b.trim()))
    } else {
        (q, None)
    };

    if let Some(loc) = loc_q {
        let bl = base_q.to_ascii_lowercase();
        let ll = loc.to_ascii_lowercase();
        if (bl == "py" || bl == "pyrite") && !ll.is_empty() {
            if "jokisivu".starts_with(&ll) {
                return lookup_mineral("Py\\Joki");
            }
            if "pyhäsalmi".starts_with(&ll) || "pyhasalmi".starts_with(&ll) {
                return lookup_mineral("ID:12616");
            }
        }
    }

    if let Some(id_num) = base_q.strip_prefix("ID:").or_else(|| base_q.strip_prefix("id:")) {
        if let Ok(id_val) = id_num.trim().parse::<u32>() {
            if let Some(m) = MINERALS_RAW.iter().find(|r| r.id == id_val) {
                return from_raw_mineral(m);
            }
        }
    }

    let found = MINERALS_RAW
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(base_q) || m.code.eq_ignore_ascii_case(base_q));
    if let Some(m) = found {
        let mut rec = from_raw_mineral(m)?;
        if let Some(loc) = loc_q {
            if !loc.is_empty() {
                rec.location = loc.to_string();
            }
        }
        return Some(rec);
    }

    // Fallback: partial prefix match
    let partial = MINERALS_RAW.iter().find(|m| {
        m.name.to_ascii_lowercase().starts_with(&base_q.to_ascii_lowercase())
    });
    partial.and_then(from_raw_mineral).map(|mut rec| {
        if let Some(loc) = loc_q {
            if !loc.is_empty() {
                rec.location = loc.to_string();
            }
        }
        rec
    })
}

fn from_raw_mineral(m: &MineralRawRow) -> Option<MineralRecord> {
    let comp = parse_comp_string(m.composition);
    let (den, hard, col, lus) = physical_defaults_for_mineral(m.name);
    let id = if m.code.eq_ignore_ascii_case("Py") { 54 } else { m.id };
    Some(MineralRecord {
        id,
        name: m.name.to_string(),
        code: m.code.to_string(),
        formula: m.formula.to_string(),
        group: m.group.to_string(),
        elements_str: m.elements.to_string(),
        density: den,
        hardness: hard,
        color: col.to_string(),
        luster: lus.to_string(),
        location: "[stoichiometric]".to_string(),
        database: "HSC".to_string(),
        reference: m.group.to_string(),
        comp_wt_pct: comp,
    })
}

#[derive(Debug, Clone)]
pub enum ArgValue {
    Scalar(String),
    Range(Vec<String>),
}

impl ArgValue {
    fn as_scalar_str(&self) -> &str {
        match self {
            Self::Scalar(s) => s.as_str(),
            Self::Range(v) => v.first().map(|s| s.as_str()).unwrap_or(""),
        }
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_scalar_str().trim().parse::<f64>().ok()
    }

    fn as_list(&self) -> Vec<String> {
        match self {
            Self::Scalar(s) => vec![s.clone()],
            Self::Range(v) => v.clone(),
        }
    }
}

fn resolve_species_fallback(catalog: &dyn SpeciesCatalogPort, name: &str) -> Option<ChemicalSpecies> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(sp) = catalog.get_species(trimmed) {
        return Some(sp);
    }
    // Strip spaces & common HSC aliases such as CH4(a) vs CH4(g) if exact absent
    if trimmed.eq_ignore_ascii_case("CH4(a)") {
        if let Some(sp) = catalog.get_species("CH4(g)").or_else(|| catalog.get_species("CH4(aq)")) {
            return Some(sp);
        }
    }
    None
}

fn eval_latent_free_enthalpy_kj(sp: &ChemicalSpecies, t_k: f64) -> Option<f64> {
    if t_k <= 0.0 || sp.intervals.is_empty() {
        return None;
    }
    let idx_ref = sp
        .intervals
        .iter()
        .position(|r| r.t_min <= T_REF_K + 1e-4 && T_REF_K - 1e-4 <= r.t_max)
        .unwrap_or(0);
    let idx_t = sp
        .intervals
        .iter()
        .position(|r| r.t_min - 1e-6 <= t_k && t_k <= r.t_max + 1e-6)
        .unwrap_or(if t_k < sp.intervals[0].t_min { 0 } else { sp.intervals.len() - 1 });

    // Integrate Cp continuously from 298.15 K to t_k skipping ΔH_tr jumps and Hf°(298.15)
    let mut h_sens_j = 0.0;
    if idx_t == idx_ref {
        h_sens_j += sp.intervals[idx_ref].kelley.integrate_h(T_REF_K, t_k);
    } else if idx_t > idx_ref {
        h_sens_j += sp.intervals[idx_ref].kelley.integrate_h(T_REF_K, sp.intervals[idx_ref].t_max);
        for k in (idx_ref + 1)..idx_t {
            h_sens_j += sp.intervals[k].kelley.integrate_h(sp.intervals[k].t_min, sp.intervals[k].t_max);
        }
        h_sens_j += sp.intervals[idx_t].kelley.integrate_h(sp.intervals[idx_t].t_min, t_k);
    } else {
        h_sens_j -= sp.intervals[idx_ref].kelley.integrate_h(sp.intervals[idx_ref].t_min, T_REF_K);
        for k in ((idx_t + 1)..idx_ref).rev() {
            h_sens_j -= sp.intervals[k].kelley.integrate_h(sp.intervals[k].t_min, sp.intervals[k].t_max);
        }
        h_sens_j -= sp.intervals[idx_t].kelley.integrate_h(t_k, sp.intervals[idx_t].t_max);
    }
    Some(h_sens_j / 1000.0)
}

#[derive(Clone, Copy)]
enum PropKind {
    H,
    Hkg,
    Hnm3,
    Hlat,
    S,
    Cp,
    G,
    K,
}

fn eval_target_prop(
    catalog: &dyn SpeciesCatalogPort,
    units: &AddInUnits,
    target: &str,
    t_user: f64,
    kind: PropKind,
) -> Option<f64> {
    let t_k = units.temp_to_kelvin(t_user);
    if t_k <= 0.0 || !t_k.is_finite() {
        return None;
    }
    let e_fac = units.energy_from_mj_kmol_factor();
    let s_fac = units.entropy_from_j_molk_factor();

    if target.contains('=') || target.contains("->") {
        let norm = target.replace("->", "=");
        let parsed = parse_input(&norm).ok()?;
        let mut sum_h_kj = 0.0;
        let mut sum_s_jk = 0.0;
        let mut sum_g_kj = 0.0;
        let mut sum_cp_jk = 0.0;
        let mut reactant_mass_kg = 0.0;
        for term in &parsed.reactants {
            let sp = resolve_species_fallback(catalog, &term.formula)?;
            let (pt, _) = evaluate_species_si(&sp, t_k, true, false).ok()?;
            sum_h_kj -= term.coeff * (pt.h / 1000.0);
            sum_s_jk -= term.coeff * pt.s;
            sum_g_kj -= term.coeff * (pt.g / 1000.0);
            sum_cp_jk -= term.coeff * pt.cp;
            reactant_mass_kg += term.coeff * sp.molecular_weight;
        }
        for term in &parsed.products {
            let sp = resolve_species_fallback(catalog, &term.formula)?;
            let (pt, _) = evaluate_species_si(&sp, t_k, true, false).ok()?;
            sum_h_kj += term.coeff * (pt.h / 1000.0);
            sum_s_jk += term.coeff * pt.s;
            sum_g_kj += term.coeff * (pt.g / 1000.0);
            sum_cp_jk += term.coeff * pt.cp;
        }
        return match kind {
            PropKind::H => Some(sum_h_kj * e_fac),
            PropKind::Hkg => (reactant_mass_kg > 1e-12).then_some((sum_h_kj * e_fac) / reactant_mass_kg),
            PropKind::Hnm3 => Some((sum_h_kj * e_fac) / NORMAL_MOLAR_VOL_NM3_PER_KMOL),
            PropKind::Hlat => Some(sum_h_kj * e_fac),
            PropKind::S => Some(sum_s_jk * s_fac),
            PropKind::Cp => Some(sum_cp_jk * s_fac),
            PropKind::G => Some(sum_g_kj * e_fac),
            PropKind::K => {
                let dg_j = sum_g_kj * 1000.0;
                let exponent = -dg_j / (R_JOULE * t_k);
                if exponent > 709.0 {
                    Some(f64::INFINITY)
                } else {
                    Some(exponent.exp())
                }
            }
        };
    }

    let sp = resolve_species_fallback(catalog, target)?;
    let (pt, _) = evaluate_species_si(&sp, t_k, true, false).ok()?;
    match kind {
        PropKind::H => Some((pt.h / 1000.0) * e_fac),
        PropKind::Hkg => (sp.molecular_weight > 1e-12).then_some(((pt.h / 1000.0) * e_fac) / sp.molecular_weight),
        PropKind::Hnm3 => Some(((pt.h / 1000.0) * e_fac) / NORMAL_MOLAR_VOL_NM3_PER_KMOL),
        PropKind::Hlat => eval_latent_free_enthalpy_kj(&sp, t_k).map(|v| v * e_fac),
        PropKind::S => Some(pt.s * s_fac),
        PropKind::Cp => Some(pt.cp * s_fac),
        PropKind::G => Some((pt.g / 1000.0) * e_fac),
        PropKind::K => None,
    }
}

fn invert_monotone_temp<F>(target: f64, t_min_u: f64, t_max_u: f64, units: &AddInUnits, eval: F) -> Option<f64>
where
    F: Fn(f64) -> Option<f64>,
{
    let t_low = t_min_u.max(units.kelvin_to_temp(2.0));
    let t_high = t_max_u;
    if t_high <= t_low {
        return None;
    }
    let steps = 120usize;
    let dt = (t_high - t_low) / (steps as f64);
    let mut prev_t = t_low;
    let mut prev_f = eval(prev_t)? - target;
    if prev_f.abs() < 1e-9 {
        return Some(prev_t);
    }

    for i in 1..=steps {
        let cur_t = t_low + (i as f64) * dt;
        let cur_f = eval(cur_t)? - target;
        if cur_f.abs() < 1e-9 {
            return Some(cur_t);
        }
        if prev_f * cur_f <= 0.0 {
            let mut a = prev_t;
            let mut b = cur_t;
            let mut fa = prev_f;
            for _ in 0..64 {
                let mid = 0.5 * (a + b);
                let fm = eval(mid)? - target;
                if fm.abs() < 1e-11 || (b - a).abs() < 1e-8 {
                    return Some(mid);
                }
                if fa * fm <= 0.0 {
                    b = mid;
                } else {
                    a = mid;
                    fa = fm;
                }
            }
            return Some(0.5 * (a + b));
        }
        prev_t = cur_t;
        prev_f = cur_f;
    }
    None
}

fn eval_aqueous_density_kg_m3(species: &[String], wt_fracs: &[String], t_c: f64) -> CellEvalValue {
    let mut rows = Vec::with_capacity(species.len().min(wt_fracs.len()));
    let mut w_sum = 0.0;
    for (sp, wf_s) in species.iter().zip(wt_fracs.iter()) {
        let w = wf_s.trim().parse::<f64>().unwrap_or(0.0);
        if w < 0.0 || !w.is_finite() {
            return CellEvalValue::Error("#VALUE!".into());
        }
        w_sum += w;
        rows.push(crate::aqua::DensityInputRow {
            component: sp.trim().to_string(),
            mass_fraction: w,
        });
    }
    if w_sum > 1.0 + 1e-9 {
        return CellEvalValue::Error("#NUM!".into());
    }
    let res = crate::aqua::calculate_solution_density(&rows, t_c);
    if res.global_error.is_some() || !res.solution_density_kg_m3.is_finite() {
        return CellEvalValue::Error("#NUM!".into());
    }
    CellEvalValue::Number(res.solution_density_kg_m3)
}

fn eval_composition_pct(species1: &str, species2: &str, wt_mode: bool) -> CellEvalValue {
    // Support composite HSC oxide strings like "ZnO*Fe2O3" by replacing '*' with concatenated formula tokens
    let clean1 = species1.replace('*', "");
    let clean2 = species2.replace('*', "");
    let Ok(c1) = parse_formula_composition(&clean1) else {
        return CellEvalValue::Error("#VALUE!".into());
    };
    let Ok(c2) = parse_formula_composition(&clean2) else {
        return CellEvalValue::Error("#VALUE!".into());
    };
    if c1.is_empty() || c2.is_empty() {
        return CellEvalValue::Error("#VALUE!".into());
    }
    // Limiting molar ratio k of species1 contained inside 1 mol of species2
    let mut k = f64::INFINITY;
    for (el, &cnt1) in &c1 {
        let cnt2 = c2.get(el).copied().unwrap_or(0.0);
        if cnt1 > 0.0 {
            k = k.min(cnt2 / cnt1);
        }
    }
    if !k.is_finite() || k <= 0.0 {
        return CellEvalValue::Number(0.0);
    }
    let atoms1: f64 = c1.values().sum();
    let atoms2: f64 = c2.values().sum();
    if wt_mode {
        let mw1: f64 = c1.iter().map(|(el, &n)| crate::estimate::atomic_weight(el) * n).sum();
        let mw2: f64 = c2.iter().map(|(el, &n)| crate::estimate::atomic_weight(el) * n).sum();
        if mw2 <= 1e-12 {
            return CellEvalValue::Error("#DIV/0!".into());
        }
        CellEvalValue::Number(100.0 * (k * mw1) / mw2)
    } else {
        CellEvalValue::Number(100.0 * (k * atoms1) / atoms2.max(1e-12))
    }
}

/// Evaluates `StreamEQ(SpeciesRange, InputAmountRange, T, P)` reusing `solve_single_point`.
pub fn eval_streameq(
    catalog: &dyn SpeciesCatalogPort,
    units: &AddInUnits,
    species_rows: &[String],
    input_rows: &[String],
    t_user: f64,
    p_user: f64,
) -> CellEvalValue {
    if species_rows.len() != input_rows.len()
        || species_rows.is_empty()
        || !t_user.is_finite()
        || !p_user.is_finite()
        || p_user <= 0.0
    {
        return CellEvalValue::Error("#VALUE!".into());
    }
    let t_k = units.temp_to_kelvin(t_user);
    if !t_k.is_finite() || t_k <= 0.0 {
        return CellEvalValue::Error("#VALUE!".into());
    }
    let p_bar = units.pressure_to_bar(p_user).max(1e-6);

    let mut phases: Vec<EquilibriumPhase> = Vec::new();
    let mut phase_is_pure: Vec<bool> = Vec::new();
    let mut row_map: Vec<Option<(usize, usize)>> = vec![None; species_rows.len()];

    for (idx, sp_raw) in species_rows.iter().enumerate() {
        let trimmed = sp_raw.trim();
        if trimmed.starts_with('§') || trimmed.to_ascii_uppercase().starts_with("PHASE") {
            let is_pure_header = trimmed.starts_with("§§");
            let clean_name = trimmed.trim_start_matches('§').trim().to_string();
            let is_gas = !is_pure_header && (phases.is_empty() || clean_name.to_ascii_uppercase().contains("GAS"));
            phase_is_pure.push(is_pure_header);
            phases.push(EquilibriumPhase {
                name: clean_name,
                model: if is_pure_header {
                    SolutionModel::PureSubstance
                } else {
                    SolutionModel::IdealMixture
                },
                is_gas_phase: is_gas,
                species: Vec::new(),
                electrode_role: String::new(),
                capacitance_farads: 0.0,
            });
            continue;
        }
        if trimmed.is_empty() {
            continue;
        }
        if resolve_species_fallback(catalog, trimmed).is_none() {
            return CellEvalValue::Error("#NAME?".into());
        }
        if phases.is_empty() {
            phase_is_pure.push(false);
            phases.push(EquilibriumPhase {
                name: "PHASE 1".into(),
                model: SolutionModel::IdealMixture,
                is_gas_phase: true,
                species: Vec::new(),
                electrode_role: String::new(),
                capacitance_farads: 0.0,
            });
        }
        let amount = input_rows[idx].trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let p_idx = phases.len() - 1;
        let sp_idx = phases[p_idx].species.len();
        phases[p_idx].species.push(EquilibriumSpeciesInput {
            formula: trimmed.to_string(),
            feed_temp_c: units.kelvin_to_temp(t_k),
            activity_coeff_expr: "1.0".into(),
            fixed_activity: None,
            discharge_coeff: 0.0,
            amount_unit: EquilibriumAmountUnit::Kmol,
            initial_amount: amount,
            add_x_amount: 0.0,
            add_z_amount: 0.0,
            remove_percent: 0.0,
            excluded: false,
            species_data: None,
            stoichiometry: BTreeMap::new(),
        });
        row_map[idx] = Some((p_idx, sp_idx));
    }

    // Respect §§ Pure Substance headers (a = 1) vs § Mixture headers (ideal gas/condensed mixture)
    for (ph, is_pure) in phases.iter_mut().zip(phase_is_pure.iter()) {
        if *is_pure {
            ph.is_gas_phase = false;
            ph.model = SolutionModel::PureSubstance;
            continue;
        }
        let all_gases = !ph.species.is_empty()
            && ph.species.iter().all(|s| s.formula.ends_with("(g)"));
        ph.is_gas_phase = all_gases;
        if !all_gases && ph.species.len() == 1 {
            ph.model = SolutionModel::PureSubstance;
        } else {
            ph.model = SolutionModel::IdealMixture;
        }
    }

    let system = EquilibriumSystem {
        title: "StreamEQ".into(),
        phases,
    };
    let mut params = EquilibriumParams::default();
    params.input_scale_to_mol = 1000.0;

    struct CatalogRepoAdapter<'a>(&'a dyn SpeciesCatalogPort);
    impl SpeciesRepository for CatalogRepoAdapter<'_> {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get_species(formula)
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            self.0.get_reference_element(element_symbol)
        }
    }
    let repo_adapter = CatalogRepoAdapter(catalog);
    let Ok(pt) = solve_single_point(&system, &params, &repo_adapter, t_k, p_bar, 0, t_user, None, None, 0.0) else {
        return CellEvalValue::Error("#VALUE!".into());
    };

    let mut out = Vec::with_capacity(species_rows.len());
    for (idx, mapping) in row_map.iter().enumerate() {
        if let Some((p_idx, _)) = mapping {
            let sp_name = species_rows[idx].trim();
            let val_kmol = pt
                .species
                .iter()
                .find(|r| r.phase_idx == *p_idx && r.formula.eq_ignore_ascii_case(sp_name))
                .map(|r| r.equil_amount_mol / 1000.0)
                .unwrap_or(0.0);
            out.push(CellEvalValue::Number(val_kmol));
        } else if species_rows[idx].trim().starts_with('§') {
            out.push(CellEvalValue::Number(t_user));
        } else {
            out.push(CellEvalValue::Text(String::new()));
        }
    }
    CellEvalValue::Array(out)
}

/// Core dispatcher for all HSC6.XLL + Mineral Add-In functions.
pub fn eval_addin_function(
    catalog: &dyn SpeciesCatalogPort,
    units: &mut AddInUnits,
    fn_name: &str,
    args: &[ArgValue],
) -> CellEvalValue {
    let fname = fn_name.trim().to_ascii_uppercase();
    match fname.as_str() {
        "UNITS" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let t_u = args[0].as_scalar_str().trim().trim_start_matches('°').trim();
            let e_u = args[1].as_scalar_str().trim();
            if !t_u.is_empty() {
                units.temp_unit = t_u.to_string();
            }
            if !e_u.is_empty() {
                units.energy_unit = e_u.to_string();
            }
            if let Some(p_u) = args.get(2).map(|a| a.as_scalar_str().trim()).filter(|s| !s.is_empty()) {
                units.pressure_unit = p_u.to_string();
            }
            CellEvalValue::Text(format!("{} and {}", units.temp_unit, units.energy_unit))
        }
        "ENERGYU" => CellEvalValue::Text(units.energy_unit.clone()),
        "TEMPU" => CellEvalValue::Text(units.temp_unit.clone()),
        "PRESU" => CellEvalValue::Text(units.pressure_unit.clone()),
        "BAL" => {
            let eq = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            let mult = args
                .get(1)
                .and_then(|a| a.as_f64())
                .filter(|v| v.is_finite() && *v > 0.0)
                .unwrap_or(1.0);
            match balance_reaction(eq, mult) {
                Ok((bal, _)) => CellEvalValue::Text(bal),
                Err(_) => CellEvalValue::Error("#VALUE!".into()),
            }
        }
        "SPECIES" => {
            let _db_no = args.first().and_then(|a| a.as_f64()).unwrap_or(2.0) as usize;
            let pos = args.get(1).and_then(|a| a.as_f64()).unwrap_or(1.0) as usize;
            if pos == 0 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let sp_opt = catalog.get_species_at(pos - 1).or_else(|| {
                let list = catalog.search_species("", pos + 8);
                list.get(pos.saturating_sub(1)).cloned()
            });
            if let Some(sp) = sp_opt {
                CellEvalValue::Text(sp.formula)
            } else {
                CellEvalValue::Error("#N/A".into())
            }
        }
        "H" | "HKG" | "HNM3" | "HCM" | "HLAT" | "S" | "CP" | "G" | "K"
        | "REACTIONH" | "REACTIONS" | "REACTIONCP" | "REACTIONG" | "REACTIONK" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let target = args[0].as_scalar_str();
            let Some(t_val) = args[1].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let kind = match fname.as_str() {
                "H" | "REACTIONH" => PropKind::H,
                "HKG" => PropKind::Hkg,
                "HNM3" | "HCM" => PropKind::Hnm3,
                "HLAT" => PropKind::Hlat,
                "S" | "REACTIONS" => PropKind::S,
                "CP" | "REACTIONCP" => PropKind::Cp,
                "G" | "REACTIONG" => PropKind::G,
                _ => PropKind::K,
            };
            eval_target_prop(catalog, units, target, t_val, kind)
                .map(CellEvalValue::Number)
                .unwrap_or_else(|| CellEvalValue::Error("#NAME?".into()))
        }
        "TATH" | "TATHKG" | "TATHNM3" | "TATHCM" | "TATHLAT" | "TATS" | "TATCP" | "TATG" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let target_sp = args[0].as_scalar_str();
            let Some(y_target) = args[1].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let kind = match fname.as_str() {
                "TATH" => PropKind::H,
                "TATHKG" => PropKind::Hkg,
                "TATHNM3" | "TATHCM" => PropKind::Hnm3,
                "TATHLAT" => PropKind::Hlat,
                "TATS" => PropKind::S,
                "TATCP" => PropKind::Cp,
                _ => PropKind::G,
            };
            let (sp_lo_k, sp_hi_k) = resolve_species_fallback(catalog, target_sp)
                .and_then(|sp| Some((sp.intervals.first()?.t_min, sp.intervals.last()?.t_max)))
                .unwrap_or((150.0, 6000.0));
            let eval_fn = |t| eval_target_prop(catalog, units, target_sp, t, kind);
            invert_monotone_temp(
                y_target,
                units.kelvin_to_temp(sp_lo_k),
                units.kelvin_to_temp(sp_hi_k.max(sp_lo_k + 1.0)),
                units,
                eval_fn,
            )
            .or_else(|| {
                invert_monotone_temp(
                    y_target,
                    units.kelvin_to_temp(sp_hi_k),
                    units.kelvin_to_temp(6000.0),
                    units,
                    eval_fn,
                )
            })
            .or_else(|| {
                invert_monotone_temp(
                    y_target,
                    units.kelvin_to_temp(50.0),
                    units.kelvin_to_temp(sp_lo_k),
                    units,
                    eval_fn,
                )
            })
            .map(CellEvalValue::Number)
            .unwrap_or_else(|| CellEvalValue::Error("#VALUE!".into()))
        }
        "FLOWH" | "FLOWHKG" | "FLOWHNM3" | "FLOWHLAT" | "FLOWS" | "FLOWCP" | "FLOWG"
        | "STREAMH" | "STREAMHKG" | "STREAMHNM3" | "STREAMHLAT" | "STREAMS" | "STREAMCP"
        | "STREAMG" | "DENSITYA" => {
            if args.len() < 3 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let species = args[0].as_list();
            let amounts = args[1].as_list();
            let Some(t_val) = args[2].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            if species.len() != amounts.len() {
                return CellEvalValue::Error("#VALUE!".into());
            }
            if fname == "DENSITYA" {
                let t_c = units.temp_to_kelvin(t_val) - 273.15;
                return eval_aqueous_density_kg_m3(&species, &amounts, t_c);
            }
            let kind = if fname.ends_with("HLAT") {
                PropKind::Hlat
            } else if fname.ends_with("HKG") {
                PropKind::Hkg
            } else if fname.ends_with("HNM3") {
                PropKind::Hnm3
            } else if fname.ends_with('H') {
                PropKind::H
            } else if fname.ends_with('S') {
                PropKind::S
            } else if fname.ends_with("CP") {
                PropKind::Cp
            } else {
                PropKind::G
            };
            let mut sum = 0.0;
            for (sp_name, amt_s) in species.iter().zip(amounts.iter()) {
                let sp_clean = sp_name.trim();
                if sp_clean.is_empty() {
                    continue;
                }
                let n = amt_s.trim().parse::<f64>().unwrap_or(0.0);
                if n == 0.0 {
                    continue;
                }
                let Some(val) = eval_target_prop(catalog, units, sp_clean, t_val, kind) else {
                    return CellEvalValue::Error("#NAME?".into());
                };
                sum += n * val;
            }
            CellEvalValue::Number(sum)
        }
        "STREAMTH" | "STREAMTHKG" | "STREAMTHNM3" | "STREAMTHLAT" | "STREAMTS" | "STREAMTCP"
        | "STREAMTG" | "FLOWTH" | "FLOWTS" | "FLOWTCP" | "FLOWTG" => {
            if args.len() < 5 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let species = args[0].as_list();
            let amounts = args[1].as_list();
            let Some(target_y) = args[2].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let Some(t_min) = args[3].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let Some(t_max) = args[4].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let kind = if fname.ends_with("THLAT") {
                PropKind::Hlat
            } else if fname.ends_with("THKG") {
                PropKind::Hkg
            } else if fname.ends_with("THNM3") {
                PropKind::Hnm3
            } else if fname.ends_with("TH") {
                PropKind::H
            } else if fname.ends_with("TS") {
                PropKind::S
            } else if fname.ends_with("TCP") {
                PropKind::Cp
            } else {
                PropKind::G
            };
            invert_monotone_temp(target_y, t_min, t_max, units, |t| {
                let mut sum = 0.0;
                for (sp_name, amt_s) in species.iter().zip(amounts.iter()) {
                    let sp_clean = sp_name.trim();
                    if sp_clean.is_empty() {
                        continue;
                    }
                    let n = amt_s.trim().parse::<f64>().unwrap_or(0.0);
                    sum += n * eval_target_prop(catalog, units, sp_clean, t, kind)?;
                }
                Some(sum)
            })
            .map(CellEvalValue::Number)
            .unwrap_or_else(|| CellEvalValue::Error("#VALUE!".into()))
        }
        "STREAMEQ" => {
            if args.len() < 4 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let sp_rows = args[0].as_list();
            let in_rows = args[1].as_list();
            let Some(t_val) = args[2].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            let Some(p_val) = args[3].as_f64() else {
                return CellEvalValue::Error("#VALUE!".into());
            };
            eval_streameq(catalog, units, &sp_rows, &in_rows, t_val, p_val)
        }
        "MW" | "MOLECULARWEIGHT" | "DE" | "REF" | "BP" | "MP" | "STRUCT" | "CHNAME" | "CONAME"
        | "CAN" | "PHASE" | "RGBCOLOR" | "SOLU" | "REL" | "CPA" | "CPB" | "CPC" | "CPD"
        | "CPFUNCTION" | "TMIN" | "TMAX" => {
            let target = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            if target.trim().is_empty() {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let Some(sp) = resolve_species_fallback(catalog, target) else {
                return CellEvalValue::Error("#NAME?".into());
            };
            let iv = sp.intervals.first();
            let meta = catalog.get_species_metadata(&sp.formula).unwrap_or_default();
            match fname.as_str() {
                "MW" | "MOLECULARWEIGHT" => CellEvalValue::Number(sp.molecular_weight),
                "DE" => CellEvalValue::Number(sp.density_g_cm3),
                "PHASE" => CellEvalValue::Text(match sp.phase {
                    Phase::Solid => "s",
                    Phase::Liquid => "l",
                    Phase::Gas => "g",
                    Phase::AqueousIon => "ia",
                    Phase::AqueousNeutral => "a",
                }.to_string()),
                "CHNAME" => CellEvalValue::Text(if sp.name.is_empty() { sp.formula.clone() } else { sp.name.clone() }),
                "CONAME" => CellEvalValue::Text(if !meta.common_name.is_empty() {
                    meta.common_name
                } else if !sp.name.is_empty() {
                    sp.name.clone()
                } else {
                    sp.formula.clone()
                }),
                "STRUCT" => CellEvalValue::Text(if !meta.structural_formula.is_empty() {
                    meta.structural_formula
                } else {
                    sp.formula.clone()
                }),
                "CAN" => CellEvalValue::Text(if !meta.cas_number.is_empty() {
                    meta.cas_number
                } else if sp.formula.eq_ignore_ascii_case("H2O(l)") {
                    "7732-18-5".into()
                } else {
                    "-".into()
                }),
                "REF" => CellEvalValue::Text(iv.map(|r| r.reference.clone()).unwrap_or_default()),
                "BP" => CellEvalValue::Number(if meta.boiling_point_k > 0.0 {
                    units.kelvin_to_temp(meta.boiling_point_k)
                } else if sp.formula.eq_ignore_ascii_case("H2O(l)") {
                    units.kelvin_to_temp(373.15)
                } else {
                    0.0
                }),
                "MP" => CellEvalValue::Number(if meta.melting_point_k > 0.0 {
                    units.kelvin_to_temp(meta.melting_point_k)
                } else if sp.formula.eq_ignore_ascii_case("H2O(l)") {
                    units.kelvin_to_temp(273.15)
                } else {
                    0.0
                }),
                "RGBCOLOR" => CellEvalValue::Text(if meta.color_code != 0 {
                    meta.color_code.to_string()
                } else if sp.formula.eq_ignore_ascii_case("H2O(l)") {
                    "16".into()
                } else {
                    "0".into()
                }),
                "SOLU" => CellEvalValue::Text(if !meta.solubility.is_empty() { meta.solubility } else { "-".into() }),
                "REL" => CellEvalValue::Number(if meta.reliability_class > 0 { meta.reliability_class as f64 } else { 1.0 }),
                "CPA" => CellEvalValue::Number(iv.map(|r| r.kelley.a).unwrap_or(0.0)),
                "CPB" => CellEvalValue::Number(iv.map(|r| r.kelley.b).unwrap_or(0.0)),
                "CPC" => CellEvalValue::Number(iv.map(|r| r.kelley.c).unwrap_or(0.0)),
                "CPD" => CellEvalValue::Number(iv.map(|r| r.kelley.d).unwrap_or(0.0)),
                "TMIN" => CellEvalValue::Number(iv.map(|r| r.t_min).unwrap_or(0.0)),
                "TMAX" => CellEvalValue::Number(iv.map(|r| r.t_max).unwrap_or(0.0)),
                "CPFUNCTION" => {
                    if let Some(k) = iv.map(|r| r.kelley) {
                        CellEvalValue::Text(format!(
                            "{:.2} + {:.2}*10^(-3)*T + {:.2}*10^5*T^(-2) + {:.2}*10^(-6)*T^2",
                            k.a, k.b, k.c, k.d
                        ))
                    } else {
                        CellEvalValue::Error("#N/A".into())
                    }
                }
                _ => CellEvalValue::Error("#N/A".into()),
            }
        }
        "MOLP" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            eval_composition_pct(args[0].as_scalar_str(), args[1].as_scalar_str(), false)
        }
        "WTP" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            eval_composition_pct(args[0].as_scalar_str(), args[1].as_scalar_str(), true)
        }
        // Mineralogy functions from Fig. 7
        "MINERALCODE" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            lookup_mineral(q)
                .map(|m| CellEvalValue::Text(m.code))
                .unwrap_or_else(|| CellEvalValue::Error("#N/A".into()))
        }
        "MINERALFORMULA" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            lookup_mineral(q)
                .map(|m| CellEvalValue::Text(m.formula))
                .unwrap_or_else(|| CellEvalValue::Error("#N/A".into()))
        }
        "MINERALNAME" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            lookup_mineral(q)
                .map(|m| CellEvalValue::Text(m.name))
                .unwrap_or_else(|| CellEvalValue::Error("#N/A".into()))
        }
        "MINERALC" | "MINERALELEMENTWT" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let q = args[0].as_scalar_str();
            let el = args[1].as_scalar_str().trim();
            let Some(m) = lookup_mineral(q) else {
                return CellEvalValue::Error("#N/A".into());
            };
            let v = m.comp_wt_pct.iter().find(|(k, _)| k.eq_ignore_ascii_case(el)).map(|(_, val)| *val).unwrap_or(0.0);
            CellEvalValue::Number(v)
        }
        "MINERALCOLOR" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            lookup_mineral(q)
                .map(|m| CellEvalValue::Text(m.color))
                .unwrap_or_else(|| CellEvalValue::Error("#N/A".into()))
        }
        "MINERALDENSITY" | "MINERALSG" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            match lookup_mineral(q) {
                Some(m) => m.density.map(CellEvalValue::Number).unwrap_or_else(|| CellEvalValue::Text("-".into())),
                None => CellEvalValue::Error("#N/A".into()),
            }
        }
        "MINERALHARDNESS" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            match lookup_mineral(q) {
                Some(m) => m.hardness.map(CellEvalValue::Number).unwrap_or_else(|| CellEvalValue::Text("-".into())),
                None => CellEvalValue::Error("#N/A".into()),
            }
        }
        "MINERALLUSTER" => {
            let q = args.first().map(|a| a.as_scalar_str()).unwrap_or("");
            lookup_mineral(q)
                .map(|m| CellEvalValue::Text(m.luster))
                .unwrap_or_else(|| CellEvalValue::Error("#N/A".into()))
        }
        "MINERALPROPERTY" => {
            if args.len() < 2 {
                return CellEvalValue::Error("#VALUE!".into());
            }
            let q = args[0].as_scalar_str();
            let prop = args[1].as_scalar_str().trim().trim_start_matches('-');
            let Some(m) = lookup_mineral(q) else {
                return CellEvalValue::Error("#N/A".into());
            };
            match prop.to_ascii_lowercase().as_str() {
                "code" => CellEvalValue::Text(m.code),
                "formula" => CellEvalValue::Text(m.formula),
                "name" => CellEvalValue::Text(m.name),
                "group" => CellEvalValue::Text(m.group),
                "density" | "sg" => m.density.map(CellEvalValue::Number).unwrap_or_else(|| CellEvalValue::Text("-".into())),
                "hardness" => m.hardness.map(CellEvalValue::Number).unwrap_or_else(|| CellEvalValue::Text("-".into())),
                "color" => CellEvalValue::Text(m.color),
                "luster" => CellEvalValue::Text(m.luster),
                "location" => CellEvalValue::Text(m.location),
                "id" => CellEvalValue::Number(m.id as f64),
                "elements" => CellEvalValue::Text(m.elements_str),
                "database" => CellEvalValue::Text(m.database),
                "reference" => CellEvalValue::Text(m.reference),
                _ => {
                    let v = m.comp_wt_pct.iter().find(|(k, _)| k.eq_ignore_ascii_case(prop)).map(|(_, val)| *val).unwrap_or(0.0);
                    CellEvalValue::Number(v)
                }
            }
        }
        "STREAMX" | "RECOVERYX" | "PARTICLERECX" | "FRACTIONX" | "MINERALX" => {
            CellEvalValue::Error("#HSC_SIM_ONLY".into())
        }
        _ => CellEvalValue::Error("#NAME?".into()),
    }
}

/// Parse A1 / $A$1 cell coordinate to 0-based (row, col).
pub fn parse_a1(token: &str) -> Option<(usize, usize)> {
    let clean = token.trim().replace('$', "");
    let mut split = 0usize;
    for (i, ch) in clean.char_indices() {
        if ch.is_ascii_digit() {
            split = i;
            break;
        }
    }
    if split == 0 {
        return None;
    }
    let col_part = &clean[..split];
    let row_part = &clean[split..];
    if !col_part.chars().all(|c| c.is_ascii_alphabetic()) || row_part.is_empty() {
        return None;
    }
    let row = row_part.parse::<usize>().ok()?.checked_sub(1)?;
    let mut col = 0usize;
    for b in col_part.to_ascii_uppercase().bytes() {
        col = col * 26 + ((b - b'A') as usize + 1);
    }
    Some((row, col.checked_sub(1)?))
}

fn split_top_level_args(inner: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
                cur.push(ch);
            }
            '(' if !in_quote => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_quote => {
                depth = (depth - 1).max(0);
                cur.push(ch);
            }
            ';' | ',' if !in_quote && depth == 0 => {
                args.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() || !args.is_empty() {
        args.push(cur.trim().to_string());
    }
    args
}

fn is_known_formula_fn(name: &str) -> bool {
    let upper = name.trim().to_ascii_uppercase();
    matches!(upper.as_str(), "SUM" | "AVERAGE" | "ABS" | "EXP" | "LN" | "LOG10")
        || ADDIN_FUNCTION_CATALOG.iter().any(|f| f.name.eq_ignore_ascii_case(&upper))
}

/// Find the outermost matching single function call `NAME(...)` spanning the entire trimmed expression.
fn split_single_fn_call(expr: &str) -> Option<(&str, &str)> {
    let open = expr.find('(')?;
    let prefix = expr[..open].trim();
    if prefix.is_empty() || !prefix.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
        return None;
    }
    if !expr.ends_with(')') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_quote = false;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => {
                depth -= 1;
                if depth == 0 && idx < expr.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some((prefix, &expr[open + 1..expr.len() - 1]))
    } else {
        None
    }
}

/// Split at the rightmost top-level operator (`+`/`-` for `low_prec = true`, `*`/`/` for `false`).
fn split_top_level_binop(expr: &str, low_prec: bool) -> Option<(&str, char, &str)> {
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut last_match: Option<(usize, char)> = None;
    for (i, ch) in expr.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => depth = (depth - 1).max(0),
            '+' | '-' if low_prec && !in_quote && depth == 0 && i > 0 => {
                // Skip unary sign after operator or scientific exponent `1e-4`
                let prev = bytes[i - 1] as char;
                if !matches!(prev, '+' | '-' | '*' | '/' | '(' | 'e' | 'E') {
                    last_match = Some((i, ch));
                }
            }
            '*' | '/' if !low_prec && !in_quote && depth == 0 && i > 0 => {
                last_match = Some((i, ch));
            }
            _ => {}
        }
    }
    let (pos, op) = last_match?;
    Some((expr[..pos].trim(), op, expr[pos + 1..].trim()))
}

fn eval_expr_recursive<F>(
    catalog: &dyn SpeciesCatalogPort,
    units: &mut AddInUnits,
    raw_expr: &str,
    cell_lookup: &mut F,
    depth: usize,
) -> CellEvalValue
where
    F: FnMut(usize, usize) -> String,
{
    if depth > 16 {
        return CellEvalValue::Error("#VALUE!".into());
    }
    let expr = raw_expr.trim();
    if expr.is_empty() {
        return CellEvalValue::Text(String::new());
    }
    if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
        return CellEvalValue::Text(expr[1..expr.len() - 1].to_string());
    }
    if expr.starts_with('(') && expr.ends_with(')') {
        if let Some(("", inner)) = split_single_fn_call(expr) {
            return eval_expr_recursive(catalog, units, inner, cell_lookup, depth + 1);
        }
    }
    if let Some((fn_name, inner)) = split_single_fn_call(expr) {
        let raw_args = split_top_level_args(inner);
        let upper_fn = fn_name.to_ascii_uppercase();
        if matches!(upper_fn.as_str(), "SUM" | "AVERAGE" | "ABS" | "EXP" | "LN" | "LOG10") {
            let mut nums = Vec::new();
            for a in raw_args {
                if let Some((lhs, rhs)) = a.split_once(':') {
                    if let (Some((r1, c1)), Some((r2, c2))) = (parse_a1(lhs), parse_a1(rhs)) {
                        for rr in r1.min(r2)..=r1.max(r2) {
                            for cc in c1.min(c2)..=c1.max(c2) {
                                let raw_c = cell_lookup(rr, cc);
                                let tr = raw_c.trim();
                                let unwrapped = tr
                                    .strip_prefix('{')
                                    .and_then(|s| s.strip_suffix('}'))
                                    .unwrap_or(tr)
                                    .trim();
                                if let Some(sub) = unwrapped.strip_prefix('=') {
                                    if let Some(v) = eval_expr_recursive(catalog, units, sub.trim(), cell_lookup, depth + 1).as_f64() {
                                        nums.push(v);
                                    }
                                } else if let Ok(v) = tr.parse::<f64>() {
                                    nums.push(v);
                                }
                            }
                        }
                    }
                } else if let Some(v) = eval_expr_recursive(catalog, units, &a, cell_lookup, depth + 1).as_f64() {
                    nums.push(v);
                }
            }
            return match upper_fn.as_str() {
                "SUM" => CellEvalValue::Number(nums.iter().sum()),
                "AVERAGE" => {
                    if nums.is_empty() {
                        CellEvalValue::Error("#DIV/0!".into())
                    } else {
                        CellEvalValue::Number(nums.iter().sum::<f64>() / nums.len() as f64)
                    }
                }
                "ABS" => nums.first().map(|v| CellEvalValue::Number(v.abs())).unwrap_or_else(|| CellEvalValue::Error("#VALUE!".into())),
                "EXP" => nums.first().map(|v| CellEvalValue::Number(v.exp())).unwrap_or_else(|| CellEvalValue::Error("#VALUE!".into())),
                "LN" => nums.first().filter(|&&v| v > 0.0).map(|v| CellEvalValue::Number(v.ln())).unwrap_or_else(|| CellEvalValue::Error("#NUM!".into())),
                "LOG10" => nums.first().filter(|&&v| v > 0.0).map(|v| CellEvalValue::Number(v.log10())).unwrap_or_else(|| CellEvalValue::Error("#NUM!".into())),
                _ => CellEvalValue::Error("#NAME?".into()),
            };
        }
        let mut parsed_args = Vec::with_capacity(raw_args.len());
        for a in raw_args {
            if a.starts_with('"') && a.ends_with('"') && a.len() >= 2 {
                parsed_args.push(ArgValue::Scalar(a[1..a.len() - 1].to_string()));
            } else if let Some((lhs, rhs)) = a.split_once(':') {
                if let (Some((r1, c1)), Some((r2, c2))) = (parse_a1(lhs), parse_a1(rhs)) {
                    let mut vals = Vec::new();
                    for rr in r1.min(r2)..=r1.max(r2) {
                        for cc in c1.min(c2)..=c1.max(c2) {
                            let raw_c = cell_lookup(rr, cc);
                            let tr = raw_c.trim();
                            let unwrapped = tr
                                .strip_prefix('{')
                                .and_then(|s| s.strip_suffix('}'))
                                .unwrap_or(tr)
                                .trim();
                            if let Some(sub) = unwrapped.strip_prefix('=') {
                                let v = eval_expr_recursive(catalog, units, sub.trim(), cell_lookup, depth + 1);
                                vals.push(v.as_display_string(8));
                            } else {
                                vals.push(raw_c);
                            }
                        }
                    }
                    parsed_args.push(ArgValue::Range(vals));
                } else {
                    parsed_args.push(ArgValue::Scalar(a));
                }
            } else if let Some((r, c)) = parse_a1(&a) {
                let raw_cell = cell_lookup(r, c);
                let trimmed = raw_cell.trim();
                let unwrapped = trimmed
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                    .unwrap_or(trimmed)
                    .trim();
                if let Some(sub_expr) = unwrapped.strip_prefix('=') {
                    let nested = eval_expr_recursive(
                        catalog,
                        units,
                        sub_expr.trim(),
                        cell_lookup,
                        depth + 1,
                    );
                    parsed_args.push(ArgValue::Scalar(nested.as_display_string(8)));
                } else {
                    parsed_args.push(ArgValue::Scalar(raw_cell));
                }
            } else if let Some((sub_fn, _)) = split_single_fn_call(&a) {
                if is_known_formula_fn(sub_fn) {
                    let nested = eval_expr_recursive(catalog, units, &a, cell_lookup, depth + 1);
                    parsed_args.push(ArgValue::Scalar(nested.as_display_string(8)));
                } else {
                    parsed_args.push(ArgValue::Scalar(a));
                }
            } else if split_top_level_binop(&a, true).is_some() || split_top_level_binop(&a, false).is_some() {
                // Evaluate arithmetic expressions inside args only if RHS/LHS are numbers or cells/functions (not species equations with `+`/`=`)
                if !a.contains('=') && !a.chars().any(|c| c == '(') || split_top_level_binop(&a, true).is_some_and(|(l, _, r)| {
                    l.parse::<f64>().is_ok() || parse_a1(l).is_some() || r.parse::<f64>().is_ok() || parse_a1(r).is_some()
                }) {
                    let nested = eval_expr_recursive(catalog, units, &a, cell_lookup, depth + 1);
                    parsed_args.push(ArgValue::Scalar(nested.as_display_string(8)));
                } else {
                    parsed_args.push(ArgValue::Scalar(a));
                }
            } else {
                parsed_args.push(ArgValue::Scalar(a));
            }
        }
        return eval_addin_function(catalog, units, fn_name, &parsed_args);
    }

    if let Some((lhs, op, rhs)) = split_top_level_binop(expr, true).or_else(|| split_top_level_binop(expr, false)) {
        let lv = eval_expr_recursive(catalog, units, lhs, cell_lookup, depth + 1).as_f64();
        let rv = eval_expr_recursive(catalog, units, rhs, cell_lookup, depth + 1).as_f64();
        return match (lv, op, rv) {
            (Some(a), '+', Some(b)) => CellEvalValue::Number(a + b),
            (Some(a), '-', Some(b)) => CellEvalValue::Number(a - b),
            (Some(a), '*', Some(b)) => CellEvalValue::Number(a * b),
            (Some(_), '/', Some(b)) if b.abs() < 1e-18 => CellEvalValue::Error("#DIV/0!".into()),
            (Some(a), '/', Some(b)) => CellEvalValue::Number(a / b),
            _ => CellEvalValue::Error("#VALUE!".into()),
        };
    }

    if let Some((r, c)) = parse_a1(expr) {
        let v = cell_lookup(r, c);
        if v.trim_start().starts_with('=') {
            return eval_expr_recursive(catalog, units, v.trim_start_matches('='), cell_lookup, depth + 1);
        }
        return v.trim().parse::<f64>().map(CellEvalValue::Number).unwrap_or(CellEvalValue::Text(v));
    }
    if let Ok(num) = expr.parse::<f64>() {
        return CellEvalValue::Number(num);
    }
    CellEvalValue::Text(expr.to_string())
}

/// Evaluate a spreadsheet formula like `=H(D11;E11)`, `=H(SPECIES(2;5);25)`, `=H("CO2(g)";200)-H("CO2(g)";25)`, or `{=StreamEQ(A4:A15;B4:B15;C1;C2)}`.
pub fn eval_formula_string<F>(
    catalog: &dyn SpeciesCatalogPort,
    units: &mut AddInUnits,
    formula: &str,
    mut cell_lookup: F,
) -> CellEvalValue
where
    F: FnMut(usize, usize) -> String,
{
    let mut expr = formula.trim();
    if expr.starts_with('{') && expr.ends_with('}') {
        expr = &expr[1..expr.len() - 1];
    }
    let expr = expr.trim().trim_start_matches('=').trim();
    eval_expr_recursive(catalog, units, expr, &mut cell_lookup, 0)
}

/// Migrate legacy `HSC5.xla` formulas (`='C:\HSC5\...\HSC5.xla'!H(D11;E11)`) to clean HSC6 / multi-suite calls.
pub fn migrate_legacy_formula(raw: &str) -> (String, Vec<String>) {
    let mut notes = Vec::new();
    let mut s = raw.trim().to_string();
    // Iteratively strip any `'...\foo.xla'!` or `...\HSC6.XLL!` prefixes
    loop {
        let lower = s.to_ascii_lowercase();
        if let Some(excl) = lower.find(".xla'!") {
            if let Some(qstart) = s[..excl].rfind('\'') {
                s.replace_range(qstart..excl + 6, "");
                notes.push("Removed legacy .xla workbook path reference".into());
                continue;
            }
        }
        if let Some(excl) = lower.find(".xll'!") {
            if let Some(qstart) = s[..excl].rfind('\'') {
                s.replace_range(qstart..excl + 6, "");
                notes.push("Removed hardcoded .xll path reference".into());
                continue;
            }
        }
        break;
    }
    if notes.is_empty() && s != raw.trim() {
        notes.push("Normalized formula".into());
    }
    (s, notes)
}

/// Build universal macro-free `=WEBSERVICE(...)` formula supported natively by Excel 2013+, LibreOffice, ONLYOFFICE, and WPS.
pub fn build_webservice_formula(port: u16, fn_name: &str, args: &[&str]) -> String {
    let enc = |s: &str| -> String {
        let mut out = String::new();
        for b in s.bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{:02X}", b));
            }
        }
        out
    };
    let mut url = format!("http://127.0.0.1:{port}/eval?fn={}", enc(fn_name));
    for (key, val) in ["a", "b", "c", "d", "e"].iter().zip(args.iter()) {
        url.push_str(&format!("&{key}={}", enc(val)));
    }
    format!("=WEBSERVICE(\"{url}\")")
}

pub fn format_temp_unit_display(u: &str) -> String {
    let clean = u.trim().trim_start_matches('°');
    if clean.eq_ignore_ascii_case("K") {
        "K".into()
    } else {
        format!("°{clean}")
    }
}

/// Build the `StreamEQ` sheet matching Fig. 4–6 (`A1:D15`) for live display or `.xlsx`/`.ods` export.
pub fn build_streameq_export_sheet(
    catalog: &dyn SpeciesCatalogPort,
    units: &AddInUnits,
    species: &[String],
    inputs: &[String],
    t_val: f64,
    p_val: f64,
) -> ExportSheet {
    let t_lbl = format_temp_unit_display(&units.temp_unit);
    let mut cells = vec![
        ExportCell { row: 0, col: 0, value: ExportCellValue::Text("Temperature".into()), style: ExportCellStyle::Header },
        ExportCell { row: 0, col: 2, value: ExportCellValue::Number(t_val), style: ExportCellStyle::Default },
        ExportCell { row: 0, col: 3, value: ExportCellValue::Text(t_lbl.clone()), style: ExportCellStyle::Default },
        ExportCell { row: 1, col: 0, value: ExportCellValue::Text("Pressure".into()), style: ExportCellStyle::Header },
        ExportCell { row: 1, col: 2, value: ExportCellValue::Number(p_val), style: ExportCellStyle::Default },
        ExportCell { row: 1, col: 3, value: ExportCellValue::Text(units.pressure_unit.clone()), style: ExportCellStyle::Default },
        ExportCell { row: 2, col: 0, value: ExportCellValue::Text("SPECIES:".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 2, col: 1, value: ExportCellValue::Text("Input".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 2, col: 2, value: ExportCellValue::Text("Output".into()), style: ExportCellStyle::BoldBox },
    ];

    let eq = eval_streameq(catalog, units, species, inputs, t_val, p_val);
    let arr = match eq {
        CellEvalValue::Array(a) => a,
        _ => Vec::new(),
    };
    let end_row = 3 + species.len();
    for (i, sp) in species.iter().enumerate() {
        let r = 3 + i;
        let is_phase = sp.trim().starts_with('§');
        cells.push(ExportCell {
            row: r,
            col: 0,
            value: ExportCellValue::Text(sp.clone()),
            style: if is_phase { ExportCellStyle::BoldBox } else { ExportCellStyle::Default },
        });
        if let Ok(iv) = inputs[i].trim().parse::<f64>() {
            cells.push(ExportCell { row: r, col: 1, value: ExportCellValue::Number(iv), style: ExportCellStyle::Default });
        }
        let cached = arr.get(i).and_then(|v| v.as_f64());
        cells.push(ExportCell {
            row: r,
            col: 2,
            value: if i == 0 {
                ExportCellValue::Formula {
                    formula: format!("=StreamEQ(A4:A{end_row};B4:B{end_row};C1;C2)"),
                    cached_number: cached,
                    cached_text: None,
                    array_rows: species.len(),
                }
            } else if let Some(v) = cached {
                ExportCellValue::Number(v)
            } else {
                ExportCellValue::Text(String::new())
            },
            style: ExportCellStyle::Default,
        });
        cells.push(ExportCell {
            row: r,
            col: 3,
            value: ExportCellValue::Text(if is_phase { t_lbl.clone() } else { "kmol".into() }),
            style: ExportCellStyle::Default,
        });
    }
    ExportSheet {
        name: "StreamEQ".into(),
        cells,
        col_widths: vec![18.0, 12.0, 18.0, 10.0],
    }
}

/// Build the Minerals sheet matching Fig. 7 (`Mineral 1..4` formulas + `Mineral 5` pasted static values) for `.xlsx`/`.ods` export.
pub fn build_minerals_export_sheet() -> ExportSheet {
    let queries = ["Pyrite", "Py", "Py\\Joki", "ID:12616"];
    let labels = [
        ("Mineral", ExportCellStyle::Olive),
        ("Code", ExportCellStyle::Olive),
        ("Formula", ExportCellStyle::Olive),
        ("Name", ExportCellStyle::Olive),
        ("Cu", ExportCellStyle::Olive),
        ("Zn", ExportCellStyle::Olive),
        ("Fe", ExportCellStyle::Olive),
        ("S", ExportCellStyle::Olive),
        ("Density", ExportCellStyle::Green),
        ("Hardness", ExportCellStyle::Green),
        ("Color", ExportCellStyle::Green),
        ("Luster", ExportCellStyle::Green),
        ("Location", ExportCellStyle::Teal),
        ("ID", ExportCellStyle::Teal),
        ("Elements", ExportCellStyle::Teal),
        ("Database", ExportCellStyle::Teal),
        ("Reference", ExportCellStyle::Teal),
    ];
    let mut cells = vec![
        ExportCell { row: 0, col: 0, value: ExportCellValue::Text("Example of using HSC Chemistry Add-in functions to create a table of minerals".into()), style: ExportCellStyle::Header },
    ];
    for (ci, h) in ["", "Mineral 1", "Mineral 2", "Mineral 3", "Mineral 4", "Mineral 5"].iter().enumerate() {
        if !h.is_empty() {
            cells.push(ExportCell { row: 2, col: ci, value: ExportCellValue::Text((*h).into()), style: ExportCellStyle::Header });
        }
    }
    for (ri, (lbl, sty)) in labels.iter().enumerate() {
        let r = 3 + ri;
        cells.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text((*lbl).into()), style: *sty });
    }
    for (mi, q) in queries.iter().enumerate() {
        let col = 1 + mi;
        cells.push(ExportCell { row: 3, col, value: ExportCellValue::Text((*q).into()), style: ExportCellStyle::InputRed });
        let Some(rec) = lookup_mineral(q) else { continue; };
        let col_let = crate::spreadsheet_export::col_to_a1(col);
        let props: Vec<(&str, ExportCellValue)> = vec![
            ("Code", ExportCellValue::Formula { formula: format!("=MineralCode({col_let}4)"), cached_number: None, cached_text: Some(rec.code.clone()), array_rows: 0 }),
            ("Formula", ExportCellValue::Formula { formula: format!("=MineralFormula({col_let}4)"), cached_number: None, cached_text: Some(rec.formula.clone()), array_rows: 0 }),
            ("Name", ExportCellValue::Formula { formula: format!("=MineralName({col_let}4)"), cached_number: None, cached_text: Some(rec.name.clone()), array_rows: 0 }),
            ("Cu", ExportCellValue::Formula { formula: format!("=MineralC({col_let}4;\"Cu\")"), cached_number: Some(*rec.comp_wt_pct.get("Cu").unwrap_or(&0.0)), cached_text: None, array_rows: 0 }),
            ("Zn", ExportCellValue::Formula { formula: format!("=MineralC({col_let}4;\"Zn\")"), cached_number: Some(*rec.comp_wt_pct.get("Zn").unwrap_or(&0.0)), cached_text: None, array_rows: 0 }),
            ("Fe", ExportCellValue::Formula { formula: format!("=MineralC({col_let}4;\"Fe\")"), cached_number: Some(*rec.comp_wt_pct.get("Fe").unwrap_or(&0.0)), cached_text: None, array_rows: 0 }),
            ("S", ExportCellValue::Formula { formula: format!("=MineralC({col_let}4;\"S\")"), cached_number: Some(*rec.comp_wt_pct.get("S").unwrap_or(&0.0)), cached_text: None, array_rows: 0 }),
            ("Density", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A12)"), cached_number: rec.density, cached_text: None, array_rows: 0 }),
            ("Hardness", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A13)"), cached_number: rec.hardness, cached_text: None, array_rows: 0 }),
            ("Color", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A14)"), cached_number: None, cached_text: Some(rec.color.clone()), array_rows: 0 }),
            ("Luster", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A15)"), cached_number: None, cached_text: Some(rec.luster.clone()), array_rows: 0 }),
            ("Location", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A16)"), cached_number: None, cached_text: Some(rec.location.clone()), array_rows: 0 }),
            ("ID", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A17)"), cached_number: Some(rec.id as f64), cached_text: None, array_rows: 0 }),
            ("Elements", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A18)"), cached_number: None, cached_text: Some(rec.elements_str.clone()), array_rows: 0 }),
            ("Database", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A19)"), cached_number: None, cached_text: Some(rec.database.clone()), array_rows: 0 }),
            ("Reference", ExportCellValue::Formula { formula: format!("=MineralProperty({col_let}4;$A20)"), cached_number: None, cached_text: Some(rec.reference.clone()), array_rows: 0 }),
        ];
        for (pi, (_, val)) in props.into_iter().enumerate() {
            cells.push(ExportCell { row: 4 + pi, col, value: val, style: ExportCellStyle::Default });
        }
        if mi == 3 {
            // Mineral 5 (col F): pasted values snapshot of Mineral 4 matching Fig. 7
            cells.push(ExportCell { row: 3, col: 5, value: ExportCellValue::Text((*q).into()), style: ExportCellStyle::InputRed });
            let static_vals: Vec<ExportCellValue> = vec![
                ExportCellValue::Text(rec.code.clone()),
                ExportCellValue::Text(rec.formula.clone()),
                ExportCellValue::Text(rec.name.clone()),
                ExportCellValue::Number(*rec.comp_wt_pct.get("Cu").unwrap_or(&0.0)),
                ExportCellValue::Number(*rec.comp_wt_pct.get("Zn").unwrap_or(&0.0)),
                ExportCellValue::Number(*rec.comp_wt_pct.get("Fe").unwrap_or(&0.0)),
                ExportCellValue::Number(*rec.comp_wt_pct.get("S").unwrap_or(&0.0)),
                rec.density.map(ExportCellValue::Number).unwrap_or_else(|| ExportCellValue::Text("-".into())),
                rec.hardness.map(ExportCellValue::Number).unwrap_or_else(|| ExportCellValue::Text("-".into())),
                ExportCellValue::Text(rec.color.clone()),
                ExportCellValue::Text(rec.luster.clone()),
                ExportCellValue::Text(rec.location.clone()),
                ExportCellValue::Number(rec.id as f64),
                ExportCellValue::Text(rec.elements_str.clone()),
                ExportCellValue::Text(rec.database.clone()),
                ExportCellValue::Text(rec.reference.clone()),
            ];
            for (pi, val) in static_vals.into_iter().enumerate() {
                cells.push(ExportCell { row: 4 + pi, col: 5, value: val, style: ExportCellStyle::Default });
            }
        }
    }
    cells.push(ExportCell {
        row: 21,
        col: 1,
        value: ExportCellValue::Text("Note: Mineral 5 holds static values pasted from Mineral 4 (ID:12616 Pyhäsalmi) as shown in HSC Manual Fig. 7.".into()),
        style: ExportCellStyle::YellowNote,
    });
    ExportSheet {
        name: "Minerals".into(),
        cells,
        col_widths: vec![14.0, 18.0, 18.0, 18.0, 18.0, 18.0],
    }
}

/// Build a valid, coordinate-consistent `AddInSample` sheet matching HSC6 `AddInSample.xls` + HSC10 Table 1 functions.
pub fn build_addinsample_export_sheet(catalog: &dyn SpeciesCatalogPort, units: &AddInUnits) -> ExportSheet {
    let mut u = units.clone();
    let mut cells = vec![
        ExportCell { row: 0, col: 0, value: ExportCellValue::Text("HSC Chemistry Add-In Sample Reference Sheet".into()), style: ExportCellStyle::Header },
        ExportCell { row: 1, col: 0, value: ExportCellValue::Text("Active Units:".into()), style: ExportCellStyle::BoldBox },
        ExportCell { row: 1, col: 1, value: ExportCellValue::Formula { formula: "=TempU()".into(), cached_number: None, cached_text: Some(u.temp_unit.clone()), array_rows: 0 }, style: ExportCellStyle::Default },
        ExportCell { row: 1, col: 2, value: ExportCellValue::Formula { formula: "=EnergyU()".into(), cached_number: None, cached_text: Some(u.energy_unit.clone()), array_rows: 0 }, style: ExportCellStyle::Default },
        ExportCell { row: 1, col: 3, value: ExportCellValue::Formula { formula: "=PresU()".into(), cached_number: None, cached_text: Some(u.pressure_unit.clone()), array_rows: 0 }, style: ExportCellStyle::Default },
    ];
    for (ci, hdr) in ["#", "Function", "Description", "Arg 1 (Formula/Eq)", "Arg 2 (T/Amount/Mult)", "Result", "Unit / Notes"].iter().enumerate() {
        cells.push(ExportCell { row: 2, col: ci, value: ExportCellValue::Text((*hdr).into()), style: ExportCellStyle::Header });
    }

    let push_eval = |cells: &mut Vec<ExportCell>, r: usize, num: &str, fname: &str, desc: &str, arg1: ExportCellValue, arg2: Option<ExportCellValue>, formula: String, unit_note: &str, eval_val: CellEvalValue| {
        cells.push(ExportCell { row: r, col: 0, value: ExportCellValue::Text(num.into()), style: ExportCellStyle::Default });
        cells.push(ExportCell { row: r, col: 1, value: ExportCellValue::Text(fname.into()), style: ExportCellStyle::BoldBox });
        cells.push(ExportCell { row: r, col: 2, value: ExportCellValue::Text(desc.into()), style: ExportCellStyle::Default });
        cells.push(ExportCell { row: r, col: 3, value: arg1, style: ExportCellStyle::InputRed });
        if let Some(a2) = arg2 {
            cells.push(ExportCell { row: r, col: 4, value: a2, style: ExportCellStyle::InputRed });
        }
        let (cn, ct) = match eval_val {
            CellEvalValue::Number(v) => (Some(v), None),
            CellEvalValue::Text(t) => (None, Some(t)),
            CellEvalValue::Error(e) => (None, Some(e)),
            CellEvalValue::Array(arr) => (arr.first().and_then(|v| v.as_f64()), None),
        };
        cells.push(ExportCell {
            row: r,
            col: 5,
            value: ExportCellValue::Formula { formula, cached_number: cn, cached_text: ct, array_rows: 0 },
            style: ExportCellStyle::Default,
        });
        cells.push(ExportCell { row: r, col: 6, value: ExportCellValue::Text(unit_note.into()), style: ExportCellStyle::Default });
    };

    // Row 4 (r=3) .. Row 25 (r=24): scalar and pair functions with valid A1 coordinates (`D4`, `E4`, ...)
    let bal_v = eval_addin_function(catalog, &mut u, "BAL", &[ArgValue::Scalar("C+O2(g)=CO(g)".into()), ArgValue::Scalar("0.5".into())]);
    push_eval(&mut cells, 3, "2", "BAL", "Balanced equation with 0.5 multiplier", ExportCellValue::Text("C+O2(g)=CO(g)".into()), Some(ExportCellValue::Number(0.5)), "=BAL(D4;E4)".into(), "str", bal_v);

    let sp_v = eval_addin_function(catalog, &mut u, "Species", &[ArgValue::Scalar("2".into()), ArgValue::Scalar("3".into())]);
    push_eval(&mut cells, 4, "1", "Species", "Species at database position", ExportCellValue::Number(2.0), Some(ExportCellValue::Number(3.0)), "=SPECIES(D5;E5)".into(), "formula", sp_v);

    let h_co2 = eval_addin_function(catalog, &mut u, "H", &[ArgValue::Scalar("CO2(g)".into()), ArgValue::Scalar("100".into())]);
    let h_co2_val = h_co2.as_f64().unwrap_or(0.0);
    push_eval(&mut cells, 5, "4", "H", "Enthalpy of CO2(g) at 100 C", ExportCellValue::Text("CO2(g)".into()), Some(ExportCellValue::Number(100.0)), "=H(D6;E6)".into(), &format!("{}/kmol", u.energy_unit), h_co2);

    let h_rxn = eval_addin_function(catalog, &mut u, "H", &[ArgValue::Scalar("2H2(g)+O2(g)=2H2O(g)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 6, "4b", "H (Rxn)", "Reaction enthalpy", ExportCellValue::Text("2H2(g)+O2(g)=2H2O(g)".into()), Some(ExportCellValue::Number(100.0)), "=H(D7;E7)".into(), &u.energy_unit, h_rxn);

    let hkg = eval_addin_function(catalog, &mut u, "HKG", &[ArgValue::Scalar("Fe".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 7, "7", "HKG", "Enthalpy per kg of Fe", ExportCellValue::Text("Fe".into()), Some(ExportCellValue::Number(100.0)), "=HKG(D8;E8)".into(), &format!("{}/kg", u.energy_unit), hkg);

    let hnm3 = eval_addin_function(catalog, &mut u, "HNM3", &[ArgValue::Scalar("N2(g)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 8, "6", "HNM3", "Enthalpy per Nm3 of N2(g)", ExportCellValue::Text("N2(g)".into()), Some(ExportCellValue::Number(100.0)), "=HNM3(D9;E9)".into(), &format!("{}/Nm3", u.energy_unit), hnm3);

    let s_h2o = eval_addin_function(catalog, &mut u, "S", &[ArgValue::Scalar("H2O(l)".into()), ArgValue::Scalar("25".into())]);
    let s_h2o_val = s_h2o.as_f64().unwrap_or(0.0);
    push_eval(&mut cells, 9, "8", "S", "Entropy of H2O(l) at 25 C", ExportCellValue::Text("H2O(l)".into()), Some(ExportCellValue::Number(25.0)), "=S(D10;E10)".into(), "J/(mol*K)", s_h2o);

    let g_co2 = eval_addin_function(catalog, &mut u, "G", &[ArgValue::Scalar("CO2(g)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 10, "9", "G", "Gibbs energy of CO2(g)", ExportCellValue::Text("CO2(g)".into()), Some(ExportCellValue::Number(100.0)), "=G(D11;E11)".into(), &format!("{}/kmol", u.energy_unit), g_co2);

    let k_rxn = eval_addin_function(catalog, &mut u, "K", &[ArgValue::Scalar("2H2(g)+O2(g)=2H2O(g)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 11, "10", "K", "Equilibrium constant K(T)", ExportCellValue::Text("2H2(g)+O2(g)=2H2O(g)".into()), Some(ExportCellValue::Number(100.0)), "=K(D12;E12)".into(), "-", k_rxn);

    let hlat = eval_addin_function(catalog, &mut u, "HLAT", &[ArgValue::Scalar("H2O(l)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 12, "11", "HLAT", "Latent-free sensible enthalpy", ExportCellValue::Text("H2O(l)".into()), Some(ExportCellValue::Number(100.0)), "=HLAT(D13;E13)".into(), &format!("{}/kmol", u.energy_unit), hlat);

    let cp_co2 = eval_addin_function(catalog, &mut u, "CP", &[ArgValue::Scalar("CO2(g)".into()), ArgValue::Scalar("100".into())]);
    push_eval(&mut cells, 13, "12", "CP", "Heat capacity of CO2(g)", ExportCellValue::Text("CO2(g)".into()), Some(ExportCellValue::Number(100.0)), "=CP(D14;E14)".into(), "J/(mol*K)", cp_co2);

    let tath = eval_addin_function(catalog, &mut u, "TatH", &[ArgValue::Scalar("CO2(g)".into()), ArgValue::Scalar(format!("{h_co2_val:.6}"))]);
    push_eval(&mut cells, 14, "13", "TatH", "Inverse temperature T(H)", ExportCellValue::Text("CO2(g)".into()), Some(ExportCellValue::Number(h_co2_val)), "=TatH(D15;E15)".into(), &u.temp_unit, tath);

    let tats = eval_addin_function(catalog, &mut u, "TatS", &[ArgValue::Scalar("H2O(l)".into()), ArgValue::Scalar(format!("{s_h2o_val:.6}"))]);
    push_eval(&mut cells, 15, "18", "TatS", "Inverse temperature T(S)", ExportCellValue::Text("H2O(l)".into()), Some(ExportCellValue::Number(s_h2o_val)), "=TatS(D16;E16)".into(), &u.temp_unit, tats);

    let mw = eval_addin_function(catalog, &mut u, "MW", &[ArgValue::Scalar("H2O(l)".into())]);
    push_eval(&mut cells, 16, "21", "MW", "Molecular weight", ExportCellValue::Text("H2O(l)".into()), None, "=MW(D17)".into(), "g/mol", mw);

    let can = eval_addin_function(catalog, &mut u, "CAN", &[ArgValue::Scalar("H2O(l)".into())]);
    push_eval(&mut cells, 17, "29", "CAN", "CAS number", ExportCellValue::Text("H2O(l)".into()), None, "=CAN(D18)".into(), "CAS", can);

    let molp = eval_addin_function(catalog, &mut u, "MolP", &[ArgValue::Scalar("ZnO".into()), ArgValue::Scalar("ZnO*Fe2O3".into())]);
    push_eval(&mut cells, 18, "41", "MolP", "Mole % in compound", ExportCellValue::Text("ZnO".into()), Some(ExportCellValue::Text("ZnO*Fe2O3".into())), "=MolP(D19;E19)".into(), "mol-%", molp);

    let wtp = eval_addin_function(catalog, &mut u, "WTP", &[ArgValue::Scalar("ZnO".into()), ArgValue::Scalar("ZnO*Fe2O3".into())]);
    push_eval(&mut cells, 19, "42", "WTP", "Weight % in compound", ExportCellValue::Text("ZnO".into()), Some(ExportCellValue::Text("ZnO*Fe2O3".into())), "=WTP(D20;E20)".into(), "wt-%", wtp);

    // Multi-row Stream block at Rows 21..23 (1-based rows 22..24: D22:D24 species, E22:E24 amounts, G22=100 C, G23=0, G24=1000)
    let sh_val = eval_addin_function(catalog, &mut u, "StreamH", &[
        ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
        ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
        ArgValue::Scalar("100".into()),
    ]);
    let sth_val = eval_addin_function(catalog, &mut u, "StreamTH", &[
        ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
        ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
        ArgValue::Scalar(format!("{:.6}", sh_val.as_f64().unwrap_or(0.0))),
        ArgValue::Scalar("0".into()),
        ArgValue::Scalar("1000".into()),
    ]);
    let ss_val = eval_addin_function(catalog, &mut u, "StreamS", &[
        ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
        ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
        ArgValue::Scalar("100".into()),
    ]);
    cells.push(ExportCell { row: 20, col: 0, value: ExportCellValue::Text("51".into()), style: ExportCellStyle::Default });
    cells.push(ExportCell { row: 20, col: 1, value: ExportCellValue::Text("StreamH / StreamTH".into()), style: ExportCellStyle::BoldBox });
    cells.push(ExportCell { row: 20, col: 2, value: ExportCellValue::Text("Multi-species range block (D22:E24, T in G22)".into()), style: ExportCellStyle::YellowNote });
    for (offset, (sp, amt, f_str, ev, g_val)) in [
        ("Fe", 1.0, "=StreamH(D22:D24;E22:E24;G22)", sh_val, 100.0),
        ("Cu", 1.0, "=StreamTH(D22:D24;E22:E24;F22;G23;G24)", sth_val, 0.0),
        ("Zn", 1.0, "=StreamS(D22:D24;E22:E24;G22)", ss_val, 1000.0),
    ]
    .into_iter()
    .enumerate()
    {
        let r = 21 + offset;
        cells.push(ExportCell { row: r, col: 3, value: ExportCellValue::Text(sp.into()), style: ExportCellStyle::InputRed });
        cells.push(ExportCell { row: r, col: 4, value: ExportCellValue::Number(amt), style: ExportCellStyle::InputRed });
        cells.push(ExportCell {
            row: r,
            col: 5,
            value: ExportCellValue::Formula { formula: f_str.into(), cached_number: ev.as_f64(), cached_text: None, array_rows: 0 },
            style: ExportCellStyle::Default,
        });
        cells.push(ExportCell { row: r, col: 6, value: ExportCellValue::Number(g_val), style: ExportCellStyle::InputRed });
    }

    // Aqueous DensityA block at Rows 25..26 (1-based 26..27)
    let den_val = eval_addin_function(catalog, &mut u, "DensityA", &[
        ArgValue::Range(vec!["NaCl".into(), "FeSO4".into()]),
        ArgValue::Range(vec!["0.2".into(), "0.1".into()]),
        ArgValue::Scalar("50".into()),
    ]);
    cells.push(ExportCell { row: 24, col: 0, value: ExportCellValue::Text("49".into()), style: ExportCellStyle::Default });
    cells.push(ExportCell { row: 24, col: 1, value: ExportCellValue::Text("DensityA".into()), style: ExportCellStyle::BoldBox });
    cells.push(ExportCell { row: 24, col: 2, value: ExportCellValue::Text("Aqueous electrolyte solution density (D26:E27, T=G26)".into()), style: ExportCellStyle::Default });
    cells.push(ExportCell { row: 25, col: 3, value: ExportCellValue::Text("NaCl".into()), style: ExportCellStyle::InputRed });
    cells.push(ExportCell { row: 25, col: 4, value: ExportCellValue::Number(0.2), style: ExportCellStyle::InputRed });
    cells.push(ExportCell { row: 25, col: 5, value: ExportCellValue::Formula { formula: "=DensityA(D26:D27;E26:E27;G26)".into(), cached_number: den_val.as_f64(), cached_text: None, array_rows: 0 }, style: ExportCellStyle::Default });
    cells.push(ExportCell { row: 25, col: 6, value: ExportCellValue::Number(50.0), style: ExportCellStyle::InputRed });
    cells.push(ExportCell { row: 26, col: 3, value: ExportCellValue::Text("FeSO4".into()), style: ExportCellStyle::InputRed });
    cells.push(ExportCell { row: 26, col: 4, value: ExportCellValue::Number(0.1), style: ExportCellStyle::InputRed });

    // Mineralogy HSC10 functions at rows 27..29 (1-based 28..30)
    let m_zn = eval_addin_function(catalog, &mut u, "MineralElementWt", &[ArgValue::Scalar("Py\\Joki".into()), ArgValue::Scalar("Zn".into())]);
    push_eval(&mut cells, 27, "77", "MineralElementWt", "Zn wt-% in Py\\Joki (Fig. 7)", ExportCellValue::Text("Py\\Joki".into()), Some(ExportCellValue::Text("Zn".into())), "=MineralElementWt(D28;E28)".into(), "wt-%", m_zn);
    let m_den = eval_addin_function(catalog, &mut u, "MineralDensity", &[ArgValue::Scalar("Pyrite".into())]);
    push_eval(&mut cells, 28, "76", "MineralDensity", "Mineral density g/cm3", ExportCellValue::Text("Pyrite".into()), None, "=MineralDensity(D29)".into(), "g/cm3", m_den);
    let m_grp = eval_addin_function(catalog, &mut u, "MineralProperty", &[ArgValue::Scalar("Pyrite".into()), ArgValue::Scalar("Group".into())]);
    push_eval(&mut cells, 29, "74", "MineralProperty", "Mineral crystallographic/chemical group", ExportCellValue::Text("Pyrite".into()), Some(ExportCellValue::Text("Group".into())), "=MineralProperty(D30;E30)".into(), "group", m_grp);

    ExportSheet {
        name: "AddInSample".into(),
        cells,
        col_widths: vec![6.0, 20.0, 44.0, 24.0, 18.0, 18.0, 14.0],
    }
}

/// Compute elemental atom balance `(element, input_kmol, output_kmol)` across a StreamEQ block.
pub fn streameq_element_balance(
    species: &[String],
    inputs: &[String],
    outputs: &[CellEvalValue],
) -> Vec<(String, f64, f64)> {
    let mut totals: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for (idx, sp_raw) in species.iter().enumerate() {
        let sp = sp_raw.trim();
        if sp.is_empty() || sp.starts_with('§') || sp.to_ascii_uppercase().starts_with("PHASE") {
            continue;
        }
        let Ok(comp) = parse_formula_composition(sp) else { continue; };
        let in_mol = inputs.get(idx).and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(0.0).max(0.0);
        let out_mol = outputs.get(idx).and_then(|v| v.as_f64()).unwrap_or(0.0).max(0.0);
        for (el, coeff) in comp {
            if el == "e-" {
                continue;
            }
            let entry = totals.entry(el).or_insert((0.0, 0.0));
            entry.0 += in_mol * coeff;
            entry.1 += out_mol * coeff;
        }
    }
    totals.into_iter().map(|(el, (i, o))| (el, i, o)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spreadsheet_export::{
        build_libreoffice_oxt, build_ods, build_onlyoffice_plugin_zip, build_xlsx,
        generate_excel_wps_vba,
    };
    use crate::{KelleyCoefficients, Phase, SpeciesRepository, SpeciesThermoInterval};
    use soryntel_core::CAL_TO_JOULE;

    struct TestRepo {
        map: BTreeMap<String, ChemicalSpecies>,
    }

    impl TestRepo {
        fn sample() -> Self {
            let mut map = BTreeMap::new();
            let mut add = |formula: &str, mw: f64, den: f64, phase: Phase, hf_kj: f64, s_j: f64, a: f64, b: f64, c: f64, d: f64| {
                map.insert(
                    formula.to_string(),
                    ChemicalSpecies {
                        formula: formula.to_string(),
                        name: formula.to_string(),
                        phase,
                        molecular_weight: mw,
                        density_g_cm3: den,
                        charge: 0,
                        intervals: vec![SpeciesThermoInterval {
                            t_min: 200.0,
                            t_max: 3000.0,
                            phase,
                            kelley: KelleyCoefficients { a, b, c, d },
                            h_or_delta_h_tr: hf_kj * 1000.0,
                            s_or_delta_s_tr: s_j,
                            reference: "TestDB".into(),
                        }],
                    },
                );
            };
            add("CO2(g)", 44.01, 0.0, Phase::Gas, -393.51, 213.785, 44.14, 9.04, -8.53, 0.0);
            add("CO(g)", 28.01, 0.0, Phase::Gas, -110.53, 197.66, 28.41, 4.10, -0.46, 0.0);
            add("C", 12.011, 2.26, Phase::Solid, 0.0, 5.74, 16.86, 4.77, -8.54, 0.0);
            add("N2(g)", 28.013, 0.0, Phase::Gas, 0.0, 191.61, 27.87, 4.27, 0.0, 0.0);
            add("O2(g)", 31.999, 0.0, Phase::Gas, 0.0, 205.15, 29.96, 4.18, -1.67, 0.0);
            add("NO(g)", 30.006, 0.0, Phase::Gas, 90.29, 210.76, 29.40, 3.85, 0.0, 0.0);
            add("H2(g)", 2.016, 0.0, Phase::Gas, 0.0, 130.68, 27.28, 3.26, 0.50, 0.0);
            add("H2O(g)", 18.015, 0.0, Phase::Gas, -241.826, 188.835, 30.00, 10.71, 0.33, 0.0);
            add("H2O(l)", 18.015, 0.997, Phase::Liquid, -285.83, 69.95, 75.3, 0.0, 0.0, 0.0);
            add("Fe", 55.845, 7.87, Phase::Solid, 0.0, 27.3, 25.0, 6.0, 0.0, 0.0);
            add("Cu", 63.546, 8.96, Phase::Solid, 0.0, 33.15, 24.4, 3.0, 0.0, 0.0);
            add("Zn", 65.38, 7.14, Phase::Solid, 0.0, 41.6, 25.4, 5.0, 0.0, 0.0);
            Self { map }
        }
    }

    impl SpeciesRepository for TestRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.map.get(formula).cloned()
        }
        fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
            self.map.get(element_symbol).cloned()
        }
    }

    impl SpeciesCatalogPort for TestRepo {
        fn search_species(&self, q: &str, limit: usize) -> Vec<ChemicalSpecies> {
            let ql = q.to_ascii_lowercase();
            self.map.values().filter(|s| ql.is_empty() || s.formula.to_ascii_lowercase().contains(&ql)).take(limit).cloned().collect()
        }
        fn search_species_by_elements(&self, _: &[String], _: bool, _: bool, _: bool, _: bool, _: bool, _: usize) -> Vec<ChemicalSpecies> {
            self.map.values().cloned().collect()
        }
        fn insert_or_update_species(&mut self, sp: ChemicalSpecies) {
            self.map.insert(sp.formula.clone(), sp);
        }
        fn species_count(&self) -> usize {
            self.map.len()
        }
    }

    #[test]
    fn excel_addins_validation_x1_to_x8() {
        let repo = TestRepo::sample();
        let mut units = AddInUnits::default();

        // X1: exact 69 HSC6.XLL functions + Minerals + HSC-Sim
        let xll_count = ADDIN_FUNCTION_CATALOG.iter().filter(|f| f.origin == AddInOrigin::Hsc6Xll).count();
        assert_eq!(xll_count, 69);

        let u_res = eval_formula_string(&repo, &mut units, "=UNITS(\"C\";\"MJ\")", |_, _| String::new());
        assert_eq!(u_res, CellEvalValue::Text("C and MJ".into()));

        let bal_res = eval_formula_string(&repo, &mut units, "=BAL(\"H2(g)+O2(g)=H2O(g)\")", |_, _| String::new());
        match bal_res {
            CellEvalValue::Text(s) => assert!(s.contains("2H2(g)") && s.contains("2H2O(g)"), "bal={s}"),
            other => panic!("unexpected BAL {other:?}"),
        }

        // X2 & X3: Species + Reaction H, K, HLAT
        let h_co2 = eval_formula_string(&repo, &mut units, "=H(\"CO2(g)\";100)", |_, _| String::new()).as_f64().unwrap();
        assert!((h_co2 - (-390.6)).abs() < 1.5, "h_co2={h_co2}");
        let hlat_w = eval_formula_string(&repo, &mut units, "=HLAT(\"H2O(l)\";25)", |_, _| String::new()).as_f64().unwrap();
        assert!(hlat_w.abs() < 1e-6);

        let h_rxn = eval_formula_string(&repo, &mut units, "=H(\"2H2(g)+O2(g)=2H2O(g)\";100)", |_, _| String::new()).as_f64().unwrap();
        assert!((h_rxn - (-485.1)).abs() < 2.5, "h_rxn={h_rxn}");

        // Check Mcal scaling
        let _ = eval_formula_string(&repo, &mut units, "=UNITS(\"C\";\"Mcal\")", |_, _| String::new());
        let h_co2_mcal = eval_formula_string(&repo, &mut units, "=H(\"CO2(g)\";100)", |_, _| String::new()).as_f64().unwrap();
        assert!(((h_co2 / CAL_TO_JOULE) - h_co2_mcal).abs() < 1e-4);
        let _ = eval_formula_string(&repo, &mut units, "=UNITS(\"C\";\"MJ\")", |_, _| String::new());

        // X4: TatH round-trip
        let tat = eval_addin_function(&repo, &mut units, "TatH", &[ArgValue::Scalar("CO2(g)".into()), ArgValue::Scalar(format!("{h_co2}"))]).as_f64().unwrap();
        assert!((tat - 100.0).abs() < 1e-3, "tat={tat}");

        // X5 & X6: StreamH + StreamTH + out-of-range #VALUE! + DensityA
        let sh = eval_addin_function(&repo, &mut units, "StreamH", &[
            ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
            ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
            ArgValue::Scalar("100".into()),
        ]).as_f64().unwrap();
        assert!(sh > 5.0 && sh < 7.0, "sh={sh}");

        let sth_ok = eval_addin_function(&repo, &mut units, "StreamTH", &[
            ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
            ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
            ArgValue::Scalar(format!("{sh}")),
            ArgValue::Scalar("0".into()),
            ArgValue::Scalar("1000".into()),
        ]).as_f64().unwrap();
        assert!((sth_ok - 100.0).abs() < 1e-3);

        let sth_oor = eval_addin_function(&repo, &mut units, "StreamTH", &[
            ArgValue::Range(vec!["Fe".into(), "Cu".into(), "Zn".into()]),
            ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
            ArgValue::Scalar(format!("{sh}")),
            ArgValue::Scalar("200".into()),
            ArgValue::Scalar("1000".into()),
        ]);
        assert_eq!(sth_oor, CellEvalValue::Error("#VALUE!".into()));

        let den = eval_addin_function(&repo, &mut units, "DensityA", &[
            ArgValue::Range(vec!["NaCl".into(), "FeSO4".into()]),
            ArgValue::Range(vec!["0.2".into(), "0.1".into()]),
            ArgValue::Scalar("50".into()),
        ]).as_f64().unwrap();
        assert!(den > 1200.0 && den < 1320.0, "den={den}");

        // X7: MolP / WTP + Mineralogy Fig. 7
        let molp = eval_addin_function(&repo, &mut units, "MolP", &[ArgValue::Scalar("ZnO".into()), ArgValue::Scalar("ZnO*Fe2O3".into())]).as_f64().unwrap();
        let wtp = eval_addin_function(&repo, &mut units, "WTP", &[ArgValue::Scalar("ZnO".into()), ArgValue::Scalar("ZnO*Fe2O3".into())]).as_f64().unwrap();
        assert!((molp - 28.571428).abs() < 0.01, "molp={molp}");
        assert!((wtp - 33.757).abs() < 0.1, "wtp={wtp}");

        assert_eq!(eval_addin_function(&repo, &mut units, "MineralCode", &[ArgValue::Scalar("Pyrite".into())]), CellEvalValue::Text("Py".into()));
        assert_eq!(eval_addin_function(&repo, &mut units, "MineralFormula", &[ArgValue::Scalar("Galena".into())]), CellEvalValue::Text("PbS".into()));
        let s_py = eval_addin_function(&repo, &mut units, "MineralC", &[ArgValue::Scalar("Py\\Joki".into()), ArgValue::Scalar("S".into())]).as_f64().unwrap();
        assert!((s_py - 53.29).abs() < 1e-6);

        // X8: StreamEQ combustion Fig. 6 (1500 C, 1 bar) + HSC5.xla migration + multi-suite exports
        let sp_rows: Vec<String> = ["§ PHASE 1:", "CO(g)", "CO2(g)", "N2(g)", "NO(g)", "O2(g)", "§ PHASE 2:", "C"]
            .into_iter().map(str::to_string).collect();
        let in_rows: Vec<String> = ["25", "0", "0", "3.02", "0", "0.8", "25", "1"]
            .into_iter().map(str::to_string).collect();
        let eq = eval_streameq(&repo, &units, &sp_rows, &in_rows, 1500.0, 1.0);
        if let CellEvalValue::Array(arr) = eq {
            let co = arr[1].as_f64().unwrap();
            let co2 = arr[2].as_f64().unwrap();
            let c_solid = arr[7].as_f64().unwrap();
            assert!((co + co2 - 1.0).abs() < 1e-4, "C balance failed: co={co} co2={co2}");
            assert!(c_solid < 1e-4, "C should be consumed at 1500C");
        } else {
            panic!("expected StreamEQ array, got {eq:?}");
        }

        let (mig, notes) = migrate_legacy_formula("='C:\\HSC5\\AddIns_BackUp\\HSC5.xla'!H(D11;E11)");
        assert_eq!(mig, "=H(D11;E11)");
        assert!(!notes.is_empty());

        let sh_eq = build_streameq_export_sheet(&repo, &units, &sp_rows, &in_rows, 1500.0, 1.0);
        let sh_min = build_minerals_export_sheet();
        let sh_samp = build_addinsample_export_sheet(&repo, &units);
        let xlsx = build_xlsx(&[sh_eq.clone(), sh_min.clone(), sh_samp.clone()]);
        let ods = build_ods(&[sh_eq, sh_min, sh_samp]);
        let oxt = build_libreoffice_oxt(8765);
        let onlyoffice = build_onlyoffice_plugin_zip(8765);
        let vba = generate_excel_wps_vba(8765);
        assert!(xlsx.starts_with(b"PK\x03\x04"));
        assert!(ods.starts_with(b"PK\x03\x04"));
        assert!(oxt.starts_with(b"PK\x03\x04"));
        assert!(onlyoffice.starts_with(b"PK\x03\x04"));
        assert!(vba.contains("Public Function MineralProperty"));
        assert!(vba.contains("Public Function EnergyU"));
    }

    #[test]
    fn hsc10_manual_parity_and_audit_fixes() {
        let repo = TestRepo::sample();
        let mut units = AddInUnits::default();

        // 1. EnergyU / TempU / PresU (Ch. 30 Table 1)
        assert_eq!(eval_formula_string(&repo, &mut units, "=EnergyU()", |_, _| String::new()), CellEvalValue::Text("MJ".into()));
        assert_eq!(eval_formula_string(&repo, &mut units, "=TempU()", |_, _| String::new()), CellEvalValue::Text("C".into()));
        assert_eq!(eval_formula_string(&repo, &mut units, "=PresU()", |_, _| String::new()), CellEvalValue::Text("bar".into()));
        let _ = eval_formula_string(&repo, &mut units, "=UNITS(\"K\";\"kcal\")", |_, _| String::new());
        assert_eq!(eval_formula_string(&repo, &mut units, "=EnergyU()", |_, _| String::new()), CellEvalValue::Text("kcal".into()));
        assert_eq!(eval_formula_string(&repo, &mut units, "=TempU()", |_, _| String::new()), CellEvalValue::Text("K".into()));
        units = AddInUnits::default();

        // 2. BAL optional multiplier
        let bal_half = eval_formula_string(&repo, &mut units, "=BAL(\"C + O2(g) = CO(g)\"; 0.5)", |_, _| String::new());
        match bal_half {
            CellEvalValue::Text(s) => assert!(s.contains("0.5000O2(g)"), "expected 0.5000O2(g) in {s}"),
            other => panic!("unexpected BAL with mult: {other:?}"),
        }

        // 3. Py\Joki Zn=0.02 + all HSC10 Ch. 30.5 Mineral functions + Group
        let zn_joki = eval_addin_function(&repo, &mut units, "MineralElementWt", &[ArgValue::Scalar("Py\\Joki".into()), ArgValue::Scalar("Zn".into())]).as_f64().unwrap();
        assert!((zn_joki - 0.02).abs() < 1e-6, "Py\\Joki Zn={zn_joki}");
        assert_eq!(eval_addin_function(&repo, &mut units, "MineralColor", &[ArgValue::Scalar("Pyrite".into())]), CellEvalValue::Text("yellow, pale brass".into()));
        assert_eq!(eval_addin_function(&repo, &mut units, "MineralLuster", &[ArgValue::Scalar("Pyrite".into())]), CellEvalValue::Text("Metallic".into()));
        assert!((eval_addin_function(&repo, &mut units, "MineralDensity", &[ArgValue::Scalar("Pyrite".into())]).as_f64().unwrap() - 5.01).abs() < 1e-6);
        assert!((eval_addin_function(&repo, &mut units, "MineralSG", &[ArgValue::Scalar("Pyrite".into())]).as_f64().unwrap() - 5.01).abs() < 1e-6);
        assert!((eval_addin_function(&repo, &mut units, "MineralHardness", &[ArgValue::Scalar("Pyrite".into())]).as_f64().unwrap() - 6.5).abs() < 1e-6);
        assert_eq!(eval_addin_function(&repo, &mut units, "MineralProperty", &[ArgValue::Scalar("Pyrite".into()), ArgValue::Scalar("Group".into())]), CellEvalValue::Text("Pyrite".into()));

        // 4. Nested call + arithmetic in eval_formula_string
        let h_nested = eval_formula_string(&repo, &mut units, "=H(\"CO2(g)\";200) - H(\"CO2(g)\";100)", |_, _| String::new()).as_f64().unwrap();
        assert!(h_nested > 3.0 && h_nested < 6.0, "dH(200-100)={h_nested}");
        let sp1 = eval_formula_string(&repo, &mut units, "=H(SPECIES(2;1);100)", |_, _| String::new()).as_f64();
        assert!(sp1.is_some());

        // 5. StreamEQ §§ multi-species pure phase stays pure (does not crash and solves cleanly)
        let sp_pure: Vec<String> = ["§ GAS:", "CO(g)", "CO2(g)", "O2(g)", "§§ PURE_SOLIDS:", "C", "Fe"]
            .into_iter().map(str::to_string).collect();
        let in_pure: Vec<String> = ["1500", "0", "0", "0.5", "25", "1", "1"]
            .into_iter().map(str::to_string).collect();
        let eq_pure = eval_streameq(&repo, &units, &sp_pure, &in_pure, 1000.0, 1.0);
        assert!(matches!(eq_pure, CellEvalValue::Array(_)), "got {eq_pure:?}");

        // 6. Export sheet StreamEQ only emits Formula on row 0 (no overlapping CSE formulas)
        let sh_eq = build_streameq_export_sheet(&repo, &units, &sp_pure, &in_pure, 1000.0, 1.0);
        let eq_formula_cells: Vec<_> = sh_eq.cells.iter().filter(|c| matches!(c.value, ExportCellValue::Formula { .. })).collect();
        assert_eq!(eq_formula_cells.len(), 1, "expected exactly 1 array anchor cell in StreamEQ");
        let sh_min = build_minerals_export_sheet();
        assert!(sh_min.cells.iter().any(|c| c.col == 5 && c.row == 3), "Mineral 5 col must be populated");

        // 7. DensityA respects Kelvin units, StreamEQ detects unknown species (#NAME?), SUM evaluates formula cells
        let den_c = eval_addin_function(&repo, &mut units, "DensityA", &[
            ArgValue::Range(vec!["NaCl".into(), "FeSO4".into()]),
            ArgValue::Range(vec!["0.2".into(), "0.1".into()]),
            ArgValue::Scalar("50".into()),
        ]).as_f64().unwrap();
        let mut units_k = AddInUnits { temp_unit: "K".into(), ..AddInUnits::default() };
        let den_k = eval_addin_function(&repo, &mut units_k, "DensityA", &[
            ArgValue::Range(vec!["NaCl".into(), "FeSO4".into()]),
            ArgValue::Range(vec!["0.2".into(), "0.1".into()]),
            ArgValue::Scalar("323.15".into()),
        ]).as_f64().unwrap();
        assert!((den_c - den_k).abs() < 1e-6, "DensityA Kelvin mismatch: C={den_c}, K={den_k}");

        let unk_sp: Vec<String> = ["§ GAS:", "FAKE_SPECIES_XYZ"].into_iter().map(str::to_string).collect();
        let unk_in: Vec<String> = ["25", "1"].into_iter().map(str::to_string).collect();
        assert_eq!(eval_streameq(&repo, &units, &unk_sp, &unk_in, 1000.0, 1.0), CellEvalValue::Error("#NAME?".into()));
        assert_eq!(eval_streameq(&repo, &units, &sp_pure, &in_pure, 1000.0, 0.0), CellEvalValue::Error("#VALUE!".into()));

        let sum_formulas = eval_formula_string(&repo, &mut units, "=SUM(A1:A2)", |r, _| match r {
            0 => "=10+5".into(),
            1 => "=25".into(),
            _ => String::new(),
        }).as_f64().unwrap();
        assert!((sum_formulas - 40.0).abs() < 1e-9, "sum_formulas={sum_formulas}");

        // 8. Reaction*/MolecularWeight aliases and catalog dispatch coverage for all 83 active functions
        let g_official = eval_addin_function(&repo, &mut units, "G", &[
            ArgValue::Scalar("C + O2(g) = CO2(g)".into()),
            ArgValue::Scalar("1000".into()),
        ]).as_f64().unwrap();
        let g_alias = eval_addin_function(&repo, &mut units, "ReactionG", &[
            ArgValue::Scalar("C + O2(g) = CO2(g)".into()),
            ArgValue::Scalar("1000".into()),
        ]).as_f64().unwrap();
        assert!((g_official - g_alias).abs() < 1e-9, "ReactionG alias mismatch");

        let mw_official = eval_addin_function(&repo, &mut units, "MW", &[ArgValue::Scalar("CO2(g)".into())]).as_f64().unwrap();
        let mw_alias = eval_addin_function(&repo, &mut units, "MolecularWeight", &[ArgValue::Scalar("CO2(g)".into())]).as_f64().unwrap();
        assert!((mw_official - mw_alias).abs() < 1e-9, "MolecularWeight alias mismatch");

        // Ensure every active catalog function hits a defined handler (not the catch-all unknown-function branch)
        let active_metas: Vec<_> = ADDIN_FUNCTION_CATALOG
            .iter()
            .filter(|m| m.origin != AddInOrigin::HscSimOnly)
            .collect();
        assert_eq!(active_metas.len(), 83, "expected 83 active functions");
        for meta in active_metas {
            // Passing empty args returns either a value (UNITS/EnergyU/TempU/PresU/SPECIES) or #VALUE!/#N/A,
            // never #NAME? from an unhandled function name match arm.
            let probe = eval_addin_function(&repo, &mut units, meta.name, &[]);
            assert_ne!(
                probe,
                CellEvalValue::Error("#NAME?".into()),
                "active function {} fell through to unknown function handler",
                meta.name
            );
        }
    }
}
