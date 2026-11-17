//! Module 05 — Exergy Balance Engine (Szargut 1988, dead state 25 °C / 298.15 K, 1 bar).
//!
//! Implements material (Normal & Gas) and Energy (Heat via Carnot + Electricity) streams,
//! ideal mixing exergy `R * T0 * sum(n_i * ln(x_i))`, and `OUT - IN` global balances.

use crate::balance::species_chemical_exergy_kj_mol;
use crate::{evaluate_species_si, Phase, SpeciesRepository};
use serde::{Deserialize, Serialize};
use soryntel_core::{R_JOULE, T_REF_K};

/// Standard dead-state pressure for Module 05 Exergy Balance (bar).
pub const EXERGY_P0_BAR: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExergySide {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExergyStreamKind {
    Normal,
    Gas,
    Energy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExergySpeciesRow {
    pub formula: String,
    pub mass_kg: f64,
    pub amount_mol: f64,
    pub mol_fraction: f64,
    pub temp_k: f64,
    pub pressure_bar: f64,
    pub s_j_molk: f64,
    pub h_kj_mol: f64,
    pub h_total_kj: f64,
    pub ex_elemental_kj_mol: f64,
    pub ex_che_kj_mol: f64,
    pub ex_phy_kj_mol: f64,
    pub ex_tot_kj_mol: f64,
    pub ex_che_kj: f64,
    pub ex_phy_kj: f64,
    pub ex_tot_kj: f64,
    pub missing_in_db: bool,
    pub phase_warning: Option<String>,
}

impl ExergySpeciesRow {
    pub fn new(formula: impl Into<String>, amount_mol: f64, temp_k: f64, pressure_bar: f64) -> Self {
        Self {
            formula: formula.into(),
            mass_kg: 0.0,
            amount_mol,
            mol_fraction: 0.0,
            temp_k,
            pressure_bar,
            s_j_molk: 0.0,
            h_kj_mol: 0.0,
            h_total_kj: 0.0,
            ex_elemental_kj_mol: 0.0,
            ex_che_kj_mol: 0.0,
            ex_phy_kj_mol: 0.0,
            ex_tot_kj_mol: 0.0,
            ex_che_kj: 0.0,
            ex_phy_kj: 0.0,
            ex_tot_kj: 0.0,
            missing_in_db: false,
            phase_warning: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExergyStream {
    pub name: String,
    pub side: ExergySide,
    pub kind: ExergyStreamKind,
    pub temp_k: f64,
    pub pressure_bar: f64,
    pub species: Vec<ExergySpeciesRow>,
    // Energy stream inputs
    pub heat_kj: f64,
    pub heat_temp_k: f64,
    pub electricity_kj: f64,
    // Computed totals
    pub mass_kg: f64,
    pub amount_mol: f64,
    pub mol_fraction: f64,
    pub s_j_molk_sum: f64,
    pub h_kj_mol_sum: f64,
    pub h_total_kj: f64,
    pub ex_elemental_kj_mol: f64,
    pub ex_che_kj_mol_sum: f64,
    pub ex_phy_kj_mol_sum: f64,
    pub ex_tot_kj_mol_sum: f64,
    pub ex_che_kj: f64,
    pub ex_phy_kj: f64,
    pub ex_tot_kj: f64,
    pub ex_tot_stream_kj_mol: f64,
    pub ex_tot_stream_kj: f64,
}

impl ExergyStream {
    pub fn new_material(
        name: impl Into<String>,
        side: ExergySide,
        kind: ExergyStreamKind,
        temp_k: f64,
        pressure_bar: f64,
    ) -> Self {
        Self {
            name: name.into(),
            side,
            kind,
            temp_k,
            pressure_bar,
            species: Vec::new(),
            heat_kj: 0.0,
            heat_temp_k: 298.15,
            electricity_kj: 0.0,
            mass_kg: 0.0,
            amount_mol: 0.0,
            mol_fraction: 0.0,
            s_j_molk_sum: 0.0,
            h_kj_mol_sum: 0.0,
            h_total_kj: 0.0,
            ex_elemental_kj_mol: 0.0,
            ex_che_kj_mol_sum: 0.0,
            ex_phy_kj_mol_sum: 0.0,
            ex_tot_kj_mol_sum: 0.0,
            ex_che_kj: 0.0,
            ex_phy_kj: 0.0,
            ex_tot_kj: 0.0,
            ex_tot_stream_kj_mol: 0.0,
            ex_tot_stream_kj: 0.0,
        }
    }

    pub fn new_energy(name: impl Into<String>, side: ExergySide, heat_temp_k: f64) -> Self {
        Self {
            name: name.into(),
            side,
            kind: ExergyStreamKind::Energy,
            temp_k: heat_temp_k,
            pressure_bar: EXERGY_P0_BAR,
            species: Vec::new(),
            heat_kj: 0.0,
            heat_temp_k,
            electricity_kj: 0.0,
            mass_kg: 0.0,
            amount_mol: 0.0,
            mol_fraction: 0.0,
            s_j_molk_sum: 0.0,
            h_kj_mol_sum: 0.0,
            h_total_kj: 0.0,
            ex_elemental_kj_mol: 0.0,
            ex_che_kj_mol_sum: 0.0,
            ex_phy_kj_mol_sum: 0.0,
            ex_tot_kj_mol_sum: 0.0,
            ex_che_kj: 0.0,
            ex_phy_kj: 0.0,
            ex_tot_kj: 0.0,
            ex_tot_stream_kj_mol: 0.0,
            ex_tot_stream_kj: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExergyWorkbook {
    pub streams: Vec<ExergyStream>,
    pub mass_balance_kg: f64,
    pub heat_balance_kj: f64,
    pub exergy_balance_kj: f64,
}

impl Default for ExergyWorkbook {
    fn default() -> Self {
        Self::new_default()
    }
}

impl ExergyWorkbook {
    /// Default Fig. 1 layout (`Input Stream 1` with `H2O` 1.00 mol and `Output Stream 1` with `H2O` 1.00 mol).
    pub fn new_default() -> Self {
        let mut s_in = ExergyStream::new_material(
            "Input Stream 1",
            ExergySide::Input,
            ExergyStreamKind::Normal,
            T_REF_K,
            EXERGY_P0_BAR,
        );
        s_in.species
            .push(ExergySpeciesRow::new("H2O", 1.0, T_REF_K, EXERGY_P0_BAR));

        let mut s_out = ExergyStream::new_material(
            "Output Stream 1",
            ExergySide::Output,
            ExergyStreamKind::Normal,
            T_REF_K,
            EXERGY_P0_BAR,
        );
        s_out
            .species
            .push(ExergySpeciesRow::new("H2O", 1.0, T_REF_K, EXERGY_P0_BAR));

        Self {
            streams: vec![s_in, s_out],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        }
    }

    /// Reference Fig. 2 example (Copper sulfide roasting/conversion).
    pub fn new_fig2_example() -> Self {
        let mut in1 = ExergyStream::new_material(
            "Input Stream 1",
            ExergySide::Input,
            ExergyStreamKind::Normal,
            1100.0,
            1.0,
        );
        in1.species
            .push(ExergySpeciesRow::new("CuS", 800.0, 1100.0, 1.0));
        in1.species
            .push(ExergySpeciesRow::new("FeS", 100.0, 1100.0, 1.0));

        let mut in2 = ExergyStream::new_material(
            "Input Stream 2",
            ExergySide::Input,
            ExergyStreamKind::Normal,
            298.15,
            1.0,
        );
        in2.species
            .push(ExergySpeciesRow::new("SiO2", 50.0, 298.15, 1.0));

        let mut in_gas = ExergyStream::new_material(
            "Input Gas Stream 1",
            ExergySide::Input,
            ExergyStreamKind::Gas,
            298.15,
            1.0,
        );
        in_gas
            .species
            .push(ExergySpeciesRow::new("O2(g)", 2000.0, 298.15, 1.0));
        in_gas
            .species
            .push(ExergySpeciesRow::new("N2(g)", 8000.0, 298.15, 1.0));

        let mut in_en = ExergyStream::new_energy("Input Energy Stream 1", ExergySide::Input, 298.15);
        in_en.heat_kj = 0.0;
        in_en.electricity_kj = 10000.0;

        let mut out1 = ExergyStream::new_material(
            "Output Stream 1",
            ExergySide::Output,
            ExergyStreamKind::Normal,
            1300.0,
            1.0,
        );
        out1.species
            .push(ExergySpeciesRow::new("Cu", 600.0, 1300.0, 1.0));
        out1.species
            .push(ExergySpeciesRow::new("CuO", 200.0, 1300.0, 1.0));

        let mut out2 = ExergyStream::new_material(
            "Output Stream 2",
            ExergySide::Output,
            ExergyStreamKind::Normal,
            1300.0,
            1.0,
        );
        out2.species
            .push(ExergySpeciesRow::new("*2FeO*SiO2", 50.0, 1300.0, 1.0));

        let mut out_gas = ExergyStream::new_material(
            "Output Gas Stream 1",
            ExergySide::Output,
            ExergyStreamKind::Gas,
            1100.0,
            1.0,
        );
        out_gas
            .species
            .push(ExergySpeciesRow::new("SO2(g)", 900.0, 1100.0, 1.0));
        out_gas
            .species
            .push(ExergySpeciesRow::new("SO3(g)", 0.0, 1100.0, 1.0));
        out_gas
            .species
            .push(ExergySpeciesRow::new("O2(g)", 550.0, 1100.0, 1.0));
        out_gas
            .species
            .push(ExergySpeciesRow::new("N2(g)", 8000.0, 1100.0, 1.0));

        let mut out_en =
            ExergyStream::new_energy("Output Energy Stream 1", ExergySide::Output, 1100.0);
        out_en.heat_kj = 20000.0;
        out_en.electricity_kj = 0.0;

        Self {
            streams: vec![in1, in2, in_gas, in_en, out1, out2, out_gas, out_en],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        }
    }

    /// Reference 15-column Sulphides Roasting example (Sulphides, Air, Pure Oxygen -> Oxides, Offgas, Heat output).
    pub fn new_sulphides_example() -> Self {
        let mut sulphides = ExergyStream::new_material(
            "Sulphides",
            ExergySide::Input,
            ExergyStreamKind::Normal,
            1300.0,
            1.0,
        );
        for (f, n) in [
            ("FeS2", 10.0),
            ("Cu2S", 12.0),
            ("CuS", 3.0),
            ("ZnS", 5.0),
            ("PbS", 2.0),
        ] {
            sulphides
                .species
                .push(ExergySpeciesRow::new(f, n, 1300.0, 1.0));
        }

        let mut air = ExergyStream::new_material(
            "Air",
            ExergySide::Input,
            ExergyStreamKind::Gas,
            900.0,
            5.0,
        );
        air.species
            .push(ExergySpeciesRow::new("N2(g)", 37.77, 900.0, 5.0));
        air.species
            .push(ExergySpeciesRow::new("O2(g)", 10.0, 900.0, 5.0));

        let mut pure_o2 = ExergyStream::new_material(
            "Pure Oxygen",
            ExergySide::Input,
            ExergyStreamKind::Gas,
            900.0,
            5.0,
        );
        pure_o2
            .species
            .push(ExergySpeciesRow::new("O2(g)", 58.50, 900.0, 5.0));

        let mut oxides = ExergyStream::new_material(
            "Oxides",
            ExergySide::Output,
            ExergyStreamKind::Normal,
            1300.0,
            1.0,
        );
        for (f, n) in [("Fe2O3", 5.0), ("CuO", 27.0), ("ZnO", 5.0), ("PbO", 2.0)] {
            oxides
                .species
                .push(ExergySpeciesRow::new(f, n, 1300.0, 1.0));
        }

        let mut offgas = ExergyStream::new_material(
            "Offgas",
            ExergySide::Output,
            ExergyStreamKind::Gas,
            1300.0,
            1.0,
        );
        for (f, n) in [("SO2(g)", 42.0), ("N2(g)", 37.77), ("O2(g)", 2.0)] {
            offgas
                .species
                .push(ExergySpeciesRow::new(f, n, 1300.0, 1.0));
        }

        let mut heat_out =
            ExergyStream::new_energy("Heat output", ExergySide::Output, 1300.0);
        heat_out.heat_kj = 17326.58;
        heat_out.electricity_kj = 0.0;

        Self {
            streams: vec![sulphides, air, pure_o2, oxides, offgas, heat_out],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        }
    }

    pub fn next_stream_name(&self, side: ExergySide, kind: ExergyStreamKind) -> String {
        let count = self
            .streams
            .iter()
            .filter(|s| s.side == side && s.kind == kind)
            .count()
            + 1;
        let side_str = match side {
            ExergySide::Input => "Input",
            ExergySide::Output => "Output",
        };
        match kind {
            ExergyStreamKind::Normal => format!("{side_str} Stream {count}"),
            ExergyStreamKind::Gas => format!("{side_str} Gas Stream {count}"),
            ExergyStreamKind::Energy => format!("{side_str} Energy Stream {count}"),
        }
    }

    /// Insert a new stream at the end of its side (Inputs before Outputs). Returns the inserted stream index.
    pub fn insert_stream(&mut self, side: ExergySide, kind: ExergyStreamKind) -> usize {
        let name = self.next_stream_name(side, kind);
        let stream = match kind {
            ExergyStreamKind::Energy => ExergyStream::new_energy(name, side, T_REF_K),
            _ => ExergyStream::new_material(name, side, kind, T_REF_K, EXERGY_P0_BAR),
        };
        match side {
            ExergySide::Input => {
                let pos = self
                    .streams
                    .iter()
                    .rposition(|s| s.side == ExergySide::Input)
                    .map(|i| i + 1)
                    .unwrap_or(0);
                self.streams.insert(pos, stream);
                pos
            }
            ExergySide::Output => {
                self.streams.push(stream);
                self.streams.len() - 1
            }
        }
    }

    pub fn delete_stream(&mut self, stream_idx: usize) {
        if stream_idx < self.streams.len() {
            self.streams.remove(stream_idx);
        }
    }

    pub fn to_exe8_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_exe8_str(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|e| e.to_string())
    }

    pub fn add_species_to_stream(
        &mut self,
        stream_idx: usize,
        formula: &str,
        amount_mol: f64,
    ) -> Result<(), String> {
        let trimmed = formula.trim();
        if trimmed.is_empty() {
            return Err("Fórmula vacía".to_string());
        }
        let Some(stream) = self.streams.get_mut(stream_idx) else {
            return Err("Corriente no válida".to_string());
        };
        if stream.kind == ExergyStreamKind::Energy {
            return Err("Energy stream no acepta especies químicas".to_string());
        }
        stream.species.push(ExergySpeciesRow::new(
            trimmed,
            amount_mol,
            stream.temp_k,
            stream.pressure_bar,
        ));
        Ok(())
    }

    /// Add one or multiple species pasted as newline-, comma-, semicolon-, or tab-separated text (`15.2`).
    pub fn add_species_list_to_stream(
        &mut self,
        stream_idx: usize,
        raw_list: &str,
        default_amount_mol: f64,
    ) -> Result<usize, String> {
        let formulas: Vec<String> = raw_list
            .split(|c: char| c == '\n' || c == '\r' || c == ',' || c == ';' || c == '\t')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if formulas.is_empty() {
            return Err("Fórmula vacía".to_string());
        }
        let mut added = 0usize;
        for f in formulas {
            self.add_species_to_stream(stream_idx, &f, default_amount_mol)?;
            added += 1;
        }
        Ok(added)
    }

    /// Set a species row's mass in kg and update `amount_mol` from its molar mass (`15.2`).
    pub fn set_species_mass_kg(
        &mut self,
        stream_idx: usize,
        row_idx: usize,
        mass_kg: f64,
        repo: &dyn SpeciesRepository,
    ) -> bool {
        let Some(stream) = self.streams.get_mut(stream_idx) else {
            return false;
        };
        let Some(row) = stream.species.get_mut(row_idx) else {
            return false;
        };
        if let Some(sp) = repo.get_species(&row.formula) {
            if sp.molecular_weight > 0.0 {
                row.mass_kg = mass_kg.max(0.0);
                row.amount_mol = (row.mass_kg * 1000.0) / sp.molecular_weight;
                return true;
            }
        }
        false
    }

    pub fn delete_species(&mut self, stream_idx: usize, row_idx: usize) {
        if let Some(stream) = self.streams.get_mut(stream_idx) {
            if row_idx < stream.species.len() {
                stream.species.remove(row_idx);
            }
        }
    }

    /// Recalculate all species, stream totals, mixing/Carnot terms, and global `OUT - IN` balances.
    pub fn calculate(&mut self, repo: &dyn SpeciesRepository) {
        let mut mass_in = 0.0;
        let mut mass_out = 0.0;
        let mut heat_in = 0.0;
        let mut heat_out = 0.0;
        let mut ex_in = 0.0;
        let mut ex_out = 0.0;

        for stream in &mut self.streams {
            match stream.kind {
                ExergyStreamKind::Energy => {
                    stream.mass_kg = 0.0;
                    stream.amount_mol = 0.0;
                    stream.mol_fraction = 0.0;
                    stream.h_total_kj = stream.heat_kj + stream.electricity_kj;
                    stream.ex_elemental_kj_mol = 0.0;
                    stream.ex_che_kj = 0.0;
                    stream.ex_phy_kj = 0.0;
                    stream.ex_tot_kj = 0.0;

                    let carnot = if stream.heat_temp_k > 0.0 {
                        1.0 - T_REF_K / stream.heat_temp_k
                    } else {
                        0.0
                    };
                    stream.ex_tot_stream_kj = stream.heat_kj * carnot + stream.electricity_kj;
                }
                ExergyStreamKind::Normal | ExergyStreamKind::Gas => {
                    let sum_mol: f64 = stream
                        .species
                        .iter()
                        .map(|r| r.amount_mol.max(0.0))
                        .sum();

                    let mut s_mass = 0.0;
                    let mut s_s_sum = 0.0;
                    let mut s_h_mol_sum = 0.0;
                    let mut s_h_tot = 0.0;
                    let mut s_ex_el_sum = 0.0;
                    let mut s_ex_che_mol_sum = 0.0;
                    let mut s_ex_phy_mol_sum = 0.0;
                    let mut s_ex_tot_mol_sum = 0.0;
                    let mut s_ex_che = 0.0;
                    let mut s_ex_phy = 0.0;
                    let mut s_ex_tot = 0.0;
                    let mut ex_mix_kj = 0.0;
                    let mut ex_mix_kj_mol = 0.0;

                    for row in &mut stream.species {
                        row.temp_k = stream.temp_k;
                        row.pressure_bar = stream.pressure_bar;
                        let n = row.amount_mol.max(0.0);
                        let x = if sum_mol > 0.0 { n / sum_mol } else { 0.0 };
                        row.mol_fraction = x;

                        if let Some(sp) = repo.get_species(&row.formula) {
                            row.missing_in_db = false;
                            if stream.kind == ExergyStreamKind::Gas && sp.phase != Phase::Gas {
                                row.phase_warning = Some(
                                    "Gas stream acepta preferentemente especies en fase gas".to_string(),
                                );
                            } else {
                                row.phase_warning = None;
                            }

                            row.mass_kg = n * sp.molecular_weight / 1000.0;
                            let t_eval = row.temp_k.max(1.0);
                            if let (Ok((pt_t, _)), Ok((pt_ref, _))) = (
                                evaluate_species_si(&sp, t_eval, false, false),
                                evaluate_species_si(&sp, T_REF_K, false, false),
                            ) {
                                row.s_j_molk = pt_t.s;
                                row.h_kj_mol = pt_t.h / 1000.0;
                                row.h_total_kj = n * row.h_kj_mol;
                                let ex_el = species_chemical_exergy_kj_mol(repo, &sp);
                                row.ex_elemental_kj_mol = ex_el;
                                row.ex_che_kj_mol = ex_el;
                                row.ex_che_kj = n * ex_el;

                                let ds_pressure =
                                    if stream.kind == ExergyStreamKind::Gas && row.pressure_bar > 0.0 {
                                        -R_JOULE * (row.pressure_bar / EXERGY_P0_BAR).ln()
                                    } else {
                                        0.0
                                    };
                                let ex_phy_mol_j = (pt_t.h - pt_ref.h)
                                    - T_REF_K * ((pt_t.s + ds_pressure) - pt_ref.s);
                                row.ex_phy_kj_mol = ex_phy_mol_j / 1000.0;
                                row.ex_phy_kj = n * row.ex_phy_kj_mol;
                                row.ex_tot_kj_mol = row.ex_che_kj_mol + row.ex_phy_kj_mol;
                                row.ex_tot_kj = row.ex_che_kj + row.ex_phy_kj;
                            } else {
                                row.s_j_molk = 0.0;
                                row.h_kj_mol = 0.0;
                                row.h_total_kj = 0.0;
                                row.ex_elemental_kj_mol = 0.0;
                                row.ex_che_kj_mol = 0.0;
                                row.ex_phy_kj_mol = 0.0;
                                row.ex_tot_kj_mol = 0.0;
                                row.ex_che_kj = 0.0;
                                row.ex_phy_kj = 0.0;
                                row.ex_tot_kj = 0.0;
                            }
                        } else {
                            row.missing_in_db = true;
                            row.phase_warning = None;
                            row.mass_kg = 0.0;
                            row.s_j_molk = 0.0;
                            row.h_kj_mol = 0.0;
                            row.h_total_kj = 0.0;
                            row.ex_elemental_kj_mol = 0.0;
                            row.ex_che_kj_mol = 0.0;
                            row.ex_phy_kj_mol = 0.0;
                            row.ex_tot_kj_mol = 0.0;
                            row.ex_che_kj = 0.0;
                            row.ex_phy_kj = 0.0;
                            row.ex_tot_kj = 0.0;
                        }

                        s_mass += row.mass_kg;
                        s_s_sum += x * row.s_j_molk;
                        s_h_mol_sum += x * row.h_kj_mol;
                        s_h_tot += row.h_total_kj;
                        s_ex_el_sum += x * row.ex_elemental_kj_mol;
                        s_ex_che_mol_sum += x * row.ex_che_kj_mol;
                        s_ex_phy_mol_sum += x * row.ex_phy_kj_mol;
                        s_ex_tot_mol_sum += x * row.ex_tot_kj_mol;
                        s_ex_che += row.ex_che_kj;
                        s_ex_phy += row.ex_phy_kj;
                        s_ex_tot += row.ex_tot_kj;
                        if n > 0.0 && x > 0.0 {
                            // HSC6 Exergy Balance computes mixing exergy using kmol-scaled R*T0
                            // (matching Fig. 2: -0.78 kJ, -12.40 kJ, -1.11 kJ, -12.43 kJ).
                            ex_mix_kj += (R_JOULE * T_REF_K * 1.0e-6) * n * x.ln();
                            ex_mix_kj_mol += (R_JOULE * T_REF_K / 1000.0) * x * x.ln();
                        }
                    }

                    stream.mass_kg = s_mass;
                    stream.amount_mol = sum_mol;
                    stream.mol_fraction = if sum_mol > 0.0 { 1.0 } else { 0.0 };
                    stream.s_j_molk_sum = s_s_sum;
                    stream.h_kj_mol_sum = s_h_mol_sum;
                    stream.h_total_kj = s_h_tot;
                    stream.ex_elemental_kj_mol = s_ex_el_sum;
                    stream.ex_che_kj_mol_sum = s_ex_che_mol_sum;
                    stream.ex_phy_kj_mol_sum = s_ex_phy_mol_sum;
                    stream.ex_tot_kj_mol_sum = s_ex_tot_mol_sum;
                    stream.ex_che_kj = s_ex_che;
                    stream.ex_phy_kj = s_ex_phy;
                    stream.ex_tot_kj = s_ex_tot;
                    // `row.ex_phy_kj_mol` already includes `-T0 * (-R * ln(P/P0))` for Gas streams,
                    // so only add the mixing exergy term here (no pressure double-counting).
                    stream.ex_tot_stream_kj_mol = s_ex_tot_mol_sum + ex_mix_kj_mol;
                    stream.ex_tot_stream_kj = s_ex_tot + ex_mix_kj;
                }
            }

            match stream.side {
                ExergySide::Input => {
                    mass_in += stream.mass_kg;
                    heat_in += stream.h_total_kj;
                    ex_in += stream.ex_tot_stream_kj;
                }
                ExergySide::Output => {
                    mass_out += stream.mass_kg;
                    heat_out += stream.h_total_kj;
                    ex_out += stream.ex_tot_stream_kj;
                }
            }
        }

        self.mass_balance_kg = mass_out - mass_in;
        self.heat_balance_kj = heat_out - heat_in;
        self.exergy_balance_kj = ex_out - ex_in;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChemicalSpecies, KelleyCoefficients, SpeciesThermoInterval};
    use std::collections::HashMap;

    struct TestRepo(HashMap<String, ChemicalSpecies>);

    impl TestRepo {
        fn new() -> Self {
            let mut map = HashMap::new();
            let make_sp = |formula: &str, phase: Phase, mw: f64, hf_j: f64, s_j: f64| {
                ChemicalSpecies {
                    formula: formula.to_string(),
                    name: formula.to_string(),
                    phase,
                    molecular_weight: mw,
                    density_g_cm3: 1.0,
                    charge: 0,
                    intervals: vec![SpeciesThermoInterval {
                        t_min: 298.15,
                        t_max: 2000.0,
                        phase,
                        kelley: KelleyCoefficients {
                            a: 30.0,
                            b: 0.0,
                            c: 0.0,
                            d: 0.0,
                        },
                        h_or_delta_h_tr: hf_j,
                        s_or_delta_s_tr: s_j,
                        reference: "Test".to_string(),
                    }],
                }
            };
            map.insert("H2O".to_string(), make_sp("H2O", Phase::Liquid, 18.015, -285830.0, 69.95));
            map.insert("H2(g)".to_string(), make_sp("H2(g)", Phase::Gas, 2.016, 0.0, 130.68));
            map.insert("O2(g)".to_string(), make_sp("O2(g)", Phase::Gas, 31.999, 0.0, 205.15));
            map.insert("Cu".to_string(), make_sp("Cu", Phase::Solid, 63.546, 0.0, 33.15));
            map.insert("CuO".to_string(), make_sp("CuO", Phase::Solid, 79.545, -156060.0, 42.59));
            Self(map)
        }
    }

    impl SpeciesRepository for TestRepo {
        fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
            self.0.get(formula).cloned()
        }

        fn get_reference_element(&self, symbol: &str) -> Option<ChemicalSpecies> {
            match symbol {
                "H" => self.get_species("H2(g)"),
                "O" => self.get_species("O2(g)"),
                "Cu" => self.get_species("Cu"),
                _ => None,
            }
        }
    }

    #[test]
    fn carnot_energy_stream_matches_fig2() {
        let db = TestRepo::new();
        let mut wb = ExergyWorkbook {
            streams: vec![{
                let mut s =
                    ExergyStream::new_energy("Output Energy Stream 1", ExergySide::Output, 1100.0);
                s.heat_kj = 20000.0;
                s.electricity_kj = 0.0;
                s
            }],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        };
        wb.calculate(&db);
        assert!((wb.streams[0].ex_tot_stream_kj - 14579.0909).abs() < 0.02);
    }

    #[test]
    fn dead_state_physical_exergy_is_zero() {
        let db = TestRepo::new();
        let mut wb = ExergyWorkbook::new_default();
        wb.calculate(&db);
        for stream in &wb.streams {
            for row in &stream.species {
                assert!(row.ex_phy_kj.abs() < 1e-6);
            }
        }
        assert!(wb.mass_balance_kg.abs() < 1e-9);
        assert!(wb.heat_balance_kj.abs() < 1e-9);
        assert!(wb.exergy_balance_kj.abs() < 1e-9);
    }

    #[test]
    fn mixing_exergy_matches_ideal_formula() {
        let db = TestRepo::new();
        let mut s = ExergyStream::new_material(
            "Output Stream 1",
            ExergySide::Output,
            ExergyStreamKind::Normal,
            1300.0,
            1.0,
        );
        s.species.push(ExergySpeciesRow::new("Cu", 600.0, 1300.0, 1.0));
        s.species
            .push(ExergySpeciesRow::new("CuO", 200.0, 1300.0, 1.0));

        let mut wb = ExergyWorkbook {
            streams: vec![s],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        };
        wb.calculate(&db);
        let diff = wb.streams[0].ex_tot_stream_kj - wb.streams[0].ex_tot_kj;
        let expected_mix = (R_JOULE * T_REF_K * 1.0e-6)
            * (600.0 * (0.75_f64).ln() + 200.0 * (0.25_f64).ln());
        assert!((diff - expected_mix).abs() < 1e-6);
        assert!((diff - (-1.115)).abs() < 0.01);
    }

    #[test]
    fn gas_pressure_not_double_counted_and_mass_paste_work() {
        let db = TestRepo::new();
        let mut wb = ExergyWorkbook {
            streams: vec![ExergyStream::new_material(
                "Gas In",
                ExergySide::Input,
                ExergyStreamKind::Gas,
                T_REF_K,
                5.0,
            )],
            mass_balance_kg: 0.0,
            heat_balance_kj: 0.0,
            exergy_balance_kj: 0.0,
        };
        let n = wb.add_species_list_to_stream(0, "O2(g), H2(g)", 1.0).unwrap();
        assert_eq!(n, 2);
        assert!(wb.set_species_mass_kg(0, 0, 0.031999, &db));
        wb.calculate(&db);

        // 0.031999 kg O2(g) == 1.0 mol O2(g)
        assert!((wb.streams[0].species[0].amount_mol - 1.0).abs() < 1e-4);
        // At 298.15 K, 5 bar, physical exergy per mol is R*T0*ln(5)/1000
        let expected_phy = (R_JOULE * T_REF_K / 1000.0) * 5.0_f64.ln();
        assert!((wb.streams[0].ex_phy_kj_mol_sum - expected_phy).abs() < 1e-4);
        // Total stream molar exergy = ex_tot_kj_mol_sum + mixing term (no extra gas_p_kj_mol)
        let expected_mix_mol = (R_JOULE * T_REF_K / 1000.0) * (0.5 * 0.5_f64.ln() * 2.0);
        assert!(
            (wb.streams[0].ex_tot_stream_kj_mol - (wb.streams[0].ex_tot_kj_mol_sum + expected_mix_mol)).abs()
                < 1e-6
        );
        // .exe8 JSON roundtrip
        let json = wb.to_exe8_json().unwrap();
        let parsed = ExergyWorkbook::from_exe8_str(&json).unwrap();
        assert_eq!(parsed.streams[0].species.len(), 2);
    }
}
