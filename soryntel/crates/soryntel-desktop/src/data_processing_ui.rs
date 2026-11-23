use std::collections::HashMap;

use eframe::egui;
use serde::{Deserialize, Serialize};
use soryntel_thermo::data_processing::{
    compute_column_analytics, eval_excel_expr, moving_average, parse_flex_f64, parse_nn_table,
    parse_pso_x_comment_with_defaults, ActivationFunc, ColumnAnalytics, FfnnConfig, FfnnTrainer,
    ParsedNnDataset, PsoBoundaryCondition, PsoConfig, PsoOptimizer, PsoVarSpec, RnnConfig,
    RnnTrainer, RnnValidationMethod, SimpleRng,
};

use crate::excel_grid::{
    header_cell, row_num_cell, widget_cell, EXCEL_ALT_ROW_BG, EXCEL_GRID_BORDER,
    EXCEL_READONLY_BG,
};
use crate::ui_common::{draw_dashed_line, draw_rotated_y_title, format_tick, nice_step};

// ============================================================================
// Top-level Module Enums & Theme (Figs. 1 & 2)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTool {
    Home,
    Charts,
    Ffnn,
    Rnn,
    Pso,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DataTheme {
    DevExpressStyle,
    OfficeLight,
    SoftBlue,
    ClassicGray,
}

impl DataTheme {
    const ALL: &'static [DataTheme] = &[
        DataTheme::DevExpressStyle,
        DataTheme::OfficeLight,
        DataTheme::SoftBlue,
        DataTheme::ClassicGray,
    ];

    fn label(self) -> &'static str {
        match self {
            DataTheme::DevExpressStyle => "DevExpress Style",
            DataTheme::OfficeLight => "Office Light",
            DataTheme::SoftBlue => "Soft Blue",
            DataTheme::ClassicGray => "Classic Gray",
        }
    }

    fn panel_fill(self) -> egui::Color32 {
        match self {
            DataTheme::DevExpressStyle => egui::Color32::from_rgb(240, 243, 248),
            DataTheme::OfficeLight => egui::Color32::from_rgb(246, 247, 249),
            DataTheme::SoftBlue => egui::Color32::from_rgb(230, 239, 250),
            DataTheme::ClassicGray => egui::Color32::from_rgb(232, 234, 238),
        }
    }

    fn ribbon_fill(self) -> egui::Color32 {
        match self {
            DataTheme::DevExpressStyle => egui::Color32::from_rgb(235, 240, 248),
            DataTheme::OfficeLight => egui::Color32::from_rgb(243, 244, 246),
            DataTheme::SoftBlue => egui::Color32::from_rgb(219, 234, 254),
            DataTheme::ClassicGray => egui::Color32::from_rgb(224, 227, 232),
        }
    }
}

// ============================================================================
// Charts Sub-Tool State (Fig. 3)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ChartKind {
    LineMarkers,
    Line,
    Scatter,
    Bar,
}

impl ChartKind {
    const ALL: &'static [ChartKind] = &[
        ChartKind::LineMarkers,
        ChartKind::Line,
        ChartKind::Scatter,
        ChartKind::Bar,
    ];
    fn label(self) -> &'static str {
        match self {
            ChartKind::LineMarkers => "Line + Markers",
            ChartKind::Line => "Line",
            ChartKind::Scatter => "Scatter",
            ChartKind::Bar => "Bar",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChartSpec {
    id: usize,
    tab_name: String,
    title: String,
    x_axis_title: String,
    y_axis_title: String,
    kind: ChartKind,
    show_grid: bool,
    show_trendline: bool,
    smooth_window: usize,
    line_width: f32,
    auto_y: bool,
    y_min_str: String,
    y_max_str: String,
    series_visible: Vec<bool>,
    color_offset: usize,
}

impl ChartSpec {
    fn new(id: usize) -> Self {
        Self {
            id,
            tab_name: format!("Chart - {id}"),
            title: "New Chart".to_string(),
            x_axis_title: "XAxis".to_string(),
            y_axis_title: "YAxis".to_string(),
            kind: ChartKind::LineMarkers,
            show_grid: true,
            show_trendline: false,
            smooth_window: 1,
            line_width: 1.6,
            auto_y: true,
            y_min_str: "-20000".to_string(),
            y_max_str: "18000".to_string(),
            series_visible: vec![true; 8],
            color_offset: (id.saturating_sub(1)) % 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChartSheet {
    name: String,
    grid: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HscChartFile {
    version: u32,
    sheets: Vec<ChartSheet>,
    charts: Vec<ChartSpec>,
}

struct ChartsToolState {
    sheets: Vec<ChartSheet>,
    active_sheet: usize,
    active_cell: (usize, usize),
    charts: Vec<ChartSpec>,
    active_chart: usize,
    next_chart_id: usize,
    auto_update: bool,
    dashboard_layout: bool,
    show_analytics: bool,
    show_modify: bool,
    show_about: bool,
    show_paste_modal: bool,
    paste_buf: String,
    show_file_modal: bool,
    file_modal_is_save: bool,
    file_path: String,
    status_msg: Option<String>,
    status_err: Option<String>,
}

impl Default for ChartsToolState {
    fn default() -> Self {
        Self::with_fig3_sample()
    }
}

impl ChartsToolState {
    fn with_fig3_sample() -> Self {
        let mut grid = vec![vec![String::new(); 8]; 22];
        grid[0][0] = "X".into();
        grid[0][1] = "Y".into();
        let fig3_rows = [
            ("1", "13815.38"),
            ("2", "8678.181"),
            ("3", "-85.7106"),
            ("4", "-9563.4"),
            ("5", "-759.026"),
            ("6", "-8427.38"),
            ("7", "-7655.88"),
            ("8", "-4558.14"),
            ("9", "-14464.3"),
        ];
        for (i, (x, y)) in fig3_rows.iter().enumerate() {
            grid[i + 1][0] = (*x).into();
            grid[i + 1][1] = (*y).into();
        }
        Self {
            sheets: vec![ChartSheet {
                name: "Sheet1".into(),
                grid,
            }],
            active_sheet: 0,
            active_cell: (4, 4),
            charts: vec![ChartSpec::new(1)],
            active_chart: 0,
            next_chart_id: 2,
            auto_update: true,
            dashboard_layout: false,
            show_analytics: false,
            show_modify: false,
            show_about: false,
            show_paste_modal: false,
            paste_buf: String::new(),
            show_file_modal: false,
            file_modal_is_save: true,
            file_path: "/home/nando/.gemini/antigravity/scratch/chart.HSCChart".into(),
            status_msg: None,
            status_err: None,
        }
    }

    fn reset_blank(&mut self) {
        self.sheets = vec![ChartSheet {
            name: "Sheet1".into(),
            grid: vec![vec![String::new(); 8]; 22],
        }];
        self.sheets[0].grid[0][0] = "X".into();
        self.sheets[0].grid[0][1] = "Y".into();
        self.active_sheet = 0;
        self.charts = vec![ChartSpec::new(1)];
        self.active_chart = 0;
        self.next_chart_id = 2;
        self.status_msg = Some("Created new blank chart sheet.".into());
        self.status_err = None;
    }

    fn extract_xy_series(&self) -> (Vec<f64>, Vec<(String, Vec<f64>)>) {
        let grid = &self.sheets[self.active_sheet].grid;
        if grid.len() < 2 || grid[0].len() < 2 {
            return (Vec::new(), Vec::new());
        }
        let n_cols = grid[0].len();
        let mut xs = Vec::new();
        let mut col_ys: Vec<Vec<f64>> = vec![Vec::new(); n_cols - 1];
        let mut names: Vec<String> = (1..n_cols)
            .map(|c| {
                let h = grid[0][c].trim();
                if h.is_empty() {
                    col_letter(c)
                } else {
                    h.to_string()
                }
            })
            .collect();

        for row in grid.iter().skip(1) {
            let Some(xv) = row.first().and_then(|s| parse_flex_f64(s)) else {
                continue;
            };
            let mut any_y = false;
            for c in 1..n_cols {
                if row.get(c).and_then(|s| parse_flex_f64(s)).is_some() {
                    any_y = true;
                    break;
                }
            }
            if !any_y {
                continue;
            }
            xs.push(xv);
            for c in 1..n_cols {
                let yv = row
                    .get(c)
                    .and_then(|s| parse_flex_f64(s))
                    .unwrap_or(f64::NAN);
                col_ys[c - 1].push(yv);
            }
        }

        let mut out = Vec::new();
        for (name, ys) in names.drain(..).zip(col_ys) {
            if ys.iter().any(|v| v.is_finite()) {
                out.push((name, ys));
            }
        }
        (xs, out)
    }

    fn save_file(&mut self, path: &str) {
        let doc = HscChartFile {
            version: 1,
            sheets: self.sheets.clone(),
            charts: self.charts.clone(),
        };
        match serde_json::to_string_pretty(&doc) {
            Ok(json) => match std::fs::write(path, json) {
                Ok(_) => {
                    self.status_err = None;
                    self.status_msg = Some(format!("Saved {path}"));
                }
                Err(e) => self.status_err = Some(format!("Save failed: {e}")),
            },
            Err(e) => self.status_err = Some(format!("Serialization error: {e}")),
        }
    }

    fn open_file(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(doc) = serde_json::from_str::<HscChartFile>(&content) {
                    if !doc.sheets.is_empty() {
                        self.sheets = doc.sheets;
                        self.active_sheet = 0;
                    }
                    if !doc.charts.is_empty() {
                        self.charts = doc.charts;
                        self.active_chart = 0;
                        self.next_chart_id =
                            self.charts.iter().map(|c| c.id).max().unwrap_or(0) + 1;
                    }
                    self.status_err = None;
                    self.status_msg = Some(format!("Opened {path}"));
                } else {
                    // Fallback: import delimited CSV/TSV into active sheet
                    apply_tsv_to_grid(&mut self.sheets[self.active_sheet].grid, &content, 0);
                    self.status_err = None;
                    self.status_msg = Some(format!("Imported CSV/TSV from {path}"));
                }
            }
            Err(e) => self.status_err = Some(format!("Open failed: {e}")),
        }
    }
}

// ============================================================================
// FFNN Sub-Tool State (Figs. 4–11)
// ============================================================================

struct FfnnToolState {
    grid: Vec<Vec<String>>,
    active_cell: (usize, usize),
    // Hyperparameters strings (Fig. 5)
    lr_str: String,
    lr_decay_str: String,
    momentum_str: String,
    hidden_act: ActivationFunc,
    output_act: ActivationFunc,
    hidden_layers_str: String,
    val_pct_str: String,
    train_pct_str: String,
    batch_size_str: String,
    stop_mse_str: String,
    max_iter_str: String,
    selected_prop: &'static str,
    // Runtime state
    parsed: Option<ParsedNnDataset>,
    trainer: Option<FfnnTrainer>,
    show_val_parity: bool,
    show_train_parity: bool,
    show_val_mse: bool,
    show_train_mse: bool,
    show_nn_compare: bool,
    show_orig_compare: bool,
    show_nn_picture: bool,
    show_xml_window: bool,
    xml_cache: String,
    show_paste_modal: bool,
    paste_buf: String,
    show_file_modal: bool,
    file_modal_mode: FileModalMode,
    file_path: String,
    status_msg: Option<String>,
    status_err: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileModalMode {
    OpenData,
    SaveData,
    ExportXml,
}

impl Default for FfnnToolState {
    fn default() -> Self {
        Self::with_fig4_sample()
    }
}

impl FfnnToolState {
    fn with_fig4_sample() -> Self {
        // Fig. 4 layout: RULE | X (Var 1..8) | Y (Output) | YNN (Prediction)
        let mut grid = vec![
            vec![
                "RULE".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "Y".into(),
                "YNN".into(),
            ],
            vec![
                String::new(),
                "Variable 1".into(),
                "Variable 2".into(),
                "Variable 3".into(),
                "Variable 4".into(),
                "Variable 5".into(),
                "Variable 6".into(),
                "Variable 7".into(),
                "Variable 8".into(),
                "Output".into(),
                "Prediction".into(),
            ],
            vec![String::new(); 11],
        ];
        // Generate 40 realistic nonlinear metallurgical samples matching Fig. 4 scale & 'E' exclusions
        let mut rng = SimpleRng::new(2026_0004);
        for i in 0..40 {
            let xs: Vec<f64> = (0..8).map(|_| rng.range(-950.0, 950.0)).collect();
            let y = 9.2 * xs[0]
                - 6.4 * xs[1]
                + 5.1 * xs[2]
                + 8.8 * xs[3]
                - 4.3 * xs[4]
                + 3.7 * xs[5]
                - 2.9 * xs[6]
                + 4.5 * xs[7]
                + 0.002 * xs[0] * xs[3];
            let rule = if i == 1 || i == 4 || i == 12 || i == 25 {
                "E".to_string()
            } else {
                String::new()
            };
            let mut row = vec![rule];
            for x in xs {
                row.push(format!("{x:.4}"));
            }
            row.push(format!("{y:.3}"));
            row.push(String::new());
            grid.push(row);
        }

        let mut st = Self {
            grid,
            active_cell: (3, 0),
            lr_str: "0.02".into(),
            lr_decay_str: "1E-06".into(),
            momentum_str: "0.1".into(),
            hidden_act: ActivationFunc::TanH,
            output_act: ActivationFunc::Linear,
            hidden_layers_str: "16".into(),
            val_pct_str: "20".into(),
            train_pct_str: "80".into(),
            batch_size_str: "1".into(),
            stop_mse_str: "1E-05".into(),
            max_iter_str: "500".into(),
            selected_prop: "Learning Rate",
            parsed: None,
            trainer: None,
            show_val_parity: true,
            show_train_parity: true,
            show_val_mse: true,
            show_train_mse: true,
            show_nn_compare: true,
            show_orig_compare: true,
            show_nn_picture: false,
            show_xml_window: false,
            xml_cache: String::new(),
            show_paste_modal: false,
            paste_buf: String::new(),
            show_file_modal: false,
            file_modal_mode: FileModalMode::ExportXml,
            file_path: "/home/nando/.gemini/antigravity/scratch/ffnn_model.xml".into(),
            status_msg: Some("Ready. Click 'Apply Data' to validate table, then 'Learn'.".into()),
            status_err: None,
        };
        st.apply_data();
        st
    }

    fn build_config(&self) -> FfnnConfig {
        let hidden: Vec<usize> = self
            .hidden_layers_str
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&v| v > 0)
            .collect();
        FfnnConfig {
            learning_rate: parse_flex_f64(&self.lr_str).unwrap_or(0.02).clamp(1e-5, 1.0),
            learning_rate_decay: parse_flex_f64(&self.lr_decay_str).unwrap_or(1e-6).max(0.0),
            momentum: parse_flex_f64(&self.momentum_str).unwrap_or(0.1).clamp(0.0, 0.99),
            hidden_activation: self.hidden_act,
            output_activation: self.output_act,
            hidden_layers: if hidden.is_empty() { vec![12] } else { hidden },
            validation_pct: parse_flex_f64(&self.val_pct_str).unwrap_or(20.0).clamp(5.0, 50.0),
            training_pct: parse_flex_f64(&self.train_pct_str).unwrap_or(80.0).clamp(50.0, 95.0),
            batch_size: self.batch_size_str.trim().parse::<usize>().unwrap_or(1).max(1),
            stopping_mse: parse_flex_f64(&self.stop_mse_str).unwrap_or(1e-5).max(1e-12),
            max_iterations: self.max_iter_str.trim().parse::<usize>().unwrap_or(500).max(1),
        }
    }

    fn apply_data(&mut self) {
        match parse_nn_table(&self.grid) {
            Ok(ds) => {
                let cfg = self.build_config();
                match FfnnTrainer::from_dataset(&ds, cfg) {
                    Ok(tr) => {
                        let msg = format!(
                            "Data applied: {} inputs (X), {} outputs (Y), {} active rows ({} excluded 'E').",
                            ds.x_cols.len(),
                            ds.y_cols.len(),
                            ds.active_count,
                            ds.excluded_count
                        );
                        self.xml_cache = tr.best_model.to_xml();
                        self.parsed = Some(ds);
                        self.trainer = Some(tr);
                        self.status_err = None;
                        self.status_msg = Some(msg);
                    }
                    Err(e) => {
                        self.status_err = Some(e);
                    }
                }
            }
            Err(e) => {
                self.status_err = Some(e);
            }
        }
    }

    fn start_learn(&mut self) {
        if self.trainer.is_none() {
            self.apply_data();
        }
        let cfg = self.build_config();
        let need_reset = self.trainer.as_ref().map(|t| t.finished).unwrap_or(false);
        if need_reset {
            if let Some(ds) = &self.parsed {
                if let Ok(fresh) = FfnnTrainer::from_dataset(ds, cfg.clone()) {
                    self.trainer = Some(fresh);
                }
            }
        }
        if let Some(tr) = &mut self.trainer {
            tr.config = cfg;
            tr.running = true;
            tr.finished = false;
            self.status_err = None;
            self.status_msg = Some("Training FFNN...".into());
        }
    }

    fn stop_learn(&mut self) {
        if let Some(tr) = &mut self.trainer {
            tr.running = false;
            self.xml_cache = tr.best_model.to_xml();
            self.status_msg = Some(format!(
                "Stopped at iter {} (Best Val MSE = {:.6e} at iter {}).",
                tr.iteration, tr.best_val_mse, tr.best_iter
            ));
        }
    }

    fn calculate_nn(&mut self) {
        let (Some(ds), Some(tr)) = (&self.parsed, &self.trainer) else {
            self.status_err = Some("Apply Data and Learn before Calculate NN.".into());
            return;
        };
        // Ensure a YNN column exists
        let ynn_cols = if ds.ynn_cols.is_empty() {
            let new_c = self.grid[0].len();
            for (r_idx, row) in self.grid.iter_mut().enumerate() {
                row.push(match r_idx {
                    0 => "YNN".into(),
                    1 => "Prediction".into(),
                    _ => String::new(),
                });
            }
            vec![new_c]
        } else {
            ds.ynn_cols.clone()
        };

        let mut predicted = 0usize;
        for r_idx in 3..self.grid.len() {
            let mut x_vals = Vec::with_capacity(ds.x_cols.len());
            let mut ok = true;
            for &xc in &ds.x_cols {
                if let Some(v) = self.grid[r_idx].get(xc).and_then(|s| parse_flex_f64(s)) {
                    x_vals.push(v);
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let preds = tr.best_model.predict_raw(&x_vals);
            for (o, &yc) in ynn_cols.iter().enumerate() {
                if let Some(&val) = preds.get(o) {
                    if yc < self.grid[r_idx].len() {
                        self.grid[r_idx][yc] = format!("{val:.4}");
                    }
                }
            }
            predicted += 1;
        }
        self.xml_cache = tr.best_model.to_xml();
        self.status_err = None;
        self.status_msg = Some(format!("Calculated FFNN predictions for {predicted} rows into YNN."));
    }

    fn prop_description(&self) -> (&'static str, &'static str) {
        match self.selected_prop {
            "Learning Rate" => (
                "Learning Rate",
                "The learning rate is a parameter that determines how much an updating step influences the current value of the weights. Range(0...1), Default(0.02)",
            ),
            "Learning Rate Decay" => (
                "Learning Rate Decay",
                "Controls how quickly the learning rate decreases across iterations: lr(t) = lr0 / (1 + decay * t). Range(0...1), Default(1E-06)",
            ),
            "Momentum" => (
                "Momentum",
                "Fraction of the previous weight update added to the current step to accelerate convergence and damp oscillations. Range(0...1), Default(0.1)",
            ),
            "Hidden Layer Activation Func" => (
                "Hidden Layer Activation Func",
                "Non-linear activation function applied to hidden layer neurons (TanH, Sigmoid, ReLU, or Linear). Default(TanH)",
            ),
            "Output Layer Activation Func" => (
                "Output Layer Activation Func",
                "Activation function applied to output neurons. Use Linear for continuous regression targets. Default(Linear)",
            ),
            "Hidden Layers" => (
                "Hidden Layers",
                "Number of neurons per hidden layer (e.g. '16' or '36', or comma-separated '16, 8' for deep networks).",
            ),
            "Validation Data Partition" => (
                "Validation Data Partition",
                "Percentage of active (non-'E') rows held out for validation MSE tracking. Range(5...50), Default(20)",
            ),
            "Training Data Partition" => (
                "Training Data Partition",
                "Percentage of active rows used for backpropagation weight updates. Default(80)",
            ),
            "Batch Size" => (
                "Batch Size",
                "Number of training samples evaluated per gradient update (1 = stochastic gradient descent). Default(1)",
            ),
            "Stopping MSE" => (
                "Stopping MSE",
                "Training stops automatically once scaled Mean Squared Error falls below this threshold. Default(1E-05)",
            ),
            "Max Iteration Rounds" => (
                "Max Iteration Rounds",
                "Maximum number of training epochs before stopping automatically. Default(1000000 / 500)",
            ),
            _ => (
                "Structure",
                "Inputs (X) and Outputs (Y) are automatically detected from Row 1 tags when you click Apply Data.",
            ),
        }
    }
}

// ============================================================================
// RNN Sub-Tool State (Figs. 12–18)
// ============================================================================

struct RnnToolState {
    grid: Vec<Vec<String>>,
    active_cell: (usize, usize),
    // Fig. 13 Hyperparameters
    momentum_str: String,
    supervising_str: String,
    val_method: RnnValidationMethod,
    regularized: bool,
    reg_param_str: String,
    batch_prob_str: String,
    val_pct_str: String,
    train_pct_str: String,
    stop_mse_str: String,
    max_iter_str: String,
    selected_prop: &'static str,
    // Runtime state
    parsed: Option<ParsedNnDataset>,
    trainer: Option<RnnTrainer>,
    show_val_parity: bool,
    show_train_parity: bool,
    show_val_mse: bool,
    show_train_mse: bool,
    show_nn_compare: bool,
    show_orig_compare: bool,
    show_rnn_picture: bool,
    show_xml_window: bool,
    xml_cache: String,
    show_paste_modal: bool,
    paste_buf: String,
    show_file_modal: bool,
    file_modal_mode: FileModalMode,
    file_path: String,
    status_msg: Option<String>,
    status_err: Option<String>,
}

impl Default for RnnToolState {
    fn default() -> Self {
        Self::with_fig12_iris_sample()
    }
}

impl RnnToolState {
    fn with_fig12_iris_sample() -> Self {
        // Exact Fig. 12 layout: RULE | X | X | X | X | Species | (blank) | Y | YNN
        let mut grid = vec![
            vec![
                "RULE".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                "X".into(),
                String::new(),
                String::new(),
                "Y".into(),
                "YNN".into(),
            ],
            vec![
                String::new(),
                "SepalL".into(),
                "SepalW".into(),
                "PetalL".into(),
                "PetalW".into(),
                "Species".into(),
                String::new(),
                "Class".into(),
                "Pred".into(),
            ],
            vec![String::new(); 9],
        ];
        // Iris-setosa (class 1), Iris-versicolor (class 2), Iris-virginica (class 3)
        let iris_rows: &[(&str, f64, f64, f64, f64, &str, f64)] = &[
            ("", 5.1, 3.5, 1.4, 0.2, "Iris-setosa", 1.0),
            ("", 4.9, 3.0, 1.4, 0.2, "Iris-setosa", 1.0),
            ("E", 4.7, 3.2, 1.3, 0.2, "Iris-setosa", 1.0),
            ("E", 4.6, 3.1, 1.5, 0.2, "Iris-setosa", 1.0),
            ("", 5.0, 3.6, 1.4, 0.2, "Iris-setosa", 1.0),
            ("", 5.4, 3.9, 1.7, 0.4, "Iris-setosa", 1.0),
            ("", 4.6, 3.4, 1.4, 0.3, "Iris-setosa", 1.0),
            ("", 5.0, 3.4, 1.5, 0.2, "Iris-setosa", 1.0),
            ("", 4.8, 3.4, 1.6, 0.2, "Iris-setosa", 1.0),
            ("", 5.2, 3.5, 1.5, 0.2, "Iris-setosa", 1.0),
            ("", 7.0, 3.2, 4.7, 1.4, "Iris-versicolor", 2.0),
            ("", 6.4, 3.2, 4.5, 1.5, "Iris-versicolor", 2.0),
            ("", 6.9, 3.1, 4.9, 1.5, "Iris-versicolor", 2.0),
            ("E", 5.5, 2.3, 4.0, 1.3, "Iris-versicolor", 2.0),
            ("", 6.5, 2.8, 4.6, 1.5, "Iris-versicolor", 2.0),
            ("", 5.7, 2.8, 4.5, 1.3, "Iris-versicolor", 2.0),
            ("", 6.3, 3.3, 4.7, 1.6, "Iris-versicolor", 2.0),
            ("", 6.1, 2.9, 4.7, 1.4, "Iris-versicolor", 2.0),
            ("", 6.3, 3.3, 6.0, 2.5, "Iris-virginica", 3.0),
            ("", 5.8, 2.7, 5.1, 1.9, "Iris-virginica", 3.0),
            ("", 7.1, 3.0, 5.9, 2.1, "Iris-virginica", 3.0),
            ("E", 6.3, 2.9, 5.6, 1.8, "Iris-virginica", 3.0),
            ("", 6.5, 3.0, 5.8, 2.2, "Iris-virginica", 3.0),
            ("", 7.6, 3.0, 6.6, 2.1, "Iris-virginica", 3.0),
            ("", 6.7, 3.1, 5.6, 2.4, "Iris-virginica", 3.0),
            ("", 6.9, 3.2, 5.7, 2.3, "Iris-virginica", 3.0),
        ];
        for &(rule, x1, x2, x3, x4, sp, y) in iris_rows {
            grid.push(vec![
                rule.into(),
                format!("{x1}"),
                format!("{x2}"),
                format!("{x3}"),
                format!("{x4}"),
                sp.into(),
                String::new(),
                format!("{y:.0}"),
                String::new(),
            ]);
        }

        let mut st = Self {
            grid,
            active_cell: (3, 0),
            momentum_str: "1E-07".into(),
            supervising_str: "0.9".into(),
            val_method: RnnValidationMethod::Shepard,
            regularized: true,
            reg_param_str: "0.0002".into(),
            batch_prob_str: "0.8".into(),
            val_pct_str: "30".into(),
            train_pct_str: "70".into(),
            stop_mse_str: "1E-05".into(),
            max_iter_str: "250".into(),
            selected_prop: "Momentum",
            parsed: None,
            trainer: None,
            show_val_parity: true,
            show_train_parity: true,
            show_val_mse: true,
            show_train_mse: true,
            show_nn_compare: true,
            show_orig_compare: true,
            show_rnn_picture: false,
            show_xml_window: false,
            xml_cache: String::new(),
            show_paste_modal: false,
            paste_buf: String::new(),
            show_file_modal: false,
            file_modal_mode: FileModalMode::ExportXml,
            file_path: "/home/nando/.gemini/antigravity/scratch/rnn_model.xml".into(),
            status_msg: Some("Iris sample loaded (Fig. 12). Click 'Apply Data' then 'Learn'.".into()),
            status_err: None,
        };
        st.apply_data();
        st
    }

    fn build_config(&self) -> RnnConfig {
        RnnConfig {
            momentum: parse_flex_f64(&self.momentum_str).unwrap_or(1e-7).clamp(0.0, 0.95),
            supervising_param: parse_flex_f64(&self.supervising_str).unwrap_or(0.9).clamp(0.05, 2.0),
            validation_method: self.val_method,
            regularized: self.regularized,
            regularization_param: parse_flex_f64(&self.reg_param_str).unwrap_or(0.0002).max(0.0),
            batch_probability: parse_flex_f64(&self.batch_prob_str).unwrap_or(0.8).clamp(0.1, 1.0),
            validation_pct: parse_flex_f64(&self.val_pct_str).unwrap_or(30.0).clamp(5.0, 50.0),
            training_pct: parse_flex_f64(&self.train_pct_str).unwrap_or(70.0).clamp(50.0, 95.0),
            stopping_mse: parse_flex_f64(&self.stop_mse_str).unwrap_or(1e-5).max(1e-12),
            max_iterations: self.max_iter_str.trim().parse::<usize>().unwrap_or(250).max(1),
            extra_hidden_neurons: 1,
        }
    }

    fn apply_data(&mut self) {
        match parse_nn_table(&self.grid) {
            Ok(ds) => {
                let cfg = self.build_config();
                match RnnTrainer::from_dataset(&ds, cfg) {
                    Ok(tr) => {
                        let msg = format!(
                            "RNN Data applied: {} neurons ({} X + 1 hidden + {} Y), {} active rows ({} excluded).",
                            tr.model.n_neurons,
                            ds.x_cols.len(),
                            ds.y_cols.len(),
                            ds.active_count,
                            ds.excluded_count
                        );
                        self.xml_cache = tr.best_model.to_xml();
                        self.parsed = Some(ds);
                        self.trainer = Some(tr);
                        self.status_err = None;
                        self.status_msg = Some(msg);
                    }
                    Err(e) => self.status_err = Some(e),
                }
            }
            Err(e) => self.status_err = Some(e),
        }
    }

    fn start_learn(&mut self) {
        if self.trainer.is_none() {
            self.apply_data();
        }
        let cfg = self.build_config();
        let need_reset = self.trainer.as_ref().map(|t| t.finished).unwrap_or(false);
        if need_reset {
            if let Some(ds) = &self.parsed {
                if let Ok(fresh) = RnnTrainer::from_dataset(ds, cfg.clone()) {
                    self.trainer = Some(fresh);
                }
            }
        }
        if let Some(tr) = &mut self.trainer {
            tr.config = cfg;
            tr.running = true;
            tr.finished = false;
            self.status_err = None;
            self.status_msg = Some("Preparing and training RNN...".into());
        }
    }

    fn abort_learn(&mut self) {
        if let Some(tr) = &mut self.trainer {
            tr.running = false;
            self.xml_cache = tr.best_model.to_xml();
            self.status_msg = Some(format!(
                "Aborted at iter {} (Best Val MSE = {:.6e}).",
                tr.iteration, tr.best_val_mse
            ));
        }
    }

    fn calculate_rnn(&mut self) {
        let (Some(ds), Some(tr)) = (&self.parsed, &self.trainer) else {
            self.status_err = Some("Apply Data and Learn before Calculate RNN.".into());
            return;
        };
        let ynn_cols = if ds.ynn_cols.is_empty() {
            let new_c = self.grid[0].len();
            for (r_idx, row) in self.grid.iter_mut().enumerate() {
                row.push(match r_idx {
                    0 => "YNN".into(),
                    1 => "Pred".into(),
                    _ => String::new(),
                });
            }
            vec![new_c]
        } else {
            ds.ynn_cols.clone()
        };

        let mut predicted = 0usize;
        for r_idx in 3..self.grid.len() {
            let mut x_vals = Vec::with_capacity(ds.x_cols.len());
            let mut ok = true;
            for &xc in &ds.x_cols {
                if let Some(v) = self.grid[r_idx].get(xc).and_then(|s| parse_flex_f64(s)) {
                    x_vals.push(v);
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            let preds = tr.best_model.predict_raw(&x_vals);
            for (o, &yc) in ynn_cols.iter().enumerate() {
                if let Some(&val) = preds.get(o) {
                    if yc < self.grid[r_idx].len() {
                        self.grid[r_idx][yc] = format!("{val:.6}");
                    }
                }
            }
            predicted += 1;
        }
        self.xml_cache = tr.best_model.to_xml();
        self.status_err = None;
        self.status_msg = Some(format!("Calculated RNN predictions for {predicted} rows into YNN."));
    }

    fn prop_description(&self) -> (&'static str, &'static str) {
        match self.selected_prop {
            "Momentum" => (
                "Momentum",
                "Between 0-1. The closer it is to one the more momentum there is. Default(1E-07)",
            ),
            "Supervising Parameter" => (
                "Supervising Parameter",
                "Scales the gradient update rate for excitation (W+) and inhibition (W-) weights. Default(0.9)",
            ),
            "Validation method" => (
                "Validation method",
                "Strategy for selecting validation samples: Shepard (quantile coverage), Random, or Uniform stride.",
            ),
            "Regularized" => (
                "Regularized",
                "Enables L2 weight decay regularization to prevent overfitting in recurrent loops. Default(True)",
            ),
            "Regularization parameter" => (
                "Regularization parameter",
                "Strength of L2 regularization applied to W+ and W- weights when Regularized is True. Default(0.0002)",
            ),
            "Batch Probability" => (
                "Batch Probability",
                "Probability that each training sample is included in a stochastic iteration update. Range(0.1...1), Default(0.8)",
            ),
            "Validation Data Partition" => (
                "Validation Data Partition",
                "Percentage of active rows assigned to the validation set. Default(30)",
            ),
            "Training Data Partition" => (
                "Training Data Partition",
                "Percentage of active rows assigned to the training set. Default(70)",
            ),
            "Stopping MSE" => (
                "Stopping MSE",
                "Training stops automatically when scaled MSE falls below this value. Default(1E-05)",
            ),
            _ => (
                "Max Iteration Rounds",
                "Maximum number of Gelenbe RNN training iterations. Default(1000)",
            ),
        }
    }
}

// ============================================================================
// PSO Sub-Tool State (§95.6 — Figs. 20–24)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PsoExample {
    CrossInTray,
    Rosenbrock,
    Rastrigin,
    Himmelblau,
    Ackley,
    SphereSin,
}

impl PsoExample {
    const ALL: &'static [PsoExample] = &[
        PsoExample::CrossInTray,
        PsoExample::Rosenbrock,
        PsoExample::Rastrigin,
        PsoExample::Himmelblau,
        PsoExample::Ackley,
        PsoExample::SphereSin,
    ];

    fn title(self) -> &'static str {
        match self {
            PsoExample::CrossInTray => "Cross-in-tray Function",
            PsoExample::Rosenbrock => "Rosenbrock Function",
            PsoExample::Rastrigin => "Rastrigin Function",
            PsoExample::Himmelblau => "Himmelblau Function",
            PsoExample::Ackley => "Ackley Function",
            PsoExample::SphereSin => "Shifted Sphere + Sin",
        }
    }

    fn formula_display(self) -> &'static str {
        match self {
            PsoExample::CrossInTray => {
                "f(x,y) = -0.0001 * ( |sin(x) sin(y) exp(|100 - sqrt(x^2+y^2)/pi|)| + 1 )^0.1"
            }
            PsoExample::Rosenbrock => "f(x,y) = (1 - x)^2 + 100 * (y - x^2)^2",
            PsoExample::Rastrigin => "f(x,y) = 20 + (x^2 - 10 cos(2*pi*x)) + (y^2 - 10 cos(2*pi*y))",
            PsoExample::Himmelblau => "f(x,y) = (x^2 + y - 11)^2 + (x + y^2 - 7)^2",
            PsoExample::Ackley => {
                "f(x,y) = -20 exp(-0.2 sqrt(0.5(x^2+y^2))) - exp(0.5(cos(2*pi*x)+cos(2*pi*y))) + e + 20"
            }
            PsoExample::SphereSin => "f(x,y) = (x - 3)^2 + (y + 2)^2 + |sin(x - 3)|",
        }
    }

    fn excel_formula(self) -> &'static str {
        match self {
            PsoExample::CrossInTray => {
                "=-0.0001*(ABS(SIN(B2)*SIN(C2))*EXP(ABS(100-SQRT(B2^2+C2^2)/PI()))+1)^0.1"
            }
            PsoExample::Rosenbrock => "=(1-B2)^2 + 100*(C2-B2^2)^2",
            PsoExample::Rastrigin => {
                "=20 + (B2^2 - 10*COS(2*PI()*B2)) + (C2^2 - 10*COS(2*PI()*C2))"
            }
            PsoExample::Himmelblau => "=(B2^2 + C2 - 11)^2 + (B2 + C2^2 - 7)^2",
            PsoExample::Ackley => {
                "=-20*EXP(-0.2*SQRT(0.5*(B2^2+C2^2))) - EXP(0.5*(COS(2*PI()*B2)+COS(2*PI()*C2))) + E + 20"
            }
            PsoExample::SphereSin => "=(B2-3)^2 + (C2+2)^2 + ABS(SIN(B2-3))",
        }
    }

    fn domain(self) -> (f64, f64) {
        match self {
            PsoExample::CrossInTray | PsoExample::SphereSin => (-10.0, 10.0),
            PsoExample::Rosenbrock | PsoExample::Himmelblau | PsoExample::Ackley => (-5.0, 5.0),
            PsoExample::Rastrigin => (-5.12, 5.12),
        }
    }

    fn domain_text(self) -> &'static str {
        match self {
            PsoExample::CrossInTray | PsoExample::SphereSin => "-10 <= x, y <= 10",
            PsoExample::Rosenbrock | PsoExample::Himmelblau | PsoExample::Ackley => {
                "-5 <= x, y <= 5"
            }
            PsoExample::Rastrigin => "-5.12 <= x, y <= 5.12",
        }
    }

    fn known_min_lines(self) -> &'static [&'static str] {
        match self {
            PsoExample::CrossInTray => &[
                "f( 1.34941, -1.34941) = -2.06261",
                "f( 1.34941,  1.34941) = -2.06261",
                "f(-1.34941,  1.34941) = -2.06261",
                "f(-1.34941, -1.34941) = -2.06261",
            ],
            PsoExample::Rosenbrock => &["f(1.0, 1.0) = 0.0"],
            PsoExample::Rastrigin => &["f(0.0, 0.0) = 0.0"],
            PsoExample::Himmelblau => &[
                "f( 3.0000,  2.0000) = 0.0",
                "f(-2.8051,  3.1313) = 0.0",
                "f(-3.7793, -3.2832) = 0.0",
                "f( 3.5844, -1.8481) = 0.0",
            ],
            PsoExample::Ackley => &["f(0.0, 0.0) = 0.0"],
            PsoExample::SphereSin => &["f(3.0, -2.0) = 0.0"],
        }
    }

    fn eval_preview_z(self, x: f64, y: f64) -> f64 {
        match self {
            PsoExample::CrossInTray => {
                let r = (x * x + y * y).sqrt() / std::f64::consts::PI;
                -0.0001 * ((x.sin() * y.sin() * (100.0 - r).abs().clamp(-20.0, 20.0).exp()).abs() + 1.0).powf(0.1)
            }
            PsoExample::Rosenbrock => ((1.0 - x).powi(2) + 100.0 * (y - x * x).powi(2)).ln_1p(),
            PsoExample::Rastrigin => {
                20.0 + (x * x - 10.0 * (std::f64::consts::TAU * x).cos())
                    + (y * y - 10.0 * (std::f64::consts::TAU * y).cos())
            }
            PsoExample::Himmelblau => ((x * x + y - 11.0).powi(2) + (x + y * y - 7.0).powi(2)).ln_1p(),
            PsoExample::Ackley => {
                -20.0 * (-0.2 * (0.5 * (x * x + y * y)).sqrt()).exp()
                    - (0.5 * ((std::f64::consts::TAU * x).cos() + (std::f64::consts::TAU * y).cos())).exp()
                    + std::f64::consts::E
                    + 20.0
            }
            PsoExample::SphereSin => (x - 3.0).powi(2) + (y + 2.0).powi(2) + (x - 3.0).sin().abs(),
        }
    }
}

struct PsoToolState {
    grid: Vec<Vec<String>>,
    comments: HashMap<(usize, usize), String>,
    active_cell: (usize, usize),
    current_example: Option<PsoExample>,
    show_comments: bool,
    // Fig. 22 PSO Properties -> Parameters
    particles_per_var_str: String,
    extra_particles_str: String,
    max_rounds_str: String,
    boundary_cond: PsoBoundaryCondition,
    use_diversity: bool,
    w_str: String,
    w_min_str: String,
    w_damp_str: String,
    c1_str: String,
    c2_str: String,
    c3_str: String,
    // Fig. 22 PSO Properties -> Decision Variable
    lower_bound_str: String,
    upper_bound_str: String,
    selected_prop: &'static str,
    // File modal
    show_file_modal: bool,
    file_modal_save: bool,
    file_path_str: String,
    // Optimizer & status
    optimizer: Option<PsoOptimizer>,
    status_msg: Option<String>,
    status_err: Option<String>,
}

impl Default for PsoToolState {
    fn default() -> Self {
        let mut st = Self {
            grid: vec![vec![String::new(); 6]; 10],
            comments: HashMap::new(),
            active_cell: (1, 3),
            current_example: Some(PsoExample::CrossInTray),
            show_comments: true,
            particles_per_var_str: "50".into(),
            extra_particles_str: "300".into(),
            max_rounds_str: "1000".into(),
            boundary_cond: PsoBoundaryCondition::Damping,
            use_diversity: false,
            w_str: "0.9".into(),
            w_min_str: "0.4".into(),
            w_damp_str: "0.999".into(),
            c1_str: "0.5".into(),
            c2_str: "1.5".into(),
            c3_str: "2".into(),
            lower_bound_str: "[Min]".into(),
            upper_bound_str: "[Max]".into(),
            selected_prop: "W",
            show_file_modal: false,
            file_modal_save: false,
            file_path_str: "/tmp/hsc_pso_sheet.csv".into(),
            optimizer: None,
            status_msg: None,
            status_err: None,
        };
        st.load_example(PsoExample::CrossInTray);
        st
    }
}

impl PsoToolState {
    fn load_example(&mut self, ex: PsoExample) {
        self.grid = vec![vec![String::new(); 6]; 10];
        self.grid[0][0] = "PSO".into();
        self.grid[0][1] = "x".into();
        self.grid[0][2] = "y".into();
        self.grid[0][3] = "f(x,y)".into();
        // Leave B2 and C2 blank initially just like Fig. 21; D2 holds the Excel formula
        self.grid[1][1] = String::new();
        self.grid[1][2] = String::new();
        self.grid[1][3] = ex.excel_formula().into();

        let (lo, hi) = ex.domain();
        let tag_x = format!("PSO_X/{lo}/{hi}");
        self.comments.clear();
        self.comments.insert((1, 1), tag_x.clone());
        self.comments.insert((1, 2), tag_x);
        self.comments.insert((1, 3), "PSO_M".into());

        self.active_cell = (1, 3);
        self.current_example = Some(ex);
        self.optimizer = None;
        self.status_err = None;
        self.status_msg = Some(format!(
            "Loaded '{}'. Click Optimize to find the minimum.",
            ex.title()
        ));
    }

    fn default_bounds(&self) -> (f64, f64) {
        let lo = parse_flex_f64(&self.lower_bound_str).unwrap_or(-10.0);
        let hi = parse_flex_f64(&self.upper_bound_str).unwrap_or(10.0);
        if lo < hi {
            (lo, hi)
        } else {
            (-10.0, 10.0)
        }
    }

    fn set_active_as_x(&mut self) {
        let lo_opt = parse_flex_f64(&self.lower_bound_str);
        let hi_opt = parse_flex_f64(&self.upper_bound_str);
        let comment = match (lo_opt, hi_opt) {
            (Some(lo), Some(hi)) => format!("PSO_X/{lo}/{hi}"),
            _ => "PSO_X//".into(),
        };
        self.comments.insert(self.active_cell, comment.clone());
        let cref = cell_ref(self.active_cell.0, self.active_cell.1);
        self.status_msg = Some(format!("Assigned {cref} with comment '{comment}'."));
    }

    fn set_active_as_m(&mut self) {
        self.comments.insert(self.active_cell, "PSO_M".into());
        let cref = cell_ref(self.active_cell.0, self.active_cell.1);
        self.status_msg = Some(format!("Assigned {cref} as objective function (PSO_M)."));
    }

    fn eval_env(&self) -> HashMap<String, f64> {
        let mut env = HashMap::new();
        for (r, row) in self.grid.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                let trimmed = val.trim();
                if let Some(v) = parse_flex_f64(trimmed) {
                    env.insert(cell_ref(r, c), v);
                } else if trimmed.is_empty()
                    && self
                        .comments
                        .get(&(r, c))
                        .map(|t| t.starts_with("PSO_X"))
                        .unwrap_or(false)
                {
                    env.insert(cell_ref(r, c), 0.0);
                }
            }
        }
        env
    }

    fn evaluate_formula_cell(&self, r: usize, c: usize) -> Option<f64> {
        let raw = self.grid.get(r)?.get(c)?.trim();
        if !raw.starts_with('=') {
            return None;
        }
        let env = self.eval_env();
        eval_excel_expr(raw, &env).ok()
    }

    fn prop_description(&self) -> (&'static str, &'static str) {
        match self.selected_prop {
            "Particles Per Variable" => (
                "Particles Per Variable",
                "Number of swarm particles added per decision variable (default: 50).",
            ),
            "Extra Particles" => (
                "Extra Particles",
                "Base number of extra swarm particles independent of variable count (default: 300).",
            ),
            "Max Rounds" => (
                "Max Rounds",
                "Maximum number of PSO iterations (rounds) before termination (default: 1000).",
            ),
            "Boundary Condition" => (
                "Boundary Condition",
                "Behavior when a particle exceeds domain bounds: Damping, Absorbing, Reflecting, or Invisible.",
            ),
            "Use Diversity" => (
                "Use Diversity",
                "Enable diversity-preserving repulsion term weighted by C3 to avoid premature local-minima collapse.",
            ),
            "W" => ("W", "Inertia Coefficient (0-1)"),
            "WMin" => (
                "WMin",
                "Minimum Inertia Coefficient lower floor during damping (0-1).",
            ),
            "WDamp" => (
                "WDamp",
                "Inertia Damping Ratio multiplied with W after each round (0-1).",
            ),
            "C1" => (
                "C1",
                "Personal Learning (Cognitive) Coefficient pulling particles toward their own best position.",
            ),
            "C2" => (
                "C2",
                "Global Learning (Social) Coefficient pulling particles toward the swarm's global best position.",
            ),
            "C3" => (
                "C3",
                "Diversity Coefficient active when Use Diversity is set to True.",
            ),
            "Lower Bound" => (
                "Lower Bound",
                "Default minimum domain bound applied when a PSO_X// variable omits an explicit lower bound.",
            ),
            "Upper Bound" => (
                "Upper Bound",
                "Default maximum domain bound applied when a PSO_X// variable omits an explicit upper bound.",
            ),
            _ => ("W", "Inertia Coefficient (0-1)"),
        }
    }

    fn start_optimize(&mut self) {
        let (def_lo, def_hi) = self.default_bounds();
        let mut vars = Vec::new();
        let mut obj_formulas = Vec::new();
        let base_cells = self.eval_env();

        let mut sorted_keys: Vec<(usize, usize)> = self.comments.keys().copied().collect();
        sorted_keys.sort();
        for (r, c) in sorted_keys {
            if let Some(tag) = self.comments.get(&(r, c)) {
                if let Some((lo, hi)) = parse_pso_x_comment_with_defaults(tag, def_lo, def_hi) {
                    vars.push(PsoVarSpec {
                        cell_ref: cell_ref(r, c),
                        min_val: lo,
                        max_val: hi,
                    });
                } else if tag.trim().eq_ignore_ascii_case("PSO_M") {
                    if let Some(f) = self.grid.get(r).and_then(|row| row.get(c)) {
                        if !f.trim().is_empty() {
                            obj_formulas.push(f.trim().to_string());
                        }
                    }
                }
            }
        }

        let cfg = PsoConfig {
            particles_per_variable: self.particles_per_var_str.trim().parse().unwrap_or(50),
            extra_particles: self.extra_particles_str.trim().parse().unwrap_or(300),
            max_rounds: self.max_rounds_str.trim().parse().unwrap_or(1000),
            boundary_condition: self.boundary_cond,
            use_diversity: self.use_diversity,
            w: parse_flex_f64(&self.w_str).unwrap_or(0.9),
            w_min: parse_flex_f64(&self.w_min_str).unwrap_or(0.4),
            w_damp: parse_flex_f64(&self.w_damp_str).unwrap_or(0.999),
            c1: parse_flex_f64(&self.c1_str).unwrap_or(0.5),
            c2: parse_flex_f64(&self.c2_str).unwrap_or(1.5),
            c3: parse_flex_f64(&self.c3_str).unwrap_or(2.0),
        };
        match PsoOptimizer::new(cfg, vars, obj_formulas, base_cells) {
            Ok(opt) => {
                self.optimizer = Some(opt);
                self.status_err = None;
                self.status_msg = Some("Running Particle Swarm Optimization...".into());
            }
            Err(e) => self.status_err = Some(e),
        }
    }

    fn write_best_to_grid(&mut self) {
        let Some(opt) = &self.optimizer else {
            return;
        };
        for (var, &val) in opt.vars.iter().zip(opt.gbest_pos.iter()) {
            if let Some((r, c)) = parse_cell_ref(&var.cell_ref) {
                if r < self.grid.len() && c < self.grid[r].len() {
                    self.grid[r][c] = format!("{val:.9}");
                }
            }
        }
    }

    fn save_csv(&mut self, path: &str) {
        let csv = grid_to_csv(&self.grid);
        match std::fs::write(path, csv) {
            Ok(()) => {
                self.status_msg = Some(format!("Saved PSO sheet to {path}"));
                self.status_err = None;
            }
            Err(e) => self.status_err = Some(format!("Save failed: {e}")),
        }
    }

    fn open_csv(&mut self, path: &str) {
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                self.grid = vec![vec![String::new(); 6]; 10];
                apply_tsv_to_grid(&mut self.grid, &raw, 0);
                self.current_example = None;
                self.optimizer = None;
                self.status_msg = Some(format!("Loaded PSO sheet from {path}"));
                self.status_err = None;
            }
            Err(e) => self.status_err = Some(format!("Open failed: {e}")),
        }
    }
}

// ============================================================================
// Main DataProcessingModuleState
// ============================================================================

pub struct DataProcessingModuleState {
    pub active_tool: DataTool,
    theme: DataTheme,
    show_app_menu: bool,
    show_help_modal: bool,
    charts: ChartsToolState,
    ffnn: FfnnToolState,
    rnn: RnnToolState,
    pso: PsoToolState,
}

impl Default for DataProcessingModuleState {
    fn default() -> Self {
        Self {
            active_tool: DataTool::Home,
            theme: DataTheme::DevExpressStyle,
            show_app_menu: false,
            show_help_modal: false,
            charts: ChartsToolState::default(),
            ffnn: FfnnToolState::default(),
            rnn: RnnToolState::default(),
            pso: PsoToolState::default(),
        }
    }
}

impl DataProcessingModuleState {
    pub fn ui(&mut self, ctx: &egui::Context) {
        let mut vis = egui::Visuals::light();
        vis.panel_fill = self.theme.panel_fill();
        vis.window_fill = egui::Color32::WHITE;
        ctx.set_visuals(vis);

        // Advance live incremental training / optimization cleanly per frame
        if let Some(tr) = &mut self.ffnn.trainer {
            if tr.running {
                tr.step_epochs(8);
                if tr.finished {
                    self.ffnn.xml_cache = tr.best_model.to_xml();
                    self.ffnn.status_msg = Some(format!(
                        "FFNN finished at iter {} (Best Val MSE = {:.6e}). Click 'Calculate NN' to populate YNN.",
                        tr.iteration, tr.best_val_mse
                    ));
                } else {
                    ctx.request_repaint();
                }
            }
        }
        if let Some(tr) = &mut self.rnn.trainer {
            if tr.running {
                tr.step_epochs(4);
                if tr.finished {
                    self.rnn.xml_cache = tr.best_model.to_xml();
                    self.rnn.status_msg = Some(format!(
                        "RNN finished at iter {} (Best Val MSE = {:.6e}). Click 'Calculate RNN' to populate YNN.",
                        tr.iteration, tr.best_val_mse
                    ));
                } else {
                    ctx.request_repaint();
                }
            }
        }
        if let Some(opt) = &mut self.pso.optimizer {
            if opt.running {
                if let Err(e) = opt.step_iterations(10) {
                    opt.running = false;
                    self.pso.status_err = Some(e);
                } else {
                    let done = opt.finished;
                    let best_val = opt.gbest_val;
                    let it = opt.iteration;
                    self.pso.write_best_to_grid();
                    if done {
                        self.pso.status_msg = Some(format!(
                            "PSO finished at round {it}: Best f(x) = {best_val:.9}"
                        ));
                    } else {
                        ctx.request_repaint();
                    }
                }
            }
        }

        // Top Title / Application Orb Bar (Figs. 1, 2, 3)
        let title_str = match self.active_tool {
            DataTool::Home => "HSC Data",
            DataTool::Charts => "HSC Data - Charts",
            DataTool::Ffnn => "HSC Data - FFNN (Feedforward Neural Network)",
            DataTool::Rnn => "HSC Data - RNN (Random Neural Network)",
            DataTool::Pso => "HSC Data - PSO (Particle Swarm Optimization)",
        };

        egui::TopBottomPanel::top("data_proc_app_bar")
            .frame(
                egui::Frame::none()
                    .fill(self.theme.ribbon_fill())
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Top-left Application Menu button (Fig. 1 & Fig. 2)
                    let menu_btn = egui::Button::new(
                        egui::RichText::new(" ⚙ Menu ").strong().size(12.0),
                    )
                    .fill(if self.show_app_menu {
                        egui::Color32::from_rgb(191, 219, 254)
                    } else {
                        egui::Color32::from_rgb(225, 232, 245)
                    })
                    .min_size(egui::vec2(72.0, 24.0));
                    if ui
                        .add(menu_btn)
                        .on_hover_text("Open HSC Data Menu (Charts / FFNN / RNN / PSO / Close / Theme)")
                        .clicked()
                    {
                        self.show_app_menu = !self.show_app_menu;
                    }

                    ui.separator();
                    ui.label(egui::RichText::new(title_str).strong().size(13.5));

                    if self.active_tool != DataTool::Home {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    egui::RichText::new("✖ Close Editor")
                                        .color(egui::Color32::from_rgb(185, 28, 28))
                                        .strong(),
                                )
                                .on_hover_text("Close current tool to return to main view (Fig. 2)")
                                .clicked()
                            {
                                self.close_active_tool();
                            }
                        });
                    }
                });
            });

        // Bottom Status Bar (Fig. 1 & Fig. 3 footer)
        egui::TopBottomPanel::bottom("data_proc_footer")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(235, 238, 244))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(10.0, 3.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Soryntel HSC Data  |  Chapter 95 Data Processing")
                            .size(10.5)
                            .color(egui::Color32::from_rgb(80, 90, 105)),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("Theme: {}", self.theme.label()))
                                .size(10.5)
                                .color(egui::Color32::from_rgb(95, 105, 120)),
                        );
                    });
                });
            });

        // Active workspace body
        match self.active_tool {
            DataTool::Home => self.ui_home_view(ctx),
            DataTool::Charts => self.ui_charts_tool(ctx),
            DataTool::Ffnn => self.ui_ffnn_tool(ctx),
            DataTool::Rnn => self.ui_rnn_tool(ctx),
            DataTool::Pso => self.ui_pso_tool(ctx),
        }

        // Application Menu Overlay (Fig. 2)
        if self.show_app_menu {
            self.ui_app_menu_popup(ctx);
        }

        if self.show_help_modal {
            let mut open = self.show_help_modal;
            egui::Window::new("HSC Data — Help and Documents (Ch. 95)")
                .open(&mut open)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    ui.strong("Data Processing Module Overview");
                    ui.separator();
                    ui.label("1. Charts: Import/paste data or open .HSCChart files, create multiple charts, clone charts, run Data Analytics (stats + regression), modify chart style, or switch to Dashboard Layout.");
                    ui.label("2. FFNN (Feedforward Neural Network): Assign RULE/X/Y/YNN in Row 1, names in Rows 2–3, numeric data starting at Row 4 ('E'/'e' excludes rows from training). Click Apply Data -> Learn -> Calculate NN -> View NN Picture / View NN XML File / Export.");
                    ui.label("3. RNN (Random Neural Network): Recurrent Gelenbe network with excitation/inhibition loops, Shepard/Random validation, progress bar + Abort, circular topology diagram (red=inputs, black=output), and XML export.");
                    ui.label("4. PSO (Particle Swarm Optimization): Mark variables with Set X (PSO_X/min/max) and objective formula cells with Set M (PSO_M), then click Optimize.");
                    ui.separator();
                    ui.label("Note (Fig. 2): Close the currently open tool via 'Close' before switching to another tool.");
                });
            self.show_help_modal = open;
        }
    }

    pub fn close_active_tool(&mut self) {
        if let Some(tr) = &mut self.ffnn.trainer {
            tr.running = false;
        }
        if let Some(tr) = &mut self.rnn.trainer {
            tr.running = false;
        }
        if let Some(opt) = &mut self.pso.optimizer {
            opt.running = false;
        }
        self.active_tool = DataTool::Home;
        self.show_app_menu = false;
    }

    fn ui_app_menu_popup(&mut self, ctx: &egui::Context) {
        let tool_open = self.active_tool != DataTool::Home;
        egui::Area::new(egui::Id::new("hsc_data_app_menu_area"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(8.0, 34.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgb(248, 250, 253))
                    .stroke(egui::Stroke::new(1.2, egui::Color32::from_rgb(160, 174, 192)))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_width(250.0);
                        // Fig. 2: Tools are disabled when a tool is currently open!
                        let menu_item = |ui: &mut egui::Ui,
                                         enabled: bool,
                                         icon: &str,
                                         title: &str,
                                         subtitle: &str|
                         -> bool {
                            ui.add_enabled_ui(enabled, |ui| {
                                let btn = egui::Button::new(
                                    egui::RichText::new(format!("{icon}  {title}\n      {subtitle}"))
                                        .size(11.5),
                                )
                                .min_size(egui::vec2(236.0, 38.0))
                                .fill(if enabled {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::from_rgb(238, 240, 244)
                                });
                                ui.add(btn).clicked()
                            })
                            .inner
                        };

                        if menu_item(ui, !tool_open, "📊", "Charts", "Charts Module") {
                            self.active_tool = DataTool::Charts;
                            self.show_app_menu = false;
                        }
                        if menu_item(ui, !tool_open, "🧠", "FFNN", "Feedforward Neural Network") {
                            self.active_tool = DataTool::Ffnn;
                            self.show_app_menu = false;
                        }
                        if menu_item(ui, !tool_open, "🕸", "RNN", "Random Neural Network") {
                            self.active_tool = DataTool::Rnn;
                            self.show_app_menu = false;
                        }
                        if menu_item(ui, !tool_open, "✦", "PSO", "Particle Swarm Optimizer") {
                            self.active_tool = DataTool::Pso;
                            self.show_app_menu = false;
                        }

                        ui.separator();
                        if menu_item(ui, tool_open, "✖", "Close", "Close Editor") {
                            self.close_active_tool();
                        }
                        if menu_item(ui, true, "❓", "Help", "Help and Documents") {
                            self.show_help_modal = true;
                            self.show_app_menu = false;
                        }

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Theme");
                            egui::ComboBox::from_id_salt("data_proc_theme_combo")
                                .width(165.0)
                                .selected_text(self.theme.label())
                                .show_ui(ui, |ui| {
                                    for &th in DataTheme::ALL {
                                        ui.selectable_value(&mut self.theme, th, th.label());
                                    }
                                });
                        });

                        ui.separator();
                        if menu_item(ui, true, "⏻", "Exit", "Exit Program") {
                            self.show_app_menu = false;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
            });
    }

    // ========================================================================
    // Fig. 1 — Home Screen with 4 Vector Artwork Cards
    // ========================================================================

    fn ui_home_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 249))
                    .inner_margin(egui::Margin::same(24.0)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    egui::Grid::new("hsc_data_home_2x2")
                        .spacing(egui::vec2(42.0, 26.0))
                        .show(ui, |ui| {
                            if draw_home_tool_card(
                                ui,
                                "Charts - Data Visualization",
                                draw_art_charts_surface,
                            ) {
                                self.active_tool = DataTool::Charts;
                            }
                            if draw_home_tool_card(
                                ui,
                                "RNN - Random Neural Network",
                                draw_art_rnn_ring,
                            ) {
                                self.active_tool = DataTool::Rnn;
                            }
                            ui.end_row();

                            if draw_home_tool_card(
                                ui,
                                "FFNN - Feedforward Neural Network",
                                draw_art_ffnn_layers,
                            ) {
                                self.active_tool = DataTool::Ffnn;
                            }
                            if draw_home_tool_card(
                                ui,
                                "PSO - Particle Swarm Optimization",
                                draw_art_pso_swarm,
                            ) {
                                self.active_tool = DataTool::Pso;
                            }
                            ui.end_row();
                        });
                });
            });
    }

    // ========================================================================
    // Fig. 3 — Charts Tool
    // ========================================================================

    fn ui_charts_tool(&mut self, ctx: &egui::Context) {
        // Ribbon Bar matching Fig. 3: File | Tools | View | Help
        egui::TopBottomPanel::top("charts_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(self.theme.ribbon_fill())
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // File Group
                    ribbon_group(ui, "File", |ui| {
                        if ribbon_btn(ui, "📄", "New", false) {
                            self.charts.reset_blank();
                        }
                        if ribbon_btn(ui, "💾", "Save", false) {
                            self.charts.file_modal_is_save = true;
                            self.charts.show_file_modal = true;
                        }
                        if ribbon_btn(ui, "📂", "Open...", false) {
                            self.charts.file_modal_is_save = false;
                            self.charts.show_file_modal = true;
                        }
                    });
                    // Tools Group
                    ribbon_group(ui, "Tools", |ui| {
                        if ribbon_btn(ui, "📈", "Add New\nChart", false) {
                            let id = self.charts.next_chart_id;
                            self.charts.next_chart_id += 1;
                            self.charts.charts.push(ChartSpec::new(id));
                            self.charts.active_chart = self.charts.charts.len() - 1;
                        }
                        if ribbon_btn(ui, "📊", "Data\nAnalytics", self.charts.show_analytics) {
                            self.charts.show_analytics = !self.charts.show_analytics;
                        }
                        if ribbon_btn(ui, "🔄", "Auto\nUpdate", self.charts.auto_update) {
                            self.charts.auto_update = !self.charts.auto_update;
                        }
                        if ribbon_btn(ui, "✏", "Modify\nChart", self.charts.show_modify) {
                            self.charts.show_modify = !self.charts.show_modify;
                        }
                        if ribbon_btn(ui, "📋", "Clone\nChart", false) {
                            if let Some(mut cloned) =
                                self.charts.charts.get(self.charts.active_chart).cloned()
                            {
                                let id = self.charts.next_chart_id;
                                self.charts.next_chart_id += 1;
                                cloned.id = id;
                                cloned.tab_name = format!("Chart - {id}");
                                self.charts.charts.push(cloned);
                                self.charts.active_chart = self.charts.charts.len() - 1;
                            }
                        }
                    });
                    // View Group
                    ribbon_group(ui, "View", |ui| {
                        if ribbon_btn(ui, "↺", "Reset to\nDefault Layout", false) {
                            self.charts.dashboard_layout = false;
                            self.charts.show_analytics = false;
                            self.charts.show_modify = false;
                        }
                        if ribbon_btn(
                            ui,
                            "⊞",
                            "Dashboard\nLayout",
                            self.charts.dashboard_layout,
                        ) {
                            self.charts.dashboard_layout = !self.charts.dashboard_layout;
                        }
                    });
                    // Help Group
                    ribbon_group(ui, "Help", |ui| {
                        if ribbon_btn(ui, "❓", "Help", false) {
                            self.show_help_modal = true;
                        }
                        if ribbon_btn(ui, "ℹ", "About", false) {
                            self.charts.show_about = true;
                        }
                    });

                    if let Some(msg) = &self.charts.status_msg {
                        ui.colored_label(egui::Color32::from_rgb(21, 128, 61), msg);
                    }
                    if let Some(err) = &self.charts.status_err {
                        ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                    }
                });
            });

        // Left Pane: Data Table (Fig. 3)
        egui::SidePanel::left("charts_data_table_panel")
            .resizable(true)
            .default_width(430.0)
            .min_width(240.0)
            .max_width(720.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("📋 Data Table");
                    ui.separator();
                    if ui.small_button("Paste TSV/CSV").clicked() {
                        self.charts.show_paste_modal = true;
                    }
                    if ui.small_button("+ Row").clicked() {
                        let cols = self.charts.sheets[self.charts.active_sheet].grid[0].len();
                        self.charts.sheets[self.charts.active_sheet]
                            .grid
                            .push(vec![String::new(); cols]);
                    }
                    if ui.small_button("+ Col").clicked() {
                        for row in &mut self.charts.sheets[self.charts.active_sheet].grid {
                            row.push(String::new());
                        }
                    }
                    if ui.small_button("Fig. 3 Sample").clicked() {
                        let s = ChartsToolState::with_fig3_sample();
                        self.charts.sheets = s.sheets;
                        self.charts.active_sheet = 0;
                    }
                });

                // Formula Bar (Fig. 3)
                let (ar, ac) = self.charts.active_cell;
                let cref = cell_ref(ar, ac);
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [44.0, 20.0],
                        egui::Button::new(egui::RichText::new(&cref).monospace().size(11.0)),
                    );
                    ui.label("fx");
                    if let Some(cell) = self.charts.sheets[self.charts.active_sheet]
                        .grid
                        .get_mut(ar)
                        .and_then(|r| r.get_mut(ac))
                    {
                        ui.add_sized(
                            [ui.available_width().max(120.0), 20.0],
                            egui::TextEdit::singleline(cell),
                        );
                    }
                });
                ui.separator();

                // Spreadsheet Grid
                let grid = &mut self.charts.sheets[self.charts.active_sheet].grid;
                let n_cols = grid.first().map(|r| r.len()).unwrap_or(0);
                egui::ScrollArea::both()
                    .id_salt("charts_grid_scroll")
                    .max_height((ui.available_height() - 32.0).max(160.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            row_num_cell(ui, 0, 28.0);
                            for c in 0..n_cols {
                                header_cell(ui, &col_letter(c), 76.0);
                            }
                        });
                        for (r_idx, row) in grid.iter_mut().enumerate() {
                            let bg = if r_idx == 0 {
                                egui::Color32::from_rgb(242, 245, 250)
                            } else if r_idx % 2 == 1 {
                                egui::Color32::WHITE
                            } else {
                                EXCEL_ALT_ROW_BG
                            };
                            ui.horizontal(|ui| {
                                row_num_cell(ui, r_idx + 1, 28.0);
                                for (c_idx, cell) in row.iter_mut().enumerate() {
                                    let is_act = self.charts.active_cell == (r_idx, c_idx);
                                    widget_cell(ui, 76.0, bg, is_act, |ui| {
                                        let r = ui.text_edit_singleline(cell);
                                        if r.gained_focus() {
                                            self.charts.active_cell = (r_idx, c_idx);
                                        }
                                    });
                                }
                            });
                        }
                    });

                // Bottom Sheet Tabs (Sheet1 | +)
                ui.separator();
                ui.horizontal(|ui| {
                    for s_idx in 0..self.charts.sheets.len() {
                        let sel = self.charts.active_sheet == s_idx;
                        if ui
                            .selectable_label(sel, &self.charts.sheets[s_idx].name)
                            .clicked()
                        {
                            self.charts.active_sheet = s_idx;
                        }
                    }
                    if ui.small_button("+").on_hover_text("Add new sheet").clicked() {
                        let n = self.charts.sheets.len() + 1;
                        let mut g = vec![vec![String::new(); 8]; 22];
                        g[0][0] = "X".into();
                        g[0][1] = "Y".into();
                        self.charts.sheets.push(ChartSheet {
                            name: format!("Sheet{n}"),
                            grid: g,
                        });
                        self.charts.active_sheet = self.charts.sheets.len() - 1;
                    }
                });
            });

        // Right Central Pane: Chart Tabs OR Dashboard Layout (Fig. 3)
        let (xs, series_list) = self.charts.extract_xy_series();
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                if self.charts.dashboard_layout {
                    ui.strong("Dashboard Layout — All Open Charts");
                    ui.separator();
                    let n = self.charts.charts.len();
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let cols = if n <= 1 { 1 } else { 2 };
                        let mut idx = 0;
                        while idx < n {
                            ui.columns(cols, |ui_cols| {
                                for c in 0..cols {
                                    let ci = idx + c;
                                    if ci < self.charts.charts.len() {
                                        let ch = &mut self.charts.charts[ci];
                                        draw_data_chart(&mut ui_cols[c], ch, &xs, &series_list, 280.0);
                                    }
                                }
                            });
                            idx += cols;
                        }
                    });
                } else {
                    // Chart tab bar (Chart - 1 × | Chart - 2 × ...)
                    let mut close_idx = None;
                    ui.horizontal(|ui| {
                        for (i, ch) in self.charts.charts.iter().enumerate() {
                            let sel = self.charts.active_chart == i;
                            if ui.selectable_label(sel, &ch.tab_name).clicked() {
                                self.charts.active_chart = i;
                            }
                            if self.charts.charts.len() > 1 && ui.small_button("×").clicked() {
                                close_idx = Some(i);
                            }
                        }
                    });
                    if let Some(ci) = close_idx {
                        self.charts.charts.remove(ci);
                        self.charts.active_chart =
                            self.charts.active_chart.min(self.charts.charts.len() - 1);
                    }
                    ui.separator();
                    let h = (ui.available_height() - 12.0).max(260.0);
                    if let Some(ch) = self.charts.charts.get_mut(self.charts.active_chart) {
                        draw_data_chart(ui, ch, &xs, &series_list, h);
                    }
                }
            });

        // Charts Tool Modals: Data Analytics, Modify Chart, Paste, File, About
        self.ui_charts_modals(ctx, &xs, &series_list);
    }

    fn ui_charts_modals(
        &mut self,
        ctx: &egui::Context,
        xs: &[f64],
        series_list: &[(String, Vec<f64>)],
    ) {
        if self.charts.show_analytics {
            let mut open = self.charts.show_analytics;
            egui::Window::new("Data Analytics")
                .open(&mut open)
                .resizable(true)
                .default_width(520.0)
                .show(ctx, |ui| {
                    ui.strong("Descriptive Statistics & Linear Regression");
                    ui.separator();
                    egui::Grid::new("charts_analytics_tbl")
                        .striped(true)
                        .spacing(egui::vec2(10.0, 4.0))
                        .show(ui, |ui| {
                            for h in ["Series", "N", "Mean", "StdDev", "Min", "Max", "Slope", "R²"] {
                                ui.strong(h);
                            }
                            ui.end_row();
                            for (name, ys) in series_list {
                                let clean: Vec<(f64, f64)> = xs
                                    .iter()
                                    .copied()
                                    .zip(ys.iter().copied())
                                    .filter(|(_, y)| y.is_finite())
                                    .collect();
                                let cxs: Vec<f64> = clean.iter().map(|p| p.0).collect();
                                let cys: Vec<f64> = clean.iter().map(|p| p.1).collect();
                                let st: ColumnAnalytics = compute_column_analytics(name, &cxs, &cys);
                                ui.label(&st.name);
                                ui.label(st.count.to_string());
                                ui.label(format!("{:.3}", st.mean));
                                ui.label(format!("{:.3}", st.std_dev));
                                ui.label(format!("{:.3}", st.min));
                                ui.label(format!("{:.3}", st.max));
                                ui.label(
                                    st.slope
                                        .map(|s| format!("{s:.3}"))
                                        .unwrap_or_else(|| "—".into()),
                                );
                                ui.label(
                                    st.r_squared
                                        .map(|r| format!("{r:.4}"))
                                        .unwrap_or_else(|| "—".into()),
                                );
                                ui.end_row();
                            }
                        });
                    ui.separator();
                    if let Some(ch) = self.charts.charts.get_mut(self.charts.active_chart) {
                        ui.checkbox(&mut ch.show_trendline, "Overlay Linear Regression Trendline");
                        ui.add(
                            egui::Slider::new(&mut ch.smooth_window, 1..=9)
                                .text("Moving Average Window"),
                        );
                    }
                });
            self.charts.show_analytics = open;
        }

        if self.charts.show_modify {
            let mut open = self.charts.show_modify;
            if let Some(ch) = self.charts.charts.get_mut(self.charts.active_chart) {
                egui::Window::new(format!("Modify Chart — {}", ch.tab_name))
                    .open(&mut open)
                    .default_width(340.0)
                    .show(ctx, |ui| {
                        egui::Grid::new("modify_chart_grid")
                            .num_columns(2)
                            .spacing(egui::vec2(8.0, 4.0))
                            .show(ui, |ui| {
                                ui.label("Chart Title:");
                                ui.text_edit_singleline(&mut ch.title);
                                ui.end_row();
                                ui.label("X Axis Label:");
                                ui.text_edit_singleline(&mut ch.x_axis_title);
                                ui.end_row();
                                ui.label("Y Axis Label:");
                                ui.text_edit_singleline(&mut ch.y_axis_title);
                                ui.end_row();
                                ui.label("Chart Style:");
                                egui::ComboBox::from_id_salt("chart_style_combo")
                                    .selected_text(ch.kind.label())
                                    .show_ui(ui, |ui| {
                                        for &k in ChartKind::ALL {
                                            ui.selectable_value(&mut ch.kind, k, k.label());
                                        }
                                    });
                                ui.end_row();
                            });
                        ui.checkbox(&mut ch.show_grid, "Show Gridlines");
                        ui.checkbox(&mut ch.auto_y, "Auto Y-Axis Range");
                        if !ch.auto_y {
                            ui.horizontal(|ui| {
                                ui.label("Y Min:");
                                ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut ch.y_min_str));
                                ui.label("Y Max:");
                                ui.add_sized([70.0, 20.0], egui::TextEdit::singleline(&mut ch.y_max_str));
                            });
                        }
                        ui.add(egui::Slider::new(&mut ch.line_width, 0.8..=4.0).text("Line Width"));
                    });
            }
            self.charts.show_modify = open;
        }

        if self.charts.show_paste_modal {
            let mut open = self.charts.show_paste_modal;
            let mut do_paste = false;
            egui::Window::new("Paste Data (TSV / CSV)")
                .open(&mut open)
                .default_width(420.0)
                .show(ctx, |ui| {
                    ui.label("Paste tab-, semicolon-, or comma-separated columns below:");
                    ui.add(
                        egui::TextEdit::multiline(&mut self.charts.paste_buf)
                            .desired_rows(8)
                            .desired_width(400.0),
                    );
                    if ui.button("Apply Paste").clicked() {
                        do_paste = true;
                    }
                });
            if do_paste {
                let txt = self.charts.paste_buf.clone();
                apply_tsv_to_grid(&mut self.charts.sheets[self.charts.active_sheet].grid, &txt, 0);
                self.charts.show_paste_modal = false;
            } else {
                self.charts.show_paste_modal = open;
            }
        }

        if self.charts.show_file_modal {
            let mut open = self.charts.show_file_modal;
            let mut do_act = false;
            let title = if self.charts.file_modal_is_save {
                "Save .HSCChart"
            } else {
                "Open .HSCChart / .CSV"
            };
            egui::Window::new(title).open(&mut open).default_width(420.0).show(ctx, |ui| {
                ui.label("Path:");
                ui.text_edit_singleline(&mut self.charts.file_path);
                if ui
                    .button(if self.charts.file_modal_is_save { "Save" } else { "Open" })
                    .clicked()
                {
                    do_act = true;
                }
            });
            if do_act {
                let p = self.charts.file_path.clone();
                if self.charts.file_modal_is_save {
                    self.charts.save_file(&p);
                } else {
                    self.charts.open_file(&p);
                }
                self.charts.show_file_modal = false;
            } else {
                self.charts.show_file_modal = open;
            }
        }

        if self.charts.show_about {
            let mut open = self.charts.show_about;
            egui::Window::new("About HSC Data — Charts")
                .open(&mut open)
                .default_width(340.0)
                .show(ctx, |ui| {
                    ui.strong("Soryntel HSC Data Module (Ch. 95)");
                    ui.label("Interactive data visualization, FFNN, Gelenbe RNN, and Particle Swarm Optimization.");
                });
            self.charts.show_about = open;
        }
    }

    // ========================================================================
    // Figs. 4–11 — FFNN Tool
    // ========================================================================

    fn ui_ffnn_tool(&mut self, ctx: &egui::Context) {
        let is_running = self.ffnn.trainer.as_ref().map(|t| t.running).unwrap_or(false);
        let has_trainer = self.ffnn.trainer.is_some();

        // Ribbon Bar matching Figs. 6, 8, 10: File | Training | Neural Network
        egui::TopBottomPanel::top("ffnn_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(self.theme.ribbon_fill())
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ribbon_group(ui, "File", |ui| {
                        if ribbon_btn(ui, "📄", "Sample\n(Fig. 4)", false) {
                            self.ffnn = FfnnToolState::with_fig4_sample();
                        }
                        if ribbon_btn(ui, "📋", "Paste\nData", false) {
                            self.ffnn.show_paste_modal = true;
                        }
                        if ribbon_btn(ui, "📂", "Open\n.xlsNN", false) {
                            self.ffnn.file_modal_mode = FileModalMode::OpenData;
                            self.ffnn.file_path =
                                "/home/nando/.gemini/antigravity/scratch/ffnn_data.csv".into();
                            self.ffnn.show_file_modal = true;
                        }
                        if ribbon_btn(ui, "💾", "Save\nData", false) {
                            self.ffnn.file_modal_mode = FileModalMode::SaveData;
                            self.ffnn.file_path =
                                "/home/nando/.gemini/antigravity/scratch/ffnn_data.csv".into();
                            self.ffnn.show_file_modal = true;
                        }
                    });
                    // Fig. 8: Training group
                    ribbon_group(ui, "Training", |ui| {
                        if ribbon_btn(ui, "✅", "Apply\nData", false) {
                            self.ffnn.apply_data();
                        }
                        ui.add_enabled_ui(!is_running, |ui| {
                            if ribbon_btn(ui, "▶", "Learn", is_running) {
                                self.ffnn.start_learn();
                            }
                        });
                        ui.add_enabled_ui(is_running, |ui| {
                            if ribbon_btn(ui, "⏹", "Stop", false) {
                                self.ffnn.stop_learn();
                            }
                        });
                    });
                    // Figs. 6 & 10: Neural Network group
                    ribbon_group(ui, "Neural Network", |ui| {
                        ui.add_enabled_ui(has_trainer && !is_running, |ui| {
                            if ribbon_btn(ui, "🧮", "Calculate\nNN", false) {
                                self.ffnn.calculate_nn();
                            }
                        });
                        ui.add_enabled_ui(has_trainer, |ui| {
                            if ribbon_btn(
                                ui,
                                "🌿",
                                "View NN\nPicture",
                                self.ffnn.show_nn_picture,
                            ) {
                                self.ffnn.show_nn_picture = true;
                            }
                            if ribbon_btn(
                                ui,
                                "📃",
                                "View NN\nXML File",
                                self.ffnn.show_xml_window,
                            ) {
                                if let Some(tr) = &self.ffnn.trainer {
                                    self.ffnn.xml_cache = tr.best_model.to_xml();
                                }
                                self.ffnn.show_xml_window = true;
                            }
                            if ribbon_btn(ui, "💾", "Export", false) {
                                if let Some(tr) = &self.ffnn.trainer {
                                    self.ffnn.xml_cache = tr.best_model.to_xml();
                                }
                                self.ffnn.file_modal_mode = FileModalMode::ExportXml;
                                self.ffnn.file_path =
                                    "/home/nando/.gemini/antigravity/scratch/ffnn_model.xml".into();
                                self.ffnn.show_file_modal = true;
                            }
                        });
                    });

                    if let Some(msg) = &self.ffnn.status_msg {
                        ui.colored_label(egui::Color32::from_rgb(21, 128, 61), msg);
                    }
                    if let Some(err) = &self.ffnn.status_err {
                        ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                    }
                });
            });

        // Right Side Panel: Properties Inspector (Fig. 5)
        egui::SidePanel::right("ffnn_properties_panel")
            .resizable(true)
            .default_width(275.0)
            .min_width(220.0)
            .max_width(380.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                ui.strong("Properties");
                ui.separator();
                let n_in = self.ffnn.parsed.as_ref().map(|d| d.x_cols.len()).unwrap_or(0);
                let n_out = self.ffnn.parsed.as_ref().map(|d| d.y_cols.len()).unwrap_or(0);
                let desc_h = 92.0;

                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - desc_h - 14.0).max(160.0))
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new("Hyperparameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(ui, "Learning Rate", &mut self.ffnn.lr_str, &mut self.ffnn.selected_prop);
                                prop_row_text(ui, "Learning Rate Decay", &mut self.ffnn.lr_decay_str, &mut self.ffnn.selected_prop);
                                prop_row_text(ui, "Momentum", &mut self.ffnn.momentum_str, &mut self.ffnn.selected_prop);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.ffnn.selected_prop == "Hidden Layer Activation Func", "Hidden Activation").clicked() {
                                        self.ffnn.selected_prop = "Hidden Layer Activation Func";
                                    }
                                    egui::ComboBox::from_id_salt("ffnn_hid_act")
                                        .width(86.0)
                                        .selected_text(self.ffnn.hidden_act.label())
                                        .show_ui(ui, |ui| {
                                            for &a in ActivationFunc::ALL {
                                                ui.selectable_value(&mut self.ffnn.hidden_act, a, a.label());
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.ffnn.selected_prop == "Output Layer Activation Func", "Output Activation").clicked() {
                                        self.ffnn.selected_prop = "Output Layer Activation Func";
                                    }
                                    egui::ComboBox::from_id_salt("ffnn_out_act")
                                        .width(86.0)
                                        .selected_text(self.ffnn.output_act.label())
                                        .show_ui(ui, |ui| {
                                            for &a in ActivationFunc::ALL {
                                                ui.selectable_value(&mut self.ffnn.output_act, a, a.label());
                                            }
                                        });
                                });
                            });

                        egui::CollapsingHeader::new("Structure")
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_readonly(ui, "Inputs (X)", &n_in.to_string(), &mut self.ffnn.selected_prop);
                                prop_row_text(ui, "Hidden Layers", &mut self.ffnn.hidden_layers_str, &mut self.ffnn.selected_prop);
                                prop_row_readonly(ui, "Outputs (Y)", &n_out.to_string(), &mut self.ffnn.selected_prop);
                            });

                        egui::CollapsingHeader::new("Data")
                            .default_open(true)
                            .show(ui, |ui| {
                                if prop_row_text(ui, "Validation Data Partition", &mut self.ffnn.val_pct_str, &mut self.ffnn.selected_prop) {
                                    if let Ok(v) = self.ffnn.val_pct_str.trim().parse::<f64>() {
                                        self.ffnn.train_pct_str = format!("{:.0}", (100.0 - v).clamp(50.0, 95.0));
                                    }
                                }
                                if prop_row_text(ui, "Training Data Partition", &mut self.ffnn.train_pct_str, &mut self.ffnn.selected_prop) {
                                    if let Ok(t) = self.ffnn.train_pct_str.trim().parse::<f64>() {
                                        self.ffnn.val_pct_str = format!("{:.0}", (100.0 - t).clamp(5.0, 50.0));
                                    }
                                }
                                prop_row_text(ui, "Batch Size", &mut self.ffnn.batch_size_str, &mut self.ffnn.selected_prop);
                            });

                        egui::CollapsingHeader::new("Stopping Condition")
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(ui, "Stopping MSE", &mut self.ffnn.stop_mse_str, &mut self.ffnn.selected_prop);
                                prop_row_text(ui, "Max Iteration Rounds", &mut self.ffnn.max_iter_str, &mut self.ffnn.selected_prop);
                            });
                    });

                // Bottom Parameter Explanation Card (Fig. 5)
                ui.separator();
                let (pname, pdesc) = self.ffnn.prop_description();
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.set_min_height(desc_h - 14.0);
                        ui.strong(pname);
                        ui.label(egui::RichText::new(pdesc).size(10.5));
                    });
            });

        // Central Workspace: Top Data Table (Fig. 4) + Bottom 3 Training Result Charts (Fig. 9)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                let total_h = ui.available_height();
                let table_h = (total_h * 0.48).max(170.0);
                let charts_h = (total_h - table_h - 24.0).max(200.0);

                // Top: Data Table (Fig. 4)
                ui.horizontal(|ui| {
                    ui.strong("Data Table (Row 1: RULE/X/Y/YNN • Row 2–3: Names/Units • Row 4+: Data)");
                    if ui.small_button("+ Row").clicked() {
                        let nc = self.ffnn.grid[0].len();
                        self.ffnn.grid.push(vec![String::new(); nc]);
                    }
                    if ui.small_button("+ Col").clicked() {
                        for r in &mut self.ffnn.grid {
                            r.push(String::new());
                        }
                    }
                });
                draw_nn_spreadsheet(ui, "ffnn_grid", &mut self.ffnn.grid, &mut self.ffnn.active_cell, table_h);

                ui.add_space(6.0);
                ui.separator();

                // Bottom: 3 Charts Side-by-Side matching Fig. 9
                if let Some(tr) = &self.ffnn.trainer {
                    let (tr_pts, va_pts) = tr.parity_points();
                    ui.columns(3, |cols| {
                        draw_parity_chart(
                            &mut cols[0],
                            "Parity Chart",
                            &tr_pts,
                            &va_pts,
                            -1.0,
                            1.0,
                            &mut self.ffnn.show_val_parity,
                            &mut self.ffnn.show_train_parity,
                            charts_h,
                        );
                        draw_mse_chart(
                            &mut cols[1],
                            "Performance MSE",
                            &tr.train_mse_history,
                            &tr.val_mse_history,
                            tr.best_iter,
                            &mut self.ffnn.show_val_mse,
                            &mut self.ffnn.show_train_mse,
                            charts_h,
                        );
                        draw_compare_values_chart(
                            &mut cols[2],
                            "Compare Values",
                            &tr_pts,
                            &va_pts,
                            &mut self.ffnn.show_nn_compare,
                            &mut self.ffnn.show_orig_compare,
                            charts_h,
                        );
                    });
                }
            });

        // FFNN Windows: View NN Picture (Fig. 7), View NN XML File (Fig. 11), Paste, File/Export
        if self.ffnn.show_nn_picture {
            let mut open = self.ffnn.show_nn_picture;
            egui::Window::new("Feedforward Neural Network FFNN")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 420.0])
                .show(ctx, |ui| {
                    if let Some(tr) = &self.ffnn.trainer {
                        draw_ffnn_picture_canvas(ui, &tr.best_model);
                    } else {
                        ui.label("Click 'Apply Data' first to initialize the FFNN topology.");
                    }
                });
            self.ffnn.show_nn_picture = open;
        }

        if self.ffnn.show_xml_window {
            let mut open = self.ffnn.show_xml_window;
            egui::Window::new("Neural Network XML file")
                .open(&mut open)
                .resizable(true)
                .default_size([560.0, 400.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Copy XML").clicked() {
                            ui.output_mut(|o| o.copied_text = self.ffnn.xml_cache.clone());
                            self.ffnn.status_msg = Some("Copied FFNN XML to clipboard.".into());
                        }
                        if ui.button("Export to File...").clicked() {
                            self.ffnn.file_modal_mode = FileModalMode::ExportXml;
                            self.ffnn.show_file_modal = true;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.ffnn.xml_cache)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(20),
                        );
                    });
                });
            self.ffnn.show_xml_window = open;
        }

        if self.ffnn.show_paste_modal {
            let mut open = self.ffnn.show_paste_modal;
            let mut do_p = false;
            egui::Window::new("Paste FFNN Data (Row 1 tags or Row 4+ raw data)")
                .open(&mut open)
                .default_width(440.0)
                .show(ctx, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.ffnn.paste_buf)
                            .desired_rows(8)
                            .desired_width(420.0),
                    );
                    if ui.button("Paste from Row 1").clicked() {
                        do_p = true;
                    }
                });
            if do_p {
                let txt = self.ffnn.paste_buf.clone();
                apply_tsv_to_grid(&mut self.ffnn.grid, &txt, 0);
                self.ffnn.show_paste_modal = false;
            } else {
                self.ffnn.show_paste_modal = open;
            }
        }

        if self.ffnn.show_file_modal {
            let mut open = self.ffnn.show_file_modal;
            let mut do_act = false;
            let title = match self.ffnn.file_modal_mode {
                FileModalMode::OpenData => "Open FFNN Data (.xlsNN / .csv)",
                FileModalMode::SaveData => "Save FFNN Data (.csv)",
                FileModalMode::ExportXml => "Export FFNN Model XML",
            };
            egui::Window::new(title).open(&mut open).default_width(420.0).show(ctx, |ui| {
                ui.label("File path:");
                ui.text_edit_singleline(&mut self.ffnn.file_path);
                if ui.button("Confirm").clicked() {
                    do_act = true;
                }
            });
            if do_act {
                let p = self.ffnn.file_path.clone();
                match self.ffnn.file_modal_mode {
                    FileModalMode::OpenData => match std::fs::read_to_string(&p) {
                        Ok(txt) => {
                            apply_tsv_to_grid(&mut self.ffnn.grid, &txt, 0);
                            self.ffnn.apply_data();
                        }
                        Err(e) => self.ffnn.status_err = Some(format!("Read error: {e}")),
                    },
                    FileModalMode::SaveData => match std::fs::write(&p, grid_to_csv(&self.ffnn.grid)) {
                        Ok(_) => self.ffnn.status_msg = Some(format!("Saved {p}")),
                        Err(e) => self.ffnn.status_err = Some(format!("Save error: {e}")),
                    },
                    FileModalMode::ExportXml => match std::fs::write(&p, &self.ffnn.xml_cache) {
                        Ok(_) => self.ffnn.status_msg = Some(format!("Exported FFNN XML to {p}")),
                        Err(e) => self.ffnn.status_err = Some(format!("Export error: {e}")),
                    },
                }
                self.ffnn.show_file_modal = false;
            } else {
                self.ffnn.show_file_modal = open;
            }
        }
    }

    // ========================================================================
    // Figs. 12–18 — RNN Tool
    // ========================================================================

    fn ui_rnn_tool(&mut self, ctx: &egui::Context) {
        let is_running = self.rnn.trainer.as_ref().map(|t| t.running).unwrap_or(false);
        let has_trainer = self.rnn.trainer.is_some();

        // Ribbon Bar matching Figs. 14 & 16
        egui::TopBottomPanel::top("rnn_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(self.theme.ribbon_fill())
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ribbon_group(ui, "File", |ui| {
                        if ribbon_btn(ui, "📄", "Iris Sample\n(Fig. 12)", false) {
                            self.rnn = RnnToolState::with_fig12_iris_sample();
                        }
                        if ribbon_btn(ui, "📋", "Paste\nData", false) {
                            self.rnn.show_paste_modal = true;
                        }
                        if ribbon_btn(ui, "📂", "Open\n.xlsNN", false) {
                            self.rnn.file_modal_mode = FileModalMode::OpenData;
                            self.rnn.file_path =
                                "/home/nando/.gemini/antigravity/scratch/rnn_data.csv".into();
                            self.rnn.show_file_modal = true;
                        }
                    });
                    // Fig. 16: Training group
                    ribbon_group(ui, "Training", |ui| {
                        if ribbon_btn(ui, "✅", "Apply\nData", false) {
                            self.rnn.apply_data();
                        }
                        ui.add_enabled_ui(!is_running, |ui| {
                            if ribbon_btn(ui, "▶", "Learn", is_running) {
                                self.rnn.start_learn();
                            }
                        });
                    });
                    // Fig. 14: Neural Network group
                    ribbon_group(ui, "Neural Network", |ui| {
                        ui.add_enabled_ui(has_trainer && !is_running, |ui| {
                            if ribbon_btn(ui, "🧮", "Calculate\nRNN", false) {
                                self.rnn.calculate_rnn();
                            }
                        });
                        ui.add_enabled_ui(has_trainer, |ui| {
                            if ribbon_btn(
                                ui,
                                "🕸",
                                "View RNN\nPicture",
                                self.rnn.show_rnn_picture,
                            ) {
                                self.rnn.show_rnn_picture = true;
                            }
                            if ribbon_btn(
                                ui,
                                "📃",
                                "View RNN\nXML File",
                                self.rnn.show_xml_window,
                            ) {
                                if let Some(tr) = &self.rnn.trainer {
                                    self.rnn.xml_cache = tr.best_model.to_xml();
                                }
                                self.rnn.show_xml_window = true;
                            }
                            if ribbon_btn(ui, "💾", "Export", false) {
                                if let Some(tr) = &self.rnn.trainer {
                                    self.rnn.xml_cache = tr.best_model.to_xml();
                                }
                                self.rnn.file_modal_mode = FileModalMode::ExportXml;
                                self.rnn.file_path =
                                    "/home/nando/.gemini/antigravity/scratch/rnn_model.xml".into();
                                self.rnn.show_file_modal = true;
                            }
                        });
                    });

                    if let Some(msg) = &self.rnn.status_msg {
                        ui.colored_label(egui::Color32::from_rgb(21, 128, 61), msg);
                    }
                    if let Some(err) = &self.rnn.status_err {
                        ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                    }
                });

                // Fig. 17: Live Progress Bar + Abort button while training!
                let rnn_prog = self.rnn.trainer.as_ref().and_then(|tr| {
                    tr.running
                        .then_some((tr.iteration, tr.config.max_iterations, tr.best_val_mse))
                });
                if let Some((iter, max_it, best_mse)) = rnn_prog {
                    let frac = (iter as f32 / max_it.max(1) as f32).clamp(0.0, 1.0);
                    let mut abort_clicked = false;
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                        .show(ui, |ui| {
                            ui.label(format!(
                                "Preparing and training... (Iter {iter}/{max_it}, Val MSE = {best_mse:.4e})"
                            ));
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_width((ui.available_width() - 80.0).max(160.0))
                                        .fill(egui::Color32::from_rgb(192, 132, 252))
                                        .show_percentage(),
                                );
                                if ui.button("Abort").clicked() {
                                    abort_clicked = true;
                                }
                            });
                        });
                    if abort_clicked {
                        self.rnn.abort_learn();
                    }
                }
            });

        // Right Properties Panel (Fig. 13)
        egui::SidePanel::right("rnn_properties_panel")
            .resizable(true)
            .default_width(275.0)
            .min_width(220.0)
            .max_width(380.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                ui.strong("Properties");
                ui.separator();
                let desc_h = 88.0;

                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - desc_h - 14.0).max(160.0))
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new("HyperParameters")
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(ui, "Momentum", &mut self.rnn.momentum_str, &mut self.rnn.selected_prop);
                                prop_row_text(ui, "Supervising Parameter", &mut self.rnn.supervising_str, &mut self.rnn.selected_prop);
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.rnn.selected_prop == "Validation method", "Validation method").clicked() {
                                        self.rnn.selected_prop = "Validation method";
                                    }
                                    egui::ComboBox::from_id_salt("rnn_val_method")
                                        .width(90.0)
                                        .selected_text(self.rnn.val_method.label())
                                        .show_ui(ui, |ui| {
                                            for &m in RnnValidationMethod::ALL {
                                                ui.selectable_value(&mut self.rnn.val_method, m, m.label());
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    if ui.selectable_label(self.rnn.selected_prop == "Regularized", "Regularized").clicked() {
                                        self.rnn.selected_prop = "Regularized";
                                    }
                                    let reg_lbl = if self.rnn.regularized { "True" } else { "False" };
                                    ui.checkbox(&mut self.rnn.regularized, reg_lbl);
                                });
                                prop_row_text(ui, "Regularization parameter", &mut self.rnn.reg_param_str, &mut self.rnn.selected_prop);
                                prop_row_text(ui, "Batch Probability", &mut self.rnn.batch_prob_str, &mut self.rnn.selected_prop);
                            });

                        egui::CollapsingHeader::new("Data")
                            .default_open(true)
                            .show(ui, |ui| {
                                if prop_row_text(ui, "Validation Data Partition", &mut self.rnn.val_pct_str, &mut self.rnn.selected_prop) {
                                    if let Ok(v) = self.rnn.val_pct_str.trim().parse::<f64>() {
                                        self.rnn.train_pct_str = format!("{:.0}", (100.0 - v).clamp(50.0, 95.0));
                                    }
                                }
                                if prop_row_text(ui, "Training Data Partition", &mut self.rnn.train_pct_str, &mut self.rnn.selected_prop) {
                                    if let Ok(t) = self.rnn.train_pct_str.trim().parse::<f64>() {
                                        self.rnn.val_pct_str = format!("{:.0}", (100.0 - t).clamp(5.0, 50.0));
                                    }
                                }
                            });

                        egui::CollapsingHeader::new("Stopping Condition")
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(ui, "Stopping MSE", &mut self.rnn.stop_mse_str, &mut self.rnn.selected_prop);
                                prop_row_text(ui, "Max Iteration Rounds", &mut self.rnn.max_iter_str, &mut self.rnn.selected_prop);
                            });
                    });

                ui.separator();
                let (pname, pdesc) = self.rnn.prop_description();
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.set_min_height(desc_h - 14.0);
                        ui.strong(pname);
                        ui.label(egui::RichText::new(pdesc).size(10.5));
                    });
            });

        // Central Workspace: Fig. 12 Table + Fig. 18 3 Result Charts
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(6.0)),
            )
            .show(ctx, |ui| {
                let total_h = ui.available_height();
                let table_h = (total_h * 0.48).max(170.0);
                let charts_h = (total_h - table_h - 24.0).max(200.0);

                ui.horizontal(|ui| {
                    ui.strong("RNN Data Table (Row 1: RULE/X/Y/YNN • Row 4+: Numeric Data)");
                    if ui.small_button("+ Row").clicked() {
                        let nc = self.rnn.grid[0].len();
                        self.rnn.grid.push(vec![String::new(); nc]);
                    }
                });
                draw_nn_spreadsheet(ui, "rnn_grid", &mut self.rnn.grid, &mut self.rnn.active_cell, table_h);

                ui.add_space(6.0);
                ui.separator();

                if let Some(tr) = &self.rnn.trainer {
                    let (tr_pts, va_pts) = tr.parity_points();
                    ui.columns(3, |cols| {
                        draw_parity_chart(
                            &mut cols[0],
                            "Parity Chart",
                            &tr_pts,
                            &va_pts,
                            0.0,
                            1.0,
                            &mut self.rnn.show_val_parity,
                            &mut self.rnn.show_train_parity,
                            charts_h,
                        );
                        draw_mse_chart(
                            &mut cols[1],
                            "Performance MSE",
                            &tr.train_mse_history,
                            &tr.val_mse_history,
                            tr.best_iter,
                            &mut self.rnn.show_val_mse,
                            &mut self.rnn.show_train_mse,
                            charts_h,
                        );
                        draw_compare_values_chart(
                            &mut cols[2],
                            "Compare Values",
                            &tr_pts,
                            &va_pts,
                            &mut self.rnn.show_nn_compare,
                            &mut self.rnn.show_orig_compare,
                            charts_h,
                        );
                    });
                }
            });

        // Fig. 15: View RNN Picture Window
        if self.rnn.show_rnn_picture {
            let mut open = self.rnn.show_rnn_picture;
            egui::Window::new("Random Neural Network RNN")
                .open(&mut open)
                .resizable(true)
                .default_size([500.0, 440.0])
                .show(ctx, |ui| {
                    if let Some(tr) = &self.rnn.trainer {
                        ui.label(
                            egui::RichText::new("Red stem = Input neuron • Black stem = Output neuron • Green = W+ (excitation) • Blue = W- (inhibition)")
                                .size(10.5)
                                .color(egui::Color32::from_rgb(80, 90, 105)),
                        );
                        draw_rnn_picture_canvas(ui, &tr.best_model);
                    }
                });
            self.rnn.show_rnn_picture = open;
        }

        if self.rnn.show_xml_window {
            let mut open = self.rnn.show_xml_window;
            egui::Window::new("Random Neural Network XML file")
                .open(&mut open)
                .resizable(true)
                .default_size([540.0, 380.0])
                .show(ctx, |ui| {
                    if ui.button("Copy XML").clicked() {
                        ui.output_mut(|o| o.copied_text = self.rnn.xml_cache.clone());
                        self.rnn.status_msg = Some("Copied RNN XML to clipboard.".into());
                    }
                    ui.separator();
                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.rnn.xml_cache)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                    });
                });
            self.rnn.show_xml_window = open;
        }

        if self.rnn.show_paste_modal {
            let mut open = self.rnn.show_paste_modal;
            let mut do_p = false;
            egui::Window::new("Paste RNN Data").open(&mut open).default_width(440.0).show(ctx, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.rnn.paste_buf)
                        .desired_rows(8)
                        .desired_width(420.0),
                );
                if ui.button("Apply Paste").clicked() {
                    do_p = true;
                }
            });
            if do_p {
                let txt = self.rnn.paste_buf.clone();
                apply_tsv_to_grid(&mut self.rnn.grid, &txt, 0);
                self.rnn.show_paste_modal = false;
            } else {
                self.rnn.show_paste_modal = open;
            }
        }

        if self.rnn.show_file_modal {
            let mut open = self.rnn.show_file_modal;
            let mut do_act = false;
            egui::Window::new("RNN File Operation").open(&mut open).default_width(420.0).show(ctx, |ui| {
                ui.label("File path:");
                ui.text_edit_singleline(&mut self.rnn.file_path);
                if ui.button("Confirm").clicked() {
                    do_act = true;
                }
            });
            if do_act {
                let p = self.rnn.file_path.clone();
                match self.rnn.file_modal_mode {
                    FileModalMode::OpenData => match std::fs::read_to_string(&p) {
                        Ok(txt) => {
                            apply_tsv_to_grid(&mut self.rnn.grid, &txt, 0);
                            self.rnn.apply_data();
                        }
                        Err(e) => self.rnn.status_err = Some(format!("Read error: {e}")),
                    },
                    FileModalMode::SaveData => match std::fs::write(&p, grid_to_csv(&self.rnn.grid)) {
                        Ok(_) => self.rnn.status_msg = Some(format!("Saved {p}")),
                        Err(e) => self.rnn.status_err = Some(format!("Save error: {e}")),
                    },
                    FileModalMode::ExportXml => match std::fs::write(&p, &self.rnn.xml_cache) {
                        Ok(_) => self.rnn.status_msg = Some(format!("Exported RNN XML to {p}")),
                        Err(e) => self.rnn.status_err = Some(format!("Export error: {e}")),
                    },
                }
                self.rnn.show_file_modal = false;
            } else {
                self.rnn.show_file_modal = open;
            }
        }
    }

    // ========================================================================
    // PSO Tool (§95.6 — Figs. 20–24)
    // ========================================================================

    fn ui_pso_tool(&mut self, ctx: &egui::Context) {
        let is_running = self.pso.optimizer.as_ref().map(|o| o.running).unwrap_or(false);

        // Top Ribbon (Fig. 20: Examples ▾ | Set X | Set M | Optimize | Stop)
        egui::TopBottomPanel::top("pso_ribbon")
            .frame(
                egui::Frame::none()
                    .fill(self.theme.ribbon_fill())
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ribbon_group(ui, "PSO", |ui| {
                        let ex_btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new("🖵\nExamples ▾")
                                    .size(10.5)
                                    .color(egui::Color32::from_rgb(25, 35, 50)),
                            )
                            .min_size(egui::vec2(64.0, 44.0))
                            .fill(egui::Color32::from_rgb(248, 250, 253)),
                        );
                        let ex_popup_id = ui.make_persistent_id("pso_examples_popup");
                        if ex_btn.clicked() {
                            ui.memory_mut(|m| m.toggle_popup(ex_popup_id));
                        }
                        egui::popup_below_widget(
                            ui,
                            ex_popup_id,
                            &ex_btn,
                            egui::PopupCloseBehavior::CloseOnClick,
                            |ui| {
                                ui.set_min_width(185.0);
                                for &ex in PsoExample::ALL {
                                    let selected = self.pso.current_example == Some(ex);
                                    if ui.selectable_label(selected, ex.title()).clicked() {
                                        self.pso.load_example(ex);
                                    }
                                }
                            },
                        );

                        if ribbon_btn(ui, "⊕", "Set X", false) {
                            self.pso.set_active_as_x();
                        }
                        if ribbon_btn(ui, "⊕", "Set M", false) {
                            self.pso.set_active_as_m();
                        }
                        ui.add_enabled_ui(!is_running, |ui| {
                            if ribbon_btn(ui, "▶", "Optimize", is_running) {
                                self.pso.start_optimize();
                            }
                        });
                        ui.add_enabled_ui(is_running, |ui| {
                            if ribbon_btn(ui, "⏹", "Stop", false) {
                                if let Some(o) = &mut self.pso.optimizer {
                                    o.running = false;
                                }
                            }
                        });
                    });

                    ribbon_group(ui, "Sheet & Tags", |ui| {
                        if ribbon_btn(ui, "⌫", "Clear\nTag", false) {
                            self.pso.comments.remove(&self.pso.active_cell);
                        }
                        if ribbon_btn(ui, "💬", "Comments", self.pso.show_comments) {
                            self.pso.show_comments = !self.pso.show_comments;
                        }
                        if ribbon_btn(ui, "📂", "Open...", false) {
                            self.pso.file_modal_save = false;
                            self.pso.show_file_modal = true;
                        }
                        if ribbon_btn(ui, "💾", "Save", false) {
                            self.pso.file_modal_save = true;
                            self.pso.show_file_modal = true;
                        }
                    });

                    ui.vertical(|ui| {
                        let cref = cell_ref(self.pso.active_cell.0, self.pso.active_cell.1);
                        let tag_lbl = self
                            .pso
                            .comments
                            .get(&self.pso.active_cell)
                            .map(|s| s.as_str())
                            .unwrap_or("none");
                        ui.label(
                            egui::RichText::new(format!("Active cell: {cref}   |   Comment: {tag_lbl}"))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(55, 65, 81)),
                        );
                        if let Some(msg) = &self.pso.status_msg {
                            ui.colored_label(egui::Color32::from_rgb(21, 128, 61), msg);
                        }
                        if let Some(err) = &self.pso.status_err {
                            ui.colored_label(egui::Color32::from_rgb(185, 28, 28), err);
                        }
                    });
                });
            });

        // Right Dock: PSO Properties (Fig. 22)
        egui::SidePanel::right("pso_properties")
            .default_width(275.0)
            .min_width(245.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 246, 248))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                ui.strong("PSO Properties");
                ui.separator();

                egui::ScrollArea::vertical()
                    .id_salt("pso_props_scroll")
                    .max_height((ui.available_height() - 125.0).max(160.0))
                    .show(ui, |ui| {
                        egui::CollapsingHeader::new(egui::RichText::new("Parameters").strong())
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(
                                    ui,
                                    "Particles Per Variable",
                                    &mut self.pso.particles_per_var_str,
                                    &mut self.pso.selected_prop,
                                );
                                prop_row_text(
                                    ui,
                                    "Extra Particles",
                                    &mut self.pso.extra_particles_str,
                                    &mut self.pso.selected_prop,
                                );
                                prop_row_text(
                                    ui,
                                    "Max Rounds",
                                    &mut self.pso.max_rounds_str,
                                    &mut self.pso.selected_prop,
                                );

                                ui.horizontal(|ui| {
                                    if ui
                                        .add_sized(
                                            [142.0, 20.0],
                                            egui::SelectableLabel::new(
                                                self.pso.selected_prop == "Boundary Condition",
                                                egui::RichText::new("Boundary Condition").size(11.0),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.pso.selected_prop = "Boundary Condition";
                                    }
                                    egui::ComboBox::from_id_salt("pso_boundary_combo")
                                        .width(92.0)
                                        .selected_text(self.pso.boundary_cond.label())
                                        .show_ui(ui, |ui| {
                                            for &bc in PsoBoundaryCondition::ALL {
                                                if ui
                                                    .selectable_value(
                                                        &mut self.pso.boundary_cond,
                                                        bc,
                                                        bc.label(),
                                                    )
                                                    .clicked()
                                                {
                                                    self.pso.selected_prop = "Boundary Condition";
                                                }
                                            }
                                        });
                                });

                                ui.horizontal(|ui| {
                                    if ui
                                        .add_sized(
                                            [142.0, 20.0],
                                            egui::SelectableLabel::new(
                                                self.pso.selected_prop == "Use Diversity",
                                                egui::RichText::new("Use Diversity").size(11.0),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.pso.selected_prop = "Use Diversity";
                                    }
                                    egui::ComboBox::from_id_salt("pso_diversity_combo")
                                        .width(92.0)
                                        .selected_text(if self.pso.use_diversity {
                                            "True"
                                        } else {
                                            "False"
                                        })
                                        .show_ui(ui, |ui| {
                                            if ui
                                                .selectable_value(
                                                    &mut self.pso.use_diversity,
                                                    false,
                                                    "False",
                                                )
                                                .clicked()
                                            {
                                                self.pso.selected_prop = "Use Diversity";
                                            }
                                            if ui
                                                .selectable_value(
                                                    &mut self.pso.use_diversity,
                                                    true,
                                                    "True",
                                                )
                                                .clicked()
                                            {
                                                self.pso.selected_prop = "Use Diversity";
                                            }
                                        });
                                });

                                prop_row_text(ui, "W", &mut self.pso.w_str, &mut self.pso.selected_prop);
                                prop_row_text(ui, "WMin", &mut self.pso.w_min_str, &mut self.pso.selected_prop);
                                prop_row_text(ui, "WDamp", &mut self.pso.w_damp_str, &mut self.pso.selected_prop);
                                prop_row_text(ui, "C1", &mut self.pso.c1_str, &mut self.pso.selected_prop);
                                prop_row_text(ui, "C2", &mut self.pso.c2_str, &mut self.pso.selected_prop);
                                prop_row_text(ui, "C3", &mut self.pso.c3_str, &mut self.pso.selected_prop);
                            });

                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(egui::RichText::new("Decision Variable").strong())
                            .default_open(true)
                            .show(ui, |ui| {
                                prop_row_text(
                                    ui,
                                    "Lower Bound",
                                    &mut self.pso.lower_bound_str,
                                    &mut self.pso.selected_prop,
                                );
                                prop_row_text(
                                    ui,
                                    "Upper Bound",
                                    &mut self.pso.upper_bound_str,
                                    &mut self.pso.selected_prop,
                                );
                            });

                        if let Some(opt) = &self.pso.optimizer {
                            ui.add_space(6.0);
                            ui.separator();
                            ui.strong("Best Solution");
                            ui.label(
                                egui::RichText::new(format!(
                                    "Round: {} / {}   (Swarm: {})",
                                    opt.iteration,
                                    opt.config.max_rounds,
                                    opt.particles.len()
                                ))
                                .size(11.0),
                            );
                            ui.label(
                                egui::RichText::new(format!("f(x) = {:.9}", opt.gbest_val))
                                    .strong()
                                    .size(11.5)
                                    .color(egui::Color32::from_rgb(29, 78, 216)),
                            );
                            for (v, &val) in opt.vars.iter().zip(opt.gbest_pos.iter()) {
                                ui.label(
                                    egui::RichText::new(format!("{} = {:.9}", v.cell_ref, val))
                                        .monospace()
                                        .size(11.0),
                                );
                            }
                        }
                    });

                // Bottom parameter description box (exact Fig. 22)
                ui.add_space(6.0);
                let (p_title, p_desc) = self.pso.prop_description();
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(248, 249, 252))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(62.0);
                        ui.label(egui::RichText::new(p_title).strong().size(11.5));
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(p_desc)
                                .size(10.5)
                                .color(egui::Color32::from_rgb(55, 65, 81)),
                        );
                    });
            });

        // Central Panel: Formula Bar + Example Card + PSO Worksheet (Figs. 21 & 24) + Performance Chart (Fig. 23)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                // 1. Excel-style Formula Bar (Fig. 21 top)
                let (ar, ac) = self.pso.active_cell;
                let active_ref = cell_ref(ar, ac);
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 247, 250))
                    .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                    .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [46.0, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(&active_ref)
                                        .strong()
                                        .monospace()
                                        .size(11.5),
                                ),
                            );
                            ui.separator();
                            ui.label(
                                egui::RichText::new("fx")
                                    .italics()
                                    .strong()
                                    .color(egui::Color32::from_rgb(75, 85, 99)),
                            );
                            if ar < self.pso.grid.len() && ac < self.pso.grid[ar].len() {
                                let w = (ui.available_width() - 8.0).max(160.0);
                                ui.add_sized(
                                    [w, 20.0],
                                    egui::TextEdit::singleline(&mut self.pso.grid[ar][ac])
                                        .font(egui::TextStyle::Monospace),
                                );
                            }
                        });
                    });

                ui.add_space(6.0);

                // 2. Benchmark Example Info Card with 3D surface preview (Figs. 21 & 24)
                if let Some(ex) = self.pso.current_example {
                    egui::Frame::none()
                        .fill(egui::Color32::WHITE)
                        .stroke(egui::Stroke::new(1.2, egui::Color32::from_rgb(55, 65, 81)))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (surf_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(175.0, 118.0),
                                    egui::Sense::hover(),
                                );
                                draw_art_pso_surface(ui.painter(), surf_rect, ex);
                                ui.add_space(10.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(ex.title())
                                            .strong()
                                            .size(14.5)
                                            .color(egui::Color32::from_rgb(20, 30, 48)),
                                    );
                                    ui.label(
                                        egui::RichText::new(ex.formula_display())
                                            .monospace()
                                            .size(10.5)
                                            .color(egui::Color32::from_rgb(30, 41, 59)),
                                    );
                                    ui.add_space(3.0);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Search Domain:").strong().size(11.0));
                                        ui.label(
                                            egui::RichText::new(ex.domain_text())
                                                .strong()
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(15, 23, 42)),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Min =").strong().size(11.0));
                                        ui.vertical(|ui| {
                                            for line in ex.known_min_lines() {
                                                ui.label(
                                                    egui::RichText::new(*line)
                                                        .monospace()
                                                        .size(10.0)
                                                        .color(egui::Color32::from_rgb(30, 58, 138)),
                                                );
                                            }
                                        });
                                    });
                                });
                            });
                        });
                    ui.add_space(6.0);
                }

                // 3. PSO Spreadsheet with Red Comment Corner Triangles + Yellow Callouts (Figs. 21 & 24)
                let rem_h = ui.available_height();
                let sheet_h = (rem_h * 0.45).clamp(155.0, 250.0);
                let chart_h = (rem_h - sheet_h - 14.0).max(165.0);

                // Precompute evaluated formula values before borrowing self.pso.grid mutably
                let mut eval_cache: HashMap<(usize, usize), f64> = HashMap::new();
                for r in 0..self.pso.grid.len() {
                    for c in 0..self.pso.grid[r].len() {
                        if let Some(val) = self.pso.evaluate_formula_cell(r, c) {
                            eval_cache.insert((r, c), val);
                        }
                    }
                }

                let mut comment_anchors: Vec<(egui::Pos2, String)> = Vec::new();
                egui::ScrollArea::both()
                    .id_salt("pso_grid_scroll")
                    .max_height(sheet_h)
                    .show(ui, |ui| {
                        let nc = self.pso.grid[0].len();
                        ui.horizontal(|ui| {
                            row_num_cell(ui, 0, 28.0);
                            for c in 0..nc {
                                let w = match c {
                                    0 => 64.0,
                                    1 | 2 => 118.0,
                                    3 => 132.0,
                                    _ => 92.0,
                                };
                                header_cell(ui, &col_letter(c), w);
                            }
                        });
                        for (r, row) in self.pso.grid.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                row_num_cell(ui, r + 1, 28.0);
                                for (c, cell) in row.iter_mut().enumerate() {
                                    let w = match c {
                                        0 => 64.0,
                                        1 | 2 => 118.0,
                                        3 => 132.0,
                                        _ => 92.0,
                                    };
                                    let tag = self.pso.comments.get(&(r, c)).cloned();
                                    let is_formula = cell.trim().starts_with('=');
                                    let bg = if r == 0 && c <= 3 {
                                        egui::Color32::from_rgb(219, 232, 246)
                                    } else if is_formula {
                                        egui::Color32::from_rgb(255, 253, 242)
                                    } else {
                                        egui::Color32::WHITE
                                    };
                                    let is_act = self.pso.active_cell == (r, c);
                                    let cell_rect = ui
                                        .scope(|ui| {
                                            widget_cell(ui, w, bg, is_act, |ui| {
                                                if is_formula {
                                                    let display_str = if let Some(&v) = eval_cache.get(&(r, c)) {
                                                        format!("{v:.9}")
                                                            .trim_end_matches('0')
                                                            .trim_end_matches('.')
                                                            .to_string()
                                                    } else {
                                                        "#ERR".into()
                                                    };
                                                    let resp = ui.add_sized(
                                                        [w - 8.0, 18.0],
                                                        egui::SelectableLabel::new(
                                                            false,
                                                            egui::RichText::new(display_str)
                                                                .monospace()
                                                                .size(11.0)
                                                                .color(egui::Color32::from_rgb(15, 23, 42)),
                                                        ),
                                                    );
                                                    if resp.clicked() {
                                                        self.pso.active_cell = (r, c);
                                                    }
                                                    let tip = if let Some(t) = &tag {
                                                        format!("Comment: {t}\nFormula: {cell}\n(Edit formula in fx bar above)")
                                                    } else {
                                                        format!("Formula: {cell}\n(Edit formula in fx bar above)")
                                                    };
                                                    resp.on_hover_text(tip);
                                                } else {
                                                    let resp = ui.text_edit_singleline(cell);
                                                    if resp.gained_focus() || resp.clicked() {
                                                        self.pso.active_cell = (r, c);
                                                    }
                                                    if let Some(t) = &tag {
                                                        resp.on_hover_text(format!("Comment: {t}"));
                                                    }
                                                }
                                            });
                                        })
                                        .response
                                        .rect;
                                    // Red comment indicator triangle at top-right corner (exact Excel / Figs. 21 & 24)
                                    if let Some(t) = tag {
                                        let tr = cell_rect.right_top();
                                        ui.painter().add(egui::Shape::convex_polygon(
                                            vec![
                                                tr,
                                                egui::pos2(tr.x - 6.0, tr.y),
                                                egui::pos2(tr.x, tr.y + 6.0),
                                            ],
                                            egui::Color32::from_rgb(220, 38, 38),
                                            egui::Stroke::NONE,
                                        ));
                                        if self.pso.show_comments {
                                            comment_anchors.push((tr, t));
                                        }
                                    }
                                }
                            });
                        }

                        // Paint floating yellow Excel-style comment callouts below tagged cells (Figs. 21 & 24)
                        if self.pso.show_comments && !comment_anchors.is_empty() {
                            let p = ui.painter();
                            let mut prev_right_x = f32::NEG_INFINITY;
                            for (anchor, text) in comment_anchors {
                                let box_w = (text.len() as f32 * 6.5 + 14.0).max(58.0);
                                let box_h = 20.0;
                                let bx = (anchor.x + 12.0).max(prev_right_x + 8.0);
                                let by = anchor.y + 28.0;
                                prev_right_x = bx + box_w;
                                let rect = egui::Rect::from_min_size(
                                    egui::pos2(bx, by),
                                    egui::vec2(box_w, box_h),
                                );
                                p.line_segment(
                                    [anchor, egui::pos2(bx, by + 2.0)],
                                    egui::Stroke::new(0.9, egui::Color32::from_rgb(70, 75, 85)),
                                );
                                p.rect_filled(rect, 0.0, egui::Color32::from_rgb(255, 255, 225));
                                p.rect_stroke(
                                    rect,
                                    0.0,
                                    egui::Stroke::new(1.0, egui::Color32::from_rgb(90, 95, 105)),
                                );
                                p.text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    text,
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_rgb(20, 25, 35),
                                );
                            }
                        }
                    });

                ui.add_space(6.0);
                ui.separator();

                // 4. Performance Review Chart with LINEAR Y-axis (Fig. 23)
                if let Some(opt) = &self.pso.optimizer {
                    draw_pso_performance_chart(ui, &opt.history, chart_h);
                } else {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(248, 250, 252))
                        .stroke(egui::Stroke::new(1.0, EXCEL_GRID_BORDER))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.set_min_height((chart_h - 10.0).max(110.0));
                            ui.strong("Performance");
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new(
                                    "Click 'Optimize' in the PSO ribbon to start optimization and view the live convergence curve (Fig. 23).",
                                )
                                .color(egui::Color32::from_rgb(75, 85, 99)),
                            );
                        });
                }
            });

        // Open / Save CSV modal for PSO sheet
        if self.pso.show_file_modal {
            let mut open = self.pso.show_file_modal;
            let title = if self.pso.file_modal_save {
                "Save PSO Sheet (.csv)"
            } else {
                "Open PSO Sheet (.csv)"
            };
            egui::Window::new(title)
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("File path:");
                    ui.add_sized(
                        [340.0, 22.0],
                        egui::TextEdit::singleline(&mut self.pso.file_path_str),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .button(if self.pso.file_modal_save { "Save" } else { "Open" })
                            .clicked()
                        {
                            let p = self.pso.file_path_str.clone();
                            if self.pso.file_modal_save {
                                self.pso.save_csv(&p);
                            } else {
                                self.pso.open_csv(&p);
                            }
                            self.pso.show_file_modal = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.pso.show_file_modal = false;
                        }
                    });
                });
            if !self.pso.show_file_modal {
                // closed via button
            } else {
                self.pso.show_file_modal = open;
            }
        }
    }
}

/// Linear-axis PSO Performance convergence chart matching Fig. 23 (handles negative objective values!).
fn draw_pso_performance_chart(ui: &mut egui::Ui, history: &[f64], height: f32) {
    ui.horizontal(|ui| {
        ui.strong("Performance");
        if let Some(&last) = history.last() {
            ui.label(
                egui::RichText::new(format!(
                    "  |  Rounds: {}   |   Latest Best: {:.9}",
                    history.len(),
                    last
                ))
                .size(11.0)
                .color(egui::Color32::from_rgb(55, 65, 81)),
            );
        }
    });

    let avail_w = ui.available_width().max(260.0);
    let h = (height - 22.0).max(130.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(avail_w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);

    p.rect_filled(rect, 2.0, egui::Color32::WHITE);
    p.rect_stroke(rect, 2.0, egui::Stroke::new(1.0, EXCEL_GRID_BORDER));

    let plot = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 68.0, rect.top() + 12.0),
        egui::pos2(rect.right() - 16.0, rect.bottom() - 26.0),
    );
    if plot.width() < 40.0 || plot.height() < 40.0 {
        return;
    }

    p.rect_filled(plot, 0.0, egui::Color32::WHITE);
    p.rect_stroke(plot, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(130, 138, 150)));

    if history.is_empty() {
        p.text(
            plot.center(),
            egui::Align2::CENTER_CENTER,
            "Waiting for PSO iterations...",
            egui::FontId::proportional(11.5),
            egui::Color32::GRAY,
        );
        return;
    }

    let mut y_min = f64::INFINITY;
    let mut y_max = f64::NEG_INFINITY;
    for &v in history {
        if v.is_finite() {
            y_min = y_min.min(v);
            y_max = y_max.max(v);
        }
    }
    if !y_min.is_finite() || !y_max.is_finite() {
        y_min = -1.0;
        y_max = 1.0;
    }
    let span = (y_max - y_min).abs();
    let pad = if span < 1e-12 {
        (y_min.abs() * 0.05).max(1e-4)
    } else {
        span * 0.12
    };
    let y0 = y_min - pad;
    let y1 = y_max + pad;

    let max_x = history.len().max(10) as f64;
    let x_step = nice_step(max_x / 5.0);
    let y_step = nice_step((y1 - y0) / 5.0);

    let to_x = |x: f64| plot.left() + ((x / max_x) as f32) * plot.width();
    let to_y = |y: f64| plot.bottom() - (((y - y0) / (y1 - y0)) as f32) * plot.height();

    // Horizontal grid + linear Y ticks
    let mut yv = (y0 / y_step).ceil() * y_step;
    let mut guard = 0;
    while yv <= y1 + y_step * 0.01 && guard < 30 {
        let py = to_y(yv);
        if py >= plot.top() - 1.0 && py <= plot.bottom() + 1.0 {
            p.line_segment(
                [egui::pos2(plot.left(), py), egui::pos2(plot.right(), py)],
                egui::Stroke::new(0.7, egui::Color32::from_rgb(228, 232, 238)),
            );
            p.line_segment(
                [egui::pos2(plot.left() - 4.0, py), egui::pos2(plot.left(), py)],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 118, 130)),
            );
            p.text(
                egui::pos2(plot.left() - 7.0, py),
                egui::Align2::RIGHT_CENTER,
                format_tick(yv),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(60, 68, 80),
            );
        }
        yv += y_step;
        guard += 1;
    }

    // Vertical grid + X round ticks
    let mut xv = 0.0;
    let mut xguard = 0;
    while xv <= max_x + x_step * 0.01 && xguard < 30 {
        let px = to_x(xv);
        if px >= plot.left() - 1.0 && px <= plot.right() + 1.0 {
            p.line_segment(
                [egui::pos2(px, plot.top()), egui::pos2(px, plot.bottom())],
                egui::Stroke::new(0.7, egui::Color32::from_rgb(235, 238, 242)),
            );
            p.line_segment(
                [egui::pos2(px, plot.bottom()), egui::pos2(px, plot.bottom() + 4.0)],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 118, 130)),
            );
            p.text(
                egui::pos2(px, plot.bottom() + 7.0),
                egui::Align2::CENTER_TOP,
                format!("{}", xv.round() as i64),
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(60, 68, 80),
            );
        }
        xv += x_step;
        xguard += 1;
    }

    // Blue convergence curve (exact Fig. 23 color)
    let curve_col = egui::Color32::from_rgb(79, 129, 189);
    let pts: Vec<egui::Pos2> = history
        .iter()
        .enumerate()
        .filter(|(_, v)| v.is_finite())
        .map(|(i, &v)| egui::pos2(to_x(i as f64), to_y(v)))
        .collect();
    if pts.len() >= 2 {
        p.add(egui::Shape::line(pts, egui::Stroke::new(2.0, curve_col)));
    } else if let Some(&pt) = pts.first() {
        p.circle_filled(pt, 3.0, curve_col);
    }

    // Interactive vertical cursor line + Fig. 23 floating rounded callout box (`— <round> : <value>`)
    let cursor_idx = if let Some(hp) = resp.hover_pos() {
        if plot.contains(hp) {
            let frac = ((hp.x - plot.left()) / plot.width()).clamp(0.0, 1.0) as f64;
            ((frac * max_x).round() as usize).min(history.len() - 1)
        } else {
            history.len() - 1
        }
    } else {
        // Match Fig. 23 look: highlight latest round (or 15% round if already flat? latest is most informative)
        history.len() - 1
    };
    let cursor_x = to_x(cursor_idx as f64);
    let cursor_val = history[cursor_idx];
    p.line_segment(
        [egui::pos2(cursor_x, plot.top()), egui::pos2(cursor_x, plot.bottom())],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(165, 172, 182)),
    );
    p.circle_filled(
        egui::pos2(cursor_x, to_y(cursor_val)),
        3.2,
        curve_col,
    );

    let callout_text = format!("{} : {:.12}", cursor_idx + 1, cursor_val);
    let box_w = (callout_text.len() as f32 * 6.8 + 42.0).clamp(150.0, plot.width() - 16.0);
    let box_h = 30.0;
    let box_rect = egui::Rect::from_min_size(
        egui::pos2(plot.right() - box_w - 10.0, plot.top() + 12.0),
        egui::vec2(box_w, box_h),
    );
    p.rect_filled(box_rect, 4.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 240));
    p.rect_stroke(box_rect, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 118, 130)));
    p.line_segment(
        [
            egui::pos2(box_rect.left() + 10.0, box_rect.center().y),
            egui::pos2(box_rect.left() + 26.0, box_rect.center().y),
        ],
        egui::Stroke::new(2.0, curve_col),
    );
    p.text(
        egui::pos2(box_rect.left() + 32.0, box_rect.center().y),
        egui::Align2::LEFT_CENTER,
        callout_text,
        egui::FontId::proportional(11.0),
        egui::Color32::from_rgb(30, 38, 52),
    );
}

/// Miniature 3D surface preview matching the Cross-in-tray illustration in Figs. 21 & 24.
fn draw_art_pso_surface(p: &egui::Painter, rect: egui::Rect, ex: PsoExample) {
    p.rect_filled(rect, 3.0, egui::Color32::from_rgb(250, 251, 253));
    p.rect_stroke(rect, 3.0, egui::Stroke::new(0.8, egui::Color32::from_rgb(215, 222, 232)));

    let n = 22usize;
    let (lo, hi) = ex.domain();
    let mut zs = vec![vec![0.0f64; n + 1]; n + 1];
    let mut zmin = f64::INFINITY;
    let mut zmax = f64::NEG_INFINITY;
    for i in 0..=n {
        let x = lo + (hi - lo) * (i as f64) / (n as f64);
        for j in 0..=n {
            let y = lo + (hi - lo) * (j as f64) / (n as f64);
            let z = ex.eval_preview_z(x, y);
            zs[i][j] = z;
            zmin = zmin.min(z);
            zmax = zmax.max(z);
        }
    }
    let dz = (zmax - zmin).max(1e-9);
    let project = |i: usize, j: usize| -> egui::Pos2 {
        let u = (i as f32) / (n as f32) - 0.5;
        let v = (j as f32) / (n as f32) - 0.5;
        let zn = ((zs[i][j] - zmin) / dz) as f32;
        let sx = rect.center().x + (u - v) * rect.width() * 0.44;
        let sy = rect.center().y + (u + v) * rect.height() * 0.25 + (0.48 - zn) * rect.height() * 0.34;
        egui::pos2(sx, sy)
    };

    for i in 0..n {
        for j in 0..n {
            let p0 = project(i, j);
            let p1 = project(i + 1, j);
            let p2 = project(i + 1, j + 1);
            let p3 = project(i, j + 1);
            let t = (((zs[i][j] + zs[i + 1][j + 1]) * 0.5 - zmin) / dz).clamp(0.0, 1.0) as f32;
            let fill = if t > 0.72 {
                egui::Color32::from_rgb(235, 65, 25)
            } else if t > 0.48 {
                egui::Color32::from_rgb(245, 195, 20)
            } else if t > 0.25 {
                egui::Color32::from_rgb(45, 195, 70)
            } else {
                egui::Color32::from_rgb(40, 110, 220)
            };
            p.add(egui::Shape::convex_polygon(
                vec![p0, p1, p2, p3],
                fill,
                egui::Stroke::new(0.4, egui::Color32::from_rgb(40, 48, 60)),
            ));
        }
    }
}

// ============================================================================
// Reusable UI Chrome & Vector Graphics Helpers (Figs. 1, 3, 7, 9, 15, 18)
// ============================================================================

fn ribbon_group(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none()
        .fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 165))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(198, 206, 220)))
        .rounding(3.0)
        .inner_margin(egui::Margin::symmetric(5.0, 3.0))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    add_contents(ui);
                });
                ui.vertical_centered(|ui| {
                    ui.label(
                        egui::RichText::new(title)
                            .size(9.5)
                            .color(egui::Color32::from_rgb(100, 112, 130)),
                    );
                });
            });
        });
}

fn ribbon_btn(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> bool {
    let btn = egui::Button::new(
        egui::RichText::new(format!("{icon}\n{label}"))
            .size(10.5)
            .color(egui::Color32::from_rgb(25, 35, 50)),
    )
    .min_size(egui::vec2(54.0, 44.0))
    .fill(if active {
        egui::Color32::from_rgb(191, 219, 254)
    } else {
        egui::Color32::from_rgb(248, 250, 253)
    });
    ui.add(btn).clicked()
}

fn prop_row_text(
    ui: &mut egui::Ui,
    name: &'static str,
    val: &mut String,
    selected: &mut &'static str,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui
            .add_sized(
                [142.0, 20.0],
                egui::SelectableLabel::new(*selected == name, egui::RichText::new(name).size(11.0)),
            )
            .clicked()
        {
            *selected = name;
        }
        let r = ui.add_sized([90.0, 20.0], egui::TextEdit::singleline(val));
        if r.gained_focus() {
            *selected = name;
        }
        if r.changed() {
            changed = true;
        }
    });
    changed
}

fn prop_row_readonly(
    ui: &mut egui::Ui,
    name: &'static str,
    val: &str,
    selected: &mut &'static str,
) {
    ui.horizontal(|ui| {
        if ui
            .add_sized(
                [142.0, 20.0],
                egui::SelectableLabel::new(*selected == name, egui::RichText::new(name).size(11.0)),
            )
            .clicked()
        {
            *selected = name;
        }
        ui.label(egui::RichText::new(val).strong().color(egui::Color32::from_rgb(75, 85, 99)));
    });
}

fn draw_nn_spreadsheet(
    ui: &mut egui::Ui,
    id: &str,
    grid: &mut [Vec<String>],
    active_cell: &mut (usize, usize),
    max_h: f32,
) {
    let n_cols = grid.first().map(|r| r.len()).unwrap_or(0);
    let rule_col = grid[0]
        .iter()
        .position(|s| s.trim().eq_ignore_ascii_case("RULE"));

    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(max_h)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                row_num_cell(ui, 0, 28.0);
                for c in 0..n_cols {
                    header_cell(ui, &col_letter(c), 78.0);
                }
            });
            for (r_idx, row) in grid.iter_mut().enumerate() {
                let is_excluded = r_idx >= 3
                    && rule_col
                        .and_then(|rc| row.get(rc))
                        .map(|s| s.trim().eq_ignore_ascii_case("E"))
                        .unwrap_or(false);
                ui.horizontal(|ui| {
                    row_num_cell(ui, r_idx + 1, 28.0);
                    for (c_idx, cell) in row.iter_mut().enumerate() {
                        let bg = if r_idx == 0 {
                            match cell.trim().to_ascii_uppercase().as_str() {
                                "X" => egui::Color32::from_rgb(219, 234, 254),
                                "Y" => egui::Color32::from_rgb(220, 252, 231),
                                "YNN" => egui::Color32::from_rgb(254, 249, 195),
                                "RULE" => egui::Color32::from_rgb(254, 226, 226),
                                _ => EXCEL_READONLY_BG,
                            }
                        } else if is_excluded {
                            egui::Color32::from_rgb(255, 241, 242)
                        } else if r_idx % 2 == 1 {
                            egui::Color32::WHITE
                        } else {
                            EXCEL_ALT_ROW_BG
                        };
                        let is_act = *active_cell == (r_idx, c_idx);
                        widget_cell(ui, 78.0, bg, is_act, |ui| {
                            let r = ui.text_edit_singleline(cell);
                            if r.gained_focus() {
                                *active_cell = (r_idx, c_idx);
                            }
                        });
                    }
                });
            }
        });
}

fn draw_home_tool_card(
    ui: &mut egui::Ui,
    title: &str,
    paint_art: fn(&egui::Painter, egui::Rect),
) -> bool {
    let card_size = egui::vec2(265.0, 205.0);
    let (rect, resp) = ui.allocate_exact_size(card_size, egui::Sense::click());
    let painter = ui.painter_at(rect);
    let hovered = resp.hovered();
    painter.rect_filled(
        rect,
        6.0,
        if hovered {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(250, 251, 253)
        },
    );
    painter.rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(
            if hovered { 1.8 } else { 1.0 },
            if hovered {
                egui::Color32::from_rgb(59, 130, 246)
            } else {
                egui::Color32::from_rgb(210, 218, 230)
            },
        ),
    );
    painter.text(
        egui::pos2(rect.center().x, rect.top() + 16.0),
        egui::Align2::CENTER_CENTER,
        title,
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(25, 35, 50),
    );
    let art_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(14.0, 30.0),
        rect.max - egui::vec2(14.0, 10.0),
    );
    paint_art(&painter, art_rect);
    resp.clicked()
}

/// Fig. 1 Top-Left Artwork: 3D rainbow surface mesh
fn draw_art_charts_surface(painter: &egui::Painter, rect: egui::Rect) {
    let n = 14usize;
    let project = |u: f32, v: f32, z: f32| -> egui::Pos2 {
        let sx = rect.center().x + (u - v) * rect.width() * 0.34;
        let sy = rect.center().y + (u + v) * rect.height() * 0.20 - z * rect.height() * 0.24;
        egui::pos2(sx, sy)
    };
    let surf = |u: f32, v: f32| -> f32 {
        0.55 * (u * 2.5).sin() * (v * 2.3).cos() + 0.35 * (u * u - v * v)
    };
    for i in 0..n {
        let u0 = -1.0 + 2.0 * (i as f32 / n as f32);
        let u1 = -1.0 + 2.0 * ((i + 1) as f32 / n as f32);
        for j in 0..n {
            let v0 = -1.0 + 2.0 * (j as f32 / n as f32);
            let v1 = -1.0 + 2.0 * ((j + 1) as f32 / n as f32);
            let z00 = surf(u0, v0);
            let z10 = surf(u1, v0);
            let z11 = surf(u1, v1);
            let z01 = surf(u0, v1);
            let zm = 0.25 * (z00 + z10 + z11 + z01);
            let t = ((zm + 0.8) / 1.6).clamp(0.0, 1.0);
            let col = rainbow_color(t);
            let pts = vec![
                project(u0, v0, z00),
                project(u1, v0, z10),
                project(u1, v1, z11),
                project(u0, v1, z01),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                col,
                egui::Stroke::new(0.45, egui::Color32::from_rgb(35, 35, 45)),
            ));
        }
    }
}

/// Fig. 1 Top-Right Artwork: RNN circular graph with petal loops
fn draw_art_rnn_ring(painter: &egui::Painter, rect: egui::Rect) {
    let c = rect.center();
    let r = rect.height().min(rect.width()) * 0.36;
    let n = 8usize;
    let nodes: Vec<egui::Pos2> = (0..n)
        .map(|i| {
            let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
            egui::pos2(c.x + r * ang.cos(), c.y + r * ang.sin())
        })
        .collect();
    for i in 0..n {
        for j in (i + 1)..n {
            let col = if (i + j) % 2 == 0 {
                egui::Color32::from_rgb(70, 120, 200)
            } else {
                egui::Color32::from_rgb(70, 150, 95)
            };
            painter.line_segment([nodes[i], nodes[j]], egui::Stroke::new(0.85, col));
        }
    }
    for (i, &p) in nodes.iter().enumerate() {
        let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
        let loop_p = egui::pos2(p.x + 15.0 * ang.cos(), p.y + 15.0 * ang.sin());
        painter.circle_stroke(loop_p, 6.0, egui::Stroke::new(0.9, egui::Color32::from_rgb(95, 110, 210)));
        painter.circle_filled(p, 5.5, egui::Color32::from_rgb(235, 130, 125));
        painter.circle_stroke(p, 5.5, egui::Stroke::new(0.9, egui::Color32::from_rgb(120, 45, 45)));
    }
}

/// Fig. 1 Bottom-Left Artwork: FFNN 3-layer graph
fn draw_art_ffnn_layers(painter: &egui::Painter, rect: egui::Rect) {
    let counts = [3usize, 7, 2];
    let xs = [
        rect.left() + rect.width() * 0.22,
        rect.center().x,
        rect.right() - rect.width() * 0.22,
    ];
    let cols = [
        egui::Color32::from_rgb(235, 130, 125),
        egui::Color32::from_rgb(234, 195, 65),
        egui::Color32::from_rgb(165, 220, 235),
    ];
    let mut layer_pts: Vec<Vec<egui::Pos2>> = Vec::new();
    for (l, &cnt) in counts.iter().enumerate() {
        let mut pts = Vec::new();
        for i in 0..cnt {
            let fy = (i as f32 + 0.5) / cnt as f32;
            pts.push(egui::pos2(xs[l], rect.top() + 8.0 + fy * (rect.height() - 16.0)));
        }
        layer_pts.push(pts);
    }
    for l in 0..2 {
        for &p0 in &layer_pts[l] {
            for &p1 in &layer_pts[l + 1] {
                painter.line_segment([p0, p1], egui::Stroke::new(0.9, egui::Color32::from_rgb(35, 125, 45)));
            }
        }
    }
    for (l, pts) in layer_pts.iter().enumerate() {
        for &p in pts {
            painter.circle_filled(p, 5.0, cols[l]);
            painter.circle_stroke(p, 5.0, egui::Stroke::new(0.9, egui::Color32::from_rgb(55, 65, 75)));
        }
    }
}

/// Fig. 1 Bottom-Right Artwork: PSO converging swarm cloud
fn draw_art_pso_swarm(painter: &egui::Painter, rect: egui::Rect) {
    let mut rng = SimpleRng::new(95_2026);
    let c = rect.center();
    let rx = rect.width() * 0.42;
    let ry = rect.height() * 0.42;
    for _ in 0..550 {
        let r = rng.next_f64().powf(2.1) as f32;
        let a = (rng.next_f64() * std::f64::consts::TAU) as f32;
        let p = egui::pos2(c.x + r * rx * a.cos(), c.y + r * ry * a.sin());
        painter.circle_filled(p, 1.1, egui::Color32::from_rgb(75, 58, 35));
    }
}

fn rainbow_color(t: f32) -> egui::Color32 {
    let h = (1.0 - t.clamp(0.0, 1.0)) * 240.0;
    let c = 0.85_f32;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else {
        (0.0, x, c)
    };
    egui::Color32::from_rgb((r * 235.0) as u8, (g * 235.0) as u8, (b * 235.0) as u8)
}

/// Interactive chart renderer for the Charts Tool (Fig. 3)
fn draw_data_chart(
    ui: &mut egui::Ui,
    spec: &mut ChartSpec,
    xs: &[f64],
    series_list: &[(String, Vec<f64>)],
    height: f32,
) {
    let palette = [
        egui::Color32::from_rgb(95, 130, 75),
        egui::Color32::from_rgb(37, 99, 235),
        egui::Color32::from_rgb(185, 28, 28),
        egui::Color32::from_rgb(180, 110, 25),
        egui::Color32::from_rgb(124, 58, 237),
        egui::Color32::from_rgb(13, 148, 136),
    ];
    while spec.series_visible.len() < series_list.len() {
        spec.series_visible.push(true);
    }

    ui.vertical_centered(|ui| {
        ui.label(egui::RichText::new(&spec.title).strong().size(13.5));
    });

    let plot_w = (ui.available_width() - 8.0).max(200.0);
    let plot_h = (height - 46.0).max(160.0);
    let (outer_rect, resp) =
        ui.allocate_exact_size(egui::vec2(plot_w, plot_h), egui::Sense::hover());
    let painter = ui.painter_at(outer_rect);
    let plot_rect = egui::Rect::from_min_max(
        outer_rect.min + egui::vec2(64.0, 10.0),
        outer_rect.max - egui::vec2(18.0, 34.0),
    );
    painter.rect_filled(plot_rect, 0.0, egui::Color32::WHITE);

    let (x_min, x_max) = if xs.is_empty() {
        (0.0, 10.0)
    } else {
        let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        (lo, (hi).max(lo + 1.0))
    };

    let (y_min, y_max) = if spec.auto_y {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for (si, (_, ys)) in series_list.iter().enumerate() {
            if !spec.series_visible.get(si).copied().unwrap_or(true) {
                continue;
            }
            for &y in ys {
                if y.is_finite() {
                    lo = lo.min(y);
                    hi = hi.max(y);
                }
            }
        }
        if lo.is_finite() && hi.is_finite() {
            let pad = ((hi - lo) * 0.08).max(1.0);
            (lo - pad, hi + pad)
        } else {
            (-10.0, 10.0)
        }
    } else {
        let lo = parse_flex_f64(&spec.y_min_str).unwrap_or(-10.0);
        let hi = parse_flex_f64(&spec.y_max_str).unwrap_or(10.0).max(lo + 1e-6);
        (lo, hi)
    };

    let x_px = |x: f64| -> f32 {
        plot_rect.left() + (((x - x_min) / (x_max - x_min)) as f32) * plot_rect.width()
    };
    let y_px = |y: f64| -> f32 {
        plot_rect.bottom() - (((y - y_min) / (y_max - y_min)) as f32) * plot_rect.height()
    };

    // Gridlines & ticks
    let x_step = nice_step((x_max - x_min) / 8.0);
    let mut gx = (x_min / x_step).ceil() * x_step;
    while gx <= x_max + 1e-9 {
        let x = x_px(gx);
        if spec.show_grid {
            draw_dashed_line(
                &painter,
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
                2.0,
                3.0,
                egui::Stroke::new(0.7, egui::Color32::from_rgb(218, 223, 232)),
            );
        }
        painter.text(
            egui::pos2(x, plot_rect.bottom() + 11.0),
            egui::Align2::CENTER_CENTER,
            format_tick(gx),
            egui::FontId::proportional(10.0),
            egui::Color32::BLACK,
        );
        gx += x_step;
    }
    let y_step = nice_step((y_max - y_min) / 8.0);
    let mut gy = (y_min / y_step).ceil() * y_step;
    while gy <= y_max + 1e-9 {
        let y = y_px(gy);
        if spec.show_grid {
            draw_dashed_line(
                &painter,
                egui::pos2(plot_rect.left(), y),
                egui::pos2(plot_rect.right(), y),
                2.0,
                3.0,
                egui::Stroke::new(0.7, egui::Color32::from_rgb(218, 223, 232)),
            );
        }
        painter.text(
            egui::pos2(plot_rect.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            format_tick(gy),
            egui::FontId::proportional(10.0),
            egui::Color32::BLACK,
        );
        gy += y_step;
    }

    let clipped = painter.with_clip_rect(plot_rect);
    for (si, (name, raw_ys)) in series_list.iter().enumerate() {
        if !spec.series_visible.get(si).copied().unwrap_or(true) {
            continue;
        }
        let col = palette[(si + spec.color_offset) % palette.len()];
        let clean: Vec<(f64, f64)> = xs
            .iter()
            .copied()
            .zip(raw_ys.iter().copied())
            .filter(|(_, y)| y.is_finite())
            .collect();
        if clean.is_empty() {
            continue;
        }
        let cxs: Vec<f64> = clean.iter().map(|p| p.0).collect();
        let cys: Vec<f64> = if spec.smooth_window > 1 {
            moving_average(&clean.iter().map(|p| p.1).collect::<Vec<_>>(), spec.smooth_window)
        } else {
            clean.iter().map(|p| p.1).collect()
        };

        let pts: Vec<egui::Pos2> = cxs
            .iter()
            .zip(cys.iter())
            .map(|(&x, &y)| egui::pos2(x_px(x), y_px(y)))
            .collect();

        match spec.kind {
            ChartKind::LineMarkers => {
                for w in pts.windows(2) {
                    clipped.line_segment([w[0], w[1]], egui::Stroke::new(spec.line_width, col));
                }
                for &p in &pts {
                    clipped.circle_filled(p, 3.6, col);
                }
            }
            ChartKind::Line => {
                for w in pts.windows(2) {
                    clipped.line_segment([w[0], w[1]], egui::Stroke::new(spec.line_width, col));
                }
            }
            ChartKind::Scatter => {
                for &p in &pts {
                    clipped.circle_filled(p, 3.8, col);
                }
            }
            ChartKind::Bar => {
                let bw = (plot_rect.width() / (pts.len().max(1) as f32 * 2.0)).clamp(4.0, 26.0);
                let y0 = y_px(0.0_f64.clamp(y_min, y_max));
                for &p in &pts {
                    let r = egui::Rect::from_two_pos(
                        egui::pos2(p.x - 0.5 * bw, y0),
                        egui::pos2(p.x + 0.5 * bw, p.y),
                    );
                    clipped.rect_filled(r, 0.0, col);
                }
            }
        }

        if spec.show_trendline {
            let an = compute_column_analytics(name, &cxs, &cys);
            if let (Some(b), Some(a)) = (an.slope, an.intercept) {
                let p0 = egui::pos2(x_px(x_min), y_px(a + b * x_min));
                let p1 = egui::pos2(x_px(x_max), y_px(a + b * x_max));
                draw_dashed_line(&clipped, p0, p1, 5.0, 3.0, egui::Stroke::new(1.3, egui::Color32::from_rgb(185, 28, 28)));
            }
        }
    }

    painter.rect_stroke(plot_rect, 0.0, egui::Stroke::new(1.2, egui::Color32::BLACK));
    painter.text(
        egui::pos2(plot_rect.center().x, plot_rect.bottom() + 24.0),
        egui::Align2::CENTER_CENTER,
        &spec.x_axis_title,
        egui::FontId::proportional(11.0),
        egui::Color32::BLACK,
    );
    draw_rotated_y_title(
        &painter,
        outer_rect.left() + 12.0,
        plot_rect.center().y,
        &spec.y_axis_title,
        egui::FontId::proportional(11.0),
        egui::Color32::BLACK,
    );

    if let Some(pos) = resp.hover_pos() {
        if plot_rect.contains(pos) {
            let fx = (pos.x - plot_rect.left()) / plot_rect.width();
            let fy = (plot_rect.bottom() - pos.y) / plot_rect.height();
            let hx = x_min + fx as f64 * (x_max - x_min);
            let hy = y_min + fy as f64 * (y_max - y_min);
            resp.on_hover_text(format!("{}: {hx:.3}\n{}: {hy:.3}", spec.x_axis_title, spec.y_axis_title));
        }
    }

    // Legend checkboxes matching Fig. 3
    ui.horizontal_wrapped(|ui| {
        for (si, (name, _)) in series_list.iter().enumerate() {
            let col = palette[(si + spec.color_offset) % palette.len()];
            if let Some(vis) = spec.series_visible.get_mut(si) {
                ui.checkbox(vis, egui::RichText::new(format!("● {name}")).color(col).size(11.0));
            }
        }
    });
}

/// Parity Chart (Figs. 9 & 18)
#[allow(clippy::too_many_arguments)]
fn draw_parity_chart(
    ui: &mut egui::Ui,
    title: &str,
    train_pts: &[(f64, f64)],
    val_pts: &[(f64, f64)],
    lo: f64,
    hi: f64,
    show_val: &mut bool,
    show_train: &mut bool,
    height: f32,
) {
    ui.strong(title);
    let w = (ui.available_width() - 4.0).max(150.0);
    let h = (height - 46.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let pr = egui::Rect::from_min_max(rect.min + egui::vec2(38.0, 6.0), rect.max - egui::vec2(8.0, 22.0));
    p.rect_filled(pr, 0.0, egui::Color32::WHITE);

    let map_x = |v: f64| pr.left() + (((v - lo) / (hi - lo)) as f32) * pr.width();
    let map_y = |v: f64| pr.bottom() - (((v - lo) / (hi - lo)) as f32) * pr.height();

    for i in 0..=4 {
        let v = lo + (hi - lo) * (i as f64 / 4.0);
        let x = map_x(v);
        let y = map_y(v);
        p.line_segment([egui::pos2(x, pr.top()), egui::pos2(x, pr.bottom())], egui::Stroke::new(0.5, egui::Color32::from_rgb(230, 234, 240)));
        p.line_segment([egui::pos2(pr.left(), y), egui::pos2(pr.right(), y)], egui::Stroke::new(0.5, egui::Color32::from_rgb(230, 234, 240)));
        p.text(egui::pos2(x, pr.bottom() + 9.0), egui::Align2::CENTER_CENTER, format!("{v:.1}"), egui::FontId::proportional(9.0), egui::Color32::BLACK);
        p.text(egui::pos2(pr.left() - 4.0, y), egui::Align2::RIGHT_CENTER, format!("{v:.1}"), egui::FontId::proportional(9.0), egui::Color32::BLACK);
    }

    let c = p.with_clip_rect(pr);
    // Gray reference diagonal y = x
    c.line_segment([egui::pos2(map_x(lo), map_y(lo)), egui::pos2(map_x(hi), map_y(hi))], egui::Stroke::new(1.0, egui::Color32::from_rgb(155, 160, 170)));
    if *show_train {
        for &(x, y) in train_pts {
            c.circle_filled(egui::pos2(map_x(x), map_y(y)), 2.5, egui::Color32::from_rgb(155, 65, 65));
        }
    }
    if *show_val {
        for &(x, y) in val_pts {
            c.circle_filled(egui::pos2(map_x(x), map_y(y)), 2.8, egui::Color32::from_rgb(45, 100, 200));
        }
    }
    p.rect_stroke(pr, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 120, 135)));

    ui.horizontal(|ui| {
        ui.checkbox(show_val, egui::RichText::new("Validation").size(10.0).color(egui::Color32::from_rgb(45, 100, 200)));
        ui.checkbox(show_train, egui::RichText::new("Training").size(10.0).color(egui::Color32::from_rgb(155, 65, 65)));
    });
}

/// Performance MSE Log-Scale Chart (Figs. 9, 18, 23)
#[allow(clippy::too_many_arguments)]
fn draw_mse_chart(
    ui: &mut egui::Ui,
    title: &str,
    train_hist: &[f64],
    val_hist: &[f64],
    best_iter: usize,
    show_val: &mut bool,
    show_train: &mut bool,
    height: f32,
) {
    ui.strong(title);
    let w = (ui.available_width() - 4.0).max(150.0);
    let h = (height - 46.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let pr = egui::Rect::from_min_max(rect.min + egui::vec2(46.0, 6.0), rect.max - egui::vec2(8.0, 22.0));
    p.rect_filled(pr, 0.0, egui::Color32::WHITE);

    let max_x = train_hist.len().max(val_hist.len()).max(2) as f64;
    let all_vals = train_hist.iter().chain(val_hist.iter()).copied().filter(|v| v.is_finite() && *v > 0.0);
    let min_log = all_vals.clone().map(|v| v.log10()).fold(f64::INFINITY, f64::min);
    let max_log = all_vals.map(|v| v.log10()).fold(f64::NEG_INFINITY, f64::max);
    let (lo_l, hi_l) = if min_log.is_finite() && max_log.is_finite() {
        ((min_log.floor() - 0.5).max(-10.0), (max_log.ceil() + 0.5).max(min_log + 1.0))
    } else {
        (-5.0, 0.0)
    };

    let map_x = |it: f64| pr.left() + ((it / max_x) as f32) * pr.width();
    let map_y = |val: f64| {
        let lv = val.max(1e-12).log10().clamp(lo_l, hi_l);
        pr.bottom() - (((lv - lo_l) / (hi_l - lo_l)) as f32) * pr.height()
    };

    let mut d = lo_l.ceil();
    while d <= hi_l {
        let y = pr.bottom() - (((d - lo_l) / (hi_l - lo_l)) as f32) * pr.height();
        p.line_segment([egui::pos2(pr.left(), y), egui::pos2(pr.right(), y)], egui::Stroke::new(0.5, egui::Color32::from_rgb(230, 234, 240)));
        p.text(egui::pos2(pr.left() - 4.0, y), egui::Align2::RIGHT_CENTER, format!("1e{d:.0}"), egui::FontId::proportional(8.5), egui::Color32::BLACK);
        d += 1.0;
    }

    let c = p.with_clip_rect(pr);
    if best_iter > 0 {
        let bx = map_x(best_iter as f64);
        draw_dashed_line(&c, egui::pos2(bx, pr.top()), egui::pos2(bx, pr.bottom()), 3.0, 3.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 115, 125)));
    }
    if *show_train && train_hist.len() >= 2 {
        for i in 0..(train_hist.len() - 1) {
            c.line_segment(
                [egui::pos2(map_x(i as f64 + 1.0), map_y(train_hist[i])), egui::pos2(map_x(i as f64 + 2.0), map_y(train_hist[i + 1]))],
                egui::Stroke::new(1.3, egui::Color32::from_rgb(155, 65, 65)),
            );
        }
    }
    if *show_val && val_hist.len() >= 2 {
        for i in 0..(val_hist.len() - 1) {
            c.line_segment(
                [egui::pos2(map_x(i as f64 + 1.0), map_y(val_hist[i])), egui::pos2(map_x(i as f64 + 2.0), map_y(val_hist[i + 1]))],
                egui::Stroke::new(1.3, egui::Color32::from_rgb(45, 100, 200)),
            );
        }
    }
    p.rect_stroke(pr, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 120, 135)));
    p.text(egui::pos2(pr.center().x, pr.bottom() + 9.0), egui::Align2::CENTER_CENTER, format!("Iter (max {max_x:.0})"), egui::FontId::proportional(9.0), egui::Color32::BLACK);

    if !val_hist.is_empty() {
        ui.horizontal(|ui| {
            ui.checkbox(show_val, egui::RichText::new("Validation").size(10.0).color(egui::Color32::from_rgb(45, 100, 200)));
            ui.checkbox(show_train, egui::RichText::new("Training").size(10.0).color(egui::Color32::from_rgb(155, 65, 65)));
        });
    }
}

/// Compare Values Chart (Figs. 9 & 18 — ascending sorted original vs NN prediction)
fn draw_compare_values_chart(
    ui: &mut egui::Ui,
    title: &str,
    train_pts: &[(f64, f64)],
    val_pts: &[(f64, f64)],
    show_nn: &mut bool,
    show_orig: &mut bool,
    height: f32,
) {
    ui.strong(title);
    let w = (ui.available_width() - 4.0).max(150.0);
    let h = (height - 46.0).max(120.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let pr = egui::Rect::from_min_max(rect.min + egui::vec2(38.0, 6.0), rect.max - egui::vec2(8.0, 22.0));
    p.rect_filled(pr, 0.0, egui::Color32::WHITE);

    let mut all: Vec<(f64, f64)> = train_pts.iter().chain(val_pts.iter()).copied().collect();
    all.sort_by(|a, b| a.0.total_cmp(&b.0));

    let n = all.len().max(2) as f64;
    let lo = all.iter().map(|&(a, b)| a.min(b)).fold(f64::INFINITY, f64::min);
    let hi = all.iter().map(|&(a, b)| a.max(b)).fold(f64::NEG_INFINITY, f64::max);
    let (ymin, ymax) = if lo.is_finite() && hi.is_finite() {
        let pad = ((hi - lo) * 0.08).max(0.1);
        (lo - pad, hi + pad)
    } else {
        (-1.0, 1.0)
    };

    let map_x = |i: usize| pr.left() + ((i as f64 / (n - 1.0)) as f32) * pr.width();
    let map_y = |v: f64| pr.bottom() - (((v - ymin) / (ymax - ymin)) as f32) * pr.height();

    let c = p.with_clip_rect(pr);
    if *show_nn && all.len() >= 2 {
        for i in 0..(all.len() - 1) {
            c.line_segment(
                [egui::pos2(map_x(i), map_y(all[i].1)), egui::pos2(map_x(i + 1), map_y(all[i + 1].1))],
                egui::Stroke::new(1.2, egui::Color32::from_rgb(45, 100, 200)),
            );
        }
    }
    if *show_orig && all.len() >= 2 {
        for i in 0..(all.len() - 1) {
            c.line_segment(
                [egui::pos2(map_x(i), map_y(all[i].0)), egui::pos2(map_x(i + 1), map_y(all[i + 1].0))],
                egui::Stroke::new(1.4, egui::Color32::from_rgb(165, 50, 50)),
            );
        }
    }
    p.rect_stroke(pr, 0.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(110, 120, 135)));
    p.text(egui::pos2(pr.center().x, pr.bottom() + 9.0), egui::Align2::CENTER_CENTER, format!("Samples (1..{})", all.len()), egui::FontId::proportional(9.0), egui::Color32::BLACK);

    ui.horizontal(|ui| {
        ui.checkbox(show_nn, egui::RichText::new("NN Value").size(10.0).color(egui::Color32::from_rgb(45, 100, 200)));
        ui.checkbox(show_orig, egui::RichText::new("Original").size(10.0).color(egui::Color32::from_rgb(165, 50, 50)));
    });
}

/// Fig. 7: Visualization of the FFNN model structure
fn draw_ffnn_picture_canvas(ui: &mut egui::Ui, model: &soryntel_thermo::data_processing::FfnnModel) {
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(360.0), avail.y.max(280.0)), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(245, 246, 250));

    let mut dims = vec![model.n_inputs];
    for l in &model.layers {
        dims.push(l.n_out);
    }
    let nl = dims.len();
    let mut node_pos: Vec<Vec<egui::Pos2>> = Vec::with_capacity(nl);
    for (l, &cnt) in dims.iter().enumerate() {
        let fx = (l as f32 + 0.5) / nl as f32;
        let x = rect.left() + 30.0 + fx * (rect.width() - 60.0);
        let mut pts = Vec::with_capacity(cnt);
        for i in 0..cnt {
            let fy = (i as f32 + 0.5) / cnt.max(1) as f32;
            pts.push(egui::pos2(x, rect.top() + 20.0 + fy * (rect.height() - 40.0)));
        }
        node_pos.push(pts);
    }

    // Green weighted connections (matching Fig. 7)
    for (l, layer) in model.layers.iter().enumerate() {
        for (o, &p_to) in node_pos[l + 1].iter().enumerate() {
            for (i, &p_from) in node_pos[l].iter().enumerate() {
                let w_abs = layer.weights[o][i].abs().clamp(0.05, 1.5) as f32;
                let width = (0.5 + w_abs * 1.1).clamp(0.5, 2.2);
                p.line_segment([p_from, p_to], egui::Stroke::new(width, egui::Color32::from_rgb(20, 115, 25)));
            }
        }
    }

    // Nodes: coral-red inputs, gold hidden, cyan outputs
    for (l, pts) in node_pos.iter().enumerate() {
        let (fill, rad) = if l == 0 {
            (egui::Color32::from_rgb(235, 130, 125), 9.0)
        } else if l + 1 == nl {
            (egui::Color32::from_rgb(175, 225, 240), 10.0)
        } else {
            (egui::Color32::from_rgb(234, 195, 65), if pts.len() > 20 { 4.2 } else { 6.5 })
        };
        for &pt in pts {
            p.circle_filled(pt, rad, fill);
            p.circle_stroke(pt, rad, egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 65, 75)));
        }
    }
}

/// Fig. 15: Visualization of the RNN model structure (red stems = inputs, black stem = output)
fn draw_rnn_picture_canvas(ui: &mut egui::Ui, model: &soryntel_thermo::data_processing::RnnModel) {
    let avail = ui.available_size();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail.x.max(360.0), avail.y.max(320.0)), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(245, 246, 250));

    let c = rect.center();
    let rad = rect.width().min(rect.height()) * 0.34;
    let n = model.n_neurons.max(2);
    let nodes: Vec<egui::Pos2> = (0..n)
        .map(|i| {
            let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
            egui::pos2(c.x + rad * ang.cos(), c.y + rad * ang.sin())
        })
        .collect();

    // Draw W+ (green) and W- (blue) edges scaled by weight strength
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let wp = model.w_plus[i][j] as f32;
            let wm = model.w_minus[i][j] as f32;
            let dir = (nodes[j] - nodes[i]).normalized();
            let perp = egui::vec2(-dir.y, dir.x) * 2.4;
            p.line_segment(
                [nodes[i] + perp, nodes[j] + perp],
                egui::Stroke::new((0.6 + wp * 1.4).clamp(0.6, 3.2), egui::Color32::from_rgb(35, 130, 50)),
            );
            p.line_segment(
                [nodes[i] - perp, nodes[j] - perp],
                egui::Stroke::new((0.6 + wm * 1.4).clamp(0.6, 3.2), egui::Color32::from_rgb(75, 110, 200)),
            );
        }
    }

    // Self-loops + red input stems + black output stem (exact Fig. 15 conventions!)
    let out_start = n - model.n_outputs;
    for (i, &pt) in nodes.iter().enumerate() {
        let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
        let dir = egui::vec2(ang.cos(), ang.sin());
        let perp = egui::vec2(-dir.y, dir.x);
        // Excitation & inhibition petal self-loops
        p.circle_stroke(pt + dir * 18.0 + perp * 11.0, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 150, 70)));
        p.circle_stroke(pt + dir * 18.0 - perp * 11.0, 8.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(75, 100, 205)));
        // Input red stem / Output black stem
        if i < model.n_inputs {
            p.line_segment([pt, pt + dir * 28.0], egui::Stroke::new(1.6, egui::Color32::from_rgb(220, 38, 38)));
        }
        if i >= out_start {
            p.line_segment([pt, pt + (dir * 0.3 + perp * 0.95).normalized() * 34.0], egui::Stroke::new(2.0, egui::Color32::BLACK));
        }
        p.circle_filled(pt, 9.5, egui::Color32::from_rgb(235, 130, 125));
        p.circle_stroke(pt, 9.5, egui::Stroke::new(1.1, egui::Color32::from_rgb(115, 45, 45)));
    }
}

// ============================================================================
// Spreadsheet Utilities
// ============================================================================

fn col_letter(idx: usize) -> String {
    let mut n = idx + 1;
    let mut s = String::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    s
}

fn cell_ref(row: usize, col: usize) -> String {
    format!("{}{}", col_letter(col), row + 1)
}

fn parse_cell_ref(s: &str) -> Option<(usize, usize)> {
    let up = s.trim().to_ascii_uppercase();
    let split = up.find(|c: char| c.is_ascii_digit())?;
    let letters = &up[..split];
    let digits = &up[split..];
    let mut col = 0usize;
    for b in letters.bytes() {
        if !b.is_ascii_uppercase() {
            return None;
        }
        col = col * 26 + (b - b'A' + 1) as usize;
    }
    let row = digits.parse::<usize>().ok()?;
    if col == 0 || row == 0 {
        return None;
    }
    Some((row - 1, col - 1))
}

fn apply_tsv_to_grid(grid: &mut Vec<Vec<String>>, raw: &str, start_row: usize) {
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return;
    }
    let mut max_cols = grid.first().map(|r| r.len()).unwrap_or(8);
    let parsed_rows: Vec<Vec<String>> = lines
        .iter()
        .map(|l| {
            let sep = if l.contains('\t') {
                '\t'
            } else if l.contains(';') {
                ';'
            } else {
                ','
            };
            let parts: Vec<String> = l.split(sep).map(|s| s.trim().to_string()).collect();
            max_cols = max_cols.max(parts.len());
            parts
        })
        .collect();

    let needed_rows = (start_row + parsed_rows.len()).max(grid.len());
    while grid.len() < needed_rows {
        grid.push(vec![String::new(); max_cols]);
    }
    for row in grid.iter_mut() {
        if row.len() < max_cols {
            row.resize(max_cols, String::new());
        }
    }
    for (i, pr) in parsed_rows.into_iter().enumerate() {
        for (c, val) in pr.into_iter().enumerate() {
            grid[start_row + i][c] = val;
        }
    }
}

fn grid_to_csv(grid: &[Vec<String>]) -> String {
    grid.iter()
        .map(|r| r.join(","))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_menu_locks_other_tools_until_closed() {
        let mut st = DataProcessingModuleState::default();
        assert_eq!(st.active_tool, DataTool::Home);
        st.active_tool = DataTool::Charts;
        st.close_active_tool();
        assert_eq!(st.active_tool, DataTool::Home);
    }

    #[test]
    fn charts_tool_analytics_clone_and_hscchart_roundtrip() {
        let mut st = ChartsToolState::with_fig3_sample();
        let (xs, series) = st.extract_xy_series();
        assert_eq!(xs.len(), 9);
        assert_eq!(series.len(), 1);

        let tmp = format!("/tmp/test_chart_{}.HSCChart", std::process::id());
        st.save_file(&tmp);
        assert!(st.status_err.is_none());

        let mut st2 = ChartsToolState::with_fig3_sample();
        st2.reset_blank();
        st2.open_file(&tmp);
        let _ = std::fs::remove_file(&tmp);
        assert!(st2.status_err.is_none());
        let (xs2, _) = st2.extract_xy_series();
        assert_eq!(xs2.len(), 9);
    }

    #[test]
    fn ffnn_ui_apply_learn_calculate_nn_and_xml() {
        let mut st = FfnnToolState::with_fig4_sample();
        assert!(st.parsed.is_some());
        assert!(st.trainer.is_some());
        st.start_learn();
        st.trainer.as_mut().unwrap().step_epochs(40);
        st.stop_learn();
        st.calculate_nn();
        assert!(st.status_err.is_none(), "{:?}", st.status_err);
        // Excluded Row 5 (index 4) must ALSO receive prediction in YNN column (col 10)
        assert!(!st.grid[4][10].trim().is_empty());
        assert!(st.xml_cache.contains("<ANN z:Id=\"1\""));
    }

    #[test]
    fn rnn_ui_apply_learn_abort_calculate_rnn_and_xml() {
        let mut st = RnnToolState::with_fig12_iris_sample();
        assert!(st.parsed.is_some());
        st.start_learn();
        st.trainer.as_mut().unwrap().step_epochs(25);
        st.abort_learn();
        st.calculate_rnn();
        assert!(st.status_err.is_none(), "{:?}", st.status_err);
        assert!(!st.grid[5][8].trim().is_empty());
        assert!(st.xml_cache.contains("<RNN z:Id=\"1\""));
    }

    #[test]
    fn pso_ui_set_x_set_m_and_optimize() {
        let mut st = PsoToolState::default();
        // Fig. 21 initial state: x,y blank -> f(x,y) evaluates to -0.0001
        let f_init = st.evaluate_formula_cell(1, 3).unwrap();
        assert!((f_init - (-0.0001)).abs() < 1e-12, "f_init={f_init}");

        st.start_optimize();
        assert!(st.status_err.is_none(), "{:?}", st.status_err);
        st.optimizer.as_mut().unwrap().step_iterations(180).unwrap();
        st.write_best_to_grid();

        // Fig. 24 result state: Cross-in-tray minimum ~ -2.06261 while D2 keeps its formula
        assert!(st.grid[1][3].starts_with('='));
        let f_opt = st.evaluate_formula_cell(1, 3).unwrap();
        assert!(f_opt < -2.062, "f_opt={f_opt}");

        // Verify Set X / Set M and Examples switching
        st.load_example(PsoExample::SphereSin);
        st.active_cell = (3, 1);
        st.lower_bound_str = "-5".into();
        st.upper_bound_str = "5".into();
        st.set_active_as_x();
        assert_eq!(st.comments.get(&(3, 1)).map(|s| s.as_str()), Some("PSO_X/-5/5"));
        st.active_cell = (3, 2);
        st.set_active_as_m();
        assert_eq!(st.comments.get(&(3, 2)).map(|s| s.as_str()), Some("PSO_M"));
    }
}
