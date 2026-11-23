//! Module 21: HSC Data — Data Processing Engine (Chapter 95).
//!
//! Provides numerical engines for all four tools of the HSC Data module:
//! - **Charts Analytics**: descriptive statistics, linear regression, R², correlation, moving average.
//! - **FFNN (Feedforward Neural Network)**: multi-layer perceptron with backpropagation, momentum,
//!   learning-rate decay, configurable activations (`TanH`, `Sigmoid`, `ReLU`, `Linear`),
//!   `[-1, 1]` normalization, incremental per-epoch training, and Outotec DataContract XML export.
//! - **RNN (Random Neural Network)**: Gelenbe recurrent random neural network with positive
//!   excitation (`W+`) and negative inhibition (`W-`) weights, fixed-point firing probabilities
//!   `q_i in [0, 1]`, Shepard/Random validation splitting, L2 regularization, momentum, and XML export.
//! - **PSO (Particle Swarm Optimization)**: bounded particle swarm optimizer with a recursive-descent
//!   Excel-like formula evaluator (`ABS`, `SIN`, `COS`, `TAN`, `EXP`, `LN`, `LOG`, `SQRT`, `POW`,
//!   `MIN`, `MAX`, and spreadsheet cell references `A1`..`Z99`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ============================================================================
// Shared deterministic PRNG (xorshift64*) — no external rand crate required
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = (self.next_u64() as usize) % (i + 1);
            slice.swap(i, j);
        }
    }
}

/// Parses numbers formatted with either `.` or `,` decimal separators (matching HSC Fig. 4).
pub fn parse_flex_f64(raw: &str) -> Option<f64> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    if let Ok(v) = s.parse::<f64>() {
        return v.is_finite().then_some(v);
    }
    let normalized = s.replace(',', ".");
    normalized.parse::<f64>().ok().filter(|v| v.is_finite())
}

// ============================================================================
// 1. Charts Data Analytics
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct ColumnAnalytics {
    pub name: String,
    pub count: usize,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub slope: Option<f64>,
    pub intercept: Option<f64>,
    pub r_squared: Option<f64>,
}

pub fn compute_column_analytics(name: &str, xs: &[f64], ys: &[f64]) -> ColumnAnalytics {
    let n = ys.len();
    if n == 0 {
        return ColumnAnalytics {
            name: name.to_string(),
            ..Default::default()
        };
    }
    let mean = ys.iter().sum::<f64>() / n as f64;
    let var = if n > 1 {
        ys.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (n - 1) as f64
    } else {
        0.0
    };
    let mut sorted = ys.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    };
    let min = sorted[0];
    let max = sorted[n - 1];

    let (slope, intercept, r_squared) = if xs.len() == n && n >= 2 {
        let xm = xs.iter().sum::<f64>() / n as f64;
        let sxx: f64 = xs.iter().map(|x| (x - xm).powi(2)).sum();
        let syy: f64 = ys.iter().map(|y| (y - mean).powi(2)).sum();
        let sxy: f64 = xs
            .iter()
            .zip(ys.iter())
            .map(|(x, y)| (x - xm) * (y - mean))
            .sum();
        if sxx > 1e-18 {
            let b = sxy / sxx;
            let a = mean - b * xm;
            let r2 = if syy > 1e-18 {
                (sxy * sxy / (sxx * syy)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            (Some(b), Some(a), Some(r2))
        } else {
            (None, None, None)
        }
    } else {
        (None, None, None)
    };

    ColumnAnalytics {
        name: name.to_string(),
        count: n,
        mean,
        std_dev: var.sqrt(),
        min,
        max,
        median,
        slope,
        intercept,
        r_squared,
    }
}

pub fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
    let w = window.max(1);
    let mut out = Vec::with_capacity(values.len());
    for i in 0..values.len() {
        let start = i.saturating_sub(w / 2);
        let end = (start + w).min(values.len());
        let avg = values[start..end].iter().sum::<f64>() / (end - start) as f64;
        out.push(avg);
    }
    out
}

// ============================================================================
// 2. Shared Dataset Parsing for FFNN & RNN (Row 1: RULE/X/Y/YNN, Row 4+: Data)
// ============================================================================

#[derive(Debug, Clone)]
pub struct NnSample {
    pub sheet_row: usize,
    pub inputs: Vec<f64>,
    pub targets: Vec<f64>,
    pub excluded: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedNnDataset {
    pub x_cols: Vec<usize>,
    pub y_cols: Vec<usize>,
    pub ynn_cols: Vec<usize>,
    pub rule_col: Option<usize>,
    pub x_names: Vec<String>,
    pub y_names: Vec<String>,
    pub samples: Vec<NnSample>,
    pub active_count: usize,
    pub excluded_count: usize,
}

/// Parses a spreadsheet grid (`grid[row][col]`, 0-indexed) following Ch. 95 §§95.4–95.5:
/// - Row 0 (Excel Row 1): variable tags `RULE`, `X`, `Y`, `YNN`
/// - Row 1 (Excel Row 2): variable names
/// - Row 2 (Excel Row 3): units / info
/// - Row 3+ (Excel Row 4+): numeric sample data; `'E'`/`'e'` in `RULE` marks excluded rows.
pub fn parse_nn_table(grid: &[Vec<String>]) -> Result<ParsedNnDataset, String> {
    if grid.len() < 4 {
        return Err("Data table must have at least 4 rows (Row 1 tags, Row 4+ data).".into());
    }
    let header = &grid[0];
    let mut x_cols = Vec::new();
    let mut y_cols = Vec::new();
    let mut ynn_cols = Vec::new();
    let mut rule_col = None;

    for (c, cell) in header.iter().enumerate() {
        match cell.trim().to_ascii_uppercase().as_str() {
            "X" => x_cols.push(c),
            "Y" => y_cols.push(c),
            "YNN" => ynn_cols.push(c),
            "RULE" => rule_col = Some(c),
            _ => {}
        }
    }

    if x_cols.is_empty() {
        return Err("No 'X' input columns defined in Row 1.".into());
    }
    if y_cols.is_empty() {
        return Err("No 'Y' output column defined in Row 1.".into());
    }

    let x_names: Vec<String> = x_cols
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            grid.get(1)
                .and_then(|r| r.get(c))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("X{}", i + 1))
        })
        .collect();
    let y_names: Vec<String> = y_cols
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            grid.get(1)
                .and_then(|r| r.get(c))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Y{}", i + 1))
        })
        .collect();

    let mut samples = Vec::new();
    let mut active_count = 0usize;
    let mut excluded_count = 0usize;

    for (r_idx, row) in grid.iter().enumerate().skip(3) {
        let mut inputs = Vec::with_capacity(x_cols.len());
        let mut all_x_ok = true;
        for &c in &x_cols {
            let v = row.get(c).and_then(|s| parse_flex_f64(s));
            if let Some(val) = v {
                inputs.push(val);
            } else {
                all_x_ok = false;
                break;
            }
        }
        if !all_x_ok {
            continue;
        }
        let mut targets = Vec::with_capacity(y_cols.len());
        let mut all_y_ok = true;
        for &c in &y_cols {
            if let Some(val) = row.get(c).and_then(|s| parse_flex_f64(s)) {
                targets.push(val);
            } else {
                all_y_ok = false;
                break;
            }
        }
        if !all_y_ok {
            continue;
        }
        let excluded = rule_col
            .and_then(|rc| row.get(rc))
            .map(|s| s.trim().eq_ignore_ascii_case("E"))
            .unwrap_or(false);
        if excluded {
            excluded_count += 1;
        } else {
            active_count += 1;
        }
        samples.push(NnSample {
            sheet_row: r_idx,
            inputs,
            targets,
            excluded,
        });
    }

    if active_count < 3 {
        return Err(format!(
            "Need at least 3 non-excluded numeric data rows starting at Row 4 (found {active_count})."
        ));
    }

    Ok(ParsedNnDataset {
        x_cols,
        y_cols,
        ynn_cols,
        rule_col,
        x_names,
        y_names,
        samples,
        active_count,
        excluded_count,
    })
}

/// Min-max scaler mapping `[min, max]` into `[out_lo, out_hi]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinMaxScaler {
    pub mins: Vec<f64>,
    pub maxs: Vec<f64>,
    pub out_lo: f64,
    pub out_hi: f64,
}

impl MinMaxScaler {
    pub fn fit(rows: &[Vec<f64>], out_lo: f64, out_hi: f64) -> Self {
        let dim = rows.first().map(|r| r.len()).unwrap_or(0);
        let mut mins = vec![f64::INFINITY; dim];
        let mut maxs = vec![f64::NEG_INFINITY; dim];
        for r in rows {
            for (j, &v) in r.iter().enumerate() {
                if v < mins[j] {
                    mins[j] = v;
                }
                if v > maxs[j] {
                    maxs[j] = v;
                }
            }
        }
        for j in 0..dim {
            if !mins[j].is_finite() || !maxs[j].is_finite() || (maxs[j] - mins[j]).abs() < 1e-12 {
                mins[j] = 0.0;
                maxs[j] = 1.0;
            }
        }
        Self {
            mins,
            maxs,
            out_lo,
            out_hi,
        }
    }

    pub fn transform(&self, v: &[f64]) -> Vec<f64> {
        let span_out = self.out_hi - self.out_lo;
        v.iter()
            .enumerate()
            .map(|(j, &x)| {
                let lo = self.mins.get(j).copied().unwrap_or(0.0);
                let hi = self.maxs.get(j).copied().unwrap_or(1.0);
                let span = (hi - lo).max(1e-12);
                self.out_lo + ((x - lo) / span) * span_out
            })
            .collect()
    }

    pub fn inverse(&self, s: &[f64]) -> Vec<f64> {
        let span_out = (self.out_hi - self.out_lo).max(1e-12);
        s.iter()
            .enumerate()
            .map(|(j, &y)| {
                let lo = self.mins.get(j).copied().unwrap_or(0.0);
                let hi = self.maxs.get(j).copied().unwrap_or(1.0);
                lo + ((y - self.out_lo) / span_out) * (hi - lo)
            })
            .collect()
    }
}

// ============================================================================
// 3. FFNN (Feedforward Neural Network)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationFunc {
    TanH,
    Sigmoid,
    ReLU,
    Linear,
}

impl ActivationFunc {
    pub const ALL: &'static [ActivationFunc] = &[
        ActivationFunc::TanH,
        ActivationFunc::Sigmoid,
        ActivationFunc::ReLU,
        ActivationFunc::Linear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ActivationFunc::TanH => "TanH",
            ActivationFunc::Sigmoid => "Sigmoid",
            ActivationFunc::ReLU => "ReLU",
            ActivationFunc::Linear => "Linear",
        }
    }

    #[inline]
    pub fn eval(self, z: f64) -> f64 {
        match self {
            ActivationFunc::TanH => z.tanh(),
            ActivationFunc::Sigmoid => 1.0 / (1.0 + (-z.clamp(-40.0, 40.0)).exp()),
            ActivationFunc::ReLU => z.max(0.0),
            ActivationFunc::Linear => z,
        }
    }

    #[inline]
    pub fn deriv_from_output(self, a: f64) -> f64 {
        match self {
            ActivationFunc::TanH => 1.0 - a * a,
            ActivationFunc::Sigmoid => a * (1.0 - a),
            ActivationFunc::ReLU => {
                if a > 0.0 {
                    1.0
                } else {
                    0.0
                }
            }
            ActivationFunc::Linear => 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfnnConfig {
    pub learning_rate: f64,
    pub learning_rate_decay: f64,
    pub momentum: f64,
    pub hidden_activation: ActivationFunc,
    pub output_activation: ActivationFunc,
    pub hidden_layers: Vec<usize>,
    pub validation_pct: f64,
    pub training_pct: f64,
    pub batch_size: usize,
    pub stopping_mse: f64,
    pub max_iterations: usize,
}

impl Default for FfnnConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.02,
            learning_rate_decay: 1e-6,
            momentum: 0.1,
            hidden_activation: ActivationFunc::TanH,
            output_activation: ActivationFunc::Linear,
            hidden_layers: vec![12],
            validation_pct: 20.0,
            training_pct: 80.0,
            batch_size: 1,
            stopping_mse: 1e-5,
            max_iterations: 2000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfnnLayer {
    pub n_in: usize,
    pub n_out: usize,
    /// `weights[out][in]` where last element `in == n_in` is bias weight (matches Fig. 11 size `n_in + 1`).
    pub weights: Vec<Vec<f64>>,
    pub delta_weights: Vec<Vec<f64>>,
    pub activation: ActivationFunc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FfnnModel {
    pub n_inputs: usize,
    pub n_outputs: usize,
    pub layers: Vec<FfnnLayer>,
    pub x_scaler: MinMaxScaler,
    pub y_scaler: MinMaxScaler,
}

impl FfnnModel {
    pub fn new(
        n_inputs: usize,
        hidden: &[usize],
        n_outputs: usize,
        hidden_act: ActivationFunc,
        output_act: ActivationFunc,
        x_scaler: MinMaxScaler,
        y_scaler: MinMaxScaler,
        rng: &mut SimpleRng,
    ) -> Self {
        let mut dims = vec![n_inputs];
        for &h in hidden {
            dims.push(h.clamp(1, 128));
        }
        dims.push(n_outputs);

        let mut layers = Vec::with_capacity(dims.len() - 1);
        for l in 0..(dims.len() - 1) {
            let nin = dims[l];
            let nout = dims[l + 1];
            let act = if l + 1 == dims.len() - 1 {
                output_act
            } else {
                hidden_act
            };
            let limit = (6.0 / (nin + nout) as f64).sqrt().min(0.6);
            let mut weights = Vec::with_capacity(nout);
            let mut delta_weights = Vec::with_capacity(nout);
            for _ in 0..nout {
                let mut w = Vec::with_capacity(nin + 1);
                for _ in 0..=nin {
                    w.push(rng.range(-limit, limit));
                }
                weights.push(w);
                delta_weights.push(vec![0.0; nin + 1]);
            }
            layers.push(FfnnLayer {
                n_in: nin,
                n_out: nout,
                weights,
                delta_weights,
                activation: act,
            });
        }
        Self {
            n_inputs,
            n_outputs,
            layers,
            x_scaler,
            y_scaler,
        }
    }

    pub fn forward_scaled(&self, x_scaled: &[f64]) -> Vec<Vec<f64>> {
        let mut acts = Vec::with_capacity(self.layers.len() + 1);
        acts.push(x_scaled.to_vec());
        for layer in &self.layers {
            let prev = acts.last().unwrap();
            let mut cur = Vec::with_capacity(layer.n_out);
            for o in 0..layer.n_out {
                let w = &layer.weights[o];
                let mut z = w[layer.n_in]; // bias
                for i in 0..layer.n_in {
                    z += w[i] * prev[i];
                }
                cur.push(layer.activation.eval(z));
            }
            acts.push(cur);
        }
        acts
    }

    pub fn predict_scaled(&self, x_scaled: &[f64]) -> Vec<f64> {
        self.forward_scaled(x_scaled).pop().unwrap_or_default()
    }

    pub fn predict_raw(&self, x_raw: &[f64]) -> Vec<f64> {
        let xs = self.x_scaler.transform(x_raw);
        let ys = self.predict_scaled(&xs);
        self.y_scaler.inverse(&ys)
    }

    /// Generates Outotec HSC DataContract XML matching Fig. 11 (`<ANN z:Id="1" ...>`).
    pub fn to_xml(&self) -> String {
        let mut id = 1usize;
        let mut next_id = || {
            let cur = id;
            id += 1;
            cur
        };
        let ann_id = next_id();
        let layers_id = next_id();
        let mut out = format!(
            "<ANN z:Id=\"{ann_id}\"\n\
             xmlns=\"http://schemas.datacontract.org/2004/07/Outotec.HSC.NeuralNetwork\"\n\
             xmlns:i=\"http://www.w3.org/2001/XMLSchema-instance\"\n\
             xmlns:z=\"http://schemas.microsoft.com/2003/10/Serialization/\">\n\
             <Layers z:Id=\"{layers_id}\" z:Size=\"{}\">\n",
            self.layers.len()
        );
        for layer in &self.layers {
            let lid = next_id();
            let nid = next_id();
            out.push_str(&format!(
                "  <ANN.Layer z:Id=\"{lid}\" Activation=\"{}\">\n    <Neurons z:Id=\"{nid}\" z:Size=\"{}\">\n",
                layer.activation.label(),
                layer.n_out
            ));
            for (o, w) in layer.weights.iter().enumerate() {
                let neuron_id = next_id();
                let w_id = next_id();
                let dw_id = next_id();
                out.push_str(&format!(
                    "      <ANN.Neuron z:Id=\"{neuron_id}\">\n\
                     \x20       <Weights z:Id=\"{w_id}\" z:Size=\"{}\" xmlns:a=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\">",
                    w.len()
                ));
                for &val in w {
                    out.push_str(&format!("<a:double>{val:.15}</a:double>"));
                }
                out.push_str(&format!(
                    "</Weights>\n\
                     \x20       <DeltaWeights z:Id=\"{dw_id}\" z:Size=\"{}\" xmlns:a=\"http://schemas.microsoft.com/2003/10/Serialization/Arrays\">",
                    layer.delta_weights[o].len()
                ));
                for &dv in &layer.delta_weights[o] {
                    out.push_str(&format!("<a:double>{dv:.15e}</a:double>"));
                }
                out.push_str("</DeltaWeights>\n      </ANN.Neuron>\n");
            }
            out.push_str("    </Neurons>\n  </ANN.Layer>\n");
        }
        out.push_str("</Layers>\n<Scaling>\n  <XMin>");
        for (i, v) in self.x_scaler.mins.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{v:.8}"));
        }
        out.push_str("</XMin>\n  <XMax>");
        for (i, v) in self.x_scaler.maxs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{v:.8}"));
        }
        out.push_str("</XMax>\n  <YMin>");
        for (i, v) in self.y_scaler.mins.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{v:.8}"));
        }
        out.push_str("</YMin>\n  <YMax>");
        for (i, v) in self.y_scaler.maxs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&format!("{v:.8}"));
        }
        out.push_str("</YMax>\n</Scaling>\n</ANN>\n");
        out
    }
}

#[derive(Debug, Clone)]
pub struct FfnnTrainer {
    pub config: FfnnConfig,
    pub model: FfnnModel,
    pub best_model: FfnnModel,
    pub train_x: Vec<Vec<f64>>,
    pub train_y: Vec<Vec<f64>>,
    pub val_x: Vec<Vec<f64>>,
    pub val_y: Vec<Vec<f64>>,
    pub iteration: usize,
    pub train_mse_history: Vec<f64>,
    pub val_mse_history: Vec<f64>,
    pub best_val_mse: f64,
    pub best_iter: usize,
    pub running: bool,
    pub finished: bool,
    rng: SimpleRng,
}

impl FfnnTrainer {
    pub fn from_dataset(ds: &ParsedNnDataset, config: FfnnConfig) -> Result<Self, String> {
        let active: Vec<&NnSample> = ds.samples.iter().filter(|s| !s.excluded).collect();
        if active.len() < 3 {
            return Err("At least 3 non-excluded samples are required.".into());
        }
        let raw_x: Vec<Vec<f64>> = active.iter().map(|s| s.inputs.clone()).collect();
        let raw_y: Vec<Vec<f64>> = active.iter().map(|s| s.targets.clone()).collect();
        let x_scaler = MinMaxScaler::fit(&raw_x, -1.0, 1.0);
        let y_scaler = MinMaxScaler::fit(&raw_y, -1.0, 1.0);

        let mut indices: Vec<usize> = (0..active.len()).collect();
        let mut rng = SimpleRng::new(0xF5F5_2026_0042);
        rng.shuffle(&mut indices);

        let val_frac = (config.validation_pct / 100.0).clamp(0.05, 0.5);
        let n_val = ((active.len() as f64 * val_frac).round() as usize)
            .max(1)
            .min(active.len() - 2);
        let n_train = active.len() - n_val;

        let mut train_x = Vec::with_capacity(n_train);
        let mut train_y = Vec::with_capacity(n_train);
        let mut val_x = Vec::with_capacity(n_val);
        let mut val_y = Vec::with_capacity(n_val);

        for (pos, &idx) in indices.iter().enumerate() {
            let xs = x_scaler.transform(&raw_x[idx]);
            let ys = y_scaler.transform(&raw_y[idx]);
            if pos < n_train {
                train_x.push(xs);
                train_y.push(ys);
            } else {
                val_x.push(xs);
                val_y.push(ys);
            }
        }

        let n_in = raw_x[0].len();
        let n_out = raw_y[0].len();
        let model = FfnnModel::new(
            n_in,
            &config.hidden_layers,
            n_out,
            config.hidden_activation,
            config.output_activation,
            x_scaler,
            y_scaler,
            &mut rng,
        );
        let best_model = model.clone();

        Ok(Self {
            config,
            model,
            best_model,
            train_x,
            train_y,
            val_x,
            val_y,
            iteration: 0,
            train_mse_history: Vec::new(),
            val_mse_history: Vec::new(),
            best_val_mse: f64::INFINITY,
            best_iter: 0,
            running: false,
            finished: false,
            rng,
        })
    }

    /// Runs `epochs` backpropagation rounds incrementally (allows smooth UI updates + Stop button).
    pub fn step_epochs(&mut self, epochs: usize) {
        if !self.running || self.finished {
            return;
        }
        let n_train = self.train_x.len();
        let batch_sz = self.config.batch_size.clamp(1, n_train);
        let mut order: Vec<usize> = (0..n_train).collect();
        let n_layers = self.model.layers.len();

        for _ in 0..epochs {
            if self.iteration >= self.config.max_iterations {
                self.running = false;
                self.finished = true;
                break;
            }
            self.rng.shuffle(&mut order);
            let lr = self.config.learning_rate
                / (1.0 + self.config.learning_rate_decay * self.iteration as f64);
            let mom = self.config.momentum.clamp(0.0, 0.99);

            for chunk in order.chunks(batch_sz) {
                let mut grad_w: Vec<Vec<Vec<f64>>> = self
                    .model
                    .layers
                    .iter()
                    .map(|l| vec![vec![0.0; l.n_in + 1]; l.n_out])
                    .collect();

                for &idx in chunk {
                    let acts = self.model.forward_scaled(&self.train_x[idx]);
                    let target = &self.train_y[idx];
                    let mut deltas = vec![Vec::new(); n_layers];

                    // Output layer delta
                    let last = n_layers - 1;
                    let out_a = &acts[n_layers];
                    let out_act = self.model.layers[last].activation;
                    deltas[last] = (0..out_a.len())
                        .map(|o| (target[o] - out_a[o]) * out_act.deriv_from_output(out_a[o]))
                        .collect();

                    // Hidden layers deltas
                    for l in (0..last).rev() {
                        let cur_a = &acts[l + 1];
                        let cur_act = self.model.layers[l].activation;
                        let next_l = &self.model.layers[l + 1];
                        let mut d = vec![0.0; cur_a.len()];
                        for i in 0..cur_a.len() {
                            let mut sum = 0.0;
                            for o in 0..next_l.n_out {
                                sum += deltas[l + 1][o] * next_l.weights[o][i];
                            }
                            d[i] = sum * cur_act.deriv_from_output(cur_a[i]);
                        }
                        deltas[l] = d;
                    }

                    // Accumulate gradients
                    for l in 0..n_layers {
                        let prev_a = &acts[l];
                        let n_in = self.model.layers[l].n_in;
                        for (o, &dl) in deltas[l].iter().enumerate() {
                            for i in 0..n_in {
                                grad_w[l][o][i] += dl * prev_a[i];
                            }
                            grad_w[l][o][n_in] += dl;
                        }
                    }
                }

                let inv_b = 1.0 / chunk.len() as f64;
                for l in 0..n_layers {
                    let layer = &mut self.model.layers[l];
                    for o in 0..layer.n_out {
                        for i in 0..=layer.n_in {
                            let dw = lr * (grad_w[l][o][i] * inv_b) + mom * layer.delta_weights[o][i];
                            layer.weights[o][i] += dw;
                            layer.delta_weights[o][i] = dw;
                        }
                    }
                }
            }

            self.iteration += 1;
            let tr_mse = self.eval_mse(&self.train_x, &self.train_y);
            let va_mse = self.eval_mse(&self.val_x, &self.val_y);
            self.train_mse_history.push(tr_mse);
            self.val_mse_history.push(va_mse);

            if va_mse < self.best_val_mse {
                self.best_val_mse = va_mse;
                self.best_iter = self.iteration;
                self.best_model = self.model.clone();
            }
            if tr_mse <= self.config.stopping_mse {
                self.running = false;
                self.finished = true;
                break;
            }
        }
    }

    fn eval_mse(&self, xs: &[Vec<f64>], ys: &[Vec<f64>]) -> f64 {
        if xs.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut cnt = 0usize;
        for (x, y) in xs.iter().zip(ys.iter()) {
            let pred = self.model.predict_scaled(x);
            for (p, &t) in pred.iter().zip(y.iter()) {
                sum += (p - t).powi(2);
                cnt += 1;
            }
        }
        sum / cnt.max(1) as f64
    }

    pub fn parity_points(&self) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let tr: Vec<(f64, f64)> = self
            .train_x
            .iter()
            .zip(self.train_y.iter())
            .map(|(x, y)| (y[0], self.best_model.predict_scaled(x)[0]))
            .collect();
        let va: Vec<(f64, f64)> = self
            .val_x
            .iter()
            .zip(self.val_y.iter())
            .map(|(x, y)| (y[0], self.best_model.predict_scaled(x)[0]))
            .collect();
        (tr, va)
    }
}

// ============================================================================
// 4. RNN (Random Neural Network — Gelenbe Recurrent G-Network, Figs. 12–18)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RnnValidationMethod {
    Shepard,
    Random,
    Uniform,
}

impl RnnValidationMethod {
    pub const ALL: &'static [RnnValidationMethod] = &[
        RnnValidationMethod::Shepard,
        RnnValidationMethod::Random,
        RnnValidationMethod::Uniform,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RnnValidationMethod::Shepard => "Shepard",
            RnnValidationMethod::Random => "Random",
            RnnValidationMethod::Uniform => "Uniform",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RnnConfig {
    pub momentum: f64,
    pub supervising_param: f64,
    pub validation_method: RnnValidationMethod,
    pub regularized: bool,
    pub regularization_param: f64,
    pub batch_probability: f64,
    pub validation_pct: f64,
    pub training_pct: f64,
    pub stopping_mse: f64,
    pub max_iterations: usize,
    pub extra_hidden_neurons: usize,
}

impl Default for RnnConfig {
    fn default() -> Self {
        Self {
            momentum: 1e-7,
            supervising_param: 0.9,
            validation_method: RnnValidationMethod::Shepard,
            regularized: true,
            regularization_param: 0.0002,
            batch_probability: 0.8,
            validation_pct: 30.0,
            training_pct: 70.0,
            stopping_mse: 1e-5,
            max_iterations: 1000,
            extra_hidden_neurons: 2,
        }
    }
}

/// Gelenbe Random Neural Network with `n_neurons = n_inputs + extra_hidden + n_outputs`
/// arranged in a fully connected recurrent graph (`W+` excitation, `W-` inhibition).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RnnModel {
    pub n_inputs: usize,
    pub n_outputs: usize,
    pub n_neurons: usize,
    /// Positive (excitation) weights `w_plus[from][to] >= 0`.
    pub w_plus: Vec<Vec<f64>>,
    /// Negative (inhibition) weights `w_minus[from][to] >= 0`.
    pub w_minus: Vec<Vec<f64>>,
    pub x_scaler: MinMaxScaler,
    pub y_scaler: MinMaxScaler,
}

impl RnnModel {
    pub fn new(
        n_inputs: usize,
        extra_hidden: usize,
        n_outputs: usize,
        x_scaler: MinMaxScaler,
        y_scaler: MinMaxScaler,
        rng: &mut SimpleRng,
    ) -> Self {
        let n_neurons = (n_inputs + extra_hidden + n_outputs).clamp(3, 32);
        let mut w_plus = vec![vec![0.0; n_neurons]; n_neurons];
        let mut w_minus = vec![vec![0.0; n_neurons]; n_neurons];
        for i in 0..n_neurons {
            for j in 0..n_neurons {
                w_plus[i][j] = rng.range(0.05, 0.35);
                w_minus[i][j] = rng.range(0.05, 0.35);
            }
        }
        Self {
            n_inputs,
            n_outputs,
            n_neurons,
            w_plus,
            w_minus,
            x_scaler,
            y_scaler,
        }
    }

    /// Computes steady-state neuron firing probabilities `q in [0, 1]^N` via damped fixed-point iteration.
    pub fn solve_firing_rates(&self, x_scaled: &[f64]) -> Vec<f64> {
        let n = self.n_neurons;
        let mut r = vec![0.0; n];
        for i in 0..n {
            let mut sum = 0.0;
            for j in 0..n {
                sum += self.w_plus[i][j] + self.w_minus[i][j];
            }
            r[i] = sum.max(1e-6);
        }

        let mut ext_pos = vec![0.05; n];
        let mut ext_neg = vec![0.05; n];
        for i in 0..self.n_inputs.min(x_scaled.len()) {
            let v = x_scaled[i];
            if v >= 0.0 {
                ext_pos[i] += v;
            } else {
                ext_neg[i] += -v;
            }
        }

        let mut q = vec![0.5; n];
        for _ in 0..28 {
            let mut max_diff = 0.0_f64;
            let mut q_next = vec![0.0; n];
            for i in 0..n {
                let mut num = ext_pos[i];
                let mut den = r[i] + ext_neg[i];
                for j in 0..n {
                    num += q[j] * self.w_plus[j][i];
                    den += q[j] * self.w_minus[j][i];
                }
                let qi = (num / den.max(1e-6)).clamp(0.0, 1.0);
                q_next[i] = 0.65 * qi + 0.35 * q[i];
                max_diff = max_diff.max((q_next[i] - q[i]).abs());
            }
            q = q_next;
            if max_diff < 1e-6 {
                break;
            }
        }
        q
    }

    pub fn predict_scaled(&self, x_scaled: &[f64]) -> Vec<f64> {
        let q = self.solve_firing_rates(x_scaled);
        let out_start = self.n_neurons - self.n_outputs;
        q[out_start..].to_vec()
    }

    pub fn predict_raw(&self, x_raw: &[f64]) -> Vec<f64> {
        let xs = self.x_scaler.transform(x_raw);
        let ys = self.predict_scaled(&xs);
        self.y_scaler.inverse(&ys)
    }

    pub fn to_xml(&self) -> String {
        let mut out = format!(
            "<RNN z:Id=\"1\"\n\
             xmlns=\"http://schemas.datacontract.org/2004/07/Outotec.HSC.RandomNeuralNetwork\"\n\
             xmlns:z=\"http://schemas.microsoft.com/2003/10/Serialization/\"\n\
             Inputs=\"{}\" Outputs=\"{}\" Neurons=\"{}\">\n  <WPlus>\n",
            self.n_inputs, self.n_outputs, self.n_neurons
        );
        for (i, row) in self.w_plus.iter().enumerate() {
            out.push_str(&format!("    <Neuron Index=\"{i}\">"));
            for &v in row {
                out.push_str(&format!("<double>{v:.10}</double>"));
            }
            out.push_str("</Neuron>\n");
        }
        out.push_str("  </WPlus>\n  <WMinus>\n");
        for (i, row) in self.w_minus.iter().enumerate() {
            out.push_str(&format!("    <Neuron Index=\"{i}\">"));
            for &v in row {
                out.push_str(&format!("<double>{v:.10}</double>"));
            }
            out.push_str("</Neuron>\n");
        }
        out.push_str("  </WMinus>\n</RNN>\n");
        out
    }
}

#[derive(Debug, Clone)]
pub struct RnnTrainer {
    pub config: RnnConfig,
    pub model: RnnModel,
    pub best_model: RnnModel,
    pub train_x: Vec<Vec<f64>>,
    pub train_y: Vec<Vec<f64>>,
    pub val_x: Vec<Vec<f64>>,
    pub val_y: Vec<Vec<f64>>,
    pub iteration: usize,
    pub train_mse_history: Vec<f64>,
    pub val_mse_history: Vec<f64>,
    pub best_val_mse: f64,
    pub best_iter: usize,
    pub running: bool,
    pub finished: bool,
    vel_plus: Vec<Vec<f64>>,
    vel_minus: Vec<Vec<f64>>,
    rng: SimpleRng,
}

impl RnnTrainer {
    pub fn from_dataset(ds: &ParsedNnDataset, config: RnnConfig) -> Result<Self, String> {
        let active: Vec<&NnSample> = ds.samples.iter().filter(|s| !s.excluded).collect();
        if active.len() < 3 {
            return Err("At least 3 non-excluded samples are required for RNN.".into());
        }
        let raw_x: Vec<Vec<f64>> = active.iter().map(|s| s.inputs.clone()).collect();
        let raw_y: Vec<Vec<f64>> = active.iter().map(|s| s.targets.clone()).collect();
        let x_scaler = MinMaxScaler::fit(&raw_x, 0.05, 0.95);
        // Scale targets into [0.05, 0.95] so Gelenbe q_i in [0, 1] fits cleanly (matches Fig. 18 [0, 1] scale)
        let y_scaler = MinMaxScaler::fit(&raw_y, 0.05, 0.95);

        let mut rng = SimpleRng::new(0xA11C_E950_2026);
        let n = active.len();
        let val_frac = (config.validation_pct / 100.0).clamp(0.05, 0.5);
        let n_val = ((n as f64 * val_frac).round() as usize).max(1).min(n - 2);

        let mut is_val = vec![false; n];
        match config.validation_method {
            RnnValidationMethod::Uniform => {
                let stride = (n as f64 / n_val as f64).max(1.0);
                for k in 0..n_val {
                    let idx = ((k as f64 * stride).floor() as usize).min(n - 1);
                    is_val[idx] = true;
                }
            }
            RnnValidationMethod::Shepard => {
                // Select evenly distributed interior points by target quantile for Shepard coverage
                let mut order: Vec<usize> = (0..n).collect();
                order.sort_by(|&a, &b| raw_y[a][0].total_cmp(&raw_y[b][0]));
                let stride = (n as f64 / n_val as f64).max(1.0);
                for k in 0..n_val {
                    let pos = (((k as f64 + 0.5) * stride).floor() as usize).min(n - 1);
                    is_val[order[pos]] = true;
                }
            }
            RnnValidationMethod::Random => {
                let mut order: Vec<usize> = (0..n).collect();
                rng.shuffle(&mut order);
                for &idx in order.iter().take(n_val) {
                    is_val[idx] = true;
                }
            }
        }

        let mut train_x = Vec::new();
        let mut train_y = Vec::new();
        let mut val_x = Vec::new();
        let mut val_y = Vec::new();
        for i in 0..n {
            let xs = x_scaler.transform(&raw_x[i]);
            let ys = y_scaler.transform(&raw_y[i]);
            if is_val[i] {
                val_x.push(xs);
                val_y.push(ys);
            } else {
                train_x.push(xs);
                train_y.push(ys);
            }
        }

        let n_in = raw_x[0].len();
        let n_out = raw_y[0].len();
        let model = RnnModel::new(
            n_in,
            config.extra_hidden_neurons,
            n_out,
            x_scaler,
            y_scaler,
            &mut rng,
        );
        let nn = model.n_neurons;
        let best_model = model.clone();

        Ok(Self {
            config,
            model,
            best_model,
            train_x,
            train_y,
            val_x,
            val_y,
            iteration: 0,
            train_mse_history: Vec::new(),
            val_mse_history: Vec::new(),
            best_val_mse: f64::INFINITY,
            best_iter: 0,
            running: false,
            finished: false,
            vel_plus: vec![vec![0.0; nn]; nn],
            vel_minus: vec![vec![0.0; nn]; nn],
            rng,
        })
    }

    /// Gelenbe gradient step on `W+` and `W-` using the linear sensitivity approximation `(I - W)^{-1}`
    /// with L2 regularization, momentum, and stochastic `batch_probability`.
    pub fn step_epochs(&mut self, epochs: usize) {
        if !self.running || self.finished {
            return;
        }
        let nn = self.model.n_neurons;
        let out_start = nn - self.model.n_outputs;
        let lr = (0.18 * self.config.supervising_param).clamp(0.01, 0.45);
        let mom = self.config.momentum.clamp(0.0, 0.95);
        let reg = if self.config.regularized {
            self.config.regularization_param.max(0.0)
        } else {
            0.0
        };
        let p_batch = self.config.batch_probability.clamp(0.1, 1.0);

        for _ in 0..epochs {
            if self.iteration >= self.config.max_iterations {
                self.running = false;
                self.finished = true;
                break;
            }
            let mut g_plus = vec![vec![0.0; nn]; nn];
            let mut g_minus = vec![vec![0.0; nn]; nn];
            let mut used = 0usize;

            for s_idx in 0..self.train_x.len() {
                if self.rng.next_f64() > p_batch && used > 0 {
                    continue;
                }
                used += 1;
                let x = &self.train_x[s_idx];
                let target = &self.train_y[s_idx];
                let q = self.model.solve_firing_rates(x);

                // Compute effective denominator D_i = r_i + ext_neg_i + sum_j q_j w_minus[j][i]
                let mut d_vec = vec![0.05; nn];
                for i in 0..nn {
                    let mut r_i = 0.0;
                    for j in 0..nn {
                        r_i += self.model.w_plus[i][j] + self.model.w_minus[i][j];
                    }
                    let mut inh = 0.0;
                    for j in 0..nn {
                        inh += q[j] * self.model.w_minus[j][i];
                    }
                    d_vec[i] = (r_i + inh + 0.05).max(1e-4);
                }

                // Back-propagate output error through 2 recurrent steps of the Jacobian J[to][from]
                let mut adj = vec![0.0; nn];
                for o in 0..self.model.n_outputs {
                    let oi = out_start + o;
                    adj[oi] = target[o] - q[oi];
                }
                for _ in 0..2 {
                    let mut next_adj = adj.clone();
                    for from in 0..nn {
                        let mut acc = 0.0;
                        for to in 0..nn {
                            let j_tf = (self.model.w_plus[from][to]
                                - q[to] * self.model.w_minus[from][to])
                                / d_vec[to];
                            acc += adj[to] * j_tf;
                        }
                        next_adj[from] = adj[from] + 0.45 * acc;
                    }
                    adj = next_adj;
                }

                // Exact partial derivatives of q_to w.r.t. w_plus[from][to] and w_minus[from][to]
                for from in 0..nn {
                    let qf = q[from];
                    for to in 0..nn {
                        let d_to = d_vec[to];
                        let d_from = d_vec[from];
                        let dq_to_dwp = qf / d_to;
                        let dq_from_dwp = -q[from] / d_from;
                        let dq_to_dwm = -qf * q[to] / d_to;
                        let dq_from_dwm = -q[from] / d_from;
                        g_plus[from][to] += adj[to] * dq_to_dwp + adj[from] * dq_from_dwp;
                        g_minus[from][to] += adj[to] * dq_to_dwm + adj[from] * dq_from_dwm;
                    }
                }
            }

            let inv = 1.0 / used.max(1) as f64;
            for i in 0..nn {
                for j in 0..nn {
                    let dp = lr * (g_plus[i][j] * inv - reg * self.model.w_plus[i][j])
                        + mom * self.vel_plus[i][j];
                    let dm = lr * (g_minus[i][j] * inv - reg * self.model.w_minus[i][j])
                        + mom * self.vel_minus[i][j];
                    self.model.w_plus[i][j] = (self.model.w_plus[i][j] + dp).clamp(1e-5, 8.0);
                    self.model.w_minus[i][j] = (self.model.w_minus[i][j] + dm).clamp(1e-5, 8.0);
                    self.vel_plus[i][j] = dp;
                    self.vel_minus[i][j] = dm;
                }
            }

            self.iteration += 1;
            let tr_mse = self.eval_mse(&self.train_x, &self.train_y);
            let va_mse = self.eval_mse(&self.val_x, &self.val_y);
            self.train_mse_history.push(tr_mse);
            self.val_mse_history.push(va_mse);

            if va_mse < self.best_val_mse {
                self.best_val_mse = va_mse;
                self.best_iter = self.iteration;
                self.best_model = self.model.clone();
            }
            if tr_mse <= self.config.stopping_mse {
                self.running = false;
                self.finished = true;
                break;
            }
        }
    }

    fn eval_mse(&self, xs: &[Vec<f64>], ys: &[Vec<f64>]) -> f64 {
        if xs.is_empty() {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut cnt = 0usize;
        for (x, y) in xs.iter().zip(ys.iter()) {
            let pred = self.model.predict_scaled(x);
            for (p, &t) in pred.iter().zip(y.iter()) {
                sum += (p - t).powi(2);
                cnt += 1;
            }
        }
        sum / cnt.max(1) as f64
    }

    pub fn parity_points(&self) -> (Vec<(f64, f64)>, Vec<(f64, f64)>) {
        let tr: Vec<(f64, f64)> = self
            .train_x
            .iter()
            .zip(self.train_y.iter())
            .map(|(x, y)| (y[0], self.best_model.predict_scaled(x)[0]))
            .collect();
        let va: Vec<(f64, f64)> = self
            .val_x
            .iter()
            .zip(self.val_y.iter())
            .map(|(x, y)| (y[0], self.best_model.predict_scaled(x)[0]))
            .collect();
        (tr, va)
    }
}

// ============================================================================
// 5. PSO (Particle Swarm Optimization + Excel Formula Evaluator, §95.6)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PsoBoundaryCondition {
    Damping,
    Absorbing,
    Reflecting,
    Invisible,
}

impl PsoBoundaryCondition {
    pub const ALL: &'static [PsoBoundaryCondition] = &[
        PsoBoundaryCondition::Damping,
        PsoBoundaryCondition::Absorbing,
        PsoBoundaryCondition::Reflecting,
        PsoBoundaryCondition::Invisible,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PsoBoundaryCondition::Damping => "Damping",
            PsoBoundaryCondition::Absorbing => "Absorbing",
            PsoBoundaryCondition::Reflecting => "Reflecting",
            PsoBoundaryCondition::Invisible => "Invisible",
        }
    }
}

/// PSO parameters matching HSC Data Fig. 22 (`PSO Properties`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsoConfig {
    pub particles_per_variable: usize,
    pub extra_particles: usize,
    pub max_rounds: usize,
    pub boundary_condition: PsoBoundaryCondition,
    pub use_diversity: bool,
    pub w: f64,
    pub w_min: f64,
    pub w_damp: f64,
    pub c1: f64,
    pub c2: f64,
    pub c3: f64,
}

impl Default for PsoConfig {
    fn default() -> Self {
        Self {
            particles_per_variable: 50,
            extra_particles: 300,
            max_rounds: 1000,
            boundary_condition: PsoBoundaryCondition::Damping,
            use_diversity: false,
            w: 0.9,
            w_min: 0.4,
            w_damp: 0.999,
            c1: 0.5,
            c2: 1.5,
            c3: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PsoVarSpec {
    pub cell_ref: String,
    pub min_val: f64,
    pub max_val: f64,
}

#[derive(Debug, Clone)]
pub struct PsoParticle {
    pub pos: Vec<f64>,
    pub vel: Vec<f64>,
    pub pbest_pos: Vec<f64>,
    pub pbest_val: f64,
}

#[derive(Debug, Clone)]
pub struct PsoOptimizer {
    pub config: PsoConfig,
    pub vars: Vec<PsoVarSpec>,
    pub objective_formulas: Vec<String>,
    pub base_cells: HashMap<String, f64>,
    pub particles: Vec<PsoParticle>,
    pub gbest_pos: Vec<f64>,
    pub gbest_val: f64,
    pub current_w: f64,
    pub iteration: usize,
    pub stagnant_rounds: usize,
    pub history: Vec<f64>,
    pub running: bool,
    pub finished: bool,
    rng: SimpleRng,
}

impl PsoOptimizer {
    pub fn new(
        config: PsoConfig,
        vars: Vec<PsoVarSpec>,
        objective_formulas: Vec<String>,
        base_cells: HashMap<String, f64>,
    ) -> Result<Self, String> {
        if vars.is_empty() {
            return Err("Specify at least one variable with Set X (PSO_X/min/max).".into());
        }
        if objective_formulas.is_empty() {
            return Err("Specify at least one objective cell with Set M (PSO_M).".into());
        }
        let mut rng = SimpleRng::new(0x5050_2026_1337);
        let dim = vars.len();
        let n_part = (config.particles_per_variable.saturating_mul(dim) + config.extra_particles)
            .clamp(12, 600);
        let mut particles = Vec::with_capacity(n_part);
        let mut gbest_pos = vec![0.0; dim];
        let mut gbest_val = f64::INFINITY;

        for _ in 0..n_part {
            let mut pos = Vec::with_capacity(dim);
            let mut vel = Vec::with_capacity(dim);
            for v in &vars {
                let lo = v.min_val.min(v.max_val);
                let hi = v.max_val.max(lo + 1e-6);
                pos.push(rng.range(lo, hi));
                let span = (hi - lo) * 0.2;
                vel.push(rng.range(-span, span));
            }
            let val = eval_pso_objectives(&vars, &pos, &objective_formulas, &base_cells)?;
            if val < gbest_val {
                gbest_val = val;
                gbest_pos = pos.clone();
            }
            particles.push(PsoParticle {
                pbest_pos: pos.clone(),
                pbest_val: val,
                pos,
                vel,
            });
        }

        let current_w = config.w.clamp(0.05, 1.5);
        Ok(Self {
            config,
            vars,
            objective_formulas,
            base_cells,
            particles,
            gbest_pos,
            gbest_val,
            current_w,
            iteration: 0,
            stagnant_rounds: 0,
            history: vec![gbest_val],
            running: true,
            finished: false,
            rng,
        })
    }

    pub fn step_iterations(&mut self, steps: usize) -> Result<(), String> {
        if !self.running || self.finished {
            return Ok(());
        }
        let dim = self.vars.len();
        for _ in 0..steps {
            if self.iteration >= self.config.max_rounds {
                self.running = false;
                self.finished = true;
                break;
            }
            let prev_best = self.gbest_val;
            let n_part = self.particles.len();

            // Optional centroid for diversity repulsion when `use_diversity` is True
            let mut mean_pos = vec![0.0; dim];
            if self.config.use_diversity && n_part > 0 {
                for p in &self.particles {
                    for d in 0..dim {
                        mean_pos[d] += p.pos[d];
                    }
                }
                for m in &mut mean_pos {
                    *m /= n_part as f64;
                }
            }

            let w = self.current_w;
            for p in &mut self.particles {
                let mut out_of_bounds = false;
                for d in 0..dim {
                    let lo = self.vars[d].min_val.min(self.vars[d].max_val);
                    let hi = self.vars[d].max_val.max(lo + 1e-6);
                    let span = hi - lo;
                    let r1 = self.rng.next_f64();
                    let r2 = self.rng.next_f64();
                    let div_term = if self.config.use_diversity {
                        let r3 = self.rng.next_f64() - 0.5;
                        0.05 * self.config.c3 * r3 * (p.pos[d] - mean_pos[d])
                    } else {
                        0.0
                    };
                    let v = w * p.vel[d]
                        + self.config.c1 * r1 * (p.pbest_pos[d] - p.pos[d])
                        + self.config.c2 * r2 * (self.gbest_pos[d] - p.pos[d])
                        + div_term;
                    let vmax = span * 0.5;
                    p.vel[d] = v.clamp(-vmax, vmax);
                    let next_x = p.pos[d] + p.vel[d];

                    if next_x < lo || next_x > hi {
                        match self.config.boundary_condition {
                            PsoBoundaryCondition::Damping => {
                                p.pos[d] = next_x.clamp(lo, hi);
                                p.vel[d] = -0.5 * p.vel[d];
                            }
                            PsoBoundaryCondition::Absorbing => {
                                p.pos[d] = next_x.clamp(lo, hi);
                                p.vel[d] = 0.0;
                            }
                            PsoBoundaryCondition::Reflecting => {
                                let reflected = if next_x < lo {
                                    lo + (lo - next_x)
                                } else {
                                    hi - (next_x - hi)
                                };
                                p.pos[d] = reflected.clamp(lo, hi);
                                p.vel[d] = -p.vel[d];
                            }
                            PsoBoundaryCondition::Invisible => {
                                p.pos[d] = next_x;
                                out_of_bounds = true;
                            }
                        }
                    } else {
                        p.pos[d] = next_x;
                    }
                }
                if out_of_bounds {
                    continue;
                }
                let val = eval_pso_objectives(
                    &self.vars,
                    &p.pos,
                    &self.objective_formulas,
                    &self.base_cells,
                )?;
                if val < p.pbest_val {
                    p.pbest_val = val;
                    p.pbest_pos = p.pos.clone();
                }
                if val < self.gbest_val {
                    self.gbest_val = val;
                    self.gbest_pos = p.pos.clone();
                }
            }

            self.current_w = (self.current_w * self.config.w_damp.clamp(0.8, 1.0))
                .max(self.config.w_min.clamp(0.05, 1.0));
            self.iteration += 1;
            self.history.push(self.gbest_val);

            if (prev_best - self.gbest_val).abs() < 1e-12 {
                self.stagnant_rounds += 1;
            } else {
                self.stagnant_rounds = 0;
            }
            if self.stagnant_rounds >= 120 && self.iteration >= 150 {
                self.running = false;
                self.finished = true;
                break;
            }
        }
        Ok(())
    }
}

fn eval_pso_objectives(
    vars: &[PsoVarSpec],
    pos: &[f64],
    formulas: &[String],
    base_cells: &HashMap<String, f64>,
) -> Result<f64, String> {
    let mut env = base_cells.clone();
    for (spec, &val) in vars.iter().zip(pos.iter()) {
        env.insert(spec.cell_ref.to_ascii_uppercase(), val);
    }
    let mut total = 0.0;
    for f in formulas {
        let v = eval_excel_expr(f, &env)?;
        if !v.is_finite() {
            return Ok(1e30);
        }
        total += v;
    }
    Ok(total)
}

/// Evaluates an Excel-style mathematical formula (`=ABS(A1-3) + SIN(B1)^2 + EXP(-A1)`).
pub fn eval_excel_expr(expr: &str, env: &HashMap<String, f64>) -> Result<f64, String> {
    let trimmed = expr.trim().trim_start_matches('=');
    let mut parser = ExprParser::new(trimmed, env);
    let val = parser.parse_add_sub()?;
    parser.skip_ws();
    if parser.pos < parser.bytes.len() {
        return Err(format!(
            "Unexpected token '{}' in formula '{expr}'",
            &trimmed[parser.pos..]
        ));
    }
    Ok(val)
}

struct ExprParser<'a> {
    bytes: &'a [u8],
    src: &'a str,
    pos: usize,
    env: &'a HashMap<String, f64>,
}

impl<'a> ExprParser<'a> {
    fn new(src: &'a str, env: &'a HashMap<String, f64>) -> Self {
        Self {
            bytes: src.as_bytes(),
            src,
            pos: 0,
            env,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_add_sub(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_mul_div()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    lhs += self.parse_mul_div()?;
                }
                Some(b'-') => {
                    self.pos += 1;
                    lhs -= self.parse_mul_div()?;
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_mul_div(&mut self) -> Result<f64, String> {
        let mut lhs = self.parse_power()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    lhs *= self.parse_power()?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    let rhs = self.parse_power()?;
                    lhs = if rhs.abs() < 1e-18 {
                        1e30
                    } else {
                        lhs / rhs
                    };
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if self.peek() == Some(b'^') {
            self.pos += 1;
            let exp = self.parse_power()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'+') => {
                self.pos += 1;
                self.parse_unary()
            }
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        let ch = self.peek().ok_or_else(|| "Unexpected end of formula".to_string())?;
        if ch == b'(' {
            self.pos += 1;
            let val = self.parse_add_sub()?;
            if self.peek() != Some(b')') {
                return Err("Missing closing ')'".into());
            }
            self.pos += 1;
            return Ok(val);
        }
        if ch.is_ascii_digit() || ch == b'.' {
            let start = self.pos;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
            {
                self.pos += 1;
            }
            if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'e' | b'E') {
                self.pos += 1;
                if self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b'+' | b'-') {
                    self.pos += 1;
                }
                while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                    self.pos += 1;
                }
            }
            return self.src[start..self.pos]
                .parse::<f64>()
                .map_err(|_| format!("Invalid number '{}'", &self.src[start..self.pos]));
        }
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let start = self.pos;
            while self.pos < self.bytes.len()
                && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
            {
                self.pos += 1;
            }
            let ident = self.src[start..self.pos].to_ascii_uppercase();
            if self.peek() == Some(b'(') {
                self.pos += 1;
                let mut args = Vec::new();
                if self.peek() != Some(b')') {
                    loop {
                        args.push(self.parse_add_sub()?);
                        if matches!(self.peek(), Some(b',') | Some(b';')) {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                }
                if self.peek() != Some(b')') {
                    return Err(format!("Missing ')' after function {ident}"));
                }
                self.pos += 1;
                return eval_builtin_fn(&ident, &args);
            }
            if ident == "PI" {
                return Ok(std::f64::consts::PI);
            }
            if ident == "E" {
                return Ok(std::f64::consts::E);
            }
            return self
                .env
                .get(&ident)
                .copied()
                .ok_or_else(|| format!("Unknown variable or cell '{ident}'"));
        }
        Err(format!("Unexpected character '{}'", ch as char))
    }
}

fn eval_builtin_fn(name: &str, args: &[f64]) -> Result<f64, String> {
    match name {
        "ABS" if args.len() == 1 => Ok(args[0].abs()),
        "SIN" if args.len() == 1 => Ok(args[0].sin()),
        "COS" if args.len() == 1 => Ok(args[0].cos()),
        "TAN" if args.len() == 1 => Ok(args[0].tan()),
        "EXP" if args.len() == 1 => Ok(args[0].clamp(-700.0, 700.0).exp()),
        "LN" if args.len() == 1 => Ok(args[0].max(1e-18).ln()),
        "LOG" | "LOG10" if args.len() == 1 => Ok(args[0].max(1e-18).log10()),
        "SQRT" if args.len() == 1 => Ok(args[0].max(0.0).sqrt()),
        "POW" | "POWER" if args.len() == 2 => Ok(args[0].powf(args[1])),
        "MIN" if !args.is_empty() => Ok(args.iter().copied().fold(f64::INFINITY, f64::min)),
        "MAX" if !args.is_empty() => Ok(args.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
        "SUM" => Ok(args.iter().sum()),
        "PI" if args.is_empty() => Ok(std::f64::consts::PI),
        _ => Err(format!("Unsupported function '{name}' with {} args", args.len())),
    }
}

/// Parses a `PSO_X/min/max` or `PSO_X//` comment string (`§95.6`), falling back to `(default_lo, default_hi)`.
pub fn parse_pso_x_comment_with_defaults(
    comment: &str,
    default_lo: f64,
    default_hi: f64,
) -> Option<(f64, f64)> {
    let s = comment.trim();
    let rest = s.strip_prefix("PSO_X/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let lo = parse_flex_f64(parts[0]).unwrap_or(default_lo);
    let hi = parse_flex_f64(parts[1]).unwrap_or(default_hi);
    Some((lo, hi))
}

/// Parses a `PSO_X/min/max` comment string (`§95.6`) with default bounds `(-10.0, 10.0)`.
pub fn parse_pso_x_comment(comment: &str) -> Option<(f64, f64)> {
    parse_pso_x_comment_with_defaults(comment, -10.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffnn_learns_nonlinear_function_and_exports_xml() {
        let mut grid = vec![
            vec!["RULE".into(), "X".into(), "X".into(), "Y".into(), "YNN".into()],
            vec!["".into(), "Var1".into(), "Var2".into(), "Out".into(), "Pred".into()],
            vec!["".into(); 5],
        ];
        let mut rng = SimpleRng::new(77);
        for i in 0..45 {
            let x1 = rng.range(-2.0, 2.0);
            let x2 = rng.range(-2.0, 2.0);
            let y = 2.5 * x1 - 1.5 * x2 + 0.4 * x1 * x2;
            let rule = if i % 9 == 0 { "E" } else { "" };
            grid.push(vec![
                rule.into(),
                format!("{x1:.4}"),
                format!("{x2:.4}"),
                format!("{y:.4}"),
                String::new(),
            ]);
        }

        let ds = parse_nn_table(&grid).unwrap();
        assert_eq!(ds.x_cols, vec![1, 2]);
        assert_eq!(ds.y_cols, vec![3]);
        assert_eq!(ds.excluded_count, 5);

        let cfg = FfnnConfig {
            hidden_layers: vec![8],
            learning_rate: 0.05,
            max_iterations: 400,
            ..Default::default()
        };
        let mut tr = FfnnTrainer::from_dataset(&ds, cfg).unwrap();
        tr.running = true;
        tr.step_epochs(300);
        assert!(tr.best_val_mse < 0.05, "best_val_mse={}", tr.best_val_mse);
        let xml = tr.best_model.to_xml();
        assert!(xml.contains("<ANN z:Id=\"1\""));
        assert!(xml.contains("Outotec.HSC.NeuralNetwork"));
        assert!(xml.contains("<DeltaWeights"));
    }

    #[test]
    fn rnn_learns_iris_like_dataset_and_generates_xml() {
        let mut grid = vec![
            vec!["RULE".into(), "X".into(), "X".into(), "Y".into(), "YNN".into()],
            vec!["".into(), "SepalL".into(), "PetalL".into(), "Class".into(), "Pred".into()],
            vec!["".into(); 5],
        ];
        for i in 0..30 {
            let x1 = if i < 15 { 5.0 + (i as f64) * 0.04 } else { 6.6 + ((i - 15) as f64) * 0.04 };
            let x2 = if i < 15 { 1.4 + (i as f64) * 0.02 } else { 5.2 + ((i - 15) as f64) * 0.03 };
            let y = if i < 15 { 1.0 } else { 2.0 };
            let rule = if i == 2 || i == 17 { "E" } else { "" };
            grid.push(vec![
                rule.into(),
                format!("{x1:.2}"),
                format!("{x2:.2}"),
                format!("{y:.2}"),
                String::new(),
            ]);
        }
        let ds = parse_nn_table(&grid).unwrap();
        let cfg = RnnConfig {
            max_iterations: 150,
            ..Default::default()
        };
        let mut tr = RnnTrainer::from_dataset(&ds, cfg).unwrap();
        tr.running = true;
        tr.step_epochs(120);
        assert!(tr.best_val_mse < 0.15, "rnn best_val_mse={}", tr.best_val_mse);
        let xml = tr.best_model.to_xml();
        assert!(xml.contains("<RNN z:Id=\"1\""));
        assert!(xml.contains("<WPlus>"));
        assert!(xml.contains("<WMinus>"));
    }

    #[test]
    fn pso_minimizes_2d_parabola_and_evaluates_excel_functions() {
        let vars = vec![
            PsoVarSpec { cell_ref: "A2".into(), min_val: -10.0, max_val: 10.0 },
            PsoVarSpec { cell_ref: "B2".into(), min_val: -10.0, max_val: 10.0 },
        ];
        let mut pso = PsoOptimizer::new(
            PsoConfig::default(),
            vars,
            vec!["=(A2 - 3)^2 + (B2 + 2)^2 + ABS(SIN(A2 - 3))".into()],
            HashMap::new(),
        )
        .unwrap();
        pso.step_iterations(150).unwrap();
        assert!(pso.gbest_val < 1e-3, "gbest_val={}", pso.gbest_val);
        assert!((pso.gbest_pos[0] - 3.0).abs() < 0.05);
        assert!((pso.gbest_pos[1] - (-2.0)).abs() < 0.05);
    }

    #[test]
    fn pso_solves_fig21_cross_in_tray_function() {
        let formula = "=-0.0001*(ABS(SIN(B2)*SIN(C2))*EXP(ABS(100-SQRT(B2^2+C2^2)/PI()))+1)^0.1";
        let mut env0 = HashMap::new();
        env0.insert("B2".into(), 0.0);
        env0.insert("C2".into(), 0.0);
        let f0 = eval_excel_expr(formula, &env0).unwrap();
        assert!((f0 - (-0.0001)).abs() < 1e-12, "f(0,0)={f0}");

        assert_eq!(
            parse_pso_x_comment_with_defaults("PSO_X//", -5.0, 5.0),
            Some((-5.0, 5.0))
        );
        assert_eq!(
            parse_pso_x_comment_with_defaults("PSO_X/-10/10", -5.0, 5.0),
            Some((-10.0, 10.0))
        );

        let vars = vec![
            PsoVarSpec { cell_ref: "B2".into(), min_val: -10.0, max_val: 10.0 },
            PsoVarSpec { cell_ref: "C2".into(), min_val: -10.0, max_val: 10.0 },
        ];
        let mut pso = PsoOptimizer::new(
            PsoConfig::default(),
            vars,
            vec![formula.into()],
            HashMap::new(),
        )
        .unwrap();
        pso.step_iterations(180).unwrap();
        assert!(
            pso.gbest_val < -2.062,
            "Cross-in-tray minimum expected ~ -2.06261, got {}",
            pso.gbest_val
        );
        assert!(
            (pso.gbest_pos[0].abs() - 1.34941).abs() < 0.05,
            "x={}",
            pso.gbest_pos[0]
        );
        assert!(
            (pso.gbest_pos[1].abs() - 1.34941).abs() < 0.05,
            "y={}",
            pso.gbest_pos[1]
        );
    }

    #[test]
    fn chart_analytics_computes_exact_linear_regression() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        let ys = [3.0, 5.0, 7.0, 9.0];
        let st = compute_column_analytics("Y", &xs, &ys);
        assert_eq!(st.count, 4);
        assert!((st.mean - 6.0).abs() < 1e-9);
        assert!((st.slope.unwrap() - 2.0).abs() < 1e-9);
        assert!((st.intercept.unwrap() - 1.0).abs() < 1e-9);
        assert!((st.r_squared.unwrap() - 1.0).abs() < 1e-9);
    }
}
