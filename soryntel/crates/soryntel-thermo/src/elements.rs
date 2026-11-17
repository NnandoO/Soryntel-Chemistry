//! Module 12 — Elements Module (Periodic Table & Element Properties Database `Element4`).
//!
//! Provides comprehensive physical, chemical, structural, and thermochemical properties
//! for elements Z = 1..=103 (HSC6 `ELEMENT4.ELE` compatibility), proportional sphere
//! periodic diagram normalization, editable spreadsheet rows, CSV/JSON serialization,
//! and `SpeciesCatalogPort` cross-lookup.

use crate::{ChemicalSpecies, SpeciesCatalogPort};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ElementCategory {
    Alkali,
    Alkaline,
    TransitionMetal,
    PostTransitionMetal,
    Metalloid,
    NonMetal,
    Halogen,
    NobleGas,
    Lanthanide,
    Actinide,
}

impl ElementCategory {
    pub const LEGEND_ORDER: [ElementCategory; 10] = [
        Self::Alkali,
        Self::Alkaline,
        Self::TransitionMetal,
        Self::PostTransitionMetal,
        Self::Metalloid,
        Self::NonMetal,
        Self::Halogen,
        Self::NobleGas,
        Self::Lanthanide,
        Self::Actinide,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Alkali => "Alkali",
            Self::Alkaline => "Alkaline",
            Self::TransitionMetal => "Transition\nmetals",
            Self::PostTransitionMetal => "Post-transition\nmetals",
            Self::Metalloid => "Metalloid",
            Self::NonMetal => "Non-metal",
            Self::Halogen => "Halogens",
            Self::NobleGas => "Noble gases",
            Self::Lanthanide => "Lanthanides",
            Self::Actinide => "Actinides",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StandardState {
    Solid,
    Liquid,
    Gas,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementStaticSpec {
    pub z: u16,
    pub symbol: &'static str,
    pub name: &'static str,
    pub atomic_weight: f64,
    pub category: ElementCategory,
    pub state: StandardState,
    /// Visual row in standard 18-column table (0..6 main, 7 Lanthanides, 8 Actinides)
    pub grid_row: usize,
    /// Visual column (0..17)
    pub grid_col: usize,
    pub most_stable_ox: &'static str,
    pub ox_states: &'static str,
    pub density: Option<f64>, // g/ml (HSC stores gases in g/L as 0.0899 etc.)
    pub electron_config: &'static str,
    pub melting_k: Option<f64>,
    pub melting_p_atm: Option<f64>,
    pub boiling_k: Option<f64>,
    pub electronegativity: Option<f64>,
    pub heat_vap_kj: Option<f64>,
    pub heat_fus_kj: Option<f64>,
    pub elec_cond: Option<f64>,    // E6/(ohm*cm)
    pub therm_cond: Option<f64>,   // W/(cm*K)
    pub spec_heat: Option<f64>,    // J/(g*K)
    pub ionization_ev: Option<f64>,
    pub atomic_vol: Option<f64>,   // ml/mol
    pub atomic_rad: Option<f64>,   // A
    pub covalent_rad: Option<f64>, // A
    pub crystal: &'static str,
    pub acid_base: &'static str,
    pub year_discovered: &'static str,
    pub color_name: &'static str,
    pub color_rgb: u32,
    pub electron_affinity_ev: Option<f64>,
    pub polarizability: Option<f64>, // 1e-24 cm3
    pub dh_gas_kj: Option<f64>,
    pub dh_solid_kj: Option<f64>,
    pub s_gas_jk: Option<f64>,
    pub s_solid_jk: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementPropertyRow {
    pub property: String,
    pub units: String,
    pub diagram: bool,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementWorkbook {
    pub element_symbols: Vec<String>,
    pub rows: Vec<ElementPropertyRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpherePoint {
    pub z: u16,
    pub symbol: String,
    pub raw_value: Option<f64>,
    /// Linear normalized diameter ratio in `[0.0, 1.0]` (`|v| / max|v|`).
    pub norm_diameter: f64,
    pub is_negative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicDiagramData {
    pub property_name: String,
    pub units: String,
    pub min_val: f64,
    pub max_val: f64,
    pub max_abs: f64,
    pub mean_val: f64,
    pub valid_count: usize,
    pub points: Vec<SpherePoint>,
}

impl ElementWorkbook {
    pub fn default_workbook() -> Self {
        let specs = PERIODIC_ELEMENTS;
        let symbols: Vec<String> = specs.iter().map(|e| e.symbol.to_string()).collect();

        let fmt_opt = |v: Option<f64>, decimals: usize| -> String {
            match v {
                Some(x) if x.abs() >= 1e5 || (x != 0.0 && x.abs() < 1e-4) => format!("{x:.2E}"),
                Some(x) => {
                    let s = format!("{:.*}", decimals, x);
                    if s.contains('.') {
                        s.trim_end_matches('0').trim_end_matches('.').to_string()
                    } else {
                        s
                    }
                }
                None => "-".to_string(),
            }
        };

        let mut rows = Vec::new();
        let push_row = |rows: &mut Vec<ElementPropertyRow>,
                        prop: &str,
                        units: &str,
                        diag: bool,
                        vals: Vec<String>| {
            rows.push(ElementPropertyRow {
                property: prop.to_string(),
                units: units.to_string(),
                diagram: diag,
                values: vals,
            });
        };

        push_row(
            &mut rows,
            "Atomic Number",
            "",
            true,
            specs.iter().map(|e| e.z.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Symbol",
            "",
            false,
            specs.iter().map(|e| e.symbol.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Name",
            "",
            false,
            specs.iter().map(|e| e.name.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Atomic Weight",
            "g/mol",
            true,
            specs.iter().map(|e| fmt_opt(Some(e.atomic_weight), 5)).collect(),
        );
        push_row(
            &mut rows,
            "Oxidation States, Most Stable",
            "",
            false,
            specs.iter().map(|e| e.most_stable_ox.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Oxidation States",
            "",
            false,
            specs.iter().map(|e| e.ox_states.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Density",
            "g/ml",
            true,
            specs.iter().map(|e| fmt_opt(e.density, 4)).collect(),
        );
        push_row(
            &mut rows,
            "Electron Configuration",
            "",
            false,
            specs.iter().map(|e| e.electron_config.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Melting Point",
            "K",
            true,
            specs.iter().map(|e| fmt_opt(e.melting_k, 3)).collect(),
        );
        push_row(
            &mut rows,
            "Melting Point Pressure",
            "atm",
            false,
            specs.iter().map(|e| fmt_opt(e.melting_p_atm, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Boiling Point",
            "K",
            true,
            specs.iter().map(|e| fmt_opt(e.boiling_k, 3)).collect(),
        );
        push_row(
            &mut rows,
            "Electronegativity",
            "",
            true,
            specs.iter().map(|e| fmt_opt(e.electronegativity, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Heat of Vaporization",
            "kJ/mol",
            true,
            specs.iter().map(|e| fmt_opt(e.heat_vap_kj, 5)).collect(),
        );
        push_row(
            &mut rows,
            "Heat of Fusion",
            "kJ/mol",
            true,
            specs.iter().map(|e| fmt_opt(e.heat_fus_kj, 5)).collect(),
        );
        push_row(
            &mut rows,
            "Electrical Conductivity",
            "E6/(ohm*cm)",
            true,
            specs.iter().map(|e| fmt_opt(e.elec_cond, 3)).collect(),
        );
        push_row(
            &mut rows,
            "Thermal Conductivity",
            "W/(cm*K)",
            true,
            specs.iter().map(|e| fmt_opt(e.therm_cond, 6)).collect(),
        );
        push_row(
            &mut rows,
            "Specific Heat Capacity",
            "J/(g*K)",
            true,
            specs.iter().map(|e| fmt_opt(e.spec_heat, 3)).collect(),
        );
        push_row(
            &mut rows,
            "First Ionization Potential",
            "V",
            true,
            specs.iter().map(|e| fmt_opt(e.ionization_ev, 3)).collect(),
        );
        push_row(
            &mut rows,
            "Atomic Volume",
            "ml/mol",
            true,
            specs.iter().map(|e| fmt_opt(e.atomic_vol, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Atomic Radius",
            "A",
            true,
            specs.iter().map(|e| fmt_opt(e.atomic_rad, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Covalent Radius",
            "A",
            true,
            specs.iter().map(|e| fmt_opt(e.covalent_rad, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Crystal Structure",
            "",
            false,
            specs.iter().map(|e| e.crystal.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Acid-Base Properties",
            "",
            false,
            specs.iter().map(|e| e.acid_base.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Year when discovered",
            "",
            false,
            specs.iter().map(|e| e.year_discovered.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Color",
            "",
            false,
            specs.iter().map(|e| e.color_name.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Color RGB-number",
            "RGB",
            false,
            specs.iter().map(|e| e.color_rgb.to_string()).collect(),
        );
        push_row(
            &mut rows,
            "Atomic Electron Affinity",
            "eV",
            true,
            specs.iter().map(|e| fmt_opt(e.electron_affinity_ev, 3)).collect(),
        );
        push_row(
            &mut rows,
            "Polarizability",
            "E-24 cm3",
            true,
            specs.iter().map(|e| fmt_opt(e.polarizability, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Enthalpy of Formation 298.15 K Gas",
            "kJ/mol",
            true,
            specs.iter().map(|e| fmt_opt(e.dh_gas_kj, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Enthalpy of Formation 298.15 K Solid",
            "kJ/mol",
            true,
            specs.iter().map(|e| fmt_opt(e.dh_solid_kj, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Standard Entropy 298.15 K Gas",
            "J/(mol*K)",
            true,
            specs.iter().map(|e| fmt_opt(e.s_gas_jk, 2)).collect(),
        );
        push_row(
            &mut rows,
            "Standard Entropy 298.15 K Solid",
            "J/(mol*K)",
            true,
            specs.iter().map(|e| fmt_opt(e.s_solid_jk, 2)).collect(),
        );

        Self {
            element_symbols: symbols,
            rows,
        }
    }

    pub fn diagram_property_indices(&self) -> Vec<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| if r.diagram { Some(i) } else { None })
            .collect()
    }

    pub fn add_custom_row(&mut self, property: String, units: String, diagram: bool) {
        let n = self.element_symbols.len();
        self.rows.push(ElementPropertyRow {
            property,
            units,
            diagram,
            values: vec!["-".to_string(); n],
        });
    }

    pub fn compute_diagram(&self, row_idx: usize) -> Result<PeriodicDiagramData, String> {
        let row = self
            .rows
            .get(row_idx)
            .ok_or_else(|| "Selected property row out of range.".to_string())?;
        let mut parsed = Vec::with_capacity(self.element_symbols.len());
        let mut min_val = f64::INFINITY;
        let mut max_val = f64::NEG_INFINITY;
        let mut max_abs = 0.0_f64;
        let mut sum = 0.0_f64;
        let mut valid_count = 0usize;

        for (idx, sym) in self.element_symbols.iter().enumerate() {
            let z = PERIODIC_ELEMENTS.get(idx).map(|e| e.z).unwrap_or((idx + 1) as u16);
            let raw = row
                .values
                .get(idx)
                .and_then(|s| parse_numeric_cell(s));
            if let Some(v) = raw {
                if v < min_val {
                    min_val = v;
                }
                if v > max_val {
                    max_val = v;
                }
                if v.abs() > max_abs {
                    max_abs = v.abs();
                }
                sum += v;
                valid_count += 1;
            }
            parsed.push((z, sym.clone(), raw));
        }

        if valid_count == 0 {
            return Err(format!("No numeric values in property '{}'.", row.property));
        }

        let denom = if max_abs > 0.0 { max_abs } else { 1.0 };
        let points = parsed
            .into_iter()
            .map(|(z, symbol, raw_value)| {
                let (norm_diameter, is_negative) = match raw_value {
                    Some(v) => ((v.abs() / denom).clamp(0.0, 1.0), v < 0.0),
                    None => (0.0, false),
                };
                SpherePoint {
                    z,
                    symbol,
                    raw_value,
                    norm_diameter,
                    is_negative,
                }
            })
            .collect();

        Ok(PeriodicDiagramData {
            property_name: row.property.clone(),
            units: row.units.clone(),
            min_val,
            max_val,
            max_abs,
            mean_val: sum / valid_count as f64,
            valid_count,
            points,
        })
    }

    pub fn to_csv(&self) -> String {
        let mut out = String::from("Property,Units,Diagram");
        for sym in &self.element_symbols {
            out.push(',');
            out.push_str(sym);
        }
        out.push('\n');
        for r in &self.rows {
            out.push_str(&csv_escape(&r.property));
            out.push(',');
            out.push_str(&csv_escape(&r.units));
            out.push(',');
            out.push_str(if r.diagram { "TRUE" } else { "FALSE" });
            for val in &r.values {
                out.push(',');
                out.push_str(&csv_escape(val));
            }
            out.push('\n');
        }
        out
    }
}

fn parse_numeric_cell(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" || t.eq_ignore_ascii_case("N/A") {
        return None;
    }
    t.replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite())
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub fn lookup_element_species(
    catalog: &dyn SpeciesCatalogPort,
    symbol: &str,
) -> Vec<ChemicalSpecies> {
    catalog.search_species_by_elements(
        &[symbol.to_string()],
        true,
        true,
        true,
        true,
        true,
        40,
    )
}

pub fn find_element_spec(z: u16) -> Option<&'static ElementStaticSpec> {
    PERIODIC_ELEMENTS.iter().find(|e| e.z == z)
}

pub fn find_element_index_by_symbol(sym: &str) -> Option<usize> {
    let t = sym.trim();
    PERIODIC_ELEMENTS
        .iter()
        .position(|e| e.symbol.eq_ignore_ascii_case(t) || e.name.eq_ignore_ascii_case(t))
}

// Helper macro to keep the 103-element dataset readable and compact without losing precision.
macro_rules! elem {
    (
        $z:expr, $sym:expr, $name:expr, $aw:expr, $cat:ident, $st:ident, ($gr:expr, $gc:expr),
        $oxm:expr, $ox:expr, $dens:expr, $cfg:expr,
        $tm:expr, $pm:expr, $tb:expr, $en:expr, $hv:expr, $hf:expr,
        $ec:expr, $tc:expr, $cp:expr, $ip:expr, $av:expr, $ar:expr, $cr:expr,
        $crys:expr, $ab:expr, $yr:expr, $col:expr, $rgb:expr,
        $ea:expr, $pol:expr, $dhg:expr, $dhs:expr, $sg:expr, $ss:expr
    ) => {
        ElementStaticSpec {
            z: $z,
            symbol: $sym,
            name: $name,
            atomic_weight: $aw,
            category: ElementCategory::$cat,
            state: StandardState::$st,
            grid_row: $gr,
            grid_col: $gc,
            most_stable_ox: $oxm,
            ox_states: $ox,
            density: $dens,
            electron_config: $cfg,
            melting_k: $tm,
            melting_p_atm: $pm,
            boiling_k: $tb,
            electronegativity: $en,
            heat_vap_kj: $hv,
            heat_fus_kj: $hf,
            elec_cond: $ec,
            therm_cond: $tc,
            spec_heat: $cp,
            ionization_ev: $ip,
            atomic_vol: $av,
            atomic_rad: $ar,
            covalent_rad: $cr,
            crystal: $crys,
            acid_base: $ab,
            year_discovered: $yr,
            color_name: $col,
            color_rgb: $rgb,
            electron_affinity_ev: $ea,
            polarizability: $pol,
            dh_gas_kj: $dhg,
            dh_solid_kj: $dhs,
            s_gas_jk: $sg,
            s_solid_jk: $ss,
        }
    };
}

pub const PERIODIC_ELEMENTS: &[ElementStaticSpec] = &[
    // Period 1
    elem!(1, "H", "Hydrogen", 1.00794, NonMetal, Gas, (0, 0), "1", "1", Some(0.0899), "1s1", Some(14.025), Some(1.0), Some(20.268), Some(2.2), Some(0.44936), Some(0.05868), None, Some(0.001815), Some(14.304), Some(13.598), Some(14.4), Some(0.79), Some(0.32), "hexagonal", "amphoteric", "1766", "colorless", 16777215, Some(0.754), Some(0.667), Some(217.99), None, Some(114.72), None),
    elem!(2, "He", "Helium", 4.002602, NobleGas, Gas, (0, 17), "-", "-", Some(0.1787), "1s2", Some(0.95), Some(26.0), Some(4.215), None, Some(0.0845), None, None, Some(0.00152), Some(5.193), Some(24.587), None, Some(0.49), Some(0.93), "hexagonal", "-", "1868", "colorless", 16777215, None, Some(0.205), Some(0.0), None, Some(126.15), None),
    // Period 2
    elem!(3, "Li", "Lithium", 6.941, Alkali, Solid, (1, 0), "1", "1", Some(0.53), "1s22s1", Some(453.7), Some(1.0), Some(1615.0), Some(0.98), Some(145.92), Some(3.0), Some(0.108), Some(0.847), Some(3.6), Some(5.392), Some(13.1), Some(2.05), Some(1.23), "body centered", "basic", "1817", "silver white", 16777088, Some(0.618), Some(24.3), Some(159.3), Some(0.0), Some(138.78), Some(29.12)),
    elem!(4, "Be", "Beryllium", 9.012183, Alkaline, Solid, (1, 1), "2", "2", Some(1.85), "1s22s2", Some(1560.0), Some(1.0), Some(2745.0), Some(1.57), Some(292.4), Some(12.2), Some(0.313), Some(2.0), Some(1.82), Some(9.322), Some(5.0), Some(1.4), Some(0.9), "hexagonal", "amphoteric", "1798", "grey", 16777215, None, Some(5.6), Some(324.0), Some(0.0), Some(136.28), Some(9.5)),
    elem!(5, "B", "Boron", 10.811, Metalloid, Solid, (1, 12), "3", "3", Some(2.34), "1s22s22p1", Some(2300.0), Some(1.0), Some(4275.0), Some(2.04), Some(489.7), Some(50.2), Some(1e-12), Some(0.27), Some(1.02), Some(8.298), Some(4.6), Some(1.17), Some(0.82), "rhombohedral", "acidic", "1808", "yellow or brown", 65535, Some(0.277), Some(3.03), Some(565.0), Some(0.0), Some(153.43), Some(5.9)),
    elem!(6, "C", "Carbon", 12.011, NonMetal, Solid, (1, 13), "4", "+/- 4,2", Some(2.26), "1s22s22p2", Some(3823.0), Some(1.0), Some(4300.0), Some(2.55), Some(355.8), Some(105.0), Some(0.0007), Some(1.29), Some(0.709), Some(11.26), Some(5.3), Some(0.91), Some(0.77), "hexagonal", "weak acidic", "Ancient", "black", 0, Some(1.263), Some(1.76), Some(716.68), Some(0.0), Some(158.1), Some(5.74)),
    elem!(7, "N", "Nitrogen", 14.0067, NonMetal, Gas, (1, 14), "-3", "+/- 3,5,4,2", Some(1.2506), "1s22s22p3", Some(63.15), Some(1.0), Some(77.36), Some(3.04), Some(2.79), Some(0.36), None, Some(0.00026), Some(1.04), Some(14.534), Some(17.3), Some(0.75), Some(0.75), "hexagonal", "acidic", "1772", "colorless", 16777215, None, Some(1.1), Some(472.68), None, Some(153.3), None),
    elem!(8, "O", "Oxygen", 15.9994, NonMetal, Gas, (1, 15), "-2", "-2", Some(1.429), "1s22s22p4", Some(54.36), Some(1.0), Some(90.19), Some(3.44), Some(3.41), Some(0.222), None, Some(0.00027), Some(0.918), Some(13.618), Some(17.4), Some(0.65), Some(0.73), "cubic", "neutral", "1774", "colorless", 16777215, Some(1.461), Some(0.802), Some(249.18), None, Some(161.06), None),
    elem!(9, "F", "Fluorine", 18.9984, Halogen, Gas, (1, 16), "-1", "-1", Some(1.696), "1s22s22p5", Some(53.53), Some(1.0), Some(85.03), Some(3.98), Some(3.27), Some(0.255), None, Some(0.00028), Some(0.824), Some(17.423), Some(11.2), Some(0.57), Some(0.72), "monoclinic", "strong acidic", "1886", "pale yellow", 16777160, Some(3.401), Some(0.557), Some(79.38), None, Some(158.75), None),
    elem!(10, "Ne", "Neon", 20.1797, NobleGas, Gas, (1, 17), "-", "-", Some(0.8999), "1s22s22p6", Some(24.56), Some(1.0), Some(27.07), None, Some(1.71), Some(0.335), None, Some(0.00049), Some(1.03), Some(21.565), Some(16.7), Some(0.51), Some(0.71), "face centered", "-", "1898", "colorless", 16777215, None, Some(0.396), Some(0.0), None, Some(146.33), None),
    // Period 3
    elem!(11, "Na", "Sodium", 22.98977, Alkali, Solid, (2, 0), "1", "1", Some(0.971), "[Ne]3s1", Some(370.87), Some(1.0), Some(1156.0), Some(0.93), Some(97.42), Some(2.6), Some(0.21), Some(1.42), Some(1.23), Some(5.139), Some(23.7), Some(2.23), Some(1.54), "body centered", "strong basic", "1807", "silver white", 15790320, Some(0.548), Some(24.1), Some(107.3), Some(0.0), Some(153.71), Some(51.3)),
    elem!(12, "Mg", "Magnesium", 24.305, Alkaline, Solid, (2, 1), "2", "2", Some(1.738), "[Ne]3s2", Some(923.0), Some(1.0), Some(1363.0), Some(1.31), Some(128.0), Some(8.48), Some(0.226), Some(1.56), Some(1.02), Some(7.646), Some(14.0), Some(1.72), Some(1.36), "hexagonal", "basic", "1755", "silver white", 15132390, None, Some(10.6), Some(147.1), Some(0.0), Some(148.65), Some(32.67)),
    elem!(13, "Al", "Aluminum", 26.98154, PostTransitionMetal, Solid, (2, 12), "3", "3", Some(2.70), "[Ne]3s23p1", Some(933.47), Some(1.0), Some(2792.0), Some(1.61), Some(294.0), Some(10.71), Some(0.377), Some(2.37), Some(0.897), Some(5.986), Some(10.0), Some(1.82), Some(1.18), "face centered", "amphoteric", "1825", "silver", 14474460, Some(0.433), Some(6.8), Some(330.0), Some(0.0), Some(164.55), Some(28.3)),
    elem!(14, "Si", "Silicon", 28.0855, Metalloid, Solid, (2, 13), "4", "+/- 4", Some(2.33), "[Ne]3s23p2", Some(1687.0), Some(1.0), Some(3538.0), Some(1.90), Some(359.0), Some(50.21), Some(0.0001), Some(1.49), Some(0.712), Some(8.152), Some(12.1), Some(1.46), Some(1.11), "diamond cubic", "weak acidic", "1824", "dark grey", 9868950, Some(1.389), Some(5.38), Some(450.0), Some(0.0), Some(167.98), Some(18.81)),
    elem!(15, "P", "Phosphorus", 30.97376, NonMetal, Solid, (2, 14), "5", "+/- 3,5,4", Some(1.82), "[Ne]3s23p3", Some(317.3), Some(1.0), Some(553.0), Some(2.19), Some(12.4), Some(0.66), None, Some(0.00236), Some(0.769), Some(10.487), Some(17.0), Some(1.23), Some(1.06), "cubic", "acidic", "1669", "white/red", 16776960, Some(0.746), Some(3.63), Some(316.5), Some(0.0), Some(163.2), Some(41.09)),
    elem!(16, "S", "Sulfur", 32.065, NonMetal, Solid, (2, 15), "-2", "+/- 2,4,6", Some(2.07), "[Ne]3s23p4", Some(388.36), Some(1.0), Some(717.8), Some(2.58), Some(45.0), Some(1.73), None, Some(0.00205), Some(0.71), Some(10.36), Some(15.5), Some(1.09), Some(1.02), "orthorhombic", "acidic", "Ancient", "yellow", 16776960, Some(2.077), Some(2.9), Some(277.17), Some(0.0), Some(167.83), Some(32.05)),
    elem!(17, "Cl", "Chlorine", 35.453, Halogen, Gas, (2, 16), "-1", "+/- 1,3,5,7", Some(3.214), "[Ne]3s23p5", Some(171.6), Some(1.0), Some(239.11), Some(3.16), Some(20.41), Some(6.41), None, Some(0.00009), Some(0.479), Some(12.968), Some(18.7), Some(0.97), Some(0.99), "orthorhombic", "strong acidic", "1774", "greenish yellow", 12648320, Some(3.613), Some(2.18), Some(121.3), None, Some(165.19), None),
    elem!(18, "Ar", "Argon", 39.948, NobleGas, Gas, (2, 17), "-", "-", Some(1.784), "[Ne]3s23p6", Some(83.8), Some(1.0), Some(87.3), None, Some(6.53), Some(1.18), None, Some(0.00018), Some(0.52), Some(15.76), Some(24.2), Some(0.88), Some(0.98), "face centered", "-", "1894", "colorless", 16777215, None, Some(1.64), Some(0.0), None, Some(154.84), None),
    // Period 4
    elem!(19, "K", "Potassium", 39.0983, Alkali, Solid, (3, 0), "1", "1", Some(0.862), "[Ar]4s1", Some(336.53), Some(1.0), Some(1032.0), Some(0.82), Some(76.9), Some(2.33), Some(0.139), Some(1.02), Some(0.757), Some(4.341), Some(45.3), Some(2.77), Some(2.03), "body centered", "strong basic", "1807", "silver white", 15790320, Some(0.501), Some(43.4), Some(89.0), Some(0.0), Some(160.34), Some(64.68)),
    elem!(20, "Ca", "Calcium", 40.078, Alkaline, Solid, (3, 1), "2", "2", Some(1.55), "[Ar]4s2", Some(1115.0), Some(1.0), Some(1757.0), Some(1.0), Some(154.7), Some(8.54), Some(0.298), Some(2.0), Some(0.647), Some(6.113), Some(26.2), Some(2.23), Some(1.74), "face centered", "basic", "1808", "silver grey", 13882323, Some(0.024), Some(22.8), Some(177.8), Some(0.0), Some(154.88), Some(41.59)),
    elem!(21, "Sc", "Scandium", 44.95591, TransitionMetal, Solid, (3, 2), "3", "3", Some(2.989), "[Ar]3d14s2", Some(1814.0), Some(1.0), Some(3109.0), Some(1.36), Some(332.7), Some(14.1), Some(0.018), Some(0.158), Some(0.568), Some(6.561), Some(15.0), Some(2.09), Some(1.44), "hexagonal", "weak basic", "1879", "silver white", 14474460, Some(0.188), Some(17.8), Some(377.8), Some(0.0), Some(174.79), Some(34.64)),
    elem!(22, "Ti", "Titanium", 47.867, TransitionMetal, Solid, (3, 3), "4", "4,3,2", Some(4.54), "[Ar]3d24s2", Some(1941.0), Some(1.0), Some(3560.0), Some(1.54), Some(425.0), Some(14.15), Some(0.023), Some(0.219), Some(0.523), Some(6.828), Some(10.6), Some(2.0), Some(1.32), "hexagonal", "amphoteric", "1791", "silver grey", 13158600, Some(0.079), Some(14.6), Some(473.0), Some(0.0), Some(180.3), Some(30.72)),
    elem!(23, "V", "Vanadium", 50.9415, TransitionMetal, Solid, (3, 4), "5", "5,4,3,2", Some(6.11), "[Ar]3d34s2", Some(2183.0), Some(1.0), Some(3680.0), Some(1.63), Some(459.0), Some(21.5), Some(0.05), Some(0.307), Some(0.489), Some(6.746), Some(8.35), Some(1.92), Some(1.22), "body centered", "amphoteric", "1801", "silver grey", 13158600, Some(0.525), Some(12.4), Some(514.2), Some(0.0), Some(182.3), Some(28.91)),
    elem!(24, "Cr", "Chromium", 51.9961, TransitionMetal, Solid, (3, 5), "3", "6,3,2", Some(7.19), "[Ar]3d54s1", Some(2180.0), Some(1.0), Some(2944.0), Some(1.66), Some(339.5), Some(21.0), Some(0.079), Some(0.939), Some(0.449), Some(6.767), Some(7.23), Some(1.85), Some(1.18), "body centered", "amphoteric", "1797", "silver metallic", 13882323, Some(0.666), Some(11.6), Some(396.6), Some(0.0), Some(174.5), Some(23.77)),
    elem!(25, "Mn", "Manganese", 54.93805, TransitionMetal, Solid, (3, 6), "2", "7,6,4,3,2", Some(7.44), "[Ar]3d54s2", Some(1519.0), Some(1.0), Some(2334.0), Some(1.55), Some(221.0), Some(12.91), Some(0.007), Some(0.078), Some(0.479), Some(7.434), Some(7.35), Some(1.79), Some(1.17), "body centered", "amphoteric", "1774", "silver grey", 13158600, None, Some(9.4), Some(280.7), Some(0.0), Some(173.7), Some(32.01)),
    elem!(26, "Fe", "Iron", 55.845, TransitionMetal, Solid, (3, 7), "3", "2,3", Some(7.874), "[Ar]3d64s2", Some(1811.0), Some(1.0), Some(3134.0), Some(1.83), Some(340.0), Some(13.81), Some(0.104), Some(0.804), Some(0.449), Some(7.902), Some(7.09), Some(1.72), Some(1.17), "body centered", "amphoteric", "Ancient", "lustrous metallic", 12500670, Some(0.151), Some(8.4), Some(416.3), Some(0.0), Some(180.49), Some(27.28)),
    elem!(27, "Co", "Cobalt", 58.93319, TransitionMetal, Solid, (3, 8), "2", "2,3", Some(8.9), "[Ar]3d74s2", Some(1768.0), Some(1.0), Some(3200.0), Some(1.88), Some(377.0), Some(16.06), Some(0.172), Some(1.0), Some(0.421), Some(7.881), Some(6.62), Some(1.67), Some(1.16), "hexagonal", "amphoteric", "1735", "metallic grey", 12500670, Some(0.662), Some(7.5), Some(424.7), Some(0.0), Some(179.52), Some(30.04)),
    elem!(28, "Ni", "Nickel", 58.6934, TransitionMetal, Solid, (3, 9), "2", "2,3", Some(8.902), "[Ar]3d84s2", Some(1728.0), Some(1.0), Some(3186.0), Some(1.91), Some(377.5), Some(17.48), Some(0.143), Some(0.909), Some(0.444), Some(7.64), Some(6.59), Some(1.62), Some(1.15), "face centered", "weak basic", "1751", "silver metallic", 13882323, Some(1.156), Some(6.8), Some(429.7), Some(0.0), Some(182.19), Some(29.87)),
    elem!(29, "Cu", "Copper", 63.546, TransitionMetal, Solid, (3, 10), "2", "2,1", Some(8.96), "[Ar]3d104s1", Some(1357.77), Some(1.0), Some(2835.0), Some(1.90), Some(300.4), Some(13.26), Some(0.596), Some(4.01), Some(0.385), Some(7.726), Some(7.11), Some(1.57), Some(1.17), "face centered", "basic", "Ancient", "red orange", 13132850, Some(1.235), Some(6.1), Some(337.4), Some(0.0), Some(166.38), Some(33.15)),
    elem!(30, "Zn", "Zinc", 65.38, TransitionMetal, Solid, (3, 11), "2", "2", Some(7.133), "[Ar]3d104s2", Some(692.68), Some(1.0), Some(1180.0), Some(1.65), Some(123.6), Some(7.32), Some(0.169), Some(1.16), Some(0.388), Some(9.394), Some(9.16), Some(1.53), Some(1.25), "hexagonal", "amphoteric", "1746", "bluish grey", 11842740, None, Some(7.1), Some(130.4), Some(0.0), Some(160.99), Some(41.63)),
    elem!(31, "Ga", "Gallium", 69.723, PostTransitionMetal, Solid, (3, 12), "3", "3", Some(5.907), "[Ar]3d104s24p1", Some(302.91), Some(1.0), Some(2477.0), Some(1.81), Some(254.0), Some(5.59), Some(0.067), Some(0.406), Some(0.371), Some(5.999), Some(11.8), Some(1.81), Some(1.26), "orthorhombic", "amphoteric", "1875", "silvery blue", 13158600, Some(0.43), Some(8.12), Some(272.0), Some(0.0), Some(169.06), Some(40.88)),
    elem!(32, "Ge", "Germanium", 72.64, Metalloid, Solid, (3, 13), "4", "4,2", Some(5.323), "[Ar]3d104s24p2", Some(1211.4), Some(1.0), Some(3106.0), Some(2.01), Some(334.0), Some(36.94), Some(2e-8), Some(0.602), Some(0.32), Some(7.899), Some(13.6), Some(1.52), Some(1.22), "diamond cubic", "amphoteric", "1886", "greyish white", 12500670, Some(1.233), Some(6.07), Some(372.0), Some(0.0), Some(167.9), Some(31.09)),
    elem!(33, "As", "Arsenic", 74.9216, Metalloid, Solid, (3, 14), "3", "+/- 3,5", Some(5.727), "[Ar]3d104s24p3", Some(1090.0), Some(28.0), Some(887.0), Some(2.18), Some(32.4), Some(24.44), Some(0.03), Some(0.502), Some(0.329), Some(9.789), Some(13.1), Some(1.33), Some(1.2), "rhombohedral", "acidic", "Ancient", "metallic grey", 10526880, Some(0.814), Some(4.31), Some(302.5), Some(0.0), Some(174.21), Some(35.1)),
    elem!(34, "Se", "Selenium", 78.96, NonMetal, Solid, (3, 15), "4", "-2,4,6", Some(4.809), "[Ar]3d104s24p4", Some(494.0), Some(1.0), Some(958.0), Some(2.55), Some(95.48), Some(6.69), Some(1e-12), Some(0.0052), Some(0.321), Some(9.752), Some(16.4), Some(1.22), Some(1.16), "hexagonal", "acidic", "1817", "grey", 10526880, Some(2.021), Some(3.77), Some(227.1), Some(0.0), Some(176.72), Some(42.44)),
    elem!(35, "Br", "Bromine", 79.904, Halogen, Liquid, (3, 16), "-1", "+/- 1,5", Some(3.12), "[Ar]3d104s24p5", Some(265.8), Some(1.0), Some(332.0), Some(2.96), Some(14.8), Some(5.29), None, Some(0.00122), Some(0.474), Some(11.814), Some(25.6), Some(1.12), Some(1.14), "orthorhombic", "acidic", "1826", "reddish brown", 10821120, Some(3.364), Some(3.05), Some(111.87), None, Some(175.02), None),
    elem!(36, "Kr", "Krypton", 83.798, NobleGas, Gas, (3, 17), "-", "-", Some(3.75), "[Ar]3d104s24p6", Some(115.79), Some(1.0), Some(119.93), Some(3.0), Some(9.08), Some(1.64), None, Some(0.000095), Some(0.248), Some(14.0), Some(32.2), Some(1.03), Some(1.12), "face centered", "-", "1898", "colorless", 16777215, None, Some(2.48), Some(0.0), None, Some(164.08), None),
    // Period 5
    elem!(37, "Rb", "Rubidium", 85.4678, Alkali, Solid, (4, 0), "1", "1", Some(1.532), "[Kr]5s1", Some(312.46), Some(1.0), Some(961.0), Some(0.82), Some(69.0), Some(2.19), Some(0.08), Some(0.582), Some(0.363), Some(4.177), Some(55.9), Some(2.98), Some(2.16), "body centered", "strong basic", "1861", "silver white", 15790320, Some(0.486), Some(47.3), Some(80.9), Some(0.0), Some(170.09), Some(76.78)),
    elem!(38, "Sr", "Strontium", 87.62, Alkaline, Solid, (4, 1), "2", "2", Some(2.64), "[Kr]5s2", Some(1050.0), Some(1.0), Some(1655.0), Some(0.95), Some(137.0), Some(7.43), Some(0.076), Some(0.354), Some(0.301), Some(5.695), Some(33.7), Some(2.45), Some(1.91), "face centered", "basic", "1790", "silvery white", 15132390, Some(0.05), Some(27.6), Some(164.4), Some(0.0), Some(164.64), Some(55.0)),
    elem!(39, "Y", "Yttrium", 88.90585, TransitionMetal, Solid, (4, 2), "3", "3", Some(4.469), "[Kr]4d15s2", Some(1799.0), Some(1.0), Some(3609.0), Some(1.22), Some(365.0), Some(11.42), Some(0.017), Some(0.172), Some(0.298), Some(6.217), Some(19.9), Some(2.27), Some(1.62), "hexagonal", "weak basic", "1794", "silvery white", 14474460, Some(0.307), Some(22.7), Some(421.3), Some(0.0), Some(179.48), Some(44.43)),
    elem!(40, "Zr", "Zirconium", 91.224, TransitionMetal, Solid, (4, 3), "4", "4", Some(6.506), "[Kr]4d25s2", Some(2128.0), Some(1.0), Some(4682.0), Some(1.33), Some(573.0), Some(14.0), Some(0.024), Some(0.226), Some(0.278), Some(6.634), Some(14.0), Some(2.16), Some(1.45), "hexagonal", "amphoteric", "1789", "greyish white", 13882323, Some(0.426), Some(17.9), Some(608.8), Some(0.0), Some(181.36), Some(38.99)),
    elem!(41, "Nb", "Niobium", 92.90638, TransitionMetal, Solid, (4, 4), "5", "5,3", Some(8.57), "[Kr]4d45s1", Some(2750.0), Some(1.0), Some(5017.0), Some(1.6), Some(689.9), Some(30.0), Some(0.066), Some(0.537), Some(0.265), Some(6.759), Some(10.8), Some(2.08), Some(1.34), "body centered", "amphoteric", "1801", "grey metallic", 12500670, Some(0.893), Some(15.7), Some(725.9), Some(0.0), Some(186.26), Some(36.4)),
    elem!(42, "Mo", "Molybdenum", 95.94, TransitionMetal, Solid, (4, 5), "6", "6,5,4,3,2", Some(10.22), "[Kr]4d55s1", Some(2896.0), Some(1.0), Some(4912.0), Some(2.16), Some(617.0), Some(37.48), Some(0.187), Some(1.38), Some(0.251), Some(7.092), Some(9.4), Some(2.01), Some(1.3), "body centered", "acidic", "1781", "grey metallic", 12500670, Some(0.746), Some(12.8), Some(658.1), Some(0.0), Some(181.95), Some(28.66)),
    elem!(43, "Tc", "Technetium", 98.0, TransitionMetal, Solid, (4, 6), "7", "7,6,4", Some(11.5), "[Kr]4d55s2", Some(2430.0), Some(1.0), Some(4538.0), Some(1.9), Some(585.0), Some(33.29), Some(0.05), Some(0.506), Some(0.24), Some(7.28), Some(8.5), Some(1.95), Some(1.27), "hexagonal", "acidic", "1937", "silver grey", 13158600, Some(0.55), Some(11.4), Some(677.0), Some(0.0), Some(181.07), Some(33.47)),
    elem!(44, "Ru", "Ruthenium", 101.07, TransitionMetal, Solid, (4, 7), "3", "2,3,4,6,8", Some(12.41), "[Kr]4d75s1", Some(2607.0), Some(1.0), Some(4423.0), Some(2.2), Some(591.6), Some(38.59), Some(0.14), Some(1.17), Some(0.238), Some(7.361), Some(8.3), Some(1.89), Some(1.25), "hexagonal", "acidic", "1844", "silvery white", 13882323, Some(1.05), Some(9.6), Some(642.7), Some(0.0), Some(186.51), Some(28.53)),
    elem!(45, "Rh", "Rhodium", 102.9055, TransitionMetal, Solid, (4, 8), "3", "2,3,4", Some(12.41), "[Kr]4d85s1", Some(2237.0), Some(1.0), Some(3968.0), Some(2.28), Some(494.0), Some(26.59), Some(0.22), Some(1.5), Some(0.243), Some(7.459), Some(8.3), Some(1.83), Some(1.25), "face centered", "amphoteric", "1803", "silver white", 14474460, Some(1.137), Some(8.6), Some(556.9), Some(0.0), Some(185.81), Some(31.51)),
    elem!(46, "Pd", "Palladium", 106.42, TransitionMetal, Solid, (4, 9), "2", "2,4", Some(12.02), "[Kr]4d10", Some(1828.05), Some(1.0), Some(3236.0), Some(2.2), Some(362.0), Some(16.74), Some(0.095), Some(0.718), Some(0.244), Some(8.337), Some(8.9), Some(1.79), Some(1.28), "face centered", "basic", "1803", "silver white", 14474460, Some(0.562), Some(4.8), Some(378.2), Some(0.0), Some(167.05), Some(37.57)),
    elem!(47, "Ag", "Silver", 107.8682, TransitionMetal, Solid, (4, 10), "1", "1", Some(10.50), "[Kr]4d105s1", Some(1234.93), Some(1.0), Some(2435.0), Some(1.93), Some(254.0), Some(11.28), Some(0.63), Some(4.29), Some(0.235), Some(7.576), Some(10.3), Some(1.75), Some(1.34), "face centered", "amphoteric", "Ancient", "silver", 15132390, Some(1.302), Some(7.2), Some(284.9), Some(0.0), Some(173.0), Some(42.55)),
    elem!(48, "Cd", "Cadmium", 112.411, TransitionMetal, Solid, (4, 11), "2", "2", Some(8.65), "[Kr]4d105s2", Some(594.22), Some(1.0), Some(1040.0), Some(1.69), Some(99.87), Some(6.21), Some(0.14), Some(0.969), Some(0.232), Some(8.994), Some(13.1), Some(1.71), Some(1.48), "hexagonal", "basic", "1817", "silvery grey", 13158600, None, Some(7.2), Some(111.8), Some(0.0), Some(167.75), Some(51.8)),
    elem!(49, "In", "Indium", 114.818, PostTransitionMetal, Solid, (4, 12), "3", "3", Some(7.31), "[Kr]4d105s25p1", Some(429.75), Some(1.0), Some(2345.0), Some(1.78), Some(231.8), Some(3.28), Some(0.12), Some(0.818), Some(0.233), Some(5.786), Some(15.7), Some(2.0), Some(1.44), "tetragonal", "amphoteric", "1863", "silvery", 13882323, Some(0.3), Some(10.2), Some(243.3), Some(0.0), Some(173.79), Some(57.82)),
    elem!(50, "Sn", "Tin", 118.71, PostTransitionMetal, Solid, (4, 13), "4", "4,2", Some(7.31), "[Kr]4d105s25p2", Some(505.08), Some(1.0), Some(2875.0), Some(1.96), Some(296.1), Some(7.03), Some(0.091), Some(0.668), Some(0.228), Some(7.344), Some(16.3), Some(1.72), Some(1.41), "tetragonal", "amphoteric", "Ancient", "silver white", 13882323, Some(1.112), Some(7.7), Some(301.2), Some(0.0), Some(168.49), Some(51.18)),
    elem!(51, "Sb", "Antimony", 121.76, Metalloid, Solid, (4, 14), "3", "+/- 3,5", Some(6.691), "[Kr]4d105s25p3", Some(903.78), Some(1.0), Some(1860.0), Some(2.05), Some(193.43), Some(19.79), Some(0.025), Some(0.244), Some(0.207), Some(8.608), Some(18.4), Some(1.53), Some(1.4), "rhombohedral", "amphoteric", "Ancient", "silvery grey", 13158600, Some(1.046), Some(6.6), Some(262.3), Some(0.0), Some(180.28), Some(45.69)),
    elem!(52, "Te", "Tellurium", 127.6, Metalloid, Solid, (4, 15), "4", "-2,4,6", Some(6.24), "[Kr]4d105s25p4", Some(722.66), Some(1.0), Some(1261.0), Some(2.1), Some(114.1), Some(17.49), Some(2e-6), Some(0.0338), Some(0.202), Some(9.01), Some(20.5), Some(1.42), Some(1.36), "hexagonal", "amphoteric", "1782", "silvery grey", 13158600, Some(1.971), Some(5.5), Some(196.73), Some(0.0), Some(182.74), Some(49.71)),
    elem!(53, "I", "Iodine", 126.9045, Halogen, Solid, (4, 16), "-1", "+/- 1,5,7", Some(4.93), "[Kr]4d105s25p5", Some(386.85), Some(1.0), Some(457.4), Some(2.66), Some(41.57), Some(15.52), None, Some(0.00449), Some(0.214), Some(10.451), Some(25.7), Some(1.32), Some(1.33), "orthorhombic", "acidic", "1811", "violet dark", 6684800, Some(3.059), Some(5.35), Some(106.76), Some(0.0), Some(180.79), Some(58.07)),
    elem!(54, "Xe", "Xenon", 131.293, NobleGas, Gas, (4, 17), "-", "-", Some(5.89), "[Kr]4d105s25p6", Some(161.4), Some(1.0), Some(165.03), Some(2.6), Some(12.64), Some(2.27), None, Some(0.000057), Some(0.158), Some(12.13), Some(37.3), Some(1.24), Some(1.31), "face centered", "-", "1898", "colorless", 16777215, None, Some(4.04), Some(0.0), None, Some(169.68), None),
    // Period 6
    elem!(55, "Cs", "Cesium", 132.9055, Alkali, Solid, (5, 0), "1", "1", Some(1.873), "[Xe]6s1", Some(301.59), Some(1.0), Some(944.0), Some(0.79), Some(63.9), Some(2.09), Some(0.05), Some(0.359), Some(0.242), Some(3.894), Some(70.0), Some(3.34), Some(2.35), "body centered", "strong basic", "1860", "silvery gold", 16772520, Some(0.472), Some(59.6), Some(76.5), Some(0.0), Some(175.6), Some(85.23)),
    elem!(56, "Ba", "Barium", 137.327, Alkaline, Solid, (5, 1), "2", "2", Some(3.594), "[Xe]6s2", Some(1000.0), Some(1.0), Some(2170.0), Some(0.89), Some(140.3), Some(7.12), Some(0.03), Some(0.184), Some(0.204), Some(5.212), Some(39.0), Some(2.78), Some(1.98), "body centered", "basic", "1808", "silvery grey", 13882323, Some(0.145), Some(39.7), Some(180.0), Some(0.0), Some(170.24), Some(62.48)),
    // Lanthanides (grid row 7, cols 3..17 -> 57..71)
    elem!(57, "La", "Lanthanum", 138.9055, Lanthanide, Solid, (7, 3), "3", "3", Some(6.145), "[Xe]5d16s2", Some(1193.0), Some(1.0), Some(3737.0), Some(1.1), Some(402.1), Some(6.2), Some(0.017), Some(0.134), Some(0.195), Some(5.577), Some(22.5), Some(2.74), Some(1.69), "hexagonal", "basic", "1839", "silvery white", 14474460, Some(0.47), Some(31.1), Some(431.0), Some(0.0), Some(182.38), Some(56.9)),
    elem!(58, "Ce", "Cerium", 140.116, Lanthanide, Solid, (7, 4), "3", "3,4", Some(6.77), "[Xe]4f15d16s2", Some(1068.0), Some(1.0), Some(3716.0), Some(1.12), Some(398.0), Some(5.46), Some(0.013), Some(0.113), Some(0.192), Some(5.539), Some(21.0), Some(2.7), Some(1.65), "face centered", "basic", "1803", "silvery white", 14474460, Some(0.57), Some(29.6), Some(422.6), Some(0.0), Some(191.77), Some(72.0)),
    elem!(59, "Pr", "Praseodymium", 140.9077, Lanthanide, Solid, (7, 5), "3", "3,4", Some(6.773), "[Xe]4f36s2", Some(1208.0), Some(1.0), Some(3793.0), Some(1.13), Some(331.0), Some(6.89), Some(0.015), Some(0.125), Some(0.193), Some(5.473), Some(20.8), Some(2.67), Some(1.65), "hexagonal", "basic", "1885", "silvery white", 14474460, None, Some(28.2), Some(355.6), Some(0.0), Some(189.8), Some(73.2)),
    elem!(60, "Nd", "Neodymium", 144.242, Lanthanide, Solid, (7, 6), "3", "3", Some(7.007), "[Xe]4f46s2", Some(1297.0), Some(1.0), Some(3347.0), Some(1.14), Some(289.0), Some(7.14), Some(0.016), Some(0.165), Some(0.19), Some(5.525), Some(20.6), Some(2.64), Some(1.64), "hexagonal", "basic", "1885", "silvery white", 14474460, None, Some(31.4), Some(327.6), Some(0.0), Some(189.41), Some(71.5)),
    elem!(61, "Pm", "Promethium", 145.0, Lanthanide, Solid, (7, 7), "3", "3", Some(7.26), "[Xe]4f56s2", Some(1315.0), Some(1.0), Some(3273.0), Some(1.13), Some(289.0), Some(7.13), Some(0.013), Some(0.179), None, Some(5.582), Some(20.2), Some(2.62), Some(1.63), "hexagonal", "basic", "1945", "metallic", 14474460, None, Some(30.1), Some(348.0), Some(0.0), Some(187.0), Some(71.5)),
    elem!(62, "Sm", "Samarium", 150.36, Lanthanide, Solid, (7, 8), "3", "3,2", Some(7.52), "[Xe]4f66s2", Some(1345.0), Some(1.0), Some(2067.0), Some(1.17), Some(165.0), Some(8.62), Some(0.011), Some(0.133), Some(0.197), Some(5.644), Some(19.9), Some(2.59), Some(1.62), "rhombohedral", "basic", "1879", "silvery white", 14474460, None, Some(28.8), Some(206.7), Some(0.0), Some(183.04), Some(69.58)),
    elem!(63, "Eu", "Europium", 151.964, Lanthanide, Solid, (7, 9), "3", "3,2", Some(5.243), "[Xe]4f76s2", Some(1099.0), Some(1.0), Some(1802.0), Some(1.2), Some(176.0), Some(9.21), Some(0.011), Some(0.139), Some(0.182), Some(5.67), Some(28.9), Some(2.56), Some(1.85), "body centered", "basic", "1901", "silvery white", 14474460, None, Some(27.7), Some(175.3), Some(0.0), Some(188.79), Some(77.78)),
    elem!(64, "Gd", "Gadolinium", 157.25, Lanthanide, Solid, (7, 10), "3", "3", Some(7.901), "[Xe]4f75d16s2", Some(1585.0), Some(1.0), Some(3546.0), Some(1.2), Some(301.3), Some(10.05), Some(0.008), Some(0.106), Some(0.236), Some(6.15), Some(19.9), Some(2.54), Some(1.61), "hexagonal", "basic", "1880", "silvery white", 14474460, None, Some(23.5), Some(397.5), Some(0.0), Some(194.31), Some(68.07)),
    elem!(65, "Tb", "Terbium", 158.9254, Lanthanide, Solid, (7, 11), "3", "3,4", Some(8.229), "[Xe]4f96s2", Some(1629.0), Some(1.0), Some(3503.0), Some(1.2), Some(293.0), Some(10.15), Some(0.009), Some(0.111), Some(0.182), Some(5.864), Some(19.3), Some(2.51), Some(1.59), "hexagonal", "basic", "1843", "silvery white", 14474460, None, Some(25.5), Some(388.7), Some(0.0), Some(203.0), Some(73.22)),
    elem!(66, "Dy", "Dysprosium", 162.5, Lanthanide, Solid, (7, 12), "3", "3", Some(8.55), "[Xe]4f106s2", Some(1680.0), Some(1.0), Some(2840.0), Some(1.22), Some(280.0), Some(11.06), Some(0.011), Some(0.107), Some(0.17), Some(5.939), Some(19.0), Some(2.49), Some(1.59), "hexagonal", "basic", "1886", "silvery white", 14474460, None, Some(24.5), Some(290.4), Some(0.0), Some(196.6), Some(74.77)),
    elem!(67, "Ho", "Holmium", 164.9303, Lanthanide, Solid, (7, 13), "3", "3", Some(8.795), "[Xe]4f116s2", Some(1734.0), Some(1.0), Some(2993.0), Some(1.23), Some(251.0), Some(17.0), Some(0.012), Some(0.162), Some(0.165), Some(6.022), Some(18.7), Some(2.47), Some(1.58), "hexagonal", "basic", "1878", "silvery white", 14474460, None, Some(23.6), Some(300.8), Some(0.0), Some(195.6), Some(75.3)),
    elem!(68, "Er", "Erbium", 167.259, Lanthanide, Solid, (7, 14), "3", "3", Some(9.066), "[Xe]4f126s2", Some(1802.0), Some(1.0), Some(3141.0), Some(1.24), Some(280.0), Some(19.9), Some(0.012), Some(0.145), Some(0.168), Some(6.108), Some(18.4), Some(2.45), Some(1.57), "hexagonal", "basic", "1842", "silvery white", 14474460, None, Some(22.7), Some(317.1), Some(0.0), Some(195.6), Some(73.18)),
    elem!(69, "Tm", "Thulium", 168.9342, Lanthanide, Solid, (7, 15), "3", "3,2", Some(9.321), "[Xe]4f136s2", Some(1818.0), Some(1.0), Some(2223.0), Some(1.25), Some(247.0), Some(16.84), Some(0.015), Some(0.169), Some(0.16), Some(6.184), Some(18.1), Some(2.42), Some(1.56), "hexagonal", "basic", "1879", "silvery grey", 13882323, None, Some(21.8), Some(232.2), Some(0.0), Some(190.1), Some(74.01)),
    elem!(70, "Yb", "Ytterbium", 173.04, Lanthanide, Solid, (7, 16), "3", "3,2", Some(6.965), "[Xe]4f146s2", Some(1097.0), Some(1.0), Some(1469.0), Some(1.1), Some(159.0), Some(7.66), Some(0.035), Some(0.385), Some(0.155), Some(6.254), Some(24.8), Some(2.4), Some(1.74), "face centered", "basic", "1878", "silvery white", 14474460, None, Some(21.0), Some(152.1), Some(0.0), Some(173.13), Some(59.87)),
    elem!(71, "Lu", "Lutetium", 174.967, Lanthanide, Solid, (7, 17), "3", "3", Some(9.84), "[Xe]4f145d16s2", Some(1925.0), Some(1.0), Some(3675.0), Some(1.27), Some(414.0), Some(22.0), Some(0.017), Some(0.164), Some(0.154), Some(5.426), Some(17.8), Some(2.25), Some(1.56), "hexagonal", "weak basic", "1907", "silvery white", 14474460, Some(0.34), Some(21.9), Some(427.6), Some(0.0), Some(184.8), Some(50.96)),
    // Rest of Period 6 (72..86)
    elem!(72, "Hf", "Hafnium", 178.49, TransitionMetal, Solid, (5, 3), "4", "4", Some(13.31), "[Xe]4f145d26s2", Some(2506.0), Some(1.0), Some(4876.0), Some(1.3), Some(571.0), Some(27.2), Some(0.033), Some(0.23), Some(0.144), Some(6.825), Some(13.6), Some(2.16), Some(1.44), "hexagonal", "amphoteric", "1923", "steel grey", 12500670, None, Some(16.2), Some(619.2), Some(0.0), Some(186.89), Some(43.56)),
    elem!(73, "Ta", "Tantalum", 180.9479, TransitionMetal, Solid, (5, 4), "5", "5", Some(16.654), "[Xe]4f145d36s2", Some(3290.0), Some(1.0), Some(5731.0), Some(1.5), Some(732.8), Some(36.57), Some(0.076), Some(0.575), Some(0.14), Some(7.55), Some(10.9), Some(2.09), Some(1.34), "body centered", "acidic", "1802", "grey blue", 11842740, Some(0.322), Some(13.1), Some(782.0), Some(0.0), Some(185.21), Some(41.51)),
    elem!(74, "W", "Tungsten", 183.84, TransitionMetal, Solid, (5, 5), "6", "6,5,4,3,2", Some(19.25), "[Xe]4f145d46s2", Some(3695.0), Some(1.0), Some(5828.0), Some(2.36), Some(806.7), Some(52.31), Some(0.189), Some(1.73), Some(0.132), Some(7.864), Some(9.53), Some(2.02), Some(1.3), "body centered", "acidic", "1783", "greyish white", 12500670, Some(0.815), Some(11.1), Some(849.4), Some(0.0), Some(173.95), Some(32.64)),
    elem!(75, "Re", "Rhenium", 186.207, TransitionMetal, Solid, (5, 6), "7", "7,6,4,2,-1", Some(21.02), "[Xe]4f145d56s2", Some(3459.0), Some(1.0), Some(5869.0), Some(1.9), Some(704.0), Some(60.43), Some(0.056), Some(0.48), Some(0.137), Some(7.833), Some(8.85), Some(1.97), Some(1.28), "hexagonal", "acidic", "1925", "greyish white", 13158600, Some(0.15), Some(9.7), Some(769.9), Some(0.0), Some(188.94), Some(36.86)),
    elem!(76, "Os", "Osmium", 190.23, TransitionMetal, Solid, (5, 7), "4", "2,3,4,6,8", Some(22.59), "[Xe]4f145d66s2", Some(3306.0), Some(1.0), Some(5285.0), Some(2.2), Some(738.0), Some(57.85), Some(0.123), Some(0.876), Some(0.13), Some(8.438), Some(8.43), Some(1.92), Some(1.26), "hexagonal", "acidic", "1803", "bluish grey", 11842740, Some(1.1), Some(8.5), Some(791.0), Some(0.0), Some(192.58), Some(32.64)),
    elem!(77, "Ir", "Iridium", 192.217, TransitionMetal, Solid, (5, 8), "4", "2,3,4,6", Some(22.56), "[Xe]4f145d76s2", Some(2719.0), Some(1.0), Some(4701.0), Some(2.2), Some(563.7), Some(41.12), Some(0.21), Some(1.47), Some(0.131), Some(8.967), Some(8.54), Some(1.87), Some(1.27), "face centered", "amphoteric", "1803", "silvery white", 14474460, Some(1.564), Some(7.6), Some(665.3), Some(0.0), Some(193.58), Some(35.48)),
    elem!(78, "Pt", "Platinum", 195.084, TransitionMetal, Solid, (5, 9), "4", "2,4", Some(21.45), "[Xe]4f145d96s1", Some(2041.4), Some(1.0), Some(4098.0), Some(2.28), Some(469.0), Some(22.17), Some(0.094), Some(0.716), Some(0.133), Some(8.959), Some(9.1), Some(1.83), Some(1.3), "face centered", "amphoteric", "1735", "greyish white", 13882323, Some(2.128), Some(6.5), Some(565.3), Some(0.0), Some(192.41), Some(41.63)),
    elem!(79, "Au", "Gold", 196.9666, TransitionMetal, Solid, (5, 10), "3", "3,1", Some(19.32), "[Xe]4f145d106s1", Some(1337.33), Some(1.0), Some(3129.0), Some(2.54), Some(324.0), Some(12.55), Some(0.452), Some(3.18), Some(0.129), Some(9.226), Some(10.2), Some(1.79), Some(1.34), "face centered", "amphoteric", "Ancient", "gold yellow", 16766720, Some(2.309), Some(5.8), Some(366.1), Some(0.0), Some(180.5), Some(47.4)),
    elem!(80, "Hg", "Mercury", 200.59, TransitionMetal, Liquid, (5, 11), "2", "2,1", Some(13.534), "[Xe]4f145d106s2", Some(234.32), Some(1.0), Some(629.88), Some(2.0), Some(59.11), Some(2.29), Some(0.01), Some(0.083), Some(0.14), Some(10.438), Some(14.8), Some(1.76), Some(1.49), "rhombohedral", "weak basic", "Ancient", "silver", 13882323, None, Some(5.7), Some(61.38), None, Some(174.97), Some(75.9)),
    elem!(81, "Tl", "Thallium", 204.3833, PostTransitionMetal, Solid, (5, 12), "1", "3,1", Some(11.85), "[Xe]4f145d106s26p1", Some(577.0), Some(1.0), Some(1746.0), Some(1.62), Some(165.0), Some(4.14), Some(0.056), Some(0.461), Some(0.129), Some(6.108), Some(17.2), Some(2.08), Some(1.48), "hexagonal", "basic", "1861", "silvery white", 13882323, Some(0.377), Some(7.6), Some(182.2), Some(0.0), Some(180.96), Some(64.18)),
    elem!(82, "Pb", "Lead", 207.2, PostTransitionMetal, Solid, (5, 13), "2", "4,2", Some(11.34), "[Xe]4f145d106s26p2", Some(600.61), Some(1.0), Some(2022.0), Some(2.33), Some(179.5), Some(4.77), Some(0.048), Some(0.353), Some(0.129), Some(7.417), Some(18.3), Some(1.81), Some(1.47), "face centered", "amphoteric", "Ancient", "bluish white", 11842740, Some(0.364), Some(6.8), Some(195.2), Some(0.0), Some(175.37), Some(64.8)),
    elem!(83, "Bi", "Bismuth", 208.9804, PostTransitionMetal, Solid, (5, 14), "3", "3,5", Some(9.747), "[Xe]4f145d106s26p3", Some(544.7), Some(1.0), Some(1837.0), Some(2.02), Some(151.0), Some(11.3), Some(0.008), Some(0.079), Some(0.122), Some(7.286), Some(21.3), Some(1.63), Some(1.46), "rhombohedral", "weak basic", "1753", "pinkish white", 15132390, Some(0.942), Some(7.4), Some(207.1), Some(0.0), Some(187.0), Some(56.74)),
    elem!(84, "Po", "Polonium", 209.0, PostTransitionMetal, Solid, (5, 15), "4", "4,2", Some(9.32), "[Xe]4f145d106s26p4", Some(527.0), Some(1.0), Some(1235.0), Some(2.0), Some(102.91), Some(13.0), Some(0.023), Some(0.2), Some(0.125), Some(8.417), Some(22.2), Some(1.53), Some(1.46), "cubic", "amphoteric", "1898", "silvery", 13158600, Some(1.9), Some(6.8), Some(144.3), Some(0.0), Some(188.9), Some(62.76)),
    elem!(85, "At", "Astatine", 210.0, Halogen, Solid, (5, 16), "-1", "+/- 1,3,5,7", Some(7.0), "[Xe]4f145d106s26p5", Some(575.0), Some(1.0), Some(610.0), Some(2.2), Some(54.4), None, None, Some(0.017), None, Some(9.318), None, Some(1.43), Some(1.45), "face centered", "acidic", "1940", "metallic", 11842740, Some(2.8), Some(6.0), Some(90.0), Some(0.0), Some(187.0), Some(60.0)),
    elem!(86, "Rn", "Radon", 222.0, NobleGas, Gas, (5, 17), "0", "0,2", Some(9.73), "[Xe]4f145d106s26p6", Some(202.0), Some(1.0), Some(211.3), Some(2.2), Some(18.1), Some(3.25), None, Some(0.000036), Some(0.094), Some(10.748), Some(50.5), Some(1.34), Some(1.45), "face centered", "-", "1900", "colorless", 16777215, None, Some(5.3), Some(0.0), None, Some(176.2), None),
    // Period 7 + Actinides (87..103)
    elem!(87, "Fr", "Francium", 223.0, Alkali, Solid, (6, 0), "1", "1", Some(1.87), "[Rn]7s1", Some(300.0), Some(1.0), Some(950.0), Some(0.7), Some(65.0), Some(2.0), None, Some(0.15), None, Some(4.073), None, Some(2.7), None, "body centered", "strong basic", "1939", "metallic", 13882323, Some(0.486), Some(48.6), Some(73.0), Some(0.0), Some(179.9), Some(95.4)),
    elem!(88, "Ra", "Radium", 226.0, Alkaline, Solid, (6, 1), "2", "2", Some(5.5), "[Rn]7s2", Some(973.0), Some(1.0), Some(2010.0), Some(0.9), Some(113.0), Some(8.5), Some(0.01), Some(0.186), Some(0.094), Some(5.278), Some(45.2), Some(2.83), None, "body centered", "basic", "1898", "silvery white", 14474460, None, Some(38.3), Some(159.0), Some(0.0), Some(176.5), Some(71.0)),
    // Actinides (89..103 -> grid row 8, cols 3..17)
    elem!(89, "Ac", "Actinium", 227.0, Actinide, Solid, (8, 3), "3", "3", Some(10.07), "[Rn]6d17s2", Some(1323.0), Some(1.0), Some(3471.0), Some(1.1), Some(400.0), Some(14.0), None, Some(0.12), Some(0.12), Some(5.17), Some(22.5), Some(2.03), None, "face centered", "basic", "1899", "silvery", 13882323, None, Some(32.1), Some(406.0), Some(0.0), Some(188.0), Some(56.5)),
    elem!(90, "Th", "Thorium", 232.0381, Actinide, Solid, (8, 4), "4", "4", Some(11.72), "[Rn]6d27s2", Some(2023.0), Some(1.0), Some(5061.0), Some(1.3), Some(514.0), Some(13.81), Some(0.067), Some(0.54), Some(0.113), Some(6.307), Some(19.9), Some(1.8), Some(1.65), "face centered", "basic", "1828", "silvery white", 13882323, None, Some(32.1), Some(602.0), Some(0.0), Some(190.17), Some(51.8)),
    elem!(91, "Pa", "Protactinium", 231.0359, Actinide, Solid, (8, 5), "5", "5,4", Some(15.37), "[Rn]5f26d17s2", Some(1841.0), Some(1.0), Some(4300.0), Some(1.5), Some(481.0), Some(12.34), Some(0.056), Some(0.47), Some(0.12), Some(5.89), Some(15.0), Some(1.8), None, "tetragonal", "weak basic", "1913", "bright silvery", 13882323, None, Some(25.4), Some(607.0), Some(0.0), Some(198.0), Some(51.9)),
    elem!(92, "U", "Uranium", 238.0289, Actinide, Solid, (8, 6), "6", "6,5,4,3", Some(18.95), "[Rn]5f36d17s2", Some(1405.3), Some(1.0), Some(4404.0), Some(1.38), Some(417.1), Some(9.14), Some(0.036), Some(0.275), Some(0.116), Some(6.194), Some(12.5), Some(1.75), Some(1.42), "orthorhombic", "amphoteric", "1789", "silvery grey", 12500670, None, Some(24.9), Some(533.0), Some(0.0), Some(199.79), Some(50.2)),
    elem!(93, "Np", "Neptunium", 237.0, Actinide, Solid, (8, 7), "5", "6,5,4,3", Some(20.45), "[Rn]5f46d17s2", Some(917.0), Some(1.0), Some(4273.0), Some(1.36), Some(336.0), Some(5.19), Some(0.008), Some(0.063), None, Some(6.266), Some(11.6), Some(1.75), None, "orthorhombic", "amphoteric", "1940", "silvery metallic", 13158600, None, Some(24.8), Some(465.0), Some(0.0), Some(197.6), Some(50.5)),
    elem!(94, "Pu", "Plutonium", 244.0, Actinide, Solid, (8, 8), "4", "6,5,4,3", Some(19.84), "[Rn]5f67s2", Some(912.5), Some(1.0), Some(3501.0), Some(1.28), Some(333.5), Some(2.82), Some(0.007), Some(0.067), Some(0.13), Some(6.026), Some(12.3), Some(1.75), None, "monoclinic", "basic", "1940", "silvery white", 13158600, None, Some(24.5), Some(345.2), Some(0.0), Some(177.2), Some(56.1)),
    elem!(95, "Am", "Americium", 243.0, Actinide, Solid, (8, 9), "3", "6,5,4,3", Some(13.69), "[Rn]5f77s2", Some(1449.0), Some(1.0), Some(2880.0), Some(1.13), None, Some(14.39), Some(0.015), Some(0.1), None, Some(5.974), Some(17.8), Some(1.75), None, "hexagonal", "basic", "1944", "silvery white", 13882323, None, Some(23.3), Some(284.1), Some(0.0), Some(194.5), Some(55.4)),
    elem!(96, "Cm", "Curium", 247.0, Actinide, Solid, (8, 10), "3", "3,4", Some(13.51), "[Rn]5f76d17s2", Some(1613.0), Some(1.0), Some(3383.0), Some(1.28), None, Some(15.0), None, None, None, Some(5.991), Some(18.3), None, None, "hexagonal", "basic", "1944", "silvery", 13882323, None, Some(23.0), Some(387.4), Some(0.0), Some(197.3), Some(72.0)),
    elem!(97, "Bk", "Berkelium", 247.0, Actinide, Solid, (8, 11), "3", "3,4", Some(14.79), "[Rn]5f97s2", Some(1259.0), Some(1.0), Some(2900.0), Some(1.3), None, None, None, None, None, Some(6.198), Some(16.7), None, None, "hexagonal", "basic", "1949", "silvery", 13882323, None, Some(22.7), Some(310.0), Some(0.0), Some(200.6), Some(78.2)),
    elem!(98, "Cf", "Californium", 251.0, Actinide, Solid, (8, 12), "3", "3", Some(15.1), "[Rn]5f107s2", Some(1173.0), Some(1.0), Some(1743.0), Some(1.3), None, None, None, None, None, Some(6.282), Some(16.5), None, None, "hexagonal", "basic", "1950", "silvery", 13882323, None, Some(20.5), Some(196.2), Some(0.0), Some(200.3), Some(77.0)),
    elem!(99, "Es", "Einsteinium", 252.0, Actinide, Solid, (8, 13), "3", "3", Some(8.84), "[Rn]5f117s2", Some(1133.0), Some(1.0), Some(1269.0), Some(1.3), None, None, None, None, None, Some(6.42), None, None, None, "face centered", "basic", "1952", "silvery", 13882323, None, Some(19.7), Some(133.0), Some(0.0), Some(200.5), Some(89.5)),
    elem!(100, "Fm", "Fermium", 257.0, Actinide, Solid, (8, 14), "3", "3", None, "[Rn]5f127s2", Some(1800.0), Some(1.0), None, Some(1.3), None, None, None, None, None, Some(6.5), None, None, None, "-", "basic", "1952", "-", 13882323, None, Some(23.8), Some(130.0), Some(0.0), None, None),
    elem!(101, "Md", "Mendelevium", 258.0, Actinide, Solid, (8, 15), "3", "3,2", None, "[Rn]5f137s2", Some(1100.0), Some(1.0), None, Some(1.3), None, None, None, None, None, Some(6.58), None, None, None, "-", "basic", "1955", "-", 13882323, None, Some(18.2), Some(128.0), Some(0.0), None, None),
    elem!(102, "No", "Nobelium", 259.0, Actinide, Solid, (8, 16), "2", "3,2", None, "[Rn]5f147s2", Some(1100.0), Some(1.0), None, Some(1.3), None, None, None, None, None, Some(6.65), None, None, None, "-", "basic", "1958", "-", 13882323, None, Some(17.5), Some(126.0), Some(0.0), None, None),
    elem!(103, "Lr", "Lawrencium", 262.0, Actinide, Solid, (8, 17), "3", "3", None, "[Rn]5f147s27p1", Some(1900.0), Some(1.0), None, Some(1.3), None, None, None, None, None, Some(4.9), None, None, None, "-", "basic", "1961", "-", 13882323, None, Some(32.0), Some(341.0), Some(0.0), None, None),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workbook_validation_cases_e1_to_e5() {
        let wb = ElementWorkbook::default_workbook();
        assert_eq!(wb.element_symbols.len(), 103);

        // E1: Atomic weights IUPAC
        assert!((find_element_spec(1).unwrap().atomic_weight - 1.00794).abs() < 0.01);
        assert!((find_element_spec(6).unwrap().atomic_weight - 12.011).abs() < 0.01);
        assert!((find_element_spec(26).unwrap().atomic_weight - 55.845).abs() < 0.01);
        assert!((find_element_spec(29).unwrap().atomic_weight - 63.546).abs() < 0.01);
        assert!((find_element_spec(79).unwrap().atomic_weight - 196.967).abs() < 0.01);

        // E2: Density diagram max is Osmium (22.59) with normalized diameter == 1.0
        let dens_row = wb.rows.iter().position(|r| r.property == "Density").unwrap();
        let diag = wb.compute_diagram(dens_row).unwrap();
        let os = diag.points.iter().find(|p| p.symbol == "Os").unwrap();
        assert!((diag.max_val - 22.59).abs() < 0.05);
        assert!((os.norm_diameter - 1.0).abs() < 1e-9);
        assert!(diag.points.iter().all(|p| (0.0..=1.0).contains(&p.norm_diameter)));

        // E3: Pauling Electronegativity max is Fluorine (3.98) and He/Ne/Ar are None
        let en_row = wb
            .rows
            .iter()
            .position(|r| r.property == "Electronegativity")
            .unwrap();
        let en_diag = wb.compute_diagram(en_row).unwrap();
        assert!((en_diag.max_val - 3.98).abs() < 0.01);
        assert!(en_diag.points.iter().find(|p| p.symbol == "He").unwrap().raw_value.is_none());
        assert!(en_diag.points.iter().find(|p| p.symbol == "F").unwrap().norm_diameter > 0.99);

        // E4: Melting point check (Carbon sublimation/melting 3823 K, Tungsten max metal 3695 K)
        let tm_row = wb
            .rows
            .iter()
            .position(|r| r.property == "Melting Point")
            .unwrap();
        let tm_diag = wb.compute_diagram(tm_row).unwrap();
        assert!((tm_diag.max_val - 3823.0).abs() < 1.0);
        let w_pt = tm_diag.points.iter().find(|p| p.symbol == "W").unwrap();
        assert!((w_pt.raw_value.unwrap() - 3695.0).abs() < 1.0);
    }

    #[test]
    fn csv_roundtrip_contains_all_rows_and_symbols() {
        let mut wb = ElementWorkbook::default_workbook();
        wb.add_custom_row("Custom Price".into(), "USD/kg".into(), true);
        let csv = wb.to_csv();
        assert!(csv.contains("Property,Units,Diagram,H,He,Li"));
        assert!(csv.contains("Custom Price,USD/kg,TRUE"));
    }
}
