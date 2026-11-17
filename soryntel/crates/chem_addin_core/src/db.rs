use crate::error::{ChemAddinError, Result};
use crate::parser::{parse_species, PhaseTag};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One temperature interval row from SQLite table `species_thermo`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeciesInterval {
    pub symbol: String,
    pub phase: String,
    pub mw: f64,
    /// Standard enthalpy of formation at 298.15 K (`kJ/mol`).
    pub h298: f64,
    /// Standard molar entropy at 298.15 K (`J/(mol·K)`).
    pub s298: f64,
    /// Interval lower bound (`K`).
    pub t_min: f64,
    /// Interval upper bound (`K`).
    pub t_max: f64,
    /// Kelley polynomial coefficients for `Cp(T) = A + B*1e-3*T + C*1e5*T^-2 + D*1e-6*T^2` [`J/(mol·K)`].
    pub cp_a: f64,
    pub cp_b: f64,
    pub cp_c: f64,
    pub cp_d: f64,
    /// Phase transition enthalpy at `t_min` for segments with `t_min > 298.15` (`kJ/mol`).
    pub dh_tr: f64,
    /// Phase transition entropy at `t_min` for segments with `t_min > 298.15` (`J/(mol·K)`).
    pub ds_tr: f64,
}

impl SpeciesInterval {
    /// Evaluate heat capacity `Cp(T)` in `J/(mol·K)` at temperature `t_k` (Kelvin).
    pub fn cp_j(&self, t_k: f64) -> f64 {
        self.cp_a
            + self.cp_b * 1e-3 * t_k
            + self.cp_c * 1e5 / (t_k * t_k)
            + self.cp_d * 1e-6 * (t_k * t_k)
    }

    /// Analytical integral of `Cp(T) dT` from `t1` to `t2` in `J/mol`.
    pub fn integrate_enthalpy_j(&self, t1: f64, t2: f64) -> f64 {
        self.cp_a * (t2 - t1)
            + 0.5 * self.cp_b * 1e-3 * (t2 * t2 - t1 * t1)
            - self.cp_c * 1e5 * (1.0 / t2 - 1.0 / t1)
            + (self.cp_d * 1e-6 / 3.0) * (t2 * t2 * t2 - t1 * t1 * t1)
    }

    /// Analytical integral of `Cp(T)/T dT` from `t1` to `t2` in `J/(mol·K)`.
    pub fn integrate_entropy_j(&self, t1: f64, t2: f64) -> f64 {
        self.cp_a * (t2 / t1).ln()
            + self.cp_b * 1e-3 * (t2 - t1)
            - 0.5 * self.cp_c * 1e5 * (1.0 / (t2 * t2) - 1.0 / (t1 * t1))
            + 0.5 * self.cp_d * 1e-6 * (t2 * t2 - t1 * t1)
    }
}

/// Mineral record backed by SQLite table `minerals` and `mineral_elements`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MineralRow {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub formula: String,
    pub density: Option<f64>,
    pub hardness: Option<f64>,
    pub color: String,
    pub luster: String,
    pub group_name: String,
    pub location: String,
    pub elements_str: String,
    pub database_name: String,
    pub elements_wt_pct: BTreeMap<String, f64>,
}

/// SQLite persistence layer for `species_thermo` and `minerals`.
pub struct ChemDatabase {
    conn: Connection,
}

impl ChemDatabase {
    /// Open an in-memory SQLite database initialized with schema and seed thermochemical data.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema_and_seed()?;
        Ok(db)
    }

    /// Open (or create) an on-disk SQLite database initialized with schema and seed data.
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema_and_seed()?;
        Ok(db)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Initialize SQLite tables `species_thermo`, `minerals`, `mineral_elements` and seed baseline rows.
    pub fn init_schema_and_seed(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS species_thermo (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                symbol TEXT NOT NULL,
                phase TEXT NOT NULL CHECK(phase IN ('g', 'l', 's', 'aq')),
                mw REAL NOT NULL,
                h298 REAL NOT NULL,
                s298 REAL NOT NULL,
                t_min REAL NOT NULL,
                t_max REAL NOT NULL,
                cp_a REAL NOT NULL,
                cp_b REAL NOT NULL,
                cp_c REAL NOT NULL,
                cp_d REAL NOT NULL,
                dh_tr REAL NOT NULL DEFAULT 0.0,
                ds_tr REAL NOT NULL DEFAULT 0.0,
                UNIQUE(symbol, t_min)
            );

            CREATE INDEX IF NOT EXISTS idx_species_symbol
                ON species_thermo(symbol COLLATE NOCASE);

            CREATE TABLE IF NOT EXISTS minerals (
                id INTEGER PRIMARY KEY,
                code TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL UNIQUE,
                formula TEXT NOT NULL,
                density REAL,
                hardness REAL,
                color TEXT NOT NULL DEFAULT '',
                luster TEXT NOT NULL DEFAULT '',
                group_name TEXT NOT NULL DEFAULT '',
                location TEXT NOT NULL DEFAULT '',
                elements_str TEXT NOT NULL DEFAULT '',
                database_name TEXT NOT NULL DEFAULT 'Minerals',
                elements_wt_json TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS mineral_elements (
                mineral_id INTEGER NOT NULL REFERENCES minerals(id) ON DELETE CASCADE,
                element TEXT NOT NULL,
                weight_pct REAL NOT NULL,
                PRIMARY KEY (mineral_id, element)
            );
            "#,
        )?;

        self.seed_species()?;
        self.seed_minerals()?;
        Ok(())
    }

    /// Insert or update one `species_thermo` interval row.
    pub fn upsert_species_interval(&self, row: &SpeciesInterval) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO species_thermo
                (symbol, phase, mw, h298, s298, t_min, t_max, cp_a, cp_b, cp_c, cp_d, dh_tr, ds_tr)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(symbol, t_min) DO UPDATE SET
                phase = excluded.phase,
                mw    = excluded.mw,
                h298  = excluded.h298,
                s298  = excluded.s298,
                t_max = excluded.t_max,
                cp_a  = excluded.cp_a,
                cp_b  = excluded.cp_b,
                cp_c  = excluded.cp_c,
                cp_d  = excluded.cp_d,
                dh_tr = excluded.dh_tr,
                ds_tr = excluded.ds_tr
            "#,
            params![
                row.symbol,
                row.phase,
                row.mw,
                row.h298,
                row.s298,
                row.t_min,
                row.t_max,
                row.cp_a,
                row.cp_b,
                row.cp_c,
                row.cp_d,
                row.dh_tr,
                row.ds_tr,
            ],
        )?;
        Ok(())
    }

    /// Fetch ordered Cp intervals for a species symbol, resolving optional omitted phase tags
    /// (e.g. `"H2O"` -> `"H2O(l)"`, `"C(s)"` -> `"C"`) and mineral names (`"Pyrite"` -> `"FeS2"`).
    pub fn get_species_intervals(&self, query: &str) -> Result<Vec<SpeciesInterval>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(ChemAddinError::SpeciesNotFound(query.to_string()));
        }

        // 1. Exact case-insensitive match in `species_thermo`.
        let exact = self.query_exact_symbol(trimmed)?;
        if !exact.is_empty() {
            return Ok(exact);
        }

        // 2. Try stripped / phase-qualified variants if chemical formula parses.
        if let Ok(parsed) = parse_species(trimmed) {
            if parsed.phase == PhaseTag::Unspecified {
                for suffix in ["(l)", "(s)", "(g)", "(aq)"] {
                    let candidate = format!("{}{}", parsed.formula_core, suffix);
                    let rows = self.query_exact_symbol(&candidate)?;
                    if !rows.is_empty() {
                        return Ok(rows);
                    }
                }
            } else {
                // Match unsuffixed stored symbol (e.g. query "C(s)" against stored "C" with phase='s').
                let rows = self.query_symbol_and_phase(
                    &parsed.formula_core,
                    parsed.phase.as_db_str(),
                )?;
                if !rows.is_empty() {
                    return Ok(rows);
                }
            }
        }

        // 3. Fallback through `minerals` (`"Pyrite"` / `"Pyr"` -> `FeS2`).
        if let Some(mineral) = self.get_mineral(trimmed)? {
            let m_exact = self.query_exact_symbol(&mineral.formula)?;
            if !m_exact.is_empty() {
                return Ok(m_exact);
            }
            for suffix in ["(s)", "(l)", "(g)"] {
                let candidate = format!("{}{}", mineral.formula, suffix);
                let rows = self.query_exact_symbol(&candidate)?;
                if !rows.is_empty() {
                    return Ok(rows);
                }
            }
        }

        Err(ChemAddinError::SpeciesNotFound(trimmed.to_string()))
    }

    /// Look up a mineral row by name, code, or formula (case-insensitive).
    pub fn get_mineral(&self, query: &str) -> Result<Option<MineralRow>> {
        let trimmed = query.trim();
        let row = self
            .conn
            .query_row(
                r#"
                SELECT id, code, name, formula, density, hardness, color, luster,
                       group_name, location, elements_str, database_name, elements_wt_json
                FROM minerals
                WHERE LOWER(name) = LOWER(?1)
                   OR LOWER(code) = LOWER(?1)
                   OR LOWER(formula) = LOWER(?1)
                LIMIT 1
                "#,
                params![trimmed],
                |r| {
                    let wt_json: String = r.get(12)?;
                    let elements_wt_pct: BTreeMap<String, f64> =
                        serde_json::from_str(&wt_json).unwrap_or_default();
                    Ok(MineralRow {
                        id: r.get(0)?,
                        code: r.get(1)?,
                        name: r.get(2)?,
                        formula: r.get(3)?,
                        density: r.get(4)?,
                        hardness: r.get(5)?,
                        color: r.get(6)?,
                        luster: r.get(7)?,
                        group_name: r.get(8)?,
                        location: r.get(9)?,
                        elements_str: r.get(10)?,
                        database_name: r.get(11)?,
                        elements_wt_pct,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    fn query_exact_symbol(&self, symbol: &str) -> Result<Vec<SpeciesInterval>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT symbol, phase, mw, h298, s298, t_min, t_max,
                   cp_a, cp_b, cp_c, cp_d, dh_tr, ds_tr
            FROM species_thermo
            WHERE LOWER(symbol) = LOWER(?1)
            ORDER BY t_min ASC
            "#,
        )?;
        let iter = stmt.query_map(params![symbol], map_species_interval)?;
        let mut out = Vec::new();
        for item in iter {
            out.push(item?);
        }
        Ok(out)
    }

    fn query_symbol_and_phase(&self, core: &str, phase: &str) -> Result<Vec<SpeciesInterval>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT symbol, phase, mw, h298, s298, t_min, t_max,
                   cp_a, cp_b, cp_c, cp_d, dh_tr, ds_tr
            FROM species_thermo
            WHERE LOWER(symbol) = LOWER(?1) AND LOWER(phase) = LOWER(?2)
            ORDER BY t_min ASC
            "#,
        )?;
        let iter = stmt.query_map(params![core, phase], map_species_interval)?;
        let mut out = Vec::new();
        for item in iter {
            out.push(item?);
        }
        Ok(out)
    }

    fn seed_species(&self) -> Result<()> {
        // Real NIST-JANAF / Barin / Kelley polynomial thermochemical data:
        // Cp(T) [J/(mol·K)] = A + B*1e-3*T + C*1e5*T^-2 + D*1e-6*T^2
        let seeds: &[SpeciesInterval] = &[
            // H2O(l) — liquid water [273.15 .. 373.15 K]
            SpeciesInterval {
                symbol: "H2O(l)".into(),
                phase: "l".into(),
                mw: 18.015,
                h298: -285.830,
                s298: 69.950,
                t_min: 273.15,
                t_max: 373.15,
                cp_a: 72.420,
                cp_b: 10.350,
                cp_c: -0.190,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // H2O(g) — steam [298.15 .. 3000 K]
            SpeciesInterval {
                symbol: "H2O(g)".into(),
                phase: "g".into(),
                mw: 18.015,
                h298: -241.826,
                s298: 188.835,
                t_min: 298.15,
                t_max: 3000.0,
                cp_a: 30.000,
                cp_b: 10.710,
                cp_c: 0.335,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // CO2(g) [298.15 .. 2500 K]
            SpeciesInterval {
                symbol: "CO2(g)".into(),
                phase: "g".into(),
                mw: 44.009,
                h298: -393.510,
                s298: 213.785,
                t_min: 298.15,
                t_max: 2500.0,
                cp_a: 44.140,
                cp_b: 9.040,
                cp_c: -8.535,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // CO(g) [298.15 .. 2500 K]
            SpeciesInterval {
                symbol: "CO(g)".into(),
                phase: "g".into(),
                mw: 28.010,
                h298: -110.530,
                s298: 197.660,
                t_min: 298.15,
                t_max: 2500.0,
                cp_a: 28.410,
                cp_b: 4.100,
                cp_c: -0.460,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // C (graphite) [298.15 .. 2500 K]
            SpeciesInterval {
                symbol: "C".into(),
                phase: "s".into(),
                mw: 12.011,
                h298: 0.0,
                s298: 5.740,
                t_min: 298.15,
                t_max: 2500.0,
                cp_a: 16.860,
                cp_b: 4.770,
                cp_c: -8.540,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // O2(g) [298.15 .. 3000 K]
            SpeciesInterval {
                symbol: "O2(g)".into(),
                phase: "g".into(),
                mw: 31.998,
                h298: 0.0,
                s298: 205.152,
                t_min: 298.15,
                t_max: 3000.0,
                cp_a: 29.960,
                cp_b: 4.184,
                cp_c: -1.674,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // H2(g) [298.15 .. 3000 K]
            SpeciesInterval {
                symbol: "H2(g)".into(),
                phase: "g".into(),
                mw: 2.016,
                h298: 0.0,
                s298: 130.680,
                t_min: 298.15,
                t_max: 3000.0,
                cp_a: 27.280,
                cp_b: 3.260,
                cp_c: 0.500,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // N2(g) [298.15 .. 3000 K]
            SpeciesInterval {
                symbol: "N2(g)".into(),
                phase: "g".into(),
                mw: 28.014,
                h298: 0.0,
                s298: 191.609,
                t_min: 298.15,
                t_max: 3000.0,
                cp_a: 27.870,
                cp_b: 4.270,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // CH4(g) [298.15 .. 1500 K]
            SpeciesInterval {
                symbol: "CH4(g)".into(),
                phase: "g".into(),
                mw: 16.043,
                h298: -74.870,
                s298: 186.251,
                t_min: 298.15,
                t_max: 1500.0,
                cp_a: 23.640,
                cp_b: 47.860,
                cp_c: -1.920,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // Fe (alpha -> beta -> gamma solid intervals)
            SpeciesInterval {
                symbol: "Fe".into(),
                phase: "s".into(),
                mw: 55.845,
                h298: 0.0,
                s298: 27.280,
                t_min: 298.15,
                t_max: 1043.0,
                cp_a: 17.490,
                cp_b: 24.770,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            SpeciesInterval {
                symbol: "Fe".into(),
                phase: "s".into(),
                mw: 55.845,
                h298: 0.0,
                s298: 27.280,
                t_min: 1043.0,
                t_max: 1185.0,
                cp_a: 37.660,
                cp_b: 0.0,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            SpeciesInterval {
                symbol: "Fe".into(),
                phase: "s".into(),
                mw: 55.845,
                h298: 0.0,
                s298: 27.280,
                t_min: 1185.0,
                t_max: 1667.0,
                cp_a: 23.970,
                cp_b: 8.370,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.900,
                ds_tr: 0.759,
            },
            // S (rhombic -> monoclinic -> liquid)
            SpeciesInterval {
                symbol: "S".into(),
                phase: "s".into(),
                mw: 32.060,
                h298: 0.0,
                s298: 32.054,
                t_min: 298.15,
                t_max: 368.30,
                cp_a: 14.980,
                cp_b: 26.110,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            SpeciesInterval {
                symbol: "S".into(),
                phase: "s".into(),
                mw: 32.060,
                h298: 0.0,
                s298: 32.054,
                t_min: 368.30,
                t_max: 388.36,
                cp_a: 14.900,
                cp_b: 29.120,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.401,
                ds_tr: 1.089,
            },
            SpeciesInterval {
                symbol: "S".into(),
                phase: "s".into(),
                mw: 32.060,
                h298: 0.0,
                s298: 32.054,
                t_min: 388.36,
                t_max: 717.80,
                cp_a: 35.600,
                cp_b: 0.0,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 1.721,
                ds_tr: 4.431,
            },
            // FeS (troilite alpha -> beta)
            SpeciesInterval {
                symbol: "FeS".into(),
                phase: "s".into(),
                mw: 87.905,
                h298: -101.670,
                s298: 60.320,
                t_min: 298.15,
                t_max: 411.0,
                cp_a: 21.710,
                cp_b: 110.460,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            SpeciesInterval {
                symbol: "FeS".into(),
                phase: "s".into(),
                mw: 87.905,
                h298: -101.670,
                s298: 60.320,
                t_min: 411.0,
                t_max: 1463.0,
                cp_a: 72.800,
                cp_b: 0.0,
                cp_c: 0.0,
                cp_d: 0.0,
                dh_tr: 2.385,
                ds_tr: 5.803,
            },
            // FeS2 (Pyrite) [298.15 .. 1016 K]
            SpeciesInterval {
                symbol: "FeS2".into(),
                phase: "s".into(),
                mw: 119.965,
                h298: -171.540,
                s298: 52.930,
                t_min: 298.15,
                t_max: 1016.0,
                cp_a: 74.810,
                cp_b: 5.520,
                cp_c: -12.760,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // SO2(g) [298.15 .. 2000 K]
            SpeciesInterval {
                symbol: "SO2(g)".into(),
                phase: "g".into(),
                mw: 64.058,
                h298: -296.810,
                s298: 248.223,
                t_min: 298.15,
                t_max: 2000.0,
                cp_a: 43.430,
                cp_b: 10.630,
                cp_c: -5.940,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
            // Fe2O3 (Hematite) [298.15 .. 950 K]
            SpeciesInterval {
                symbol: "Fe2O3".into(),
                phase: "s".into(),
                mw: 159.687,
                h298: -824.200,
                s298: 87.400,
                t_min: 298.15,
                t_max: 950.0,
                cp_a: 98.280,
                cp_b: 77.820,
                cp_c: -14.850,
                cp_d: 0.0,
                dh_tr: 0.0,
                ds_tr: 0.0,
            },
        ];

        for row in seeds {
            self.upsert_species_interval(row)?;
        }
        Ok(())
    }

    fn seed_minerals(&self) -> Result<()> {
        let seeds = [
            MineralRow {
                id: 1,
                code: "Pyr".into(),
                name: "Pyrite".into(),
                formula: "FeS2".into(),
                density: Some(5.01),
                hardness: Some(6.25),
                color: "Brass yellow".into(),
                luster: "Metallic".into(),
                group_name: "Pyrite".into(),
                location: "Spain, Peru, Italy, USA".into(),
                elements_str: "\\Fe\\S\\".into(),
                database_name: "Minerals".into(),
                elements_wt_pct: BTreeMap::from([
                    ("Fe".into(), 46.551),
                    ("S".into(), 53.449),
                ]),
            },
            MineralRow {
                id: 2,
                code: "Tro".into(),
                name: "Troilite".into(),
                formula: "FeS".into(),
                density: Some(4.61),
                hardness: Some(4.0),
                color: "Bronze brown".into(),
                luster: "Metallic".into(),
                group_name: "Pyrrhotite".into(),
                location: "Meteorites, Norilsk".into(),
                elements_str: "\\Fe\\S\\".into(),
                database_name: "Minerals".into(),
                elements_wt_pct: BTreeMap::from([
                    ("Fe".into(), 63.529),
                    ("S".into(), 36.471),
                ]),
            },
            MineralRow {
                id: 3,
                code: "Hem".into(),
                name: "Hematite".into(),
                formula: "Fe2O3".into(),
                density: Some(5.26),
                hardness: Some(6.0),
                color: "Steel gray to reddish brown".into(),
                luster: "Metallic".into(),
                group_name: "Hematite".into(),
                location: "Brazil, Australia, USA".into(),
                elements_str: "\\Fe\\O\\".into(),
                database_name: "Minerals".into(),
                elements_wt_pct: BTreeMap::from([
                    ("Fe".into(), 69.943),
                    ("O".into(), 30.057),
                ]),
            },
            MineralRow {
                id: 4,
                code: "Ccp".into(),
                name: "Chalcopyrite".into(),
                formula: "CuFeS2".into(),
                density: Some(4.19),
                hardness: Some(3.75),
                color: "Brass yellow".into(),
                luster: "Metallic".into(),
                group_name: "Chalcopyrite".into(),
                location: "Chile, Peru, Canada, Zambia".into(),
                elements_str: "\\Cu\\Fe\\S\\".into(),
                database_name: "Minerals".into(),
                elements_wt_pct: BTreeMap::from([
                    ("Cu".into(), 34.626),
                    ("Fe".into(), 30.431),
                    ("S".into(), 34.943),
                ]),
            },
        ];

        for m in &seeds {
            let wt_json = serde_json::to_string(&m.elements_wt_pct)?;
            self.conn.execute(
                r#"
                INSERT INTO minerals
                    (id, code, name, formula, density, hardness, color, luster,
                     group_name, location, elements_str, database_name, elements_wt_json)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                ON CONFLICT(id) DO UPDATE SET
                    code = excluded.code,
                    name = excluded.name,
                    formula = excluded.formula,
                    density = excluded.density,
                    hardness = excluded.hardness,
                    color = excluded.color,
                    luster = excluded.luster,
                    group_name = excluded.group_name,
                    location = excluded.location,
                    elements_str = excluded.elements_str,
                    database_name = excluded.database_name,
                    elements_wt_json = excluded.elements_wt_json
                "#,
                params![
                    m.id,
                    m.code,
                    m.name,
                    m.formula,
                    m.density,
                    m.hardness,
                    m.color,
                    m.luster,
                    m.group_name,
                    m.location,
                    m.elements_str,
                    m.database_name,
                    wt_json,
                ],
            )?;
            for (elem, wt) in &m.elements_wt_pct {
                self.conn.execute(
                    r#"
                    INSERT INTO mineral_elements (mineral_id, element, weight_pct)
                    VALUES (?1, ?2, ?3)
                    ON CONFLICT(mineral_id, element) DO UPDATE SET weight_pct = excluded.weight_pct
                    "#,
                    params![m.id, elem, wt],
                )?;
            }
        }
        Ok(())
    }
}

fn map_species_interval(row: &rusqlite::Row<'_>) -> rusqlite::Result<SpeciesInterval> {
    Ok(SpeciesInterval {
        symbol: row.get(0)?,
        phase: row.get(1)?,
        mw: row.get(2)?,
        h298: row.get(3)?,
        s298: row.get(4)?,
        t_min: row.get(5)?,
        t_max: row.get(6)?,
        cp_a: row.get(7)?,
        cp_b: row.get(8)?,
        cp_c: row.get(9)?,
        cp_d: row.get(10)?,
        dh_tr: row.get(11)?,
        ds_tr: row.get(12)?,
    })
}
