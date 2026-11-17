//! Built-in material/fluid property tables and documented HSC manual example presets
//! (`SimpleCase`, `Smelting1`, `Radiator`, `Hotwatertank`, `OilFurnace`).

use super::gas_radiation::GasComposition;
use super::model::{
    CalculationMode, ColumnType, ConvectionFunctionPreset, ConvectionFunctionTerm,
    ConvectionGeometry, ConvectionRegime, FluidPropertiesPoint, FluidPropertyTable, Geometry,
    HeatLossColumn, HeatLossSheet, HeatLossWorkbook, SolidConductivityData, SurfaceEmissivityData,
};
use soryntel_core::{HeatFlowUnit, LengthUnit, TemperatureUnit};

pub fn builtin_solids() -> Vec<SolidConductivityData> {
    vec![
        SolidConductivityData {
            name: "REXAL 2S EXTRA".into(),
            category: "Refractories".into(),
            density_kg_m3: Some(2850.0),
            k_table: vec![
                (293.15, 4.50),
                (673.15, 4.10),
                (1073.15, 3.85),
                (1473.15, 3.65),
            ],
            reference: "HSC6 HeatCond".into(),
        },
        SolidConductivityData {
            name: "Carbon steel: (0.2% C)".into(),
            category: "Metals & Alloys".into(),
            density_kg_m3: Some(7850.0),
            k_table: vec![
                (273.15, 51.0),
                (293.15, 50.0),
                (373.15, 49.0),
                (573.15, 44.0),
            ],
            reference: "Incropera & DeWitt".into(),
        },
        SolidConductivityData {
            name: "Stainless steel".into(),
            category: "Metals & Alloys".into(),
            density_kg_m3: Some(7900.0),
            k_table: vec![
                (273.15, 14.8),
                (300.0, 15.1),
                (400.0, 17.3),
                (600.0, 20.0),
                (800.0, 22.8),
                (1000.0, 25.4),
            ],
            reference: "Incropera & DeWitt / Fig. 4".into(),
        },
        SolidConductivityData {
            name: "Urethane foam".into(),
            category: "Insulations".into(),
            density_kg_m3: Some(40.0),
            k_table: vec![(273.15, 0.058), (293.15, 0.060), (373.15, 0.064)],
            reference: "HSC6 HeatCond / Fig. 4".into(),
        },
        SolidConductivityData {
            name: "Fireclay brick".into(),
            category: "Refractories".into(),
            density_kg_m3: Some(2100.0),
            k_table: vec![(473.15, 1.00), (873.15, 1.25), (1473.15, 1.55)],
            reference: "Incropera & DeWitt".into(),
        },
        SolidConductivityData {
            name: "Aluminium Al (pure)".into(),
            category: "Metals & Alloys".into(),
            density_kg_m3: Some(2702.0),
            k_table: vec![
                (100.0, 302.0),
                (200.0, 237.0),
                (300.0, 237.0),
                (400.0, 240.0),
                (600.0, 231.0),
                (800.0, 218.0),
            ],
            reference: "Fig. 5 HeatCond.XLS".into(),
        },
        SolidConductivityData {
            name: "Aluminium 2024-T6".into(),
            category: "Metals & Alloys".into(),
            density_kg_m3: Some(2770.0),
            k_table: vec![
                (100.0, 65.0),
                (200.0, 163.0),
                (300.0, 177.0),
                (400.0, 186.0),
                (600.0, 186.0),
            ],
            reference: "Fig. 5 HeatCond.XLS".into(),
        },
        SolidConductivityData {
            name: "Copper Cu (pure)".into(),
            category: "Metals & Alloys".into(),
            density_kg_m3: Some(8933.0),
            k_table: vec![
                (100.0, 482.0),
                (200.0, 413.0),
                (300.0, 401.0),
                (400.0, 393.0),
                (600.0, 379.0),
                (800.0, 366.0),
            ],
            reference: "Fig. 5 HeatCond.XLS".into(),
        },
        SolidConductivityData {
            name: "Firebrick".into(),
            category: "Refractories".into(),
            density_kg_m3: Some(2050.0),
            k_table: vec![(293.15, 0.40), (773.15, 0.40), (1273.15, 0.45)],
            reference: "SimpleCase Ch. 12.2.1".into(),
        },
        SolidConductivityData {
            name: "Silica brick".into(),
            category: "Refractories".into(),
            density_kg_m3: Some(1800.0),
            k_table: vec![(293.15, 0.20), (773.15, 0.20), (1273.15, 0.24)],
            reference: "SimpleCase Ch. 12.2.1".into(),
        },
    ]
}

pub fn builtin_fluids() -> Vec<FluidPropertyTable> {
    vec![
        FluidPropertyTable {
            name: "Air (gas)".into(),
            is_gas: true,
            points: vec![
                FluidPropertiesPoint { t_k: 250.0, density_kg_m3: 1.3947, cp_j_kg_k: 1006.0, k_w_m_k: 0.0223, mu_pa_s: 1.596e-5, beta_1_k: None },
                FluidPropertiesPoint { t_k: 300.0, density_kg_m3: 1.1614, cp_j_kg_k: 1007.0, k_w_m_k: 0.0263, mu_pa_s: 1.846e-5, beta_1_k: None },
                FluidPropertiesPoint { t_k: 350.0, density_kg_m3: 0.9950, cp_j_kg_k: 1009.0, k_w_m_k: 0.0300, mu_pa_s: 2.082e-5, beta_1_k: None },
                FluidPropertiesPoint { t_k: 400.0, density_kg_m3: 0.8711, cp_j_kg_k: 1014.0, k_w_m_k: 0.0338, mu_pa_s: 2.301e-5, beta_1_k: None },
                FluidPropertiesPoint { t_k: 600.0, density_kg_m3: 0.5804, cp_j_kg_k: 1051.0, k_w_m_k: 0.0469, mu_pa_s: 3.058e-5, beta_1_k: None },
                FluidPropertiesPoint { t_k: 1000.0, density_kg_m3: 0.3482, cp_j_kg_k: 1141.0, k_w_m_k: 0.0667, mu_pa_s: 4.244e-5, beta_1_k: None },
            ],
            reference: "Incropera & DeWitt (1 atm)".into(),
        },
        FluidPropertyTable {
            name: "Water H2O (liquid)".into(),
            is_gas: false,
            points: vec![
                FluidPropertiesPoint { t_k: 273.15, density_kg_m3: 999.8, cp_j_kg_k: 4217.0, k_w_m_k: 0.561, mu_pa_s: 1.793e-3, beta_1_k: Some(-6.8e-5) },
                FluidPropertiesPoint { t_k: 298.15, density_kg_m3: 997.06, cp_j_kg_k: 4180.0, k_w_m_k: 0.6072, mu_pa_s: 0.890e-3, beta_1_k: Some(2.57e-4) },
                FluidPropertiesPoint { t_k: 313.15, density_kg_m3: 992.2, cp_j_kg_k: 4179.0, k_w_m_k: 0.631, mu_pa_s: 0.653e-3, beta_1_k: Some(3.85e-4) },
                FluidPropertiesPoint { t_k: 323.15, density_kg_m3: 988.0, cp_j_kg_k: 4181.0, k_w_m_k: 0.644, mu_pa_s: 0.547e-3, beta_1_k: Some(4.58e-4) },
                FluidPropertiesPoint { t_k: 363.15, density_kg_m3: 965.3, cp_j_kg_k: 4206.0, k_w_m_k: 0.675, mu_pa_s: 0.315e-3, beta_1_k: Some(6.95e-4) },
            ],
            reference: "Haar, Gallagher & Kell (NBS/NRC)".into(),
        },
    ]
}

pub fn builtin_convection_functions() -> Vec<ConvectionFunctionPreset> {
    // Exact match to Fig. 7 (Thermal Convection database window, Function sheet)
    vec![
        ConvectionFunctionPreset {
            name: "Molten metal".into(),
            subtitle: "(inside furnace)".into(),
            terms: vec![
                ConvectionFunctionTerm { coeff: 100.0, exponent: 0.0 },
                ConvectionFunctionTerm { coeff: 2000.0, exponent: 1.0 },
                ConvectionFunctionTerm { coeff: 10000.0, exponent: 2.0 },
            ],
        },
        ConvectionFunctionPreset {
            name: "Water, Spray cooling".into(),
            subtitle: "(outside furnace)".into(),
            terms: vec![ConvectionFunctionTerm { coeff: 1200.0, exponent: 0.0 }],
        },
        ConvectionFunctionPreset {
            name: "Water, Iron elements cooling".into(),
            subtitle: "(outside furnace)".into(),
            terms: vec![ConvectionFunctionTerm { coeff: 2400.0, exponent: 0.0 }],
        },
        ConvectionFunctionPreset {
            name: "Water, Copper elements cooling".into(),
            subtitle: "(outside furnace)".into(),
            terms: vec![ConvectionFunctionTerm { coeff: 2400.0, exponent: 0.0 }],
        },
    ]
}

pub fn builtin_emissivities() -> Vec<SurfaceEmissivityData> {
    vec![
        SurfaceEmissivityData { name: "Paint, white (acrylic) (h)".into(), emissivity: 0.90, measurement_type: "(h)".into(), temp_range_desc: "300 K".into() },
        SurfaceEmissivityData { name: "Aluminium (h)".into(), emissivity: 0.08, measurement_type: "(h)".into(), temp_range_desc: "300 K".into() },
        SurfaceEmissivityData { name: "Oxidized steel (n)".into(), emissivity: 0.79, measurement_type: "(n)".into(), temp_range_desc: "300-800 K".into() },
        SurfaceEmissivityData { name: "Refractory brick (h)".into(), emissivity: 0.85, measurement_type: "(h)".into(), temp_range_desc: "500-1500 K".into() },
        SurfaceEmissivityData { name: "Ideal black surface".into(), emissivity: 1.00, measurement_type: "(h)".into(), temp_range_desc: "273 K".into() },
        SurfaceEmissivityData { name: "Aluminium (h) (highly polished, film)".into(), emissivity: 0.04, measurement_type: "(h)".into(), temp_range_desc: "100-600 K".into() },
        SurfaceEmissivityData { name: "Aluminium (h) (anodized)".into(), emissivity: 0.82, measurement_type: "(h)".into(), temp_range_desc: "300-400 K".into() },
        SurfaceEmissivityData { name: "Steel (sheet with rough oxide layer) (h)".into(), emissivity: 0.80, measurement_type: "(h)".into(), temp_range_desc: "295-600 K".into() },
        SurfaceEmissivityData { name: "Steel (sheet with skin due to rolling) (h)".into(), emissivity: 0.66, measurement_type: "(h)".into(), temp_range_desc: "295-1200 K".into() },
    ]
}

/// Example 1: `SimpleCase.htr8` (Wall A = 1.5 m^2, 700 -> 20 °C, exact Q = 735 W).
pub fn preset_simple_case() -> HeatLossWorkbook {
    HeatLossWorkbook {
        title: "SimpleCase.htr8".into(),
        temp_unit: TemperatureUnit::Celsius,
        heat_flow_unit: HeatFlowUnit::Watt,
        length_unit: LengthUnit::Meter,
        calculate_all_sheets: true,
        active_sheet: 0,
        sheets: vec![HeatLossSheet {
            name: "SimpleCase".into(),
            geometry: Geometry::Wall {
                width_m: 1.5,
                length_m: 1.0,
            },
            mode: CalculationMode::HeatFlow,
            fixed_heat_flow_w: 735.0,
            convection_geometry_override: None,
            columns: vec![
                HeatLossColumn {
                    column_type: ColumnType::Surface,
                    layer_material: "Hot Gas".into(),
                    t_left_k: 700.0 + 273.15,
                    t_left_fixed: true,
                    t_right_k: 418.0 + 273.15,
                    hc_w_m2_k: 1.737_588_652_5,
                    hc_fixed: true,
                    hr_w_m2_k: 0.0,
                    hr_fixed: true,
                    ..Default::default()
                },
                HeatLossColumn {
                    column_type: ColumnType::SolidLayer,
                    layer_material: "Wall".into(),
                    thickness_m: 0.25,
                    k_w_m_k: 0.333_442_255_8,
                    k_fixed: true,
                    ..Default::default()
                },
                HeatLossColumn {
                    column_type: ColumnType::Surface,
                    layer_material: "Ambient Air".into(),
                    t_right_k: 20.0 + 273.15,
                    t_right_fixed: true,
                    hc_w_m2_k: 15.995_648_052_2,
                    hc_fixed: true,
                    hr_w_m2_k: 0.0,
                    hr_fixed: true,
                    ..Default::default()
                },
            ],
        }],
    }
}

/// Example 2: `Smelting1.htr8` (Fig. 1 — 1.00 m x 1.00 m, 1200/1400/1600 -> 20 °C, Q = 4.251 kW at 1200 °C).
pub fn preset_smelting1() -> HeatLossWorkbook {
    let make_smelting_sheet = |name: &str, t_c: f64| HeatLossSheet {
        name: name.into(),
        geometry: Geometry::Wall {
            width_m: 1.0,
            length_m: 1.0,
        },
        mode: CalculationMode::HeatFlow,
        fixed_heat_flow_w: 4251.0,
        convection_geometry_override: None,
        columns: vec![
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Molten metal".into(),
                t_left_k: t_c + 273.15,
                t_left_fixed: true,
                hc_w_m2_k: 144.0,
                hc_fixed: true,
                hr_w_m2_k: 0.0,
                hr_fixed: true,
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::SolidLayer,
                layer_material: "REXAL 2S EXTRA".into(),
                thickness_m: 1.0,
                calculation_grid: 10,
                k_w_m_k: 3.7005,
                k_fixed: true,
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Water, Copper elements cooling".into(),
                t_right_k: 20.0 + 273.15,
                t_right_fixed: true,
                hc_w_m2_k: 2400.0,
                hc_fixed: true,
                hr_w_m2_k: 0.0,
                hr_fixed: true,
                ..Default::default()
            },
        ],
    };

    HeatLossWorkbook {
        title: "Smelting1.htr8".into(),
        temp_unit: TemperatureUnit::Celsius,
        heat_flow_unit: HeatFlowUnit::Kilowatt,
        length_unit: LengthUnit::Meter,
        calculate_all_sheets: false,
        active_sheet: 0,
        sheets: vec![
            make_smelting_sheet("Smelting 1200°C", 1200.0),
            make_smelting_sheet("Smelting 1400°C", 1400.0),
            make_smelting_sheet("Smelting 1600°C", 1600.0),
        ],
    }
}

/// Example 3: `Radiator.htr8` (Fig. 2 — 2.00 m x 0.50 m, 40/50/60 -> 20 °C, Q = 0.174 kW at 40 °C).
pub fn preset_radiator() -> HeatLossWorkbook {
    let fluids = builtin_fluids();
    let water = fluids[1].clone();
    let air = fluids[0].clone();

    let make_rad_sheet = |name: &str, t_c: f64| HeatLossSheet {
        name: name.into(),
        geometry: Geometry::Wall {
            width_m: 2.0,
            length_m: 0.5,
        },
        mode: CalculationMode::HeatFlow,
        fixed_heat_flow_w: 174.0,
        convection_geometry_override: None,
        columns: vec![
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Water H2O (liquid)".into(),
                t_left_k: t_c + 273.15,
                t_left_fixed: true,
                hc_fixed: false,
                convection_regime: ConvectionRegime::Free,
                convection_geometry: ConvectionGeometry::VerticalWall,
                fluid_data: Some(water.clone()),
                hr_w_m2_k: 0.0,
                hr_fixed: true,
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::SolidLayer,
                layer_material: "Carbon steel: (0.2% C)".into(),
                thickness_m: 0.003,
                calculation_grid: 10,
                k_w_m_k: 50.0,
                k_fixed: true,
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Air (gas)".into(),
                surface_material: "Paint, white (acrylic) (h)".into(),
                t_right_k: 20.0 + 273.15,
                t_right_fixed: true,
                hc_fixed: false,
                convection_regime: ConvectionRegime::Free,
                convection_geometry: ConvectionGeometry::VerticalWall,
                fluid_data: Some(air.clone()),
                hr_fixed: false,
                emissivity: 0.90,
                ..Default::default()
            },
        ],
    };

    HeatLossWorkbook {
        title: "Radiator.htr8".into(),
        temp_unit: TemperatureUnit::Celsius,
        heat_flow_unit: HeatFlowUnit::Kilowatt,
        length_unit: LengthUnit::Meter,
        calculate_all_sheets: false,
        active_sheet: 0,
        sheets: vec![
            make_rad_sheet("Radiator 40°C", 40.0),
            make_rad_sheet("Radiator 50°C", 50.0),
            make_rad_sheet("Radiator 60°C", 60.0),
        ],
    }
}

/// Example 6: `OilFurnace.htr8` (Ch. 12.2.1 item 6 — 1200 °C flue gas to 500 °C steel wall).
pub fn preset_oil_furnace() -> HeatLossWorkbook {
    let fluids = builtin_fluids();
    let air = fluids[0].clone();
    HeatLossWorkbook {
        title: "OilFurnace.htr8".into(),
        temp_unit: TemperatureUnit::Celsius,
        heat_flow_unit: HeatFlowUnit::Kilowatt,
        length_unit: LengthUnit::Meter,
        calculate_all_sheets: true,
        active_sheet: 0,
        sheets: vec![HeatLossSheet {
            name: "Oil Furnace".into(),
            geometry: Geometry::Cylinder {
                height_m: 3.0,
                diameter_m: 1.5,
            },
            mode: CalculationMode::HeatFlow,
            fixed_heat_flow_w: 50000.0,
            convection_geometry_override: None,
            columns: vec![
                HeatLossColumn {
                    column_type: ColumnType::Surface,
                    layer_material: "Flue gas (OilFurnace)".into(),
                    surface_material: "Steel (sheet with skin due to rolling) (h)".into(),
                    t_left_k: 1200.0 + 273.15,
                    t_left_fixed: true,
                    hc_fixed: false,
                    fluid_data: Some(air),
                    hr_fixed: false,
                    emissivity: 0.66,
                    gas_emissivity: Some(0.285),
                    gas_absorptivity: Some(0.395),
                    ..Default::default()
                },
                HeatLossColumn {
                    column_type: ColumnType::SolidLayer,
                    layer_material: "Plain carbon steel".into(),
                    t_right_k: 500.0 + 273.15,
                    t_right_fixed: true,
                    thickness_m: 0.010,
                    calculation_grid: 10,
                    k_w_m_k: 45.0,
                    k_fixed: true,
                    ..Default::default()
                },
            ],
        }],
    }
}

/// Example 4: `Hotwatertank.htr8` (Fig. 4 — Cylinder H=4 m, D=3 m, 50 -> 20 °C, Q ≈ 1.378 kW).
pub fn preset_hotwatertank() -> HeatLossWorkbook {
    let fluids = builtin_fluids();
    let solids = builtin_solids();
    let water = fluids[1].clone();
    let air = fluids[0].clone();
    let stainless = solids[2].clone();
    let urethane = solids[3].clone();

    let make_cols = |hc_in: f64, hc_out: f64, hr_out: f64| {
        vec![
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Water H2O (liquid)".into(),
                t_left_k: 50.0 + 273.15,
                t_left_fixed: true,
                hc_w_m2_k: hc_in,
                hc_fixed: true,
                fluid_data: Some(water.clone()),
                hr_w_m2_k: 0.0,
                hr_fixed: true,
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::SolidLayer,
                layer_material: "Stainless steel".into(),
                thickness_m: 0.005,
                calculation_grid: 10,
                k_w_m_k: 15.606,
                k_fixed: false,
                solid_data: Some(stainless.clone()),
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::SolidLayer,
                layer_material: "Urethane foam".into(),
                thickness_m: 0.050,
                calculation_grid: 10,
                k_w_m_k: 0.060,
                k_fixed: false,
                solid_data: Some(urethane.clone()),
                ..Default::default()
            },
            HeatLossColumn {
                column_type: ColumnType::Surface,
                layer_material: "Air (gas)".into(),
                surface_material: "Aluminium (h)".into(),
                t_right_k: 20.0 + 273.15,
                t_right_fixed: true,
                hc_w_m2_k: hc_out,
                hc_fixed: true,
                fluid_data: Some(air.clone()),
                hr_w_m2_k: hr_out,
                hr_fixed: false,
                emissivity: 0.08,
                ..Default::default()
            },
        ]
    };

    let d = 3.0;
    let r = 0.5 * d;
    let cap_area = std::f64::consts::PI * r * r;

    HeatLossWorkbook {
        title: "Hotwatertank.htr8".into(),
        temp_unit: TemperatureUnit::Celsius,
        heat_flow_unit: HeatFlowUnit::Kilowatt,
        length_unit: LengthUnit::Meter,
        calculate_all_sheets: true,
        active_sheet: 0,
        sheets: vec![
            HeatLossSheet {
                name: "Wall".into(),
                geometry: Geometry::Cylinder {
                    height_m: 4.0,
                    diameter_m: 3.0,
                },
                mode: CalculationMode::HeatFlow,
                fixed_heat_flow_w: 1100.0,
                convection_geometry_override: None,
                columns: make_cols(171.852, 2.719, 0.417),
            },
            HeatLossSheet {
                name: "Roof".into(),
                geometry: Geometry::Wall {
                    width_m: cap_area,
                    length_m: 1.0,
                },
                mode: CalculationMode::HeatFlow,
                fixed_heat_flow_w: 180.0,
                convection_geometry_override: None,
                columns: make_cols(171.852, 3.20, 0.417),
            },
            HeatLossSheet {
                name: "Bottom".into(),
                geometry: Geometry::Wall {
                    width_m: cap_area,
                    length_m: 1.0,
                },
                mode: CalculationMode::HeatFlow,
                fixed_heat_flow_w: 100.0,
                convection_geometry_override: None,
                columns: make_cols(171.852, 1.40, 0.417),
            },
        ],
    }
}

/// Example 5: `OilFurnace` Gas Radiation Composition (Section 12.2).
pub fn preset_oil_furnace_gas() -> GasComposition {
    GasComposition {
        x_h2o: 0.1108,
        x_co2: 0.1169,
        x_co: 0.0,
        x_no: 0.0,
        x_so2: 0.0005,
        x_ch4: 0.0,
    }
}
