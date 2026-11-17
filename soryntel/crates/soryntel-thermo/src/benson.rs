//! Module 11 — Benson Group Additivity (HSC-BEN) Thermochemical Estimation for Organic Compounds.
//!
//! Implements Benson (1958, 1969, 1976) second-order group additivity (`A-(B1)(B2)...`),
//! symmetry (`σ_tot`) and optical isomer (`n_opt`) entropy corrections, ring strain /
//! conformational (`gauche`, `cis`) corrections, SMILES / 3D molecular graph construction,
//! Kelley heat-capacity polynomial regression, Joback/Trouton condensed phase (Liquid/Solid)
//! estimation, and `ChemicalSpecies` serialization for `SpeciesCatalogPort`.

use crate::estimate::fit_kelley_cp;
use crate::{
    evaluate_species_si, ChemicalSpecies, KelleyCoefficients, Phase, SpeciesCatalogPort,
    SpeciesThermoInterval,
};
use serde::{Deserialize, Serialize};
use soryntel_core::{CAL_TO_JOULE, R_CAL, T_REF_K};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Standard Benson Cp temperature grid in Kelvin.
pub const BENSON_CP_TEMPS_K: [f64; 7] = [300.0, 400.0, 500.0, 600.0, 800.0, 1000.0, 1500.0];

/// Elements supported in the HSC-BEN molecular palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ElementKind {
    C,
    N,
    O,
    S,
    F,
    Cl,
    Br,
    I,
    B,
    P,
    Si,
    H,
}

impl ElementKind {
    pub const PALETTE_ORDER: [ElementKind; 12] = [
        ElementKind::C,
        ElementKind::N,
        ElementKind::O,
        ElementKind::S,
        ElementKind::F,
        ElementKind::Cl,
        ElementKind::Br,
        ElementKind::I,
        ElementKind::B,
        ElementKind::P,
        ElementKind::Si,
        ElementKind::H,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::N => "N",
            Self::O => "O",
            Self::S => "S",
            Self::F => "F",
            Self::Cl => "Cl",
            Self::Br => "Br",
            Self::I => "I",
            Self::B => "B",
            Self::P => "P",
            Self::Si => "Si",
            Self::H => "H",
        }
    }

    pub fn atomic_weight(self) -> f64 {
        match self {
            Self::C => 12.011,
            Self::N => 14.0067,
            Self::O => 15.9994,
            Self::S => 32.065,
            Self::F => 18.9984,
            Self::Cl => 35.453,
            Self::Br => 79.904,
            Self::I => 126.9045,
            Self::B => 10.811,
            Self::P => 30.9738,
            Self::Si => 28.0855,
            Self::H => 1.00794,
        }
    }

    pub fn default_valence(self) -> usize {
        match self {
            Self::C | Self::Si => 4,
            Self::N | Self::B | Self::P => 3,
            Self::O | Self::S => 2,
            Self::F | Self::Cl | Self::Br | Self::I | Self::H => 1,
        }
    }

    /// Covalent bond length multiplier radius (Å) for 3D geometry.
    pub fn covalent_radius_angstrom(self) -> f64 {
        match self {
            Self::H => 0.31,
            Self::C => 0.76,
            Self::N => 0.71,
            Self::O => 0.66,
            Self::S => 1.05,
            Self::F => 0.57,
            Self::Cl => 1.02,
            Self::Br => 1.20,
            Self::I => 1.39,
            Self::B => 0.84,
            Self::P => 1.07,
            Self::Si => 1.11,
        }
    }

    /// Relative display radius (base = 1.0 corresponds to C ball-and-stick).
    pub fn visual_radius(self) -> f32 {
        match self {
            Self::H => 0.65,
            Self::C => 1.00,
            Self::N => 0.96,
            Self::O => 0.94,
            Self::F => 0.88,
            Self::B => 0.98,
            Self::S | Self::P | Self::Si | Self::Cl => 1.15,
            Self::Br => 1.22,
            Self::I => 1.30,
        }
    }

    pub fn from_symbol(sym: &str) -> Option<Self> {
        match sym {
            "C" | "c" => Some(Self::C),
            "N" | "n" => Some(Self::N),
            "O" | "o" => Some(Self::O),
            "S" | "s" => Some(Self::S),
            "F" => Some(Self::F),
            "Cl" => Some(Self::Cl),
            "Br" => Some(Self::Br),
            "I" => Some(Self::I),
            "B" => Some(Self::B),
            "P" => Some(Self::P),
            "Si" => Some(Self::Si),
            "H" => Some(Self::H),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondOrder {
    Single,
    Double,
    Triple,
    Aromatic,
}

impl BondOrder {
    pub fn valence_units(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Double => 2,
            Self::Triple => 3,
            Self::Aromatic => 1, // handled via aromatic valence rules
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomNode {
    pub id: usize,
    pub element: ElementKind,
    pub aromatic: bool,
    pub pos: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondEdge {
    pub a: usize,
    pub b: usize,
    pub order: BondOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoleculeGraph {
    pub name: String,
    pub common_name: String,
    pub smiles: String,
    pub atoms: Vec<AtomNode>,
    pub bonds: Vec<BondEdge>,
}

/// Structural block categories matching HSC-BEN `Add structural block` menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockCategory {
    FunctionalGroup,
    AlkaneGroup,
    Cycloalkane,
    PolycyclicGroup,
    AlkeneGroup,
    AliphaticGroup,
    AromaticGroup,
    HeteroaromaticGroup,
    Lactone,
    CyclicKetone,
}

impl BlockCategory {
    pub const ALL: [BlockCategory; 10] = [
        Self::FunctionalGroup,
        Self::AlkaneGroup,
        Self::Cycloalkane,
        Self::PolycyclicGroup,
        Self::AlkeneGroup,
        Self::AliphaticGroup,
        Self::AromaticGroup,
        Self::HeteroaromaticGroup,
        Self::Lactone,
        Self::CyclicKetone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FunctionalGroup => "Functional group",
            Self::AlkaneGroup => "Alkane group",
            Self::Cycloalkane => "Cycloalkane",
            Self::PolycyclicGroup => "Polycyclic group",
            Self::AlkeneGroup => "Alkene group",
            Self::AliphaticGroup => "Aliphatic group",
            Self::AromaticGroup => "Aromatic group",
            Self::HeteroaromaticGroup => "Heteroaromatic group",
            Self::Lactone => "Lactone",
            Self::CyclicKetone => "Cyclic ketone",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StructuralBlockSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub common_name: &'static str,
    pub category: BlockCategory,
    pub smiles: &'static str,
    pub functional_attach: Option<ElementKind>,
}

pub const STRUCTURAL_BLOCKS: &[StructuralBlockSpec] = &[
    // Functional groups (can replace a selected H or spawn a starter molecule)
    StructuralBlockSpec { id: "fg_oh", name: "Hydroxyl (-OH / Methanol)", common_name: "Hydroxyl group", category: BlockCategory::FunctionalGroup, smiles: "CO", functional_attach: Some(ElementKind::O) },
    StructuralBlockSpec { id: "fg_nh2", name: "Amine (-NH2 / Methylamine)", common_name: "Amine group", category: BlockCategory::FunctionalGroup, smiles: "CN", functional_attach: Some(ElementKind::N) },
    StructuralBlockSpec { id: "fg_ch3", name: "Methyl (-CH3 / Ethane)", common_name: "Methyl group", category: BlockCategory::FunctionalGroup, smiles: "CC", functional_attach: Some(ElementKind::C) },
    StructuralBlockSpec { id: "fg_sh", name: "Thiol (-SH / Methanethiol)", common_name: "Thiol group", category: BlockCategory::FunctionalGroup, smiles: "CS", functional_attach: Some(ElementKind::S) },
    StructuralBlockSpec { id: "fg_cooh", name: "Carboxyl (-COOH / Acetic Acid)", common_name: "Acetic acid", category: BlockCategory::FunctionalGroup, smiles: "CC(=O)O", functional_attach: None },
    StructuralBlockSpec { id: "fg_cho", name: "Aldehyde (-CHO / Acetaldehyde)", common_name: "Acetaldehyde", category: BlockCategory::FunctionalGroup, smiles: "CC=O", functional_attach: None },
    StructuralBlockSpec { id: "fg_ket", name: "Carbonyl (Acetone)", common_name: "Acetone", category: BlockCategory::FunctionalGroup, smiles: "CC(=O)C", functional_attach: None },
    // Alkane group
    StructuralBlockSpec { id: "alk_methane", name: "Methane", common_name: "Methane", category: BlockCategory::AlkaneGroup, smiles: "C", functional_attach: None },
    StructuralBlockSpec { id: "alk_ethane", name: "Ethane", common_name: "Ethane", category: BlockCategory::AlkaneGroup, smiles: "CC", functional_attach: None },
    StructuralBlockSpec { id: "alk_propane", name: "Propane", common_name: "Propane", category: BlockCategory::AlkaneGroup, smiles: "CCC", functional_attach: None },
    StructuralBlockSpec { id: "alk_butane", name: "n-Butane", common_name: "Butane", category: BlockCategory::AlkaneGroup, smiles: "CCCC", functional_attach: None },
    StructuralBlockSpec { id: "alk_isobutane", name: "Isobutane", common_name: "2-Methylpropane", category: BlockCategory::AlkaneGroup, smiles: "CC(C)C", functional_attach: None },
    StructuralBlockSpec { id: "alk_neopentane", name: "Neopentane", common_name: "2,2-Dimethylpropane", category: BlockCategory::AlkaneGroup, smiles: "CC(C)(C)C", functional_attach: None },
    // Cycloalkane
    StructuralBlockSpec { id: "cyc_3", name: "Cyclopropane", common_name: "Cyclopropane", category: BlockCategory::Cycloalkane, smiles: "C1CC1", functional_attach: None },
    StructuralBlockSpec { id: "cyc_4", name: "Cyclobutane", common_name: "Cyclobutane", category: BlockCategory::Cycloalkane, smiles: "C1CCC1", functional_attach: None },
    StructuralBlockSpec { id: "cyc_5", name: "Cyclopentane", common_name: "Cyclopentane", category: BlockCategory::Cycloalkane, smiles: "C1CCCC1", functional_attach: None },
    StructuralBlockSpec { id: "cyc_6", name: "Cyclohexane", common_name: "Cyclohexane", category: BlockCategory::Cycloalkane, smiles: "C1CCCCC1", functional_attach: None },
    StructuralBlockSpec { id: "cyc_8", name: "Cyclooctane", common_name: "Cyclooctane", category: BlockCategory::Cycloalkane, smiles: "C1CCCCCCC1", functional_attach: None },
    // Polycyclic group
    StructuralBlockSpec { id: "poly_naphthalene", name: "Naphthalene", common_name: "Naphthalene", category: BlockCategory::PolycyclicGroup, smiles: "c1ccc2ccccc2c1", functional_attach: None },
    StructuralBlockSpec { id: "poly_anthracene", name: "Anthracene", common_name: "Anthracene", category: BlockCategory::PolycyclicGroup, smiles: "c1ccc2cc3ccccc3cc2c1", functional_attach: None },
    StructuralBlockSpec { id: "poly_phenanthrene", name: "Phenanthrene", common_name: "Phenanthrene", category: BlockCategory::PolycyclicGroup, smiles: "c1ccc2c(c1)ccc3ccccc23", functional_attach: None },
    StructuralBlockSpec { id: "poly_pyrene", name: "Pyrene", common_name: "Pyrene", category: BlockCategory::PolycyclicGroup, smiles: "c1cc2ccc3cccc4ccc(c1)c2c34", functional_attach: None },
    StructuralBlockSpec { id: "poly_coronene", name: "Coronene", common_name: "Coronene", category: BlockCategory::PolycyclicGroup, smiles: "c7(c1ccc2ccc3ccc4ccc5ccc6cc1)c2c3c4c5c67", functional_attach: None },
    // Alkene group
    StructuralBlockSpec { id: "ene_ethene", name: "Ethene (Ethylene)", common_name: "Ethylene", category: BlockCategory::AlkeneGroup, smiles: "C=C", functional_attach: None },
    StructuralBlockSpec { id: "ene_propene", name: "Propene", common_name: "Propylene", category: BlockCategory::AlkeneGroup, smiles: "CC=C", functional_attach: None },
    StructuralBlockSpec { id: "ene_1butene", name: "1-Butene", common_name: "1-Butene", category: BlockCategory::AlkeneGroup, smiles: "CCC=C", functional_attach: None },
    StructuralBlockSpec { id: "ene_2butene", name: "2-Butene", common_name: "2-Butene", category: BlockCategory::AlkeneGroup, smiles: "CC=CC", functional_attach: None },
    StructuralBlockSpec { id: "ene_isobutene", name: "Isobutene", common_name: "2-Methylpropene", category: BlockCategory::AlkeneGroup, smiles: "CC(=C)C", functional_attach: None },
    // Aliphatic group (alkynes / oxygenates)
    StructuralBlockSpec { id: "ali_acetylene", name: "Acetylene (Ethyne)", common_name: "Acetylene", category: BlockCategory::AliphaticGroup, smiles: "C#C", functional_attach: None },
    StructuralBlockSpec { id: "ali_propyne", name: "Propyne", common_name: "Propyne", category: BlockCategory::AliphaticGroup, smiles: "CC#C", functional_attach: None },
    StructuralBlockSpec { id: "ali_ethanol", name: "Ethanol", common_name: "Ethyl alcohol", category: BlockCategory::AliphaticGroup, smiles: "CCO", functional_attach: None },
    // Aromatic group (matches Fig. 5 list!)
    StructuralBlockSpec { id: "aro_anthracene", name: "Anthracene", common_name: "Anthracene", category: BlockCategory::AromaticGroup, smiles: "c1ccc2cc3ccccc3cc2c1", functional_attach: None },
    StructuralBlockSpec { id: "aro_acenaphthene", name: "Acenaphthene", common_name: "Acenaphthene", category: BlockCategory::AromaticGroup, smiles: "C1Cc2cccc3cccc1c23", functional_attach: None },
    StructuralBlockSpec { id: "aro_acenaphthylene", name: "Acenaphthylene", common_name: "Acenaphthylene", category: BlockCategory::AromaticGroup, smiles: "C1=Cc2cccc3cccc1c23", functional_attach: None },
    StructuralBlockSpec { id: "aro_benzene", name: "Benzene", common_name: "Benzene", category: BlockCategory::AromaticGroup, smiles: "c1ccccc1", functional_attach: None },
    StructuralBlockSpec { id: "aro_baanthracene", name: "Benzo(a)anthracene", common_name: "Benzo(a)anthracene", category: BlockCategory::AromaticGroup, smiles: "c1ccc2c(c1)ccc3cc4ccccc4cc23", functional_attach: None },
    StructuralBlockSpec { id: "aro_bapyrene", name: "Benzo(a)pyrene", common_name: "Benzo(a)pyrene", category: BlockCategory::AromaticGroup, smiles: "c1ccc2c(c1)cc3ccc4cccc5ccc2c3c45", functional_attach: None },
    StructuralBlockSpec { id: "aro_bepyr", name: "Benzo(e)pyrene", common_name: "Benzo(e)pyrene", category: BlockCategory::AromaticGroup, smiles: "c1ccc2c(c1)c3ccc4cccc5ccc6ccc2c3c4c56", functional_attach: None },
    StructuralBlockSpec { id: "aro_toluene", name: "Toluene", common_name: "Methylbenzene", category: BlockCategory::AromaticGroup, smiles: "Cc1ccccc1", functional_attach: None },
    StructuralBlockSpec { id: "aro_4ma", name: "4-Methylaniline", common_name: "4-Methylaniline; p-Toluidine", category: BlockCategory::AromaticGroup, smiles: "Cc1ccc(N)cc1", functional_attach: None },
    StructuralBlockSpec { id: "aro_diol_ant", name: "9,10-Anthracenedimethanol (C16H14O2)", common_name: "1,4-Bis(hydroxymethyl)anthracene", category: BlockCategory::AromaticGroup, smiles: "OCc1c2ccccc2c(CO)c3ccccc13", functional_attach: None },
    // Heteroaromatic group
    StructuralBlockSpec { id: "het_pyridine", name: "Pyridine", common_name: "Pyridine", category: BlockCategory::HeteroaromaticGroup, smiles: "c1ccncc1", functional_attach: None },
    StructuralBlockSpec { id: "het_pyrrole", name: "Pyrrole", common_name: "Pyrrole", category: BlockCategory::HeteroaromaticGroup, smiles: "c1cc[nH]c1", functional_attach: None },
    StructuralBlockSpec { id: "het_furan", name: "Furan", common_name: "Furan", category: BlockCategory::HeteroaromaticGroup, smiles: "c1ccoc1", functional_attach: None },
    StructuralBlockSpec { id: "het_thiophene", name: "Thiophene", common_name: "Thiophene", category: BlockCategory::HeteroaromaticGroup, smiles: "c1ccsc1", functional_attach: None },
    // Lactone
    StructuralBlockSpec { id: "lac_beta", name: "beta-Propiolactone", common_name: "2-Oxetanone", category: BlockCategory::Lactone, smiles: "O=C1CCO1", functional_attach: None },
    StructuralBlockSpec { id: "lac_gamma", name: "gamma-Butyrolactone", common_name: "Dihydrofuran-2(3H)-one", category: BlockCategory::Lactone, smiles: "O=C1CCCO1", functional_attach: None },
    // Cyclic ketone
    StructuralBlockSpec { id: "cket_5", name: "Cyclopentanone", common_name: "Cyclopentanone", category: BlockCategory::CyclicKetone, smiles: "O=C1CCCC1", functional_attach: None },
    StructuralBlockSpec { id: "cket_6", name: "Cyclohexanone", common_name: "Cyclohexanone", category: BlockCategory::CyclicKetone, smiles: "O=C1CCCCC1", functional_attach: None },
];

pub fn structural_blocks_for(category: BlockCategory) -> Vec<&'static StructuralBlockSpec> {
    STRUCTURAL_BLOCKS
        .iter()
        .filter(|b| b.category == category)
        .collect()
}

pub fn find_structural_block(id: &str) -> Option<&'static StructuralBlockSpec> {
    STRUCTURAL_BLOCKS.iter().find(|b| b.id == id)
}

impl MoleculeGraph {
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            common_name: String::new(),
            smiles: String::new(),
            atoms: Vec::new(),
            bonds: Vec::new(),
        }
    }

    pub fn from_smiles(smiles: &str) -> Result<Self, String> {
        let trimmed = smiles.trim();
        if trimmed.is_empty() {
            return Err("Empty SMILES string.".to_string());
        }

        let chars: Vec<char> = trimmed.chars().collect();
        let mut idx = 0usize;
        let mut atoms: Vec<AtomNode> = Vec::new();
        let mut bonds: Vec<BondEdge> = Vec::new();
        let mut explicit_h_deficit: Vec<Option<usize>> = Vec::new();
        let mut branch_stack: Vec<usize> = Vec::new();
        let mut ring_anchors: [Option<(usize, Option<BondOrder>)>; 10] = [None; 10];
        let mut prev_atom: Option<usize> = None;
        let mut next_bond_order: Option<BondOrder> = None;

        while idx < chars.len() {
            let ch = chars[idx];
            match ch {
                '(' => {
                    if let Some(pa) = prev_atom {
                        branch_stack.push(pa);
                    } else {
                        return Err("Unexpected '(' at start of SMILES.".into());
                    }
                    idx += 1;
                }
                ')' => {
                    prev_atom = branch_stack.pop();
                    if prev_atom.is_none() {
                        return Err("Unmatched ')' in SMILES.".into());
                    }
                    next_bond_order = None;
                    idx += 1;
                }
                '-' => {
                    next_bond_order = Some(BondOrder::Single);
                    idx += 1;
                }
                '=' => {
                    next_bond_order = Some(BondOrder::Double);
                    idx += 1;
                }
                '#' => {
                    next_bond_order = Some(BondOrder::Triple);
                    idx += 1;
                }
                ':' => {
                    next_bond_order = Some(BondOrder::Aromatic);
                    idx += 1;
                }
                '0'..='9' => {
                    let ring_num = (ch as u8 - b'0') as usize;
                    let cur = prev_atom.ok_or_else(|| "Ring digit without preceding atom.".to_string())?;
                    if let Some((anchor, open_order)) = ring_anchors[ring_num].take() {
                        if anchor == cur {
                            return Err("Self-loop ring bond in SMILES.".into());
                        }
                        let both_aromatic = atoms[anchor].aromatic && atoms[cur].aromatic;
                        let order = next_bond_order
                            .or(open_order)
                            .unwrap_or(if both_aromatic {
                                BondOrder::Aromatic
                            } else {
                                BondOrder::Single
                            });
                        bonds.push(BondEdge { a: anchor, b: cur, order });
                    } else {
                        ring_anchors[ring_num] = Some((cur, next_bond_order));
                    }
                    next_bond_order = None;
                    idx += 1;
                }
                '[' => {
                    let end = chars[idx + 1..]
                        .iter()
                        .position(|&c| c == ']')
                        .map(|p| idx + 1 + p)
                        .ok_or_else(|| "Unclosed '[' atom bracket in SMILES.".to_string())?;
                    let inner: String = chars[idx + 1..end].iter().collect();
                    let (elem, aromatic, h_exp) = parse_bracket_atom(&inner)?;
                    let new_id = atoms.len();
                    atoms.push(AtomNode {
                        id: new_id,
                        element: elem,
                        aromatic,
                        pos: [0.0; 3],
                    });
                    explicit_h_deficit.push(Some(h_exp));
                    if let Some(pa) = prev_atom {
                        let both_aro = aromatic && atoms[pa].aromatic;
                        let order = next_bond_order.unwrap_or(if both_aro {
                            BondOrder::Aromatic
                        } else {
                            BondOrder::Single
                        });
                        bonds.push(BondEdge { a: pa, b: new_id, order });
                    }
                    prev_atom = Some(new_id);
                    next_bond_order = None;
                    idx = end + 1;
                }
                _ => {
                    let two_char = if idx + 1 < chars.len() {
                        format!("{ch}{}", chars[idx + 1])
                    } else {
                        String::new()
                    };
                    let (elem, aromatic, advance) = if two_char == "Cl" {
                        (ElementKind::Cl, false, 2)
                    } else if two_char == "Br" {
                        (ElementKind::Br, false, 2)
                    } else if two_char == "Si" {
                        (ElementKind::Si, false, 2)
                    } else {
                        let aromatic = matches!(ch, 'c' | 'n' | 'o' | 's' | 'p' | 'b');
                        let elem = match ch {
                            'C' | 'c' => ElementKind::C,
                            'N' | 'n' => ElementKind::N,
                            'O' | 'o' => ElementKind::O,
                            'S' | 's' => ElementKind::S,
                            'F' => ElementKind::F,
                            'I' => ElementKind::I,
                            'B' | 'b' => ElementKind::B,
                            'P' | 'p' => ElementKind::P,
                            'H' => ElementKind::H,
                            other => {
                                return Err(format!("Unsupported SMILES token '{other}' in '{trimmed}'"));
                            }
                        };
                        (elem, aromatic, 1)
                    };

                    let new_id = atoms.len();
                    atoms.push(AtomNode {
                        id: new_id,
                        element: elem,
                        aromatic,
                        pos: [0.0; 3],
                    });
                    explicit_h_deficit.push(None);
                    if let Some(pa) = prev_atom {
                        let both_aro = aromatic && atoms[pa].aromatic;
                        let order = next_bond_order.unwrap_or(if both_aro {
                            BondOrder::Aromatic
                        } else {
                            BondOrder::Single
                        });
                        bonds.push(BondEdge { a: pa, b: new_id, order });
                    }
                    prev_atom = Some(new_id);
                    next_bond_order = None;
                    idx += advance;
                }
            }
        }

        if let Some(r_idx) = ring_anchors.iter().position(|x| x.is_some()) {
            return Err(format!("Unclosed ring closure '{r_idx}' in SMILES '{trimmed}'"));
        }
        if !branch_stack.is_empty() {
            return Err("Unclosed branch '(' in SMILES.".into());
        }

        // Convert alternating Kekule aromatic C=C rings (like C1=CC=CC=C1) cleanly if tagged aromatic
        // and add implicit Hydrogens based on remaining standard valence.
        let heavy_count = atoms.len();
        for i in 0..heavy_count {
            let elem = atoms[i].element;
            if elem == ElementKind::H {
                continue;
            }
            let h_count = if let Some(exp_h) = explicit_h_deficit[i] {
                exp_h
            } else {
                let mut used_val = 0usize;
                let mut aro_bonds = 0usize;
                for b in &bonds {
                    if b.a == i || b.b == i {
                        if b.order == BondOrder::Aromatic {
                            aro_bonds += 1;
                        } else {
                            used_val += b.order.valence_units();
                        }
                    }
                }
                if atoms[i].aromatic || aro_bonds > 0 {
                    // Aromatic sp2 valence: C uses 3 (1.5*2 or 1+2) + H, N uses 3 (0 H in pyridine), O/S use 2 (0 H)
                    let aro_val = if aro_bonds >= 3 {
                        4
                    } else if elem == ElementKind::C {
                        aro_bonds + 1 // 2 aromatic neighbors -> +1 pi bond = 3 used, 1 H left
                    } else if elem == ElementKind::N {
                        if aro_bonds == 2 { 3 } else { aro_bonds + 1 }
                    } else {
                        elem.default_valence()
                    };
                    elem.default_valence().saturating_sub(used_val + aro_val)
                } else {
                    elem.default_valence().saturating_sub(used_val)
                }
            };

            for _ in 0..h_count {
                let h_id = atoms.len();
                atoms.push(AtomNode {
                    id: h_id,
                    element: ElementKind::H,
                    aromatic: false,
                    pos: [0.0; 3],
                });
                bonds.push(BondEdge {
                    a: i,
                    b: h_id,
                    order: BondOrder::Single,
                });
            }
        }

        let mut mol = Self {
            name: trimmed.to_string(),
            common_name: String::new(),
            smiles: trimmed.to_string(),
            atoms,
            bonds,
        };
        mol.generate_3d_coordinates();
        Ok(mol)
    }

    pub fn from_block(spec: &StructuralBlockSpec) -> Self {
        let mut mol = Self::from_smiles(spec.smiles).unwrap_or_else(|_| Self::empty());
        mol.name = spec.name.to_string();
        mol.common_name = spec.common_name.to_string();
        mol
    }

    /// Replace an existing Hydrogen (or attach to heavy atom with H replacement) to match HSC-BEN click-build.
    pub fn substitute_atom_with_element(&mut self, target_idx: usize, new_elem: ElementKind) -> Result<(), String> {
        if target_idx >= self.atoms.len() {
            return Err("Invalid atom index.".into());
        }
        let target_elem = self.atoms[target_idx].element;
        if target_elem == ElementKind::H && new_elem == ElementKind::H {
            return Ok(());
        }

        if target_elem == ElementKind::H {
            // Find parent heavy atom
            let parent_idx = self
                .bonds
                .iter()
                .find_map(|b| {
                    if b.a == target_idx {
                        Some(b.b)
                    } else if b.b == target_idx {
                        Some(b.a)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| "Hydrogen atom has no parent bond.".to_string())?;

            let p_pos = self.atoms[parent_idx].pos;
            let h_pos = self.atoms[target_idx].pos;
            let dir = normalize3([h_pos[0] - p_pos[0], h_pos[1] - p_pos[1], h_pos[2] - p_pos[2]]);
            let bond_len = target_bond_length(self.atoms[parent_idx].element, new_elem, BondOrder::Single);

            self.atoms[target_idx].element = new_elem;
            self.atoms[target_idx].aromatic = false;
            self.atoms[target_idx].pos = [
                p_pos[0] + dir[0] * bond_len,
                p_pos[1] + dir[1] * bond_len,
                p_pos[2] + dir[2] * bond_len,
            ];

            // Add implicit hydrogens to satisfy remaining valence of new_elem
            let rem_h = new_elem.default_valence().saturating_sub(1);
            let dirs = fan_directions_away(dir, rem_h);
            let h_len = target_bond_length(new_elem, ElementKind::H, BondOrder::Single);
            for d in dirs {
                let h_id = self.atoms.len();
                let anchor = self.atoms[target_idx].pos;
                self.atoms.push(AtomNode {
                    id: h_id,
                    element: ElementKind::H,
                    aromatic: false,
                    pos: [
                        anchor[0] + d[0] * h_len,
                        anchor[1] + d[1] * h_len,
                        anchor[2] + d[2] * h_len,
                    ],
                });
                self.bonds.push(BondEdge {
                    a: target_idx,
                    b: h_id,
                    order: BondOrder::Single,
                });
            }
            self.smiles = format!("{} (edited)", self.name);
            self.common_name.clear();
            self.relax_geometry_mild(45);
            Ok(())
        } else {
            // Clicking a heavy atom: if it has an attached H, substitute that first H with new_elem!
            if let Some(h_neighbor) = self.neighbors(target_idx).into_iter().find(|&n| self.atoms[n].element == ElementKind::H) {
                if new_elem != ElementKind::H {
                    return self.substitute_atom_with_element(h_neighbor, new_elem);
                }
            }
            Err("Select a terminal H atom (or a heavy atom with available H) to attach the group.".into())
        }
    }

    pub fn neighbors(&self, atom_idx: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for b in &self.bonds {
            if b.a == atom_idx {
                out.push(b.b);
            } else if b.b == atom_idx {
                out.push(b.a);
            }
        }
        out
    }

    pub fn element_counts(&self) -> BTreeMap<ElementKind, usize> {
        let mut counts = BTreeMap::new();
        for a in &self.atoms {
            *counts.entry(a.element).or_insert(0) += 1;
        }
        counts
    }

    /// Standard Hill-system molecular formula (e.g. `C7H9N`, `C16H14O2`).
    pub fn molecular_formula(&self) -> String {
        let counts = self.element_counts();
        if counts.is_empty() {
            return String::new();
        }
        let mut out = String::new();
        let append = |out: &mut String, sym: &str, n: usize| {
            out.push_str(sym);
            if n > 1 {
                out.push_str(&n.to_string());
            }
        };
        if let Some(&nc) = counts.get(&ElementKind::C) {
            append(&mut out, "C", nc);
            if let Some(&nh) = counts.get(&ElementKind::H) {
                append(&mut out, "H", nh);
            }
            for (&el, &n) in &counts {
                if el != ElementKind::C && el != ElementKind::H {
                    append(&mut out, el.symbol(), n);
                }
            }
        } else {
            let mut entries: Vec<(&str, usize)> =
                counts.iter().map(|(e, &n)| (e.symbol(), n)).collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            for (sym, n) in entries {
                append(&mut out, sym, n);
            }
        }
        out
    }

    pub fn molecular_weight(&self) -> f64 {
        self.atoms.iter().map(|a| a.element.atomic_weight()).sum()
    }

    /// Generate clean 3D coordinates for rings, chains, and attached hydrogens.
    pub fn generate_3d_coordinates(&mut self) {
        let n = self.atoms.len();
        if n == 0 {
            return;
        }
        let heavy_indices: Vec<usize> = (0..n)
            .filter(|&i| self.atoms[i].element != ElementKind::H)
            .collect();

        if heavy_indices.is_empty() {
            for (k, a) in self.atoms.iter_mut().enumerate() {
                a.pos = [k as f64 * 0.8, 0.0, 0.0];
            }
            return;
        }

        // Initial deterministic spiral/ring placement on XY with mild Z buckle so 3D rotation looks natural
        let cycles = detect_ring_cycles(&self.atoms, &self.bonds);
        let mut placed = vec![false; n];

        // Place first cycle in a clean polygon in XY plane (with chair/crown puckering for non-aromatic rings >= 5)
        if let Some(first_cycle) = cycles.first() {
            let m = first_cycle.len();
            let bond_r = 1.44;
            let rad = bond_r / (2.0 * (std::f64::consts::PI / m as f64).sin()).max(0.2);
            let is_aro = first_cycle.iter().all(|&id| self.atoms[id].aromatic);
            for (k, &id) in first_cycle.iter().enumerate() {
                let ang = 2.0 * std::f64::consts::PI * (k as f64) / (m as f64);
                let z = if is_aro || m <= 4 {
                    0.0
                } else if k % 2 == 0 {
                    0.28
                } else {
                    -0.28
                };
                self.atoms[id].pos = [rad * ang.cos(), rad * ang.sin(), z];
                placed[id] = true;
            }
        } else {
            let root = heavy_indices[0];
            self.atoms[root].pos = [0.0, 0.0, 0.0];
            placed[root] = true;
        }

        // BFS place remaining heavy atoms
        let mut queue: VecDeque<usize> = heavy_indices
            .iter()
            .copied()
            .filter(|&id| placed[id])
            .collect();
        while let Some(u) = queue.pop_front() {
            let mut unplaced_neighbors: Vec<usize> = self
                .neighbors(u)
                .into_iter()
                .filter(|&v| !placed[v] && self.atoms[v].element != ElementKind::H)
                .collect();
            unplaced_neighbors.sort();
            let base_dir = incoming_direction(&self.atoms, &self.bonds, u, &placed);
            let dirs = fan_directions_away(base_dir, unplaced_neighbors.len().max(1));
            for (k, v) in unplaced_neighbors.into_iter().enumerate() {
                let blen = target_bond_length(
                    self.atoms[u].element,
                    self.atoms[v].element,
                    self.bond_order_between(u, v).unwrap_or(BondOrder::Single),
                );
                let d = dirs[k % dirs.len()];
                self.atoms[v].pos = [
                    self.atoms[u].pos[0] + d[0] * blen,
                    self.atoms[u].pos[1] + d[1] * blen,
                    self.atoms[u].pos[2] + d[2] * blen,
                ];
                placed[v] = true;
                queue.push_back(v);
            }
        }

        // Force-directed 3D refinement among heavy atoms to close fused rings (anthracene/coronene)
        self.relax_heavy_atoms(140);

        // Place Hydrogens outward along tetrahedral / trigonal directions from their heavy parent
        for i in 0..n {
            if self.atoms[i].element == ElementKind::H {
                continue;
            }
            let h_neighbors: Vec<usize> = self
                .neighbors(i)
                .into_iter()
                .filter(|&v| self.atoms[v].element == ElementKind::H)
                .collect();
            if h_neighbors.is_empty() {
                continue;
            }
            let heavy_dir = avg_neighbor_direction(&self.atoms, &self.bonds, i);
            let dirs = fan_directions_away(heavy_dir, h_neighbors.len());
            let h_len = target_bond_length(self.atoms[i].element, ElementKind::H, BondOrder::Single);
            for (k, h_id) in h_neighbors.into_iter().enumerate() {
                let d = dirs[k];
                self.atoms[h_id].pos = [
                    self.atoms[i].pos[0] + d[0] * h_len,
                    self.atoms[i].pos[1] + d[1] * h_len,
                    self.atoms[i].pos[2] + d[2] * h_len,
                ];
            }
        }

        // Center molecule at origin
        self.center_at_origin();
    }

    fn relax_heavy_atoms(&mut self, iters: usize) {
        let heavy: Vec<usize> = (0..self.atoms.len())
            .filter(|&i| self.atoms[i].element != ElementKind::H)
            .collect();
        if heavy.len() <= 1 {
            return;
        }
        let all_aro = heavy.iter().all(|&i| self.atoms[i].aromatic);
        for _ in 0..iters {
            let mut forces = vec![[0.0_f64; 3]; self.atoms.len()];
            // Bond spring forces
            for b in &self.bonds {
                if self.atoms[b.a].element == ElementKind::H || self.atoms[b.b].element == ElementKind::H {
                    continue;
                }
                let target = target_bond_length(self.atoms[b.a].element, self.atoms[b.b].element, b.order);
                let dx = self.atoms[b.b].pos[0] - self.atoms[b.a].pos[0];
                let dy = self.atoms[b.b].pos[1] - self.atoms[b.a].pos[1];
                let dz = self.atoms[b.b].pos[2] - self.atoms[b.a].pos[2];
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-4);
                let f = 0.35 * (dist - target) / dist;
                forces[b.a][0] += f * dx;
                forces[b.a][1] += f * dy;
                forces[b.a][2] += f * dz;
                forces[b.b][0] -= f * dx;
                forces[b.b][1] -= f * dy;
                forces[b.b][2] -= f * dz;
            }
            // Non-bonded steric repulsion between heavy atoms
            for ia in 0..heavy.len() {
                for ib in (ia + 1)..heavy.len() {
                    let a = heavy[ia];
                    let b = heavy[ib];
                    let dx = self.atoms[b].pos[0] - self.atoms[a].pos[0];
                    let dy = self.atoms[b].pos[1] - self.atoms[a].pos[1];
                    let dz = self.atoms[b].pos[2] - self.atoms[a].pos[2];
                    let d2 = (dx * dx + dy * dy + dz * dz).max(0.05);
                    let rep = if self.bond_order_between(a, b).is_some() {
                        0.0
                    } else if self.share_common_neighbor(a, b) {
                        // 1-3 angle keeper (~2.45 Å)
                        if d2 < 2.40 * 2.40 {
                            0.18 * (2.40 * 2.40 - d2) / d2
                        } else {
                            0.0
                        }
                    } else {
                        (0.35 / (d2 * d2)).min(0.6)
                    };
                    if rep > 0.0 {
                        forces[a][0] -= rep * dx;
                        forces[a][1] -= rep * dy;
                        forces[a][2] -= rep * dz;
                        forces[b][0] += rep * dx;
                        forces[b][1] += rep * dy;
                        forces[b][2] += rep * dz;
                    }
                }
            }
            for &i in &heavy {
                self.atoms[i].pos[0] += forces[i][0].clamp(-0.25, 0.25);
                self.atoms[i].pos[1] += forces[i][1].clamp(-0.25, 0.25);
                self.atoms[i].pos[2] += if all_aro {
                    -0.1 * self.atoms[i].pos[2]
                } else {
                    forces[i][2].clamp(-0.18, 0.18)
                };
            }
        }
    }

    fn relax_geometry_mild(&mut self, iters: usize) {
        self.relax_heavy_atoms(iters);
        for i in 0..self.atoms.len() {
            if self.atoms[i].element != ElementKind::H {
                continue;
            }
            if let Some(p) = self.neighbors(i).into_iter().next() {
                let target = target_bond_length(self.atoms[p].element, ElementKind::H, BondOrder::Single);
                let dir = normalize3([
                    self.atoms[i].pos[0] - self.atoms[p].pos[0],
                    self.atoms[i].pos[1] - self.atoms[p].pos[1],
                    self.atoms[i].pos[2] - self.atoms[p].pos[2],
                ]);
                self.atoms[i].pos = [
                    self.atoms[p].pos[0] + dir[0] * target,
                    self.atoms[p].pos[1] + dir[1] * target,
                    self.atoms[p].pos[2] + dir[2] * target,
                ];
            }
        }
        self.center_at_origin();
    }

    fn center_at_origin(&mut self) {
        if self.atoms.is_empty() {
            return;
        }
        let n = self.atoms.len() as f64;
        let mut cx = 0.0;
        let mut cy = 0.0;
        let mut cz = 0.0;
        for a in &self.atoms {
            cx += a.pos[0];
            cy += a.pos[1];
            cz += a.pos[2];
        }
        cx /= n;
        cy /= n;
        cz /= n;
        for a in &mut self.atoms {
            a.pos[0] -= cx;
            a.pos[1] -= cy;
            a.pos[2] -= cz;
        }
    }

    fn bond_order_between(&self, u: usize, v: usize) -> Option<BondOrder> {
        self.bonds.iter().find_map(|b| {
            if (b.a == u && b.b == v) || (b.a == v && b.b == u) {
                Some(b.order)
            } else {
                None
            }
        })
    }

    fn share_common_neighbor(&self, u: usize, v: usize) -> bool {
        let nu = self.neighbors(u);
        let nv = self.neighbors(v);
        nu.iter().any(|x| nv.contains(x))
    }
}

fn parse_bracket_atom(inner: &str) -> Result<(ElementKind, bool, usize), String> {
    let mut chars = inner.chars().peekable();
    let first = chars.next().ok_or_else(|| "Empty bracket atom".to_string())?;
    let mut sym = String::new();
    sym.push(first);
    let aromatic = first.is_ascii_lowercase();
    if let Some(&c2) = chars.peek() {
        if c2.is_ascii_lowercase() && (first == 'C' || first == 'B' || first == 'S') {
            let two = format!("{first}{c2}");
            if matches!(two.as_str(), "Cl" | "Br" | "Si") {
                sym.push(c2);
                chars.next();
            }
        }
    }
    let elem = ElementKind::from_symbol(&sym)
        .ok_or_else(|| format!("Unsupported bracket element '{sym}'"))?;
    let mut h_exp = 0usize;
    while let Some(c) = chars.next() {
        if c == 'H' {
            h_exp = 1;
            if let Some(&d) = chars.peek() {
                if let Some(num) = d.to_digit(10) {
                    h_exp = num as usize;
                    chars.next();
                }
            }
        }
    }
    Ok((elem, aromatic, h_exp))
}

fn target_bond_length(a: ElementKind, b: ElementKind, order: BondOrder) -> f64 {
    let base = a.covalent_radius_angstrom() + b.covalent_radius_angstrom();
    match order {
        BondOrder::Single => base,
        BondOrder::Aromatic => base * 0.91,
        BondOrder::Double => base * 0.86,
        BondOrder::Triple => base * 0.79,
    }
}

fn normalize3(v: [f64; 3]) -> [f64; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-9 {
        [1.0, 0.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn avg_neighbor_direction(atoms: &[AtomNode], bonds: &[BondEdge], u: usize) -> [f64; 3] {
    let mut sum = [0.0; 3];
    let mut cnt = 0;
    for b in bonds {
        let other = if b.a == u {
            b.b
        } else if b.b == u {
            b.a
        } else {
            continue;
        };
        if atoms[other].element == ElementKind::H {
            continue;
        }
        let dx = atoms[other].pos[0] - atoms[u].pos[0];
        let dy = atoms[other].pos[1] - atoms[u].pos[1];
        let dz = atoms[other].pos[2] - atoms[u].pos[2];
        let d = normalize3([dx, dy, dz]);
        sum[0] += d[0];
        sum[1] += d[1];
        sum[2] += d[2];
        cnt += 1;
    }
    if cnt == 0 {
        [0.0, 0.0, -1.0]
    } else {
        normalize3(sum)
    }
}

fn incoming_direction(atoms: &[AtomNode], bonds: &[BondEdge], u: usize, placed: &[bool]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    let mut cnt = 0;
    for b in bonds {
        let other = if b.a == u {
            b.b
        } else if b.b == u {
            b.a
        } else {
            continue;
        };
        if !placed[other] {
            continue;
        }
        let d = normalize3([
            atoms[other].pos[0] - atoms[u].pos[0],
            atoms[other].pos[1] - atoms[u].pos[1],
            atoms[other].pos[2] - atoms[u].pos[2],
        ]);
        sum[0] += d[0];
        sum[1] += d[1];
        sum[2] += d[2];
        cnt += 1;
    }
    if cnt == 0 {
        [-1.0, 0.0, 0.0]
    } else {
        normalize3(sum)
    }
}

/// Produce `k` outward 3D unit vectors pointing generally away from `incoming`.
fn fan_directions_away(incoming: [f64; 3], k: usize) -> Vec<[f64; 3]> {
    let fwd = normalize3([-incoming[0], -incoming[1], -incoming[2]]);
    let ref_up = if fwd[2].abs() < 0.85 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
    let u = normalize3(cross3(fwd, ref_up));
    let v = normalize3(cross3(fwd, u));

    if k == 0 {
        return Vec::new();
    }
    if k == 1 {
        // Tetrahedral/trigonal outward continuation
        return vec![normalize3([
            fwd[0] * 0.94 + v[0] * 0.34,
            fwd[1] * 0.94 + v[1] * 0.34,
            fwd[2] * 0.94 + v[2] * 0.34,
        ])];
    }
    if k == 2 {
        return vec![
            normalize3([
                fwd[0] * 0.58 + u[0] * 0.72 + v[0] * 0.38,
                fwd[1] * 0.58 + u[1] * 0.72 + v[1] * 0.38,
                fwd[2] * 0.58 + u[2] * 0.72 + v[2] * 0.38,
            ]),
            normalize3([
                fwd[0] * 0.58 - u[0] * 0.72 + v[0] * 0.38,
                fwd[1] * 0.58 - u[1] * 0.72 + v[1] * 0.38,
                fwd[2] * 0.58 - u[2] * 0.72 + v[2] * 0.38,
            ]),
        ];
    }
    if k == 3 {
        let mut out = Vec::with_capacity(3);
        for m in 0..3 {
            let ang = 2.0 * std::f64::consts::PI * (m as f64) / 3.0;
            let c = ang.cos();
            let s = ang.sin();
            out.push(normalize3([
                fwd[0] * 0.34 + (u[0] * c + v[0] * s) * 0.94,
                fwd[1] * 0.34 + (u[1] * c + v[1] * s) * 0.94,
                fwd[2] * 0.34 + (u[2] * c + v[2] * s) * 0.94,
            ]));
        }
        return out;
    }
    // Tetrahedral 4 (e.g. methane)
    vec![
        normalize3([0.0, 0.0, 1.0]),
        normalize3([0.943, 0.0, -0.333]),
        normalize3([-0.471, 0.816, -0.333]),
        normalize3([-0.471, -0.816, -0.333]),
    ]
}

fn detect_ring_cycles(atoms: &[AtomNode], bonds: &[BondEdge]) -> Vec<Vec<usize>> {
    let n = atoms.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for b in bonds {
        if atoms[b.a].element != ElementKind::H && atoms[b.b].element != ElementKind::H {
            adj[b.a].push(b.b);
            adj[b.b].push(b.a);
        }
    }
    // Simple fundamental cycle finder via BFS shortest-path on spanning tree edges
    let mut parent = vec![usize::MAX; n];
    let mut visited = vec![false; n];
    let mut tree_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut chord_edges: Vec<(usize, usize)> = Vec::new();
    for start in 0..n {
        if atoms[start].element == ElementKind::H || visited[start] {
            continue;
        }
        visited[start] = true;
        let mut q = VecDeque::new();
        q.push_back(start);
        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                let edge = if u < v { (u, v) } else { (v, u) };
                if !visited[v] {
                    visited[v] = true;
                    parent[v] = u;
                    tree_edges.insert(edge);
                    q.push_back(v);
                } else if v != parent[u] && !tree_edges.contains(&edge) && !chord_edges.contains(&edge) {
                    chord_edges.push(edge);
                }
            }
        }
    }

    let mut cycles = Vec::new();
    for (u, v) in chord_edges {
        // Shortest path in tree (or existing graph without chord) between u and v
        let mut prev = vec![usize::MAX; n];
        let mut seen = vec![false; n];
        let mut q = VecDeque::new();
        seen[u] = true;
        q.push_back(u);
        while let Some(cur) = q.pop_front() {
            if cur == v {
                break;
            }
            for &nxt in &adj[cur] {
                if (cur == u && nxt == v) || (cur == v && nxt == u) {
                    continue;
                }
                if !seen[nxt] {
                    seen[nxt] = true;
                    prev[nxt] = cur;
                    q.push_back(nxt);
                }
            }
        }
        if seen[v] {
            let mut path = vec![v];
            let mut cur = v;
            while cur != u {
                cur = prev[cur];
                path.push(cur);
            }
            cycles.push(path);
        }
    }
    cycles.sort_by_key(|c| c.len());
    cycles
}

/// Curated Benson group parameter entry (`kcal/mol`, `cal/(mol·K)`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BensonGroupParams {
    pub name: &'static str,
    pub dh_kcal: f64,
    pub s_cal: f64,
    /// Cp in cal/(mol·K) at [300, 400, 500, 600, 800, 1000, 1500] K.
    pub cp_cal: [f64; 7],
    /// Joback Tb increment (K) and Tm increment (K) for Liquid/Solid estimation.
    pub joback_tb: f64,
    pub joback_tm: f64,
    pub joback_cpa: f64,
}

pub const BENSON_TABLE: &[BensonGroupParams] = &[
    // sp3 Carbon centers
    BensonGroupParams { name: "C-(H)4", dh_kcal: -17.89, s_cal: 44.49, cp_cal: [8.55, 9.73, 11.13, 12.55, 15.00, 17.16, 20.63], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)3(C)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)2", dh_kcal: -4.95, s_cal: 9.42, cp_cal: [5.50, 6.95, 8.25, 9.35, 11.07, 12.34, 14.25], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)(C)3", dh_kcal: -1.90, s_cal: -12.07, cp_cal: [4.54, 6.00, 7.17, 8.05, 9.31, 10.05, 11.17], joback_tb: 21.74, joback_tm: 12.64, joback_cpa: -23.00 },
    BensonGroupParams { name: "C-(C)4", dh_kcal: 0.50, s_cal: -35.10, cp_cal: [4.37, 6.13, 7.36, 8.12, 8.77, 8.76, 8.50], joback_tb: 18.25, joback_tm: 46.43, joback_cpa: -66.20 },
    BensonGroupParams { name: "C-(H)3(Cd)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(Cd)", dh_kcal: -4.76, s_cal: 9.80, cp_cal: [5.12, 6.86, 8.32, 9.49, 11.22, 12.48, 14.35], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)3(Ct)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)3(Cb)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(Cb)", dh_kcal: -4.86, s_cal: 9.34, cp_cal: [5.84, 7.61, 9.03, 10.17, 11.79, 12.90, 14.50], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)2(Cb)(O)", dh_kcal: -8.10, s_cal: 9.70, cp_cal: [4.99, 6.85, 8.30, 9.43, 11.11, 12.33, 14.00], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)3(O)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(O)", dh_kcal: -8.10, s_cal: 9.80, cp_cal: [4.99, 6.85, 8.30, 9.43, 11.11, 12.33, 14.00], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)(C)2(O)", dh_kcal: -7.20, s_cal: -11.00, cp_cal: [4.80, 6.64, 8.10, 8.73, 9.81, 10.40, 11.30], joback_tb: 21.74, joback_tm: 12.64, joback_cpa: -23.00 },
    BensonGroupParams { name: "C-(H)3(N)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(N)", dh_kcal: -6.60, s_cal: 9.80, cp_cal: [5.25, 6.90, 8.28, 9.39, 11.09, 12.34, 14.10], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)3(CO)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(CO)", dh_kcal: -5.20, s_cal: 9.60, cp_cal: [6.20, 7.70, 8.70, 9.50, 11.10, 12.20, 13.90], joback_tb: 22.88, joback_tm: 11.27, joback_cpa: -0.909 },
    BensonGroupParams { name: "C-(H)3(S)", dh_kcal: -10.08, s_cal: 30.41, cp_cal: [6.19, 7.84, 9.40, 10.79, 13.02, 14.77, 17.58], joback_tb: 23.58, joback_tm: -5.10, joback_cpa: 19.50 },
    BensonGroupParams { name: "C-(H)2(C)(Cl)", dh_kcal: -16.50, s_cal: 37.80, cp_cal: [8.90, 10.70, 12.20, 13.40, 15.10, 16.30, 18.00], joback_tb: 61.01, joback_tm: 4.52, joback_cpa: 32.40 },
    BensonGroupParams { name: "C-(H)2(C)(F)", dh_kcal: -51.50, s_cal: 35.40, cp_cal: [8.10, 9.90, 11.50, 12.70, 14.50, 15.80, 17.60], joback_tb: 22.80, joback_tm: -4.40, joback_cpa: 25.00 },
    BensonGroupParams { name: "C-(H)2(C)(Br)", dh_kcal: -5.40, s_cal: 40.80, cp_cal: [9.10, 10.90, 12.30, 13.50, 15.20, 16.40, 18.10], joback_tb: 88.80, joback_tm: 31.80, joback_cpa: 34.00 },
    BensonGroupParams { name: "C-(H)3(Cl)", dh_kcal: -20.60, s_cal: 55.94, cp_cal: [9.74, 11.49, 13.10, 14.47, 16.64, 18.25, 20.70], joback_tb: 61.01, joback_tm: 4.52, joback_cpa: 32.40 },

    // Alkene Cd (sp2)
    BensonGroupParams { name: "Cd-(H)2", dh_kcal: 6.26, s_cal: 27.61, cp_cal: [5.10, 6.36, 7.51, 8.50, 10.07, 11.27, 13.20], joback_tb: 18.18, joback_tm: -4.32, joback_cpa: 23.60 },
    BensonGroupParams { name: "Cd-(H)(C)", dh_kcal: 8.59, s_cal: 7.97, cp_cal: [4.16, 5.03, 5.81, 6.50, 7.65, 8.45, 9.62], joback_tb: 24.96, joback_tm: 8.73, joback_cpa: -8.00 },
    BensonGroupParams { name: "Cd-(C)2", dh_kcal: 10.34, s_cal: -12.70, cp_cal: [4.10, 4.61, 4.99, 5.26, 5.80, 6.08, 6.50], joback_tb: 24.14, joback_tm: 11.14, joback_cpa: -28.10 },
    BensonGroupParams { name: "Cd-(H)(Cb)", dh_kcal: 6.78, s_cal: 6.40, cp_cal: [4.46, 5.79, 6.75, 7.42, 8.35, 9.11, 10.10], joback_tb: 24.96, joback_tm: 8.73, joback_cpa: -8.00 },
    BensonGroupParams { name: "Cd-(C)(Cb)", dh_kcal: 8.64, s_cal: -14.60, cp_cal: [4.40, 5.30, 5.90, 6.30, 6.80, 7.10, 7.40], joback_tb: 24.14, joback_tm: 11.14, joback_cpa: -28.10 },
    BensonGroupParams { name: "Cd-(H)(Cd)", dh_kcal: 6.78, s_cal: 6.38, cp_cal: [4.46, 5.79, 6.75, 7.42, 8.35, 9.11, 10.10], joback_tb: 24.96, joback_tm: 8.73, joback_cpa: -8.00 },

    // Alkyne Ct (sp)
    BensonGroupParams { name: "Ct-(H)", dh_kcal: 26.93, s_cal: 24.70, cp_cal: [5.27, 5.99, 6.49, 6.87, 7.47, 7.96, 8.85], joback_tb: 9.20, joback_tm: -11.55, joback_cpa: 24.50 },
    BensonGroupParams { name: "Ct-(C)", dh_kcal: 27.55, s_cal: 6.35, cp_cal: [3.13, 3.48, 3.81, 4.09, 4.60, 4.92, 5.40], joback_tb: 27.38, joback_tm: 64.29, joback_cpa: 7.87 },

    // Aromatic Cb
    BensonGroupParams { name: "Cb-(H)", dh_kcal: 3.30, s_cal: 11.53, cp_cal: [3.24, 4.44, 5.46, 6.30, 7.54, 8.41, 9.73], joback_tb: 26.73, joback_tm: 8.13, joback_cpa: -2.14 },
    BensonGroupParams { name: "Cb-(C)", dh_kcal: 5.51, s_cal: -7.69, cp_cal: [2.67, 3.14, 3.68, 4.15, 4.96, 5.44, 6.10], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(Cb)", dh_kcal: 4.96, s_cal: -8.64, cp_cal: [2.99, 3.66, 4.22, 4.64, 5.23, 5.55, 6.00], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(N)", dh_kcal: -0.50, s_cal: -9.69, cp_cal: [3.95, 5.21, 5.94, 6.32, 6.53, 6.56, 6.60], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(O)", dh_kcal: -0.90, s_cal: -10.20, cp_cal: [3.90, 5.30, 6.20, 6.60, 6.90, 6.90, 6.80], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(CO)", dh_kcal: 3.70, s_cal: -7.70, cp_cal: [2.67, 3.14, 3.68, 4.15, 4.96, 5.44, 6.10], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(Cd)", dh_kcal: 5.68, s_cal: -7.80, cp_cal: [3.59, 3.97, 4.38, 4.72, 5.28, 5.61, 6.10], joback_tb: 31.01, joback_tm: 37.02, joback_cpa: -8.25 },
    BensonGroupParams { name: "Cb-(Cl)", dh_kcal: -3.80, s_cal: 18.90, cp_cal: [7.40, 8.40, 9.20, 9.70, 10.50, 11.00, 11.60], joback_tb: 69.14, joback_tm: 32.47, joback_cpa: 25.00 },
    BensonGroupParams { name: "Cb-(F)", dh_kcal: -42.80, s_cal: 16.10, cp_cal: [6.30, 7.60, 8.50, 9.20, 10.10, 10.70, 11.40], joback_tb: 30.80, joback_tm: 10.00, joback_cpa: 20.00 },
    BensonGroupParams { name: "Cb-(Br)", dh_kcal: 10.70, s_cal: 21.60, cp_cal: [7.80, 8.70, 9.40, 9.90, 10.60, 11.10, 11.70], joback_tb: 95.00, joback_tm: 48.00, joback_cpa: 26.00 },
    BensonGroupParams { name: "Cb-(I)", dh_kcal: 24.00, s_cal: 23.70, cp_cal: [8.00, 8.90, 9.60, 10.10, 10.80, 11.20, 11.80], joback_tb: 120.00, joback_tm: 60.00, joback_cpa: 27.00 },

    // Oxygen & Carbonyl groups
    BensonGroupParams { name: "O-(H)(C)", dh_kcal: -37.90, s_cal: 29.07, cp_cal: [4.33, 4.45, 4.82, 5.23, 6.02, 6.61, 7.60], joback_tb: 92.88, joback_tm: 44.45, joback_cpa: 25.70 },
    BensonGroupParams { name: "O-(H)(Cb)", dh_kcal: -37.90, s_cal: 29.10, cp_cal: [4.30, 4.50, 4.80, 5.20, 6.00, 6.60, 7.60], joback_tb: 76.34, joback_tm: 82.83, joback_cpa: -2.81 },
    BensonGroupParams { name: "O-(H)(CO)", dh_kcal: -58.10, s_cal: 24.52, cp_cal: [3.81, 4.98, 5.80, 6.34, 7.19, 7.75, 8.60], joback_tb: 92.88, joback_tm: 44.45, joback_cpa: 12.00 },
    BensonGroupParams { name: "O-(C)2", dh_kcal: -23.20, s_cal: 8.68, cp_cal: [3.40, 3.70, 3.70, 3.80, 4.40, 4.60, 5.00], joback_tb: 22.42, joback_tm: 22.23, joback_cpa: 25.50 },
    BensonGroupParams { name: "O-(C)(Cb)", dh_kcal: -23.00, s_cal: 9.20, cp_cal: [3.40, 3.70, 3.80, 4.00, 4.50, 4.70, 5.10], joback_tb: 22.42, joback_tm: 22.23, joback_cpa: 25.50 },
    BensonGroupParams { name: "CO-(H)(C)", dh_kcal: -29.10, s_cal: 34.93, cp_cal: [7.03, 7.87, 8.82, 9.68, 11.16, 12.20, 13.60], joback_tb: 72.24, joback_tm: 36.90, joback_cpa: 23.20 },
    BensonGroupParams { name: "CO-(C)2", dh_kcal: -31.40, s_cal: 15.01, cp_cal: [5.59, 6.32, 7.09, 7.76, 8.89, 9.61, 10.60], joback_tb: 76.75, joback_tm: 61.20, joback_cpa: 6.45 },
    BensonGroupParams { name: "CO-(C)(O)", dh_kcal: -35.10, s_cal: 14.78, cp_cal: [5.97, 6.70, 7.40, 8.02, 8.87, 9.44, 10.30], joback_tb: 81.10, joback_tm: 68.00, joback_cpa: 12.10 },
    BensonGroupParams { name: "CO-(Cb)(O)", dh_kcal: -32.60, s_cal: 15.10, cp_cal: [5.80, 6.60, 7.30, 7.90, 8.80, 9.40, 10.20], joback_tb: 81.10, joback_tm: 68.00, joback_cpa: 12.10 },

    // Nitrogen & Heteroaromatic groups
    BensonGroupParams { name: "N-(H)2(C)", dh_kcal: 4.80, s_cal: 29.71, cp_cal: [5.72, 6.51, 7.38, 8.22, 9.60, 10.74, 12.50], joback_tb: 73.23, joback_tm: 66.89, joback_cpa: 26.90 },
    BensonGroupParams { name: "N-(H)2(Cb)", dh_kcal: 4.80, s_cal: 29.71, cp_cal: [5.72, 6.51, 7.38, 8.22, 9.60, 10.74, 12.50], joback_tb: 73.23, joback_tm: 66.89, joback_cpa: 26.90 },
    BensonGroupParams { name: "N-(H)(C)2", dh_kcal: 15.40, s_cal: 8.94, cp_cal: [4.20, 5.21, 6.13, 6.83, 7.90, 8.65, 9.80], joback_tb: 50.17, joback_tm: 52.66, joback_cpa: -1.21 },
    BensonGroupParams { name: "N-(C)3", dh_kcal: 24.40, s_cal: -13.46, cp_cal: [3.48, 4.56, 5.43, 5.97, 6.56, 6.77, 7.10], joback_tb: 11.74, joback_tm: 48.84, joback_cpa: -31.10 },
    BensonGroupParams { name: "Nb-(Cb)2", dh_kcal: 16.00, s_cal: 9.80, cp_cal: [2.80, 3.60, 4.20, 4.70, 5.30, 5.70, 6.20], joback_tb: 57.55, joback_tm: 68.40, joback_cpa: 8.83 },

    // Sulfur & Phosphine groups
    BensonGroupParams { name: "S-(H)(C)", dh_kcal: 4.62, s_cal: 32.73, cp_cal: [5.86, 6.20, 6.54, 6.88, 7.43, 7.89, 8.60], joback_tb: 63.56, joback_tm: 20.09, joback_cpa: 35.30 },
    BensonGroupParams { name: "S-(C)2", dh_kcal: 11.51, s_cal: 13.15, cp_cal: [4.99, 4.96, 5.02, 5.07, 5.41, 5.73, 6.20], joback_tb: 68.78, joback_tm: 34.40, joback_cpa: 19.60 },
    BensonGroupParams { name: "P-(H)(C)2", dh_kcal: 11.00, s_cal: 18.00, cp_cal: [5.40, 6.20, 6.90, 7.50, 8.30, 8.90, 9.70], joback_tb: 48.00, joback_tm: 20.00, joback_cpa: 15.00 },
];

pub fn lookup_benson_group(name: &str) -> Option<&'static BensonGroupParams> {
    BENSON_TABLE.iter().find(|g| g.name == name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BensonGroupEntry {
    pub group_name: String,
    pub count: usize,
    pub dh_kcal_each: f64,
    pub s_cal_each: f64,
    pub cp_cal_300_each: f64,
    pub is_estimated_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BensonParamsInput {
    pub sigma_tot: f64,
    pub n_opt: usize,
    pub gauche_count: usize,
    pub cis_count: usize,
    pub ring_strain_kcal: f64,
    pub ring_entropy_cal: f64,
}

impl Default for BensonParamsInput {
    fn default() -> Self {
        Self {
            sigma_tot: 1.0,
            n_opt: 1,
            gauche_count: 0,
            cis_count: 0,
            ring_strain_kcal: 0.0,
            ring_entropy_cal: 0.0,
        }
    }
}

/// Phase-specific summary values (matching Fig. 10 & Fig. 11 HSC-BEN Results table).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PhaseThermoSummary {
    /// Enthalpy of formation in J/mol
    pub delta_h_f_j: f64,
    /// Entropy in J/(mol·K)
    pub s_j_k: f64,
    /// Cp(298.15 K) in J/(mol·K)
    pub cp_j_k: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BensonEstimationResult {
    pub formula: String,
    pub common_name: String,
    pub chemical_name: String,
    pub molecular_weight: f64,
    pub groups: Vec<BensonGroupEntry>,
    pub warnings: Vec<String>,
    pub params_used: BensonParamsInput,
    pub gas_est: PhaseThermoSummary,
    pub liquid_est: PhaseThermoSummary,
    pub solid_est: PhaseThermoSummary,
    pub gas_db: Option<PhaseThermoSummary>,
    pub liquid_db: Option<PhaseThermoSummary>,
    pub solid_db: Option<PhaseThermoSummary>,
    pub db_matches: Vec<String>,
    pub gas_kelley: KelleyCoefficients,
    pub cp_grid_j_k: Vec<(f64, f64)>,
    pub tm_est_k: f64,
    pub tb_est_k: f64,
}

/// Auto-decompose a `MoleculeGraph` into Benson group multiplicities and structural corrections.
pub fn decompose_molecule_to_benson(mol: &MoleculeGraph) -> (Vec<(String, usize)>, BensonParamsInput) {
    let mut group_counts: BTreeMap<String, usize> = BTreeMap::new();
    let cycles = detect_ring_cycles(&mol.atoms, &mol.bonds);

    let center_type_for = |idx: usize| -> &'static str {
        let a = &mol.atoms[idx];
        match a.element {
            ElementKind::C => {
                if a.aromatic {
                    return "Cb";
                }
                let has_double_o = mol.bonds.iter().any(|b| {
                    let other = if b.a == idx {
                        Some(b.b)
                    } else if b.b == idx {
                        Some(b.a)
                    } else {
                        None
                    };
                    other
                        .map(|o| {
                            b.order == BondOrder::Double && mol.atoms[o].element == ElementKind::O
                        })
                        .unwrap_or(false)
                });
                if has_double_o {
                    return "CO";
                }
                let has_triple = mol
                    .bonds
                    .iter()
                    .any(|b| (b.a == idx || b.b == idx) && b.order == BondOrder::Triple);
                if has_triple {
                    return "Ct";
                }
                let has_double = mol
                    .bonds
                    .iter()
                    .any(|b| (b.a == idx || b.b == idx) && b.order == BondOrder::Double);
                if has_double {
                    return "Cd";
                }
                "C"
            }
            ElementKind::N => {
                if a.aromatic {
                    "Nb"
                } else {
                    "N"
                }
            }
            ElementKind::O => {
                if a.aromatic {
                    "Ob"
                } else {
                    "O"
                }
            }
            ElementKind::S => "S",
            ElementKind::P => "P",
            ElementKind::Si => "Si",
            ElementKind::B => "B",
            _ => a.element.symbol(),
        }
    };

    for (i, atom) in mol.atoms.iter().enumerate() {
        // Monovalent atoms (H, F, Cl, Br, I) and carbonyl double-bonded O are ligands only.
        if matches!(
            atom.element,
            ElementKind::H | ElementKind::F | ElementKind::Cl | ElementKind::Br | ElementKind::I
        ) {
            continue;
        }
        // Skip carbonyl =O so CO-(...) captures the whole C=O center cleanly
        let is_carbonyl_o = atom.element == ElementKind::O
            && mol.bonds.iter().any(|b| {
                (b.a == i || b.b == i)
                    && b.order == BondOrder::Double
                    && mol.atoms[if b.a == i { b.b } else { b.a }].element == ElementKind::C
            });
        if is_carbonyl_o {
            continue;
        }

        let center = center_type_for(i);
        let mut ligand_counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut skipped_aro_for_cb = 0usize;
        for nb in mol.neighbors(i) {
            let nb_elem = mol.atoms[nb].element;
            // Skip the =O of a CO center and the double/triple bond partner of Cd/Ct centers
            let bo = mol.bond_order_between(i, nb).unwrap_or(BondOrder::Single);
            if center == "CO" && nb_elem == ElementKind::O && bo == BondOrder::Double {
                continue;
            }
            if (center == "Cd" && bo == BondOrder::Double)
                || (center == "Ct" && bo == BondOrder::Triple)
            {
                continue;
            }
            // In Benson notation, aromatic carbon `Cb-(X)` already implies 2 aromatic ring bonds.
            if center == "Cb" && (bo == BondOrder::Aromatic || atom.aromatic && mol.atoms[nb].aromatic) && skipped_aro_for_cb < 2 {
                skipped_aro_for_cb += 1;
                continue;
            }
            let lig = match nb_elem {
                ElementKind::H => "H",
                ElementKind::F => "F",
                ElementKind::Cl => "Cl",
                ElementKind::Br => "Br",
                ElementKind::I => "I",
                _ => center_type_for(nb),
            };
            *ligand_counts.entry(lig).or_insert(0) += 1;
        }

        // Format canonical Benson group string: `Center-(H)3(C)` -> H first if present, then sorted ligands
        let mut key = format!("{center}-");
        if let Some(&nh) = ligand_counts.get("H") {
            if nh > 1 {
                key.push_str(&format!("(H){nh}"));
            } else {
                key.push_str("(H)");
            }
        }
        for (&lig, &cnt) in &ligand_counts {
            if lig == "H" {
                continue;
            }
            if cnt > 1 {
                key.push_str(&format!("({lig}){cnt}"));
            } else {
                key.push_str(&format!("({lig})"));
            }
        }
        *group_counts.entry(key).or_insert(0) += 1;
    }

    // Ring strain & symmetry estimation
    let mut ring_strain_kcal = 0.0;
    let mut ring_entropy_cal = 0.0;
    let mut aromatic_rings = 0usize;
    let mut non_aro_cycles = 0usize;
    for cyc in &cycles {
        let all_aro = cyc.iter().all(|&id| mol.atoms[id].aromatic);
        if all_aro {
            aromatic_rings += 1;
        } else {
            non_aro_cycles += 1;
            let has_hetero_o = cyc.iter().any(|&id| mol.atoms[id].element == ElementKind::O);
            let (dh_r, ds_r) = match (cyc.len(), has_hetero_o) {
                (3, true) => (27.2, 29.0),
                (3, false) => (27.6, 32.1),
                (4, true) => (25.5, 27.0),
                (4, false) => (26.2, 29.8),
                (5, false) => (6.3, 27.3),
                (6, false) => (0.0, 18.8),
                (7, false) => (6.4, 15.5),
                (8, false) => (9.9, 14.0),
                _ => (2.0, 16.0),
            };
            ring_strain_kcal += dh_r;
            ring_entropy_cal += ds_r;
        }
    }

    // Estimate rotational symmetry number σ_tot = σ_ext * ∏ σ_internal_rotors
    let ch3_count = group_counts
        .iter()
        .filter(|(k, _)| k.starts_with("C-(H)3"))
        .map(|(_, &c)| c)
        .sum::<usize>();
    let nh2_cb_count = group_counts.get("N-(H)2(Cb)").copied().unwrap_or(0);
    let sigma_ext = estimate_external_symmetry(&group_counts, &cycles, aromatic_rings, non_aro_cycles);
    let sigma_int = 3_f64.powi(ch3_count as i32) * 2_f64.powi(nh2_cb_count as i32);
    let sigma_tot = (sigma_ext * sigma_int).max(1.0);

    let params = BensonParamsInput {
        sigma_tot,
        n_opt: 1,
        gauche_count: 0,
        cis_count: 0,
        ring_strain_kcal,
        ring_entropy_cal,
    };
    let groups: Vec<(String, usize)> = group_counts.into_iter().collect();
    (groups, params)
}

fn estimate_external_symmetry(
    counts: &BTreeMap<String, usize>,
    cycles: &[Vec<usize>],
    aro_rings: usize,
    non_aro_rings: usize,
) -> f64 {
    if counts.len() == 1 && counts.contains_key("C-(H)4") {
        return 12.0; // Td methane
    }
    if aro_rings == 1 && counts.len() == 1 && counts.get("Cb-(H)") == Some(&6) {
        return 12.0; // D6h benzene
    }
    if non_aro_rings == 1 && aro_rings == 0 && counts.len() == 1 {
        if let Some(cyc) = cycles.first() {
            return match cyc.len() {
                3 => 6.0,
                4 => 8.0,
                5 => 10.0,
                6 => 6.0,
                8 => 8.0,
                _ => 2.0,
            };
        }
    }
    if counts.get("C-(H)3(C)") == Some(&3) && counts.get("C-(H)(C)3") == Some(&1) && counts.len() == 2 {
        return 3.0; // C3v isobutane
    }
    if counts.get("C-(H)3(C)") == Some(&4) && counts.get("C-(C)4") == Some(&1) && counts.len() == 2 {
        return 12.0; // Td neopentane
    }
    if counts.get("C-(H)3(C)") == Some(&2) && counts.len() <= 2 {
        return 2.0; // ethane / propane / n-butane C2v/C2h
    }
    if aro_rings == 1 && counts.get("Cb-(H)") == Some(&5) && counts.get("Cb-(C)") == Some(&1) {
        return 2.0; // C2v toluene
    }
    if aro_rings == 1 && counts.get("Cb-(H)") == Some(&4) && counts.get("Cb-(C)") == Some(&1) && counts.get("Cb-(N)") == Some(&1) {
        return 2.0; // C2v 4-methylaniline
    }
    if aro_rings >= 2 {
        return 4.0; // D2h naphthalene / anthracene / pyrene
    }
    1.0
}

/// Evaluate complete Benson thermochemical estimation (Gas, Liquid, Solid + Kelley + DB lookup).
pub fn estimate_benson(
    mol: &MoleculeGraph,
    custom_groups: Option<&[(String, usize)]>,
    custom_params: Option<&BensonParamsInput>,
    catalog: Option<&dyn SpeciesCatalogPort>,
) -> Result<BensonEstimationResult, String> {
    let (auto_groups, auto_params) = decompose_molecule_to_benson(mol);
    let active_groups = custom_groups.unwrap_or(&auto_groups);
    let params = custom_params.cloned().unwrap_or(auto_params);

    if active_groups.is_empty() {
        return Err("No Benson groups detected in molecule.".into());
    }

    let mut entries = Vec::with_capacity(active_groups.len());
    let warnings = Vec::new();
    let mut dh_gas_kcal = params.ring_strain_kcal
        + (params.gauche_count as f64) * 0.80
        + (params.cis_count as f64) * 1.00;
    let mut s_int_cal = params.ring_entropy_cal;
    let mut cp_grid_cal = [0.0_f64; 7];
    let mut sum_tb = 0.0_f64;
    let mut sum_tm = 0.0_f64;
    let mut sum_cpa_liq_corr = 0.0_f64;

    for (name, count) in active_groups {
        if *count == 0 {
            continue;
        }
        let n_f = *count as f64;
        if let Some(g) = lookup_benson_group(name) {
            dh_gas_kcal += n_f * g.dh_kcal;
            s_int_cal += n_f * g.s_cal;
            for (k, v) in g.cp_cal.iter().enumerate() {
                cp_grid_cal[k] += n_f * v;
            }
            sum_tb += n_f * g.joback_tb;
            sum_tm += n_f * g.joback_tm;
            sum_cpa_liq_corr += n_f * g.joback_cpa;
            entries.push(BensonGroupEntry {
                group_name: name.clone(),
                count: *count,
                dh_kcal_each: g.dh_kcal,
                s_cal_each: g.s_cal,
                cp_cal_300_each: g.cp_cal[0],
                is_estimated_fallback: false,
            });
        } else {
            return Err(format!("Faltan parámetros tabulados para el grupo de Benson '{name}'; no se puede calcular una estimación respaldada por datos."));
        }
    }

    let sigma = params.sigma_tot.max(1.0);
    let n_opt = params.n_opt.max(1) as f64;
    let s_gas_cal = s_int_cal - R_CAL * sigma.ln() + R_CAL * n_opt.ln();

    // Convert Benson Cp grid to J/(mol·K) and fit 4-parameter Kelley polynomial
    let cp_grid_j_k: Vec<(f64, f64)> = BENSON_CP_TEMPS_K
        .iter()
        .zip(cp_grid_cal.iter())
        .map(|(&t, &c)| (t, c * CAL_TO_JOULE))
        .collect();
    let gas_kelley = fit_kelley_cp(&cp_grid_j_k);
    let cp_gas_298 = gas_kelley.cp(T_REF_K).max(0.0);

    let dh_gas_j = dh_gas_kcal * CAL_TO_JOULE * 1000.0;
    let s_gas_j = s_gas_cal * CAL_TO_JOULE;

    let gas_est = PhaseThermoSummary {
        delta_h_f_j: dh_gas_j,
        s_j_k: s_gas_j,
        cp_j_k: cp_gas_298,
    };

    // Estimate Liquid & Solid phases via Joback Tb/Tm + Trouton/Walden enthalpy & entropy of phase change
    let has_hbond = active_groups
        .iter()
        .any(|(k, c)| *c > 0 && (k.starts_with("O-(H)") || k.starts_with("N-(H)")));
    let tb_est_k = (198.2 + sum_tb).max(120.0);
    let tm_est_k = (122.5 + sum_tm).clamp(80.0, tb_est_k - 10.0);

    // Trouton entropy of vaporization (~88 J/(mol*K), +18 J/(mol*K) for H-bonding -OH/-NH2)
    let ds_vap_tb = if has_hbond { 106.0 } else { 88.0 };
    let dh_vap_tb = tb_est_k * ds_vap_tb;
    // Watson correlation scaling to 298.15 K
    let tc_est = (tb_est_k / 0.64).max(tb_est_k + 80.0);
    let dh_vap_298 = if tb_est_k > 298.15 {
        let ratio = ((1.0 - 298.15 / tc_est) / (1.0 - tb_est_k / tc_est)).clamp(0.8, 1.35);
        dh_vap_tb * ratio.powf(0.38)
    } else {
        dh_vap_tb * 0.90
    };
    let ds_vap_298 = (dh_vap_tb / tb_est_k) + (if tb_est_k > 298.15 { 12.0 } else { 0.0 });
    let cp_liq_298 = (cp_gas_298 * 1.48 + if has_hbond { 22.0 } else { 8.0 } + 0.05 * sum_cpa_liq_corr).max(cp_gas_298 + 10.0);

    let liquid_est = PhaseThermoSummary {
        delta_h_f_j: dh_gas_j - dh_vap_298,
        s_j_k: (s_gas_j - ds_vap_298).max(10.0),
        cp_j_k: cp_liq_298,
    };

    // Walden / Yalkowsky fusion: ΔS_fus ≈ 44 J/(mol*K) (rigid aromatics slightly higher, flexible slightly lower)
    let ds_fus = 48.0;
    let dh_fus = tm_est_k * ds_fus;
    let cp_solid_298 = (cp_liq_298 * 0.76).max(20.0);
    let solid_est = PhaseThermoSummary {
        delta_h_f_j: liquid_est.delta_h_f_j - dh_fus,
        s_j_k: (liquid_est.s_j_k - ds_fus - 4.0).max(5.0),
        cp_j_k: cp_solid_298,
    };

    let formula = mol.molecular_formula();
    let (gas_db, liquid_db, solid_db, db_matches) = lookup_catalog_phases(&formula, catalog);

    let common_name = if mol.common_name.is_empty() {
        mol.name.clone()
    } else {
        mol.common_name.clone()
    };

    Ok(BensonEstimationResult {
        formula,
        common_name,
        chemical_name: mol.name.clone(),
        molecular_weight: mol.molecular_weight(),
        groups: entries,
        warnings,
        params_used: params,
        gas_est,
        liquid_est,
        solid_est,
        gas_db,
        liquid_db,
        solid_db,
        db_matches,
        gas_kelley,
        cp_grid_j_k,
        tm_est_k,
        tb_est_k,
    })
}

fn lookup_catalog_phases(
    formula: &str,
    catalog: Option<&dyn SpeciesCatalogPort>,
) -> (
    Option<PhaseThermoSummary>,
    Option<PhaseThermoSummary>,
    Option<PhaseThermoSummary>,
    Vec<String>,
) {
    let Some(cat) = catalog else {
        return (None, None, None, Vec::new());
    };
    if formula.is_empty() {
        return (None, None, None, Vec::new());
    }
    let candidates = cat.search_species(formula, 40);
    let mut gas = None;
    let mut liq = None;
    let mut sol = None;
    let mut matches = Vec::new();
    for sp in candidates {
        // Match exact formula or `Formula(...)`
        let base = sp.formula.split('(').next().unwrap_or(&sp.formula).trim();
        if base != formula {
            continue;
        }
        matches.push(sp.formula.clone());
        if let Ok((pt, _)) = evaluate_species_si(&sp, T_REF_K, false, false) {
            let sum = PhaseThermoSummary {
                delta_h_f_j: pt.h,
                s_j_k: pt.s,
                cp_j_k: pt.cp,
            };
            match sp.phase {
                Phase::Gas if gas.is_none() => gas = Some(sum),
                Phase::Liquid if liq.is_none() => liq = Some(sum),
                Phase::Solid if sol.is_none() => sol = Some(sum),
                _ => {}
            }
        }
    }
    (gas, liq, sol, matches)
}

/// Build a `ChemicalSpecies` record ready to persist via `SpeciesCatalogPort::insert_or_update_species`.
pub fn build_benson_chemical_species(
    est: &BensonEstimationResult,
    phase: Phase,
    custom_formula: &str,
) -> ChemicalSpecies {
    let summary = match phase {
        Phase::Gas => est.gas_est,
        Phase::Liquid => est.liquid_est,
        Phase::Solid | Phase::AqueousIon | Phase::AqueousNeutral => est.solid_est,
    };
    let kelley = if phase == Phase::Gas {
        est.gas_kelley
    } else {
        KelleyCoefficients {
            a: summary.cp_j_k,
            b: 0.0,
            c: 0.0,
            d: 0.0,
        }
    };
    let clean = custom_formula.trim();
    let tagged_formula = if clean.is_empty() {
        format!("{}(BEst)", est.formula)
    } else if clean.contains('(') {
        clean.to_string()
    } else {
        format!("{clean}(BEst)")
    };

    ChemicalSpecies {
        formula: tagged_formula,
        name: if est.common_name.is_empty() {
            est.chemical_name.clone()
        } else {
            est.common_name.clone()
        },
        phase,
        molecular_weight: est.molecular_weight,
        density_g_cm3: 1.0,
        charge: 0,
        intervals: vec![SpeciesThermoInterval {
            t_min: T_REF_K,
            t_max: if phase == Phase::Gas { 1500.0 } else { est.tb_est_k.max(400.0) },
            phase,
            kelley,
            h_or_delta_h_tr: summary.delta_h_f_j,
            s_or_delta_s_tr: summary.s_j_k,
            reference: "BEst (Soryntel Benson GAV)".to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_kcal(j: f64) -> f64 {
        j / (CAL_TO_JOULE * 1000.0)
    }

    fn to_cal_k(j_k: f64) -> f64 {
        j_k / CAL_TO_JOULE
    }

    #[test]
    fn benson_validation_cases_b1_to_b8() {
        struct Case<'a> {
            smiles: &'a str,
            expected_dh_kcal: f64,
            expected_s_cal: f64,
            tol_h: f64,
            tol_s: f64,
        }
        let cases = [
            Case { smiles: "CCCC", expected_dh_kcal: -30.0, expected_s_cal: 74.1, tol_h: 1.5, tol_s: 1.5 },
            Case { smiles: "CC(C)C", expected_dh_kcal: -32.1, expected_s_cal: 70.4, tol_h: 1.5, tol_s: 1.5 },
            Case { smiles: "c1ccccc1", expected_dh_kcal: 19.8, expected_s_cal: 64.3, tol_h: 1.5, tol_s: 1.5 },
            Case { smiles: "Cc1ccccc1", expected_dh_kcal: 12.0, expected_s_cal: 76.6, tol_h: 1.5, tol_s: 2.0 },
            Case { smiles: "CCO", expected_dh_kcal: -56.2, expected_s_cal: 67.5, tol_h: 1.5, tol_s: 2.0 },
            Case { smiles: "CC(=O)O", expected_dh_kcal: -103.3, expected_s_cal: 67.5, tol_h: 2.0, tol_s: 2.5 },
            Case { smiles: "C1CCCCC1", expected_dh_kcal: -29.5, expected_s_cal: 71.3, tol_h: 1.5, tol_s: 2.0 },
        ];

        for c in &cases {
            let mol = MoleculeGraph::from_smiles(c.smiles).expect(c.smiles);
            let res = estimate_benson(&mol, None, None, None).expect(c.smiles);
            let dh = to_kcal(res.gas_est.delta_h_f_j);
            let s = to_cal_k(res.gas_est.s_j_k);
            assert!(
                (dh - c.expected_dh_kcal).abs() <= c.tol_h,
                "SMILES {} ΔH = {:.2} kcal/mol (expected {:.2})",
                c.smiles,
                dh,
                c.expected_dh_kcal
            );
            assert!(
                (s - c.expected_s_cal).abs() <= c.tol_s,
                "SMILES {} S = {:.2} cal/molK (expected {:.2})",
                c.smiles,
                s,
                c.expected_s_cal
            );
            // Case B8: Kelley Cp fit accuracy across 300..1500 K
            for &(t, cp_ref) in &res.cp_grid_j_k {
                let err = (res.gas_kelley.cp(t) - cp_ref).abs();
                assert!(
                    err < 2.5,
                    "Kelley Cp fit error {:.2} >= 2.5 J/(mol·K) at T={t}K for {}",
                    err,
                    c.smiles
                );
            }
        }
    }

    #[test]
    fn benzene_to_4_methylaniline_click_build_workflow() {
        // Reproduces Fig. 6 -> Fig. 7 -> Fig. 8 -> Fig. 10 workflow
        let mut mol = MoleculeGraph::from_smiles("c1ccccc1").unwrap();
        assert_eq!(mol.molecular_formula(), "C6H6");
        assert!((mol.molecular_weight() - 78.11).abs() < 0.1);

        // Substitute first aromatic H with Carbon -> Toluene (C7H8, MW ~92.14)
        let h1 = mol
            .atoms
            .iter()
            .position(|a| a.element == ElementKind::H)
            .unwrap();
        mol.substitute_atom_with_element(h1, ElementKind::C).unwrap();
        assert_eq!(mol.molecular_formula(), "C7H8");
        assert!((mol.molecular_weight() - 92.14).abs() < 0.1);

        // Substitute another ring H with Nitrogen -> 4-Methylaniline (C7H9N, MW ~107.15)
        let h2 = mol
            .atoms
            .iter()
            .enumerate()
            .find(|&(idx, a)| {
                a.element == ElementKind::H
                    && mol.neighbors(idx).first().map(|&p| mol.atoms[p].aromatic).unwrap_or(false)
            })
            .unwrap()
            .0;
        mol.substitute_atom_with_element(h2, ElementKind::N).unwrap();
        assert_eq!(mol.molecular_formula(), "C7H9N");
        assert!((mol.molecular_weight() - 107.15).abs() < 0.1);

        let est = estimate_benson(&mol, None, None, None).unwrap();
        // Compare against Fig. 10: Gas ΔHf° ≈ 54 kJ/mol, Cp ≈ 130 J/(mol*K)
        assert!((est.gas_est.delta_h_f_j / 1000.0 - 54.1).abs() < 2.5);
        assert!((est.gas_est.cp_j_k - 131.0).abs() < 5.0);
    }

    #[test]
    fn anthracene_dimethanol_and_coronene_formulas() {
        let spec = find_structural_block("aro_diol_ant").unwrap();
        let mol = MoleculeGraph::from_block(spec);
        assert_eq!(mol.molecular_formula(), "C16H14O2");
        assert!((mol.molecular_weight() - 238.28).abs() < 0.1);

        let coronene = find_structural_block("poly_coronene").unwrap();
        let cor = MoleculeGraph::from_block(coronene);
        assert_eq!(cor.molecular_formula(), "C24H12");
    }
}
