//! Module 3: 1D Steady-State Heat Loss & Heat Transfer Calculations.
//!
//! Implements conduction (Eqs. 8-12), free & forced convection correlations (Eqs. 13-40),
//! surface & enclosure radiation (Eqs. 41-43), gas mixture EWBM radiation (Eqs. 44-47),
//! combined gas + particle suspension radiation (Eqs. 48-53), coupled 1D thermal network
//! solver (`HeatFlow` and `TemperatureProfile`), `Target` root finder, and `Diagram` sweeps.

pub mod conduction;
pub mod convection;
pub mod diagram;
pub mod gas_radiation;
pub mod material_db;
pub mod model;
pub mod presets;
pub mod radiation;
pub mod solver;
pub mod target;

pub use diagram::{
    generate_optimum_insulation_diagram, generate_parameter_sweep, generate_temperature_profile,
    OptimumInsulationPoint, OptimumInsulationSpec, SweepParameter, SweepPoint,
    TemperatureProfileSeries,
};
pub use gas_radiation::{
    calculate_gas_particle_radiation, calculate_gas_radiation,
    evaluate_particle_suspension_detailed, generate_iterative_gas_radiation_table, GasComposition,
    GasRadiationInput, GasRadiationResult, OpticalDepthGeometry, ParticleCloudInput,
    ParticleSuspensionDetailedResult, ParticleSuspensionSpec, SpectralBandRow,
};
pub use material_db::{
    MaterialCompositionRow, MaterialCpRange, MaterialDatabase, MaterialPropertyRow, MaterialRecord,
    MaterialUserProperty, ResolvedElementWt, ResolvedSpeciesWt,
};
pub use model::{
    apply_geometry_transition, build_heat_loss_export_sheets, deserialize_htr8_workbook,
    serialize_htr8_workbook, CalculationMode, ColumnResult, ColumnType, ConvectionFunctionPreset,
    ConvectionFunctionTerm, ConvectionGeometry, ConvectionRegime, FluidPropertiesPoint,
    FluidPropertyTable, Geometry, HeatLossColumn, HeatLossSheet, HeatLossWorkbook, SheetResult,
    SolidConductivityData, SurfaceEmissivityData, WorkbookResult,
};
pub use solver::{heat_flow_w_to_balance_energy_per_hour, solve_sheet, solve_workbook};
pub use target::{solve_target, TargetGoal, TargetResult, TargetSpec, TargetVariable};
