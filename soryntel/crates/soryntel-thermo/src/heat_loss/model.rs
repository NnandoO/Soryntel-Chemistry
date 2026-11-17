//! Domain models for 1D steady-state heat transfer networks.

use serde::{Deserialize, Serialize};
use soryntel_core::{HeatFlowUnit, LengthUnit, SoryntelError, TemperatureUnit};
use std::f64::consts::PI;

/// Equipment inner geometry (all dimensions correspond to the inner surface).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    Wall {
        width_m: f64,
        length_m: f64,
    },
    Cube {
        width_m: f64,
        length_m: f64,
        height_m: f64,
    },
    Cylinder {
        height_m: f64,
        diameter_m: f64,
    },
    Sphere {
        diameter_m: f64,
    },
}

impl Default for Geometry {
    fn default() -> Self {
        Self::Wall {
            width_m: 1.0,
            length_m: 1.0,
        }
    }
}

impl Geometry {
    /// Inner surface area at cumulative thickness `d_m = 0`, or surface area at cumulative
    /// radial/normal thickness `d_m >= 0`.
    pub fn surface_area_at_thickness(&self, d_m: f64) -> Result<f64, SoryntelError> {
        let d = d_m.max(0.0);
        let area = match *self {
            Geometry::Wall { width_m, length_m } => {
                if width_m <= 0.0 || length_m <= 0.0 {
                    return Err(SoryntelError::HeatLossError(
                        "Wall width and length must be > 0".into(),
                    ));
                }
                width_m * length_m
            }
            Geometry::Cube {
                width_m,
                length_m,
                height_m,
            } => {
                if width_m <= 0.0 || length_m <= 0.0 || height_m <= 0.0 {
                    return Err(SoryntelError::HeatLossError(
                        "Cube dimensions must be > 0".into(),
                    ));
                }
                let w = width_m + 2.0 * d;
                let l = length_m + 2.0 * d;
                let h = height_m + 2.0 * d;
                2.0 * (w * l + w * h + l * h)
            }
            Geometry::Cylinder {
                height_m,
                diameter_m,
            } => {
                if height_m <= 0.0 || diameter_m <= 0.0 {
                    return Err(SoryntelError::HeatLossError(
                        "Cylinder height and diameter must be > 0".into(),
                    ));
                }
                let d_out = diameter_m + 2.0 * d;
                let h_out = height_m + 2.0 * d;
                PI * d_out * h_out
            }
            Geometry::Sphere { diameter_m } => {
                if diameter_m <= 0.0 {
                    return Err(SoryntelError::HeatLossError(
                        "Sphere diameter must be > 0".into(),
                    ));
                }
                let d_out = diameter_m + 2.0 * d;
                PI * d_out * d_out
            }
        };
        Ok(area)
    }

    /// Conduction shape factor S (Eqs. 10-12) between cumulative thickness `d_in` and `d_out`.
    /// Thermal resistance of sublayer is `R = 1.0 / (S * k)`.
    pub fn conduction_shape_factor(&self, d_in: f64, d_out: f64) -> Result<f64, SoryntelError> {
        let dx = d_out - d_in;
        if dx <= 0.0 {
            return Err(SoryntelError::HeatLossError(
                "Layer thickness must be > 0".into(),
            ));
        }
        match *self {
            Geometry::Wall { width_m, length_m } => Ok((width_m * length_m) / dx),
            Geometry::Cube { .. } => {
                let a_in = self.surface_area_at_thickness(d_in)?;
                let a_out = self.surface_area_at_thickness(d_out)?;
                // Geometric mean area for thick cube walls (matches 1D spherical/planar limit)
                let a_mean = (a_in * a_out).sqrt();
                Ok(a_mean / dx)
            }
            Geometry::Cylinder {
                height_m,
                diameter_m,
            } => {
                let r_i = 0.5 * diameter_m + d_in;
                let r_o = 0.5 * diameter_m + d_out;
                Ok((2.0 * PI * height_m) / (r_o / r_i).ln())
            }
            Geometry::Sphere { diameter_m } => {
                let r_i = 0.5 * diameter_m + d_in;
                let r_o = 0.5 * diameter_m + d_out;
                Ok((4.0 * PI * r_i * r_o) / (r_o - r_i))
            }
        }
    }

    /// Characteristic length L (m) at cumulative thickness `d_m` for convection correlations.
    pub fn characteristic_length(&self, d_m: f64) -> f64 {
        let d = d_m.max(0.0);
        match *self {
            Geometry::Wall { width_m: _, length_m } => length_m,
            Geometry::Cube {
                width_m,
                length_m,
                height_m,
            } => {
                let _ = (width_m, length_m);
                height_m + 2.0 * d
            }
            Geometry::Cylinder {
                height_m,
                diameter_m,
            } => {
                let _ = diameter_m;
                height_m
            }
            Geometry::Sphere { diameter_m } => diameter_m + 2.0 * d,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Geometry::Wall { .. } => "Wall",
            Geometry::Cube { .. } => "Cube",
            Geometry::Cylinder { .. } => "Cylinder",
            Geometry::Sphere { .. } => "Sphere",
        }
    }
}

/// Column type in the 1D heat transfer spreadsheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnType {
    Surface,
    SolidLayer,
    Enclosure,
    LayerContact,
}

impl ColumnType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Surface => "Superficie",
            Self::SolidLayer => "Capa sólida",
            Self::Enclosure => "Cavidad",
            Self::LayerContact => "Contacto entre capas",
        }
    }
}

/// Solver mode for a sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CalculationMode {
    /// Two boundary/node temperatures are fixed; solve heat flow Q and intermediate temperatures.
    #[default]
    HeatFlow,
    /// One temperature is fixed and total sheet heat flow Q is fixed; solve temperature profile.
    TemperatureProfile,
}

/// Convection regime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConvectionRegime {
    #[default]
    Free,
    Forced,
}

/// Convection correlation geometry/orientation (Section 12.4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ConvectionGeometry {
    #[default]
    VerticalWall,
    HorizontalPlateHotUp,
    HorizontalPlateHotDown,
    Cube,
    VerticalCylinder,
    HorizontalCylinder,
    Sphere,
    VerticalEnclosure,
    HorizontalEnclosure,
    ConcentricCylinders,
    ConcentricSpheres,
}

impl ConvectionGeometry {
    pub fn label(self) -> &'static str {
        match self {
            Self::VerticalWall => "Pared o placa vertical",
            Self::HorizontalPlateHotUp => "Placa horizontal (caliente arriba / fría abajo)",
            Self::HorizontalPlateHotDown => "Placa horizontal (caliente abajo / fría arriba)",
            Self::Cube => "Cubo",
            Self::VerticalCylinder => "Cilindro vertical",
            Self::HorizontalCylinder => "Cilindro horizontal",
            Self::Sphere => "Esfera",
            Self::VerticalEnclosure => "Cavidad vertical",
            Self::HorizontalEnclosure => "Cavidad horizontal",
            Self::ConcentricCylinders => "Cilindros concéntricos",
            Self::ConcentricSpheres => "Esferas concéntricas",
        }
    }
}

/// Tabulated thermal conductivity k(T) for a solid material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolidConductivityData {
    pub name: String,
    pub category: String,
    pub density_kg_m3: Option<f64>,
    /// Pairs of `(T_K, k [W/(m·K)])` sorted ascending by temperature.
    pub k_table: Vec<(f64, f64)>,
    pub reference: String,
}

impl SolidConductivityData {
    /// Linearly interpolate `k(T)` with clamping outside `[T_min, T_max]`.
    pub fn conductivity_at(&self, t_k: f64) -> Result<f64, SoryntelError> {
        interpolate_sorted_table(&self.k_table, t_k, &self.name)
    }

    pub fn t_range_k(&self) -> Option<(f64, f64)> {
        Some((self.k_table.first()?.0, self.k_table.last()?.0))
    }
}

/// Single temperature point of fluid thermophysical properties.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FluidPropertiesPoint {
    pub t_k: f64,
    pub density_kg_m3: f64,
    pub cp_j_kg_k: f64,
    pub k_w_m_k: f64,
    pub mu_pa_s: f64,
    /// Optional volumetric thermal expansion coefficient beta [1/K]. If None, uses 1/T_K (ideal gas).
    pub beta_1_k: Option<f64>,
}

impl FluidPropertiesPoint {
    pub fn nu_m2_s(&self) -> f64 {
        self.mu_pa_s / self.density_kg_m3
    }

    pub fn alpha_m2_s(&self) -> f64 {
        self.k_w_m_k / (self.cp_j_kg_k * self.density_kg_m3)
    }

    pub fn prandtl(&self) -> f64 {
        (self.cp_j_kg_k * self.mu_pa_s) / self.k_w_m_k
    }

    pub fn beta_or_ideal(&self, t_k: f64) -> f64 {
        self.beta_1_k.unwrap_or_else(|| 1.0 / t_k.max(1.0))
    }
}

/// Tabulated fluid properties for convection calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FluidPropertyTable {
    pub name: String,
    pub is_gas: bool,
    pub points: Vec<FluidPropertiesPoint>,
    pub reference: String,
}

impl FluidPropertyTable {
    pub fn interpolate(&self, t_k: f64) -> Result<FluidPropertiesPoint, SoryntelError> {
        if self.points.is_empty() {
            return Err(SoryntelError::HeatLossError(format!(
                "Empty fluid property table for {}",
                self.name
            )));
        }
        if t_k <= self.points[0].t_k {
            return Ok(self.points[0]);
        }
        let last = self.points.len() - 1;
        if t_k >= self.points[last].t_k {
            return Ok(self.points[last]);
        }
        for w in self.points.windows(2) {
            let (p0, p1) = (w[0], w[1]);
            if t_k >= p0.t_k && t_k <= p1.t_k {
                let f = (t_k - p0.t_k) / (p1.t_k - p0.t_k);
                let lerp = |a: f64, b: f64| a + f * (b - a);
                let beta = match (p0.beta_1_k, p1.beta_1_k) {
                    (Some(b0), Some(b1)) => Some(lerp(b0, b1)),
                    _ => None,
                };
                return Ok(FluidPropertiesPoint {
                    t_k,
                    density_kg_m3: lerp(p0.density_kg_m3, p1.density_kg_m3),
                    cp_j_kg_k: lerp(p0.cp_j_kg_k, p1.cp_j_kg_k),
                    k_w_m_k: lerp(p0.k_w_m_k, p1.k_w_m_k),
                    mu_pa_s: lerp(p0.mu_pa_s, p1.mu_pa_s),
                    beta_1_k: beta,
                });
            }
        }
        Ok(self.points[last])
    }
}

/// Term `coeff * v^exponent` for custom `hc(v)` convection function tab.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConvectionFunctionTerm {
    pub coeff: f64,
    pub exponent: f64,
}

/// Named preset row in the Convection Database `Function` sheet (Ch. 12.3.5, Fig. 7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvectionFunctionPreset {
    pub name: String,
    pub subtitle: String,
    pub terms: Vec<ConvectionFunctionTerm>,
}

/// Surface emissivity record (normal `(n)` or hemispherical `(h)`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfaceEmissivityData {
    pub name: String,
    pub emissivity: f64,
    pub measurement_type: String, // "(h)" or "(n)"
    pub temp_range_desc: String,
}

/// One column in a heat loss sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatLossColumn {
    pub column_type: ColumnType,
    pub layer_material: String,
    pub surface_material: String,
    /// Left edge temperature in Kelvin.
    pub t_left_k: f64,
    pub t_left_fixed: bool,
    /// Right edge temperature in Kelvin.
    pub t_right_k: f64,
    pub t_right_fixed: bool,
    /// Thickness in meters (for SolidLayer and Enclosure).
    pub thickness_m: f64,
    /// Calculation grid subdivisions (default 10 for SolidLayer).
    pub calculation_grid: usize,
    /// Thermal conductivity W/(m·K).
    pub k_w_m_k: f64,
    pub k_fixed: bool,
    pub solid_data: Option<SolidConductivityData>,
    /// Convection coefficient W/(m^2·K).
    pub hc_w_m2_k: f64,
    pub hc_fixed: bool,
    pub convection_regime: ConvectionRegime,
    pub convection_geometry: ConvectionGeometry,
    pub fluid_speed_m_s: f64,
    pub characteristic_length_override_m: Option<f64>,
    pub fluid_data: Option<FluidPropertyTable>,
    pub custom_hc_terms: Vec<ConvectionFunctionTerm>,
    /// Radiation coefficient W/(m^2·K).
    pub hr_w_m2_k: f64,
    pub hr_fixed: bool,
    pub emissivity: f64,
    /// Optional second surface emissivity for Enclosure.
    pub emissivity_opposite: f64,
    /// Thermal contact resistance in K/W (used when ColumnType::LayerContact).
    pub contact_resistance_k_w: f64,
    /// Detailed row 61: Gas emissivity (Ch. 12.1 step 4 / Ch. 12.3.7).
    #[serde(default)]
    pub gas_emissivity: Option<f64>,
    /// Detailed row 62: Gas absorptivity (Ch. 12.1 step 4 / Ch. 12.3.7).
    #[serde(default)]
    pub gas_absorptivity: Option<f64>,
    /// Detailed row 69: Particle cloud emissivity (Ch. 12.1 step 4 / Ch. 12.3.8).
    #[serde(default)]
    pub particle_cloud_emissivity: Option<f64>,
    /// Iterative 0..3000 °C (step 50 °C) gas emissivity table from `Select iterative`.
    #[serde(default)]
    pub iterative_gas_eps_table: Vec<(f64, f64)>,
    /// Detailed row 92: Min valid material temperature in K (Ch. 12.3.2 item 4).
    #[serde(default)]
    pub t_min_k: Option<f64>,
    /// Detailed row 93: Max valid material temperature in K (Ch. 12.3.2 item 4).
    #[serde(default)]
    pub t_max_k: Option<f64>,
}

impl Default for HeatLossColumn {
    fn default() -> Self {
        Self {
            column_type: ColumnType::SolidLayer,
            layer_material: "Custom Solid".into(),
            surface_material: String::new(),
            t_left_k: 373.15,
            t_left_fixed: false,
            t_right_k: 293.15,
            t_right_fixed: false,
            thickness_m: 0.1,
            calculation_grid: 10,
            k_w_m_k: 1.0,
            k_fixed: true,
            solid_data: None,
            hc_w_m2_k: 10.0,
            hc_fixed: true,
            convection_regime: ConvectionRegime::Free,
            convection_geometry: ConvectionGeometry::VerticalWall,
            fluid_speed_m_s: 0.0,
            characteristic_length_override_m: None,
            fluid_data: None,
            custom_hc_terms: Vec::new(),
            hr_w_m2_k: 0.0,
            hr_fixed: true,
            emissivity: 0.9,
            emissivity_opposite: 0.9,
            contact_resistance_k_w: 0.001,
            gas_emissivity: None,
            gas_absorptivity: None,
            particle_cloud_emissivity: None,
            iterative_gas_eps_table: Vec::new(),
            t_min_k: None,
            t_max_k: None,
        }
    }
}

/// One sheet (e.g., Wall, Roof, Bottom) in a Heat Loss workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatLossSheet {
    pub name: String,
    pub geometry: Geometry,
    pub mode: CalculationMode,
    /// Used in `TemperatureProfile` mode as the fixed sheet heat flow in Watts.
    pub fixed_heat_flow_w: f64,
    pub columns: Vec<HeatLossColumn>,
    pub convection_geometry_override: Option<ConvectionGeometry>,
}

/// Complete multi-sheet Heat Loss workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatLossWorkbook {
    pub title: String,
    pub temp_unit: TemperatureUnit,
    pub heat_flow_unit: HeatFlowUnit,
    pub length_unit: LengthUnit,
    pub calculate_all_sheets: bool,
    pub active_sheet: usize,
    pub sheets: Vec<HeatLossSheet>,
}

impl Default for HeatLossWorkbook {
    fn default() -> Self {
        Self {
            title: "Untitled.htr".into(),
            temp_unit: TemperatureUnit::Celsius,
            heat_flow_unit: HeatFlowUnit::Kilowatt,
            length_unit: LengthUnit::Meter,
            calculate_all_sheets: true,
            active_sheet: 0,
            sheets: Vec::new(),
        }
    }
}

/// Calculated output for one column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnResult {
    pub column_index: usize,
    pub t_left_k: f64,
    pub t_right_k: f64,
    pub k_avg_w_m_k: f64,
    pub hc_w_m2_k: f64,
    pub hr_w_m2_k: f64,
    pub thermal_resistance_k_w: f64,
    pub heat_flow_w: f64,
    pub heat_flux_w_m2: f64,
    pub surface_area_m2: f64,
    pub cumulative_thickness_m: f64,
    /// Discretized temperature profile points `(cumulative_x_m, T_K)` inside this column.
    pub profile_points: Vec<(f64, f64)>,
}

/// Calculated output for one sheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetResult {
    pub sheet_name: String,
    pub heat_flow_w: f64,
    pub iterations: usize,
    pub converged: bool,
    pub columns: Vec<ColumnResult>,
    pub warnings: Vec<String>,
}

/// Calculated output for the entire workbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookResult {
    pub total_heat_flow_w: f64,
    pub sheet_results: Vec<SheetResult>,
}

/// Piecewise linear interpolation over a sorted `(x, y)` table.
pub fn interpolate_sorted_table(
    table: &[(f64, f64)],
    x: f64,
    label: &str,
) -> Result<f64, SoryntelError> {
    if table.is_empty() || !x.is_finite()
        || table.iter().any(|(t, value)| !t.is_finite() || !value.is_finite())
        || table.windows(2).any(|w| w[0].0 >= w[1].0)
    {
        return Err(SoryntelError::HeatLossError(format!(
            "Tabla no válida para {label}: se requieren valores finitos y abscisas estrictamente crecientes"
        )));
    }
    if x <= table[0].0 {
        return Ok(table[0].1);
    }
    let last = table.len() - 1;
    if x >= table[last].0 {
        return Ok(table[last].1);
    }
    for w in table.windows(2) {
        if x >= w[0].0 && x <= w[1].0 {
            let dx = w[1].0 - w[0].0;
            if dx.abs() < 1e-15 {
                return Ok(w[0].1);
            }
            let frac = (x - w[0].0) / dx;
            return Ok(w[0].1 + frac * (w[1].1 - w[0].1));
        }
    }
    Ok(table[last].1)
}

/// Apply geometry transition rules (Ch. 12.3.2 item 6):
/// When changing from a single-sheet `Wall` or `Sphere` to `Cube` or `Cylinder`,
/// automatically copy the wall layout to the `Roof` and `Bottom` sheets if only 1 sheet exists.
pub fn apply_geometry_transition(wb: &mut HeatLossWorkbook, sheet_idx: usize, new_geom: Geometry) {
    if sheet_idx >= wb.sheets.len() {
        return;
    }
    wb.sheets[sheet_idx].geometry = new_geom;
    if wb.sheets.len() == 1 {
        match new_geom {
            Geometry::Cylinder {
                height_m: _,
                diameter_m,
            } => {
                wb.sheets[0].name = "Wall".into();
                let r = 0.5 * diameter_m;
                let cap_area = PI * r * r;
                let mut roof = wb.sheets[0].clone();
                roof.name = "Roof".into();
                roof.geometry = Geometry::Wall {
                    width_m: cap_area,
                    length_m: 1.0,
                };
                let mut bottom = roof.clone();
                bottom.name = "Bottom".into();
                wb.sheets.push(roof);
                wb.sheets.push(bottom);
                wb.calculate_all_sheets = true;
            }
            Geometry::Cube {
                width_m,
                length_m,
                height_m,
            } => {
                wb.sheets[0].name = "Wall".into();
                wb.sheets[0].geometry = Geometry::Wall {
                    width_m: 2.0 * (width_m + length_m),
                    length_m: height_m,
                };
                let mut roof = wb.sheets[0].clone();
                roof.name = "Roof".into();
                roof.geometry = Geometry::Wall {
                    width_m,
                    length_m,
                };
                let mut bottom = roof.clone();
                bottom.name = "Bottom".into();
                wb.sheets.push(roof);
                wb.sheets.push(bottom);
                wb.calculate_all_sheets = true;
            }
            _ => {}
        }
    }
}

/// Serialize a `HeatLossWorkbook` to `.htr8` JSON format.
pub fn serialize_htr8_workbook(wb: &HeatLossWorkbook) -> Result<String, String> {
    serde_json::to_string_pretty(wb).map_err(|e| format!("Failed to serialize .htr8: {e}"))
}

/// Deserialize a `HeatLossWorkbook` from `.htr8` JSON format.
pub fn deserialize_htr8_workbook(raw: &str) -> Result<HeatLossWorkbook, String> {
    serde_json::from_str(raw).map_err(|e| format!("Failed to parse .htr8: {e}"))
}

/// Build exportable 2D string grids (`ExportSheet`) for `.xlsx` / `.tsv` export.
pub fn build_heat_loss_export_sheets(
    wb: &HeatLossWorkbook,
    res: Option<&WorkbookResult>,
) -> Vec<crate::spreadsheet_export::ExportSheet> {
    let mut out = Vec::new();
    let t_u = wb.temp_unit;
    let q_u = wb.heat_flow_unit;
    let l_u = wb.length_unit;

    for (s_idx, sheet) in wb.sheets.iter().enumerate() {
        let sr = res.and_then(|r| {
            r.sheet_results
                .iter()
                .find(|x| x.sheet_name == sheet.name)
                .or_else(|| r.sheet_results.get(s_idx))
        });

        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut hdr = vec!["Property".to_string(), "Unit".to_string()];
        for i in 0..sheet.columns.len() {
            hdr.push(format!("Layer {}", i + 1));
        }
        rows.push(hdr);

        let mut r_type = vec!["Layer Type".to_string(), String::new()];
        let mut r_mat = vec!["Layer Material".to_string(), String::new()];
        let mut r_surf = vec!["Surface Material".to_string(), String::new()];
        let mut r_tl = vec!["Temperature (left edge)".to_string(), t_u.label().to_string()];
        let mut r_tr = vec!["Temperature (right edge)".to_string(), t_u.label().to_string()];
        let mut r_th = vec!["Thickness".to_string(), l_u.label().to_string()];
        let mut r_gr = vec!["Calculation Grid".to_string(), String::new()];
        let mut r_k = vec!["Average conductivity (k)".to_string(), "W/(mK)".to_string()];
        let mut r_hc = vec!["Convection coefficient (hc)".to_string(), "W/(m2K)".to_string()];
        let mut r_hr = vec!["Radiation coefficient (hr)".to_string(), "W/(m2K)".to_string()];
        let mut r_res = vec!["Thermal Resistance".to_string(), "°C/kW".to_string()];
        let mut r_qf = vec!["Heat Flow".to_string(), q_u.label().to_string()];
        let mut r_qflx = vec!["Heat Flux".to_string(), q_u.flux_label().to_string()];
        let mut r_area = vec!["Surface Area".to_string(), "m2".to_string()];
        let mut r_cum = vec!["Cumulative Thickness".to_string(), l_u.label().to_string()];

        for (i, col) in sheet.columns.iter().enumerate() {
            let cr = sr.and_then(|s| s.columns.get(i));
            r_type.push(col.column_type.label().to_string());
            r_mat.push(col.layer_material.clone());
            r_surf.push(col.surface_material.clone());
            let tl = cr.map(|c| c.t_left_k).unwrap_or(col.t_left_k);
            let tr = cr.map(|c| c.t_right_k).unwrap_or(col.t_right_k);
            r_tl.push(format!("{:.2}", t_u.from_kelvin(tl)));
            r_tr.push(format!("{:.2}", t_u.from_kelvin(tr)));
            r_th.push(format!("{:.3}", col.thickness_m * l_u.from_meters_factor()));
            r_gr.push(format!("{}", col.calculation_grid));
            r_k.push(format!("{:.3}", cr.map(|c| c.k_avg_w_m_k).unwrap_or(col.k_w_m_k)));
            r_hc.push(format!("{:.3}", cr.map(|c| c.hc_w_m2_k).unwrap_or(col.hc_w_m2_k)));
            r_hr.push(format!("{:.3}", cr.map(|c| c.hr_w_m2_k).unwrap_or(col.hr_w_m2_k)));
            r_res.push(format!(
                "{:.3}",
                cr.map(|c| c.thermal_resistance_k_w * 1000.0).unwrap_or(0.0)
            ));
            r_qf.push(format!(
                "{:.3}",
                cr.map(|c| c.heat_flow_w * q_u.from_watts_factor()).unwrap_or(0.0)
            ));
            r_qflx.push(format!(
                "{:.3}",
                cr.map(|c| c.heat_flux_w_m2 * q_u.from_watts_factor()).unwrap_or(0.0)
            ));
            r_area.push(format!("{:.3}", cr.map(|c| c.surface_area_m2).unwrap_or(0.0)));
            r_cum.push(format!(
                "{:.3}",
                cr.map(|c| c.cumulative_thickness_m * l_u.from_meters_factor())
                    .unwrap_or(0.0)
            ));
        }

        rows.extend([
            r_type, r_mat, r_surf, r_tl, r_tr, r_th, r_gr, r_k, r_hc, r_hr, r_res, r_qf, r_qflx,
            r_area, r_cum,
        ]);
        let mut cells = Vec::new();
        for (r_i, row_vec) in rows.iter().enumerate() {
            let style = if r_i == 0 {
                crate::spreadsheet_export::ExportCellStyle::Header
            } else {
                crate::spreadsheet_export::ExportCellStyle::Default
            };
            for (c_i, val_str) in row_vec.iter().enumerate() {
                let value = if c_i >= 2 && r_i >= 4 {
                    if let Ok(num) = val_str.parse::<f64>() {
                        crate::spreadsheet_export::ExportCellValue::Number(num)
                    } else {
                        crate::spreadsheet_export::ExportCellValue::Text(val_str.clone())
                    }
                } else {
                    crate::spreadsheet_export::ExportCellValue::Text(val_str.clone())
                };
                cells.push(crate::spreadsheet_export::ExportCell {
                    row: r_i,
                    col: c_i,
                    value,
                    style,
                });
            }
        }
        out.push(crate::spreadsheet_export::ExportSheet {
            name: sheet.name.clone(),
            cells,
            col_widths: vec![18.0; sheet.columns.len() + 2],
        });
    }
    out
}
