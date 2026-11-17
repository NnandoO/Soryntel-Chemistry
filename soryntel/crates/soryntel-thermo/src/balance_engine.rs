//! Module 2 Engine: BalanceWorkbook, Formula Evaluator, Cyclic Convergence,
//! Element/Total Balance, Temp. Balance (Brent), Target Dialog, Diagram Sweep, and Presets.

use crate::balance::{evaluate_balance_point_si, parse_hsc_species_elements};
use crate::{Phase, SpeciesRepository};
use serde::{Deserialize, Serialize};
use soryntel_core::{BalanceEnergyUnit, SoryntelError, TemperatureUnit, STP_MOLAR_VOL_L};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesRow {
    pub formula: String,
    pub temp_display: f64,
    pub temp_formula: Option<String>,
    pub pressure_bar: f64,
    pub amount_kmol: f64,
    pub amount_formula: Option<String>,
    // Computed outputs (in workbook display energy unit)
    pub mw: f64,
    pub amount_kg: f64,
    pub amount_nm3: f64,
    pub heat_content: f64,
    pub total_h: f64,
    pub hc_per_kmol: f64,
    pub tot_h_per_kmol: f64,
    pub ts: f64,
    pub g: f64,
    pub ex_phys: f64,
    pub ex_tot: f64,
    pub missing_in_db: bool,
}

impl SpeciesRow {
    pub fn new(formula: impl Into<String>, temp_display: f64, amount_kmol: f64) -> Self {
        Self {
            formula: formula.into(),
            temp_display,
            temp_formula: None,
            pressure_bar: 1.01325,
            amount_kmol,
            amount_formula: None,
            mw: 0.0,
            amount_kg: 0.0,
            amount_nm3: 0.0,
            heat_content: 0.0,
            total_h: 0.0,
            hc_per_kmol: 0.0,
            tot_h_per_kmol: 0.0,
            ts: 0.0,
            g: 0.0,
            ex_phys: 0.0,
            ex_tot: 0.0,
            missing_in_db: false,
        }
    }

    pub fn with_amount_formula(mut self, f: impl Into<String>) -> Self {
        self.amount_formula = Some(f.into());
        self
    }

    pub fn with_pressure(mut self, p_bar: f64) -> Self {
        self.pressure_bar = p_bar;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamGroup {
    pub name: String,
    pub stream_temp_display: f64,
    pub species: Vec<SpeciesRow>,
    // Computed stream sums
    pub sum_kmol: f64,
    pub sum_kg: f64,
    pub sum_nm3: f64,
    pub sum_heat_content: f64,
    pub sum_total_h: f64,
}

impl StreamGroup {
    pub fn new(name: impl Into<String>, stream_temp_display: f64, species: Vec<SpeciesRow>) -> Self {
        Self {
            name: name.into(),
            stream_temp_display,
            species,
            sum_kmol: 0.0,
            sum_kg: 0.0,
            sum_nm3: 0.0,
            sum_heat_content: 0.0,
            sum_total_h: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceArea {
    pub name: String,
    pub in_streams: Vec<StreamGroup>,
    pub extra_heat: f64, // Extra Heat on INx (display energy unit)
    pub out_streams: Vec<StreamGroup>,
    pub heat_losses: f64, // Heat Losses on OUTx (display energy unit)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ElementBalanceRow {
    pub element: String,
    pub in_kmol: f64,
    pub out_kmol: f64,
    pub balance_kmol: f64,
    pub in_kg: f64,
    pub out_kg: f64,
    pub balance_kg: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SheetTotals {
    pub kmol: f64,
    pub kg: f64,
    pub nm3: f64,
    pub heat_content: f64,
    pub total_h: f64,
    pub ts: f64,
    pub g: f64,
    pub ex_phys: f64,
    pub ex_tot: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AreaBalanceSummary {
    pub area_index: usize,
    pub area_name: String,
    pub in_totals: SheetTotals,
    pub out_totals: SheetTotals,
    /// HSC BALANCE convention: OUT - IN
    pub balance_totals: SheetTotals,
    pub elements: Vec<ElementBalanceRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamElementRow {
    pub element: String,
    pub kmol: f64,
    pub kg: f64,
    pub mol_pct: f64,
    pub wt_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamComposition {
    pub stream_name: String,
    pub temp_display: f64,
    pub total_kmol: f64,
    pub total_kg: f64,
    pub elements: Vec<StreamElementRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRow {
    pub name: String,
    /// e.g. "IN1!D11" or "IN1!STREAM:GAS:TEMP"
    pub var_cell_ref: String,
    pub test_value: f64,
    pub min_val: f64,
    pub max_val: f64,
    pub target_name: String,
    /// e.g. "BALANCE1!H4" (Total H Balance) or "EL1!Cu:IN"
    pub target_cell_ref: String,
    pub current_target_val: f64,
    pub goal_value: f64,
    pub max_iterations: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramSeriesDef {
    pub cell_ref: String,
    pub label: String,
    pub units: String,
    pub target_rows: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramConfig {
    pub title: String,
    pub x_min: f64,
    pub x_max: f64,
    pub x_step: f64,
    pub x_cell_ref: String,
    pub x_label: String,
    pub y_series: Vec<DiagramSeriesDef>,
    pub use_target_iteration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    pub title: String,
    pub x_label: String,
    pub x_values: Vec<f64>,
    pub series: Vec<(String, String, Vec<f64>)>, // (label, units, y_values)
}

/// Custom Excel-like worksheet inside a Heat & Material Balance workbook (Ch. 11.6 & 11.9, Figs. 6, 13, 14).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceUserSheet {
    pub name: String,
    #[serde(with = "cell_map_serde")]
    pub cells: BTreeMap<(usize, usize), String>,
    pub red_cells: BTreeSet<(usize, usize)>,
}

mod cell_map_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(
        map: &BTreeMap<(usize, usize), String>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let entries: Vec<(usize, usize, &String)> =
            map.iter().map(|(&(r, c), v)| (r, c, v)).collect();
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<(usize, usize), String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries: Vec<(usize, usize, String)> = Vec::deserialize(deserializer)?;
        Ok(entries.into_iter().map(|(r, c, v)| ((r, c), v)).collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceWorkbook {
    pub title: String,
    pub temp_unit: TemperatureUnit,
    pub energy_unit: BalanceEnergyUnit,
    pub h2o_pressure_corr: bool,
    #[serde(default)]
    pub red_font_shield: bool,
    pub areas: Vec<BalanceArea>,
    pub target_rows: Vec<TargetRow>,
    pub diagram_config: DiagramConfig,
    #[serde(default)]
    pub user_sheets: Vec<BalanceUserSheet>,
    // Computed summaries
    #[serde(skip)]
    pub summaries: Vec<AreaBalanceSummary>,
    #[serde(skip)]
    pub circular_iterations: usize,
    #[serde(skip)]
    pub circular_converged: bool,
}

fn atomic_weight(el: &str) -> f64 {
    crate::species::awdb_atomic_weight(el).unwrap_or(0.0)
}

/// Evaluate all areas, formulas, cyclic references, element balances, and total balances.
pub fn evaluate_workbook(
    wb: &mut BalanceWorkbook,
    repo: &dyn SpeciesRepository,
) -> Result<(), SoryntelError> {
    let e_factor = wb.energy_unit.from_mj_factor();
    let max_passes = 40;
    let mut converged = false;
    let mut passes = 0;

    for pass in 0..max_passes {
        passes = pass + 1;
        let mut max_delta = 0.0_f64;

        // Build lookup snapshot of element totals and sheet cells before evaluating formulas
        let elem_vars = collect_element_vars(wb);
        let cell_map = build_cell_value_map(wb);

        for area in &mut wb.areas {
            for is_out in [false, true] {
                let streams = if is_out {
                    &mut area.out_streams
                } else {
                    &mut area.in_streams
                };
                for stream in streams.iter_mut() {
                    let mut s_kmol = 0.0;
                    let mut s_kg = 0.0;
                    let mut s_nm3 = 0.0;
                    let mut s_hc = 0.0;
                    let mut s_th = 0.0;

                    for sp in stream.species.iter_mut() {
                        if let Some(ref tf) = sp.temp_formula {
                            if let Ok(val) = eval_formula_expr(tf, &cell_map, &elem_vars) {
                                sp.temp_display = val;
                            }
                        }
                        if let Some(ref af) = sp.amount_formula {
                            if let Ok(val) = eval_formula_expr(af, &cell_map, &elem_vars) {
                                let clamped = val.max(0.0);
                                max_delta = max_delta.max((clamped - sp.amount_kmol).abs());
                                sp.amount_kmol = clamped;
                            }
                        }

                        let t_k = wb.temp_unit.to_kelvin(sp.temp_display).max(1.0);
                        if let Some(species_rec) = repo.get_species(&sp.formula) {
                            sp.missing_in_db = false;
                            sp.mw = species_rec.molecular_weight;
                            sp.amount_kg = sp.amount_kmol * sp.mw;
                            sp.amount_nm3 = if species_rec.phase == Phase::Gas {
                                sp.amount_kmol * STP_MOLAR_VOL_L
                            } else if species_rec.density_g_cm3 > 0.0 {
                                sp.amount_kg / (species_rec.density_g_cm3 * 1000.0)
                            } else {
                                0.0
                            };

                            if let Ok(pt) = evaluate_balance_point_si(
                                repo,
                                &species_rec,
                                t_k,
                                sp.pressure_bar,
                                wb.h2o_pressure_corr,
                                false,
                            ) {
                                sp.hc_per_kmol = pt.h_cont_mj_kmol * e_factor;
                                sp.tot_h_per_kmol = pt.h_tot_mj_kmol * e_factor;
                                sp.heat_content = sp.amount_kmol * sp.hc_per_kmol;
                                sp.total_h = sp.amount_kmol * sp.tot_h_per_kmol;
                                sp.ts = sp.amount_kmol * pt.ts_mj_kmol * e_factor;
                                sp.g = sp.amount_kmol * pt.g_mj_kmol * e_factor;
                                sp.ex_phys = sp.amount_kmol * pt.ex_phys_mj_kmol * e_factor;
                                sp.ex_tot = sp.amount_kmol * pt.ex_tot_mj_kmol * e_factor;
                            }
                        } else {
                            sp.missing_in_db = true;
                        }

                        s_kmol += sp.amount_kmol;
                        s_kg += sp.amount_kg;
                        s_nm3 += sp.amount_nm3;
                        s_hc += sp.heat_content;
                        s_th += sp.total_h;
                    }

                    stream.sum_kmol = s_kmol;
                    stream.sum_kg = s_kg;
                    stream.sum_nm3 = s_nm3;
                    stream.sum_heat_content = s_hc;
                    stream.sum_total_h = s_th;
                }
            }
        }

        if max_delta < 1e-9 {
            converged = true;
            break;
        }
    }

    wb.circular_iterations = passes;
    wb.circular_converged = converged;

    // Build per-area summaries (Element Balance + Total Balance)
    let mut summaries = Vec::with_capacity(wb.areas.len());
    for (idx, area) in wb.areas.iter().enumerate() {
        let mut in_tot = SheetTotals::default();
        let mut out_tot = SheetTotals::default();
        let mut elem_in: BTreeMap<String, f64> = BTreeMap::new();
        let mut elem_out: BTreeMap<String, f64> = BTreeMap::new();
        let mut all_elems: BTreeSet<String> = BTreeSet::new();

        for stream in &area.in_streams {
            for sp in &stream.species {
                in_tot.kmol += sp.amount_kmol;
                in_tot.kg += sp.amount_kg;
                in_tot.nm3 += sp.amount_nm3;
                in_tot.heat_content += sp.heat_content;
                in_tot.total_h += sp.total_h;
                in_tot.ts += sp.ts;
                in_tot.g += sp.g;
                in_tot.ex_phys += sp.ex_phys;
                in_tot.ex_tot += sp.ex_tot;

                if let Ok(comp) = parse_hsc_species_elements(&sp.formula) {
                    for (el, nu) in comp {
                        all_elems.insert(el.clone());
                        *elem_in.entry(el).or_insert(0.0) += sp.amount_kmol * nu;
                    }
                }
            }
        }
        in_tot.total_h += area.extra_heat;

        for stream in &area.out_streams {
            for sp in &stream.species {
                out_tot.kmol += sp.amount_kmol;
                out_tot.kg += sp.amount_kg;
                out_tot.nm3 += sp.amount_nm3;
                out_tot.heat_content += sp.heat_content;
                out_tot.total_h += sp.total_h;
                out_tot.ts += sp.ts;
                out_tot.g += sp.g;
                out_tot.ex_phys += sp.ex_phys;
                out_tot.ex_tot += sp.ex_tot;

                if let Ok(comp) = parse_hsc_species_elements(&sp.formula) {
                    for (el, nu) in comp {
                        all_elems.insert(el.clone());
                        *elem_out.entry(el).or_insert(0.0) += sp.amount_kmol * nu;
                    }
                }
            }
        }
        out_tot.total_h += area.heat_losses;

        let bal_tot = SheetTotals {
            kmol: out_tot.kmol - in_tot.kmol,
            kg: out_tot.kg - in_tot.kg,
            nm3: out_tot.nm3 - in_tot.nm3,
            heat_content: out_tot.heat_content - in_tot.heat_content,
            total_h: out_tot.total_h - in_tot.total_h,
            ts: out_tot.ts - in_tot.ts,
            g: out_tot.g - in_tot.g,
            ex_phys: out_tot.ex_phys - in_tot.ex_phys,
            ex_tot: out_tot.ex_tot - in_tot.ex_tot,
        };

        let elements = all_elems
            .into_iter()
            .map(|el| {
                let i_kmol = *elem_in.get(&el).unwrap_or(&0.0);
                let o_kmol = *elem_out.get(&el).unwrap_or(&0.0);
                let aw = atomic_weight(&el);
                ElementBalanceRow {
                    element: el,
                    in_kmol: i_kmol,
                    out_kmol: o_kmol,
                    balance_kmol: i_kmol - o_kmol,
                    in_kg: i_kmol * aw,
                    out_kg: o_kmol * aw,
                    balance_kg: (i_kmol - o_kmol) * aw,
                }
            })
            .collect();

        summaries.push(AreaBalanceSummary {
            area_index: idx,
            area_name: area.name.clone(),
            in_totals: in_tot,
            out_totals: out_tot,
            balance_totals: bal_tot,
            elements,
        });
    }

    wb.summaries = summaries;
    Ok(())
}

/// Generate `Inx-%` / `Outx-%` elemental stream compositions for an area.
pub fn compute_stream_compositions(area: &BalanceArea, is_out: bool) -> Vec<StreamComposition> {
    let streams = if is_out {
        &area.out_streams
    } else {
        &area.in_streams
    };
    let mut out = Vec::with_capacity(streams.len());
    for stream in streams {
        let mut el_kmol: BTreeMap<String, f64> = BTreeMap::new();
        for sp in &stream.species {
            if let Ok(comp) = parse_hsc_species_elements(&sp.formula) {
                for (el, nu) in comp {
                    *el_kmol.entry(el).or_insert(0.0) += sp.amount_kmol * nu;
                }
            }
        }
        let total_kmol: f64 = el_kmol.values().sum();
        let total_kg: f64 = el_kmol
            .iter()
            .map(|(el, &km)| km * atomic_weight(el))
            .sum();

        let elements = el_kmol
            .into_iter()
            .map(|(el, km)| {
                let kg = km * atomic_weight(&el);
                StreamElementRow {
                    element: el,
                    kmol: km,
                    kg,
                    mol_pct: if total_kmol > 0.0 {
                        100.0 * km / total_kmol
                    } else {
                        0.0
                    },
                    wt_pct: if total_kg > 0.0 {
                        100.0 * kg / total_kg
                    } else {
                        0.0
                    },
                }
            })
            .collect();

        out.push(StreamComposition {
            stream_name: stream.name.clone(),
            temp_display: stream.stream_temp_display,
            total_kmol,
            total_kg,
            elements,
        });
    }
    out
}

/// Solve Adiabatic Product Temperature (`Temp. Balance`) for `area_idx` using Brent's method.
/// Returns the adiabatic temperature in the workbook's display temperature unit.
pub fn solve_adiabatic_temperature(
    wb: &BalanceWorkbook,
    area_idx: usize,
    repo: &dyn SpeciesRepository,
) -> Result<f64, SoryntelError> {
    let mut trial = wb.clone();
    evaluate_workbook(&mut trial, repo)?;
    let area = trial
        .areas
        .get(area_idx)
        .ok_or_else(|| SoryntelError::InvalidCellReference(format!("Area {area_idx}")))?;

    let target_in_h = trial.summaries[area_idx].in_totals.total_h;
    let heat_losses = area.heat_losses;
    let e_factor = trial.energy_unit.from_mj_factor();
    let out_species: Vec<(String, f64, f64)> = area
        .out_streams
        .iter()
        .flat_map(|s| {
            s.species
                .iter()
                .map(|sp| (sp.formula.clone(), sp.amount_kmol, sp.pressure_bar))
        })
        .collect();

    if out_species.is_empty() {
        return Err(SoryntelError::BalanceError(
            "No output species in selected balance area".into(),
        ));
    }

    let eval_f = |t_k: f64| -> Result<f64, SoryntelError> {
        let mut h_out = heat_losses;
        for (formula, kmol, p_bar) in &out_species {
            if let Some(sp) = repo.get_species(formula) {
                let pt = evaluate_balance_point_si(
                    repo,
                    &sp,
                    t_k,
                    *p_bar,
                    trial.h2o_pressure_corr,
                    false,
                )?;
                h_out += kmol * pt.h_tot_mj_kmol * e_factor;
            }
        }
        Ok(h_out - target_in_h)
    };

    let t_star_k = brent_root(eval_f, 200.0, 5000.0, 1e-7)?;
    Ok(wb.temp_unit.from_kelvin(t_star_k))
}

/// Robust Brent root-finder on `[a, b]`.
fn brent_root<F>(mut f: F, mut a: f64, mut b: f64, tol: f64) -> Result<f64, SoryntelError>
where
    F: FnMut(f64) -> Result<f64, SoryntelError>,
{
    let mut fa = f(a)?;
    let mut fb = f(b)?;
    if fa * fb > 0.0 {
        return Err(SoryntelError::TargetNotBracketed(format!(
            "No sign change in [{a}, {b}]: f(a)={fa:.4}, f(b)={fb:.4}"
        )));
    }
    if fa.abs() < fb.abs() {
        std::mem::swap(&mut a, &mut b);
        std::mem::swap(&mut fa, &mut fb);
    }
    let mut c = a;
    let mut fc = fa;
    let mut mflag = true;
    let mut d = 0.0;

    for _ in 0..100 {
        if fb.abs() < 1e-9 || (b - a).abs() < tol {
            return Ok(b);
        }
        let s = if (fa - fc).abs() > 1e-15 && (fb - fc).abs() > 1e-15 {
            // Inverse quadratic interpolation
            a * fb * fc / ((fa - fb) * (fa - fc))
                + b * fa * fc / ((fb - fa) * (fb - fc))
                + c * fa * fb / ((fc - fa) * (fc - fb))
        } else {
            b - fb * (b - a) / (fb - fa)
        };

        let cond1 = !(s > (3.0 * a + b) / 4.0 && s < b || s < (3.0 * a + b) / 4.0 && s > b);
        let cond2 = mflag && (s - b).abs() >= (b - c).abs() / 2.0;
        let cond3 = !mflag && (s - b).abs() >= (c - d).abs() / 2.0;
        let cond4 = mflag && (b - c).abs() < tol;
        let cond5 = !mflag && (c - d).abs() < tol;

        let s = if cond1 || cond2 || cond3 || cond4 || cond5 {
            mflag = true;
            0.5 * (a + b)
        } else {
            mflag = false;
            s
        };

        let fs = f(s)?;
        d = c;
        c = b;
        fc = fb;
        if fa * fs < 0.0 {
            b = s;
            fb = fs;
        } else {
            a = s;
            fa = fs;
        }
        if fa.abs() < fb.abs() {
            std::mem::swap(&mut a, &mut b);
            std::mem::swap(&mut fa, &mut fb);
        }
    }
    Ok(b)
}

/// Set a workbook cell value by reference string (e.g. "IN1!D11", "IN1!STREAM:GAS:TEMP", "IN1!EXTRA_HEAT").
pub fn set_cell_value(wb: &mut BalanceWorkbook, cell_ref: &str, val: f64) -> Result<(), SoryntelError> {
    let norm = cell_ref.trim().replace('\'', "");
    let Some((sheet_part, loc_part)) = norm.split_once('!') else {
        return Err(SoryntelError::InvalidCellReference(cell_ref.into()));
    };
    let sheet_upper = sheet_part.to_uppercase();

    // Check user_sheets first (e.g. "FEED!C11", "Analysis!L29")
    if let Some(us) = wb
        .user_sheets
        .iter_mut()
        .find(|s| s.name.eq_ignore_ascii_case(sheet_part.trim()))
    {
        let (col, row) = parse_a1_ref(loc_part)?;
        let c_idx = (col as u8 - b'A') as usize;
        let r_idx = row.saturating_sub(1);
        us.cells.insert((r_idx, c_idx), format!("{val}"));
        return Ok(());
    }

    let is_out = sheet_upper.starts_with("OUT");
    let area_num: usize = sheet_upper
        .trim_start_matches("IN")
        .trim_start_matches("OUT")
        .parse()
        .unwrap_or(1);
    let area_idx = area_num.saturating_sub(1);
    let area = wb
        .areas
        .get_mut(area_idx)
        .ok_or_else(|| SoryntelError::InvalidCellReference(cell_ref.into()))?;

    if loc_part.eq_ignore_ascii_case("EXTRA_HEAT") {
        area.extra_heat = val;
        return Ok(());
    }
    if loc_part.eq_ignore_ascii_case("HEAT_LOSSES") {
        area.heat_losses = val;
        return Ok(());
    }

    // Stream temperature reference: STREAM:<Name>:TEMP
    if let Some(rest) = loc_part.strip_prefix("STREAM:") {
        if let Some(stream_name) = rest.strip_suffix(":TEMP") {
            let streams = if is_out {
                &mut area.out_streams
            } else {
                &mut area.in_streams
            };
            for st in streams {
                if st.name.trim_end_matches(':') == stream_name.trim_end_matches(':') {
                    st.stream_temp_display = val;
                    for sp in &mut st.species {
                        sp.temp_display = val;
                    }
                    return Ok(());
                }
            }
        }
    }

    // Excel-style row/col like B3, D11
    let (col, row) = parse_a1_ref(loc_part)?;
    let streams = if is_out {
        &mut area.out_streams
    } else {
        &mut area.in_streams
    };
    // Row 1 is header; Row 2+ walks stream header + species rows
    let mut cur_row = 2usize;
    for st in streams {
        if cur_row == row {
            match col {
                'B' => {
                    st.stream_temp_display = val;
                    for sp in &mut st.species {
                        sp.temp_display = val;
                    }
                }
                'C' => {
                    for sp in &mut st.species {
                        sp.pressure_bar = val.max(1e-4);
                    }
                }
                'D' => scale_stream_by_total_kmol(st, val),
                'E' => scale_stream_by_total_kg(st, val),
                'F' => scale_stream_by_total_nm3(st, val),
                _ => {}
            }
            return Ok(());
        }
        cur_row += 1;
        for sp in &mut st.species {
            if cur_row == row {
                match col {
                    'B' => sp.temp_display = val,
                    'C' => sp.pressure_bar = val,
                    'D' => {
                        sp.amount_formula = None;
                        sp.amount_kmol = val.max(0.0);
                    }
                    'E' => set_species_amount_from_kg(sp, val),
                    'F' => {
                        sp.amount_formula = None;
                        sp.amount_kmol = (val.max(0.0) / STP_MOLAR_VOL_L).max(0.0);
                    }
                    _ => {}
                }
                return Ok(());
            }
            cur_row += 1;
        }
    }
    Err(SoryntelError::InvalidCellReference(cell_ref.into()))
}

/// Read a workbook cell or summary value by reference string.
pub fn get_cell_value(wb: &BalanceWorkbook, cell_ref: &str) -> Result<f64, SoryntelError> {
    let norm = cell_ref.trim().replace('\'', "");
    let Some((sheet_part, loc_part)) = norm.split_once('!') else {
        return Err(SoryntelError::InvalidCellReference(cell_ref.into()));
    };
    let sheet_upper = sheet_part.to_uppercase();

    // BALANCE1!H4 -> Total H Balance of Area 1
    if sheet_upper.starts_with("BALANCE") {
        let area_num: usize = sheet_upper.trim_start_matches("BALANCE").parse().unwrap_or(1);
        let idx = area_num.saturating_sub(1);
        if let Some(sum) = wb.summaries.get(idx) {
            return match loc_part.to_uppercase().as_str() {
                "D4" | "KMOL" => Ok(sum.balance_totals.kmol),
                "E4" | "KG" => Ok(sum.balance_totals.kg),
                "G4" | "HEAT_CONTENT" => Ok(sum.balance_totals.heat_content),
                "H4" | "TOTAL_H" => Ok(sum.balance_totals.total_h),
                "EXERGY" => Ok(sum.balance_totals.ex_tot),
                _ => Ok(sum.balance_totals.total_h),
            };
        }
    }

    let map = build_cell_value_map(wb);
    map.get(&norm.to_uppercase())
        .copied()
        .ok_or_else(|| SoryntelError::InvalidCellReference(cell_ref.into()))
}

/// Iterate selected rows (or all rows) of Target Dialog using bounded Brent + Gauss-Seidel outer loop.
pub fn iterate_target_rows(
    wb: &mut BalanceWorkbook,
    row_indices: &[usize],
    repo: &dyn SpeciesRepository,
) -> Result<(), SoryntelError> {
    let max_outer = wb
        .target_rows
        .iter()
        .filter(|r| !r.var_cell_ref.is_empty())
        .map(|r| r.max_iterations.max(1))
        .max()
        .unwrap_or(10);

    for _outer in 0..max_outer {
        let mut max_err = 0.0_f64;
        for &r_idx in row_indices {
            let Some(row) = wb.target_rows.get(r_idx).cloned() else {
                continue;
            };
            if row.var_cell_ref.is_empty() || row.target_cell_ref.is_empty() {
                continue;
            }
            let goal = row.goal_value;
            let var_ref = row.var_cell_ref.clone();
            let tgt_ref = row.target_cell_ref.clone();

            let mut eval_res = |x: f64| -> Result<f64, SoryntelError> {
                set_cell_value(wb, &var_ref, x)?;
                evaluate_workbook(wb, repo)?;
                let y = get_cell_value(wb, &tgt_ref)?;
                Ok(y - goal)
            };

            let x_sol = match brent_root(&mut eval_res, row.min_val, row.max_val, 1e-6) {
                Ok(x) => x,
                Err(_) => {
                    // Fallback: secant / midpoint best within [min_val, max_val]
                    let f_min = eval_res(row.min_val)?.abs();
                    let f_max = eval_res(row.max_val)?.abs();
                    if f_min < f_max {
                        row.min_val
                    } else {
                        row.max_val
                    }
                }
            };
            set_cell_value(wb, &var_ref, x_sol)?;
            evaluate_workbook(wb, repo)?;
            let cur_y = get_cell_value(wb, &tgt_ref)?;
            max_err = max_err.max((cur_y - goal).abs());
            if let Some(tr) = wb.target_rows.get_mut(r_idx) {
                tr.test_value = x_sol;
                tr.current_target_val = cur_y;
            }
        }
        if max_err < 1e-5 {
            break;
        }
    }
    Ok(())
}

/// Run 1D parametric sweep for Diagram Settings (with optional nested Target iteration).
pub fn run_diagram_sweep(
    wb: &BalanceWorkbook,
    repo: &dyn SpeciesRepository,
) -> Result<DiagramResult, SoryntelError> {
    let cfg = &wb.diagram_config;
    if cfg.x_step <= 0.0 || cfg.x_max < cfg.x_min {
        return Err(SoryntelError::InvalidRange {
            from: cfg.x_min,
            to: cfg.x_max,
            step: cfg.x_step,
        });
    }

    let mut x_values = Vec::new();
    let mut y_vecs: Vec<Vec<f64>> = vec![Vec::new(); cfg.y_series.len()];
    let mut trial = wb.clone();

    let n_steps = (((cfg.x_max - cfg.x_min) / cfg.x_step).floor() as usize).min(2000);
    for k in 0..=n_steps {
        let x = cfg.x_min + (k as f64) * cfg.x_step;
        if x > cfg.x_max + 1e-9 {
            break;
        }
        set_cell_value(&mut trial, &cfg.x_cell_ref, x)?;
        evaluate_workbook(&mut trial, repo)?;

        if cfg.use_target_iteration {
            let mut rows: Vec<usize> = cfg
                .y_series
                .iter()
                .flat_map(|s| s.target_rows.iter().copied())
                .collect();
            rows.sort_unstable();
            rows.dedup();
            if !rows.is_empty() {
                iterate_target_rows(&mut trial, &rows, repo)?;
            }
        }

        x_values.push(x);
        for (i, s) in cfg.y_series.iter().enumerate() {
            let y = get_cell_value(&trial, &s.cell_ref).unwrap_or(0.0);
            y_vecs[i].push(y);
        }
    }

    let series = cfg
        .y_series
        .iter()
        .zip(y_vecs)
        .map(|(def, ys)| (def.label.clone(), def.units.clone(), ys))
        .collect();

    Ok(DiagramResult {
        title: cfg.title.clone(),
        x_label: cfg.x_label.clone(),
        x_values,
        series,
    })
}

// ============================================================================
// Internal Formula & Cell Reference Helpers
// ============================================================================

fn parse_a1_ref(s: &str) -> Result<(char, usize), SoryntelError> {
    let up = s.trim().to_uppercase();
    let mut chars = up.chars();
    let Some(col) = chars.next() else {
        return Err(SoryntelError::InvalidCellReference(s.into()));
    };
    if !col.is_ascii_alphabetic() {
        return Err(SoryntelError::InvalidCellReference(s.into()));
    }
    let row: usize = chars
        .as_str()
        .parse()
        .map_err(|_| SoryntelError::InvalidCellReference(s.into()))?;
    Ok((col, row))
}

fn collect_element_vars(wb: &BalanceWorkbook) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for (idx, area) in wb.areas.iter().enumerate() {
        let a_num = idx + 1;
        let mut el_in: BTreeMap<String, f64> = BTreeMap::new();
        let mut el_out: BTreeMap<String, f64> = BTreeMap::new();

        for st in &area.in_streams {
            for sp in &st.species {
                if let Ok(comp) = parse_hsc_species_elements(&sp.formula) {
                    for (el, nu) in comp {
                        *el_in.entry(el).or_insert(0.0) += sp.amount_kmol * nu;
                    }
                }
            }
        }
        for st in &area.out_streams {
            for sp in &st.species {
                if let Ok(comp) = parse_hsc_species_elements(&sp.formula) {
                    for (el, nu) in comp {
                        *el_out.entry(el).or_insert(0.0) += sp.amount_kmol * nu;
                    }
                }
            }
        }

        for (el, val) in el_in {
            if a_num == 1 {
                map.insert(format!("IN{el}").to_uppercase(), val);
            }
            map.insert(format!("IN{a_num}{el}").to_uppercase(), val);
            map.insert(format!("IN{el}{a_num}").to_uppercase(), val);
        }
        for (el, val) in el_out {
            if a_num == 1 {
                map.insert(format!("OUT{el}").to_uppercase(), val);
            }
            map.insert(format!("OUT{a_num}{el}").to_uppercase(), val);
            map.insert(format!("OUT{el}{a_num}").to_uppercase(), val);
        }
    }
    map
}

fn build_cell_value_map(wb: &BalanceWorkbook) -> BTreeMap<String, f64> {
    let mut map = BTreeMap::new();
    for (idx, area) in wb.areas.iter().enumerate() {
        let n = idx + 1;
        for (is_out, prefix) in [(false, format!("IN{n}")), (true, format!("OUT{n}"))] {
            let streams = if is_out {
                &area.out_streams
            } else {
                &area.in_streams
            };
            let mut row = 2usize;
            for st in streams {
                map.insert(format!("{prefix}!B{row}"), st.stream_temp_display);
                map.insert(format!("{prefix}!D{row}"), st.sum_kmol);
                map.insert(format!("{prefix}!E{row}"), st.sum_kg);
                map.insert(format!("{prefix}!F{row}"), st.sum_nm3);
                map.insert(format!("{prefix}!G{row}"), st.sum_heat_content);
                map.insert(format!("{prefix}!H{row}"), st.sum_total_h);
                row += 1;
                for sp in &st.species {
                    map.insert(format!("{prefix}!B{row}"), sp.temp_display);
                    map.insert(format!("{prefix}!C{row}"), sp.pressure_bar);
                    map.insert(format!("{prefix}!D{row}"), sp.amount_kmol);
                    map.insert(format!("{prefix}!E{row}"), sp.amount_kg);
                    map.insert(format!("{prefix}!F{row}"), sp.amount_nm3);
                    map.insert(format!("{prefix}!G{row}"), sp.heat_content);
                    map.insert(format!("{prefix}!H{row}"), sp.total_h);
                    row += 1;
                }
            }
        }
    }
    // Include numeric cells from custom user_sheets (e.g. FEED!C11, Analysis!L29)
    for us in &wb.user_sheets {
        let up_name = us.name.to_uppercase();
        for (&(r, c), txt) in &us.cells {
            if let Ok(v) = txt.trim().parse::<f64>() {
                let col_ch = (b'A' + (c as u8).min(25)) as char;
                map.insert(format!("{up_name}!{col_ch}{}", r + 1), v);
            }
        }
    }
    map
}

/// Recursive-descent arithmetic & cell formula evaluator (`=0.938*(InCu-D13)/2`, `='OUT4'!D12`).
fn eval_formula_expr(
    formula: &str,
    cells: &BTreeMap<String, f64>,
    elems: &BTreeMap<String, f64>,
) -> Result<f64, SoryntelError> {
    let trimmed = formula.trim().trim_start_matches('=').trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    let mut p = FormulaParser {
        src: trimmed.as_bytes(),
        pos: 0,
        cells,
        elems,
    };
    p.parse_expr()
}

struct FormulaParser<'a> {
    src: &'a [u8],
    pos: usize,
    cells: &'a BTreeMap<String, f64>,
    elems: &'a BTreeMap<String, f64>,
}

impl<'a> FormulaParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.src.len() && self.src[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.src.get(self.pos).copied()
    }

    fn parse_expr(&mut self) -> Result<f64, SoryntelError> {
        let mut lhs = self.parse_term()?;
        while let Some(op) = self.peek() {
            if op == b'+' || op == b'-' {
                self.pos += 1;
                let rhs = self.parse_term()?;
                lhs = if op == b'+' { lhs + rhs } else { lhs - rhs };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<f64, SoryntelError> {
        let mut lhs = self.parse_power()?;
        while let Some(op) = self.peek() {
            if op == b'*' || op == b'/' {
                self.pos += 1;
                let rhs = self.parse_power()?;
                lhs = if op == b'*' {
                    lhs * rhs
                } else if rhs.abs() > 1e-18 {
                    lhs / rhs
                } else {
                    0.0
                };
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_power(&mut self) -> Result<f64, SoryntelError> {
        let base = self.parse_unary()?;
        if self.peek() == Some(b'^') {
            self.pos += 1;
            let exp = self.parse_power()?;
            return Ok(base.powf(exp));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, SoryntelError> {
        match self.peek() {
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.parse_unary()?)
            }
            Some(b'+') => {
                self.pos += 1;
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, SoryntelError> {
        self.skip_ws();
        if self.pos >= self.src.len() {
            return Err(SoryntelError::FormulaSyntaxError("Unexpected end".into()));
        }
        if self.src[self.pos] == b'(' {
            self.pos += 1;
            let v = self.parse_expr()?;
            if self.peek() == Some(b')') {
                self.pos += 1;
            }
            return Ok(v);
        }
        // Quoted sheet ref `'OUT4'!D12`
        let start = self.pos;
        if self.src[self.pos] == b'\'' {
            self.pos += 1;
            while self.pos < self.src.len() && self.src[self.pos] != b'\'' {
                self.pos += 1;
            }
            if self.pos < self.src.len() {
                self.pos += 1;
            }
            while self.pos < self.src.len()
                && (self.src[self.pos].is_ascii_alphanumeric()
                    || self.src[self.pos] == b'!'
                    || self.src[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let ident = std::str::from_utf8(&self.src[start..self.pos])
                .unwrap_or("")
                .replace('\'', "")
                .to_uppercase();
            return Ok(self.cells.get(&ident).copied().unwrap_or(0.0));
        }

        // Number
        if self.src[self.pos].is_ascii_digit() || self.src[self.pos] == b'.' {
            while self.pos < self.src.len()
                && (self.src[self.pos].is_ascii_digit() || self.src[self.pos] == b'.')
            {
                self.pos += 1;
            }
            let num_str = std::str::from_utf8(&self.src[start..self.pos]).unwrap_or("0");
            return num_str
                .parse::<f64>()
                .map_err(|e| SoryntelError::FormulaSyntaxError(e.to_string()));
        }

        // Identifier or function call (`ABS(...)`, `InCu`, `OUT1!D13`)
        while self.pos < self.src.len()
            && (self.src[self.pos].is_ascii_alphanumeric()
                || self.src[self.pos] == b'!'
                || self.src[self.pos] == b'_')
        {
            self.pos += 1;
        }
        let ident = std::str::from_utf8(&self.src[start..self.pos])
            .unwrap_or("")
            .to_uppercase();

        if self.peek() == Some(b'(') {
            self.pos += 1;
            let mut args = Vec::new();
            if self.peek() != Some(b')') {
                loop {
                    args.push(self.parse_expr()?);
                    if self.peek() == Some(b',') {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            if self.peek() == Some(b')') {
                self.pos += 1;
            }
            return match ident.as_str() {
                "SUM" => Ok(args.iter().sum()),
                "ABS" => Ok(args.first().copied().unwrap_or(0.0).abs()),
                "MIN" => Ok(args.iter().copied().fold(f64::INFINITY, f64::min)),
                "MAX" => Ok(args.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
                _ => Ok(0.0),
            };
        }

        if let Some(&v) = self.cells.get(&ident) {
            return Ok(v);
        }
        if let Some(&v) = self.elems.get(&ident) {
            return Ok(v);
        }
        // Fallback: local cell ref in IN1/OUT1 if missing sheet prefix
        if let Some(&v) = self.cells.get(&format!("OUT1!{ident}")) {
            return Ok(v);
        }
        if let Some(&v) = self.cells.get(&format!("IN1!{ident}")) {
            return Ok(v);
        }
        Ok(0.0)
    }
}

// ============================================================================
// Built-in Presets (Manual & HSC6 Balance Examples)
// ============================================================================

/// Fig. 1–5 (`CUCONV1.bal8`): Copper Converter example with `=0.938*(InCu-D13)/2`.
pub fn preset_cuconv1() -> BalanceWorkbook {
    let mut feed_cells = BTreeMap::new();
    let mut feed_red = BTreeSet::new();
    feed_cells.insert((0, 0), "Copper Converter Feed".into());
    feed_cells.insert((1, 0), "GAS:".into());
    feed_cells.insert((1, 1), "Amount:".into());
    feed_cells.insert((1, 2), "98.306".into());
    feed_red.insert((1, 2));
    feed_cells.insert((1, 3), "Nm3/h".into());
    feed_cells.insert((2, 1), "N2(g)".into());
    feed_cells.insert((2, 2), "79.00".into());
    feed_red.insert((2, 2));
    feed_cells.insert((2, 3), "vol-%".into());
    feed_cells.insert((2, 4), "N2(g)".into());
    feed_cells.insert((2, 5), "77.663".into());
    feed_cells.insert((2, 6), "Nm3/h".into());
    feed_cells.insert((3, 1), "O2(g)".into());
    feed_cells.insert((3, 2), "21.00".into());
    feed_red.insert((3, 2));
    feed_cells.insert((3, 3), "vol-%".into());
    feed_cells.insert((3, 4), "O2(g)".into());
    feed_cells.insert((3, 5), "20.643".into());
    feed_cells.insert((3, 6), "Nm3/h".into());
    feed_cells.insert((4, 0), "FLUX:".into());
    feed_cells.insert((4, 1), "Amount:".into());
    feed_cells.insert((4, 2), "12.918".into());
    feed_red.insert((4, 2));
    feed_cells.insert((4, 3), "kg/h".into());
    feed_cells.insert((6, 0), "MATTE:".into());
    feed_cells.insert((6, 1), "Amount:".into());
    feed_cells.insert((6, 2), "127.046".into());
    feed_red.insert((6, 2));
    feed_cells.insert((6, 3), "kg/h".into());
    feed_cells.insert((10, 0), "SCRAP:".into());
    feed_cells.insert((10, 1), "Amount:".into());
    feed_cells.insert((10, 2), "0.00".into());
    feed_red.insert((10, 2));
    feed_cells.insert((10, 3), "kg/h".into());

    BalanceWorkbook {
        title: "CUCONV1 — Copper Converter".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::MJ,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "Balance Area 1".into(),
            in_streams: vec![
                StreamGroup::new(
                    "GAS:",
                    25.0,
                    vec![
                        SpeciesRow::new("N2(g)", 25.0, 3.465),
                        SpeciesRow::new("O2(g)", 25.0, 0.921),
                    ],
                ),
                StreamGroup::new("FLUX:", 25.0, vec![SpeciesRow::new("SiO2", 25.0, 0.215)]),
                StreamGroup::new(
                    "MATTE:",
                    1250.0,
                    vec![
                        SpeciesRow::new("Cu2S(l)", 1250.0, 0.500),
                        SpeciesRow::new("FeS(l)", 1250.0, 0.540),
                    ],
                ),
                StreamGroup::new(
                    "COOLING SCRAP:",
                    25.0,
                    vec![SpeciesRow::new("Cu", 25.0, 0.0)],
                ),
            ],
            extra_heat: 0.0,
            out_streams: vec![
                StreamGroup::new(
                    "PROCESS GAS:",
                    1250.0,
                    vec![
                        SpeciesRow::new("N2(g)", 1250.0, 3.465),
                        SpeciesRow::new("O2(g)", 1250.0, 0.046),
                        SpeciesRow::new("SO2(g)", 1250.0, 0.571),
                    ],
                ),
                StreamGroup::new(
                    "SLAG:",
                    1300.0,
                    vec![
                        SpeciesRow::new("*2FeO*SiO2", 1300.0, 0.215),
                        SpeciesRow::new("Fe3O4(l)", 1300.0, 0.037),
                        SpeciesRow::new("Cu2O(l)", 1300.0, 0.031),
                    ],
                ),
                StreamGroup::new(
                    "WHITE METAL:",
                    1250.0,
                    vec![SpeciesRow::new("Cu2S(l)", 1250.0, 0.469)
                        .with_amount_formula("=0.938*(InCu-D13)/2")],
                ),
                StreamGroup::new("METAL:", 1250.0, vec![SpeciesRow::new("Cu(l)", 1250.0, 0.0)]),
            ],
            heat_losses: 0.0,
        }],
        target_rows: vec![TargetRow {
            name: "COOLING SCRAP Cu".into(),
            var_cell_ref: "IN1!D11".into(),
            test_value: 0.0,
            min_val: 0.0,
            max_val: 5.0,
            target_name: "HEAT BAL".into(),
            target_cell_ref: "BALANCE1!H4".into(),
            current_target_val: 0.0,
            goal_value: 0.0,
            max_iterations: 15,
        }],
        diagram_config: DiagramConfig {
            title: "Copper Converter Heat Balance vs Cooling Scrap".into(),
            x_min: 0.0,
            x_max: 2.0,
            x_step: 0.1,
            x_cell_ref: "IN1!D11".into(),
            x_label: "COOLING SCRAP Cu (kmol)".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "BALANCE1!H4".into(),
                label: "BALANCE".into(),
                units: "Total H MJ".into(),
                target_rows: vec![],
            }],
            use_target_iteration: false,
        },
        user_sheets: vec![BalanceUserSheet {
            name: "FEED".into(),
            cells: feed_cells,
            red_cells: feed_red,
        }],
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

/// `BUTANE.OHE`: Adiabatic combustion of 1 kmol C4H10(NBAg) with air.
pub fn preset_butane() -> BalanceWorkbook {
    BalanceWorkbook {
        title: "BUTANE — Adiabatic Flame Temperature".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::MJ,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "Balance Area 1".into(),
            in_streams: vec![
                StreamGroup::new(
                    "FUEL:",
                    25.0,
                    vec![SpeciesRow::new("C4H10(NBAg)", 25.0, 1.0)],
                ),
                StreamGroup::new(
                    "AIR:",
                    25.0,
                    vec![
                        SpeciesRow::new("O2(g)", 25.0, 6.5),
                        SpeciesRow::new("N2(g)", 25.0, 24.5),
                    ],
                ),
            ],
            extra_heat: 0.0,
            out_streams: vec![StreamGroup::new(
                "FLUE GAS:",
                2000.0,
                vec![
                    SpeciesRow::new("CO2(g)", 2000.0, 4.0),
                    SpeciesRow::new("H2O(g)", 2000.0, 5.0),
                    SpeciesRow::new("N2(g)", 2000.0, 24.5),
                ],
            )],
            heat_losses: 0.0,
        }],
        target_rows: vec![TargetRow {
            name: "FLUE GAS Temp".into(),
            var_cell_ref: "OUT1!STREAM:FLUE GAS:TEMP".into(),
            test_value: 2000.0,
            min_val: 500.0,
            max_val: 3500.0,
            target_name: "HEAT BAL".into(),
            target_cell_ref: "BALANCE1!H4".into(),
            current_target_val: 0.0,
            goal_value: 0.0,
            max_iterations: 15,
        }],
        diagram_config: DiagramConfig {
            title: "Butane Flue Gas Heat Balance vs Temperature".into(),
            x_min: 1000.0,
            x_max: 2500.0,
            x_step: 100.0,
            x_cell_ref: "OUT1!STREAM:FLUE GAS:TEMP".into(),
            x_label: "Flue Gas Temp (°C)".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "BALANCE1!H4".into(),
                label: "Heat Balance".into(),
                units: "MJ".into(),
                target_rows: vec![],
            }],
            use_target_iteration: false,
        },
        user_sheets: Vec::new(),
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

/// `CUSMELT.OHE`: Copper smelting with 500 MJ heat loss.
pub fn preset_cusmelt() -> BalanceWorkbook {
    BalanceWorkbook {
        title: "CUSMELT — Copper Smelting".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::MJ,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "Balance Area 1".into(),
            in_streams: vec![
                StreamGroup::new(
                    "CONCENTRATE:",
                    25.0,
                    vec![
                        SpeciesRow::new("Cu2S", 25.0, 1.0),
                        SpeciesRow::new("FeS", 25.0, 2.0),
                    ],
                ),
                StreamGroup::new("FLUX:", 25.0, vec![SpeciesRow::new("SiO2", 25.0, 1.0)]),
                StreamGroup::new("OXYGEN:", 25.0, vec![SpeciesRow::new("O2(g)", 25.0, 4.0)]),
            ],
            extra_heat: 0.0,
            out_streams: vec![
                StreamGroup::new("METAL:", 1100.0, vec![SpeciesRow::new("Cu", 1100.0, 2.0)]),
                StreamGroup::new(
                    "SLAG:",
                    1400.0,
                    vec![SpeciesRow::new("*2FeO*SiO2", 1400.0, 1.0)],
                ),
                StreamGroup::new(
                    "GAS:",
                    1100.0,
                    vec![SpeciesRow::new("SO2(g)", 1100.0, 3.0)],
                ),
            ],
            heat_losses: 500.0,
        }],
        target_rows: Vec::new(),
        diagram_config: DiagramConfig {
            title: "Diagram".into(),
            x_min: 0.0,
            x_max: 1000.0,
            x_step: 100.0,
            x_cell_ref: "OUT1!HEAT_LOSSES".into(),
            x_label: "Heat Losses (MJ)".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "BALANCE1!H4".into(),
                label: "BALANCE".into(),
                units: "MJ".into(),
                target_rows: vec![],
            }],
            use_target_iteration: false,
        },
        user_sheets: Vec::new(),
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

/// `H2O.OHE`: Water evaporator at 1.01325 bar with 2.793 MJ Extra Heat.
pub fn preset_h2o() -> BalanceWorkbook {
    BalanceWorkbook {
        title: "H2O — Steam Evaporator".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::MJ,
        h2o_pressure_corr: true,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "Balance Area 1".into(),
            in_streams: vec![StreamGroup::new(
                "FEED WATER:",
                25.0,
                vec![SpeciesRow::new("H2O", 25.0, 0.060).with_pressure(1.01325)],
            )],
            extra_heat: 2.793,
            out_streams: vec![StreamGroup::new(
                "STEAM:",
                100.0,
                vec![SpeciesRow::new("H2O(g)", 100.0, 0.060).with_pressure(1.01325)],
            )],
            heat_losses: 0.0,
        }],
        target_rows: Vec::new(),
        diagram_config: DiagramConfig {
            title: "Steam Evaporator Balance".into(),
            x_min: 0.0,
            x_max: 5.0,
            x_step: 0.5,
            x_cell_ref: "IN1!EXTRA_HEAT".into(),
            x_label: "Extra Heat (MJ)".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "BALANCE1!H4".into(),
                label: "Heat Balance".into(),
                units: "MJ".into(),
                target_rows: vec![],
            }],
            use_target_iteration: false,
        },
        user_sheets: Vec::new(),
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

/// Figs. 10–14 (`FSF_process.bal8`): 5-Area Flash Smelting Furnace + Converters + Boiler + ESP.
pub fn preset_fsf_process() -> BalanceWorkbook {
    let mut pf_cells = BTreeMap::new();
    let mut pf_red = BTreeSet::new();
    pf_cells.insert((0, 0), "(Red values are input values)".into());
    pf_cells.insert((2, 1), "Concentrate:".into());
    pf_cells.insert((2, 2), "25".into());
    pf_red.insert((2, 2));
    pf_cells.insert((2, 3), "°C".into());
    pf_cells.insert((3, 2), "1000.0".into());
    pf_red.insert((3, 2));
    pf_cells.insert((3, 3), "kg/h".into());
    pf_cells.insert((4, 1), "Flux:".into());
    pf_cells.insert((4, 2), "112.0".into());
    pf_red.insert((4, 2));
    pf_cells.insert((4, 3), "kg/h".into());
    pf_cells.insert((5, 4), "FSF (1)".into());
    pf_cells.insert((5, 6), "BOILER (4)".into());
    pf_cells.insert((5, 8), "ESP (5)".into());
    pf_cells.insert((8, 4), "CONV. I (2)".into());
    pf_cells.insert((11, 4), "CONV. II (3)".into());

    let mut an_cells = BTreeMap::new();
    let mut an_red = BTreeSet::new();
    an_cells.insert((0, 0), "(Red values are input values)".into());
    an_cells.insert((2, 0), "INPUT ANALYSIS".into());
    an_cells.insert((3, 0), "Concentrate:".into());
    an_cells.insert((3, 1), "CuFeS2".into());
    an_cells.insert((3, 2), "75.1".into());
    an_red.insert((3, 2));
    an_cells.insert((3, 3), "wt-%".into());
    an_cells.insert((4, 1), "FeS".into());
    an_cells.insert((4, 2), "5.9".into());
    an_red.insert((4, 2));
    an_cells.insert((4, 3), "wt-%".into());
    an_cells.insert((5, 1), "FeS2".into());
    an_cells.insert((5, 2), "3.0".into());
    an_red.insert((5, 2));
    an_cells.insert((5, 3), "wt-%".into());

    BalanceWorkbook {
        title: "FSF_process — 5-Area Flash Smelting".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::Kwh,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![
            BalanceArea {
                name: "FSF (1)".into(),
                in_streams: vec![
                    StreamGroup::new(
                        "Concentrate",
                        25.0,
                        vec![
                            SpeciesRow::new("CuFeS2", 25.0, 4.091),
                            SpeciesRow::new("FeS", 25.0, 0.669),
                            SpeciesRow::new("FeS2", 25.0, 0.253),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 25.0, 0.575),
                        ],
                    ),
                    StreamGroup::new(
                        "Flux",
                        25.0,
                        vec![
                            SpeciesRow::new("SiO2", 25.0, 1.486),
                            SpeciesRow::new("*2FeO*SiO2", 25.0, 0.020),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 25.0, 0.067),
                        ],
                    ),
                    StreamGroup::new(
                        "Air (oxygen enriched)",
                        25.0,
                        vec![
                            SpeciesRow::new("O2(g)", 25.0, 14.304),
                            SpeciesRow::new("N2(g)", 25.0, 9.536),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust (Boiler return)",
                        350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 350.0, 0.089)
                                .with_amount_formula("='OUT4'!D12"),
                            SpeciesRow::new("Fe3O4(l)", 350.0, 0.049)
                                .with_amount_formula("='OUT4'!D13"),
                            SpeciesRow::new("SiO2", 350.0, 0.001)
                                .with_amount_formula("='OUT4'!D14"),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 350.0, 0.020)
                                .with_amount_formula("='OUT4'!D15"),
                        ],
                    ),
                ],
                extra_heat: 0.0,
                out_streams: vec![
                    StreamGroup::new(
                        "Matte",
                        1260.0,
                        vec![
                            SpeciesRow::new("Cu2S(l)", 1260.0, 1.981),
                            SpeciesRow::new("FeS(l)", 1260.0, 0.525),
                        ],
                    ),
                    StreamGroup::new(
                        "Slag",
                        1330.0,
                        vec![
                            SpeciesRow::new("*2FeO*SiO2", 1330.0, 1.298),
                            SpeciesRow::new("Fe3O4(l)", 1330.0, 0.562),
                            SpeciesRow::new("Cu2O(l)", 1330.0, 0.089),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 1330.0, 0.622),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue gas",
                        1350.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 1350.0, 8.332),
                            SpeciesRow::new("N2(g)", 1350.0, 9.536),
                            SpeciesRow::new("O2(g)", 1350.0, 3.485),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust",
                        1350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 1350.0, 0.178),
                            SpeciesRow::new("Fe3O4(l)", 1350.0, 0.098),
                            SpeciesRow::new("SiO2", 1350.0, 0.002),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 1350.0, 0.040),
                        ],
                    ),
                ],
                heat_losses: 90.0,
            },
            BalanceArea {
                name: "Converter I (2)".into(),
                in_streams: vec![
                    StreamGroup::new(
                        "Matte",
                        1260.0,
                        vec![
                            SpeciesRow::new("Cu2S(l)", 1260.0, 1.981),
                            SpeciesRow::new("FeS(l)", 1260.0, 0.525),
                        ],
                    ),
                    StreamGroup::new("Flux", 25.0, vec![SpeciesRow::new("SiO2", 25.0, 0.422)]),
                    StreamGroup::new(
                        "Air",
                        25.0,
                        vec![
                            SpeciesRow::new("O2(g)", 25.0, 3.35),
                            SpeciesRow::new("N2(g)", 25.0, 12.603),
                        ],
                    ),
                ],
                extra_heat: 0.0,
                out_streams: vec![
                    StreamGroup::new(
                        "Slag",
                        1300.0,
                        vec![
                            SpeciesRow::new("*2FeO*SiO2", 1300.0, 0.422),
                            SpeciesRow::new("Fe3O4(l)", 1300.0, 0.05),
                            SpeciesRow::new("Cu2O(l)", 1300.0, 0.017),
                        ],
                    ),
                    StreamGroup::new(
                        "White metal",
                        1260.0,
                        vec![SpeciesRow::new("Cu2S", 1260.0, 1.964)],
                    ),
                    StreamGroup::new(
                        "Flue gas",
                        1260.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 1260.0, 0.525),
                            SpeciesRow::new("N2(g)", 1260.0, 12.603),
                            SpeciesRow::new("O2(g)", 1260.0, 2.0),
                        ],
                    ),
                ],
                heat_losses: 10.0,
            },
            BalanceArea {
                name: "Converter II (3)".into(),
                in_streams: vec![
                    StreamGroup::new(
                        "White metal",
                        1260.0,
                        vec![SpeciesRow::new("Cu2S", 1260.0, 0.512)],
                    ),
                    StreamGroup::new("Scrap", 25.0, vec![SpeciesRow::new("Cu", 25.0, 0.779)]),
                    StreamGroup::new(
                        "Air",
                        25.0,
                        vec![
                            SpeciesRow::new("O2(g)", 25.0, 0.54),
                            SpeciesRow::new("N2(g)", 25.0, 0.88),
                        ],
                    ),
                ],
                extra_heat: 0.0,
                out_streams: vec![
                    StreamGroup::new(
                        "Liquid Cu",
                        1260.0,
                        vec![SpeciesRow::new("Cu(l)", 1260.0, 1.803)],
                    ),
                    StreamGroup::new(
                        "Flue gas",
                        1260.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 1260.0, 0.512),
                            SpeciesRow::new("N2(g)", 1260.0, 0.88),
                        ],
                    ),
                ],
                heat_losses: 10.0,
            },
            BalanceArea {
                name: "Boiler (4)".into(),
                in_streams: vec![
                    StreamGroup::new(
                        "Flue gas (from FSF)",
                        1350.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 1350.0, 8.332),
                            SpeciesRow::new("N2(g)", 1350.0, 9.536),
                            SpeciesRow::new("O2(g)", 1350.0, 3.485),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust (from FSF)",
                        1350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 1350.0, 0.178),
                            SpeciesRow::new("Fe3O4(l)", 1350.0, 0.098),
                            SpeciesRow::new("SiO2", 1350.0, 0.002),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 1350.0, 0.040),
                        ],
                    ),
                ],
                extra_heat: 0.0,
                out_streams: vec![
                    StreamGroup::new(
                        "Flue gas",
                        350.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 350.0, 8.332),
                            SpeciesRow::new("N2(g)", 350.0, 9.536),
                            SpeciesRow::new("O2(g)", 350.0, 3.485),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust (to ESP)",
                        350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 350.0, 0.089),
                            SpeciesRow::new("Fe3O4(l)", 350.0, 0.049),
                            SpeciesRow::new("SiO2", 350.0, 0.001),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 350.0, 0.020),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust (return)",
                        350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 350.0, 0.089),
                            SpeciesRow::new("Fe3O4(l)", 350.0, 0.049),
                            SpeciesRow::new("SiO2", 350.0, 0.001),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 350.0, 0.020),
                        ],
                    ),
                ],
                heat_losses: 80.0,
            },
            BalanceArea {
                name: "ESP (5)".into(),
                in_streams: vec![
                    StreamGroup::new(
                        "Flue gas",
                        350.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 350.0, 8.332),
                            SpeciesRow::new("N2(g)", 350.0, 9.536),
                            SpeciesRow::new("O2(g)", 350.0, 3.485),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust",
                        350.0,
                        vec![
                            SpeciesRow::new("Cu2O", 350.0, 0.089),
                            SpeciesRow::new("Fe3O4(l)", 350.0, 0.049),
                            SpeciesRow::new("SiO2", 350.0, 0.001),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 350.0, 0.020),
                        ],
                    ),
                ],
                extra_heat: 0.0,
                out_streams: vec![
                    StreamGroup::new(
                        "Flue gas (stack)",
                        305.0,
                        vec![
                            SpeciesRow::new("SO2(g)", 305.0, 8.332),
                            SpeciesRow::new("N2(g)", 305.0, 9.536),
                            SpeciesRow::new("O2(g)", 305.0, 3.485),
                        ],
                    ),
                    StreamGroup::new(
                        "Flue dust (return)",
                        305.0,
                        vec![
                            SpeciesRow::new("Cu2O", 305.0, 0.089),
                            SpeciesRow::new("Fe3O4(l)", 305.0, 0.049),
                            SpeciesRow::new("SiO2", 305.0, 0.001),
                            SpeciesRow::new("CaO*Al2O3*2SiO2", 305.0, 0.020),
                        ],
                    ),
                ],
                heat_losses: 10.0,
            },
        ],
        target_rows: Vec::new(),
        diagram_config: DiagramConfig {
            title: "FSF Excess Heat vs Heat Losses".into(),
            x_min: 0.0,
            x_max: 200.0,
            x_step: 20.0,
            x_cell_ref: "OUT1!HEAT_LOSSES".into(),
            x_label: "FSF Heat Losses (kWh)".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "BALANCE1!H4".into(),
                label: "FSF Heat Balance".into(),
                units: "kWh".into(),
                target_rows: vec![],
            }],
            use_target_iteration: false,
        },
        user_sheets: vec![
            BalanceUserSheet {
                name: "Process flow".into(),
                cells: pf_cells,
                red_cells: pf_red,
            },
            BalanceUserSheet {
                name: "Analysis".into(),
                cells: an_cells,
                red_cells: an_red,
            },
        ],
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

/// Methane Burner with Water Cooling Jacket (`CH4(g) Input, Nm³` 1..12 vs `Output Gas Temperature °C`).
pub fn preset_ch4_cooling_water() -> BalanceWorkbook {
    let kmol_1nm3 = 1.0 / STP_MOLAR_VOL_L;
    BalanceWorkbook {
        title: "CH4 Burner + Cooling Water (Diagram)".into(),
        temp_unit: TemperatureUnit::Celsius,
        energy_unit: BalanceEnergyUnit::Kwh,
        h2o_pressure_corr: false,
        red_font_shield: false,
        areas: vec![BalanceArea {
            name: "Balance Area 1".into(),
            in_streams: vec![
                StreamGroup::new(
                    "Gas In",
                    25.0,
                    vec![
                        SpeciesRow::new("CH4(g)", 25.0, kmol_1nm3),
                        SpeciesRow::new("O2(g)", 25.0, 3.6 * kmol_1nm3)
                            .with_amount_formula("=3.6*IN1!D3"),
                        SpeciesRow::new("N2(g)", 25.0, 13.542857 * kmol_1nm3)
                            .with_amount_formula("=13.542857*IN1!D3"),
                    ],
                ),
                StreamGroup::new(
                    "Cooling Water In",
                    20.0,
                    vec![SpeciesRow::new("H2O(l)", 20.0, 100.0 / 18.01528)],
                ),
            ],
            extra_heat: 0.0,
            out_streams: vec![
                StreamGroup::new(
                    "Gas Out",
                    1238.938,
                    vec![
                        SpeciesRow::new("CO2(g)", 1238.938, kmol_1nm3)
                            .with_amount_formula("=IN1!D3"),
                        SpeciesRow::new("H2O(g)", 1238.938, 2.0 * kmol_1nm3)
                            .with_amount_formula("=2*IN1!D3"),
                        SpeciesRow::new("N2(g)", 1238.938, 13.542857 * kmol_1nm3)
                            .with_amount_formula("=IN1!D5"),
                        SpeciesRow::new("O2(g)", 1238.938, 1.6 * kmol_1nm3)
                            .with_amount_formula("=IN1!D4-2*IN1!D3"),
                    ],
                ),
                StreamGroup::new(
                    "Cooling Water Out",
                    90.0,
                    vec![SpeciesRow::new("H2O(l)", 90.0, 100.0 / 18.01528)
                        .with_amount_formula("=IN1!D7")],
                ),
            ],
            heat_losses: 0.0,
        }],
        target_rows: vec![TargetRow {
            name: "Gas Out Temperature".into(),
            var_cell_ref: "OUT1!B2".into(),
            test_value: 1238.938,
            min_val: 25.0,
            max_val: 2400.0,
            target_name: "Total Heat Balance".into(),
            target_cell_ref: "BALANCE1!H4".into(),
            goal_value: 0.0,
            max_iterations: 25,
            current_target_val: 0.0,
        }],
        diagram_config: DiagramConfig {
            title: "Output Gas Temperature vs CH4(g) Input".into(),
            x_min: 1.0,
            x_max: 12.0,
            x_step: 0.5,
            x_cell_ref: "IN1!F3".into(),
            x_label: "CH4(g) Input, Nm³".into(),
            y_series: vec![DiagramSeriesDef {
                cell_ref: "OUT1!B2".into(),
                label: "Output Gas".into(),
                units: "°C".into(),
                target_rows: vec![0],
            }],
            use_target_iteration: true,
        },
        user_sheets: Vec::new(),
        summaries: Vec::new(),
        circular_iterations: 0,
        circular_converged: true,
    }
}

// ============================================================================
// Domain Operations: Stream Scaling, Unit Conversions, Copy/Paste Stream & Export
// ============================================================================

/// Update a species' molar amount from an edited `kg` amount (Ch. 11.1 Steps 1–2 & Ch. 11.3).
pub fn set_species_amount_from_kg(sp: &mut SpeciesRow, new_kg: f64) {
    if sp.mw > 1e-12 {
        sp.amount_formula = None;
        sp.amount_kmol = (new_kg / sp.mw).max(0.0);
    }
}

/// Update a species' molar amount from an edited `Nm³` volume (Ch. 11.1 Steps 1–2 & Ch. 11.3).
pub fn set_species_amount_from_nm3(
    sp: &mut SpeciesRow,
    new_nm3: f64,
    repo: &dyn SpeciesRepository,
) {
    if let Some(rec) = repo.get_species(&sp.formula) {
        sp.amount_formula = None;
        if rec.phase == Phase::Gas {
            sp.amount_kmol = (new_nm3 / STP_MOLAR_VOL_L).max(0.0);
        } else if rec.density_g_cm3 > 0.0 && rec.molecular_weight > 0.0 {
            let new_kg = new_nm3 * rec.density_g_cm3 * 1000.0;
            sp.amount_kmol = (new_kg / rec.molecular_weight).max(0.0);
        }
    }
}

/// Scale all species in a stream proportionally to match a new total `kmol` (Ch. 11.3 Step 3).
pub fn scale_stream_by_total_kmol(stream: &mut StreamGroup, new_total_kmol: f64) {
    let target = new_total_kmol.max(0.0);
    if stream.sum_kmol > 1e-12 {
        let factor = target / stream.sum_kmol;
        for sp in &mut stream.species {
            sp.amount_formula = None;
            sp.amount_kmol *= factor;
        }
    } else if let Some(first) = stream.species.first_mut() {
        first.amount_formula = None;
        first.amount_kmol = target;
    }
}

/// Scale all species in a stream proportionally to match a new total `kg` (Ch. 11.3 Step 3).
pub fn scale_stream_by_total_kg(stream: &mut StreamGroup, new_total_kg: f64) {
    let target = new_total_kg.max(0.0);
    if stream.sum_kg > 1e-12 {
        let factor = target / stream.sum_kg;
        for sp in &mut stream.species {
            sp.amount_formula = None;
            sp.amount_kmol *= factor;
        }
    } else if let Some(first) = stream.species.first_mut() {
        if first.mw > 1e-12 {
            first.amount_formula = None;
            first.amount_kmol = target / first.mw;
        }
    }
}

/// Scale all species in a stream proportionally to match a new total `Nm³` (Ch. 11.3 Step 3).
pub fn scale_stream_by_total_nm3(stream: &mut StreamGroup, new_total_nm3: f64) {
    let target = new_total_nm3.max(0.0);
    if stream.sum_nm3 > 1e-12 {
        let factor = target / stream.sum_nm3;
        for sp in &mut stream.species {
            sp.amount_formula = None;
            sp.amount_kmol *= factor;
        }
    }
}

/// Convert all stream/species temperatures and temperature formulas when changing workbook
/// temperature units (`°C <-> K`), per HSC Manual Ch. 11.1 Step 3.
pub fn convert_workbook_temperature_unit(wb: &mut BalanceWorkbook, new_unit: TemperatureUnit) {
    let old_unit = wb.temp_unit;
    if old_unit == new_unit {
        return;
    }
    for area in &mut wb.areas {
        for streams in [&mut area.in_streams, &mut area.out_streams] {
            for st in streams {
                let t_k = old_unit.to_kelvin(st.stream_temp_display);
                st.stream_temp_display = new_unit.from_kelvin(t_k);
                for sp in &mut st.species {
                    let sp_k = old_unit.to_kelvin(sp.temp_display);
                    sp.temp_display = new_unit.from_kelvin(sp_k);
                    if let Some(ref mut tf) = sp.temp_formula {
                        match (old_unit, new_unit) {
                            (TemperatureUnit::Celsius, TemperatureUnit::Kelvin) => {
                                *tf = format!("{tf} + 273.15");
                            }
                            (TemperatureUnit::Kelvin, TemperatureUnit::Celsius) => {
                                *tf = format!("{tf} - 273.15");
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    wb.temp_unit = new_unit;
}

/// Convert `Extra Heat` and `Heat Losses` (green text rows) when changing workbook energy units
/// (`Mcal <-> MJ <-> kWh`), per HSC Manual Ch. 11.1 Step 3 (`1 kWh = 3.6 MJ = 0.8604 Mcal`).
pub fn convert_workbook_energy_unit(wb: &mut BalanceWorkbook, new_unit: BalanceEnergyUnit) {
    let old_unit = wb.energy_unit;
    if old_unit == new_unit {
        return;
    }
    let ratio = old_unit.to_mj_factor() * new_unit.from_mj_factor();
    for area in &mut wb.areas {
        area.extra_heat *= ratio;
        area.heat_losses *= ratio;
    }
    wb.energy_unit = new_unit;
}

/// Create a linked `StreamGroup` for `Edit -> Paste Stream` (Ch. 11.9, Fig. 11):
/// Copies the source stream and populates each species' `amount_formula` with `='OUTx'!D<row>`
/// (or `='INx'!D<row>`) so material amounts remain dynamically linked.
pub fn build_linked_paste_stream(
    wb: &BalanceWorkbook,
    src_area_idx: usize,
    src_is_out: bool,
    src_stream_idx: usize,
) -> Option<StreamGroup> {
    let area = wb.areas.get(src_area_idx)?;
    let streams = if src_is_out {
        &area.out_streams
    } else {
        &area.in_streams
    };
    let target_st = streams.get(src_stream_idx)?;
    let sheet_name = format!(
        "{}{}",
        if src_is_out { "OUT" } else { "IN" },
        src_area_idx + 1
    );

    // Compute exact Excel row number where `src_stream_idx` starts
    let mut cur_row = 2usize;
    for (idx, st) in streams.iter().enumerate() {
        if idx == src_stream_idx {
            break;
        }
        cur_row += 1 + st.species.len();
    }
    let first_sp_row = cur_row + 1;

    let mut new_species = Vec::with_capacity(target_st.species.len());
    for (k, sp) in target_st.species.iter().enumerate() {
        let sp_row = first_sp_row + k;
        let mut cloned = sp.clone();
        cloned.temp_formula = None;
        cloned.amount_formula = Some(format!("='{sheet_name}'!D{sp_row}"));
        new_species.push(cloned);
    }

    Some(StreamGroup {
        name: format!("{} (return)", target_st.name.trim_end_matches(':')),
        stream_temp_display: target_st.stream_temp_display,
        species: new_species,
        sum_kmol: target_st.sum_kmol,
        sum_kg: target_st.sum_kg,
        sum_nm3: target_st.sum_nm3,
        sum_heat_content: target_st.sum_heat_content,
        sum_total_h: target_st.sum_total_h,
    })
}

pub fn serialize_bal8_workbook(wb: &BalanceWorkbook) -> Result<String, SoryntelError> {
    serde_json::to_string_pretty(wb)
        .map_err(|e| SoryntelError::DatabaseError(format!("Failed to serialize .bal8: {e}")))
}

pub fn deserialize_bal8_workbook(raw: &str) -> Result<BalanceWorkbook, SoryntelError> {
    serde_json::from_str(raw)
        .map_err(|e| SoryntelError::DatabaseError(format!("Failed to parse .bal8: {e}")))
}

/// Build multi-sheet `.xlsx` export for the entire Heat & Material Balance workbook (`BALANCE`, `INx`, `OUTx`, `Target`).
pub fn build_balance_export_sheets(
    wb: &BalanceWorkbook,
) -> Vec<crate::spreadsheet_export::ExportSheet> {
    use crate::spreadsheet_export::{ExportCell, ExportCellStyle, ExportCellValue, ExportSheet};

    let mut sheets = Vec::new();
    let e_lbl = wb.energy_unit.label();
    let t_lbl = wb.temp_unit.label();

    // 1. BALANCE sheet
    let mut bal_cells = Vec::new();
    for (c, h) in [
        "BALANCE",
        "Amount kmol",
        "Amount kg",
        "Amount Nm3",
        &format!("Heat Content {e_lbl}"),
        &format!("Total H {e_lbl}"),
        &format!("T*S {e_lbl}"),
        &format!("G {e_lbl}"),
        &format!("Ex Phys {e_lbl}"),
        &format!("Exergy {e_lbl}"),
    ]
    .iter()
    .enumerate()
    {
        bal_cells.push(ExportCell {
            row: 0,
            col: c,
            value: ExportCellValue::Text((*h).to_string()),
            style: ExportCellStyle::Header,
        });
    }
    let mut r_idx = 1usize;
    for (i, sum) in wb.summaries.iter().enumerate() {
        let n = i + 1;
        for (lbl, tot, st) in [
            (format!("IN{n}"), &sum.in_totals, ExportCellStyle::Default),
            (format!("OUT{n}"), &sum.out_totals, ExportCellStyle::Default),
            (
                format!("BALANCE ({n})"),
                &sum.balance_totals,
                ExportCellStyle::BoldBox,
            ),
        ] {
            bal_cells.push(ExportCell {
                row: r_idx,
                col: 0,
                value: ExportCellValue::Text(lbl),
                style: st,
            });
            for (c, v) in [
                tot.kmol,
                tot.kg,
                tot.nm3,
                tot.heat_content,
                tot.total_h,
                tot.ts,
                tot.g,
                tot.ex_phys,
                tot.ex_tot,
            ]
            .into_iter()
            .enumerate()
            {
                bal_cells.push(ExportCell {
                    row: r_idx,
                    col: c + 1,
                    value: ExportCellValue::Number(v),
                    style: st,
                });
            }
            r_idx += 1;
        }
        r_idx += 1;
    }
    sheets.push(ExportSheet {
        name: "BALANCE".into(),
        cells: bal_cells,
        col_widths: vec![16.0; 10],
    });

    // 2. INx and OUTx sheets
    for (a_idx, area) in wb.areas.iter().enumerate() {
        let n = a_idx + 1;
        for (is_out, s_name, streams, heat_lbl, heat_val) in [
            (
                false,
                format!("IN{n}"),
                &area.in_streams,
                "EXTRA HEAT:",
                area.extra_heat,
            ),
            (
                true,
                format!("OUT{n}"),
                &area.out_streams,
                "HEAT LOSSES:",
                area.heat_losses,
            ),
        ] {
            let mut cells = Vec::new();
            let h0 = if is_out {
                format!("OUTPUT SPECIES ({n})")
            } else {
                format!("INPUT SPECIES ({n})")
            };
            for (c, h) in [
                h0.as_str(),
                &format!("Temper. {t_lbl}"),
                "Pressure bar",
                "Amount kmol",
                "Amount kg",
                "Amount Nm3",
                &format!("Heat Content {e_lbl}"),
                &format!("Total H {e_lbl}"),
                &format!("Heat Cont {e_lbl}/kmol"),
                &format!("Tot H {e_lbl}/kmol"),
                "MW",
            ]
            .iter()
            .enumerate()
            {
                cells.push(ExportCell {
                    row: 0,
                    col: c,
                    value: ExportCellValue::Text((*h).to_string()),
                    style: ExportCellStyle::Header,
                });
            }
            let mut r = 1usize;
            for st in streams {
                cells.push(ExportCell {
                    row: r,
                    col: 0,
                    value: ExportCellValue::Text(st.name.clone()),
                    style: ExportCellStyle::BoldBox,
                });
                cells.push(ExportCell {
                    row: r,
                    col: 1,
                    value: ExportCellValue::Number(st.stream_temp_display),
                    style: ExportCellStyle::BoldBox,
                });
                for (c, v) in [
                    (3, st.sum_kmol),
                    (4, st.sum_kg),
                    (5, st.sum_nm3),
                    (6, st.sum_heat_content),
                    (7, st.sum_total_h),
                ] {
                    cells.push(ExportCell {
                        row: r,
                        col: c,
                        value: ExportCellValue::Number(v),
                        style: ExportCellStyle::BoldBox,
                    });
                }
                r += 1;
                for sp in &st.species {
                    cells.push(ExportCell {
                        row: r,
                        col: 0,
                        value: ExportCellValue::Text(sp.formula.clone()),
                        style: ExportCellStyle::Default,
                    });
                    for (c, v) in [
                        (1, sp.temp_display),
                        (2, sp.pressure_bar),
                        (3, sp.amount_kmol),
                        (4, sp.amount_kg),
                        (5, sp.amount_nm3),
                        (6, sp.heat_content),
                        (7, sp.total_h),
                        (8, sp.hc_per_kmol),
                        (9, sp.tot_h_per_kmol),
                        (10, sp.mw),
                    ] {
                        cells.push(ExportCell {
                            row: r,
                            col: c,
                            value: ExportCellValue::Number(v),
                            style: ExportCellStyle::Default,
                        });
                    }
                    r += 1;
                }
            }
            cells.push(ExportCell {
                row: r,
                col: 0,
                value: ExportCellValue::Text(heat_lbl.to_string()),
                style: ExportCellStyle::Green,
            });
            cells.push(ExportCell {
                row: r,
                col: 7,
                value: ExportCellValue::Number(heat_val),
                style: ExportCellStyle::Green,
            });

            sheets.push(ExportSheet {
                name: s_name,
                cells,
                col_widths: vec![18.0, 12.0, 12.0, 13.0, 13.0, 13.0, 15.0, 15.0, 15.0, 15.0, 12.0],
            });
        }
    }

    sheets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_conversions_and_stream_scaling_preserve_thermodynamic_invariants() {
        let mut wb = preset_cuconv1();
        wb.areas[0].extra_heat = 3.6; // 3.6 MJ == 1.0 kWh
        wb.areas[0].heat_losses = 7.2; // 7.2 MJ == 2.0 kWh

        // Convert MJ -> kWh (Ch. 11.1 Step 3: 1 kWh = 3.6 MJ)
        convert_workbook_energy_unit(&mut wb, BalanceEnergyUnit::Kwh);
        assert!((wb.areas[0].extra_heat - 1.0).abs() < 1e-9);
        assert!((wb.areas[0].heat_losses - 2.0).abs() < 1e-9);

        // Convert °C -> K (25 °C -> 298.15 K)
        wb.areas[0].in_streams[0].species[0].temp_formula = Some("=25".into());
        convert_workbook_temperature_unit(&mut wb, TemperatureUnit::Kelvin);
        assert!((wb.areas[0].in_streams[0].stream_temp_display - 298.15).abs() < 1e-6);
        assert_eq!(
            wb.areas[0].in_streams[0].species[0].temp_formula.as_deref(),
            Some("=25 + 273.15")
        );

        // Stream total scaling preserves mole fraction ratios (Ch. 11.3 Step 3)
        let st = &mut wb.areas[0].in_streams[0];
        st.sum_kmol = st.species[0].amount_kmol + st.species[1].amount_kmol;
        let orig_ratio = st.species[0].amount_kmol / st.species[1].amount_kmol;
        scale_stream_by_total_kmol(st, 10.0);
        let new_sum = st.species[0].amount_kmol + st.species[1].amount_kmol;
        let new_ratio = st.species[0].amount_kmol / st.species[1].amount_kmol;
        assert!((new_sum - 10.0).abs() < 1e-9);
        assert!((new_ratio - orig_ratio).abs() < 1e-9);

        // Copy / Paste Stream creates dynamic ='OUT1'!D<row> links (Ch. 11.9)
        let pasted = build_linked_paste_stream(&wb, 0, true, 0).expect("paste stream");
        assert_eq!(
            pasted.species[0].amount_formula.as_deref(),
            Some("='OUT1'!D3")
        );
        assert_eq!(
            pasted.species[1].amount_formula.as_deref(),
            Some("='OUT1'!D4")
        );

        // Air-cooled reactor natural convection Eq. (1) (Ch. 11.1 Step 4)
        let h_loss_mj = crate::balance::estimate_air_cooled_reactor_heat_loss(
            10.0,
            225.0,
            25.0,
            BalanceEnergyUnit::MJ,
        );
        // (6.8 + 0.046 * 225) * 200 * 10 = 17.15 * 2000 = 34300 kcal/h = 34.3 Mcal/h = 143.5112 MJ/h
        assert!((h_loss_mj - 34.3 * 4.184).abs() < 1e-6);
    }
}
