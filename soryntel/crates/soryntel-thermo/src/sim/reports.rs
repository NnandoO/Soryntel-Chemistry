use crate::sim::model::{SimProject, SimStream, SimUnit, StreamConnectivity};
use crate::sim::presets::metric_value_and_unit;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimReportSheet {
    pub name: String,
    pub rows: Vec<Vec<String>>,
    pub freeze_rows: usize,
    pub freeze_cols: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimReportWorkbook {
    pub sheets: Vec<SimReportSheet>,
}

fn unit_name_opt(project: &SimProject, unit_id: Option<usize>) -> String {
    match unit_id {
        None => "(External)".to_string(),
        Some(id) => project
            .units
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.name_id.clone())
            .unwrap_or_else(|| format!("Unit#{id}")),
    }
}

fn connectivity_label(s: &SimStream) -> &'static str {
    match s.connectivity_kind() {
        StreamConnectivity::Input => "Input",
        StreamConnectivity::Internal => "Internal",
        StreamConnectivity::Output => "Output",
    }
}

fn fmt2(v: f64) -> String {
    if v.abs() < 1e-12 {
        "0.00".to_string()
    } else if v.abs() >= 10000.0 || v.abs() < 0.001 {
        format!("{v:.3e}")
    } else {
        format!("{v:.2}")
    }
}

fn fmt3(v: f64) -> String {
    if v.abs() < 1e-12 {
        "0.000".to_string()
    } else {
        format!("{v:.3}")
    }
}

fn unit_streams<'a>(project: &'a SimProject, unit: &SimUnit) -> (Vec<&'a SimStream>, Vec<&'a SimStream>) {
    let ins: Vec<&SimStream> = project
        .streams
        .iter()
        .filter(|s| s.dest_unit == Some(unit.id))
        .collect();
    let outs: Vec<&SimStream> = project
        .streams
        .iter()
        .filter(|s| s.source_unit == Some(unit.id))
        .collect();
    (ins, outs)
}

pub fn generate_sim_reports(project: &SimProject) -> SimReportWorkbook {
    // ------------------------------------------------------------------
    // Sheet 1: Unit Balance - General (Fig. 24)
    // ------------------------------------------------------------------
    let mut ub_rows = vec![
        vec![
            "Project:".into(),
            project.process_info.project_name.clone(),
            "HSC Ver:".into(),
            project.process_info.hsc_ver.clone(),
            "Criteria:".into(),
            project.calc_settings.criteria.as_str().into(),
            "Tol %:".into(),
            fmt3(project.calc_settings.tolerance_pct),
        ],
        vec![
            "Unit Name".into(),
            "Mode".into(),
            "Model Type".into(),
            "Model Name".into(),
            "Avail. %".into(),
            "Inputs".into(),
            "Outputs".into(),
            "Mass In (t/h)".into(),
            "Mass Out (t/h)".into(),
            "MB Error (%)".into(),
            "H In (kW)".into(),
            "H Out (kW)".into(),
            "Heat Diff (kW)".into(),
            "Active Controls".into(),
        ],
    ];

    let mut total_sys_in = 0.0_f64;
    let mut total_sys_out = 0.0_f64;
    for s in &project.streams {
        match s.connectivity_kind() {
            StreamConnectivity::Input => total_sys_in += s.mass_flow_t_h,
            StreamConnectivity::Output => total_sys_out += s.mass_flow_t_h,
            StreamConnectivity::Internal => {}
        }
    }

    for u in &project.units {
        let (ins, outs) = unit_streams(project, u);
        let in_names = if ins.is_empty() {
            "-".to_string()
        } else {
            ins.iter().map(|s| s.name_id.as_str()).collect::<Vec<_>>().join(", ")
        };
        let out_names = if outs.is_empty() {
            "-".to_string()
        } else {
            outs.iter().map(|s| s.name_id.as_str()).collect::<Vec<_>>().join(", ")
        };
        let m_in: f64 = ins.iter().map(|s| s.mass_flow_t_h).sum();
        let m_out: f64 = outs.iter().map(|s| s.mass_flow_t_h).sum();
        let h_in: f64 = ins.iter().map(|s| s.enthalpy_kw).sum();
        let h_out: f64 = outs.iter().map(|s| s.enthalpy_kw).sum();
        let mb_err = if m_in.abs() > 1e-9 {
            ((m_out - m_in) / m_in).abs() * 100.0
        } else if m_out.abs() > 1e-9 {
            100.0
        } else {
            0.0
        };
        let ctrl_count = project
            .controls
            .iter()
            .filter(|c| c.active && c.location_unit.eq_ignore_ascii_case(&u.name_id))
            .count();

        ub_rows.push(vec![
            u.name_id.clone(),
            u.model_mode.clone(),
            u.model_type.as_str().to_string(),
            u.model_name.clone(),
            fmt2(u.availability_pct),
            in_names,
            out_names,
            fmt2(m_in),
            fmt2(m_out),
            fmt3(mb_err),
            fmt2(h_in),
            fmt2(h_out),
            fmt2(h_out - h_in),
            ctrl_count.to_string(),
        ]);
    }

    let sys_mb = if total_sys_in.abs() > 1e-9 {
        ((total_sys_out - total_sys_in) / total_sys_in).abs() * 100.0
    } else {
        0.0
    };
    ub_rows.push(vec![
        "GLOBAL FLOWSHEET BOUNDARY".into(),
        "-".into(),
        "-".into(),
        "-".into(),
        "100.00".into(),
        format!("{} Input streams", project.streams.iter().filter(|s| s.source_unit.is_none()).count()),
        format!("{} Output streams", project.streams.iter().filter(|s| s.dest_unit.is_none()).count()),
        fmt2(total_sys_in),
        fmt2(total_sys_out),
        fmt3(sys_mb),
        "-".into(),
        "-".into(),
        "-".into(),
        project.controls.iter().filter(|c| c.active).count().to_string(),
    ]);

    // ------------------------------------------------------------------
    // Sheet 2: Stream Balance - Hydro (transposed matrix: property rows x stream cols, Fig. 25)
    // ------------------------------------------------------------------
    let mut hydro_rows: Vec<Vec<String>> = Vec::new();
    let mut header_stream = vec!["Property".into(), "Unit".into()];
    let mut row_from = vec!["From Unit".into(), "-".into()];
    let mut row_to = vec!["To Unit".into(), "-".into()];
    let mut row_conn = vec!["Connectivity".into(), "-".into()];

    for s in &project.streams {
        header_stream.push(s.name_id.clone());
        row_from.push(unit_name_opt(project, s.source_unit));
        row_to.push(unit_name_opt(project, s.dest_unit));
        row_conn.push(connectivity_label(s).to_string());
    }
    hydro_rows.push(header_stream);
    hydro_rows.push(row_from);
    hydro_rows.push(row_to);
    hydro_rows.push(row_conn);

    let hydro_metrics: &[&str] = &[
        "Total Mass Flow",
        "Gas Flow",
        "Liquid Flow",
        "Solids Flow",
        "Temperature",
        "Pressure",
        "Solids Weight Percent",
        "Pulp Volume Flow",
        "Solids Specific Gravity",
        "Liquid Specific Gravity",
        "Pulp Specific Gravity",
        "Total Enthalpy",
        "Thermal Energy",
        "Total Exergy",
        "Cu wt-%",
        "Fe wt-%",
        "S wt-%",
        "Cu Rec %",
        "Ccp Rec %",
        "P80",
        "P50",
    ];
    for metric in hydro_metrics {
        let mut r = Vec::with_capacity(project.streams.len() + 2);
        let unit_lbl = project
            .streams
            .first()
            .map(|s| metric_value_and_unit(s, metric).1)
            .unwrap_or("");
        r.push((*metric).to_string());
        r.push(unit_lbl.to_string());
        for s in &project.streams {
            let (val, _) = metric_value_and_unit(s, metric);
            r.push(fmt2(val));
        }
        hydro_rows.push(r);
    }

    // Species sub-table in Hydro Balance
    let mut all_species: Vec<String> = Vec::new();
    for s in &project.streams {
        for sp in &s.species_flows {
            if !all_species.iter().any(|x| x == &sp.formula) {
                all_species.push(sp.formula.clone());
            }
        }
    }
    if !all_species.is_empty() {
        let mut sec = vec!["Species Breakdown (t/h)".into(), "t/h".into()];
        sec.extend(project.streams.iter().map(|_| String::new()));
        hydro_rows.push(sec);

        for sp_name in all_species {
            let mut row = vec![sp_name.clone(), "t/h".into()];
            for s in &project.streams {
                let flow = s
                    .species_flows
                    .iter()
                    .find(|x| x.formula == sp_name)
                    .map(|x| x.mass_flow_t_h)
                    .unwrap_or(0.0);
                row.push(fmt2(flow));
            }
            hydro_rows.push(row);
        }
    }

    // ------------------------------------------------------------------
    // Sheet 3: Streams - Properties (row per stream, Fig. 26)
    // ------------------------------------------------------------------
    let mut prop_rows = vec![vec![
        "Stream Name".into(),
        "Alias".into(),
        "From".into(),
        "To".into(),
        "Type".into(),
        "T (°C)".into(),
        "P (bar)".into(),
        "Mass (t/h)".into(),
        "Gas (Nm3/h)".into(),
        "Liquid (t/h)".into(),
        "Solids (t/h)".into(),
        "Solids %".into(),
        "Pulp Vol (m3/h)".into(),
        "Cu (%)".into(),
        "Fe (%)".into(),
        "S (%)".into(),
        "Cu Rec (%)".into(),
        "P80 (µm)".into(),
        "H (kW)".into(),
        "Exergy (kW)".into(),
    ]];

    for s in &project.streams {
        prop_rows.push(vec![
            s.name_id.clone(),
            s.alias.clone(),
            unit_name_opt(project, s.source_unit),
            unit_name_opt(project, s.dest_unit),
            connectivity_label(s).to_string(),
            fmt2(s.temp_c),
            fmt2(s.pressure_bar),
            fmt2(s.mass_flow_t_h),
            fmt2(s.gas_flow_nm3_h),
            fmt2(s.liquid_flow_t_h),
            fmt2(s.solids_flow_t_h),
            fmt2(s.pct_solids),
            fmt2(s.pulp_vol_m3_h),
            fmt2(s.cu_wt_pct),
            fmt2(s.fe_wt_pct),
            fmt2(s.s_wt_pct),
            fmt2(s.cu_rec_pct),
            fmt2(s.p80_um),
            fmt2(s.enthalpy_kw),
            fmt2(s.exergy_kw),
        ]);
    }

    // ------------------------------------------------------------------
    // Sheet 4: Flowsheet Tables (Stream Table Sets + Canvas Tables)
    // ------------------------------------------------------------------
    let mut ft_rows = vec![vec![
        "Table / Stream Set".into(),
        "Mode / Object".into(),
        "Item / Stream".into(),
        "Variable / Col 1".into(),
        "Value / Col 2".into(),
        "Unit / Col 3".into(),
    ]];

    for set in &project.stream_table_sets {
        for s_name in &set.assigned_streams {
            if let Some(stream) = project
                .streams
                .iter()
                .find(|s| s.name_id.eq_ignore_ascii_case(s_name))
            {
                for var in &set.variables {
                    let (v, u) = metric_value_and_unit(stream, var);
                    ft_rows.push(vec![
                        set.name.clone(),
                        set.view_mode.as_str().to_string(),
                        stream.name_id.clone(),
                        var.clone(),
                        fmt2(v),
                        u.to_string(),
                    ]);
                }
            }
        }
    }

    for tbl in &project.tables {
        let kind_lbl = if tbl.is_title_table {
            "Title Table"
        } else {
            "Flowsheet Table"
        };
        if tbl.is_title_table {
            ft_rows.push(vec![
                format!("Table #{}", tbl.id),
                kind_lbl.into(),
                "Title".into(),
                tbl.title.clone(),
                tbl.organization_author.clone(),
                format!("{} | {}", tbl.hsc_version, tbl.date),
            ]);
        } else {
            for (r_idx, row) in tbl.cells.iter().enumerate() {
                ft_rows.push(vec![
                    format!("Table #{}", tbl.id),
                    kind_lbl.into(),
                    format!("Row {}", r_idx + 1),
                    row.first().cloned().unwrap_or_default(),
                    row.get(1).cloned().unwrap_or_default(),
                    row.get(2).cloned().unwrap_or_default(),
                ]);
            }
        }
    }

    SimReportWorkbook {
        sheets: vec![
            SimReportSheet {
                name: "Unit Balance - General".into(),
                rows: ub_rows,
                freeze_rows: 2,
                freeze_cols: 1,
            },
            SimReportSheet {
                name: "Stream Balance - Hydro".into(),
                rows: hydro_rows,
                freeze_rows: 4,
                freeze_cols: 2,
            },
            SimReportSheet {
                name: "Streams - Properties".into(),
                rows: prop_rows,
                freeze_rows: 1,
                freeze_cols: 1,
            },
            SimReportSheet {
                name: "Flowsheet Tables".into(),
                rows: ft_rows,
                freeze_rows: 1,
                freeze_cols: 2,
            },
        ],
    }
}

pub fn report_sheet_to_csv(sheet: &SimReportSheet) -> String {
    let mut out = String::new();
    for row in &sheet.rows {
        let escaped: Vec<String> = row
            .iter()
            .map(|c| {
                if c.contains(',') || c.contains('"') || c.contains('\n') {
                    format!("\"{}\"", c.replace('"', "\"\""))
                } else {
                    c.clone()
                }
            })
            .collect();
        out.push_str(&escaped.join(","));
        out.push('\n');
    }
    out
}
