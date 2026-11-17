use eframe::egui;

/// Cross-platform path inside the OS temporary directory (`/tmp/<name>` on Linux, `%TEMP%\<name>` on Windows).
pub fn temp_file_path(filename: &str) -> String {
    std::env::temp_dir()
        .join(filename)
        .to_string_lossy()
        .into_owned()
}

/// 4-column periodic-style element grid matching HSC6/HSC9 Tpp/Lpp (100 elements).
pub const ELEMENT_GRID_4COL: &[&str] = &[
    "Ac", "Dy", "Mn", "Ru",
    "Ag", "Er", "Mo", "S",
    "Al", "Es", "N", "Sb",
    "Am", "Eu", "Na", "Sc",
    "Ar", "F", "Nb", "Se",
    "As", "Fe", "Nd", "Si",
    "At", "Fm", "Ne", "Sm",
    "Au", "Fr", "Ni", "Sn",
    "B", "Ga", "Np", "Sr",
    "Ba", "Gd", "O", "Ta",
    "Be", "Ge", "Os", "Tb",
    "Bi", "H", "P", "Tc",
    "Bk", "He", "Pa", "Te",
    "Br", "Hf", "Pb", "Th",
    "C", "Hg", "Pd", "Ti",
    "Ca", "Ho", "Pm", "Tl",
    "Cd", "I", "Po", "Tm",
    "Ce", "In", "Pr", "U",
    "Cf", "Ir", "Pt", "V",
    "Cl", "K", "Pu", "W",
    "Cm", "Kr", "Ra", "Xe",
    "Co", "La", "Rb", "Y",
    "Cr", "Li", "Re", "Yb",
    "Cs", "Lu", "Rh", "Zn",
    "Cu", "Mg", "Rn", "Zr",
];

/// Draw a dashed or dotted line segment between `p0` and `p1`.
pub fn draw_dashed_line(
    painter: &egui::Painter,
    p0: egui::Pos2,
    p1: egui::Pos2,
    dash_len: f32,
    gap_len: f32,
    stroke: egui::Stroke,
) {
    let delta = p1 - p0;
    let total = delta.length();
    if total <= 1.0 {
        return;
    }
    let dir = delta / total;
    let mut dist = 0.0_f32;
    while dist < total {
        let end = (dist + dash_len).min(total);
        painter.line_segment([p0 + dir * dist, p0 + dir * end], stroke);
        dist += dash_len + gap_len;
    }
}

/// Compute a human-friendly axis tick step near `raw`.
pub fn nice_step(raw: f64) -> f64 {
    if raw <= 0.0 || !raw.is_finite() {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let mag = 10_f64.powf(exp);
    let norm = raw / mag;
    let mult = if norm <= 1.5 {
        1.0
    } else if norm <= 3.0 {
        2.0
    } else if norm <= 7.0 {
        5.0
    } else {
        10.0
    };
    mult * mag
}

/// Generate nice axis tick values within `[min, max]`.
pub fn nice_ticks(min: f64, max: f64, approx_count: usize) -> Vec<f64> {
    let span = (max - min).abs();
    if span <= 1e-12 {
        return vec![min];
    }
    let step = nice_step(span / approx_count.max(2) as f64);
    let start = (min / step).ceil() * step;
    let mut ticks = Vec::new();
    let mut v = start;
    while v <= max + 1e-9 * step {
        ticks.push(v);
        v += step;
    }
    ticks
}

/// Format an axis tick cleanly without `-0`.
pub fn format_tick(v: f64) -> String {
    if v.abs() < 1e-9 {
        "0".into()
    } else if (v - v.round()).abs() < 1e-6 {
        format!("{:.0}", v.round())
    } else {
        format!("{v:.1}")
    }
}

/// Draw a 90-degree counter-clockwise rotated Y-axis title.
pub fn draw_rotated_y_title(
    painter: &egui::Painter,
    x: f32,
    center_y: f32,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let mut shape = egui::epaint::TextShape::new(
        egui::pos2(x, center_y + 0.5 * galley.size().x),
        galley,
        color,
    );
    shape.angle = -std::f32::consts::FRAC_PI_2;
    painter.add(egui::Shape::Text(shape));
}

/// Return true if a chemical formula contains both C and H (organic heuristic).
pub fn is_organic_formula(formula: &str) -> bool {
    soryntel_solver::parse_term(formula)
        .map(|p| p.elements.contains_key("C") && p.elements.contains_key("H"))
        .unwrap_or(false)
}

/// Shared tab action returned by `render_chart_tabs`.
pub enum ChartTabAction {
    None,
    SelectChart(usize),
    CloseChart(usize),
    SelectTable,
}

/// Render a shared `Chart 1 ×  Chart 2 ×  ...  Table` tab bar.
pub fn render_chart_tabs(
    ui: &mut egui::Ui,
    chart_names: &[&str],
    active_chart: Option<usize>,
    table_selected: bool,
) -> ChartTabAction {
    let mut action = ChartTabAction::None;
    ui.horizontal_wrapped(|ui| {
        for (idx, &name) in chart_names.iter().enumerate() {
            let selected = active_chart == Some(idx) && !table_selected;
            if ui.selectable_label(selected, name).clicked() {
                action = ChartTabAction::SelectChart(idx);
            }
            if chart_names.len() > 1 && ui.small_button("×").clicked() {
                action = ChartTabAction::CloseChart(idx);
            }
        }
        if ui.selectable_label(table_selected, "Table").clicked() {
            action = ChartTabAction::SelectTable;
        }
    });
    action
}
