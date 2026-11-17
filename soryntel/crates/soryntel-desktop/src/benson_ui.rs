use eframe::egui;
use soryntel_application::SoryntelApplication;
use soryntel_core::CAL_TO_JOULE;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::{
    benson::{
        find_structural_block, structural_blocks_for, BensonEstimationResult, BensonParamsInput,
        BlockCategory, BondOrder, ElementKind, MoleculeGraph, PhaseThermoSummary,
    },
    Phase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraMode {
    Perspective,
    Orthographic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CanvasBackground {
    DarkRadial,
    SkyBlue,
    DarkSlate,
    LightStudio,
}

impl CanvasBackground {
    fn label(self) -> &'static str {
        match self {
            Self::DarkRadial => "Dark Radial (Default)",
            Self::SkyBlue => "Sky Blue (Space-Filling)",
            Self::DarkSlate => "Dark Slate",
            Self::LightStudio => "Light Studio",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultsEnergyUnit {
    Joules,
    Calories,
}

impl ResultsEnergyUnit {
    fn label(self) -> &'static str {
        match self {
            Self::Joules => "Joules",
            Self::Calories => "Calories",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Joules => "Estimation Results in joules",
            Self::Calories => "Estimation Results in calories",
        }
    }

    fn dh_header(self) -> &'static str {
        match self {
            Self::Joules => "ΔHf⁰ in kJ/mol",
            Self::Calories => "ΔHf⁰ in kcal/mol",
        }
    }

    fn s_header(self) -> &'static str {
        match self {
            Self::Joules => "S⁰ in J/mol×K",
            Self::Calories => "S⁰ in cal/mol×K",
        }
    }

    fn cp_header(self) -> &'static str {
        match self {
            Self::Joules => "Cp⁰ in J/mol×K",
            Self::Calories => "Cp⁰ in cal/mol×K",
        }
    }
}

pub struct BensonModuleState {
    molecule: MoleculeGraph,
    undo_stack: Vec<MoleculeGraph>,
    selected_element: ElementKind,
    preview_group_mol: MoleculeGraph,
    /// Visualization controls
    atom_size: f32,
    opacity: f32,
    camera_mode: CameraMode,
    background: CanvasBackground,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: egui::Vec2,
    preview_yaw: f32,
    /// Optional SMILES quick entry
    smiles_input: String,
    show_smiles_modal: bool,
    /// Estimation & Results modal state (Fig. 10 / 11)
    estimation: Option<BensonEstimationResult>,
    show_results_window: bool,
    results_unit: ResultsEnergyUnit,
    selected_save_phase: Phase,
    selected_base_ref: String,
    edit_formula: String,
    edit_common_name: String,
    edit_chemical_name: String,
    benson_params: BensonParamsInput,
    /// Dialogs & status
    show_help: bool,
    show_about: bool,
    status_text: String,
    progress_pct: f32,
    last_error: Option<String>,
}

impl BensonModuleState {
    pub fn new(db: &ThermoDatabase) -> Self {
        // Default starter molecule: 9,10-Anthracenedimethanol (C16H14O2, MW 238.272) like Fig. 1
        let starter_spec = find_structural_block("aro_diol_ant")
            .unwrap_or_else(|| find_structural_block("aro_benzene").unwrap());
        let molecule = MoleculeGraph::from_block(starter_spec);
        let preview_group_mol = build_selected_group_preview(ElementKind::O);
        let smiles_input = molecule.smiles.clone();
        let mut st = Self {
            molecule,
            undo_stack: Vec::new(),
            selected_element: ElementKind::O,
            preview_group_mol,
            atom_size: 1.00,
            opacity: 1.00,
            camera_mode: CameraMode::Perspective,
            background: CanvasBackground::DarkRadial,
            yaw: 0.42,
            pitch: -0.35,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            preview_yaw: 0.45,
            smiles_input,
            show_smiles_modal: false,
            estimation: None,
            show_results_window: false,
            results_unit: ResultsEnergyUnit::Joules,
            selected_save_phase: Phase::Solid,
            selected_base_ref: "No base reference".to_string(),
            edit_formula: String::new(),
            edit_common_name: String::new(),
            edit_chemical_name: String::new(),
            benson_params: BensonParamsInput::default(),
            show_help: false,
            show_about: false,
            status_text: "Ready · Click any H atom to attach the selected element, or drag to rotate 3D.".to_string(),
            progress_pct: 0.0,
            last_error: None,
        };
        st.recompute_estimation(db, false);
        st
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.molecule.clone());
        if self.undo_stack.len() > 30 {
            self.undo_stack.remove(0);
        }
    }

    fn load_molecule(&mut self, mol: MoleculeGraph, db: &ThermoDatabase) {
        self.push_undo();
        self.molecule = mol;
        self.smiles_input = self.molecule.smiles.clone();
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
        self.last_error = None;
        self.recompute_estimation(db, false);
    }

    fn recompute_estimation(&mut self, db: &ThermoDatabase, open_results: bool) {
        if self.molecule.atoms.is_empty() {
            self.estimation = None;
            self.progress_pct = 0.0;
            self.status_text = "Workspace empty — use 'Add structural block' or 'File -> Enter SMILES'.".into();
            return;
        }
        let (_, auto_params) = soryntel_thermo::benson::decompose_molecule_to_benson(&self.molecule);
        self.benson_params = auto_params.clone();
        match SoryntelApplication::estimate_benson_properties(db, &self.molecule, None, Some(&auto_params)) {
            Ok(est) => {
                self.edit_formula = est.formula.clone();
                self.edit_common_name = est.common_name.clone();
                self.edit_chemical_name = est.chemical_name.clone();
                self.status_text = format!(
                    "Ready · {} ({:.3} g/mol) · {} Benson groups",
                    est.formula,
                    est.molecular_weight,
                    est.groups.len()
                );
                self.progress_pct = if open_results { 1.0 } else { 0.0 };
                self.last_error = None;
                self.estimation = Some(est);
                if open_results {
                    self.show_results_window = true;
                }
            }
            Err(e) => {
                self.last_error = Some(e.clone());
                self.status_text = format!("Estimation warning: {e}");
                self.progress_pct = 0.0;
            }
        }
    }

    fn recalculate_with_current_params(&mut self, db: &ThermoDatabase) {
        match SoryntelApplication::estimate_benson_properties(
            db,
            &self.molecule,
            None,
            Some(&self.benson_params),
        ) {
            Ok(est) => {
                self.estimation = Some(est);
                self.progress_pct = 1.0;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(e);
            }
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        // Gentle idle spin on the small Selected group preview viewport
        self.preview_yaw += ctx.input(|i| i.stable_dt).min(0.05) * 0.45;
        ctx.request_repaint_after(std::time::Duration::from_millis(60));

        // 1. Top Ribbon Bar (HSC-BEN layout matching Fig. 1, 6, 7, 8)
        egui::TopBottomPanel::top("benson_ribbon_top")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(245, 247, 250))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 212, 222)))
                    .inner_margin(egui::Margin::symmetric(8.0, 5.0)),
            )
            .show(ctx, |ui| {
                self.render_top_ribbon(ui, db);
            });

        // 2. Bottom Status Bar (Outotec/Soryntel + Status + Progress Bar)
        egui::TopBottomPanel::bottom("benson_status_bottom")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(242, 244, 248))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(208, 214, 224)))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Outotec")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(198, 40, 40)),
                    );
                    ui.separator();
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let pct_bar = egui::ProgressBar::new(self.progress_pct)
                            .desired_width(210.0)
                            .text(format!("{:.0}%", self.progress_pct * 100.0));
                        ui.add(pct_bar);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(&self.status_text)
                                        .size(11.5)
                                        .color(if self.last_error.is_some() {
                                            egui::Color32::from_rgb(185, 28, 28)
                                        } else {
                                            egui::Color32::from_rgb(55, 65, 81)
                                        }),
                                )
                                .truncate(),
                            );
                        });
                    });
                });
            });

        // 3. Right Sidebar (`Please select` Palette + `Selected group` 3D Preview + Formula/MW)
        egui::SidePanel::right("benson_right_palette")
            .resizable(true)
            .default_width(190.0)
            .min_width(178.0)
            .max_width(290.0)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(243, 245, 248))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 212, 222)))
                    .inner_margin(egui::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                self.render_right_palette(ui, db);
            });

        // 4. Central 3D Interactive Molecule Workspace
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(egui::Color32::from_rgb(24, 26, 30)))
            .show(ctx, |ui| {
                self.render_3d_workspace(ui, db);
            });

        // 5. Floating Windows (Results Table Fig. 10/11, SMILES Input, Help, About)
        if self.show_results_window {
            self.render_results_window(ctx, db);
        }
        if self.show_smiles_modal {
            self.render_smiles_modal(ctx, db);
        }
        if self.show_help {
            self.render_help_window(ctx);
        }
        if self.show_about {
            self.render_about_window(ctx);
        }
    }

    fn render_top_ribbon(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal(|ui| {
                // --- Group 1: Menu ---
                ribbon_group(ui, "Menu", |ui| {
                    ui.menu_button("📄\nFile ▾", |ui| {
                        if ui.button("New / Clear Workspace").clicked() {
                            self.push_undo();
                            self.molecule = MoleculeGraph::empty();
                            self.estimation = None;
                            self.status_text = "Workspace cleared.".into();
                            ui.close_menu();
                        }
                        if ui.button("Enter / Edit SMILES...").clicked() {
                            self.show_smiles_modal = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Load Showcase Preset", |ui| {
                            let presets = [
                                ("aro_4ma", "4-Methylaniline (C7H9N · Fig. 10)"),
                                ("aro_benzene", "Benzene (C6H6 · Fig. 6)"),
                                ("aro_toluene", "Toluene (C7H8 · Fig. 7)"),
                                ("aro_diol_ant", "Anthracenedimethanol (C16H14O2 · Fig. 1)"),
                                ("poly_coronene", "Coronene (C24H12 · Fig. 1-2)"),
                                ("cyc_8", "Cyclooctane (C8H16 · Fig. 3-4)"),
                                ("alk_butane", "n-Butane (C4H10)"),
                                ("alk_isobutane", "Isobutane (C4H10)"),
                                ("ali_ethanol", "Ethanol (C2H6O)"),
                                ("fg_cooh", "Acetic Acid (C2H4O2)"),
                                ("cyc_6", "Cyclohexane (C6H12)"),
                            ];
                            for (id, label) in presets {
                                if ui.button(label).clicked() {
                                    if let Some(spec) = find_structural_block(id) {
                                        self.load_molecule(MoleculeGraph::from_block(spec), db);
                                    }
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.separator();
                        if ui.button("Export Report (CSV)...").clicked() {
                            self.export_csv_report();
                            ui.close_menu();
                        }
                    });
                });

                ribbon_sep(ui);

                // --- Group 2: Actions (`Add structural block`, `Undo last`, `Calculate`) ---
                ribbon_group(ui, "Actions", |ui| {
                    ui.menu_button("⊕\nAdd structural\nblock ▾", |ui| {
                        for cat in BlockCategory::ALL {
                            ui.menu_button(cat.label(), |ui| {
                                for spec in structural_blocks_for(cat) {
                                    if ui.button(spec.name).clicked() {
                                        if let Some(attach_el) = spec.functional_attach {
                                            // Select the corresponding element for interactive H-click,
                                            // OR if workspace empty load it as starter molecule.
                                            if self.molecule.atoms.is_empty() {
                                                self.load_molecule(MoleculeGraph::from_block(spec), db);
                                            } else {
                                                self.selected_element = attach_el;
                                                self.preview_group_mol = build_selected_group_preview(attach_el);
                                                self.status_text = format!(
                                                    "Selected '{}' — click an H atom on the molecule to attach.",
                                                    spec.name
                                                );
                                            }
                                        } else {
                                            let mol = MoleculeGraph::from_block(spec);
                                            self.preview_group_mol = mol.clone();
                                            self.load_molecule(mol, db);
                                        }
                                        ui.close_menu();
                                    }
                                }
                            });
                        }
                    });

                    let undo_btn = egui::Button::new(
                        egui::RichText::new("↶\nUndo last")
                            .size(11.5)
                            .color(if self.undo_stack.is_empty() {
                                egui::Color32::GRAY
                            } else {
                                egui::Color32::from_rgb(22, 128, 56)
                            }),
                    );
                    if ui.add_enabled(!self.undo_stack.is_empty(), undo_btn).clicked() {
                        if let Some(prev) = self.undo_stack.pop() {
                            self.molecule = prev;
                            self.smiles_input = self.molecule.smiles.clone();
                            self.recompute_estimation(db, false);
                            self.status_text = "Reverted last edit.".into();
                        }
                    }

                    let calc_btn = egui::Button::new(
                        egui::RichText::new("➜\nCalculate")
                            .strong()
                            .size(11.5)
                            .color(egui::Color32::from_rgb(21, 128, 61)),
                    );
                    if ui.add(calc_btn).clicked() {
                        self.recompute_estimation(db, true);
                    }
                });

                ribbon_sep(ui);

                // --- Group 3: Molecule Visualization (`Atom size`, `Opacity`) ---
                ribbon_group(ui, "Molecule Visualization", |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [92.0, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(format!("Atom size {:>4.2}", self.atom_size))
                                        .size(11.5),
                                ),
                            );
                            ui.spacing_mut().slider_width = 110.0;
                            ui.add(
                                egui::Slider::new(&mut self.atom_size, 0.40..=4.00)
                                    .show_value(false)
                                    .fixed_decimals(2),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [92.0, 20.0],
                                egui::Label::new(
                                    egui::RichText::new(format!("Opacity {:>3.0}%", self.opacity * 100.0))
                                        .size(11.5),
                                ),
                            );
                            ui.spacing_mut().slider_width = 110.0;
                            ui.add(
                                egui::Slider::new(&mut self.opacity, 0.15..=1.00)
                                    .show_value(false),
                            );
                        });
                    });
                });

                ribbon_sep(ui);

                // --- Group 4: Virtual Camera (`Perspective`, `Orthographic`) ---
                ribbon_group(ui, "Virtual Camera", |ui| {
                    let persp_active = self.camera_mode == CameraMode::Perspective;
                    let ortho_active = self.camera_mode == CameraMode::Orthographic;
                    if ui
                        .selectable_label(persp_active, "👁\nPerspective")
                        .clicked()
                    {
                        self.camera_mode = CameraMode::Perspective;
                    }
                    if ui
                        .selectable_label(ortho_active, "◎\nOrthographic")
                        .clicked()
                    {
                        self.camera_mode = CameraMode::Orthographic;
                    }
                });

                ribbon_sep(ui);

                // --- Group 5: Screenshot ---
                ribbon_group(ui, "Screenshot", |ui| {
                    if ui.button("💾\nSave image\nas...").clicked() {
                        self.save_svg_snapshot();
                    }
                    if ui.button("📋\nCopy to\nclipboard").clicked() {
                        let summary = self.format_clipboard_text();
                        ui.ctx().copy_text(summary);
                        self.status_text = "Copied formula, SMILES & estimation summary to clipboard.".into();
                    }
                    ui.menu_button("▦\nModel position\nand background ▾", |ui| {
                        if ui.button("Reset Camera Position").clicked() {
                            self.yaw = 0.42;
                            self.pitch = -0.35;
                            self.zoom = 1.0;
                            self.pan = egui::Vec2::ZERO;
                            ui.close_menu();
                        }
                        if ui.button("Top View (XY Plane)").clicked() {
                            self.yaw = 0.0;
                            self.pitch = 0.0;
                            ui.close_menu();
                        }
                        if ui.button("Space-Filling Preset (3.60x Sky Blue · Fig. 4)").clicked() {
                            self.atom_size = 3.60;
                            self.opacity = 1.00;
                            self.background = CanvasBackground::SkyBlue;
                            ui.close_menu();
                        }
                        if ui.button("Ball-and-Stick Preset (1.00x Dark Radial)").clicked() {
                            self.atom_size = 1.00;
                            self.opacity = 1.00;
                            self.background = CanvasBackground::DarkRadial;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label("Background:");
                        for bg in [
                            CanvasBackground::DarkRadial,
                            CanvasBackground::SkyBlue,
                            CanvasBackground::DarkSlate,
                            CanvasBackground::LightStudio,
                        ] {
                            if ui.radio(self.background == bg, bg.label()).clicked() {
                                self.background = bg;
                                ui.close_menu();
                            }
                        }
                    });
                });

                ribbon_sep(ui);

                // --- Group 6: About ---
                ribbon_group(ui, "About", |ui| {
                    if ui.button("❓\nHelp").clicked() {
                        self.show_help = true;
                    }
                    if ui.button("ℹ\nAbout this\nmodule").clicked() {
                        self.show_about = true;
                    }
                });
            });
        });
    }

    fn render_right_palette(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let panel_w = ui.available_width();
            ui.add_sized(
                [panel_w, 18.0],
                egui::Label::new(
                    egui::RichText::new("Please select")
                        .size(13.0)
                        .color(egui::Color32::from_rgb(30, 41, 59)),
                ),
            );
            ui.add_space(4.0);

            // 4x3 Element Palette matching Fig. 1 / Fig. 7 / Fig. 8 (centered inside panel)
            let btn_w = 34.0;
            let gap = 4.0;
            let grid_w = 4.0 * btn_w + 3.0 * gap;
            let left_pad = ((panel_w - grid_w) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(left_pad);
                egui::Grid::new("benson_element_grid")
                    .spacing([gap, gap])
                    .show(ui, |ui| {
                        for (idx, &el) in ElementKind::PALETTE_ORDER.iter().enumerate() {
                            let is_selected = self.selected_element == el;
                            let (fill_col, txt_col) = element_badge_colors(el);
                            let (rect, resp) = ui.allocate_exact_size(
                                egui::vec2(btn_w, 32.0),
                                egui::Sense::click(),
                            );
                            let bg = if resp.hovered() {
                                egui::Color32::from_rgb(232, 238, 247)
                            } else {
                                egui::Color32::WHITE
                            };
                            let border = if is_selected {
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(210, 32, 32))
                            } else {
                                egui::Stroke::new(1.0, egui::Color32::from_rgb(186, 194, 206))
                            };
                            ui.painter().rect(rect, 3.0, bg, border);
                            let center = rect.center();
                            let rad = 11.5;
                            ui.painter().circle_filled(center, rad, fill_col);
                            ui.painter().circle_stroke(
                                center,
                                rad,
                                egui::Stroke::new(
                                    0.8,
                                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 75),
                                ),
                            );
                            ui.painter().text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                el.symbol(),
                                egui::FontId::proportional(11.5),
                                txt_col,
                            );
                            if resp.clicked() {
                                self.selected_element = el;
                                self.preview_group_mol = build_selected_group_preview(el);
                                self.status_text = format!(
                                    "Selected {} — click any H atom in the 3D view to attach.",
                                    el.symbol()
                                );
                            }
                            if (idx + 1) % 4 == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });

            ui.add_space(6.0);
            ui.add_sized(
                [panel_w, 16.0],
                egui::Label::new(
                    egui::RichText::new("Selected group")
                        .size(11.5)
                        .color(egui::Color32::from_rgb(51, 65, 85)),
                ),
            );

            // Mini 3D Preview Box for Selected Group (strictly clipped!)
            let (prev_rect, _) = ui.allocate_exact_size(
                egui::vec2(panel_w.max(120.0), 108.0),
                egui::Sense::hover(),
            );
            paint_radial_dark_bg(ui.painter(), prev_rect, CanvasBackground::DarkRadial);
            render_molecule_3d_painter(
                ui.painter(),
                prev_rect,
                &self.preview_group_mol,
                self.preview_yaw,
                -0.35,
                1.0,
                egui::Vec2::ZERO,
                1.0,
                1.0,
                CameraMode::Perspective,
                None,
            );
            ui.painter().rect_stroke(
                prev_rect,
                2.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(148, 163, 184)),
            );

            ui.add_space(10.0);
            let formula = self.molecule.molecular_formula();
            let mw = self.molecule.molecular_weight();
            ui.add_sized(
                [panel_w, 16.0],
                egui::Label::new(
                    egui::RichText::new("Molecular formula:")
                        .size(11.5)
                        .color(egui::Color32::from_rgb(51, 65, 85)),
                ),
            );
            ui.add_sized(
                [panel_w, 22.0],
                egui::Label::new(
                    egui::RichText::new(if formula.is_empty() { "-" } else { &formula })
                        .strong()
                        .size(15.0)
                        .color(egui::Color32::from_rgb(15, 23, 42)),
                ),
            );
            ui.add_space(6.0);
            ui.add_sized(
                [panel_w, 16.0],
                egui::Label::new(
                    egui::RichText::new("Molecular weight")
                        .size(11.5)
                        .color(egui::Color32::from_rgb(51, 65, 85)),
                ),
            );
            ui.add_sized(
                [panel_w, 22.0],
                egui::Label::new(
                    egui::RichText::new(format!("{mw:.3}"))
                        .strong()
                        .size(15.0)
                        .color(egui::Color32::from_rgb(15, 23, 42)),
                ),
            );

            ui.add_space(10.0);
            ui.separator();
            ui.label(
                egui::RichText::new("Quick SMILES:")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(71, 85, 105)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let go_clicked = ui.small_button("Go").clicked();
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.smiles_input)
                        .desired_width(ui.available_width())
                        .hint_text("Cc1ccc(N)cc1"),
                );
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if go_clicked || enter {
                    match SoryntelApplication::parse_benson_smiles(&self.smiles_input) {
                        Ok(mol) => self.load_molecule(mol, db),
                        Err(e) => {
                            self.last_error = Some(e.clone());
                            self.status_text = format!("SMILES error: {e}");
                        }
                    }
                }
            });
            ui.add_space(4.0);
            ui.add_sized(
                [panel_w, 24.0],
                egui::Button::new("Inspect GAV & Results"),
            )
            .clicked()
            .then(|| self.recompute_estimation(db, true));
        });
    }

    fn render_3d_workspace(&mut self, ui: &mut egui::Ui, db: &mut ThermoDatabase) {
        let avail = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(avail, egui::Sense::click_and_drag());

        // Background
        paint_radial_dark_bg(ui.painter(), avail, self.background);

        // Orbit & Zoom & Pan controls
        if response.dragged_by(egui::PointerButton::Primary) {
            let delta = response.drag_delta();
            self.yaw += delta.x * 0.011;
            self.pitch = (self.pitch + delta.y * 0.011).clamp(-1.55, 1.55);
        }
        if response.dragged_by(egui::PointerButton::Secondary) {
            self.pan += response.drag_delta();
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.1 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.0015)).clamp(0.35, 3.8);
            }
        }

        // Hit-testing projected atoms so hovering & clicking an atom substitutes/attaches
        let pointer_pos = response.hover_pos();
        let projected = render_molecule_3d_painter(
            ui.painter(),
            avail,
            &self.molecule,
            self.yaw,
            self.pitch,
            self.zoom,
            self.pan,
            self.atom_size,
            self.opacity,
            self.camera_mode,
            pointer_pos,
        );

        // Directional white arrow buttons around canvas perimeter (Fig. 1, 6, 7, 8)
        let arrow_color = egui::Color32::from_rgba_unmultiplied(245, 247, 250, 215);
        let arrow_specs = [
            ("▲", egui::pos2(avail.center().x, avail.top() + 14.0), (0.0_f32, -0.18_f32)),
            ("▼", egui::pos2(avail.center().x, avail.bottom() - 14.0), (0.0, 0.18)),
            ("◀", egui::pos2(avail.left() + 14.0, avail.center().y), (-0.18, 0.0)),
            ("▶", egui::pos2(avail.right() - 14.0, avail.center().y), (0.18, 0.0)),
        ];
        let mut arrow_hovered_or_clicked = false;
        for (glyph, pos, (dyaw, dpitch)) in arrow_specs {
            let r = egui::Rect::from_center_size(pos, egui::vec2(22.0, 22.0));
            let resp = ui.interact(r, ui.id().with(glyph), egui::Sense::click());
            let col = if resp.hovered() {
                arrow_hovered_or_clicked = true;
                egui::Color32::WHITE
            } else {
                arrow_color
            };
            ui.painter()
                .text(pos, egui::Align2::CENTER_CENTER, glyph, egui::FontId::proportional(13.0), col);
            if resp.clicked() {
                arrow_hovered_or_clicked = true;
                self.yaw += dyaw;
                self.pitch = (self.pitch + dpitch).clamp(-1.55, 1.55);
            }
        }

        // Hover tooltip & click-to-substitute H atom (Fig. 6 -> Fig. 7 -> Fig. 8)
        if !arrow_hovered_or_clicked {
            if let Some(atom_idx) = projected.hovered_atom {
                let el = self.molecule.atoms[atom_idx].element;
                response.clone().on_hover_text(format!(
                    "{} atom #{} — click to attach {}",
                    el.symbol(),
                    atom_idx + 1,
                    self.selected_element.symbol()
                ));
                if response.clicked() {
                    self.push_undo();
                    match self
                        .molecule
                        .substitute_atom_with_element(atom_idx, self.selected_element)
                    {
                        Ok(()) => {
                            self.recompute_estimation(db, false);
                        }
                        Err(e) => {
                            let _ = self.undo_stack.pop();
                            self.last_error = Some(e.clone());
                            self.status_text = e;
                        }
                    }
                }
            }
        }
    }

    fn render_results_window(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        let mut open = self.show_results_window;
        let title = self.results_unit.title();
        let Some(est) = self.estimation.clone() else {
            self.show_results_window = false;
            return;
        };

        egui::Window::new(title)
            .id(egui::Id::new("benson_estimation_results_modal"))
            .open(&mut open)
            .resizable(true)
            .default_width(640.0)
            .min_width(420.0)
            .show(ctx, |ui| {
                // Header metadata (Formula, Common name, Chemical name — non-overlapping layout)
                egui::Grid::new("benson_results_meta_grid")
                    .num_columns(4)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Formula");
                        ui.add(egui::TextEdit::singleline(&mut self.edit_formula).desired_width(110.0));
                        ui.label("Common name");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.edit_common_name)
                                .desired_width(250.0),
                        );
                        ui.end_row();
                    });
                ui.horizontal(|ui| {
                    ui.label("Chemical name");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.edit_chemical_name)
                            .desired_width(ui.available_width().max(180.0)),
                    );
                });

                ui.add_space(6.0);

                // 6-row Phase Estimation Table (Gas/Liquid/Solid est & db)
                let to_dh = |j: f64| match self.results_unit {
                    ResultsEnergyUnit::Joules => j / 1000.0,
                    ResultsEnergyUnit::Calories => j / (CAL_TO_JOULE * 1000.0),
                };
                let to_s_cp = |jk: f64| match self.results_unit {
                    ResultsEnergyUnit::Joules => jk,
                    ResultsEnergyUnit::Calories => jk / CAL_TO_JOULE,
                };

                egui::Frame::none()
                    .fill(egui::Color32::WHITE)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(185, 193, 206)))
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal().show(ui, |ui| {
                            egui::Grid::new("benson_results_phase_table")
                                .striped(false)
                                .min_col_width(126.0)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                ui.strong("Phase,Source/Property");
                                ui.strong(self.results_unit.dh_header());
                                ui.strong(self.results_unit.s_header());
                                ui.strong(self.results_unit.cp_header());
                                ui.end_row();

                                let rows: [(&str, Option<Phase>, Option<PhaseThermoSummary>); 6] = [
                                    ("Gas, est.", Some(Phase::Gas), Some(est.gas_est)),
                                    ("Gas, db.", None, est.gas_db),
                                    ("Liquid, est.", Some(Phase::Liquid), Some(est.liquid_est)),
                                    ("Liquid, db.", None, est.liquid_db),
                                    ("Solid, est.", Some(Phase::Solid), Some(est.solid_est)),
                                    ("Solid, db.", None, est.solid_db),
                                ];

                                for (label, selectable_phase, vals) in rows {
                                    let is_selected = selectable_phase == Some(self.selected_save_phase);
                                    let hl_bg = if is_selected {
                                        egui::Color32::from_rgb(186, 242, 248) // Cyan highlight matching Fig. 11
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    };
                                    let txt_col = if label.ends_with("db.") {
                                        egui::Color32::from_rgb(51, 65, 85)
                                    } else {
                                        egui::Color32::from_rgb(15, 23, 42)
                                    };

                                    if selectable_phase.is_some() {
                                        let resp = ui.selectable_label(
                                            is_selected,
                                            egui::RichText::new(label).color(txt_col),
                                        );
                                        if resp.clicked() {
                                            if let Some(p) = selectable_phase {
                                                self.selected_save_phase = p;
                                            }
                                        }
                                    } else {
                                        ui.label(egui::RichText::new(label).color(txt_col));
                                    }

                                    let cell = |ui: &mut egui::Ui, val: Option<f64>| {
                                        let s = val.map(|v| format!("{v:.2}")).unwrap_or_default();
                                        egui::Frame::none().fill(hl_bg).show(ui, |ui| {
                                            ui.label(egui::RichText::new(s).monospace().color(txt_col));
                                        });
                                    };
                                    cell(ui, vals.map(|v| to_dh(v.delta_h_f_j)));
                                    cell(ui, vals.map(|v| to_s_cp(v.s_j_k)));
                                    cell(ui, vals.map(|v| to_s_cp(v.cp_j_k)));
                                    ui.end_row();
                                }
                            });
                        });
                    });

                ui.add_space(6.0);

                // Expandable Benson GAV Breakdown, Symmetry / Strain Parameters & Kelley Fit
                ui.collapsing("Benson Groups, Symmetry / Ring Corrections & Kelley Cp(T)", |ui| {
                    let mut changed = false;
                    ui.horizontal_wrapped(|ui| {
                        ui.label("σ_tot:");
                        changed |= ui.add(egui::DragValue::new(&mut self.benson_params.sigma_tot).range(1.0..=144.0).speed(0.5)).changed();
                        ui.label("n_opt:");
                        changed |= ui.add(egui::DragValue::new(&mut self.benson_params.n_opt).range(1..=16)).changed();
                        ui.label("Gauche:");
                        changed |= ui.add(egui::DragValue::new(&mut self.benson_params.gauche_count).range(0..=12)).changed();
                        ui.label("Cis:");
                        changed |= ui.add(egui::DragValue::new(&mut self.benson_params.cis_count).range(0..=8)).changed();
                        ui.label("Ring strain (kcal/mol):");
                        changed |= ui.add(egui::DragValue::new(&mut self.benson_params.ring_strain_kcal).speed(0.2)).changed();
                    });
                    if changed {
                        self.recalculate_with_current_params(db);
                    }
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(145.0).show(ui, |ui| {
                        egui::Grid::new("benson_groups_detail_grid")
                            .striped(true)
                            .spacing([12.0, 2.0])
                            .show(ui, |ui| {
                                ui.strong("Benson Group");
                                ui.strong("Count");
                                ui.strong("ΔH (kcal/mol)");
                                ui.strong("S (cal/mol·K)");
                                ui.strong("Cp300 (cal/mol·K)");
                                ui.end_row();
                                for g in &est.groups {
                                    ui.monospace(&g.group_name);
                                    ui.label(format!("{}", g.count));
                                    ui.label(format!("{:.2}", g.dh_kcal_each * g.count as f64));
                                    ui.label(format!("{:.2}", g.s_cal_each * g.count as f64));
                                    ui.label(format!("{:.2}", g.cp_cal_300_each * g.count as f64));
                                    ui.end_row();
                                }
                            });
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "Kelley Cp(T) [J/(mol·K)]: A={:.3}, B={:.3}, C={:.3}, D={:.3}  |  Est. Tm={:.1} K, Tb={:.1} K",
                            est.gas_kelley.a, est.gas_kelley.b, est.gas_kelley.c, est.gas_kelley.d,
                            est.tm_est_k, est.tb_est_k
                        ))
                        .small()
                        .monospace(),
                    );
                    for w in &est.warnings {
                        ui.label(egui::RichText::new(w).small().color(egui::Color32::from_rgb(180, 83, 9)));
                    }
                });

                ui.add_space(6.0);

                // Bottom toolbar matching Fig. 10 & Fig. 11
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Save to own database").clicked() {
                        let sp = SoryntelApplication::save_benson_species(
                            db,
                            &est,
                            self.selected_save_phase,
                            &self.edit_formula,
                        );
                        self.status_text = format!(
                            "Saved '{}' ({:?}) to database ({} species total).",
                            sp.formula,
                            sp.phase,
                            SoryntelApplication::species_count(db)
                        );
                    }

                    egui::ComboBox::from_id_salt("benson_base_ref_combo")
                        .selected_text(&self.selected_base_ref)
                        .width(155.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_base_ref,
                                "No base reference".to_string(),
                                "No base reference",
                            );
                            for m in &est.db_matches {
                                ui.selectable_value(&mut self.selected_base_ref, m.clone(), m);
                            }
                        });

                    if ui.button("Copy to clipboard").clicked() {
                        ctx.copy_text(self.format_clipboard_text());
                        self.status_text = "Copied estimation results table to clipboard.".into();
                    }

                    egui::ComboBox::from_id_salt("benson_unit_combo")
                        .selected_text(self.results_unit.label())
                        .width(85.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.results_unit,
                                ResultsEnergyUnit::Joules,
                                "Joules",
                            );
                            ui.selectable_value(
                                &mut self.results_unit,
                                ResultsEnergyUnit::Calories,
                                "Calories",
                            );
                        });

                    if ui.button(crate::i18n::text("common.004")).clicked() {
                        self.show_results_window = false;
                    }
                });
            });

        if !open {
            self.show_results_window = false;
        }
    }

    fn render_smiles_modal(&mut self, ctx: &egui::Context, db: &mut ThermoDatabase) {
        let mut open = self.show_smiles_modal;
        egui::Window::new("Enter / Edit SMILES Structure")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Enter a SMILES string (e.g. c1ccccc1, Cc1ccc(N)cc1, CCO, CC(=O)O, C1CCCCC1):");
                ui.add(egui::TextEdit::singleline(&mut self.smiles_input).desired_width(340.0));
                ui.horizontal(|ui| {
                    if ui.button("Build Molecule").clicked() {
                        match SoryntelApplication::parse_benson_smiles(&self.smiles_input) {
                            Ok(mol) => {
                                self.load_molecule(mol, db);
                                self.show_smiles_modal = false;
                            }
                            Err(e) => {
                                self.last_error = Some(e.clone());
                            }
                        }
                    }
                    if ui.button(crate::i18n::text("common.000")).clicked() {
                        self.show_smiles_modal = false;
                    }
                });
                if let Some(err) = &self.last_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
            });
        if !open {
            self.show_smiles_modal = false;
        }
    }

    fn render_help_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_help;
        egui::Window::new("HSC-BEN Help — Benson GAV Estimation")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("1. Load a skeleton from 'Add structural block' (e.g. Aromatic group -> Benzene) or File -> SMILES.");
                ui.label("2. Select an atom in the right 'Please select' palette (C, N, O, S, F, Cl, Br, I, B, P, Si, H).");
                ui.label("3. Click any white Hydrogen atom in the 3D viewport to replace it with that functional group (e.g. C -> -CH3, N -> -NH2).");
                ui.label("4. Left-drag to rotate the 3D model, right-drag to pan, mouse wheel to zoom.");
                ui.label("5. Adjust Atom size (1.00 = Ball-and-stick, 3.60 = Space-filling) and Camera (Perspective / Orthographic).");
                ui.label("6. Click 'Calculate' to view Gas / Liquid / Solid estimates, DB comparison, and save to database.");
            });
        if !open {
            self.show_help = false;
        }
    }

    fn render_about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_about;
        egui::Window::new("About HSC-BEN (Benson Estimation)")
            .open(&mut open)
            .show(ctx, |ui| {
                ui.strong("Module 11 — Benson Group Additivity Value (GAV) Estimation");
                ui.label("Estimates standard gas-phase ΔfH°, S°, and Cp(T) via Benson (1958, 1969, 1976) second-order group contributions, Kelley 4-parameter regression, and Joback/Trouton condensed phase transitions.");
                ui.label("Integrated with Soryntel Hexagonal Architecture & Maindb6 SpeciesCatalogPort.");
            });
        if !open {
            self.show_about = false;
        }
    }

    fn format_clipboard_text(&self) -> String {
        if let Some(est) = &self.estimation {
            format!(
                "Formula: {}\nCommon name: {}\nMW: {:.3} g/mol\nGas est: dHf={:.2} kJ/mol, S={:.2} J/molK, Cp={:.2} J/molK\nLiquid est: dHf={:.2} kJ/mol, S={:.2} J/molK, Cp={:.2} J/molK\nSolid est: dHf={:.2} kJ/mol, S={:.2} J/molK, Cp={:.2} J/molK\n",
                est.formula,
                est.common_name,
                est.molecular_weight,
                est.gas_est.delta_h_f_j / 1000.0,
                est.gas_est.s_j_k,
                est.gas_est.cp_j_k,
                est.liquid_est.delta_h_f_j / 1000.0,
                est.liquid_est.s_j_k,
                est.liquid_est.cp_j_k,
                est.solid_est.delta_h_f_j / 1000.0,
                est.solid_est.s_j_k,
                est.solid_est.cp_j_k,
            )
        } else {
            format!("Formula: {}\nMW: {:.3}", self.molecule.molecular_formula(), self.molecule.molecular_weight())
        }
    }

    fn export_csv_report(&mut self) {
        let path = "/tmp/soryntel_benson_report.csv";
        let content = self.format_clipboard_text();
        match crate::storage::write_atomic(path, content) {
            Ok(_) => {
                self.status_text = format!("Exported Benson report to {path}");
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Export failed: {e}"));
            }
        }
    }

    fn save_svg_snapshot(&mut self) {
        let path = "/tmp/soryntel_benson_molecule.svg";
        let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"640\" height=\"480\" viewBox=\"-6 -4.5 12 9\">\n<rect x=\"-6\" y=\"-4.5\" width=\"12\" height=\"9\" fill=\"#181a1e\"/>\n");
        for b in &self.molecule.bonds {
            let p1 = rotate_point(self.molecule.atoms[b.a].pos, self.yaw, self.pitch);
            let p2 = rotate_point(self.molecule.atoms[b.b].pos, self.yaw, self.pitch);
            svg.push_str(&format!(
                "<line x1=\"{:.3}\" y1=\"{:.3}\" x2=\"{:.3}\" y2=\"{:.3}\" stroke=\"#1428dc\" stroke-width=\"0.12\" stroke-linecap=\"round\"/>\n",
                p1[0], -p1[1], p2[0], -p2[1]
            ));
        }
        for a in &self.molecule.atoms {
            let p = rotate_point(a.pos, self.yaw, self.pitch);
            let col = atom_sphere_color(a.element);
            let r = 0.25 * a.element.visual_radius() * self.atom_size.min(2.5);
            svg.push_str(&format!(
                "<circle cx=\"{:.3}\" cy=\"{:.3}\" r=\"{:.3}\" fill=\"rgb({},{},{})\"/>\n",
                p[0], -p[1], r, col.r(), col.g(), col.b()
            ));
        }
        svg.push_str("</svg>\n");
        match crate::storage::write_atomic(path, svg) {
            Ok(_) => self.status_text = format!("Saved 3D snapshot SVG to {path}"),
            Err(e) => self.last_error = Some(format!("Save image failed: {e}")),
        }
    }
}

fn ribbon_group(ui: &mut egui::Ui, caption: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        let inner = ui.horizontal(|ui| {
            ui.set_min_height(46.0);
            add_contents(ui);
        });
        let w = inner.response.rect.width().max(48.0);
        ui.add_sized(
            [w, 14.0],
            egui::Label::new(
                egui::RichText::new(caption)
                    .size(10.0)
                    .color(egui::Color32::from_rgb(107, 114, 128)),
            ),
        );
    });
}

fn ribbon_sep(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 62.0), egui::Sense::hover());
    let x = rect.center().x;
    ui.painter().line_segment(
        [egui::pos2(x, rect.top() + 3.0), egui::pos2(x, rect.bottom() - 3.0)],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 212, 222)),
    );
}

fn build_selected_group_preview(el: ElementKind) -> MoleculeGraph {
    let smiles = match el {
        ElementKind::C => "C",
        ElementKind::N => "N",
        ElementKind::O => "O",
        ElementKind::S => "S",
        ElementKind::P => "P",
        ElementKind::B => "B",
        ElementKind::Si => "[SiH4]",
        ElementKind::F => "F",
        ElementKind::Cl => "Cl",
        ElementKind::Br => "Br",
        ElementKind::I => "I",
        ElementKind::H => "[H][H]",
    };
    MoleculeGraph::from_smiles(smiles).unwrap_or_else(|_| MoleculeGraph::empty())
}

fn element_badge_colors(el: ElementKind) -> (egui::Color32, egui::Color32) {
    use ElementKind::*;
    match el {
        C | Si => (egui::Color32::from_rgb(18, 20, 24), egui::Color32::WHITE),
        N => (egui::Color32::from_rgb(28, 78, 216), egui::Color32::WHITE),
        O => (egui::Color32::from_rgb(220, 38, 38), egui::Color32::WHITE),
        S => (egui::Color32::from_rgb(240, 226, 40), egui::Color32::BLACK),
        F => (egui::Color32::from_rgb(245, 240, 205), egui::Color32::BLACK),
        Cl => (egui::Color32::from_rgb(126, 232, 108), egui::Color32::BLACK),
        Br => (egui::Color32::from_rgb(192, 132, 62), egui::Color32::BLACK),
        I => (egui::Color32::from_rgb(235, 35, 215), egui::Color32::BLACK),
        B => (egui::Color32::from_rgb(172, 92, 235), egui::Color32::BLACK),
        P => (egui::Color32::from_rgb(244, 228, 198), egui::Color32::BLACK),
        H => (egui::Color32::from_rgb(248, 250, 252), egui::Color32::BLACK),
    }
}

fn atom_sphere_color(el: ElementKind) -> egui::Color32 {
    use ElementKind::*;
    match el {
        C => egui::Color32::from_rgb(112, 120, 128),
        Si => egui::Color32::from_rgb(65, 72, 82),
        N => egui::Color32::from_rgb(32, 86, 230),
        O => egui::Color32::from_rgb(225, 32, 32),
        S => egui::Color32::from_rgb(232, 216, 36),
        F => egui::Color32::from_rgb(232, 238, 185),
        Cl => egui::Color32::from_rgb(80, 210, 95),
        Br => egui::Color32::from_rgb(176, 98, 48),
        I => egui::Color32::from_rgb(212, 40, 196),
        B => egui::Color32::from_rgb(176, 102, 224),
        P => egui::Color32::from_rgb(240, 165, 65),
        H => egui::Color32::from_rgb(242, 245, 250),
    }
}

fn paint_radial_dark_bg(painter: &egui::Painter, rect: egui::Rect, bg: CanvasBackground) {
    let painter = painter.with_clip_rect(rect);
    match bg {
        CanvasBackground::SkyBlue => {
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(72, 162, 212));
        }
        CanvasBackground::LightStudio => {
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(238, 242, 248));
        }
        CanvasBackground::DarkSlate => {
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(28, 34, 46));
        }
        CanvasBackground::DarkRadial => {
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 11, 13));
            let center = rect.center();
            let max_r = rect.width().hypot(rect.height()) * 0.56;
            for step in 0..8 {
                let frac = 1.0 - (step as f32) / 8.0;
                let r = max_r * frac;
                let lum = (12.0 + 44.0 * (1.0 - frac)).round() as u8;
                painter.circle_filled(center, r, egui::Color32::from_rgb(lum, lum + 1, lum + 3));
            }
        }
    }
}

fn rotate_point(p: [f64; 3], yaw: f32, pitch: f32) -> [f32; 3] {
    let (cy, sy) = (yaw.cos(), yaw.sin());
    let (cp, sp) = (pitch.cos(), pitch.sin());
    let x = p[0] as f32;
    let y = p[1] as f32;
    let z = p[2] as f32;
    // Rotate around Y (yaw)
    let x1 = x * cy + z * sy;
    let z1 = -x * sy + z * cy;
    // Rotate around X (pitch)
    let y2 = y * cp - z1 * sp;
    let z2 = y * sp + z1 * cp;
    [x1, y2, z2]
}

struct ProjectedRenderInfo {
    hovered_atom: Option<usize>,
}

#[derive(Clone, Copy)]
enum RenderItem {
    Bond {
        depth: f32,
        a: egui::Pos2,
        b: egui::Pos2,
        width: f32,
        order: BondOrder,
    },
    Atom {
        depth: f32,
        idx: usize,
        center: egui::Pos2,
        radius: f32,
        element: ElementKind,
    },
}

fn render_molecule_3d_painter(
    painter: &egui::Painter,
    rect: egui::Rect,
    mol: &MoleculeGraph,
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: egui::Vec2,
    atom_size: f32,
    opacity: f32,
    camera: CameraMode,
    pointer_pos: Option<egui::Pos2>,
) -> ProjectedRenderInfo {
    if mol.atoms.is_empty() {
        return ProjectedRenderInfo { hovered_atom: None };
    }

    // Determine bounding radius so molecule fits comfortably inside viewport
    let mut max_ext = 1.8_f32;
    for a in &mol.atoms {
        let d = (a.pos[0] * a.pos[0] + a.pos[1] * a.pos[1] + a.pos[2] * a.pos[2]).sqrt() as f32;
        if d + 0.8 > max_ext {
            max_ext = d + 0.8;
        }
    }
    let base_scale = (rect.width().min(rect.height()) * 0.42 / max_ext) * zoom;
    let cam_dist = 13.5_f32;

    let mut proj_pts = Vec::with_capacity(mol.atoms.len());
    for a in &mol.atoms {
        let rp = rotate_point(a.pos, yaw, pitch);
        let persp = match camera {
            CameraMode::Perspective => (cam_dist / (cam_dist - rp[2]).max(2.5)).clamp(0.45, 2.2),
            CameraMode::Orthographic => 1.0,
        };
        let sx = rect.center().x + pan.x + rp[0] * base_scale * persp;
        let sy = rect.center().y + pan.y - rp[1] * base_scale * persp;
        proj_pts.push((egui::pos2(sx, sy), rp[2], persp));
    }

    let mut items = Vec::with_capacity(mol.atoms.len() + mol.bonds.len());

    // Bonds (suppressed when strongly space-filling > 2.8)
    let stick_w = (base_scale * 0.14).clamp(2.2, 8.5);
    for b in &mol.bonds {
        let (pa, za, _) = proj_pts[b.a];
        let (pb, zb, _) = proj_pts[b.b];
        items.push(RenderItem::Bond {
            depth: 0.5 * (za + zb),
            a: pa,
            b: pb,
            width: stick_w,
            order: b.order,
        });
    }

    // Atoms
    for (idx, a) in mol.atoms.iter().enumerate() {
        let (pos, z, persp) = proj_pts[idx];
        let rad = (base_scale * 0.30 * a.element.visual_radius() * atom_size * persp).clamp(4.0, 95.0);
        items.push(RenderItem::Atom {
            depth: z,
            idx,
            center: pos,
            radius: rad,
            element: a.element,
        });
    }

    // Painter's algorithm sort: back-to-front (smaller z first, larger z toward camera last)
    items.sort_by(|x, y| {
        let dx = match x {
            RenderItem::Bond { depth, .. } | RenderItem::Atom { depth, .. } => *depth,
        };
        let dy = match y {
            RenderItem::Bond { depth, .. } | RenderItem::Atom { depth, .. } => *depth,
        };
        dx.partial_cmp(&dy).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Hit-test frontmost atom under pointer
    let mut hovered_atom = None;
    if let Some(mp) = pointer_pos {
        for item in items.iter().rev() {
            if let RenderItem::Atom {
                idx,
                center,
                radius,
                ..
            } = *item
            {
                if center.distance(mp) <= radius + 3.0 {
                    hovered_atom = Some(idx);
                    break;
                }
            }
        }
    }

    let alpha_u8 = (opacity.clamp(0.15, 1.0) * 255.0).round() as u8;
    let bond_main = egui::Color32::from_rgba_unmultiplied(20, 40, 212, alpha_u8);
    let bond_hi = egui::Color32::from_rgba_unmultiplied(72, 112, 246, alpha_u8);

    let clipped = painter.with_clip_rect(rect);
    for item in items {
        match item {
            RenderItem::Bond {
                a,
                b,
                width,
                order,
                ..
            } => {
                let d = b - a;
                let len = d.length();
                if len < 1.0 {
                    continue;
                }
                let perp = egui::vec2(-d.y / len, d.x / len);
                match order {
                    BondOrder::Single => {
                        clipped.line_segment([a, b], egui::Stroke::new(width, bond_main));
                        clipped.line_segment([a, b], egui::Stroke::new(width * 0.35, bond_hi));
                    }
                    BondOrder::Double | BondOrder::Aromatic => {
                        let off = perp * (width * 0.55);
                        let w2 = (width * 0.68).max(1.8);
                        clipped.line_segment([a + off, b + off], egui::Stroke::new(w2, bond_main));
                        clipped.line_segment([a - off, b - off], egui::Stroke::new(w2, bond_main));
                    }
                    BondOrder::Triple => {
                        let off = perp * (width * 0.85);
                        let w3 = (width * 0.52).max(1.4);
                        clipped.line_segment([a, b], egui::Stroke::new(w3, bond_main));
                        clipped.line_segment([a + off, b + off], egui::Stroke::new(w3, bond_main));
                        clipped.line_segment([a - off, b - off], egui::Stroke::new(w3, bond_main));
                    }
                }
            }
            RenderItem::Atom {
                idx,
                center,
                radius,
                element,
                ..
            } => {
                let base = atom_sphere_color(element);
                paint_glossy_sphere(&clipped, center, radius, base, alpha_u8);
                if hovered_atom == Some(idx) {
                    clipped.circle_stroke(
                        center,
                        radius + 3.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(250, 204, 21)),
                    );
                }
            }
        }
    }

    ProjectedRenderInfo { hovered_atom }
}

fn paint_glossy_sphere(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    base: egui::Color32,
    alpha: u8,
) {
    let blend = |c: egui::Color32, factor: f32| -> egui::Color32 {
        if factor < 1.0 {
            egui::Color32::from_rgba_unmultiplied(
                (c.r() as f32 * factor).round() as u8,
                (c.g() as f32 * factor).round() as u8,
                (c.b() as f32 * factor).round() as u8,
                alpha,
            )
        } else {
            let t = (factor - 1.0).clamp(0.0, 1.0);
            egui::Color32::from_rgba_unmultiplied(
                (c.r() as f32 + (255.0 - c.r() as f32) * t).round() as u8,
                (c.g() as f32 + (255.0 - c.g() as f32) * t).round() as u8,
                (c.b() as f32 + (255.0 - c.b() as f32) * t).round() as u8,
                alpha,
            )
        }
    };

    // 5-layer Blinn-Phong shaded disc approximation
    let light_dir = egui::vec2(-0.34, -0.34);
    painter.circle_filled(center, radius, blend(base, 0.62));
    painter.circle_filled(
        center + light_dir * (radius * 0.14),
        radius * 0.86,
        blend(base, 0.92),
    );
    painter.circle_filled(
        center + light_dir * (radius * 0.30),
        radius * 0.64,
        blend(base, 1.18),
    );
    painter.circle_filled(
        center + light_dir * (radius * 0.45),
        radius * 0.38,
        blend(base, 1.48),
    );
    painter.circle_filled(
        center + light_dir * (radius * 0.54),
        radius * 0.17,
        blend(base, 1.82),
    );
}
