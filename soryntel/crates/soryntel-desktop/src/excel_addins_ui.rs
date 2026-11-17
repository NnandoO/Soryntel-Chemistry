//! UI del Módulo 15: HSC Excel Add-Ins (Modo Híbrido Standalone + Integración Cross-Suite).
//!
//! Clona fielmente la interfaz y flujo de trabajo de HSC Chemistry Excel Add-Ins (Cap. 30 Fig. 1–7
//! + Changing measurement units):
//! - **100% autónomo dentro de Soryntel** (zero dependencias externas para calcular las 83 funciones
//!   activas + 5 HSC-Sim en `Worksheet`, `StreamEQ`, `Minerals`, `AddInSample` y `Functions (fx)`).
//! - **Integración Cross-Suite opcional** mediante HTTP Bridge local (`127.0.0.1:8765`, CORS) y
//!   exportadores nativos para Microsoft Excel, LibreOffice Calc (`.ods`/`.oxt`), ONLYOFFICE (`.zip`)
//!   y WPS Office (`.bas` / `.xlsx` / `=WEBSERVICE(...)`).

use crate::addins_bridge::BridgeHandle;
use crate::excel_grid::{
    col_letter, header_cell, row_num_cell, text_cell, widget_cell, CELL_H, EXCEL_ALT_ROW_BG,
    EXCEL_HEADER_BG, EXCEL_SECTION_BG, EXCEL_YELLOW_BG,
};
use eframe::egui;
use soryntel_application::SoryntelApplication;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::excel_addins::{
    format_cell_number, format_temp_unit_display, parse_a1, AddInFunctionMeta, AddInOrigin,
    AddInUnits, ArgValue, CellEvalValue, MineralRecord, ADDIN_FUNCTION_CATALOG,
};
use soryntel_thermo::spreadsheet_export::{
    col_to_a1, ExportCell, ExportCellStyle, ExportCellValue, ExportSheet,
};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

const EXCEL_GREEN: egui::Color32 = egui::Color32::from_rgb(33, 115, 70);
const EXCEL_GREEN_DARK: egui::Color32 = egui::Color32::from_rgb(24, 90, 54);
const EXCEL_RIBBON_BG: egui::Color32 = egui::Color32::from_rgb(245, 246, 248);
const EXCEL_BORDER: egui::Color32 = egui::Color32::from_rgb(209, 213, 219);
const TEXT_DARK: egui::Color32 = egui::Color32::from_rgb(24, 24, 27);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(90, 96, 106);
const TEXT_BLUE: egui::Color32 = egui::Color32::from_rgb(29, 78, 216);
const TEXT_RED: egui::Color32 = egui::Color32::from_rgb(185, 28, 28);
const TEXT_ERR: egui::Color32 = egui::Color32::from_rgb(180, 30, 30);

// Paleta auténtica HSC Minerals Fig. 7 & StreamEQ Fig. 4-6
const FIG7_NAVY_HEADER: egui::Color32 = egui::Color32::from_rgb(31, 78, 120);
const FIG7_OLIVE: egui::Color32 = egui::Color32::from_rgb(122, 116, 0);
const FIG7_GREEN: egui::Color32 = egui::Color32::from_rgb(38, 120, 48);
const FIG7_TEAL: egui::Color32 = egui::Color32::from_rgb(0, 118, 138);
const FIG7_SILVER_BG: egui::Color32 = egui::Color32::from_rgb(212, 215, 220);
const FIG7_SKY_BG: egui::Color32 = egui::Color32::from_rgb(164, 212, 232);
const REF_BLUE: egui::Color32 = egui::Color32::from_rgb(37, 99, 235);
const REF_GREEN: egui::Color32 = egui::Color32::from_rgb(22, 163, 74);
const REF_PURPLE: egui::Color32 = egui::Color32::from_rgb(147, 51, 234);

const WS_ROWS: usize = 24;
const WS_COLS: usize = 8;

const TEMP_UNITS: &[&str] = &["C", "K", "F"];
const ENERGY_UNITS: &[&str] = &["kJ", "MJ", "kcal", "Mcal", "kWh", "J"];
const PRESS_UNITS: &[&str] = &["bar", "atm", "Pa", "kPa", "MPa", "mmHg"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddInRibbonTab {
    File,
    Home,
    Insert,
    PageLayout,
    Formulas,
    AddIns,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddInSheetTab {
    Worksheet,
    StreamEq,
    Minerals,
    AddInSample,
    FunctionsCatalog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileBackstageSection {
    Info,
    OpenSamples,
    ExportReport,
    Options,
    Migrator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionsCategory {
    General,
    Formulas,
    Proofing,
    Save,
    Language,
    Advanced,
    CustomizeRibbon,
    QuickAccessToolbar,
    AddIns,
    TrustCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAddInTarget {
    ExcelAddIns,
    LibreOfficeOxt,
    OnlyOfficeZip,
    VbaModule,
}

impl ManageAddInTarget {
    fn label(self) -> &'static str {
        match self {
            Self::ExcelAddIns => "Excel Add-ins (.xll)",
            Self::LibreOfficeOxt => "LibreOffice Calc (.oxt)",
            Self::OnlyOfficeZip => "ONLYOFFICE Plugin (.zip)",
            Self::VbaModule => "Excel / WPS VBA (.bas)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddInPackKind {
    Hsc6Core,
    Hsc10UnitQuery,
    Mineralogy,
    HscSimRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgSeparator {
    Semicolon,
    Comma,
}

impl ArgSeparator {
    fn ch(self) -> char {
        match self {
            Self::Semicolon => ';',
            Self::Comma => ',',
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Semicolon => "Semicolon ( ; )  [EU / HSC manual]",
            Self::Comma => "Comma ( , )  [US Excel]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct InternalAddInPack {
    pub kind: AddInPackKind,
    pub name: &'static str,
    pub module_ref: &'static str,
    pub pack_type: &'static str,
    pub description: &'static str,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct SheetCell {
    pub raw: String,
    pub eval: CellEvalValue,
    pub style: ExportCellStyle,
    pub is_formula: bool,
}

fn blank_cell_val() -> CellEvalValue {
    CellEvalValue::Text(String::new())
}

fn is_blank_cell_val(ev: &CellEvalValue) -> bool {
    matches!(ev, CellEvalValue::Text(s) if s.is_empty())
}

impl Default for SheetCell {
    fn default() -> Self {
        Self {
            raw: String::new(),
            eval: blank_cell_val(),
            style: ExportCellStyle::Default,
            is_formula: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PickerTarget {
    WorksheetActiveCell,
    StreamEqAppendRow,
}

#[derive(Debug, Clone)]
struct SampleScratchRow {
    formula: String,
    result: CellEvalValue,
}

pub struct ExcelAddInsModuleState {
    // Ribbon, sheets & backstage navigation
    pub ribbon_tab: AddInRibbonTab,
    pub active_sheet: AddInSheetTab,
    pub show_file_backstage: bool,
    pub file_section: FileBackstageSection,
    pub options_cat: OptionsCategory,

    // Global preferences & units + modal "Change measurement units..."
    pub units: AddInUnits,
    shared_units: Arc<Mutex<AddInUnits>>,
    show_units_dialog: bool,
    pending_units: AddInUnits,
    pub decimals: usize,
    pub show_formulas: bool,
    arg_separator: ArgSeparator,
    pub packs: Vec<InternalAddInPack>,
    selected_pack_idx: usize,
    manage_target: ManageAddInTarget,
    show_addins_go_dialog: bool,
    show_browse_dialog: bool,
    browse_selected_file: usize,

    // Shared formula bar
    pub active_cell_ref: String,
    pub formula_bar_text: String,
    pub status_msg: String,
    initialized_db_sheets: bool,

    // Sheet 1: Free Editable Worksheet (24x8 = A1:H24)
    ws_raw: Vec<Vec<String>>,
    ws_eval: Vec<Vec<CellEvalValue>>,
    ws_array_label: Vec<Vec<Option<String>>>,
    ws_editing: Option<(usize, usize)>,
    ws_focus_inline: bool,

    // Sheet 2: StreamEQ (Chapter 30 Fig. 4-6)
    pub eq_temp_raw: String,
    pub eq_press_raw: String,
    pub eq_species: Vec<String>,
    pub eq_inputs: Vec<String>,
    pub eq_outputs: Vec<CellEvalValue>,
    pub eq_balance: Vec<(String, f64, f64)>,
    pub eq_array_committed: bool,
    pub eq_formula_editing: bool,
    pub eq_show_range_highlights: bool,
    eq_selected_row: Option<usize>,

    // Sheet 3: Minerals (Chapter 30 Fig. 7) + inline editable red cells
    min_cells: BTreeMap<(usize, usize), SheetCell>,
    min_max_row: usize,
    min_max_col: usize,
    min_editing: Option<(usize, usize)>,
    min_edit_buf: String,
    min_focus_inline: bool,
    pub min_paste_values: bool,
    pub min_search_query: String,
    pub min_selected: Option<MineralRecord>,

    // Sheet 4: AddInSample + inline editable cells
    sample_cells: BTreeMap<(usize, usize), SheetCell>,
    sample_max_row: usize,
    sample_max_col: usize,
    sample_editing: Option<(usize, usize)>,
    sample_edit_buf: String,
    sample_focus_inline: bool,
    scratch_rows: Vec<SampleScratchRow>,

    // Sheet 5: Functions (fx) Catalog & Live Tester
    pub fn_search: String,
    pub fn_group_filter: String,
    pub selected_fn_idx: usize,
    pub test_args: [String; 5],
    pub test_result: CellEvalValue,
    pub formula_preview: String,

    // Species / Mineral DB Picker modal
    show_species_picker: bool,
    picker_target: PickerTarget,
    picker_query: String,

    // Cross-Suite HTTP Bridge (Excel / LibreOffice / ONLYOFFICE / WPS) & Export
    pub bridge_port: u16,
    bridge_handle: Option<BridgeHandle>,
    bridge_error: String,
    pub webservice_fn: String,
    pub webservice_a: String,
    pub webservice_b: String,
    pub export_dir: String,
    pub export_filename: String,
    pub export_status: String,
    pub migrator_input: String,
    pub migrator_output: String,
    pub migrator_warnings: Vec<String>,
}

impl Default for ExcelAddInsModuleState {
    fn default() -> Self {
        let packs = vec![
            InternalAddInPack {
                kind: AddInPackKind::Hsc6Core,
                name: "HSC6 Core Thermochemical Add-In",
                module_ref: "Internal :: HSC6.XLL (69 functions)",
                pack_type: "Internal Thermochemical Engine",
                description: "H, S, Cp, G, Reaction*, TP/HT/ST/GT equilibrium, Percentage, Stream*, Flow*, StreamEQ.",
                enabled: true,
            },
            InternalAddInPack {
                kind: AddInPackKind::Hsc10UnitQuery,
                name: "HSC10 Unit Query & Humid Air",
                module_ref: "Internal :: HSC10-Excel.xll (3 functions)",
                pack_type: "Internal Unit & Density Add-In",
                description: "UNITS(), DensityA(), StreamH() dynamic unit configuration inside Soryntel.",
                enabled: true,
            },
            InternalAddInPack {
                kind: AddInPackKind::Mineralogy,
                name: "HSC Mineralogy Add-In",
                module_ref: "Internal :: Minerals.txt (11 functions, 3,812 records)",
                pack_type: "Internal Mineralogy Engine",
                description: "MineralCode, MineralName, MineralFormula, MineralGroup, MineralDensity, MineralC, etc.",
                enabled: true,
            },
            InternalAddInPack {
                kind: AddInPackKind::HscSimRef,
                name: "HSC-Sim Particle / Stream Functions",
                module_ref: "Internal :: HSC-Sim Cap. 27/57 (5 functions)",
                pack_type: "Flowsheet Function Pack",
                description: "StreamX, StreamMass, StreamVol, ParticleX, MineralX (con plantilla de ejemplo).",
                enabled: true,
            },
        ];

        let mut ws_raw = vec![vec![String::new(); WS_COLS]; WS_ROWS];
        populate_default_worksheet_demo(&mut ws_raw);

        let eq_species = vec![
            "§ PHASE 1:".into(),
            "CO(g)".into(),
            "CO2(g)".into(),
            "N(g)".into(),
            "N2(g)".into(),
            "NO(g)".into(),
            "N2O3(g)".into(),
            "N2O4(g)".into(),
            "N2O5(g)".into(),
            "O2(g)".into(),
            "§ PHASE 2:".into(),
            "C".into(),
        ];
        let eq_inputs = vec![
            "25".into(),
            "0".into(),
            "".into(),
            "".into(),
            "3.02".into(),
            "".into(),
            "".into(),
            "".into(),
            "".into(),
            "0.8".into(),
            "25".into(),
            "1".into(),
        ];

        let mut min_cells = BTreeMap::new();
        let (min_max_row, min_max_col) =
            load_export_sheet_into_cells(&SoryntelApplication::addin_minerals_sheet(), &mut min_cells);

        let mut state = Self {
            ribbon_tab: AddInRibbonTab::AddIns,
            active_sheet: AddInSheetTab::Worksheet,
            show_file_backstage: false,
            file_section: FileBackstageSection::Options,
            options_cat: OptionsCategory::AddIns,

            units: AddInUnits::default(),
            shared_units: Arc::new(Mutex::new(AddInUnits::default())),
            show_units_dialog: false,
            pending_units: AddInUnits::default(),
            decimals: 6,
            show_formulas: false,
            arg_separator: ArgSeparator::Semicolon,
            packs,
            selected_pack_idx: 0,
            manage_target: ManageAddInTarget::ExcelAddIns,
            show_addins_go_dialog: false,
            show_browse_dialog: false,
            browse_selected_file: 0,

            active_cell_ref: "C2".into(),
            formula_bar_text: ws_raw[1][2].clone(),
            status_msg: "Listo (Standalone). Doble clic en celda, Ctrl+Shift+Enter en StreamEQ, o activa Bridge HTTP para suites externas.".into(),
            initialized_db_sheets: false,

            ws_raw,
            ws_eval: vec![vec![blank_cell_val(); WS_COLS]; WS_ROWS],
            ws_array_label: vec![vec![None; WS_COLS]; WS_ROWS],
            ws_editing: None,
            ws_focus_inline: false,

            eq_temp_raw: "1500".into(),
            eq_press_raw: "1".into(),
            eq_species,
            eq_inputs,
            eq_outputs: Vec::new(),
            eq_balance: Vec::new(),
            eq_array_committed: true,
            eq_formula_editing: false,
            eq_show_range_highlights: true,
            eq_selected_row: None,

            min_cells,
            min_max_row,
            min_max_col,
            min_editing: None,
            min_edit_buf: String::new(),
            min_focus_inline: false,
            min_paste_values: false,
            min_search_query: "Py".into(),
            min_selected: SoryntelApplication::lookup_addin_mineral("Py"),

            sample_cells: BTreeMap::new(),
            sample_max_row: 0,
            sample_max_col: 0,
            sample_editing: None,
            sample_edit_buf: String::new(),
            sample_focus_inline: false,
            scratch_rows: vec![
                SampleScratchRow {
                    formula: "=H(D8;E8)".into(),
                    result: blank_cell_val(),
                },
                SampleScratchRow {
                    formula: "=Cp(D8;E8)".into(),
                    result: blank_cell_val(),
                },
                SampleScratchRow {
                    formula: "=G(\"Mg = Mg(g)\";1000)".into(),
                    result: blank_cell_val(),
                },
            ],

            fn_search: String::new(),
            fn_group_filter: "All".into(),
            selected_fn_idx: 0,
            test_args: Default::default(),
            test_result: blank_cell_val(),
            formula_preview: String::new(),

            show_species_picker: false,
            picker_target: PickerTarget::WorksheetActiveCell,
            picker_query: String::new(),

            bridge_port: 8765,
            bridge_handle: None,
            bridge_error: String::new(),
            webservice_fn: "H".into(),
            webservice_a: "CO(g)".into(),
            webservice_b: "100".into(),
            export_dir: "/tmp".into(),
            export_filename: "HSC_AddIns_Workbook".into(),
            export_status: String::new(),
            migrator_input: "=H(\"Fe\";298;\"Mcal\";25)".into(),
            migrator_output: String::new(),
            migrator_warnings: Vec::new(),
        };
        let (mo, mw) = SoryntelApplication::migrate_addin_formula(&state.migrator_input);
        state.migrator_output = mo;
        state.migrator_warnings = mw;
        state
    }
}

fn populate_default_worksheet_demo(grid: &mut [Vec<String>]) {
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            cell.clear();
        }
    }
    // Bloque 1: Especies garantizadas tanto en base abierta como en Maindb6.HSC
    grid[0][0] = "Species".into();
    grid[0][1] = "T".into();
    grid[0][2] = "H".into();
    grid[0][3] = "Cp".into();
    grid[0][4] = "S".into();
    grid[0][5] = "MW".into();

    let rows = [("CO(g)", "25"), ("O2(g)", "25"), ("H2(g)", "25"), ("Al2O3", "500")];
    for (idx, (sp, t)) in rows.iter().enumerate() {
        let r = idx + 1;
        let rn = r + 1;
        grid[r][0] = (*sp).into();
        grid[r][1] = (*t).into();
        grid[r][2] = format!("=H(A{rn};B{rn})");
        grid[r][3] = format!("=Cp(A{rn};B{rn})");
        grid[r][4] = format!("=S(A{rn};B{rn})");
        grid[r][5] = format!("=MW(A{rn})");
    }

    grid[5][0] = "Sum H (rows 2-5)".into();
    grid[5][2] = "=SUM(C2:C5)".into();
    grid[6][0] = "Reaction dG".into();
    grid[6][1] = "1000".into();
    grid[6][2] = "=G(\"Mg = Mg(g)\";B7)".into();
    grid[7][0] = "Pyrite Code".into();
    grid[7][1] = "Pyrite".into();
    grid[7][2] = "=MineralCode(B8)".into();
    grid[7][3] = "=MineralFormula(B8)".into();
    grid[8][0] = "StreamEQ C1 (cross-sheet)".into();
    grid[8][2] = "=StreamEQ!C1".into();
}

fn populate_streameq_worksheet_template(grid: &mut [Vec<String>]) {
    for row in grid.iter_mut() {
        for cell in row.iter_mut() {
            cell.clear();
        }
    }
    grid[0][0] = "Temperature".into();
    grid[0][2] = "1500".into();
    grid[0][3] = "°C".into();
    grid[1][0] = "Pressure".into();
    grid[1][2] = "1".into();
    grid[1][3] = "bar".into();
    grid[2][0] = "SPECIES:".into();
    grid[2][1] = "Input".into();
    grid[2][2] = "Output".into();

    let entries = [
        ("§ PHASE 1:", "25", "°C"),
        ("CO(g)", "0", "kmol"),
        ("CO2(g)", "", "kmol"),
        ("N(g)", "", "kmol"),
        ("N2(g)", "3.02", "kmol"),
        ("NO(g)", "", "kmol"),
        ("N2O3(g)", "", "kmol"),
        ("N2O4(g)", "", "kmol"),
        ("N2O5(g)", "", "kmol"),
        ("O2(g)", "0.8", "kmol"),
        ("§ PHASE 2:", "25", "°C"),
        ("C", "1", "kmol"),
    ];
    for (i, (sp, inp, u)) in entries.iter().enumerate() {
        let r = 3 + i;
        grid[r][0] = (*sp).into();
        grid[r][1] = (*inp).into();
        grid[r][3] = (*u).into();
    }
    // Fórmula matricial en C4 que se derrama hasta C15
    grid[3][2] = "=StreamEQ(A4:A15;B4:B15;C1;C2)".into();
}

fn export_value_to_sheet_cell(cell: &ExportCell) -> SheetCell {
    match &cell.value {
        ExportCellValue::Empty => SheetCell {
            raw: String::new(),
            eval: blank_cell_val(),
            style: cell.style,
            is_formula: false,
        },
        ExportCellValue::Text(s) => SheetCell {
            raw: s.clone(),
            eval: CellEvalValue::Text(s.clone()),
            style: cell.style,
            is_formula: false,
        },
        ExportCellValue::Number(v) => SheetCell {
            raw: format_cell_number(*v, 8),
            eval: CellEvalValue::Number(*v),
            style: cell.style,
            is_formula: false,
        },
        ExportCellValue::Formula {
            formula,
            cached_number,
            cached_text,
            ..
        } => {
            let f = if formula.starts_with('=') {
                formula.clone()
            } else {
                format!("={formula}")
            };
            let eval = if let Some(n) = cached_number {
                CellEvalValue::Number(*n)
            } else if let Some(t) = cached_text {
                CellEvalValue::Text(t.clone())
            } else {
                blank_cell_val()
            };
            SheetCell {
                raw: f,
                eval,
                style: cell.style,
                is_formula: true,
            }
        }
    }
}

fn load_export_sheet_into_cells(
    sheet: &ExportSheet,
    out: &mut BTreeMap<(usize, usize), SheetCell>,
) -> (usize, usize) {
    out.clear();
    let mut max_r = 0usize;
    let mut max_c = 0usize;
    for cell in &sheet.cells {
        max_r = max_r.max(cell.row);
        max_c = max_c.max(cell.col);
        out.insert((cell.row, cell.col), export_value_to_sheet_cell(cell));
    }
    (max_r, max_c)
}

fn format_cell_for_lookup(val: &CellEvalValue, raw: &str) -> String {
    match val {
        CellEvalValue::Number(v) => {
            if v.is_finite() {
                format!("{v}")
            } else {
                raw.trim().to_string()
            }
        }
        CellEvalValue::Text(s) => {
            if s.is_empty() {
                raw.trim().to_string()
            } else {
                s.clone()
            }
        }
        CellEvalValue::Array(arr) => arr
            .first()
            .map(|x| format_cell_for_lookup(x, raw))
            .unwrap_or_default(),
        CellEvalValue::Error(e) => e.clone(),
    }
}

fn cell_literal_for_expr(val: &CellEvalValue, raw: &str) -> String {
    match val {
        CellEvalValue::Number(v) => {
            if v.is_finite() {
                format!("{v}")
            } else {
                "\"#NUM!\"".into()
            }
        }
        CellEvalValue::Text(s) if s.is_empty() => {
            let t = raw.trim();
            if t.is_empty() {
                "\"\"".into()
            } else if t.parse::<f64>().is_ok() {
                t.to_string()
            } else {
                format!("\"{}\"", t.replace('"', "\"\""))
            }
        }
        CellEvalValue::Text(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        CellEvalValue::Array(arr) => arr
            .first()
            .map(|x| cell_literal_for_expr(x, raw))
            .unwrap_or_else(|| "\"\"".into()),
        CellEvalValue::Error(e) => format!("\"{e}\""),
    }
}

fn extract_leading_fn_name(expr: &str) -> Option<&str> {
    let trimmed = expr.trim().trim_start_matches(['{', '=']).trim_start();
    let end = trimmed.find('(')?;
    let cand = trimmed[..end].trim();
    if !cand.is_empty() && cand.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Some(cand)
    } else {
        None
    }
}

fn temp_c_to_unit_value(units: &AddInUnits, t_c: f64) -> f64 {
    match units.temp_unit.trim().trim_start_matches('°') {
        "K" => t_c + 273.15,
        "F" => t_c * 9.0 / 5.0 + 32.0,
        _ => t_c,
    }
}

fn has_all_species(db: &ThermoDatabase, formulas: &[&str]) -> bool {
    use soryntel_thermo::SpeciesRepository;
    formulas.iter().all(|f| db.get_species(f).is_some())
}

fn pick_existing_species(db: &ThermoDatabase, candidates: &[&str]) -> String {
    use soryntel_thermo::SpeciesRepository;
    for &c in candidates {
        if db.get_species(c).is_some() {
            return c.to_string();
        }
    }
    candidates.last().copied().unwrap_or("CO(g)").to_string()
}

fn parse_example_top_level_args(example: &str) -> Vec<String> {
    let Some(open) = example.find('(') else {
        return Vec::new();
    };
    let Some(close) = example.rfind(')') else {
        return Vec::new();
    };
    if close <= open {
        return Vec::new();
    }
    let body = &example[open + 1..close];
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut depth = 0usize;
    for ch in body.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                cur.push(ch);
            }
            '(' if !in_quotes => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_quotes => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ';' | ',' if !in_quotes && depth == 0 => {
                args.push(cur.trim().trim_matches('"').to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        args.push(cur.trim().trim_matches('"').to_string());
    }
    args
}

/// Dibuja un contorno rectangular estilo Excel sobre un rango (con handles opcionales en esquinas o fill-handle).
fn paint_range_outline(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    width: f32,
    draw_corners: bool,
    draw_fill_handle: bool,
) {
    painter.rect_stroke(rect, 0.0, egui::Stroke::new(width, color));
    if draw_corners {
        let s = 4.0;
        for pt in [
            rect.left_top(),
            rect.right_top() - egui::vec2(s, 0.0),
            rect.left_bottom() - egui::vec2(0.0, s),
            rect.right_bottom() - egui::vec2(s, s),
        ] {
            painter.rect_filled(egui::Rect::from_min_size(pt, egui::vec2(s, s)), 0.0, color);
        }
    } else if draw_fill_handle {
        let s = 5.0;
        let handle = egui::Rect::from_min_size(
            rect.right_bottom() - egui::vec2(s - 1.0, s - 1.0),
            egui::vec2(s, s),
        );
        painter.rect_filled(handle, 0.0, color);
    }
}

/// Dibuja la fórmula `=StreamEQ(A4:A...;B4:B...;C1;C2)` inline sobre C4 con colores por rango (Fig. 5).
fn paint_streameq_inline_formula(
    painter: &egui::Painter,
    c4_rect: egui::Rect,
    last_row: usize,
    sep: char,
) {
    let overlay = egui::Rect::from_min_size(
        c4_rect.min + egui::vec2(1.0, 1.0),
        egui::vec2(262.0, c4_rect.height() - 2.0),
    );
    painter.rect_filled(overlay, 0.0, egui::Color32::WHITE);
    painter.rect_stroke(
        overlay,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(160, 174, 192)),
    );

    let font = egui::FontId::proportional(11.5);
    let mut job = egui::text::LayoutJob::default();
    let push_span = |job: &mut egui::text::LayoutJob, text: &str, color: egui::Color32| {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        );
    };
    let sep_str = sep.to_string();
    push_span(&mut job, "=StreamEQ(", TEXT_DARK);
    push_span(&mut job, &format!("A4:A{last_row}"), REF_BLUE);
    push_span(&mut job, &sep_str, TEXT_DARK);
    push_span(&mut job, &format!("B4:B{last_row}"), REF_GREEN);
    push_span(&mut job, &sep_str, TEXT_DARK);
    push_span(&mut job, "C1", REF_PURPLE);
    push_span(&mut job, &sep_str, TEXT_DARK);
    push_span(&mut job, "C2", REF_PURPLE);
    push_span(&mut job, ")|", TEXT_DARK);

    let galley = painter.layout_job(job);
    painter.galley(
        egui::pos2(
            overlay.min.x + 4.0,
            overlay.center().y - galley.size().y * 0.5,
        ),
        galley,
        TEXT_DARK,
    );
}

impl ExcelAddInsModuleState {
    pub fn ensure_initialized(&mut self, db: &ThermoDatabase) {
        if !self.initialized_db_sheets {
            self.initialized_db_sheets = true;
            let sample = SoryntelApplication::addin_sample_sheet(db, &self.units);
            let (mr, mc) = load_export_sheet_into_cells(&sample, &mut self.sample_cells);
            self.sample_max_row = mr;
            self.sample_max_col = mc;
            if let Some(meta) = ADDIN_FUNCTION_CATALOG.get(self.selected_fn_idx).copied() {
                self.populate_tester_from_example(meta, db);
            }
            self.recalculate_all(db);
        }
    }

    pub fn is_pack_enabled(&self, origin: AddInOrigin) -> bool {
        let target_kind = match origin {
            AddInOrigin::Hsc6Xll => AddInPackKind::Hsc6Core,
            AddInOrigin::Hsc10Xll => AddInPackKind::Hsc10UnitQuery,
            AddInOrigin::MineralAddIn => AddInPackKind::Mineralogy,
            AddInOrigin::HscSimOnly => AddInPackKind::HscSimRef,
        };
        self.packs
            .iter()
            .find(|p| p.kind == target_kind)
            .map(|p| p.enabled)
            .unwrap_or(true)
    }

    fn check_formula_pack_access(&self, formula: &str) -> Option<CellEvalValue> {
        let Some(fname) = extract_leading_fn_name(formula) else {
            return None;
        };
        if let Some(meta) = ADDIN_FUNCTION_CATALOG
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(fname))
        {
            if !self.is_pack_enabled(meta.origin) {
                return Some(CellEvalValue::Error("#NAME?".into()));
            }
        }
        None
    }

    fn resolve_cross_sheet_refs(&self, formula: &str) -> String {
        let bytes = formula.as_bytes();
        let mut out = String::with_capacity(formula.len());
        let mut i = 0usize;
        let mut in_quotes = false;

        while i < bytes.len() {
            let c = bytes[i] as char;
            if c == '"' {
                in_quotes = !in_quotes;
                out.push(c);
                i += 1;
                continue;
            }
            if !in_quotes {
                let prev_ok = i == 0 || {
                    let p = bytes[i - 1] as char;
                    !p.is_ascii_alphanumeric() && p != '_'
                };
                if prev_ok {
                    if let Some(bang_rel) = formula[i..].find('!') {
                        let prefix = &formula[i..i + bang_rel];
                        if !prefix.is_empty()
                            && prefix.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                        {
                            let sheet_norm = prefix.to_ascii_lowercase();
                            if matches!(
                                sheet_norm.as_str(),
                                "streameq" | "minerals" | "addinsample" | "worksheet" | "sheet1"
                            ) {
                                let after = &formula[i + bang_rel + 1..];
                                let coord_len = after
                                    .bytes()
                                    .take_while(|b| b.is_ascii_alphanumeric() || *b == b'$')
                                    .count();
                                let coord_str = &after[..coord_len];
                                let coord_clean = coord_str.replace('$', "");
                                if let Some((row, col)) = parse_a1(&coord_clean) {
                                    let val_lit = self.cross_sheet_cell_expr(&sheet_norm, row, col);
                                    out.push_str(&val_lit);
                                    i += bang_rel + 1 + coord_len;
                                    continue;
                                }
                            }
                        }
                    }
                }
            }
            out.push(c);
            i += 1;
        }
        out
    }

    fn cross_sheet_cell_expr(&self, sheet_norm: &str, row: usize, col: usize) -> String {
        match sheet_norm {
            "streameq" => {
                if row == 0 && col == 2 {
                    self.eq_temp_raw.trim().to_string()
                } else if row == 1 && col == 2 {
                    self.eq_press_raw.trim().to_string()
                } else if row >= 3 {
                    let idx = row - 3;
                    match col {
                        0 => self
                            .eq_species
                            .get(idx)
                            .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
                            .unwrap_or_else(|| "\"\"".into()),
                        1 => self
                            .eq_inputs
                            .get(idx)
                            .map(|s| {
                                let t = s.trim();
                                if t.is_empty() {
                                    "0".into()
                                } else {
                                    t.to_string()
                                }
                            })
                            .unwrap_or_else(|| "0".into()),
                        2 => self
                            .eq_outputs
                            .get(idx)
                            .map(|v| cell_literal_for_expr(v, ""))
                            .unwrap_or_else(|| "0".into()),
                        _ => "\"\"".into(),
                    }
                } else {
                    "\"\"".into()
                }
            }
            "minerals" => self
                .min_cells
                .get(&(row, col))
                .map(|c| cell_literal_for_expr(&c.eval, &c.raw))
                .unwrap_or_else(|| "\"\"".into()),
            "addinsample" => self
                .sample_cells
                .get(&(row, col))
                .map(|c| cell_literal_for_expr(&c.eval, &c.raw))
                .unwrap_or_else(|| "\"\"".into()),
            _ => {
                if row < self.ws_raw.len() && col < WS_COLS {
                    cell_literal_for_expr(&self.ws_eval[row][col], &self.ws_raw[row][col])
                } else {
                    "\"\"".into()
                }
            }
        }
    }

    pub fn eval_sheet_formula<F>(
        &mut self,
        db: &ThermoDatabase,
        raw_formula: &str,
        lookup: F,
    ) -> CellEvalValue
    where
        F: FnMut(usize, usize) -> String,
    {
        if let Some(err) = self.check_formula_pack_access(raw_formula) {
            return err;
        }
        let expanded = self.resolve_cross_sheet_refs(raw_formula);
        SoryntelApplication::eval_addin_formula(db, &mut self.units, &expanded, lookup)
    }

    pub fn recalculate_all(&mut self, db: &ThermoDatabase) {
        self.push_units_to_bridge();
        self.recalculate_streameq(db);
        self.recalculate_minerals(db);
        self.recalculate_sample_sheet(db);
        self.recalculate_worksheet(db);
        self.recalculate_scratch_rows(db);
        self.recalculate_tester(db);
    }

    fn push_units_to_bridge(&self) {
        if let Ok(mut guard) = self.shared_units.lock() {
            *guard = self.units.clone();
        }
    }

    fn pull_units_from_bridge_if_changed(&mut self, db: &ThermoDatabase) {
        let external = self
            .shared_units
            .lock()
            .ok()
            .map(|g| g.clone());
        if let Some(ext) = external {
            if ext.temp_unit != self.units.temp_unit
                || ext.energy_unit != self.units.energy_unit
                || ext.pressure_unit != self.units.pressure_unit
            {
                self.units = ext;
                self.sync_sample_d2_e2_from_units(db);
                self.recalculate_all(db);
                self.status_msg = format!(
                    "Unidades sincronizadas desde HTTP Bridge: {} / {} / {}.",
                    format_temp_unit_display(&self.units.temp_unit),
                    self.units.energy_unit,
                    self.units.pressure_unit
                );
            }
        }
    }

    pub fn is_bridge_running(&self) -> bool {
        self.bridge_handle
            .as_ref()
            .map(|h| h.is_running())
            .unwrap_or(false)
    }

    pub fn start_bridge(&mut self, db: &ThermoDatabase) {
        self.stop_bridge();
        self.push_units_to_bridge();
        match BridgeHandle::start(
            Arc::new(db.clone()),
            Arc::clone(&self.shared_units),
            self.bridge_port,
        ) {
            Ok(handle) => {
                self.bridge_port = handle.port();
                self.bridge_handle = Some(handle);
                self.bridge_error.clear();
                self.status_msg = format!(
                    "HTTP Bridge activo en http://127.0.0.1:{} (Excel/Calc/ONLYOFFICE/WPS).",
                    self.bridge_port
                );
            }
            Err(err) => {
                self.bridge_error = err.clone();
                self.status_msg = err;
            }
        }
    }

    pub fn stop_bridge(&mut self) {
        if let Some(mut handle) = self.bridge_handle.take() {
            handle.stop();
            self.status_msg = "HTTP Bridge detenido (modo 100% interno activo).".into();
        }
    }

    pub fn recalculate_worksheet(&mut self, db: &ThermoDatabase) {
        // 1) Reiniciar celdas no derramadas y parsear literales base primero
        for r in 0..WS_ROWS {
            for c in 0..WS_COLS {
                self.ws_array_label[r][c] = None;
                let raw = self.ws_raw[r][c].trim();
                if raw.is_empty() {
                    self.ws_eval[r][c] = blank_cell_val();
                } else if !raw.starts_with('=') && !raw.starts_with("{=") {
                    if let Ok(v) = raw.parse::<f64>() {
                        self.ws_eval[r][c] = CellEvalValue::Number(v);
                    } else {
                        self.ws_eval[r][c] = CellEvalValue::Text(raw.to_string());
                    }
                }
            }
        }

        // 2) Propagación multi-paso (hasta convergencia o 16 pasadas)
        for _pass in 0..16 {
            let mut changed = false;
            let mut spilled_targets: HashSet<(usize, usize)> = HashSet::new();

            for r in 0..WS_ROWS {
                for c in 0..WS_COLS {
                    let raw = self.ws_raw[r][c].trim().to_string();
                    if !(raw.starts_with('=') || raw.starts_with("{=")) {
                        continue;
                    }
                    let snap_raw = self.ws_raw.clone();
                    let snap_eval = self.ws_eval.clone();
                    let val = self.eval_sheet_formula(db, &raw, |rr, cc| {
                        if rr < WS_ROWS && cc < WS_COLS {
                            format_cell_for_lookup(&snap_eval[rr][cc], &snap_raw[rr][cc])
                        } else {
                            String::new()
                        }
                    });
                    match val {
                        CellEvalValue::Array(arr) => {
                            let tag = if raw.starts_with("{=") {
                                raw.clone()
                            } else {
                                format!("{{{raw}}}")
                            };
                            for (offset, item) in arr.into_iter().enumerate() {
                                let tr = r + offset;
                                if tr >= WS_ROWS {
                                    break;
                                }
                                if offset == 0 || self.ws_raw[tr][c].trim().is_empty() || spilled_targets.contains(&(tr, c)) {
                                    if self.ws_eval[tr][c] != item {
                                        self.ws_eval[tr][c] = item;
                                        changed = true;
                                    }
                                    self.ws_array_label[tr][c] = Some(tag.clone());
                                    spilled_targets.insert((tr, c));
                                }
                            }
                        }
                        other => {
                            if self.ws_eval[r][c] != other {
                                self.ws_eval[r][c] = other;
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    pub fn recalculate_streameq(&mut self, db: &ThermoDatabase) {
        if !self.is_pack_enabled(AddInOrigin::Hsc6Xll) {
            self.eq_outputs = vec![CellEvalValue::Error("#NAME?".into()); self.eq_species.len()];
            self.eq_balance.clear();
            self.status_msg = "StreamEQ deshabilitado: activa 'HSC6 Core' en File -> Options -> Add-Ins.".into();
            return;
        }

        let t_parsed = self.eq_temp_raw.trim().parse::<f64>().ok();
        let p_parsed = self.eq_press_raw.trim().parse::<f64>().ok();
        let (t_val, p_val) = match (t_parsed, p_parsed) {
            (Some(t), Some(p)) => (t, p),
            _ => {
                self.eq_outputs =
                    vec![CellEvalValue::Error("#VALUE!".into()); self.eq_species.len()];
                self.eq_balance.clear();
                self.status_msg = "StreamEQ #VALUE!: C1 (Temperature) y C2 (Pressure) deben ser numéricos.".into();
                return;
            }
        };

        let res = SoryntelApplication::eval_addin_streameq(
            db,
            &self.units,
            &self.eq_species,
            &self.eq_inputs,
            t_val,
            p_val,
        );
        match res {
            CellEvalValue::Array(arr) => {
                self.eq_outputs = arr;
                self.eq_balance = SoryntelApplication::addin_streameq_balance(
                    &self.eq_species,
                    &self.eq_inputs,
                    &self.eq_outputs,
                );
                let td = format_temp_unit_display(&self.units.temp_unit);
                self.status_msg = format!(
                    "StreamEQ actualizado ({t_val} {td}, {p_val} {}).",
                    self.units.pressure_unit
                );
            }
            CellEvalValue::Error(err) => {
                self.eq_outputs = vec![CellEvalValue::Error(err.clone()); self.eq_species.len()];
                self.eq_balance.clear();
                self.status_msg = if err == "#NAME?" {
                    "StreamEQ #NAME?: alguna especie no existe en la base de datos activa.".into()
                } else {
                    format!("StreamEQ {err}: revisa Temperatura (>0 K), Presión (>0) y especies.")
                };
            }
            other => {
                self.eq_outputs = vec![other];
                self.eq_balance.clear();
            }
        }
    }

    pub fn recalculate_minerals(&mut self, db: &ThermoDatabase) {
        if self.min_paste_values {
            return;
        }
        let keys: Vec<(usize, usize)> = self.min_cells.keys().copied().collect();
        for _ in 0..4 {
            let mut changed = false;
            for (r, c) in &keys {
                let raw = self.min_cells.get(&(*r, *c)).map(|x| x.raw.clone()).unwrap_or_default();
                if !(raw.starts_with('=') || raw.starts_with("{=")) {
                    continue;
                }
                let snap = self.min_cells.clone();
                let val = self.eval_sheet_formula(db, &raw, |rr, cc| {
                    snap.get(&(rr, cc))
                        .map(|cell| format_cell_for_lookup(&cell.eval, &cell.raw))
                        .unwrap_or_default()
                });
                if let Some(cell) = self.min_cells.get_mut(&(*r, *c)) {
                    if cell.eval != val {
                        cell.eval = val;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    pub fn recalculate_sample_sheet(&mut self, db: &ThermoDatabase) {
        if self.sample_cells.is_empty() {
            let sample = SoryntelApplication::addin_sample_sheet(db, &self.units);
            let (mr, mc) = load_export_sheet_into_cells(&sample, &mut self.sample_cells);
            self.sample_max_row = mr;
            self.sample_max_col = mc;
        }
        let keys: Vec<(usize, usize)> = self.sample_cells.keys().copied().collect();
        // Pasada para literales primero
        for (r, c) in &keys {
            if let Some(cell) = self.sample_cells.get_mut(&(*r, *c)) {
                let trimmed = cell.raw.trim();
                if !trimmed.starts_with('=') && !trimmed.starts_with("{=") {
                    if trimmed.is_empty() {
                        cell.eval = blank_cell_val();
                    } else if let Ok(n) = trimmed.parse::<f64>() {
                        cell.eval = CellEvalValue::Number(n);
                    } else {
                        cell.eval = CellEvalValue::Text(trimmed.to_string());
                    }
                }
            }
        }
        for _ in 0..8 {
            let mut changed = false;
            for (r, c) in &keys {
                let raw = self
                    .sample_cells
                    .get(&(*r, *c))
                    .map(|x| x.raw.clone())
                    .unwrap_or_default();
                if !(raw.starts_with('=') || raw.starts_with("{=")) {
                    continue;
                }
                let snap = self.sample_cells.clone();
                let val = self.eval_sheet_formula(db, &raw, |rr, cc| {
                    snap.get(&(rr, cc))
                        .map(|cell| format_cell_for_lookup(&cell.eval, &cell.raw))
                        .unwrap_or_default()
                });
                if let Some(cell) = self.sample_cells.get_mut(&(*r, *c)) {
                    if cell.eval != val {
                        cell.eval = val;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }

    fn sync_sample_d2_e2_from_units(&mut self, db: &ThermoDatabase) {
        if let Some(c) = self.sample_cells.get_mut(&(1, 3)) {
            c.raw = self.units.temp_unit.clone();
            c.eval = CellEvalValue::Text(self.units.temp_unit.clone());
        }
        if let Some(c) = self.sample_cells.get_mut(&(1, 4)) {
            c.raw = self.units.energy_unit.clone();
            c.eval = CellEvalValue::Text(self.units.energy_unit.clone());
        }
        self.recalculate_sample_sheet(db);
    }

    pub fn recalculate_scratch_rows(&mut self, db: &ThermoDatabase) {
        let eq_sp = self.eq_species.clone();
        let eq_in = self.eq_inputs.clone();
        let eq_t = self.eq_temp_raw.clone();
        let eq_p = self.eq_press_raw.clone();
        let sample_snap = self.sample_cells.clone();

        for idx in 0..self.scratch_rows.len() {
            let formula = self.scratch_rows[idx].formula.clone();
            let expanded = self.resolve_cross_sheet_refs(&formula);
            let res = if let Some(err) = self.check_formula_pack_access(&expanded) {
                err
            } else {
                SoryntelApplication::eval_addin_formula(db, &mut self.units, &expanded, |r, c| {
                    if let Some(cell) = sample_snap.get(&(r, c)) {
                        return format_cell_for_lookup(&cell.eval, &cell.raw);
                    }
                    if r == 0 && c == 2 {
                        eq_t.clone()
                    } else if r == 1 && c == 2 {
                        eq_p.clone()
                    } else if r >= 3 {
                        let i = r - 3;
                        match c {
                            0 => eq_sp.get(i).cloned().unwrap_or_default(),
                            1 => eq_in.get(i).cloned().unwrap_or_default(),
                            _ => String::new(),
                        }
                    } else {
                        String::new()
                    }
                })
            };
            self.scratch_rows[idx].result = res;
        }
    }

    fn resolve_tester_arg(&self, fn_upper: &str, arg_idx: usize, raw: &str) -> ArgValue {
        let t = raw.trim();
        // Soporte de rango A1:B2 desde el Worksheet actual si se pasa una referencia de hoja
        if let Some((a, b)) = t.split_once(':') {
            let a_clean = a.trim().replace('$', "");
            let b_clean = b.trim().replace('$', "");
            if let (Some((r0, c0)), Some((r1, c1))) = (parse_a1(&a_clean), parse_a1(&b_clean)) {
                let mut vals = Vec::new();
                for rr in r0.min(r1)..=r0.max(r1) {
                    for cc in c0.min(c1)..=c0.max(c1) {
                        vals.push(self.lookup_active_sheet_cell(rr, cc));
                    }
                }
                return ArgValue::Range(vals);
            }
        }

        let is_range_slot = if t.contains('|') {
            true
        } else if fn_upper.starts_with("STREAM")
            || fn_upper.starts_with("FLOW")
            || fn_upper == "DENSITYA"
        {
            arg_idx <= 1 && !matches!(fn_upper, "STREAMX" | "STREAMMASS" | "STREAMVOL")
        } else {
            matches!(fn_upper, "SUM" | "AVERAGE" | "MIN" | "MAX")
        };

        if is_range_slot {
            let sep = if t.contains('|') {
                '|'
            } else if t.contains(';') {
                ';'
            } else if t.contains(',') {
                ','
            } else {
                '\0'
            };
            if sep != '\0' {
                let items = t.split(sep).map(|s| s.trim().to_string()).collect();
                return ArgValue::Range(items);
            }
            if fn_upper.starts_with("STREAM")
                || fn_upper.starts_with("FLOW")
                || fn_upper == "DENSITYA"
            {
                return ArgValue::Range(vec![t.to_string()]);
            }
        }

        ArgValue::Scalar(t.to_string())
    }

    fn lookup_active_sheet_cell(&self, row: usize, col: usize) -> String {
        match self.active_sheet {
            AddInSheetTab::Worksheet => {
                if row < WS_ROWS && col < WS_COLS {
                    format_cell_for_lookup(&self.ws_eval[row][col], &self.ws_raw[row][col])
                } else {
                    String::new()
                }
            }
            AddInSheetTab::StreamEq => self
                .cross_sheet_cell_expr("streameq", row, col)
                .trim_matches('"')
                .to_string(),
            AddInSheetTab::Minerals => self
                .min_cells
                .get(&(row, col))
                .map(|c| format_cell_for_lookup(&c.eval, &c.raw))
                .unwrap_or_default(),
            AddInSheetTab::AddInSample | AddInSheetTab::FunctionsCatalog => self
                .sample_cells
                .get(&(row, col))
                .map(|c| format_cell_for_lookup(&c.eval, &c.raw))
                .unwrap_or_default(),
        }
    }

    pub fn recalculate_tester(&mut self, db: &ThermoDatabase) {
        let Some(meta) = ADDIN_FUNCTION_CATALOG.get(self.selected_fn_idx).copied() else {
            return;
        };
        if !self.is_pack_enabled(meta.origin) {
            self.test_result = CellEvalValue::Error("#NAME?".into());
            self.formula_preview = format!("={}(...)", meta.name);
            return;
        }
        let fn_upper = meta.name.to_ascii_uppercase();
        let mut args = Vec::new();
        for (idx, a) in self.test_args.iter().enumerate() {
            if !a.trim().is_empty() {
                args.push(self.resolve_tester_arg(&fn_upper, idx, a));
            }
        }
        let sep = self.arg_separator.ch();
        let non_empty: Vec<String> = self
            .test_args
            .iter()
            .filter(|a| !a.trim().is_empty())
            .enumerate()
            .map(|(i, a)| {
                let t = a.trim();
                let is_range = t.contains(':')
                    || ((fn_upper.starts_with("STREAM")
                        || fn_upper.starts_with("FLOW")
                        || fn_upper == "DENSITYA")
                        && i <= 1
                        && !matches!(fn_upper.as_str(), "STREAMX" | "STREAMMASS" | "STREAMVOL"));
                if is_range {
                    t.to_string()
                } else if t.parse::<f64>().is_ok() {
                    t.to_string()
                } else if t.starts_with('"') && t.ends_with('"') {
                    t.to_string()
                } else {
                    format!("\"{t}\"")
                }
            })
            .collect();
        self.formula_preview = format!("={}({})", meta.name, non_empty.join(&sep.to_string()));
        self.test_result = SoryntelApplication::eval_addin_call(db, &mut self.units, meta.name, &args);
    }

    pub fn populate_tester_from_example(&mut self, meta: AddInFunctionMeta, db: &ThermoDatabase) {
        self.test_args = Default::default();
        let upper = meta.name.to_ascii_uppercase();
        let t25 = format_cell_number(temp_c_to_unit_value(&self.units, 25.0), 4);
        let t100 = format_cell_number(temp_c_to_unit_value(&self.units, 100.0), 4);
        let t200 = format_cell_number(temp_c_to_unit_value(&self.units, 200.0), 4);
        let t1000 = format_cell_number(temp_c_to_unit_value(&self.units, 1000.0), 4);
        let t0 = format_cell_number(temp_c_to_unit_value(&self.units, 0.0), 4);
        let t_min = format_cell_number(temp_c_to_unit_value(&self.units, 0.0), 4);
        let t_max = format_cell_number(temp_c_to_unit_value(&self.units, 2000.0), 4);
        let td = format_temp_unit_display(&self.units.temp_unit);

        let sp_solid = pick_existing_species(db, &["Fe", "Fe2O3", "Al2O3", "Ni"]);
        let sp_oxide = pick_existing_species(db, &["Fe2O3", "Al2O3", "TiO2"]);
        let sp_gas = pick_existing_species(db, &["CO2(g)", "CO(g)", "O2(g)"]);
        let sp_cond = pick_existing_species(db, &["H2O(l)", "H2O", "Al2O3", "CO(g)"]);
        let reac_eq = if !db.search("CO2(g)", 1).is_empty() && !db.search("C", 1).is_empty() {
            "C + O2(g) = CO2(g)"
        } else {
            "Mg = Mg(g)"
        };
        let (stream_sp, stream_amt, stream_vec): (&str, &str, Vec<String>) =
            if has_all_species(db, &["Fe", "Cu", "Zn"]) {
                ("Fe;Cu;Zn", "1;1;1", vec!["Fe".into(), "Cu".into(), "Zn".into()])
            } else {
                ("Al2O3;TiO2;Ni", "1;1;1", vec!["Al2O3".into(), "TiO2".into(), "Ni".into()])
            };
        let eq_pair = if has_all_species(db, &["CO(g)", "CO2(g)", "O2(g)", "C"]) {
            ("CO(g);CO2(g);O2(g);C", "0;0;0.8;1")
        } else {
            ("Mg;Mg(g)", "1;0")
        };

        let mut u_tmp = self.units.clone();

        if upper == "UNITS" {
            self.test_args[0] = self.units.temp_unit.clone();
            self.test_args[1] = self.units.energy_unit.clone();
            self.test_args[2] = self.units.pressure_unit.clone();
        } else if upper == "SPECIES" {
            self.test_args[0] = "2".into();
            self.test_args[1] = "3".into();
        } else if upper == "BAL" {
            self.test_args[0] = "C+O2(g)=CO(g)".into();
            self.test_args[1] = "0.5".into();
        } else if upper == "STREAMEQ" {
            self.test_args[0] = eq_pair.0.into();
            self.test_args[1] = eq_pair.1.into();
            self.test_args[2] = t1000;
            self.test_args[3] = "1".into();
        } else if upper == "DENSITYA" {
            self.test_args[0] = "NaCl;FeSO4".into();
            self.test_args[1] = "0.2;0.1".into();
            self.test_args[2] = t25;
        } else if let Some(fwd) = upper.strip_prefix("TAT") {
            let probe_sp = if matches!(fwd, "HNM3" | "HCM") {
                sp_gas.clone()
            } else {
                sp_solid.clone()
            };
            let target = SoryntelApplication::eval_addin_call(
                db,
                &mut u_tmp,
                fwd,
                &[ArgValue::Scalar(probe_sp.clone()), ArgValue::Scalar(t200)],
            )
            .as_f64()
            .unwrap_or(0.0);
            self.test_args[0] = probe_sp;
            self.test_args[1] = format!("{target:.6}");
            self.test_args[2] = t_min;
            self.test_args[3] = t_max;
        } else if let Some(rest) = upper.strip_prefix("STREAMT") {
            let fwd = format!("Stream{rest}");
            let target = SoryntelApplication::eval_addin_call(
                db,
                &mut u_tmp,
                &fwd,
                &[
                    ArgValue::Range(stream_vec),
                    ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
                    ArgValue::Scalar(t100),
                ],
            )
            .as_f64()
            .unwrap_or(0.0);
            self.test_args[0] = stream_sp.into();
            self.test_args[1] = stream_amt.into();
            self.test_args[2] = format!("{target:.6}");
            self.test_args[3] = t0;
            self.test_args[4] = t1000;
        } else if let Some(rest) = upper.strip_prefix("FLOWT") {
            let fwd = format!("Flow{rest}");
            let target = SoryntelApplication::eval_addin_call(
                db,
                &mut u_tmp,
                &fwd,
                &[
                    ArgValue::Range(stream_vec),
                    ArgValue::Range(vec!["1".into(), "1".into(), "1".into()]),
                    ArgValue::Scalar(t100),
                ],
            )
            .as_f64()
            .unwrap_or(0.0);
            self.test_args[0] = stream_sp.into();
            self.test_args[1] = stream_amt.into();
            self.test_args[2] = format!("{target:.6}");
            self.test_args[3] = t0;
            self.test_args[4] = t1000;
        } else if (upper.starts_with("STREAM")
            && !matches!(upper.as_str(), "STREAMX" | "STREAMMASS" | "STREAMVOL"))
            || upper.starts_with("FLOW")
        {
            self.test_args[0] = stream_sp.into();
            self.test_args[1] = stream_amt.into();
            self.test_args[2] = t100;
        } else if matches!(upper.as_str(), "H" | "HKG" | "HLAT" | "S" | "G" | "CP" | "HCM" | "HNM3")
        {
            let sp = if matches!(upper.as_str(), "HCM" | "HNM3" | "G") {
                sp_gas
            } else {
                sp_solid
            };
            self.test_args[0] = sp;
            self.test_args[1] = t100;
        } else if upper == "K" || upper.starts_with("REACTION") {
            self.test_args[0] = reac_eq.into();
            self.test_args[1] = if upper == "K" || upper == "REACTIONK" || upper == "REACTIONLN K" {
                t1000
            } else {
                t25
            };
        } else if upper == "PROPT" {
            let target = SoryntelApplication::eval_addin_call(
                db,
                &mut u_tmp,
                "H",
                &[ArgValue::Scalar(sp_solid.clone()), ArgValue::Scalar(t200)],
            )
            .as_f64()
            .unwrap_or(0.0);
            self.test_args[0] = "H".into();
            self.test_args[1] = sp_solid;
            self.test_args[2] = format!("{target:.6}");
            self.test_args[3] = t_min;
            self.test_args[4] = t_max;
        } else if upper == "DELTAHT" || upper == "DELTAGT" || upper == "DELTAT" {
            self.test_args[0] = sp_cond;
            self.test_args[1] = t100;
            self.test_args[2] = t25;
        } else if upper == "PROPP" || upper == "REACTIONP" {
            self.test_args[0] = if upper == "PROPP" { "H".into() } else { "ReactionH".into() };
            self.test_args[1] = if upper == "PROPP" { sp_solid } else { reac_eq.into() };
            self.test_args[2] = t25;
            self.test_args[3] = "1".into();
        } else if upper.starts_with("MINERAL") && upper != "MINERALX" {
            self.test_args[0] = "Py".into();
            if matches!(upper.as_str(), "MINERALC" | "MINERALELEMENTWT") {
                self.test_args[1] = "Fe".into();
            } else if upper == "MINERALPROPERTY" {
                self.test_args[1] = "Formula".into();
            }
        } else if meta.origin == AddInOrigin::HscSimOnly {
            self.test_args[0] = "Feed".into();
            self.test_args[1] = "Chalcopyrite".into();
            self.test_args[2] = "Cu".into();
        } else if matches!(
            upper.as_str(),
            "ATOMAT" | "SPECIESPROPORTION" | "SPECIESWT" | "WT%" | "MOL%" | "ELEMENTWT%" | "ELEMENTMOL%"
        ) {
            self.test_args[0] = "FeS2".into();
            if upper != "ATOMAT" {
                self.test_args[1] = "Fe".into();
            }
        } else if matches!(
            upper.as_str(),
            "MOLECULARWEIGHT" | "MW" | "SPECIESNAME" | "SPECIESPHASE" | "SPECIESTMIN" | "SPECIESTMAX"
                | "SPECIESDELTAH" | "SPECIESS298" | "SPECIESCP" | "SPECIESDENSITY" | "SPECIESCOLOR"
                | "SPECIESCASCODE" | "T_MELT" | "T_BOIL" | "FORMULA" | "DE" | "REF" | "BP" | "MP"
                | "STRUCT" | "CHNAME" | "CONAME" | "CAN" | "PHASE" | "RGBCOLOR" | "SOLU"
                | "CPFUNCTION" | "TMIN" | "TMAX" | "REL"
        ) {
            self.test_args[0] = sp_oxide;
        } else if matches!(upper.as_str(), "CPA" | "CPB" | "CPC" | "CPD") {
            self.test_args[0] = sp_oxide;
            self.test_args[1] = "1".into();
        } else if upper == "UNITCONVERT" {
            self.test_args[0] = "100".into();
            self.test_args[1] = td;
            self.test_args[2] = "K".into();
        } else if matches!(upper.as_str(), "TP" | "HT" | "ST" | "GT" | "VT" | "PT") {
            self.test_args[0] = eq_pair.0.into();
            self.test_args[1] = eq_pair.1.into();
            self.test_args[2] = t1000;
            self.test_args[3] = "1".into();
        } else if matches!(upper.as_str(), "SUM" | "AVERAGE" | "MIN" | "MAX") {
            self.test_args[0] = "C2:C5".into();
        } else {
            let parsed = parse_example_top_level_args(meta.example);
            if !parsed.is_empty() {
                for (i, part) in parsed.into_iter().take(5).enumerate() {
                    self.test_args[i] = part;
                }
            } else {
                self.test_args[0] = sp_solid;
                self.test_args[1] = t25;
            }
        }
        self.recalculate_tester(db);
    }

    pub fn select_cell(&mut self, cell_ref: &str, raw_text: String) {
        self.active_cell_ref = cell_ref.to_string();
        self.formula_bar_text = raw_text;
    }

    pub fn apply_formula_bar(&mut self, db: &ThermoDatabase) {
        let cell_ref = self.active_cell_ref.trim().to_ascii_uppercase();
        let val = self.formula_bar_text.clone();
        self.active_cell_ref = cell_ref.clone();

        match self.active_sheet {
            AddInSheetTab::Worksheet => {
                if let Some((r, c)) = parse_a1(&cell_ref) {
                    if r < WS_ROWS && c < WS_COLS {
                        let stored = val
                            .trim()
                            .strip_prefix('{')
                            .and_then(|s| s.strip_suffix('}'))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| val.clone());
                        self.ws_raw[r][c] = stored;
                        self.recalculate_worksheet(db);
                        self.status_msg = format!("Worksheet {cell_ref} actualizado.");
                        return;
                    }
                }
                self.status_msg = format!("Referencia inválida para Worksheet (A1:H24): '{cell_ref}'.");
            }
            AddInSheetTab::StreamEq => {
                if cell_ref == "C1" {
                    self.eq_temp_raw = val.trim().to_string();
                    self.recalculate_streameq(db);
                } else if cell_ref == "C2" {
                    self.eq_press_raw = val.trim().to_string();
                    self.recalculate_streameq(db);
                } else if let Some((r, c)) = parse_a1(&cell_ref) {
                    if r >= 3 {
                        let idx = r - 3;
                        while self.eq_species.len() <= idx {
                            self.eq_species.push(String::new());
                            self.eq_inputs.push(String::new());
                        }
                        if c == 0 {
                            self.eq_species[idx] = val.trim().to_string();
                            self.recalculate_streameq(db);
                        } else if c == 1 {
                            self.eq_inputs[idx] = val.trim().to_string();
                            self.recalculate_streameq(db);
                        } else if c == 2 {
                            self.eq_formula_editing = false;
                            self.eq_array_committed = true;
                            self.recalculate_streameq(db);
                            self.status_msg = "StreamEQ C4:C15 recalculado (Ctrl+Shift+Enter -> {=...}).".into();
                        }
                    }
                }
            }
            AddInSheetTab::Minerals => {
                if let Some((r, c)) = parse_a1(&cell_ref) {
                    let is_f = val.trim().starts_with('=') || val.trim().starts_with("{=");
                    let prev_style = self
                        .min_cells
                        .get(&(r, c))
                        .map(|x| x.style)
                        .unwrap_or(ExportCellStyle::Default);
                    self.min_cells.insert(
                        (r, c),
                        SheetCell {
                            raw: val.clone(),
                            eval: if is_f {
                                blank_cell_val()
                            } else {
                                CellEvalValue::Text(val.clone())
                            },
                            style: prev_style,
                            is_formula: is_f,
                        },
                    );
                    self.min_max_row = self.min_max_row.max(r);
                    self.min_max_col = self.min_max_col.max(c);
                    self.recalculate_minerals(db);
                    self.status_msg = format!("Minerals {cell_ref} actualizado.");
                }
            }
            AddInSheetTab::AddInSample => {
                if let Some(idx_str) = cell_ref.strip_prefix("SCRATCH") {
                    if let Ok(n) = idx_str.parse::<usize>() {
                        if let Some(row) = self.scratch_rows.get_mut(n.saturating_sub(1)) {
                            row.formula = val;
                            self.recalculate_scratch_rows(db);
                            self.status_msg = format!("{cell_ref} actualizado.");
                            return;
                        }
                    }
                }
                if let Some((r, c)) = parse_a1(&cell_ref) {
                    let is_f = val.trim().starts_with('=') || val.trim().starts_with("{=");
                    let prev_style = self
                        .sample_cells
                        .get(&(r, c))
                        .map(|x| x.style)
                        .unwrap_or(ExportCellStyle::Default);
                    self.sample_cells.insert(
                        (r, c),
                        SheetCell {
                            raw: val.clone(),
                            eval: if is_f {
                                blank_cell_val()
                            } else {
                                CellEvalValue::Text(val.clone())
                            },
                            style: prev_style,
                            is_formula: is_f,
                        },
                    );
                    self.sample_max_row = self.sample_max_row.max(r);
                    self.sample_max_col = self.sample_max_col.max(c);
                    self.recalculate_sample_sheet(db);
                    // Si la celda modificada es D2 o E2, sincronizar AddInUnits
                    if (r == 1 && (c == 3 || c == 4)) && !is_f {
                        if let Some(CellEvalValue::Text(status)) =
                            self.sample_cells.get(&(0, 1)).map(|x| &x.eval)
                        {
                            if !status.starts_with('#') {
                                self.recalculate_all(db);
                            }
                        }
                    }
                    self.status_msg = format!("AddInSample {cell_ref} actualizado.");
                }
            }
            AddInSheetTab::FunctionsCatalog => {
                if let Some(rest) = val.trim().strip_prefix('=') {
                    if let Some((name, inside)) = rest.split_once('(') {
                        let name = name.trim();
                        if let Some(idx) = ADDIN_FUNCTION_CATALOG
                            .iter()
                            .position(|m| m.name.eq_ignore_ascii_case(name))
                        {
                            self.selected_fn_idx = idx;
                            self.test_args = Default::default();
                            let body = inside.trim_end_matches(')');
                            for (i, part) in body.split([';', ',']).take(5).enumerate() {
                                self.test_args[i] = part.trim().trim_matches('"').to_string();
                            }
                            self.recalculate_tester(db);
                        }
                    }
                }
            }
        }
    }

    pub fn insert_formula_into_worksheet(&mut self, db: &ThermoDatabase, formula: String) {
        let (r, c) = parse_a1(&self.active_cell_ref)
            .filter(|&(r, c)| r < WS_ROWS && c < WS_COLS)
            .unwrap_or((1, 2));
        self.ws_raw[r][c] = formula.clone();
        self.active_sheet = AddInSheetTab::Worksheet;
        self.active_cell_ref = format!("{}{}", col_to_a1(c), r + 1);
        self.formula_bar_text = formula;
        self.recalculate_worksheet(db);
        self.status_msg = format!("Fórmula insertada en Worksheet {}.", self.active_cell_ref);
    }

    pub fn build_worksheet_export_sheet(&self) -> ExportSheet {
        let mut cells = Vec::new();
        for r in 0..WS_ROWS {
            for c in 0..WS_COLS {
                let raw = self.ws_raw[r][c].trim();
                let ev = &self.ws_eval[r][c];
                if raw.is_empty() && is_blank_cell_val(ev) {
                    continue;
                }
                let style = if r == 0 {
                    ExportCellStyle::Header
                } else if raw.starts_with('=') {
                    ExportCellStyle::BoldBox
                } else {
                    ExportCellStyle::Default
                };
                let value = if raw.starts_with('=') {
                    ExportCellValue::Formula {
                        formula: raw.trim_start_matches('=').to_string(),
                        cached_number: ev.as_f64(),
                        cached_text: match ev {
                            CellEvalValue::Text(s) | CellEvalValue::Error(s) => Some(s.clone()),
                            _ => None,
                        },
                        array_rows: 1,
                    }
                } else {
                    match ev {
                        CellEvalValue::Number(n) => ExportCellValue::Number(*n),
                        _ => ExportCellValue::Text(raw.to_string()),
                    }
                };
                cells.push(ExportCell {
                    row: r,
                    col: c,
                    value,
                    style,
                });
            }
        }
        ExportSheet {
            name: "Worksheet".into(),
            cells,
            col_widths: vec![20.0, 16.0, 16.0, 16.0, 16.0, 16.0, 14.0, 14.0],
        }
    }

    fn export_all_workbook(&self, db: &ThermoDatabase, format_ext: &str) -> Result<String, String> {
        let dir = std::path::Path::new(self.export_dir.trim());
        std::fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear directorio: {e}"))?;
        let fname = format!("{}.{}", self.export_filename.trim(), format_ext);
        let full_path = dir.join(fname);

        let t_val = self.eq_temp_raw.trim().parse::<f64>().unwrap_or(1500.0);
        let p_val = self.eq_press_raw.trim().parse::<f64>().unwrap_or(1.0);
        let sheets = vec![
            self.build_worksheet_export_sheet(),
            SoryntelApplication::addin_streameq_sheet(
                db,
                &self.units,
                &self.eq_species,
                &self.eq_inputs,
                t_val,
                p_val,
            ),
            SoryntelApplication::addin_minerals_sheet(),
            SoryntelApplication::addin_sample_sheet(db, &self.units),
        ];

        let bytes = match format_ext {
            "xlsx" => SoryntelApplication::export_workbook_xlsx(&sheets),
            "ods" => SoryntelApplication::export_workbook_ods(&sheets),
            _ => return Err("Formato no soportado".into()),
        };
        crate::storage::write_atomic(&full_path, bytes).map_err(|e| format!("Error al escribir archivo: {e}"))?;
        Ok(full_path.to_string_lossy().to_string())
    }

    fn export_suite_connector(&self, kind: &str) -> Result<String, String> {
        let dir = std::path::Path::new(self.export_dir.trim());
        std::fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear directorio: {e}"))?;
        let (fname, bytes): (&str, Vec<u8>) = match kind {
            "oxt" => (
                "SoryntelHSC_Calc.oxt",
                SoryntelApplication::export_libreoffice_oxt(self.bridge_port),
            ),
            "onlyoffice" => (
                "SoryntelHSC_ONLYOFFICE.zip",
                SoryntelApplication::export_onlyoffice_plugin(self.bridge_port),
            ),
            "bas" => (
                "SoryntelHSC.bas",
                SoryntelApplication::export_excel_wps_vba(self.bridge_port).into_bytes(),
            ),
            _ => return Err("Conector de suite no reconocido".into()),
        };
        let full_path = dir.join(fname);
        crate::storage::write_atomic(&full_path, bytes)
            .map_err(|e| format!("Error escribiendo {fname}: {e}"))?;
        Ok(full_path.to_string_lossy().to_string())
    }

    fn export_active_csv(&self) -> Result<String, String> {
        let dir = std::path::Path::new(self.export_dir.trim());
        std::fs::create_dir_all(dir).map_err(|e| format!("No se pudo crear directorio: {e}"))?;
        let full_path = dir.join(format!("{}_Worksheet.csv", self.export_filename.trim()));
        let mut csv = String::new();
        for r in 0..WS_ROWS {
            let line: Vec<String> = (0..WS_COLS)
                .map(|c| {
                    let s = self.ws_eval[r][c].as_display_string(self.decimals);
                    if s.contains(',') || s.contains('"') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s
                    }
                })
                .collect();
            if line.iter().any(|x| !x.is_empty()) {
                csv.push_str(&line.join(","));
                csv.push('\n');
            }
        }
        crate::storage::write_atomic(&full_path, csv).map_err(|e| format!("Error al escribir CSV: {e}"))?;
        Ok(full_path.to_string_lossy().to_string())
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        self.ensure_initialized(db);
        if self.is_bridge_running() {
            self.pull_units_from_bridge_if_changed(db);
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }

        // Atajos de teclado dentro del módulo: Ctrl+Shift+Enter (Array StreamEQ) y F9 (Recalcular)
        let ctrl_shift_enter = ctx.input(|i| {
            i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Enter)
        });
        let f9_pressed = ctx.input(|i| i.key_pressed(egui::Key::F9));
        if ctrl_shift_enter {
            self.eq_formula_editing = false;
            self.eq_array_committed = true;
            self.recalculate_streameq(db);
            self.recalculate_worksheet(db);
            let last_row = 3 + self.eq_species.len();
            if self.active_sheet == AddInSheetTab::StreamEq {
                self.formula_bar_text = format!(
                    "{{=StreamEQ(A4:A{last_row}{}B4:B{last_row}{}C1{}C2)}}",
                    self.arg_separator.ch(),
                    self.arg_separator.ch(),
                    self.arg_separator.ch()
                );
            }
            self.status_msg = "Ctrl+Shift+Enter: Fórmula matricial {=StreamEQ(...)} confirmada.".into();
        } else if f9_pressed {
            self.recalculate_all(db);
            self.status_msg = "F9: Todas las hojas recalculadas.".into();
        }

        // Modales compartidos (Selector de especies, Change measurement units, Add-Ins Go / Browse)
        if self.show_species_picker {
            self.render_species_picker_window(ctx, db);
        }
        if self.show_units_dialog {
            self.render_units_modal_dialog(ctx, db);
        }
        if self.show_addins_go_dialog {
            self.render_addins_go_dialog(ctx, db);
        }
        if self.show_browse_dialog {
            self.render_browse_xll_dialog(ctx, db);
        }

        if self.show_file_backstage {
            self.render_file_backstage(ctx, db);
            return;
        }

        // 1) Barra superior estilo Excel 2013 + HSC Ribbon
        egui::TopBottomPanel::top("excel_addins_ribbon")
            .frame(egui::Frame::none().fill(EXCEL_RIBBON_BG))
            .show(ctx, |ui| {
                self.render_title_and_ribbon(ui, db);
                self.render_formula_bar(ui, db);
            });

        // 2) Barra inferior con pestañas de hojas + estado 100% interno
        egui::TopBottomPanel::bottom("excel_addins_sheet_bar")
            .frame(
                egui::Frame::none()
                    .fill(EXCEL_HEADER_BG)
                    .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                self.render_sheet_tabs_and_status(ui, db);
            });

        // 3) Contenido de la hoja activa
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::WHITE))
            .show(ctx, |ui| match self.active_sheet {
                AddInSheetTab::Worksheet => self.render_worksheet(ui, db),
                AddInSheetTab::StreamEq => self.render_streameq_sheet(ui, db),
                AddInSheetTab::Minerals => self.render_minerals_sheet(ui, db),
                AddInSheetTab::AddInSample => self.render_sample_sheet(ui, db),
                AddInSheetTab::FunctionsCatalog => self.render_functions_catalog(ui, db),
            });
    }

    fn render_title_and_ribbon(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        // Banda verde título + pestañas Excel 2013 (FILE / HOME / INSERT / PAGE LAYOUT / FORMULAS / ADD-INS)
        egui::Frame::none()
            .fill(EXCEL_GREEN)
            .inner_margin(egui::Margin::symmetric(10.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("HSC Excel Add-Ins — Soryntel Chemistry")
                            .strong()
                            .size(13.5)
                            .color(egui::Color32::WHITE),
                    );
                    ui.add_space(14.0);

                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("FILE")
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(EXCEL_GREEN_DARK)
                            .rounding(egui::Rounding::same(2.0)),
                        )
                        .clicked()
                    {
                        self.show_file_backstage = true;
                        self.ribbon_tab = AddInRibbonTab::File;
                    }

                    let tabs = [
                        (AddInRibbonTab::Home, "HOME"),
                        (AddInRibbonTab::Insert, "INSERT"),
                        (AddInRibbonTab::PageLayout, "PAGE LAYOUT"),
                        (AddInRibbonTab::Formulas, "FORMULAS"),
                        (AddInRibbonTab::AddIns, "ADD-INS"),
                    ];
                    for (tab, label) in tabs {
                        let active = self.ribbon_tab == tab;
                        let fill = if active {
                            EXCEL_RIBBON_BG
                        } else {
                            egui::Color32::TRANSPARENT
                        };
                        let fg = if active {
                            EXCEL_GREEN_DARK
                        } else {
                            egui::Color32::WHITE
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(label).strong().size(11.5).color(fg),
                                )
                                .fill(fill)
                                .rounding(egui::Rounding::same(2.0)),
                            )
                            .clicked()
                        {
                            self.ribbon_tab = tab;
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let bridge_on = self.is_bridge_running();
                        let calls = self
                            .bridge_handle
                            .as_ref()
                            .map(|h| h.requests_served())
                            .unwrap_or(0);
                        let b_label = if bridge_on {
                            format!("Bridge :{} ON ({calls})", self.bridge_port)
                        } else {
                            format!("Bridge :{} OFF", self.bridge_port)
                        };
                        let b_fill = if bridge_on {
                            egui::Color32::from_rgb(20, 83, 45)
                        } else {
                            egui::Color32::from_rgb(55, 65, 81)
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new(b_label)
                                        .size(10.5)
                                        .strong()
                                        .color(egui::Color32::WHITE),
                                )
                                .fill(b_fill)
                                .rounding(egui::Rounding::same(3.0)),
                            )
                            .on_hover_text("Activar/detener servidor HTTP local para Excel, LibreOffice Calc, ONLYOFFICE y WPS")
                            .clicked()
                        {
                            if bridge_on {
                                self.stop_bridge();
                            } else {
                                self.start_bridge(db);
                            }
                        }

                        ui.label(
                            egui::RichText::new(format!(
                                "{}  |  {}  |  {}   (DB: {} especies)",
                                format_temp_unit_display(&self.units.temp_unit),
                                self.units.energy_unit,
                                self.units.pressure_unit,
                                db.len()
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(225, 245, 232)),
                        );
                    });
                });
            });

        // Franja contextual del Ribbon según pestaña
        egui::Frame::none()
            .fill(EXCEL_RIBBON_BG)
            .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
            .inner_margin(egui::Margin::symmetric(10.0, 5.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| match self.ribbon_tab {
                    AddInRibbonTab::Home | AddInRibbonTab::File => {
                        if ui.button("Recalcular todo (F9)").clicked() {
                            self.recalculate_all(db);
                        }
                        if ui
                            .checkbox(&mut self.show_formulas, "Ver fórmulas")
                            .changed()
                        {}
                        ui.separator();
                        ui.label("Decimales:");
                        for d in [4usize, 6, 8] {
                            if ui.selectable_label(self.decimals == d, format!("{d}")).clicked() {
                                self.decimals = d;
                            }
                        }
                        ui.separator();
                        if ui.button("Cargar Demo Worksheet").clicked() {
                            populate_default_worksheet_demo(&mut self.ws_raw);
                            self.active_sheet = AddInSheetTab::Worksheet;
                            self.active_cell_ref = "C2".into();
                            self.formula_bar_text = self.ws_raw[1][2].clone();
                            self.recalculate_worksheet(db);
                        }
                        if ui.button("Plantilla StreamEQ en Worksheet").clicked() {
                            populate_streameq_worksheet_template(&mut self.ws_raw);
                            self.active_sheet = AddInSheetTab::Worksheet;
                            self.active_cell_ref = "C4".into();
                            self.formula_bar_text = self.ws_raw[3][2].clone();
                            self.recalculate_worksheet(db);
                        }
                        if ui.button("Limpiar Worksheet").clicked() {
                            for row in &mut self.ws_raw {
                                for cell in row {
                                    cell.clear();
                                }
                            }
                            self.active_sheet = AddInSheetTab::Worksheet;
                            self.formula_bar_text.clear();
                            self.recalculate_worksheet(db);
                        }
                    }
                    AddInRibbonTab::Insert => {
                        if ui.button("Insertar Función (fx)").clicked() {
                            self.active_sheet = AddInSheetTab::FunctionsCatalog;
                        }
                        if ui.button("Elegir Especie de BD...").clicked() {
                            self.picker_target = if self.active_sheet == AddInSheetTab::StreamEq {
                                PickerTarget::StreamEqAppendRow
                            } else {
                                PickerTarget::WorksheetActiveCell
                            };
                            self.show_species_picker = true;
                        }
                        if ui.button("Buscar Mineral (3,812)").clicked() {
                            self.active_sheet = AddInSheetTab::Minerals;
                        }
                    }
                    AddInRibbonTab::PageLayout => {
                        ui.label(egui::RichText::new("Vista & Hoja:").strong().color(TEXT_DARK));
                        ui.checkbox(&mut self.show_formulas, "Mostrar fórmulas en celdas");
                        ui.separator();
                        if ui.button("Exportar Libro (.xlsx / .ods)...").clicked() {
                            self.show_file_backstage = true;
                            self.file_section = FileBackstageSection::ExportReport;
                        }
                        if ui.button("Guardar Worksheet .csv").clicked() {
                            self.export_status = match self.export_active_csv() {
                                Ok(p) => {
                                    self.status_msg = format!("Guardado: {p}");
                                    format!("Guardado: {p}")
                                }
                                Err(e) => {
                                    self.status_msg = e.clone();
                                    e
                                }
                            };
                        }
                    }
                    AddInRibbonTab::Formulas => {
                        if ui.button("Recalcular Hoja (F9)").clicked() {
                            self.recalculate_all(db);
                        }
                        if ui.button("StreamEQ Matriz (Ctrl+Shift+Enter)").clicked() {
                            self.eq_formula_editing = false;
                            self.eq_array_committed = true;
                            self.recalculate_streameq(db);
                        }
                        ui.checkbox(&mut self.show_formulas, "Mostrar fórmulas");
                        ui.separator();
                        ui.label("Separador argumentos:");
                        if ui
                            .selectable_label(self.arg_separator == ArgSeparator::Semicolon, ";")
                            .clicked()
                        {
                            self.arg_separator = ArgSeparator::Semicolon;
                            self.recalculate_tester(db);
                        }
                        if ui
                            .selectable_label(self.arg_separator == ArgSeparator::Comma, ",")
                            .clicked()
                        {
                            self.arg_separator = ArgSeparator::Comma;
                            self.recalculate_tester(db);
                        }
                    }
                    AddInRibbonTab::AddIns => {
                        // Clon fiel de "Menu Commands -> HSC ▼ -> Change measurement units..."
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(235, 240, 237))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 198, 187)))
                            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.menu_button(
                                        egui::RichText::new("  HSC ▾  ")
                                            .strong()
                                            .size(12.0)
                                            .color(EXCEL_GREEN_DARK),
                                        |ui| {
                                            if ui.button(crate::i18n::text("common.035")).clicked() {
                                                self.pending_units = self.units.clone();
                                                self.show_units_dialog = true;
                                                ui.close_menu();
                                            }
                                            ui.separator();
                                            if ui.button("Recalculate Workbook (F9)").clicked() {
                                                self.recalculate_all(db);
                                                ui.close_menu();
                                            }
                                            if ui.button("StreamEQ Array Sheet (Fig. 4-6)").clicked() {
                                                self.active_sheet = AddInSheetTab::StreamEq;
                                                ui.close_menu();
                                            }
                                            if ui.button("Minerals Sample Sheet (Fig. 7)").clicked() {
                                                self.active_sheet = AddInSheetTab::Minerals;
                                                ui.close_menu();
                                            }
                                            ui.separator();
                                            if ui.button("Excel Add-Ins Manager (Go...)").clicked() {
                                                self.show_addins_go_dialog = true;
                                                ui.close_menu();
                                            }
                                            if ui.button("Cross-Suite Bridge & Packages (.oxt / ONLYOFFICE / VBA)").clicked() {
                                                self.show_file_backstage = true;
                                                self.file_section = FileBackstageSection::ExportReport;
                                                ui.close_menu();
                                            }
                                        },
                                    );
                                    ui.label(
                                        egui::RichText::new("Menu Commands")
                                            .size(9.5)
                                            .color(TEXT_MUTED),
                                    );
                                });
                            });

                        ui.separator();
                        if ui.button(crate::i18n::text("common.035")).clicked() {
                            self.pending_units = self.units.clone();
                            self.show_units_dialog = true;
                        }
                        egui::ComboBox::from_id_salt("ribbon_u_temp")
                            .width(56.0)
                            .selected_text(format_temp_unit_display(&self.units.temp_unit))
                            .show_ui(ui, |ui| {
                                for u in TEMP_UNITS {
                                    if ui
                                        .selectable_value(
                                            &mut self.units.temp_unit,
                                            (*u).to_string(),
                                            format_temp_unit_display(u),
                                        )
                                        .changed()
                                    {
                                        self.sync_sample_d2_e2_from_units(db);
                                        self.recalculate_all(db);
                                    }
                                }
                            });
                        egui::ComboBox::from_id_salt("ribbon_u_energy")
                            .width(62.0)
                            .selected_text(&self.units.energy_unit)
                            .show_ui(ui, |ui| {
                                for u in ENERGY_UNITS {
                                    if ui
                                        .selectable_value(
                                            &mut self.units.energy_unit,
                                            (*u).to_string(),
                                            *u,
                                        )
                                        .changed()
                                    {
                                        self.sync_sample_d2_e2_from_units(db);
                                        self.recalculate_all(db);
                                    }
                                }
                            });
                        egui::ComboBox::from_id_salt("ribbon_u_press")
                            .width(64.0)
                            .selected_text(&self.units.pressure_unit)
                            .show_ui(ui, |ui| {
                                for u in PRESS_UNITS {
                                    if ui
                                        .selectable_value(
                                            &mut self.units.pressure_unit,
                                            (*u).to_string(),
                                            *u,
                                        )
                                        .changed()
                                    {
                                        self.recalculate_all(db);
                                    }
                                }
                            });
                        ui.separator();
                        if ui.button("Add-Ins Options (Fig. 1-3)...").clicked() {
                            self.show_file_backstage = true;
                            self.file_section = FileBackstageSection::Options;
                            self.options_cat = OptionsCategory::AddIns;
                        }
                        if ui.button("Excel / Calc / ONLYOFFICE...").clicked() {
                            self.show_file_backstage = true;
                            self.file_section = FileBackstageSection::ExportReport;
                        }
                    }
                });
            });
    }

    fn render_formula_bar(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::Frame::none()
            .fill(egui::Color32::WHITE)
            .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let name_resp = ui.add_sized(
                        [76.0, 22.0],
                        egui::TextEdit::singleline(&mut self.active_cell_ref)
                            .font(egui::FontId::monospace(12.0)),
                    );
                    if name_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.load_cell_into_formula_bar();
                    }

                    if ui.small_button("✖").on_hover_text("Descartar").clicked() {
                        self.load_cell_into_formula_bar();
                    }
                    if ui.small_button("✔").on_hover_text("Aplicar (Enter)").clicked() {
                        self.apply_formula_bar(db);
                    }
                    if ui.small_button("fx").on_hover_text("Abrir catálogo de funciones").clicked() {
                        self.active_sheet = AddInSheetTab::FunctionsCatalog;
                    }

                    let bar_w = (ui.available_width() - 12.0).max(160.0);
                    let resp = ui.add_sized(
                        [bar_w, 22.0],
                        egui::TextEdit::singleline(&mut self.formula_bar_text)
                            .font(egui::FontId::monospace(12.0))
                            .hint_text("Valor o fórmula (=H(\"Fe\";25), =StreamEQ(...), =SUM(C2:C5), =StreamEQ!C1)"),
                    );
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.apply_formula_bar(db);
                    }
                });
            });
    }

    fn load_cell_into_formula_bar(&mut self) {
        let cell_ref = self.active_cell_ref.trim().to_ascii_uppercase();
        self.active_cell_ref = cell_ref.clone();
        let Some((r, c)) = parse_a1(&cell_ref) else {
            return;
        };
        self.formula_bar_text = match self.active_sheet {
            AddInSheetTab::Worksheet if r < WS_ROWS && c < WS_COLS => self.ws_array_label[r][c]
                .clone()
                .unwrap_or_else(|| self.ws_raw[r][c].clone()),
            AddInSheetTab::StreamEq => {
                if r == 0 && c == 2 {
                    self.eq_temp_raw.clone()
                } else if r == 1 && c == 2 {
                    self.eq_press_raw.clone()
                } else if r >= 3 {
                    let idx = r - 3;
                    match c {
                        0 => self.eq_species.get(idx).cloned().unwrap_or_default(),
                        1 => self.eq_inputs.get(idx).cloned().unwrap_or_default(),
                        2 => format!("{{=StreamEQ(A4:A{};B4:B{};C1;C2)}}", 3 + self.eq_species.len(), 3 + self.eq_species.len()),
                        _ => String::new(),
                    }
                } else {
                    String::new()
                }
            }
            AddInSheetTab::Minerals => self
                .min_cells
                .get(&(r, c))
                .map(|cell| cell.raw.clone())
                .unwrap_or_default(),
            AddInSheetTab::AddInSample => self
                .sample_cells
                .get(&(r, c))
                .map(|cell| cell.raw.clone())
                .unwrap_or_default(),
            AddInSheetTab::FunctionsCatalog => self.formula_preview.clone(),
            _ => String::new(),
        };
    }

    fn render_sheet_tabs_and_status(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        ui.horizontal(|ui| {
            let sheets = [
                (AddInSheetTab::Worksheet, "Worksheet (A1:H24)"),
                (AddInSheetTab::StreamEq, "StreamEQ"),
                (AddInSheetTab::Minerals, "Minerals"),
                (AddInSheetTab::AddInSample, "AddInSample"),
                (AddInSheetTab::FunctionsCatalog, "Functions (fx)"),
            ];
            for (tab, label) in sheets {
                let active = self.active_sheet == tab;
                let btn = egui::Button::new(
                    egui::RichText::new(label)
                        .size(11.5)
                        .strong()
                        .color(if active { EXCEL_GREEN_DARK } else { TEXT_DARK }),
                )
                .fill(if active {
                    egui::Color32::WHITE
                } else {
                    EXCEL_HEADER_BG
                })
                .stroke(if active {
                    egui::Stroke::new(1.5, EXCEL_GREEN)
                } else {
                    egui::Stroke::new(1.0, EXCEL_BORDER)
                });
                if ui.add(btn).clicked() {
                    self.active_sheet = tab;
                    if tab == AddInSheetTab::AddInSample {
                        self.recalculate_sample_sheet(db);
                    }
                    self.load_cell_into_formula_bar();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let enabled_packs = self.packs.iter().filter(|p| p.enabled).count();
                let bridge_tag = if self.is_bridge_running() {
                    format!("Bridge :{} ON", self.bridge_port)
                } else {
                    "Bridge OFF".into()
                };
                ui.label(
                    egui::RichText::new(format!(
                        "Integrado: 83 activas + 5 HSC-Sim ({enabled_packs}/{} paquetes)  |  {bridge_tag}  |  {}",
                        self.packs.len(),
                        self.status_msg
                    ))
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
            });
        });
    }

    // -------------------------------------------------------------------------
    // Hoja 1: Worksheet Editable (24x8 = A1:H24)
    // -------------------------------------------------------------------------
    fn render_worksheet(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::ScrollArea::both().show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Hoja de Cálculo Integrada (A1:H24)")
                                .strong()
                                .size(13.0)
                                .color(EXCEL_GREEN_DARK),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new(
                                "Clic selecciona la celda · Doble clic (o barra superior) edita valor o fórmula (=H, =Cp, =StreamEQ, =Mineral*, =SUM, =StreamEQ!C1)",
                            )
                            .size(11.0)
                            .color(TEXT_MUTED),
                        );
                    });
                    ui.add_space(4.0);

                    let row_w = 34.0;
                    let col_widths = [148.0, 118.0, 128.0, 128.0, 124.0, 120.0, 108.0, 108.0];

                    // Cabeceras A..H
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        header_cell(ui, "", row_w);
                        for (c, &w) in col_widths.iter().enumerate() {
                            header_cell(ui, &col_letter(c), w);
                        }
                    });

                    for r in 0..WS_ROWS {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            row_num_cell(ui, r + 1, row_w);

                            for (c, &w) in col_widths.iter().enumerate() {
                                let cell_name = format!("{}{}", col_letter(c), r + 1);
                                let selected = self.active_cell_ref.eq_ignore_ascii_case(&cell_name);
                                let is_editing = self.ws_editing == Some((r, c));
                                let raw = self.ws_raw[r][c].clone();
                                let eval = &self.ws_eval[r][c];
                                let is_spilled = self.ws_array_label[r][c].is_some();

                                let bg = if r == 0 && !raw.is_empty() {
                                    EXCEL_SECTION_BG
                                } else if is_spilled {
                                    egui::Color32::from_rgb(239, 246, 255)
                                } else if r % 2 == 1 {
                                    EXCEL_ALT_ROW_BG
                                } else {
                                    egui::Color32::WHITE
                                };

                                if is_editing {
                                    let mut commit_edit = false;
                                    widget_cell(ui, w, egui::Color32::WHITE, true, |ui| {
                                        let resp = ui.add_sized(
                                            [w - 6.0, CELL_H - 4.0],
                                            egui::TextEdit::singleline(&mut self.ws_raw[r][c])
                                                .font(egui::FontId::monospace(11.5)),
                                        );
                                        if self.ws_focus_inline {
                                            resp.request_focus();
                                            self.ws_focus_inline = false;
                                        }
                                        self.formula_bar_text = self.ws_raw[r][c].clone();
                                        if resp.lost_focus()
                                            || ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            commit_edit = true;
                                        }
                                    });
                                    if commit_edit {
                                        self.ws_editing = None;
                                        self.recalculate_worksheet(db);
                                        self.formula_bar_text = self.ws_raw[r][c].clone();
                                    }
                                } else {
                                    let display_txt = if self.show_formulas {
                                        self.ws_array_label[r][c]
                                            .clone()
                                            .unwrap_or_else(|| raw.clone())
                                    } else if raw.starts_with('=') || is_spilled {
                                        eval.as_display_string(self.decimals)
                                    } else {
                                        raw.clone()
                                    };
                                    let right_align = matches!(eval, CellEvalValue::Number(_))
                                        && !self.show_formulas
                                        && !(r == 0 && !raw.is_empty());
                                    let fg = if matches!(eval, CellEvalValue::Error(_)) {
                                        TEXT_ERR
                                    } else if is_spilled || raw.starts_with('=') {
                                        TEXT_BLUE
                                    } else {
                                        TEXT_DARK
                                    };
                                    let resp = text_cell(
                                        ui,
                                        &display_txt,
                                        w,
                                        bg,
                                        fg,
                                        right_align,
                                        r == 0,
                                        selected,
                                    );
                                    if resp.double_clicked() {
                                        self.active_cell_ref = cell_name;
                                        self.formula_bar_text = raw;
                                        self.ws_editing = Some((r, c));
                                        self.ws_focus_inline = true;
                                    } else if resp.clicked() {
                                        if selected {
                                            self.ws_editing = Some((r, c));
                                            self.ws_focus_inline = true;
                                        } else {
                                            self.active_cell_ref = cell_name;
                                            self.formula_bar_text = self.ws_array_label[r][c]
                                                .clone()
                                                .unwrap_or(raw);
                                        }
                                    }
                                }
                            }
                        });
                    }
                });
            });
            ui.add_space(10.0);
        });
    }

    // -------------------------------------------------------------------------
    // Hoja 2: StreamEQ (Cap. 30 Fig. 4-6)
    // -------------------------------------------------------------------------
    fn render_streameq_sheet(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::ScrollArea::both().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                // Columna izquierda: hoja A..E exacta a Fig. 4-6 (sin botones dentro de la rejilla)
                ui.vertical(|ui| {
                    let row_w = 34.0;
                    let cols = [155.0, 96.0, 128.0, 58.0, 62.0];
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        header_cell(ui, "", row_w);
                        for (i, w) in cols.iter().enumerate() {
                            header_cell(ui, &col_letter(i), *w);
                        }
                    });

                    let col_c_rect = |row_rect: egui::Rect| -> egui::Rect {
                        egui::Rect::from_min_size(
                            egui::pos2(row_rect.min.x + row_w + cols[0] + cols[1], row_rect.min.y),
                            egui::vec2(cols[2], CELL_H),
                        )
                    };

                    // Row 1: Temperature | "" | C1 | temp_unit
                    let td = format_temp_unit_display(&self.units.temp_unit);
                    let c1_sel = self.active_cell_ref == "C1";
                    let r1_rect = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            row_num_cell(ui, 1, row_w);
                            text_cell(ui, "Temperature", cols[0], egui::Color32::WHITE, TEXT_DARK, false, false, false);
                            text_cell(ui, "", cols[1], egui::Color32::WHITE, TEXT_DARK, false, false, false);
                            widget_cell(ui, cols[2], egui::Color32::WHITE, c1_sel, |ui| {
                                let r = ui.add_sized(
                                    [cols[2] - 6.0, CELL_H - 4.0],
                                    egui::TextEdit::singleline(&mut self.eq_temp_raw)
                                        .font(egui::FontId::monospace(11.5)),
                                );
                                if r.gained_focus() || r.clicked() {
                                    self.select_cell("C1", self.eq_temp_raw.clone());
                                }
                                if r.changed() {
                                    self.formula_bar_text = self.eq_temp_raw.clone();
                                    self.recalculate_streameq(db);
                                }
                            });
                            text_cell(ui, &td, cols[3], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                            text_cell(ui, "", cols[4], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                        })
                        .response
                        .rect;
                    let c1_rect = col_c_rect(r1_rect);

                    // Row 2: Pressure | "" | C2 | pressure_unit
                    let c2_sel = self.active_cell_ref == "C2";
                    let r2_rect = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            row_num_cell(ui, 2, row_w);
                            text_cell(ui, "Pressure", cols[0], egui::Color32::WHITE, TEXT_DARK, false, false, false);
                            text_cell(ui, "", cols[1], egui::Color32::WHITE, TEXT_DARK, false, false, false);
                            widget_cell(ui, cols[2], egui::Color32::WHITE, c2_sel, |ui| {
                                let r = ui.add_sized(
                                    [cols[2] - 6.0, CELL_H - 4.0],
                                    egui::TextEdit::singleline(&mut self.eq_press_raw)
                                        .font(egui::FontId::monospace(11.5)),
                                );
                                if r.gained_focus() || r.clicked() {
                                    self.select_cell("C2", self.eq_press_raw.clone());
                                }
                                if r.changed() {
                                    self.formula_bar_text = self.eq_press_raw.clone();
                                    self.recalculate_streameq(db);
                                }
                            });
                            text_cell(ui, &self.units.pressure_unit, cols[3], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                            text_cell(ui, "", cols[4], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                        })
                        .response
                        .rect;
                    let c2_rect = col_c_rect(r2_rect);

                    // Row 3: SPECIES: | Input | Output
                    let r3_rect = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            row_num_cell(ui, 3, row_w);
                            text_cell(ui, "SPECIES:", cols[0], egui::Color32::WHITE, TEXT_DARK, false, true, false);
                            text_cell(ui, "Input", cols[1], egui::Color32::WHITE, TEXT_DARK, true, true, false);
                            text_cell(ui, "Output", cols[2], egui::Color32::WHITE, TEXT_DARK, true, true, false);
                            text_cell(ui, "", cols[3], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                            text_cell(ui, "", cols[4], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                        })
                        .response
                        .rect;
                    // Línea negra horizontal debajo de SPECIES: | Input | Output (Fig. 4-6)
                    ui.painter().line_segment(
                        [
                            egui::pos2(r3_rect.min.x + row_w, r3_rect.max.y),
                            egui::pos2(
                                r3_rect.min.x + row_w + cols[0] + cols[1] + cols[2] + cols[3],
                                r3_rect.max.y,
                            ),
                        ],
                        egui::Stroke::new(1.5, TEXT_DARK),
                    );

                    let last_row = 3 + self.eq_species.len();
                    let sep = self.arg_separator.ch();
                    let plain_eq_formula =
                        format!("=StreamEQ(A4:A{last_row}{sep}B4:B{last_row}{sep}C1{sep}C2)");
                    let eq_formula = format!("{{{plain_eq_formula}}}");
                    let bar_formula = if self.eq_array_committed && !self.eq_formula_editing {
                        eq_formula.clone()
                    } else {
                        plain_eq_formula.clone()
                    };

                    let mut first_row_rect: Option<egui::Rect> = None;
                    let mut last_row_rect: Option<egui::Rect> = None;
                    let mut c4_rect: Option<egui::Rect> = None;

                    for i in 0..self.eq_species.len() {
                        let excel_r = i + 4;
                        let is_phase = self.eq_species[i].trim().starts_with('§')
                            || self.eq_species[i].trim().to_ascii_uppercase().starts_with("PHASE");
                        let a_ref = format!("A{excel_r}");
                        let b_ref = format!("B{excel_r}");
                        let c_ref = format!("C{excel_r}");
                        let a_sel = self.active_cell_ref == a_ref;
                        let b_sel = self.active_cell_ref == b_ref;
                        let row_bg = if self.eq_selected_row == Some(i) {
                            egui::Color32::from_rgb(245, 248, 252)
                        } else {
                            egui::Color32::WHITE
                        };

                        let out_val = self.eq_outputs.get(i).cloned().unwrap_or_else(blank_cell_val);
                        let (out_str, out_fg) = if self.eq_formula_editing || !self.eq_array_committed {
                            (String::new(), TEXT_DARK)
                        } else if self.show_formulas {
                            (eq_formula.clone(), TEXT_BLUE)
                        } else if matches!(out_val, CellEvalValue::Error(_)) {
                            (out_val.as_display_string(self.decimals.max(8)), TEXT_ERR)
                        } else {
                            (out_val.as_display_string(self.decimals.max(8)), TEXT_DARK)
                        };
                        let c_bg = if i == 0 {
                            egui::Color32::WHITE
                        } else if self.eq_formula_editing || !self.eq_array_committed {
                            egui::Color32::from_rgb(228, 232, 242)
                        } else {
                            egui::Color32::from_rgb(236, 239, 246)
                        };

                        let r_rect = ui
                            .horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                row_num_cell(ui, excel_r, row_w);
                                widget_cell(ui, cols[0], row_bg, a_sel, |ui| {
                                    let r = ui.add_sized(
                                        [cols[0] - 6.0, CELL_H - 4.0],
                                        egui::TextEdit::singleline(&mut self.eq_species[i])
                                            .font(egui::FontId::proportional(if is_phase { 11.8 } else { 11.5 })),
                                    );
                                    if r.gained_focus() || r.clicked() {
                                        self.eq_selected_row = Some(i);
                                        self.select_cell(&a_ref, self.eq_species[i].clone());
                                    }
                                    if r.changed() {
                                        self.formula_bar_text = self.eq_species[i].clone();
                                        self.recalculate_streameq(db);
                                    }
                                });
                                widget_cell(ui, cols[1], row_bg, b_sel, |ui| {
                                    let r = ui.add_sized(
                                        [cols[1] - 6.0, CELL_H - 4.0],
                                        egui::TextEdit::singleline(&mut self.eq_inputs[i])
                                            .font(egui::FontId::monospace(11.5)),
                                    );
                                    if r.gained_focus() || r.clicked() {
                                        self.eq_selected_row = Some(i);
                                        self.select_cell(&b_ref, self.eq_inputs[i].clone());
                                    }
                                    if r.changed() {
                                        self.formula_bar_text = self.eq_inputs[i].clone();
                                        self.recalculate_streameq(db);
                                    }
                                });
                                let cr = text_cell(
                                    ui,
                                    &out_str,
                                    cols[2],
                                    c_bg,
                                    out_fg,
                                    !self.show_formulas,
                                    false,
                                    false,
                                );
                                if i == 0 {
                                    c4_rect = Some(cr.rect);
                                }
                                if cr.clicked() {
                                    self.eq_selected_row = Some(i);
                                    self.select_cell(&c_ref, bar_formula.clone());
                                }
                                if cr.double_clicked() {
                                    self.eq_selected_row = Some(i);
                                    self.eq_formula_editing = true;
                                    self.select_cell("C4", plain_eq_formula.clone());
                                }
                                let u_txt = if is_phase { td.as_str() } else { "kmol" };
                                text_cell(ui, u_txt, cols[3], row_bg, TEXT_DARK, false, false, false);
                                text_cell(ui, "", cols[4], egui::Color32::WHITE, TEXT_MUTED, false, false, false);
                            })
                            .response
                            .rect;

                        if first_row_rect.is_none() {
                            first_row_rect = Some(r_rect);
                        }
                        last_row_rect = Some(r_rect);
                    }

                    // Marcos visuales exactos de Fig. 4, Fig. 5 y Fig. 6
                    if let (Some(r4), Some(rlast)) = (first_row_rect, last_row_rect) {
                        let p = ui.painter();
                        let ax0 = r4.min.x + row_w;
                        let bx0 = ax0 + cols[0];
                        let cx0 = bx0 + cols[1];

                        if self.eq_formula_editing && self.eq_show_range_highlights {
                            let a_rect = egui::Rect::from_min_max(
                                egui::pos2(ax0, r4.min.y),
                                egui::pos2(ax0 + cols[0], rlast.max.y),
                            );
                            let b_rect = egui::Rect::from_min_max(
                                egui::pos2(bx0, r4.min.y),
                                egui::pos2(bx0 + cols[1], rlast.max.y),
                            );
                            paint_range_outline(p, a_rect, REF_BLUE, 2.0, true, false);
                            paint_range_outline(p, b_rect, REF_GREEN, 2.0, true, false);
                            paint_range_outline(p, c1_rect, REF_PURPLE, 2.0, true, false);
                            paint_range_outline(p, c2_rect, REF_PURPLE, 2.0, true, false);
                        }

                        let c_block = egui::Rect::from_min_max(
                            egui::pos2(cx0, r4.min.y),
                            egui::pos2(cx0 + cols[2], rlast.max.y),
                        );
                        paint_range_outline(p, c_block, TEXT_DARK, 2.0, false, true);

                        if self.eq_formula_editing {
                            if let Some(c4r) = c4_rect {
                                paint_streameq_inline_formula(p, c4r, last_row, sep);
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("+ Fila especie").clicked() {
                            self.eq_species.push("H2O(g)".into());
                            self.eq_inputs.push("0".into());
                            self.recalculate_streameq(db);
                        }
                        if ui.button("+ Elegir desde BD...").clicked() {
                            self.picker_target = PickerTarget::StreamEqAppendRow;
                            self.show_species_picker = true;
                        }
                        if ui.button("+ § PHASE").clicked() {
                            self.eq_species.push("§ PHASE 3:".into());
                            self.eq_inputs.push("25".into());
                            self.recalculate_streameq(db);
                        }
                        if ui
                            .add_enabled(
                                self.eq_species.len() > 2,
                                egui::Button::new("- Quitar fila"),
                            )
                            .clicked()
                        {
                            let idx = self
                                .eq_selected_row
                                .filter(|&i| i < self.eq_species.len())
                                .unwrap_or(self.eq_species.len() - 1);
                            self.eq_species.remove(idx);
                            self.eq_inputs.remove(idx);
                            self.eq_selected_row = None;
                            self.recalculate_streameq(db);
                        }
                        if ui.button("Recalcular Matriz (Ctrl+Shift+Enter)").clicked() {
                            self.eq_formula_editing = false;
                            self.eq_array_committed = true;
                            self.recalculate_streameq(db);
                            self.select_cell("C4", eq_formula.clone());
                        }
                    });
                });

                ui.add_space(18.0);

                // Columna derecha: Flujo Fig. 4-6 + Preajustes + Balance elemental
                ui.vertical(|ui| {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("Flujo Matriz StreamEQ (Fig. 4–6)")
                                .strong()
                                .color(EXCEL_GREEN_DARK),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .selectable_label(
                                    !self.eq_array_committed && !self.eq_formula_editing,
                                    "Fig. 4: Selección",
                                )
                                .clicked()
                            {
                                self.eq_array_committed = false;
                                self.eq_formula_editing = false;
                                self.select_cell("C4", String::new());
                            }
                            if ui
                                .selectable_label(
                                    self.eq_formula_editing,
                                    "Fig. 5: Escribir =StreamEQ(...)",
                                )
                                .clicked()
                            {
                                let last_row = 3 + self.eq_species.len();
                                let sep = self.arg_separator.ch();
                                let plain = format!(
                                    "=StreamEQ(A4:A{last_row}{sep}B4:B{last_row}{sep}C1{sep}C2)"
                                );
                                self.eq_formula_editing = true;
                                self.eq_array_committed = false;
                                self.select_cell("C4", plain);
                            }
                            if ui
                                .selectable_label(
                                    self.eq_array_committed && !self.eq_formula_editing,
                                    "Fig. 6: Ctrl+Shift+Enter",
                                )
                                .clicked()
                            {
                                let last_row = 3 + self.eq_species.len();
                                let sep = self.arg_separator.ch();
                                let arr_f = format!(
                                    "{{=StreamEQ(A4:A{last_row}{sep}B4:B{last_row}{sep}C1{sep}C2)}}"
                                );
                                self.eq_formula_editing = false;
                                self.eq_array_committed = true;
                                self.recalculate_streameq(db);
                                self.select_cell("C4", arr_f);
                            }
                        });
                        ui.checkbox(
                            &mut self.eq_show_range_highlights,
                            "Resaltar referencias (Azul A, Verde B, Morado C1/C2)",
                        );
                    });

                    ui.add_space(6.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Preajustes StreamEQ").strong());
                        if ui.button("Combustión C + Aire (1500 °C, Fig. 4-6)").clicked() {
                            self.eq_temp_raw = "1500".into();
                            self.eq_press_raw = "1".into();
                            self.eq_species = vec![
                                "§ PHASE 1:".into(),
                                "CO(g)".into(),
                                "CO2(g)".into(),
                                "N(g)".into(),
                                "N2(g)".into(),
                                "NO(g)".into(),
                                "N2O3(g)".into(),
                                "N2O4(g)".into(),
                                "N2O5(g)".into(),
                                "O2(g)".into(),
                                "§ PHASE 2:".into(),
                                "C".into(),
                            ];
                            self.eq_inputs = vec![
                                "25".into(),
                                "0".into(),
                                "".into(),
                                "".into(),
                                "3.02".into(),
                                "".into(),
                                "".into(),
                                "".into(),
                                "".into(),
                                "0.8".into(),
                                "25".into(),
                                "1".into(),
                            ];
                            self.eq_formula_editing = false;
                            self.eq_array_committed = true;
                            self.recalculate_streameq(db);
                        }
                        if ui.button("Reformado CH4 + H2O (850 °C)").clicked() {
                            self.eq_temp_raw = "850".into();
                            self.eq_press_raw = "1".into();
                            self.eq_species = vec![
                                "§ PHASE 1:".into(),
                                "CH4(g)".into(),
                                "H2O(g)".into(),
                                "CO(g)".into(),
                                "CO2(g)".into(),
                                "H2(g)".into(),
                            ];
                            self.eq_inputs = vec![
                                "25".into(),
                                "1".into(),
                                "2".into(),
                                "0".into(),
                                "0".into(),
                                "0".into(),
                            ];
                            self.eq_formula_editing = false;
                            self.eq_array_committed = true;
                            self.recalculate_streameq(db);
                        }
                    });

                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Balance Elemental (kmol)").strong());
                        if self.eq_balance.is_empty() {
                            ui.label(egui::RichText::new("Sin datos válidos.").color(TEXT_MUTED));
                        } else {
                            egui::Grid::new("streameq_bal_grid")
                                .striped(true)
                                .show(ui, |ui| {
                                    ui.strong("Elem");
                                    ui.strong("Input");
                                    ui.strong("Output");
                                    ui.strong("Δ");
                                    ui.end_row();
                                    for (el, inp, out) in &self.eq_balance {
                                        let diff = out - inp;
                                        ui.label(el);
                                        ui.monospace(format!("{inp:.6}"));
                                        ui.monospace(format!("{out:.6}"));
                                        ui.monospace(format!("{diff:+.2E}"));
                                        ui.end_row();
                                    }
                                });
                        }
                    });
                });
            });
        });
    }

    // -------------------------------------------------------------------------
    // Hoja 3: Minerals (Cap. 30 Fig. 7 + Buscador 3,812 minerales)
    // -------------------------------------------------------------------------
    fn apply_grid_inline_commit(
        &mut self,
        db: &ThermoDatabase,
        row: usize,
        col: usize,
        val: String,
        is_minerals: bool,
    ) {
        let cname = format!("{}{}", col_to_a1(col), row + 1);
        let is_f = val.trim().starts_with('=') || val.trim().starts_with("{=");
        let map = if is_minerals {
            &mut self.min_cells
        } else {
            &mut self.sample_cells
        };
        let prev_style = map
            .get(&(row, col))
            .map(|x| x.style)
            .unwrap_or(ExportCellStyle::Default);
        let eval = if is_f {
            blank_cell_val()
        } else if let Ok(n) = val.trim().parse::<f64>() {
            CellEvalValue::Number(n)
        } else {
            CellEvalValue::Text(val.clone())
        };
        map.insert(
            (row, col),
            SheetCell {
                raw: val.clone(),
                eval,
                style: prev_style,
                is_formula: is_f,
            },
        );

        if is_minerals {
            self.min_max_row = self.min_max_row.max(row);
            self.min_max_col = self.min_max_col.max(col);
            self.min_paste_values = false;
            self.recalculate_minerals(db);
            if !is_f {
                self.min_selected = SoryntelApplication::lookup_addin_mineral(&val)
                    .or_else(|| self.min_selected.clone());
            }
            self.status_msg = format!("Minerals {cname} actualizado y recalculado.");
        } else {
            self.sample_max_row = self.sample_max_row.max(row);
            self.sample_max_col = self.sample_max_col.max(col);
            self.recalculate_sample_sheet(db);
            if (row == 1 && (col == 3 || col == 4)) && !is_f {
                self.recalculate_all(db);
            }
            self.status_msg = format!("AddInSample {cname} actualizado.");
        }
        self.select_cell(&cname, val);
    }

    fn render_minerals_sheet(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        const CMD_DOCS: &[(&str, &str, &str)] = &[
            (
                "=MineralCode(B4)",
                "=MineralCode(\"Pyrite\")",
                "Gives standard HSC mineral code for a mineral (1-3 letters)",
            ),
            (
                "=MineralFormula(B5)",
                "=MineralFormula(\"Galena\")",
                "Gives general formula of mineral",
            ),
            (
                "=MineralName(B7)",
                "=MineralName(\"Cc\")",
                "Gives mineral name according to mineral code",
            ),
            (
                "=MineralC(B4;A8)",
                "=MineralC(\"Pyrite\";\"S\")",
                "Gives wt% of element in mineral",
            ),
            (
                "=MineralC(B$6;$A11)",
                "=MineralC(\"Galena\";\"S\")",
                "Gives wt% of element in mineral",
            ),
            (
                "=MineralC(B$6;$A12)",
                "=MineralC(\"Py\";\"S\")",
                "Gives wt% of element in mineral",
            ),
            (
                "=MineralC(B$6;$A13)",
                "=MineralC(\"Kfs\";\"Al\")",
                "Gives wt% of element in mineral",
            ),
            (
                "=MineralProperty(B$6;$A14)",
                "=MineralProperty(\"Py\";\"Density\")",
                "Returns property of given mineral",
            ),
        ];
        const PROP_LIST: &[&str] = &[
            "-Code",
            "-Formula",
            "-Density",
            "-Hardness",
            "-Color",
            "-Luster",
            "- any chemical element (Cu, Zn, Pb, Si, O — elemental form, NOT oxide)",
            "-Location",
            "-ID",
            "-Elements",
            "-Database",
            "-Reference",
        ];

        let mut try_formula: Option<String> = None;

        egui::ScrollArea::both().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal_top(|ui| {
                ui.add_space(10.0);
                // Izquierda: Tabla Fig. 7 + edición inline + YellowNotes
                ui.vertical(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut self.min_paste_values, "Paste Values (congelar fórmulas)");
                        ui.separator();
                        ui.label("Buscar mineral (3,812):");
                        if ui.text_edit_singleline(&mut self.min_search_query).changed() {
                            self.min_selected =
                                SoryntelApplication::lookup_addin_mineral(&self.min_search_query);
                        }
                        if ui.button("Recalcular").clicked() {
                            self.min_paste_values = false;
                            self.recalculate_minerals(db);
                        }
                        ui.label(
                            egui::RichText::new("Doble clic en Mineral 1..4 (Pyrite, Py, Py\\Joki, ID:12616) para editar")
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    });

                    if let Some(rec) = &self.min_selected {
                        egui::Frame::none()
                            .fill(EXCEL_ALT_ROW_BG)
                            .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                            .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.strong(format!("{} ({})", rec.name, rec.code));
                                    ui.label(format!("| Fórmula: {}", rec.formula));
                                    ui.label(format!("| Grupo: {}", rec.group));
                                    if let Some(d) = rec.density {
                                        ui.label(format!("| ρ = {d:.2} g/cm³"));
                                    }
                                    if let Some(h) = rec.hardness {
                                        ui.label(format!("| Mohs = {h:.1}"));
                                    }
                                    ui.label(format!("| Composición: {}", rec.elements_str));
                                });
                            });
                    }

                    ui.add_space(6.0);
                    let col_count = (self.min_max_col + 1).clamp(6, 12);
                    let mut inline_state = GridInlineState {
                        editing: &mut self.min_editing,
                        buf: &mut self.min_edit_buf,
                        focus: &mut self.min_focus_inline,
                    };
                    if let Some(action) = render_export_cells_grid(
                        ui,
                        &self.min_cells,
                        self.min_max_row,
                        col_count,
                        &self.active_cell_ref,
                        self.show_formulas,
                        self.decimals,
                        false,
                        true,
                        &mut inline_state,
                    ) {
                        match action {
                            ExportGridAction::Selected { coord, raw } => {
                                self.select_cell(&coord, raw);
                            }
                            ExportGridAction::Committed { row, col, new_raw } => {
                                self.apply_grid_inline_commit(db, row, col, new_raw, true);
                            }
                        }
                    }

                    ui.add_space(6.0);
                    egui::Frame::none()
                        .fill(EXCEL_YELLOW_BG)
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(180, 160, 80)))
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_max_width(590.0);
                            ui.label(egui::RichText::new("You can get mineral properties by using:").strong().size(11.0));
                            ui.label(egui::RichText::new("• Mineral name (e.g. Pyrite)  |  • Mineral Code (e.g. Py)").size(11.0));
                            ui.label(egui::RichText::new("• Mineral Code AND part of Location (e.g. Py\\Joki) -> first matching").size(11.0));
                            ui.label(egui::RichText::new("• Mineral ID (e.g. ID:54 / ID:12616)  |  • Col F: Paste-values check").size(11.0));
                        });
                });

                ui.add_space(16.0);

                // Derecha: Panel de referencia fiel a Cap. 30 Fig. 7
                ui.vertical(|ui| {
                    ui.set_min_width(470.0);
                    ui.label(
                        egui::RichText::new("Example of using HSC Chemistry Add-in functions to create a table of minerals")
                            .strong()
                            .size(12.5)
                            .color(TEXT_DARK),
                    );
                    ui.label(
                        egui::RichText::new("Referencia oficial Cap. 30 Fig. 7 (clic en 'Probar' evalúa en Worksheet)")
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(4.0);

                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            egui::Grid::new("fig7_commands_ref_grid")
                                .striped(true)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.strong("HSC Command");
                                    ui.strong("Code example");
                                    ui.strong("Explanation");
                                    ui.strong("");
                                    ui.end_row();
                                    for &(cmd, ex, expl) in CMD_DOCS {
                                        ui.monospace(egui::RichText::new(cmd).size(11.0).color(TEXT_BLUE));
                                        ui.monospace(egui::RichText::new(ex).size(11.0));
                                        ui.label(egui::RichText::new(expl).size(11.0).color(TEXT_MUTED));
                                        if ui.small_button("Probar").clicked() {
                                            try_formula = Some(ex.to_string());
                                        }
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("General HSC Command MineralProperty(MineralName; Property)")
                                .strong()
                                .size(11.5),
                        );
                        ui.label(
                            egui::RichText::new("Returns the property of given mineral. Property can be any of:")
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                        for &prop in PROP_LIST {
                            ui.label(egui::RichText::new(prop).size(11.0));
                        }
                    });
                });
            });
        });

        if let Some(f) = try_formula {
            self.insert_formula_into_worksheet(db, f);
        }
    }

    // -------------------------------------------------------------------------
    // Hoja 4: AddInSample (con coordenadas A..I y filas reales)
    // -------------------------------------------------------------------------
    fn render_sample_sheet(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        egui::ScrollArea::both().show(ui, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("AddInSample (Coordenadas reales =UNITS(D2;E2), =H(D8;E8), doble clic edita)")
                                .strong()
                                .color(EXCEL_GREEN_DARK),
                        );
                        if ui.button("Restaurar AddInSample").clicked() {
                            let sample = SoryntelApplication::addin_sample_sheet(db, &self.units);
                            let (mr, mc) =
                                load_export_sheet_into_cells(&sample, &mut self.sample_cells);
                            self.sample_max_row = mr;
                            self.sample_max_col = mc;
                            self.sample_editing = None;
                            self.recalculate_sample_sheet(db);
                        }
                    });
                    ui.add_space(4.0);

                    let col_count = (self.sample_max_col + 1).clamp(8, 12);
                    let mut inline_state = GridInlineState {
                        editing: &mut self.sample_editing,
                        buf: &mut self.sample_edit_buf,
                        focus: &mut self.sample_focus_inline,
                    };
                    if let Some(action) = render_export_cells_grid(
                        ui,
                        &self.sample_cells,
                        self.sample_max_row,
                        col_count,
                        &self.active_cell_ref,
                        self.show_formulas,
                        self.decimals,
                        true,
                        false,
                        &mut inline_state,
                    ) {
                        match action {
                            ExportGridAction::Selected { coord, raw } => {
                                self.select_cell(&coord, raw);
                            }
                            ExportGridAction::Committed { row, col, new_raw } => {
                                self.apply_grid_inline_commit(db, row, col, new_raw, false);
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("Scratchpad sobre celdas de AddInSample (clic y editar en la barra superior)")
                                .strong(),
                        );
                        for i in 0..self.scratch_rows.len() {
                            let sref = format!("SCRATCH{}", i + 1);
                            let selected = self.active_cell_ref == sref;
                            ui.horizontal(|ui| {
                                let label_r = ui.selectable_label(selected, &sref);
                                if label_r.clicked() {
                                    self.select_cell(&sref, self.scratch_rows[i].formula.clone());
                                }
                                let resp = ui.add_sized(
                                    [320.0, 22.0],
                                    egui::TextEdit::singleline(&mut self.scratch_rows[i].formula)
                                        .font(egui::FontId::monospace(11.5)),
                                );
                                if resp.gained_focus() || resp.clicked() {
                                    self.select_cell(&sref, self.scratch_rows[i].formula.clone());
                                }
                                if resp.changed() {
                                    self.formula_bar_text = self.scratch_rows[i].formula.clone();
                                    self.recalculate_scratch_rows(db);
                                }
                                let out = self.scratch_rows[i].result.as_display_string(self.decimals);
                                ui.monospace(format!("=>  {out}"));
                            });
                        }
                    });
                });
            });
        });
    }

    // -------------------------------------------------------------------------
    // Hoja 5: Functions (fx) Catalog & Live Tester (88 funciones)
    // -------------------------------------------------------------------------
    fn render_functions_catalog(&mut self, ui: &mut egui::Ui, db: &ThermoDatabase) {
        let mut selected_clicked: Option<(usize, AddInFunctionMeta)> = None;

        ui.horizontal_top(|ui| {
            // Panel izquierdo: filtro + lista
            ui.allocate_ui(egui::vec2(315.0, ui.available_height()), |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label("Buscar:");
                    ui.add_sized([195.0, 22.0], egui::TextEdit::singleline(&mut self.fn_search));
                });
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label("Categoría:");
                    let groups = [
                        "All",
                        "General",
                        "Species / Reaction",
                        "Iteration (Reverse)",
                        "Temp. Independent",
                        "Cp-Function",
                        "Percentage",
                        "Stream / Flow",
                        "Stream Reverse",
                        "Equilibrium Array",
                        "Mineralogy",
                        "HSC-Sim Mineral",
                    ];
                    egui::ComboBox::from_id_salt("fx_group_filter")
                        .width(195.0)
                        .selected_text(&self.fn_group_filter)
                        .show_ui(ui, |ui| {
                            for g in groups {
                                ui.selectable_value(&mut self.fn_group_filter, g.to_string(), g);
                            }
                        });
                });
                ui.separator();

                let q = self.fn_search.trim().to_ascii_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, meta) in ADDIN_FUNCTION_CATALOG.iter().enumerate() {
                        if self.fn_group_filter != "All" && meta.group != self.fn_group_filter {
                            continue;
                        }
                        if !q.is_empty()
                            && !meta.name.to_ascii_lowercase().contains(&q)
                            && !meta.description.to_ascii_lowercase().contains(&q)
                        {
                            continue;
                        }
                        let active = idx == self.selected_fn_idx;
                        let enabled = self.is_pack_enabled(meta.origin);
                        let label = if enabled {
                            format!("{:>2}. {}  [{}]", meta.index, meta.name, meta.group)
                        } else {
                            format!("{:>2}. {}  (Desactivada)", meta.index, meta.name)
                        };
                        if ui.selectable_label(active, label).clicked() {
                            selected_clicked = Some((idx, *meta));
                        }
                    }
                });
            });

            if let Some((idx, meta)) = selected_clicked {
                self.selected_fn_idx = idx;
                self.populate_tester_from_example(meta, db);
            }

            ui.separator();

            // Panel derecho: detalle + probador + inserción en Worksheet
            ui.vertical(|ui| {
                ui.add_space(8.0);
                if let Some(meta) = ADDIN_FUNCTION_CATALOG.get(self.selected_fn_idx).copied() {
                    ui.heading(format!("={}()", meta.name));
                    ui.label(
                        egui::RichText::new(format!(
                            "Origen: {}  |  Grupo: {}  |  Export C: {}  |  XLL: {}",
                            meta.origin.label(),
                            meta.group,
                            meta.c_export,
                            meta.xll_sig
                        ))
                        .size(11.5)
                        .color(TEXT_MUTED),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(meta.syntax).monospace().strong());
                    ui.label(meta.description);
                    ui.label(
                        egui::RichText::new(format!("Ejemplo manual: {}", meta.example))
                            .size(11.5)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(8.0);

                    ui.group(|ui| {
                        ui.label(egui::RichText::new("Probador en vivo dentro de Soryntel").strong());
                        let mut changed = false;
                        for i in 0..5 {
                            ui.horizontal(|ui| {
                                ui.label(format!("Arg {}:", i + 1));
                                let r = ui.add_sized(
                                    [320.0, 22.0],
                                    egui::TextEdit::singleline(&mut self.test_args[i])
                                        .font(egui::FontId::monospace(11.5)),
                                );
                                if r.changed() {
                                    changed = true;
                                }
                            });
                        }
                        if changed {
                            self.recalculate_tester(db);
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button("Recalcular").clicked() {
                                self.recalculate_tester(db);
                            }
                            if ui.button("Restaurar ejemplo").clicked() {
                                self.populate_tester_from_example(meta, db);
                            }
                            if ui.button("Insertar en celda activa del Worksheet").clicked() {
                                self.recalculate_tester(db);
                                let f = self.formula_preview.clone();
                                self.insert_formula_into_worksheet(db, f);
                            }
                            if ui.button("Copiar fórmula").clicked() {
                                ui.output_mut(|o| o.copied_text = self.formula_preview.clone());
                                self.status_msg = format!("Copiada {}", self.formula_preview);
                            }
                            if meta.name.eq_ignore_ascii_case("UNITS")
                                && ui.button("Aplicar unidades a la app").clicked()
                            {
                                self.recalculate_tester(db);
                                self.sync_sample_d2_e2_from_units(db);
                                self.recalculate_all(db);
                            }
                        });

                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!("Fórmula: {}", self.formula_preview))
                                .monospace()
                                .color(TEXT_BLUE),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Resultado: {}",
                                self.test_result.as_multiline_string(self.decimals)
                            ))
                            .monospace()
                            .size(13.0)
                            .strong()
                            .color(if matches!(self.test_result, CellEvalValue::Error(_)) {
                                TEXT_ERR
                            } else {
                                EXCEL_GREEN_DARK
                            }),
                        );
                    });
                }
            });
        });
    }

    // -------------------------------------------------------------------------
    // Modal: Elegir especie desde la Base de Datos Activa
    // -------------------------------------------------------------------------
    fn render_species_picker_window(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_species_picker;
        let mut picked: Option<String> = None;
        egui::Window::new("Elegir Especie de la Base de Datos")
            .open(&mut open)
            .default_size([380.0, 360.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Buscar:");
                    ui.text_edit_singleline(&mut self.picker_query);
                });
                ui.separator();
                let hits = db.search(&self.picker_query, 40);
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for sp in hits {
                        let lbl = format!("{}  ({:?}, M={:.3})", sp.formula, sp.phase, sp.molecular_weight);
                        if ui.selectable_label(false, lbl).clicked() {
                            picked = Some(sp.formula);
                        }
                    }
                });
            });

        self.show_species_picker = open;
        if let Some(sp_name) = picked {
            match self.picker_target {
                PickerTarget::StreamEqAppendRow => {
                    self.eq_species.push(sp_name.clone());
                    self.eq_inputs.push("1".into());
                    self.active_sheet = AddInSheetTab::StreamEq;
                    self.recalculate_streameq(db);
                    self.status_msg = format!("Añadida especie '{sp_name}' a StreamEQ.");
                }
                PickerTarget::WorksheetActiveCell => {
                    let (r, c) = parse_a1(&self.active_cell_ref)
                        .filter(|&(r, c)| r < WS_ROWS && c < WS_COLS)
                        .unwrap_or((1, 0));
                    self.ws_raw[r][c] = sp_name.clone();
                    self.formula_bar_text = sp_name.clone();
                    self.active_sheet = AddInSheetTab::Worksheet;
                    self.recalculate_worksheet(db);
                    self.status_msg = format!("Insertada '{sp_name}' en {}.", self.active_cell_ref);
                }
            }
            self.show_species_picker = false;
        }
    }

    // -------------------------------------------------------------------------
    // Modal: Change measurement units... (Cap. 30 Sección 30.7)
    // -------------------------------------------------------------------------
    fn render_units_modal_dialog(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_units_dialog;
        let mut apply = false;
        let mut cancel = false;

        egui::Window::new(crate::i18n::text("common.035"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("HSC Add-In Global Measurement Units")
                        .strong()
                        .size(13.0)
                        .color(EXCEL_GREEN_DARK),
                );
                ui.separator();

                egui::Grid::new("hsc_units_modal_grid")
                    .num_columns(2)
                    .spacing([14.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Temperature unit:");
                        egui::ComboBox::from_id_salt("modal_unit_temp")
                            .selected_text(format_temp_unit_display(&self.pending_units.temp_unit))
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for &u in TEMP_UNITS {
                                    ui.selectable_value(
                                        &mut self.pending_units.temp_unit,
                                        u.to_string(),
                                        format_temp_unit_display(u),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Energy unit:");
                        egui::ComboBox::from_id_salt("modal_unit_energy")
                            .selected_text(&self.pending_units.energy_unit)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for &u in ENERGY_UNITS {
                                    ui.selectable_value(
                                        &mut self.pending_units.energy_unit,
                                        u.to_string(),
                                        u,
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Pressure unit:");
                        egui::ComboBox::from_id_salt("modal_unit_press")
                            .selected_text(&self.pending_units.pressure_unit)
                            .width(150.0)
                            .show_ui(ui, |ui| {
                                for &u in PRESS_UNITS {
                                    ui.selectable_value(
                                        &mut self.pending_units.pressure_unit,
                                        u.to_string(),
                                        u,
                                    );
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "After changing the measurement units, the workbook is automatically recalculated.",
                    )
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        cancel = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(crate::i18n::text("common.023"))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(EXCEL_GREEN),
                        )
                        .clicked()
                    {
                        apply = true;
                    }
                });
            });

        if !open || cancel {
            self.show_units_dialog = false;
            return;
        }
        if apply {
            self.units = self.pending_units.clone();
            self.sync_sample_d2_e2_from_units(db);
            self.recalculate_all(db);
            self.status_msg = format!(
                "Unidades aplicadas y libro recalculado: {} / {} / {}.",
                format_temp_unit_display(&self.units.temp_unit),
                self.units.energy_unit,
                self.units.pressure_unit
            );
            self.show_units_dialog = false;
        }
    }

    // -------------------------------------------------------------------------
    // Modal: Excel Add-Ins Go... (Cap. 30 Fig. 3)
    // -------------------------------------------------------------------------
    fn render_addins_go_dialog(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        let mut open = self.show_addins_go_dialog;
        let mut changed = false;
        let mut close_ok = false;
        let mut open_browse = false;

        egui::Window::new("Add-Ins")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(450.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_min_width(310.0);
                        ui.label(egui::RichText::new("Add-Ins available:").strong());
                        egui::Frame::none()
                            .fill(egui::Color32::WHITE)
                            .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                            .inner_margin(6.0)
                            .show(ui, |ui| {
                                for (idx, pack) in self.packs.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut pack.enabled, "").changed() {
                                            changed = true;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.selected_pack_idx == idx,
                                                pack.name,
                                            )
                                            .clicked()
                                        {
                                            self.selected_pack_idx = idx;
                                        }
                                    });
                                }
                            });

                        ui.add_space(6.0);
                        if let Some(p) = self.packs.get(self.selected_pack_idx) {
                            ui.label(egui::RichText::new(p.module_ref).size(11.0).color(TEXT_BLUE));
                            ui.label(egui::RichText::new(p.description).size(11.0).color(TEXT_MUTED));
                        }
                    });

                    ui.vertical(|ui| {
                        if ui.add_sized([96.0, 26.0], egui::Button::new("OK")).clicked() {
                            close_ok = true;
                        }
                        if ui.add_sized([96.0, 26.0], egui::Button::new("Cancel")).clicked() {
                            open_browse = false;
                            close_ok = true;
                        }
                        ui.add_space(6.0);
                        if ui
                            .add_sized([96.0, 26.0], egui::Button::new("Browse..."))
                            .clicked()
                        {
                            open_browse = true;
                        }
                    });
                });
            });

        if changed {
            self.recalculate_all(db);
        }
        if open_browse {
            self.show_browse_dialog = true;
        }
        if !open || close_ok {
            self.show_addins_go_dialog = false;
        }
    }

    // -------------------------------------------------------------------------
    // Modal: Browse HSC-Excel.xll / Cross-Suite Connectors (Cap. 30 Fig. 3)
    // -------------------------------------------------------------------------
    fn render_browse_xll_dialog(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        const FILES: &[(&str, &str, &str, &str)] = &[
            (
                "HSC10-Excel.xll",
                "15.11.2026 14:00",
                "Microsoft Excel XLL Add-In (83 funciones activas)",
                "xll",
            ),
            (
                "HSC9-Excel.xll",
                "16.12.2015 12:31",
                "Microsoft Excel XLL Add-In Legacy",
                "xll",
            ),
            (
                "HSC6.XLL",
                "01.10.2006 09:00",
                "Microsoft Excel XLL Core (69 funciones)",
                "xll",
            ),
            (
                "SoryntelHSC_Calc.oxt",
                "Cross-Suite",
                "LibreOffice Calc Add-In Package (83 funciones)",
                "oxt",
            ),
            (
                "SoryntelHSC_ONLYOFFICE.zip",
                "Cross-Suite",
                "ONLYOFFICE Plugin JS (83 funciones)",
                "onlyoffice",
            ),
            (
                "SoryntelHSC.bas",
                "Cross-Suite",
                "Excel / WPS Office VBA Module (83 funciones)",
                "bas",
            ),
        ];

        let mut open = self.show_browse_dialog;
        let mut confirm = false;
        let mut cancel = false;
        let sel = self.browse_selected_file.min(FILES.len() - 1);

        egui::Window::new(crate::i18n::text("common.115"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(540.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Ubicación:").strong());
                    ui.monospace("HSC10  >  Programs  >  Soryntel Internal & Cross-Suite");
                });
                ui.separator();

                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        egui::Grid::new("browse_xll_files_grid")
                            .num_columns(3)
                            .striped(true)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong("Name");
                                ui.strong("Date modified");
                                ui.strong("Type");
                                ui.end_row();
                                for (idx, &(fname, date, ftype, _)) in FILES.iter().enumerate() {
                                    let resp = ui.selectable_label(sel == idx, fname);
                                    if resp.clicked() {
                                        self.browse_selected_file = idx;
                                    }
                                    if resp.double_clicked() {
                                        self.browse_selected_file = idx;
                                        confirm = true;
                                    }
                                    ui.label(egui::RichText::new(date).size(11.0).color(TEXT_MUTED));
                                    ui.label(egui::RichText::new(ftype).size(11.0).color(TEXT_MUTED));
                                    ui.end_row();
                                }
                            });
                    });

                ui.add_space(6.0);
                let active_file = FILES[self.browse_selected_file.min(FILES.len() - 1)];
                ui.horizontal(|ui| {
                    ui.label("File name:");
                    ui.monospace(active_file.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label("Add-Ins (*.xlam;*.xla;*.xll;*.oxt;*.zip;*.bas)");
                    });
                });
                ui.separator();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        cancel = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(crate::i18n::text("common.023"))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            )
                            .fill(EXCEL_GREEN),
                        )
                        .clicked()
                    {
                        confirm = true;
                    }
                });
            });

        if !open || cancel {
            self.show_browse_dialog = false;
            return;
        }
        if confirm {
            let (fname, _, _, kind) = FILES[self.browse_selected_file.min(FILES.len() - 1)];
            if kind == "xll" {
                for p in &mut self.packs {
                    p.enabled = true;
                }
                self.recalculate_all(db);
                self.status_msg = format!(
                    "Activado complemento interno compatible con '{fname}' (83 funciones activas en Soryntel)."
                );
            } else {
                if !self.is_bridge_running() {
                    self.start_bridge(db);
                }
                match self.export_suite_connector(kind) {
                    Ok(path) => {
                        self.export_status = format!("Exportado: {path}");
                        self.status_msg = format!(
                            "Conector '{fname}' generado en {path} (Bridge :{} activo).",
                            self.bridge_port
                        );
                    }
                    Err(err) => {
                        self.export_status = err.clone();
                        self.status_msg = err;
                    }
                }
            }
            self.show_browse_dialog = false;
            self.show_addins_go_dialog = false;
        }
    }

    // -------------------------------------------------------------------------
    // FILE Backstage (Options Fig. 1-3, Open/Samples, Cross-Suite Bridge & Export)
    // -------------------------------------------------------------------------
    fn render_file_backstage(&mut self, ctx: &egui::Context, db: &ThermoDatabase) {
        egui::SidePanel::left("file_backstage_nav")
            .exact_width(195.0)
            .frame(egui::Frame::none().fill(EXCEL_GREEN).inner_margin(10.0))
            .show(ctx, |ui| {
                if ui
                    .button(egui::RichText::new("← Volver a la hoja").color(egui::Color32::WHITE))
                    .clicked()
                {
                    self.show_file_backstage = false;
                    self.ribbon_tab = AddInRibbonTab::Home;
                }
                ui.add_space(12.0);
                let items = [
                    (FileBackstageSection::Info, "Info"),
                    (FileBackstageSection::OpenSamples, "Open / Templates"),
                    (FileBackstageSection::ExportReport, "Cross-Suite & Export"),
                    (FileBackstageSection::Options, "Options (Fig. 1-3)"),
                    (FileBackstageSection::Migrator, "Migrate HSC5.xla"),
                ];
                for (sec, lbl) in items {
                    let active = self.file_section == sec;
                    if ui
                        .add_sized(
                            [175.0, 28.0],
                            egui::Button::new(
                                egui::RichText::new(lbl).color(egui::Color32::WHITE).strong(),
                            )
                            .fill(if active {
                                EXCEL_GREEN_DARK
                            } else {
                                egui::Color32::TRANSPARENT
                            }),
                        )
                        .clicked()
                    {
                        self.file_section = sec;
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.file_section {
                FileBackstageSection::Info => {
                    ui.heading("HSC Excel Add-Ins — Arquitectura Híbrida (Standalone + Cross-Suite)");
                    ui.separator();
                    ui.label("• Modo Standalone: 83 funciones activas + 5 HSC-Sim ejecutándose en Rust dentro de Soryntel sin dependencias externas.");
                    ui.label(format!("• Base de datos activa: {} especies cargadas.", db.len()));
                    ui.label("• Minerales integrados: 3,812 registros (Cap. 30 Fig. 7 editable inline).");
                    ui.label("• Compatibilidad externa opcional: Microsoft Excel, LibreOffice Calc (.ods / .oxt), ONLYOFFICE (.zip) y WPS Office (.bas / =WEBSERVICE).");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Ir a Options -> Add-Ins (Fig. 2–3)").clicked() {
                            self.file_section = FileBackstageSection::Options;
                            self.options_cat = OptionsCategory::AddIns;
                        }
                        if ui.button("Ir a Cross-Suite Bridge & Exporters").clicked() {
                            self.file_section = FileBackstageSection::ExportReport;
                        }
                    });
                }
                FileBackstageSection::OpenSamples => {
                    ui.heading("Open — Recent Workbooks & HSC Cap. 30 Templates (Fig. 1)");
                    ui.separator();
                    let templates = [
                        (
                            "Hoja Libre Editable (Worksheet A1:H24)",
                            "Fórmulas =H, =G, =MW, =MineralCode, =SUM, =StreamEQ!C1 listas para editar",
                            AddInSheetTab::Worksheet,
                        ),
                        (
                            "StreamEQ Array Workbook (Cap. 30 Fig. 4–6)",
                            "Equilibrio multifase C + Aire a 1500 °C, 1 bar con Ctrl+Shift+Enter",
                            AddInSheetTab::StreamEq,
                        ),
                        (
                            "Minerals Property Table (Cap. 30 Fig. 7)",
                            "Tabla Pyrite / Py / Py\\Joki / ID:12616 con edición inline y guía",
                            AddInSheetTab::Minerals,
                        ),
                        (
                            "AddInSample (Coordenadas A..I, =UNITS(D2;E2))",
                            "Referencia completa de unidades dinámicas y propiedades de especies",
                            AddInSheetTab::AddInSample,
                        ),
                        (
                            "Catalog & Live Tester (Functions fx — 83+5)",
                            "Explorador interactivo de todas las funciones del Add-In",
                            AddInSheetTab::FunctionsCatalog,
                        ),
                    ];
                    for (title, desc, target) in templates {
                        egui::Frame::none()
                            .fill(EXCEL_ALT_ROW_BG)
                            .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                            .inner_margin(8.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.strong(title);
                                        ui.label(egui::RichText::new(desc).size(11.0).color(TEXT_MUTED));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button("Abrir hoja").clicked() {
                                            self.active_sheet = target;
                                            if target == AddInSheetTab::AddInSample {
                                                self.recalculate_sample_sheet(db);
                                            }
                                            self.load_cell_into_formula_bar();
                                            self.show_file_backstage = false;
                                        }
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }
                }
                FileBackstageSection::ExportReport => {
                    ui.heading("Cross-Suite HTTP Bridge & Workbook / Plugin Exporters");
                    ui.label("Usa Soryntel en modo Standalone o conecta Excel, LibreOffice Calc, ONLYOFFICE y WPS en tiempo real.");
                    ui.add_space(6.0);

                    // 1) Local HTTP Bridge
                    ui.group(|ui| {
                        ui.strong("1) HTTP Bridge Local (127.0.0.1) — Cálculos en vivo para suites externas");
                        ui.horizontal(|ui| {
                            ui.label("Puerto:");
                            let enabled = !self.is_bridge_running();
                            ui.add_enabled(
                                enabled,
                                egui::DragValue::new(&mut self.bridge_port).range(1024..=65535),
                            );
                            if self.is_bridge_running() {
                                if ui.button("Detener HTTP Bridge").clicked() {
                                    self.stop_bridge();
                                }
                            } else if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Iniciar HTTP Bridge")
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                    )
                                    .fill(EXCEL_GREEN),
                                )
                                .clicked()
                            {
                                self.start_bridge(db);
                            }
                            ui.label(format!(
                                "Unidades activas: {} / {} / {}",
                                format_temp_unit_display(&self.units.temp_unit),
                                self.units.energy_unit,
                                self.units.pressure_unit
                            ));
                        });
                        if let Some(h) = &self.bridge_handle {
                            let last = h.last_call();
                            ui.monospace(format!(
                                "Estado: ACTIVO en http://127.0.0.1:{} | Peticiones atendidas: {} | Última llamada: {}",
                                h.port(),
                                h.requests_served(),
                                last.as_deref().unwrap_or("-")
                            ));
                        } else {
                            ui.label(egui::RichText::new("Estado: DETENIDO (Modo 100% interno en Soryntel).").color(TEXT_MUTED));
                        }
                        if !self.bridge_error.is_empty() {
                            ui.colored_label(TEXT_ERR, &self.bridge_error);
                        }
                    });

                    ui.add_space(6.0);

                    // 2) Export Directory & File Save
                    ui.group(|ui| {
                        ui.strong("2) Exportar Libros (.xlsx / .ods / .csv) y Paquetes Add-In (83 funciones activas)");
                        ui.horizontal(|ui| {
                            ui.label("Carpeta:");
                            ui.text_edit_singleline(&mut self.export_dir);
                            ui.label("Nombre base:");
                            ui.text_edit_singleline(&mut self.export_filename);
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Guardar .xlsx (Excel / WPS / Calc / ONLYOFFICE)").clicked() {
                                self.export_status = match self.export_all_workbook(db, "xlsx") {
                                    Ok(p) => format!("Guardado: {p}"),
                                    Err(e) => e,
                                };
                            }
                            if ui.button("Guardar .ods (LibreOffice / OpenDocument)").clicked() {
                                self.export_status = match self.export_all_workbook(db, "ods") {
                                    Ok(p) => format!("Guardado: {p}"),
                                    Err(e) => e,
                                };
                            }
                            if ui.button("Guardar Worksheet .csv").clicked() {
                                self.export_status = match self.export_active_csv() {
                                    Ok(p) => format!("Guardado: {p}"),
                                    Err(e) => e,
                                };
                            }
                        });
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Exportar LibreOffice Calc (.oxt)").clicked() {
                                self.export_status = match self.export_suite_connector("oxt") {
                                    Ok(p) => format!("Conector LibreOffice generado: {p}"),
                                    Err(e) => e,
                                };
                            }
                            if ui.button("Exportar ONLYOFFICE Plugin (.zip)").clicked() {
                                self.export_status = match self.export_suite_connector("onlyoffice") {
                                    Ok(p) => format!("Plugin ONLYOFFICE generado: {p}"),
                                    Err(e) => e,
                                };
                            }
                            if ui.button("Exportar Excel / WPS VBA (.bas)").clicked() {
                                self.export_status = match self.export_suite_connector("bas") {
                                    Ok(p) => format!("Módulo VBA generado: {p}"),
                                    Err(e) => e,
                                };
                            }
                        });
                        if !self.export_status.is_empty() {
                            ui.add_space(4.0);
                            ui.strong(&self.export_status);
                        }
                    });

                    ui.add_space(6.0);

                    // 3) Macro-free =WEBSERVICE builder
                    ui.group(|ui| {
                        ui.strong("3) Fórmula universal sin macros =WEBSERVICE(...) (Excel / Calc / WPS / ONLYOFFICE)");
                        ui.horizontal(|ui| {
                            ui.label("Fn:");
                            ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut self.webservice_fn));
                            ui.label("Arg 1:");
                            ui.add_sized([110.0, 22.0], egui::TextEdit::singleline(&mut self.webservice_a));
                            ui.label("Arg 2:");
                            ui.add_sized([90.0, 22.0], egui::TextEdit::singleline(&mut self.webservice_b));
                        });
                        let args: Vec<&str> = if self.webservice_b.trim().is_empty() {
                            vec![self.webservice_a.trim()]
                        } else {
                            vec![self.webservice_a.trim(), self.webservice_b.trim()]
                        };
                        let ws_formula = SoryntelApplication::addin_webservice_formula(
                            self.bridge_port,
                            self.webservice_fn.trim(),
                            &args,
                        );
                        ui.horizontal(|ui| {
                            ui.monospace(&ws_formula);
                            if ui.button("Copiar fórmula").clicked() {
                                ui.ctx().copy_text(ws_formula);
                                self.status_msg = "Fórmula =WEBSERVICE(...) copiada al portapapeles.".into();
                            }
                        });
                    });
                }
                FileBackstageSection::Options => {
                    ui.heading("Excel Options — View and manage Add-ins (Cap. 30 Fig. 1–3)");
                    ui.separator();
                    ui.horizontal_top(|ui| {
                        // Lista izquierda fiel a Fig. 2
                        ui.vertical(|ui| {
                            ui.set_min_width(165.0);
                            let cats = [
                                (OptionsCategory::General, "General"),
                                (OptionsCategory::Formulas, "Formulas"),
                                (OptionsCategory::Proofing, "Proofing"),
                                (OptionsCategory::Save, "Save"),
                                (OptionsCategory::Language, "Language"),
                                (OptionsCategory::Advanced, "Advanced"),
                                (OptionsCategory::CustomizeRibbon, "Customize Ribbon"),
                                (OptionsCategory::QuickAccessToolbar, "Quick Access Toolbar"),
                                (OptionsCategory::AddIns, "Add-Ins"),
                                (OptionsCategory::TrustCenter, "Trust Center"),
                            ];
                            for (cat, label) in cats {
                                let sel = self.options_cat == cat;
                                if ui
                                    .add_sized(
                                        [160.0, 24.0],
                                        egui::SelectableLabel::new(sel, label),
                                    )
                                    .clicked()
                                {
                                    self.options_cat = cat;
                                }
                            }
                        });

                        ui.separator();

                        // Panel derecho de Excel Options
                        ui.vertical(|ui| {
                            match self.options_cat {
                                OptionsCategory::General => {
                                    ui.strong("General user interface & precision options");
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Decimales por defecto en hojas:");
                                        for d in [4, 6, 8] {
                                            ui.selectable_value(&mut self.decimals, d, format!("{d}"));
                                        }
                                    });
                                }
                                OptionsCategory::Formulas => {
                                    ui.strong("Calculation & formula syntax options");
                                    ui.add_space(6.0);
                                    ui.checkbox(&mut self.show_formulas, "Mostrar fórmulas en lugar de valores");
                                    ui.horizontal(|ui| {
                                        ui.label("Separador de argumentos:");
                                        ui.selectable_value(
                                            &mut self.arg_separator,
                                            ArgSeparator::Semicolon,
                                            ArgSeparator::Semicolon.label(),
                                        );
                                        ui.selectable_value(
                                            &mut self.arg_separator,
                                            ArgSeparator::Comma,
                                            ArgSeparator::Comma.label(),
                                        );
                                    });
                                }
                                OptionsCategory::Save => {
                                    ui.strong("Save & Cross-Suite output directory");
                                    ui.horizontal(|ui| {
                                        ui.label("Carpeta destino:");
                                        ui.text_edit_singleline(&mut self.export_dir);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Nombre base:");
                                        ui.text_edit_singleline(&mut self.export_filename);
                                    });
                                }
                                OptionsCategory::Language => {
                                    ui.strong("Measurement Units & Locale Separator");
                                    ui.label(format!(
                                        "Unidades actuales: {} / {} / {}",
                                        format_temp_unit_display(&self.units.temp_unit),
                                        self.units.energy_unit,
                                        self.units.pressure_unit
                                    ));
                                    if ui.button("Open Change measurement units...").clicked() {
                                        self.pending_units = self.units.clone();
                                        self.show_units_dialog = true;
                                    }
                                }
                                OptionsCategory::Proofing
                                | OptionsCategory::Advanced
                                | OptionsCategory::CustomizeRibbon
                                | OptionsCategory::QuickAccessToolbar => {
                                    ui.strong("Excel Options — HSC Compatibility Defaults");
                                    ui.label("Los atajos de teclado (Ctrl+Shift+Enter para Array StreamEQ y F9 para Recalcular) están activos.");
                                    ui.label("La cinta ADD-INS expone Menu Commands -> HSC ▾ -> Change measurement units...");
                                }
                                OptionsCategory::TrustCenter => {
                                    ui.strong("Trust Center — Local Loopback Security");
                                    ui.label("El cálculo interno funciona 100% offline sin macros ni red.");
                                    ui.label("El HTTP Bridge opcional escucha exclusivamente en 127.0.0.1 (loopback local) con CORS para ONLYOFFICE / LibreOffice / Excel / WPS.");
                                }
                                OptionsCategory::AddIns => {
                                    ui.label(
                                        egui::RichText::new("View and manage Microsoft Office & Cross-Suite Add-ins.")
                                            .strong(),
                                    );
                                    ui.add_space(4.0);
                                    let mut changed = false;

                                    egui::Frame::none()
                                        .fill(egui::Color32::WHITE)
                                        .stroke(egui::Stroke::new(1.0, EXCEL_BORDER))
                                        .inner_margin(6.0)
                                        .show(ui, |ui| {
                                            egui::Grid::new("excel_options_addins_table")
                                                .num_columns(3)
                                                .striped(true)
                                                .spacing([16.0, 4.0])
                                                .show(ui, |ui| {
                                                    ui.strong("Name");
                                                    ui.strong("Location");
                                                    ui.strong("Type");
                                                    ui.end_row();

                                                    ui.label(
                                                        egui::RichText::new("Active Application Add-ins")
                                                            .strong()
                                                            .color(EXCEL_GREEN_DARK),
                                                    );
                                                    ui.label("");
                                                    ui.label("");
                                                    ui.end_row();

                                                    for idx in 0..self.packs.len() {
                                                        if self.packs[idx].enabled {
                                                            let p = &mut self.packs[idx];
                                                            ui.horizontal(|ui| {
                                                                if ui.checkbox(&mut p.enabled, "").changed() {
                                                                    changed = true;
                                                                }
                                                                if ui
                                                                    .selectable_label(
                                                                        self.selected_pack_idx == idx,
                                                                        p.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    self.selected_pack_idx = idx;
                                                                }
                                                            });
                                                            ui.label(egui::RichText::new(p.module_ref).size(11.0).color(TEXT_MUTED));
                                                            ui.label(egui::RichText::new("Excel Add-in (.xll)").size(11.0));
                                                            ui.end_row();
                                                        }
                                                    }

                                                    ui.label(
                                                        egui::RichText::new("Inactive / External Add-ins")
                                                            .strong()
                                                            .color(TEXT_MUTED),
                                                    );
                                                    ui.label("");
                                                    ui.label("");
                                                    ui.end_row();

                                                    for idx in 0..self.packs.len() {
                                                        if !self.packs[idx].enabled {
                                                            let p = &mut self.packs[idx];
                                                            ui.horizontal(|ui| {
                                                                if ui.checkbox(&mut p.enabled, "").changed() {
                                                                    changed = true;
                                                                }
                                                                if ui
                                                                    .selectable_label(
                                                                        self.selected_pack_idx == idx,
                                                                        p.name,
                                                                    )
                                                                    .clicked()
                                                                {
                                                                    self.selected_pack_idx = idx;
                                                                }
                                                            });
                                                            ui.label(egui::RichText::new(p.module_ref).size(11.0).color(TEXT_MUTED));
                                                            ui.label(egui::RichText::new("Inactive Add-in").size(11.0));
                                                            ui.end_row();
                                                        }
                                                    }

                                                    for (ext_name, ext_loc, ext_type) in [
                                                        ("LibreOffice Calc Extension", "SoryntelHSC_Calc.oxt", "UNO / Python Add-In"),
                                                        ("ONLYOFFICE Custom Functions", "SoryntelHSC_ONLYOFFICE.zip", "JS Plugin"),
                                                        ("Excel / WPS VBA Module", "SoryntelHSC.bas", "VBA UDF Module"),
                                                    ] {
                                                        ui.label(ext_name);
                                                        ui.label(egui::RichText::new(ext_loc).size(11.0).color(TEXT_MUTED));
                                                        ui.label(
                                                            egui::RichText::new(if self.is_bridge_running() {
                                                                format!("{ext_type} (Bridge ON)")
                                                            } else {
                                                                format!("{ext_type} (Exportable)")
                                                            })
                                                            .size(11.0),
                                                        );
                                                        ui.end_row();
                                                    }
                                                });
                                        });

                                    if changed {
                                        self.recalculate_all(db);
                                    }

                                    ui.add_space(6.0);
                                    if let Some(p) = self.packs.get(self.selected_pack_idx) {
                                        ui.group(|ui| {
                                            ui.label(format!("Add-in:         {}", p.name));
                                            ui.label("Publisher:      Soryntel / HSC Compatible Engine");
                                            ui.label(format!("Compatibility:  {} (Standalone + Cross-Suite)", p.pack_type));
                                            ui.label(format!("Location:       {}", p.module_ref));
                                            ui.label(format!("Description:    {}", p.description));
                                        });
                                    }

                                    ui.add_space(8.0);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Manage:").strong());
                                        egui::ComboBox::from_id_salt("options_manage_target")
                                            .selected_text(self.manage_target.label())
                                            .width(210.0)
                                            .show_ui(ui, |ui| {
                                                for target in [
                                                    ManageAddInTarget::ExcelAddIns,
                                                    ManageAddInTarget::LibreOfficeOxt,
                                                    ManageAddInTarget::OnlyOfficeZip,
                                                    ManageAddInTarget::VbaModule,
                                                ] {
                                                    ui.selectable_value(
                                                        &mut self.manage_target,
                                                        target,
                                                        target.label(),
                                                    );
                                                }
                                            });
                                        if ui.button("  Go...  ").clicked() {
                                            match self.manage_target {
                                                ManageAddInTarget::ExcelAddIns => {
                                                    self.show_addins_go_dialog = true;
                                                }
                                                ManageAddInTarget::LibreOfficeOxt => {
                                                    self.browse_selected_file = 3;
                                                    self.show_browse_dialog = true;
                                                }
                                                ManageAddInTarget::OnlyOfficeZip => {
                                                    self.browse_selected_file = 4;
                                                    self.show_browse_dialog = true;
                                                }
                                                ManageAddInTarget::VbaModule => {
                                                    self.browse_selected_file = 5;
                                                    self.show_browse_dialog = true;
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        });
                    });
                }
                FileBackstageSection::Migrator => {
                    ui.heading("Migrador de fórmulas legacy HSC5.xla -> HSC 6–10");
                    ui.label("Convierte sintaxis antigua (unidades inline, argumentos obsoletos) a fórmulas modernas:");
                    ui.add_space(6.0);
                    let resp = ui.add_sized(
                        [520.0, 26.0],
                        egui::TextEdit::singleline(&mut self.migrator_input)
                            .font(egui::FontId::monospace(12.0)),
                    );
                    if resp.changed() || ui.button("Convertir").clicked() {
                        let (out, warn) = SoryntelApplication::migrate_addin_formula(&self.migrator_input);
                        self.migrator_output = out;
                        self.migrator_warnings = warn;
                    }
                    ui.add_space(4.0);
                    ui.monospace(format!("Resultado: {}", self.migrator_output));
                    for w in &self.migrator_warnings {
                        ui.label(egui::RichText::new(format!("• {w}")).color(TEXT_MUTED));
                    }
                    if ui.button("Insertar resultado en Worksheet").clicked() {
                        let f = self.migrator_output.clone();
                        self.show_file_backstage = false;
                        self.insert_formula_into_worksheet(db, f);
                    }
                }
            });
        });
    }
}

struct GridInlineState<'a> {
    editing: &'a mut Option<(usize, usize)>,
    buf: &'a mut String,
    focus: &'a mut bool,
}

enum ExportGridAction {
    Selected { coord: String, raw: String },
    Committed { row: usize, col: usize, new_raw: String },
}

fn resolve_export_cell_colors(
    cell: &SheetCell,
    row_col0_style: Option<ExportCellStyle>,
    row: usize,
    col: usize,
    is_minerals: bool,
) -> (egui::Color32, egui::Color32, bool) {
    let mut bold = cell.style == ExportCellStyle::Header;
    let bg = match cell.style {
        ExportCellStyle::Olive => {
            bold = true;
            FIG7_OLIVE
        }
        ExportCellStyle::Green => {
            bold = true;
            FIG7_GREEN
        }
        ExportCellStyle::Teal => {
            bold = true;
            FIG7_TEAL
        }
        ExportCellStyle::Header => {
            if is_minerals && col >= 1 {
                FIG7_NAVY_HEADER
            } else {
                EXCEL_SECTION_BG
            }
        }
        ExportCellStyle::YellowNote => EXCEL_YELLOW_BG,
        _ if is_minerals && (1..=5).contains(&col) => match row_col0_style {
            Some(ExportCellStyle::Olive | ExportCellStyle::Green) => FIG7_SILVER_BG,
            Some(ExportCellStyle::Teal) => FIG7_SKY_BG,
            _ if row % 2 == 1 => EXCEL_ALT_ROW_BG,
            _ => egui::Color32::WHITE,
        },
        _ if row % 2 == 1 => EXCEL_ALT_ROW_BG,
        _ => egui::Color32::WHITE,
    };

    let fg = if matches!(cell.style, ExportCellStyle::Olive | ExportCellStyle::Green | ExportCellStyle::Teal)
        || (is_minerals && cell.style == ExportCellStyle::Header && col >= 1)
    {
        egui::Color32::WHITE
    } else if matches!(cell.eval, CellEvalValue::Error(_)) {
        TEXT_ERR
    } else if cell.is_formula {
        if is_minerals { TEXT_DARK } else { TEXT_BLUE }
    } else if cell.style == ExportCellStyle::InputRed {
        if is_minerals {
            bold = true;
        }
        TEXT_RED
    } else {
        TEXT_DARK
    };

    (bg, fg, bold)
}

/// Dibuja una hoja tabular (`Minerals` o `AddInSample`) con cabeceras A..N, paleta Fig. 7 y edición inline por doble clic.
#[allow(clippy::too_many_arguments)]
fn render_export_cells_grid(
    ui: &mut egui::Ui,
    cells: &BTreeMap<(usize, usize), SheetCell>,
    max_row: usize,
    col_count: usize,
    active_cell_ref: &str,
    show_formulas: bool,
    decimals: usize,
    wide_sample_cols: bool,
    is_minerals: bool,
    inline: &mut GridInlineState<'_>,
) -> Option<ExportGridAction> {
    let row_w = 34.0;
    let base_w = if wide_sample_cols { 112.0 } else { 104.0 };
    let mut action: Option<ExportGridAction> = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        header_cell(ui, "", row_w);
        for c in 0..col_count {
            let width = if wide_sample_cols && (c == 3 || c == 5) {
                138.0
            } else {
                base_w
            };
            header_cell(ui, &col_letter(c), width);
        }
    });

    for r in 0..=max_row {
        let row_col0_style = cells.get(&(r, 0)).map(|x| x.style);
        let is_mineral_input_row = is_minerals
            && row_col0_style == Some(ExportCellStyle::Olive)
            && cells
                .get(&(r, 0))
                .is_some_and(|x| x.raw.eq_ignore_ascii_case("Mineral"));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            row_num_cell(ui, r + 1, row_w);
            for c in 0..col_count {
                let width = if wide_sample_cols && (c == 3 || c == 5) {
                    138.0
                } else {
                    base_w
                };
                let cell = cells.get(&(r, c)).cloned().unwrap_or_default();
                let cname = format!("{}{}", col_letter(c), r + 1);
                let selected = active_cell_ref == cname;

                if *inline.editing == Some((r, c)) {
                    let bg = if is_minerals { FIG7_SILVER_BG } else { egui::Color32::WHITE };
                    widget_cell(ui, width, bg, true, |ui| {
                        let resp = ui.add_sized(
                            [width - 6.0, CELL_H - 4.0],
                            egui::TextEdit::singleline(inline.buf)
                                .font(egui::FontId::monospace(11.5)),
                        );
                        if *inline.focus {
                            resp.request_focus();
                            *inline.focus = false;
                        }
                        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                        if esc {
                            *inline.editing = None;
                        } else if enter || resp.lost_focus() {
                            action = Some(ExportGridAction::Committed {
                                row: r,
                                col: c,
                                new_raw: inline.buf.clone(),
                            });
                            *inline.editing = None;
                        }
                    });
                    continue;
                }

                let txt = if show_formulas {
                    cell.raw.clone()
                } else if is_blank_cell_val(&cell.eval) {
                    cell.raw.clone()
                } else {
                    cell.eval.as_display_string(decimals)
                };
                let (bg, fg, bold) =
                    resolve_export_cell_colors(&cell, row_col0_style, r, c, is_minerals);
                let right = matches!(cell.eval, CellEvalValue::Number(_)) && !show_formulas;
                let resp = text_cell(ui, &txt, width, bg, fg, right, bold, selected);
                let editable = !is_minerals
                    || cell.style == ExportCellStyle::InputRed
                    || (is_mineral_input_row && (1..=4).contains(&c));
                if resp.clicked() {
                    action = Some(ExportGridAction::Selected {
                        coord: cname,
                        raw: cell.raw.clone(),
                    });
                }
                if resp.double_clicked() && editable {
                    *inline.editing = Some((r, c));
                    *inline.buf = cell.raw;
                    *inline.focus = true;
                }
            }
        });
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use soryntel_db::ThermoDatabase;
    use soryntel_thermo::SpeciesRepository;

    fn test_db_enriched() -> ThermoDatabase {
        let mut db = ThermoDatabase::with_open_defaults();
        for p in [
            "/home/nando/HSC6/Maindb6.HSC",
            "/home/nando/Documentos/HSC-REA/Maindb6.HSC",
            "../../Maindb6.HSC",
        ] {
            if std::path::Path::new(p).exists() {
                let _ = db.load_external_hsc_readonly(p, false);
                if db.len() > 100 {
                    return db;
                }
            }
        }
        // Fallback determinista para entornos sin Maindb6.HSC: clona plantilla CO(g)/Al2O3
        if let Some(co) = db.get_species("CO(g)") {
            for f in ["CO2(g)", "N2(g)", "N(g)", "NO(g)", "N2O3(g)", "N2O4(g)", "N2O5(g)", "He(g)", "CH4(g)", "H2O(g)"] {
                if db.get_species(f).is_none() {
                    let mut sp = co.clone();
                    sp.formula = f.into();
                    sp.name = f.into();
                    db.insert_species(sp);
                }
            }
        }
        if let Some(al) = db.get_species("Al2O3") {
            for f in ["Fe", "Cu", "Zn", "C", "Fe2O3", "FeS", "H2O", "H2O(l)"] {
                if db.get_species(f).is_none() {
                    let mut sp = al.clone();
                    sp.formula = f.into();
                    sp.name = f.into();
                    db.insert_species(sp);
                }
            }
        }
        db
    }

    #[test]
    fn worksheet_multi_pass_spill_and_cross_sheet_refs_work_standalone() {
        let open_db = ThermoDatabase::with_open_defaults();
        let mut st = ExcelAddInsModuleState::default();
        st.ensure_initialized(&open_db);

        // 1) Demo inicial en Worksheet (siempre válida incluso sin Maindb6): C2 = =H(A2;B2), C6 = =SUM(C2:C5)
        let h_co = st.ws_eval[1][2]
            .as_f64()
            .expect("C2 =H(CO(g);25) must evaluate cleanly");
        assert!((h_co - (-110.5)).abs() < 3.0, "CO(g) H@25C = {h_co}");
        assert!(
            matches!(st.ws_eval[5][2], CellEvalValue::Number(v) if v.is_finite()),
            "SUM over formula cells failed: {:?}",
            st.ws_eval[5][2]
        );
        assert_eq!(st.ws_eval[8][2].as_f64(), Some(1500.0));
        assert!(
            matches!(st.ws_eval[7][2], CellEvalValue::Text(ref s) if s == "Py"),
            "MineralCode cross-check failed: {:?}",
            st.ws_eval[7][2]
        );

        // 2) Derrame matricial de StreamEQ en Worksheet sobre sistema binario abierto Mg / Mg(g)
        for row in &mut st.ws_raw {
            for c in row {
                c.clear();
            }
        }
        st.ws_raw[0][2] = "1500".into();
        st.ws_raw[1][2] = "1".into();
        st.ws_raw[3][0] = "Mg".into();
        st.ws_raw[3][1] = "1".into();
        st.ws_raw[4][0] = "Mg(g)".into();
        st.ws_raw[4][1] = "0".into();
        st.ws_raw[3][2] = "=StreamEQ(A4:A5;B4:B5;C1;C2)".into();
        st.recalculate_worksheet(&open_db);
        assert!(
            matches!(st.ws_eval[3][2], CellEvalValue::Number(v) if (0.0..=1.0).contains(&v)),
            "Mg spill row failed: {:?}",
            st.ws_eval[3][2]
        );
        assert!(
            matches!(st.ws_eval[4][2], CellEvalValue::Number(v) if (0.0..=1.0).contains(&v)),
            "Mg(g) spill row failed: {:?}",
            st.ws_eval[4][2]
        );
        assert_eq!(
            st.ws_array_label[4][2].as_deref(),
            Some("{=StreamEQ(A4:A5;B4:B5;C1;C2)}")
        );

        // 3) Plantilla StreamEQ Fig. 4-6 con BD enriquecida
        let full_db = test_db_enriched();
        populate_streameq_worksheet_template(&mut st.ws_raw);
        st.recalculate_worksheet(&full_db);
        assert_eq!(st.ws_eval[3][2].as_f64(), Some(1500.0));
        assert!(
            matches!(st.ws_eval[4][2], CellEvalValue::Number(v) if (0.0..=1.0).contains(&v)),
            "CO(g) spill row failed: {:?}",
            st.ws_eval[4][2]
        );

        // 4) Edición desde barra de fórmulas sobre StreamEQ y AddInSample
        st.active_sheet = AddInSheetTab::StreamEq;
        st.select_cell("C1", "1200".into());
        st.apply_formula_bar(&full_db);
        assert_eq!(st.eq_temp_raw, "1200");

        st.select_cell("C1", "bad_temp".into());
        st.apply_formula_bar(&full_db);
        assert!(matches!(st.eq_outputs.first(), Some(CellEvalValue::Error(e)) if e == "#VALUE!"));

        st.select_cell("C1", "1500".into());
        st.apply_formula_bar(&full_db);
        st.active_sheet = AddInSheetTab::AddInSample;
        st.select_cell("D8", "Fe2O3".into());
        st.apply_formula_bar(&full_db);
        assert!(
            st.sample_cells
                .get(&(7, 3))
                .map(|c| c.raw == "Fe2O3")
                .unwrap_or(false)
        );
    }

    #[test]
    fn disabling_addin_pack_blocks_its_formulas() {
        let db = ThermoDatabase::with_open_defaults();
        let mut st = ExcelAddInsModuleState::default();
        st.ensure_initialized(&db);

        // Desactivar Mineralogy -> MineralCode(B8) pasa a #NAME?
        st.packs[2].enabled = false;
        st.recalculate_worksheet(&db);
        assert_eq!(st.ws_eval[7][2], CellEvalValue::Error("#NAME?".into()));

        // Reactivar -> vuelve a "Py"
        st.packs[2].enabled = true;
        st.recalculate_worksheet(&db);
        assert_eq!(st.ws_eval[7][2], CellEvalValue::Text("Py".into()));
    }

    #[test]
    fn all_83_active_catalog_functions_populate_and_evaluate_cleanly() {
        for db in [ThermoDatabase::with_open_defaults(), test_db_enriched()] {
            let mut st = ExcelAddInsModuleState::default();
            st.ensure_initialized(&db);

            let mut failures = Vec::new();
            for (idx, meta) in ADDIN_FUNCTION_CATALOG.iter().enumerate() {
                if meta.origin == AddInOrigin::HscSimOnly {
                    continue;
                }
                st.selected_fn_idx = idx;
                st.populate_tester_from_example(*meta, &db);
                match &st.test_result {
                    CellEvalValue::Error(e) => {
                        failures.push(format!(
                            "[db={}] {} -> {e} (args={:?})",
                            db.len(),
                            meta.name,
                            st.test_args
                        ));
                    }
                    CellEvalValue::Text(s) if s.is_empty() => {
                        failures.push(format!("[db={}] {} -> Empty", db.len(), meta.name));
                    }
                    _ => {}
                }
            }
            assert!(
                failures.is_empty(),
                "Catalog functions failed live tester:\n{}",
                failures.join("\n")
            );
        }
    }

    #[test]
    fn minerals_inline_edit_and_export_connectors_work_end_to_end() {
        let db = ThermoDatabase::with_open_defaults();
        let mut st = ExcelAddInsModuleState::default();
        st.ensure_initialized(&db);

        // Edición inline sobre B4 ("Pyrite" -> "Chalcopyrite") recalcula Code B5 ("Ccp") y Formula B6 ("CuFeS2")
        st.apply_grid_inline_commit(&db, 3, 1, "Chalcopyrite".into(), true);
        assert_eq!(
            st.min_cells.get(&(4, 1)).map(|c| &c.eval),
            Some(&CellEvalValue::Text("Ccp".into()))
        );
        assert_eq!(
            st.min_cells.get(&(5, 1)).map(|c| &c.eval),
            Some(&CellEvalValue::Text("CuFeS2".into()))
        );

        // Exportación en memoria de conectores .oxt / .zip / .bas y libros .xlsx / .ods a carpeta temporal
        let tmp = std::env::temp_dir().join("soryntel_addin_ui_test");
        st.export_dir = tmp.to_string_lossy().to_string();
        let p_oxt = st.export_suite_connector("oxt").expect("oxt export");
        let p_zip = st.export_suite_connector("onlyoffice").expect("onlyoffice zip export");
        let p_bas = st.export_suite_connector("bas").expect("bas export");
        let p_xlsx = st.export_all_workbook(&db, "xlsx").expect("xlsx export");
        let p_ods = st.export_all_workbook(&db, "ods").expect("ods export");

        assert!(std::fs::read(&p_oxt).expect("read oxt").len() > 500);
        assert!(std::fs::read(&p_zip).expect("read zip").len() > 500);
        assert!(std::fs::read_to_string(&p_bas).expect("read bas").contains("Public Function H("));
        assert!(std::fs::read(&p_xlsx).expect("read xlsx").len() > 500);
        assert!(std::fs::read(&p_ods).expect("read ods").len() > 500);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
