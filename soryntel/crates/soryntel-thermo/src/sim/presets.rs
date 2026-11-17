//! Built-in Unit Model Library catalog, equipment icon metadata,
//! and authentic reference projects (`40.1_model.Sim`, `Hydro_example.Sim`,
//! `Concentrator_Flotation.Sim`, `Recycle_Loop.Sim`) matching Chapter 40 Figures.

use crate::sim::model::*;

pub const DEFAULT_STREAM_VARIABLES: &[(&str, &str)] = &[
    ("Total Solids Flow", "t/h"),
    ("Total Gas Flow", "t/h"),
    ("Total Liquid Flow", "t/h"),
    ("Pulp Mass Flow", "t/h"),
    ("Pulp Volumetric Flow", "m3/h"),
    ("Solids SG", "g/cm3"),
    ("Liquid SG", "g/cm3"),
    ("Pulp SG", "g/cm3"),
    ("% Solids", "wt-%"),
    ("Solids Recovery", "Rec-%"),
    ("Liquid Recovery", "Rec-%"),
    ("Cu", "wt-%"),
    ("Fe", "wt-%"),
    ("S", "wt-%"),
    ("Ccp", "Rec-%"),
    ("Cu Rec", "Rec-%"),
    ("P80", "μm"),
    ("P50", "μm"),
    ("Temperature", "°C"),
    ("Pressure", "bar"),
    ("Enthalpy Flow", "kW"),
    ("Exergy Flow", "kW"),
];

pub const VISUALIZATION_METRICS: &[&str] = &[
    "Select visualization",
    "Mass Flow (t/h)",
    "Total Solids Flow (t/h)",
    "Enthalpy Flow (kW)",
    "Exergy Flow (kW)",
    "Temperature (°C)",
    "Cu (wt-%)",
    "Solids Recovery (Rec-%)",
];

/// Unified stream metric accessor shared between Stream Tables, Sankey scaling, and Reports.
pub fn metric_value_and_unit(s: &SimStream, var_name: &str) -> (f64, &'static str) {
    match var_name {
        "Total Mass Flow" | "Mass Flow (t/h)" | "Mass Flow" | "Pulp Mass Flow" => (s.mass_flow_t_h, "t/h"),
        "Total Solids Flow" | "Total Solids Flow (t/h)" | "Solids Flow" => (s.solids_flow_t_h, "t/h"),
        "Total Liquid Flow" | "Liquid Flow" => (s.liquid_flow_t_h, "t/h"),
        "Total Gas Flow" | "Gas Flow" => (s.gas_flow_nm3_h, "Nm3/h"),
        "Pulp Volumetric Flow" | "Pulp Volume Flow" => (s.pulp_vol_m3_h, "m3/h"),
        "Solids SG" | "Solids Specific Gravity" => (s.solids_sg, "g/cm3"),
        "Liquid SG" | "Liquid Specific Gravity" => (s.liquid_sg, "g/cm3"),
        "Pulp SG" | "Pulp Specific Gravity" => (s.pulp_sg, "g/cm3"),
        "% Solids" | "Solids Weight Percent" => (s.pct_solids, "wt-%"),
        "Solids Recovery" | "Solids Recovery (Rec-%)" => (s.solids_rec_pct, "Rec-%"),
        "Liquid Recovery" => (s.liquid_rec_pct, "Rec-%"),
        "Cu" | "Cu (wt-%)" | "Cu wt-%" => (s.cu_wt_pct, "wt-%"),
        "Fe" | "Fe wt-%" => (s.fe_wt_pct, "wt-%"),
        "S" | "S wt-%" => (s.s_wt_pct, "wt-%"),
        "Ccp" | "Ccp Rec %" => (s.ccp_rec_pct, "Rec-%"),
        "Cu Rec" | "Cu Rec %" => (s.cu_rec_pct, "Rec-%"),
        "P80" => (s.p80_um, "um"),
        "P50" => (s.p50_um, "um"),
        "Temperature" | "Temperature (°C)" => (s.temp_c, "C"),
        "Pressure" => (s.pressure_bar, "bar"),
        "Enthalpy Flow" | "Enthalpy Flow (kW)" | "Total Enthalpy" => (s.enthalpy_kw, "kW"),
        "Thermal Energy" => (s.thermal_energy_kw, "kW"),
        "Exergy Flow" | "Exergy Flow (kW)" | "Total Exergy" => (s.exergy_kw, "kW"),
        _ => (s.mass_flow_t_h, "t/h"),
    }
}

fn make_lca_stream(
    id: usize,
    name: &str,
    src: Option<usize>,
    dst: Option<usize>,
    mass_t_h: f64,
    enthalpy_kw: f64,
    exergy_kw: f64,
    temp_c: f64,
    species: &[(&str, StreamPhaseKind, f64)],
) -> SimStream {
    let mut s = SimStream::new(id, name, src, dst, [0.0, 0.0], [100.0, 0.0], 0);
    s.mass_flow_t_h = mass_t_h;
    s.enthalpy_kw = enthalpy_kw;
    s.exergy_kw = exergy_kw;
    s.temp_c = temp_c;
    s.solids_flow_t_h = species
        .iter()
        .filter(|(_, p, _)| *p == StreamPhaseKind::PureSolid)
        .map(|(_, _, m)| *m)
        .sum();
    s.liquid_flow_t_h = species
        .iter()
        .filter(|(_, p, _)| *p == StreamPhaseKind::Water)
        .map(|(_, _, m)| *m)
        .sum();
    s.species_flows = species
        .iter()
        .map(|(f, p, m)| StreamSpeciesFlow {
            formula: (*f).to_string(),
            phase: *p,
            mass_flow_t_h: *m,
            enthalpy_kw: 0.0,
            exergy_kw: 0.0,
        })
        .collect();
    s
}

/// Official Chapter 49.3 benchmark flowsheet: `Zn TSL Fuming` (`Figs. 9–17`).
pub fn preset_zn_tsl_fuming() -> SimProject {
    let mut p = empty_project();
    p.process_info.process_name = "Zn TSL Fuming Process".to_string();
    p.process_info.technology = "Pyrometallurgy / Top Submerged Lance".to_string();
    p.process_info.products = "Final ZnO Dust".to_string();

    p.units = vec![
        SimUnit::new(1, "Slag Fuming", UnitKind::Reactions, [60.0, 40.0], 0),
        SimUnit::new(2, "Post-Combustion", UnitKind::Reactions, [130.0, 40.0], 0),
        SimUnit::new(3, "WHRB", UnitKind::Generic, [200.0, 40.0], 0),
        SimUnit::new(4, "Quenching", UnitKind::Generic, [270.0, 40.0], 0),
        SimUnit::new(5, "Baghouse", UnitKind::Distributions, [340.0, 40.0], 0),
        SimUnit::new(6, "Dust cooling", UnitKind::Generic, [410.0, 40.0], 0),
    ];

    p.streams = vec![
        // Boundary Inputs (Blue)
        make_lca_stream(10, "Residues", None, Some(1), 100.0, -226238.48, 30074.13, 25.0, &[
            ("FeO", StreamPhaseKind::PureSolid, 50.0),
            ("SiO2", StreamPhaseKind::PureSolid, 35.0),
            ("CaO", StreamPhaseKind::PureSolid, 5.0),
            ("ZnO", StreamPhaseKind::PureSolid, 10.0),
        ]),
        make_lca_stream(11, "Air", None, Some(1), 0.00129, 0.0, 0.0, 25.0, &[
            ("N2(g)", StreamPhaseKind::Gas, 0.00099),
            ("O2(g)", StreamPhaseKind::Gas, 0.00030),
        ]),
        make_lca_stream(12, "Reductant", None, Some(1), 4.39749, 0.0, 14500.0, 25.0, &[
            ("C", StreamPhaseKind::PureSolid, 4.39749),
        ]),
        make_lca_stream(13, "Power", None, Some(1), 0.0, 63020.50, 63020.50, 25.0, &[]),
        make_lca_stream(14, "Air - PC", None, Some(2), 34.59074, 0.0, 210.0, 25.0, &[
            ("N2(g)", StreamPhaseKind::Gas, 26.63487),
            ("O2(g)", StreamPhaseKind::Gas, 7.95587),
        ]),
        make_lca_stream(15, "Water in - WHRB", None, Some(3), 39.95775, 0.0, 55.0, 25.0, &[
            ("H2O", StreamPhaseKind::Water, 39.95775),
        ]),
        // Internal Black Streams (excluded from Black-Box LCI)
        make_lca_stream(16, "TSL Off-gas", Some(1), Some(2), 25.0, 12500.0, 8200.0, 1250.0, &[]),
        make_lca_stream(17, "Combusted Gas", Some(2), Some(3), 59.59074, 29800.0, 19400.0, 1380.0, &[]),
        make_lca_stream(18, "Cooled Gas", Some(3), Some(4), 48.36634, 7100.0, 3200.0, 380.0, &[]),
        make_lca_stream(19, "Filtered Dust", Some(5), Some(6), 8.01299, 320.0, 180.0, 180.0, &[]),
        // Boundary Outputs (Red)
        make_lca_stream(20, "Metal", Some(1), None, 15.78466, -3200.0, 18400.0, 1250.0, &[
            ("Fe", StreamPhaseKind::PureSolid, 15.78466),
        ]),
        make_lca_stream(21, "Clean Slag", Some(1), None, 71.60155, -94000.0, 4200.0, 1250.0, &[
            ("FeO", StreamPhaseKind::PureSolid, 32.2),
            ("SiO2", StreamPhaseKind::PureSolid, 34.4),
            ("CaO", StreamPhaseKind::PureSolid, 5.00155),
        ]),
        make_lca_stream(22, "Heat Loss - TSL", Some(1), None, 0.0, 6302.05, 4100.0, 1250.0, &[]),
        make_lca_stream(23, "Steam out - WHRB", Some(3), None, 39.95775, 22400.0, 11800.0, 320.0, &[
            ("H2O(g)", StreamPhaseKind::Gas, 39.95775),
        ]),
        make_lca_stream(24, "Heat Loss - Quenching", Some(4), None, 0.0, 3327.22, 1850.0, 210.0, &[]),
        make_lca_stream(25, "Off-gas to further treatment", Some(5), None, 40.35335, 3100.0, 1600.0, 140.0, &[
            ("CO2(g)", StreamPhaseKind::Gas, 15.95135),
            ("N2(g)", StreamPhaseKind::Gas, 21.102),
            ("H2O(g)", StreamPhaseKind::Gas, 2.85),
            ("O2(g)", StreamPhaseKind::Gas, 0.45),
        ]),
        make_lca_stream(26, "Final ZnO Dust", Some(6), None, 8.01299, 120.0, 9200.0, 60.0, &[
            ("ZnO", StreamPhaseKind::PureSolid, 8.01299),
        ]),
        make_lca_stream(27, "Heat Loss - Dust cooling", Some(6), None, 0.0, 441.09, 210.0, 60.0, &[]),
    ];
    p.next_id = 30;
    p
}

/// Official Chapter 49.4 benchmark flowsheet: `Iron Process` (`Figs. 14, 22–40`).
pub fn preset_iron_process() -> SimProject {
    let mut p = empty_project();
    p.process_info.process_name = "Blast Furnace Ironmaking".to_string();
    p.process_info.technology = "Blast Furnace / Pyrometallurgy".to_string();
    p.process_info.products = "Pig Iron".to_string();

    p.units = vec![
        SimUnit::new(1, "Air Blast Preheater", UnitKind::Generic, [70.0, 50.0], 0),
        SimUnit::new(2, "Blast Furnace", UnitKind::Reactions, [190.0, 50.0], 0),
    ];

    p.streams = vec![
        // Boundary Inputs
        make_lca_stream(10, "Air Blast", None, Some(1), 242.66, 0.0, 0.0, 25.0, &[
            ("N2(g)", StreamPhaseKind::Gas, 186.14),
            ("O2(g)", StreamPhaseKind::Gas, 56.52),
        ]),
        make_lca_stream(11, "Heat", None, Some(1), 0.0, 81500.48, 48900.0, 25.0, &[]),
        make_lca_stream(12, "Hematite Pellets", None, Some(2), 200.00, 0.0, 12000.0, 25.0, &[
            ("Fe2O3", StreamPhaseKind::PureSolid, 192.0),
            ("SiO2", StreamPhaseKind::PureSolid, 8.0),
        ]),
        make_lca_stream(13, "Coke", None, Some(2), 50.00, 0.0, 410000.0, 25.0, &[
            ("C", StreamPhaseKind::PureSolid, 46.5),
            ("SiO2", StreamPhaseKind::PureSolid, 3.5),
        ]),
        make_lca_stream(14, "Limestone", None, Some(2), 30.42, 0.0, 1800.0, 25.0, &[
            ("CaCO3", StreamPhaseKind::PureSolid, 30.42),
        ]),
        make_lca_stream(15, "Steel Scrap", None, Some(2), 1.87, 0.0, 2100.0, 25.0, &[
            ("Fe", StreamPhaseKind::PureSolid, 1.87),
        ]),
        make_lca_stream(16, "Reductant", None, Some(2), 16.00, 0.0, 112000.0, 25.0, &[
            ("C", StreamPhaseKind::PureSolid, 16.0),
        ]),
        // Internal Black Stream
        make_lca_stream(17, "Hot Air Blast", Some(1), Some(2), 242.66, 81500.48, 48900.0, 1150.0, &[]),
        // Boundary Outputs
        make_lca_stream(20, "Furnace Gas", Some(2), None, 367.11, 24000.0, 310000.0, 210.0, &[
            ("CO(g)", StreamPhaseKind::Gas, 64.50),
            ("CO2(g)", StreamPhaseKind::Gas, 101.3435),
            ("H2(g)", StreamPhaseKind::Gas, 1.66),
            ("H2O(g)", StreamPhaseKind::Gas, 9.92),
            ("SO2(g)", StreamPhaseKind::Gas, 0.16),
            ("N2(g)", StreamPhaseKind::Gas, 186.14),
            ("O2(g)", StreamPhaseKind::Gas, 3.3865),
        ]),
        make_lca_stream(21, "Dust", Some(2), None, 2.05, 50.0, 80.0, 210.0, &[
            ("Fe2O3", StreamPhaseKind::PureSolid, 2.05),
        ]),
        make_lca_stream(22, "Heat Loss", Some(2), None, 0.0, 18500.0, 9200.0, 150.0, &[]),
        make_lca_stream(23, "Slag", Some(2), None, 36.40, -18000.0, 3500.0, 1450.0, &[
            ("CaO", StreamPhaseKind::PureSolid, 16.2),
            ("SiO2", StreamPhaseKind::PureSolid, 14.1),
            ("Al2O3", StreamPhaseKind::PureSolid, 6.1),
        ]),
        make_lca_stream(24, "Pig Iron", Some(2), None, 138.217, -14200.0, 165000.0, 1450.0, &[
            ("Fe", StreamPhaseKind::PureSolid, 132.4),
            ("C", StreamPhaseKind::PureSolid, 5.817),
        ]),
    ];
    p.next_id = 30;
    p
}

#[derive(Debug, Clone, Copy)]
pub struct UnitIconMeta {
    pub key: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub default_kind: UnitKind,
}

/// 15 vector equipment icons rendered natively inside the `Icons` docking gallery (Fig. 65).
pub const UNIT_ICON_GALLERY: &[UnitIconMeta] = &[
    UnitIconMeta { key: "column_tall", label: "Stripper Column", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "packed_tower", label: "Cooling Tower", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "autoclave", label: "Pressure Autoclave", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "sx_mixer_settler", label: "SX Train A", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "sx_bank", label: "SX Extraction Bank", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "flotation_bank", label: "Flotation Circuit", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "thickener", label: "Thickener", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "leach_train", label: "Leach Train", category: "Hydro", default_kind: UnitKind::Reactions },
    UnitIconMeta { key: "filter_plant", label: "Pressure Filter", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "dewatering", label: "Dewatering Unit", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "reactor_tank", label: "Leach Reactor", category: "Hydro", default_kind: UnitKind::Dynamic },
    UnitIconMeta { key: "conditioner", label: "OUTOTEC Conditioner", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "hydrocyclone", label: "Hydrocyclone", category: "Particles", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "rotary_kiln", label: "Rotary Mill / Kiln", category: "Pyro", default_kind: UnitKind::Distributions },
    UnitIconMeta { key: "tankcell_200", label: "TankCell-200", category: "Particles", default_kind: UnitKind::Distributions },
];

pub fn builtin_unit_model_catalog() -> Vec<UnitModelCatalogEntry> {
    vec![
        UnitModelCatalogEntry {
            model_name: "Empty Reactions (Hydro) unit".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "General".into(),
            type_code: String::new(),
            impl_type: ModelImplType::Excel,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Static / Dynamic".into(),
            description: "Empty Reactions (Hydro) unit".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Empty Distribution (Pyro) unit".into(),
            mode: "Distribution (Pyro)".into(),
            technology: "General".into(),
            type_code: String::new(),
            impl_type: ModelImplType::Excel,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Static".into(),
            description: "Empty Distribution (Pyro) unit with element/species splitting".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Dynamic Unit".into(),
            mode: "Dynamic".into(),
            technology: "General".into(),
            type_code: String::new(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Dynamic ODE".into(),
            description: "Dynamic Unit with tank holdup integration (dt = Time Step)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Generic Cooling Tower".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Hydro".into(),
            type_code: "HU-510-10".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Sensible / Latent".into(),
            description: "Water cooling unit model (HU-510-10)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Generic Filter".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Hydro".into(),
            type_code: "HU-810-10".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Solid-Liquid Separation".into(),
            description: "Cake moisture and filtrate recovery filter (HU-810-10)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Generic Thickener".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Hydro".into(),
            type_code: "HU-820-10".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Underflow Solids %".into(),
            description: "Gravity thickener with overflow/underflow split (HU-820-10)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Heat Exchanger".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Heat Transfer".into(),
            type_code: "HU-520-11".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Enthalpy Exchange".into(),
            description: "Two-stream counter-current heat exchanger (HU-520-11)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "MixerTest".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Hydro".into(),
            type_code: "HU-100-1".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Stream Mixing".into(),
            description: "Adiabatic / isothermal stream mixer (HU-100-1)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Chemical Reactions".into(),
            mode: "Reactions (Hydro)".into(),
            technology: "Hydro".into(),
            type_code: String::new(),
            impl_type: ModelImplType::Excel,
            category: ModelLibraryTab::Species,
            version: "10.3.1".into(),
            calculation: "Reaction Equilibrium".into(),
            description: "Spreadsheet-backed hydrometallurgical leaching & oxidation unit".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Hydrocyclone Classifier".into(),
            mode: "Particles".into(),
            technology: "Minerals Processing".into(),
            type_code: "MP-210-10".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Particles,
            version: "10.3.1".into(),
            calculation: "Plitt / Whiten d50".into(),
            description: "Cyclone classification controlled by 50% cut size (um)".into(),
        },
        UnitModelCatalogEntry {
            model_name: "TankCell-200 Flotation".into(),
            mode: "Particles".into(),
            technology: "Minerals Processing".into(),
            type_code: "MP-310-20".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Particles,
            version: "10.3.1".into(),
            calculation: "First-order Kinetics".into(),
            description: "Outotec TankCell flotation model separating Concentrate and Tails".into(),
        },
        UnitModelCatalogEntry {
            model_name: "OKTOP Conditioner".into(),
            mode: "Particles".into(),
            technology: "Minerals Processing".into(),
            type_code: "MP-305-10".into(),
            impl_type: ModelImplType::Dll,
            category: ModelLibraryTab::Particles,
            version: "10.3.1".into(),
            calculation: "Reagent Conditioning".into(),
            description: "Pulp conditioning tank prior to flotation".into(),
        },
        UnitModelCatalogEntry {
            model_name: "Stream Splitter / Junction".into(),
            mode: "Generic".into(),
            technology: "Common".into(),
            type_code: "GN-100-01".into(),
            impl_type: ModelImplType::Excel,
            category: ModelLibraryTab::Others,
            version: "10.3.1".into(),
            calculation: "Mass Split".into(),
            description: "General mass splitter / divider block".into(),
        },
    ]
}

fn default_layers() -> Vec<SimLayer> {
    vec![
        SimLayer { name: "Layer 0 (Default)".into(), visible: true },
        SimLayer { name: "Layer 1 (Streams)".into(), visible: true },
        SimLayer { name: "Layer 2 (Tables)".into(), visible: true },
        SimLayer { name: "Layer 3 (Labels)".into(), visible: true },
        SimLayer { name: "Layer 4".into(), visible: true },
        SimLayer { name: "Layer 5".into(), visible: true },
    ]
}

fn default_pages() -> Vec<SimPage> {
    vec![
        SimPage { name: "Page1".into(), visible: true },
        SimPage { name: "Page2".into(), visible: true },
        SimPage { name: "Page3".into(), visible: true },
    ]
}

/// Blank HSC Sim project matching the empty startup canvas.
pub fn empty_project() -> SimProject {
    SimProject {
        next_id: 1,
        pages: default_pages(),
        active_page: 0,
        layers: default_layers(),
        groups: vec![SimGroup {
            name: "Main Group".into(),
            visible: true,
            unit_ids: Vec::new(),
            stream_ids: Vec::new(),
        }],
        units: Vec::new(),
        streams: Vec::new(),
        tables: Vec::new(),
        annotations: Vec::new(),
        stream_table_sets: vec![StreamTableSet {
            name: "Table Set 1".into(),
            view_mode: TableViewMode::NameValueAndUnit,
            columns: 3,
            rows: 3,
            variables: vec!["Total Solids Flow".into(), "Solids Recovery".into()],
            assigned_streams: Vec::new(),
            header_visible: true,
            use_alias_name: false,
            transpose: false,
        }],
        active_stream_table_set: 0,
        controls: Vec::new(),
        cell_references: Vec::new(),
        scenarios: vec![ScenarioSheet {
            name: "Scenario 1".into(),
            num_runs: 10,
            iteration_rounds: 4,
            show_mc_config: false,
            show_dist_charts: false,
            columns: vec![ScenarioColumn {
                unit_name: String::new(),
                variable_name: String::new(),
                unit_str: String::new(),
                is_set: true,
                cell_ref: "<Insert reference>".into(),
                monte_carlo_on: false,
                dist_type: MonteCarloDist::Uniform,
                mc_min: 0.0,
                mc_max: 10.0,
                mc_mean: 5.0,
                mc_sd: 2.89,
                mc_b: 2.0,
                mc_c: 8.0,
                round_to_nearest: None,
                decimals: 2,
            }],
            values: vec![vec![0.0]; 10],
            run_rounds: vec![4; 10],
        }],
        active_scenario: 0,
        process_info: ProcessInformationData::default(),
        calc_settings: SimCalcSettings::default(),
        canvas_settings: DefaultCanvasSettings::default(),
        visualization: VisualizationSettings::default(),
        lock_state: LockState::Unlocked,
        dynamic_time_s: 0.0,
        dynamic_progress_val: 50,
        convergence_history: Vec::new(),
        imported_models: Vec::new(),
        logs: vec![SimLogEntry {
            severity: LogSeverity::Success,
            duration_s: 0.05,
            message: "Initialized empty HSC Sim project".into(),
            children: Vec::new(),
        }],
    }
}

/// Preset matching Figs. 2–6 (`40.1_model.Sim`): Unit 1, Unit 2 and Streams 1..5.
pub fn preset_40_1_model() -> SimProject {
    let mut p = empty_project();
    let mut u1 = SimUnit::new(1, "Unit 1", UnitKind::Reactions, [12.0, 22.0], 0);
    let mut u2 = SimUnit::new(2, "Unit 2", UnitKind::Reactions, [84.0, 38.0], 0);
    u1.size = [44.0, 28.0];
    u2.size = [44.0, 28.0];

    let mut s1 = SimStream::new(3, "Stream 1", None, Some(1), [-26.0, 32.0], [12.0, 32.0], 0);
    s1.mass_flow_t_h = 100.0;
    s1.solids_flow_t_h = 60.0;
    s1.liquid_flow_t_h = 40.0;
    let mut s2 = SimStream::new(4, "Stream 2", Some(1), Some(2), [56.0, 32.0], [106.0, 38.0], 0);
    s2.waypoints = vec![[106.0, 32.0]];
    s2.mass_flow_t_h = 70.0;
    let mut s4 = SimStream::new(5, "Stream 4", Some(1), None, [34.0, 50.0], [34.0, 78.0], 0);
    s4.mass_flow_t_h = 30.0;
    let mut s3 = SimStream::new(6, "Stream 3", Some(2), None, [128.0, 52.0], [166.0, 52.0], 0);
    s3.mass_flow_t_h = 45.0;
    let mut s5 = SimStream::new(7, "Stream 5", Some(2), None, [106.0, 66.0], [106.0, 98.0], 0);
    s5.mass_flow_t_h = 25.0;

    p.units = vec![u1, u2];
    p.streams = vec![s1, s2, s4, s3, s5];
    p.next_id = 8;
    p.process_info.project_name = "40.1_model".into();
    p.process_info.process_name = "40.1_model".into();
    p.process_info.project_path = "examples://40.1_model.Sim".into();
    p.logs = vec![SimLogEntry {
        severity: LogSeverity::Success,
        duration_s: 0.14,
        message: "Loaded example 40.1_model.Sim".into(),
        children: Vec::new(),
    }];
    p
}

/// Preset matching Figs. 24–28, 35, 38–40, 59–70 (`Hydro_example.Sim`): Cooler + Leaching.
pub fn preset_hydro_example() -> SimProject {
    let mut p = empty_project();

    let mut cooler = SimUnit::new(1, "Cooler", UnitKind::Reactions, [18.0, 96.0], 0);
    cooler.model_mode = "Reactions (Hydro)".into();
    cooler.model_name = "Reactions (Hydro)".into();
    cooler.technology = "Hydro".into();
    cooler.behavior = UnitBehavior::HydroCooler {
        target_temp_c: 60.0,
        coolant_flow_t_h: 197.384,
        target_duty_kw: 8028.12,
    };

    let mut leaching = SimUnit::new(2, "Leaching", UnitKind::Reactions, [64.0, 26.0], 0);
    leaching.model_mode = "Reactions (Hydro)".into();
    leaching.model_name = "Chemical Reactions".into();
    leaching.technology = "Hydro".into();
    leaching.size = [48.0, 34.0];
    leaching.behavior = UnitBehavior::HydroLeaching {
        reaction_temp_c: 70.0,
        acid_feed_t_h: 10.09747,
        target_acid_g_l: 1.0,
        conversion_pct: 92.0,
    };

    let mut air = SimStream::new(3, "Air", None, Some(2), [88.0, 88.0], [88.0, 60.0], 0);
    air.temp_c = 25.0;
    air.pressure_bar = 1.0;
    air.mass_flow_t_h = 14.06;
    air.gas_flow_nm3_h = 11071.85;
    air.exergy_kw = 183.16;
    air.cp_kj_kg_k = 1.01;
    air.species_flows = vec![
        StreamSpeciesFlow { formula: "N2(g)".into(), phase: StreamPhaseKind::Gas, mass_flow_t_h: 10.79, enthalpy_kw: 0.0, exergy_kw: 140.0 },
        StreamSpeciesFlow { formula: "O2(g)".into(), phase: StreamPhaseKind::Gas, mass_flow_t_h: 3.27, enthalpy_kw: 0.0, exergy_kw: 43.16 },
    ];

    let mut fes = SimStream::new(4, "FeS", None, Some(2), [22.0, 34.0], [64.0, 34.0], 0);
    fes.temp_c = 25.0;
    fes.pressure_bar = 1.0;
    fes.mass_flow_t_h = 60.00;
    fes.liquid_flow_t_h = 50.00;
    fes.solids_flow_t_h = 10.00;
    fes.enthalpy_kw = -223449.80;
    fes.exergy_kw = 28606.72;
    fes.cp_kj_kg_k = 3.58;

    let mut h2so4 = SimStream::new(5, "H2SO4", None, Some(2), [22.0, 52.0], [64.0, 52.0], 0);
    h2so4.temp_c = 25.0;
    h2so4.pressure_bar = 1.0;
    h2so4.mass_flow_t_h = 10.10;
    h2so4.liquid_flow_t_h = 10.10;
    h2so4.enthalpy_kw = -23278.49;
    h2so4.exergy_kw = 4671.32;
    h2so4.cp_kj_kg_k = 1.42;

    let mut offgas = SimStream::new(6, "Offgas", Some(2), None, [88.0, 26.0], [88.0, 6.0], 0);
    offgas.temp_c = 70.0;
    offgas.pressure_bar = 1.0;
    offgas.mass_flow_t_h = 12.43;
    offgas.gas_flow_nm3_h = 9909.31;
    offgas.enthalpy_kw = 159.16;
    offgas.exergy_kw = 138.34;
    offgas.cp_kj_kg_k = 1.03;
    offgas.thermal_energy_kw = 159.16;
    offgas.heat_content_kw = 159.16;
    offgas.species_flows = vec![
        StreamSpeciesFlow { formula: "H2O(g)".into(), phase: StreamPhaseKind::Gas, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
        StreamSpeciesFlow { formula: "N2(g)".into(), phase: StreamPhaseKind::Gas, mass_flow_t_h: 10.79, enthalpy_kw: 138.1, exergy_kw: 118.0 },
        StreamSpeciesFlow { formula: "O2(g)".into(), phase: StreamPhaseKind::Gas, mass_flow_t_h: 1.64, enthalpy_kw: 21.06, exergy_kw: 20.34 },
        StreamSpeciesFlow { formula: "Fe(+2a)".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
        StreamSpeciesFlow { formula: "H(+a)".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
        StreamSpeciesFlow { formula: "H2O".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
        StreamSpeciesFlow { formula: "H2SO4".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
        StreamSpeciesFlow { formula: "SO4(-2a)".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 0.00, enthalpy_kw: 0.0, exergy_kw: 0.0 },
    ];

    let mut solution = SimStream::new(7, "Solution", Some(2), None, [112.0, 44.0], [154.0, 44.0], 0);
    solution.temp_c = 70.0;
    solution.pressure_bar = 1.0;
    solution.mass_flow_t_h = 71.74;
    solution.liquid_flow_t_h = 67.45;
    solution.solids_flow_t_h = 4.28;
    solution.enthalpy_kw = -254915.78;
    solution.exergy_kw = 25834.80;
    solution.cp_kj_kg_k = 2.74;
    solution.thermal_energy_kw = 2429.31;
    solution.heat_content_kw = 2429.31;
    solution.species_flows = vec![
        StreamSpeciesFlow { formula: "H2O".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 56.20, enthalpy_kw: -210000.0, exergy_kw: 4100.0 },
        StreamSpeciesFlow { formula: "Fe(+2a)".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 6.35, enthalpy_kw: -18200.0, exergy_kw: 9100.0 },
        StreamSpeciesFlow { formula: "SO4(-2a)".into(), phase: StreamPhaseKind::Water, mass_flow_t_h: 4.90, enthalpy_kw: -22500.0, exergy_kw: 8200.0 },
        StreamSpeciesFlow { formula: "FeS".into(), phase: StreamPhaseKind::PureSolid, mass_flow_t_h: 4.28, enthalpy_kw: -4215.78, exergy_kw: 4434.80 },
    ];

    let mut cold_w = SimStream::new(8, "Cold Water", None, Some(1), [-18.0, 110.0], [18.0, 110.0], 0);
    cold_w.temp_c = 25.0;
    cold_w.pressure_bar = 1.0;
    cold_w.mass_flow_t_h = 197.38;
    cold_w.liquid_flow_t_h = 197.38;
    cold_w.enthalpy_kw = -869914.76;
    cold_w.exergy_kw = 2857.51;
    cold_w.cp_kj_kg_k = 4.18;
    cold_w.species_flows = vec![StreamSpeciesFlow {
        formula: "H2O".into(),
        phase: StreamPhaseKind::Water,
        mass_flow_t_h: 197.38,
        enthalpy_kw: -869914.76,
        exergy_kw: 2857.51,
    }];

    let mut hot_w = SimStream::new(9, "Hot Water", Some(1), None, [60.0, 110.0], [104.0, 110.0], 0);
    hot_w.temp_c = 60.0;
    hot_w.pressure_bar = 1.0;
    hot_w.mass_flow_t_h = 197.38;
    hot_w.liquid_flow_t_h = 197.38;
    hot_w.enthalpy_kw = -861886.63;
    hot_w.exergy_kw = 3295.05;
    hot_w.cp_kj_kg_k = 4.19;
    hot_w.thermal_energy_kw = 8028.12;
    hot_w.heat_content_kw = 8028.12;
    hot_w.species_flows = vec![StreamSpeciesFlow {
        formula: "H2O".into(),
        phase: StreamPhaseKind::Water,
        mass_flow_t_h: 197.38,
        enthalpy_kw: -861886.63,
        exergy_kw: 3295.05,
    }];

    p.units = vec![cooler, leaching];
    p.streams = vec![air, cold_w, fes, h2so4, hot_w, offgas, solution];
    p.next_id = 10;

    p.controls = vec![
        SimControlLoop {
            id: 1,
            control_name: "Heat bal".into(),
            location_unit: "Cooler".into(),
            active: true,
            set_point: 0.0,
            measured: -0.209,
            tolerance: 0.5,
            y_target_name: "Heat bal".into(),
            y_unit: "kW".into(),
            y_cell_ref: "Model!C12".into(),
            x_var_name: "Cold Water".into(),
            control_variable: 197.384,
            x_unit: "t/h".into(),
            x_min: 50.0,
            x_max: 400.0,
            x_max_step: Some(50.0),
            x_cell_ref: "Input!E4".into(),
            control_type: "Internal".into(),
            method: ControlMethod::TangentialFast,
            max_iter: 10,
            kp: 0.8,
            ti_s: 15.0,
            td_s: 0.0,
            tdf_s: 0.0,
        },
        SimControlLoop {
            id: 2,
            control_name: "Acid conc.".into(),
            location_unit: "Leaching".into(),
            active: true,
            set_point: 1.0,
            measured: 0.997,
            tolerance: 0.01,
            y_target_name: "Acid conc.".into(),
            y_unit: "g/l".into(),
            y_cell_ref: "Output!E36".into(),
            x_var_name: "Acid feed".into(),
            control_variable: 10.09747,
            x_unit: "t/h".into(),
            x_min: 0.0,
            x_max: 100.0,
            x_max_step: None,
            x_cell_ref: "Input!F20".into(),
            control_type: "Internal".into(),
            method: ControlMethod::TangentialFast,
            max_iter: 10,
            kp: 0.5,
            ti_s: 10.0,
            td_s: 0.0,
            tdf_s: 0.0,
        },
    ];

    p.cell_references = vec![
        SimCellReference {
            unit_name: "Leaching".into(),
            ref_name: "_Signal_1".into(),
            value: -8028.330884,
            sheet: "Output".into(),
            cell: "I6".into(),
        },
        SimCellReference {
            unit_name: "Leaching".into(),
            ref_name: "_Signal_2".into(),
            value: -254756.6219,
            sheet: "Output".into(),
            cell: "I14".into(),
        },
    ];

    p.imported_models = vec![UnitModelCatalogEntry {
        model_name: "Leaching".into(),
        mode: String::new(),
        technology: String::new(),
        type_code: String::new(),
        impl_type: ModelImplType::Excel,
        category: ModelLibraryTab::Imported,
        version: String::new(),
        calculation: "Chemical Reactions".into(),
        description: "Imported hydrometallurgical leaching sheet".into(),
    }];

    p.process_info.project_name = "Hydro_example".into();
    p.process_info.process_name = "Hydro_example".into();
    p.process_info.project_path = "examples://Flowsheet_Hydro/Hydro_example.Sim".into();
    p.process_info.technology = "Hydrometallurgy".into();
    p.calc_settings.max_rounds = 5;

    p.logs = vec![
        SimLogEntry {
            severity: LogSeverity::Warning,
            duration_s: 0.0,
            message: "Process inspection messages".into(),
            children: vec![
                SimLogEntry {
                    severity: LogSeverity::Success,
                    duration_s: 0.22,
                    message: "Loading preview".into(),
                    children: Vec::new(),
                },
                SimLogEntry {
                    severity: LogSeverity::Warning,
                    duration_s: 2.04,
                    message: "Read process unit Hydro_example.Sim".into(),
                    children: Vec::new(),
                },
            ],
        },
    ];
    p
}

/// Preset matching Figs. 9–10, 29–34: Hydrocyclone + Conditioner + Flotation Cell (TankCell-200)
/// with Stream Tables, Flow Table, and Scenario Editor (Cyclone cut size 50..350 um -> Cu Rec %).
pub fn preset_concentrator_flotation() -> SimProject {
    let mut p = empty_project();

    let mut cyc = SimUnit::new(1, "Hydrocyclone", UnitKind::Distributions, [8.0, 30.0], 0);
    cyc.icon_key = Some("hydrocyclone".into());
    cyc.model_mode = "Particles".into();
    cyc.model_name = "Hydrocyclone Classifier".into();
    cyc.technology = "Minerals Processing".into();
    cyc.size = [22.0, 32.0];
    cyc.behavior = UnitBehavior::Hydrocyclone {
        cut_size_um: 175.0,
        bypass_fines: 0.15,
    };

    let mut cond = SimUnit::new(2, "Conditioner", UnitKind::Distributions, [56.0, 28.0], 0);
    cond.icon_key = Some("conditioner".into());
    cond.model_mode = "Particles".into();
    cond.model_name = "OKTOP Conditioner".into();
    cond.technology = "Minerals Processing".into();
    cond.size = [26.0, 30.0];
    cond.behavior = UnitBehavior::Conditioner {
        residence_s: 300.0,
        temp_c: 25.0,
    };

    let mut cell = SimUnit::new(3, "Flotation Cell", UnitKind::Distributions, [112.0, 26.0], 0);
    cell.icon_key = Some("tankcell_200".into());
    cell.model_mode = "Particles".into();
    cell.model_name = "TankCell-200 Flotation".into();
    cell.technology = "Minerals Processing".into();
    cell.size = [30.0, 34.0];
    cell.behavior = UnitBehavior::FlotationCell {
        concentrate_solids_rec_pct: 3.44,
        base_cu_rec_pct: 66.04,
    };

    let mut feed = SimStream::new(4, "Ore Feed", None, Some(1), [-20.0, 42.0], [8.0, 42.0], 0);
    feed.mass_flow_t_h = 850.0;
    feed.solids_flow_t_h = 600.00;
    feed.liquid_flow_t_h = 250.00;
    feed.pct_solids = 70.59;
    feed.cu_wt_pct = 0.76;
    feed.ccp_rec_pct = 100.0;
    feed.p80_um = 110.0;

    let mut ovf = SimStream::new(5, "Hydrocyclone Overflow", Some(1), Some(2), [30.0, 38.0], [56.0, 38.0], 0);
    ovf.mass_flow_t_h = 800.0;
    ovf.solids_flow_t_h = 600.00;
    ovf.liquid_flow_t_h = 200.00;
    ovf.cu_wt_pct = 0.76;
    ovf.ccp_rec_pct = 100.00;
    ovf.p80_um = 77.15;

    let mut cout = SimStream::new(6, "Conditioner Output", Some(2), Some(3), [82.0, 42.0], [112.0, 42.0], 0);
    cout.mass_flow_t_h = 800.0;
    cout.solids_flow_t_h = 600.00;
    cout.liquid_flow_t_h = 200.00;
    cout.cu_wt_pct = 0.76;
    cout.ccp_rec_pct = 100.00;
    cout.p80_um = 77.15;

    let mut tails = SimStream::new(7, "Tails", Some(3), None, [142.0, 44.0], [186.0, 44.0], 0);
    tails.mass_flow_t_h = 772.0;
    tails.solids_flow_t_h = 579.33;
    tails.solids_rec_pct = 96.56;
    tails.cu_wt_pct = 0.29;
    tails.ccp_rec_pct = 36.41;
    tails.cu_rec_pct = 33.96;
    tails.p80_um = 76.71;
    tails.visible_stream_table = true;

    let mut conc = SimStream::new(8, "Concentrate", Some(3), None, [127.0, 60.0], [127.0, 96.0], 0);
    conc.mass_flow_t_h = 28.0;
    conc.solids_flow_t_h = 20.67;
    conc.solids_rec_pct = 3.44;
    conc.cu_wt_pct = 14.08;
    conc.ccp_rec_pct = 63.59;
    conc.cu_rec_pct = 66.04;
    conc.p80_um = 85.50;
    conc.visible_stream_table = true;

    p.units = vec![cyc, cond, cell];
    p.streams = vec![feed, ovf, cout, tails, conc];
    p.next_id = 9;

    p.stream_table_sets = vec![
        StreamTableSet {
            name: "Table Set 1".into(),
            view_mode: TableViewMode::NameValueAndUnit,
            columns: 3,
            rows: 3,
            variables: vec!["Total Solids Flow".into(), "Solids Recovery".into()],
            assigned_streams: vec!["Tails".into(), "Concentrate".into()],
            header_visible: true,
            use_alias_name: false,
            transpose: false,
        },
        StreamTableSet {
            name: "Flow Table 1".into(),
            view_mode: TableViewMode::FlowTable,
            columns: 7,
            rows: 6,
            variables: vec!["Temperature".into(), "Cu".into(), "Ccp".into(), "P80".into()],
            assigned_streams: vec![
                "Hydrocyclone Overflow".into(),
                "Conditioner Output".into(),
                "Tails".into(),
                "Concentrate".into(),
            ],
            header_visible: true,
            use_alias_name: false,
            transpose: false,
        },
    ];

    // Preloaded Fig. 31 / Fig. 33 scenario: Cyclone cut size 50..350 um -> Cu Rec %
    let cuts = [50.0, 75.0, 100.0, 125.0, 150.0, 175.0, 200.0, 250.0, 300.0, 350.0];
    let recs = [53.10, 62.01, 64.08, 65.32, 65.99, 66.04, 65.98, 65.61, 65.18, 64.80];
    let values = cuts.iter().zip(recs.iter()).map(|(&c, &r)| vec![c, r]).collect();

    p.scenarios = vec![ScenarioSheet {
        name: "Scenario 1".into(),
        num_runs: 10,
        iteration_rounds: 4,
        show_mc_config: true,
        show_dist_charts: false,
        columns: vec![
            ScenarioColumn {
                unit_name: "Hydrocyclone".into(),
                variable_name: "Cyclone cut size".into(),
                unit_str: "μm".into(),
                is_set: true,
                cell_ref: "Model!C5".into(),
                monte_carlo_on: false,
                dist_type: MonteCarloDist::Normal,
                mc_min: 150.0,
                mc_max: 250.0,
                mc_mean: 185.0,
                mc_sd: 20.21,
                mc_b: 165.0,
                mc_c: 205.0,
                round_to_nearest: None,
                decimals: 1,
            },
            ScenarioColumn {
                unit_name: "Flotation Cell".into(),
                variable_name: "Cu Rec %".into(),
                unit_str: "%".into(),
                is_set: false,
                cell_ref: "Output!D12".into(),
                monte_carlo_on: false,
                dist_type: MonteCarloDist::Normal,
                mc_min: 53.10,
                mc_max: 66.04,
                mc_mean: 63.81,
                mc_sd: 3.96,
                mc_b: 0.0,
                mc_c: 0.0,
                round_to_nearest: None,
                decimals: 2,
            },
        ],
        values,
        run_rounds: vec![4; 10],
    }];

    p.tables = vec![SimTableObj {
        id: 100,
        pos: [6.0, 96.0],
        size: [72.0, 28.0],
        page: 0,
        layer: 2,
        is_title_table: true,
        size_locked: false,
        cells: vec![],
        title: "Concentrator & Flotation Circuit".into(),
        organization_author: "Soryntel Process Engineering".into(),
        hsc_version: "10.3.1.0".into(),
        date: "25.04.2023".into(),
        logo_text: "Logo".into(),
    }];

    p.process_info.project_name = "Concentrator_Flotation".into();
    p.process_info.process_name = "Concentrator_Flotation".into();
    p.process_info.technology = "Minerals Processing".into();
    p.process_info.products = "Cu Concentrate".into();
    p
}

/// Preset matching Fig. 37: recycle loop (`Unit 1 -> Stream 2 -> Unit 2 -> Stream 4` + internal `Stream 3`).
pub fn preset_recycle_loop() -> SimProject {
    let mut p = empty_project();

    let mut u1 = SimUnit::new(1, "Unit 1", UnitKind::Reactions, [24.0, 30.0], 0);
    u1.behavior = UnitBehavior::PassThrough;
    let mut u2 = SimUnit::new(2, "Unit 2", UnitKind::Distributions, [104.0, 30.0], 0);
    u2.behavior = UnitBehavior::Splitter { main_fraction: 0.72 };

    let mut s1 = SimStream::new(3, "Stream 1", None, Some(1), [-12.0, 44.0], [24.0, 44.0], 0);
    s1.mass_flow_t_h = 100.0;
    s1.enthalpy_kw = -12000.0;
    let mut s2 = SimStream::new(4, "Stream 2", Some(1), Some(2), [66.0, 44.0], [104.0, 44.0], 0);
    s2.mass_flow_t_h = 100.0;
    s2.enthalpy_kw = -12000.0;
    let mut s4 = SimStream::new(5, "Stream 4", Some(2), None, [146.0, 44.0], [186.0, 44.0], 0);
    s4.mass_flow_t_h = 72.0;
    s4.enthalpy_kw = -8640.0;
    let mut s3 = SimStream::new(6, "Stream 3", Some(2), Some(1), [125.0, 58.0], [45.0, 58.0], 0);
    s3.waypoints = vec![[125.0, 78.0], [45.0, 78.0]];
    s3.mass_flow_t_h = 0.0; // Ready to demonstrate iterative convergence!

    p.units = vec![u1, u2];
    p.streams = vec![s1, s2, s4, s3];
    p.next_id = 7;
    p.process_info.project_name = "Recycle_Loop".into();
    p.process_info.process_name = "Recycle_Loop".into();
    p
}
