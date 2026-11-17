//! Soryntel DB: Layered thermodynamic species repository (`OwnDB > MainDB > OpenDefault`),
//! full `.HSC` / `.REF` / `.CAT` codecs, and stoichiometric / elemental / keyword search.

pub mod hsc_binary;

use hsc_binary::{
    build_empty_hsc_bytes, detect_and_merge_datasets, discover_companion_ref, export_datasets_csv,
    group_records_to_datasets, import_datasets_csv, parse_hsc_binary_records, parse_ref_binary,
    serialize_datasets_to_hsc, upgrade_legacy_datasets, write_catalogue_file, ReferenceEntry,
};
use soryntel_core::{SoryntelError, CAL_TO_JOULE};
use soryntel_thermo::species::{
    formula_sort_key, parse_hsc_formula_elements, stoichiometry_matches, DbMergeConflict, DbSource,
    ElementSearchMode, KeywordSearchField, MergeConflictDecision, PhaseFilterFlags,
    SpeciesCatalogEntry, SpeciesDataset,
};
use soryntel_thermo::{
    ChemicalSpecies, KelleyCoefficients, Phase, SpeciesMetadata, SpeciesRepository,
    SpeciesThermoInterval,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Summary statistics for the active databases (`OwnDB`, `MainDB`, `MainDB6.REF`).
#[derive(Debug, Clone, Default)]
pub struct DatabaseStatusSummary {
    pub main_path: Option<PathBuf>,
    pub own_path: Option<PathBuf>,
    pub ref_path: Option<PathBuf>,
    pub main_unique_species: usize,
    pub main_binary_records: usize,
    pub own_unique_species: usize,
    pub own_binary_records: usize,
    pub resolved_total_species: usize,
    pub references_count: usize,
}

/// In-memory layered thermodynamic database implementing `SpeciesRepository` & `SpeciesCatalogPort`.
///
/// Resolution precedence mirrors HSC Chemistry 6 Ch. 28.17 strictly:
/// **`OwnDB` > `MainDB` > `OpenDefault`**.
#[derive(Clone, Debug, Default)]
pub struct ThermoDatabase {
    /// Active resolved layer used by calculation modules (`Reaction`, `Balance`, `Equilibrium`, etc.).
    species: HashMap<String, ChemicalSpecies>,
    metadata: HashMap<String, SpeciesMetadata>,
    resolved_source: HashMap<String, DbSource>,
    resolved_raw_phase: HashMap<String, String>,
    ordered_formulas: Vec<String>,
    /// Pre-parsed elemental compositions keyed by formula for sub-millisecond stoichiometry/element queries.
    composition_cache: HashMap<String, BTreeMap<String, f64>>,

    /// Three-layer storage preserving alternative temperature datasets per formula.
    open_defaults: HashMap<String, SpeciesDataset>,
    main_datasets: HashMap<String, Vec<SpeciesDataset>>,
    main_order: Vec<String>,
    main_binary_records: usize,
    main_preserve_open_defaults: bool,

    own_datasets: HashMap<String, Vec<SpeciesDataset>>,
    own_order: Vec<String>,
    own_binary_records: usize,

    /// Active dataset index per `(DbSource, formula)` when a formula has multiple literature sets.
    active_dataset_choice: HashMap<(DbSource, String), usize>,

    /// Bibliographic dictionary loaded from `MainDB6.REF` (`key -> citations`).
    references: HashMap<String, Vec<ReferenceEntry>>,
    reference_keys_ordered: Vec<String>,
    references_total: usize,

    /// Active `.HSC` / `.REF` file paths.
    main_db_path: Option<PathBuf>,
    own_db_path: Option<PathBuf>,
    ref_db_path: Option<PathBuf>,
}

impl ThermoDatabase {
    /// Create a database pre-populated with open-literature reference species (NIST/IUPAC/Barin).
    pub fn with_open_defaults() -> Self {
        let mut db = Self::default();
        for sp in open_reference_species() {
            let meta = default_metadata_for(&sp.formula);
            let raw_ph = hsc_binary::phase_to_hsc_code(sp.phase, "").to_string();
            let ds = SpeciesDataset {
                source: DbSource::OpenDefault,
                metadata: meta,
                species: sp,
                dataset_index: 0,
                raw_phase_code: raw_ph,
            };
            db.open_defaults.insert(ds.species.formula.clone(), ds);
        }
        db.rebuild_resolved();
        db
    }

    /// Rebuild `species`, `metadata`, `resolved_source`, `ordered_formulas`, and `composition_cache`
    /// enforcing **`OwnDB > MainDB > OpenDefault`** precedence.
    fn rebuild_resolved(&mut self) {
        self.species.clear();
        self.metadata.clear();
        self.resolved_source.clear();
        self.resolved_raw_phase.clear();
        self.ordered_formulas.clear();
        self.composition_cache.clear();

        let mut ordered_layers: Vec<SpeciesDataset> = Vec::new();
        let mut seen = BTreeSet::new();

        // Apply MainDB first (or after OpenDefault depending on legacy `overwrite` flag)
        if self.main_preserve_open_defaults {
            for (f, ds) in &self.open_defaults {
                ordered_layers.push(ds.clone());
                seen.insert(f.clone());
            }
            for f in &self.main_order {
                if !seen.contains(f) {
                    if let Some(ds) = self.pick_active_dataset(DbSource::Main, f) {
                        ordered_layers.push(ds);
                        seen.insert(f.clone());
                    }
                }
            }
        } else {
            for f in &self.main_order {
                if let Some(ds) = self.pick_active_dataset(DbSource::Main, f) {
                    ordered_layers.push(ds);
                    seen.insert(f.clone());
                }
            }
            for (f, ds) in &self.open_defaults {
                if !seen.contains(f) {
                    ordered_layers.push(ds.clone());
                    seen.insert(f.clone());
                }
            }
        }

        for ds in ordered_layers {
            self.insert_resolved_dataset(&ds);
        }

        // OwnDB always has highest precedence over MainDB & OpenDefault
        let own_layers: Vec<(bool, SpeciesDataset)> = self
            .own_order
            .iter()
            .filter_map(|f| {
                self.pick_active_dataset(DbSource::Own, f)
                    .map(|ds| (seen.insert(f.clone()), ds))
            })
            .collect();
        for (is_new, ds) in own_layers {
            if is_new {
                self.ordered_formulas.push(ds.species.formula.clone());
            }
            self.insert_resolved_dataset_preserve_order(&ds);
        }
    }

    fn insert_resolved_dataset(&mut self, ds: &SpeciesDataset) {
        let f = &ds.species.formula;
        if !self.species.contains_key(f) {
            self.ordered_formulas.push(f.clone());
        }
        self.insert_resolved_dataset_preserve_order(ds);
    }

    fn insert_resolved_dataset_preserve_order(&mut self, ds: &SpeciesDataset) {
        let f = ds.species.formula.clone();
        if let Ok((elems, _)) = parse_hsc_formula_elements(&f) {
            self.composition_cache.insert(f.clone(), elems);
        }
        self.species.insert(f.clone(), ds.species.clone());
        self.metadata.insert(f.clone(), ds.metadata.clone());
        self.resolved_source.insert(f.clone(), ds.source);
        self.resolved_raw_phase.insert(f, ds.raw_phase_code.clone());
    }

    fn pick_active_dataset(&self, source: DbSource, formula: &str) -> Option<SpeciesDataset> {
        let list = match source {
            DbSource::Main => self.main_datasets.get(formula)?,
            DbSource::Own => self.own_datasets.get(formula)?,
            DbSource::OpenDefault => return self.open_defaults.get(formula).cloned(),
        };
        let idx = self
            .active_dataset_choice
            .get(&(source, formula.to_string()))
            .copied()
            .unwrap_or(0);
        list.get(idx).or_else(|| list.first()).cloned()
    }

    /// Insert or update a species in the high-priority `OwnDB` layer so all calculation modules
    /// immediately observe the edited record.
    pub fn insert_species(&mut self, sp: ChemicalSpecies) {
        let meta = self
            .metadata
            .get(&sp.formula)
            .cloned()
            .unwrap_or_else(|| default_metadata_for(&sp.formula));
        self.insert_species_with_metadata(sp, meta);
    }

    /// Insert or update a species together with its extended `.HSC` metadata in `OwnDB`.
    pub fn insert_species_with_metadata(&mut self, sp: ChemicalSpecies, meta: SpeciesMetadata) {
        let raw_ph = hsc_binary::phase_to_hsc_code(sp.phase, "").to_string();
        self.upsert_own_dataset(sp, meta, raw_ph);
    }

    /// Insert or update a complete `OwnDB` dataset (replaces primary OwnDB dataset for `sp.formula`).
    pub fn upsert_own_dataset(
        &mut self,
        sp: ChemicalSpecies,
        meta: SpeciesMetadata,
        raw_phase_code: String,
    ) {
        let f = sp.formula.clone();
        if !self.own_datasets.contains_key(&f) {
            self.own_order.push(f.clone());
        }
        let ds = SpeciesDataset {
            source: DbSource::Own,
            species: sp,
            metadata: meta,
            dataset_index: 0,
            raw_phase_code,
        };
        self.own_datasets.insert(f.clone(), vec![ds.clone()]);
        self.active_dataset_choice.insert((DbSource::Own, f.clone()), 0);
        if !self.species.contains_key(&f) {
            self.ordered_formulas.push(f.clone());
        }
        self.insert_resolved_dataset_preserve_order(&ds);
        self.own_binary_records = self
            .own_datasets
            .values()
            .flatten()
            .map(|d| d.species.intervals.len())
            .sum();
    }

    /// Delete a species from `OwnDB` (`CU-05` / `CU-09`). If `MainDB` has an entry with the same
    /// formula, active calculations revert transparently to `MainDB`.
    pub fn delete_own_species(&mut self, formula: &str) -> bool {
        let existed = self.own_datasets.remove(formula).is_some();
        if existed {
            self.own_order.retain(|f| f != formula);
            self.active_dataset_choice.remove(&(DbSource::Own, formula.to_string()));
            self.own_binary_records = self
                .own_datasets
                .values()
                .flatten()
                .map(|d| d.species.intervals.len())
                .sum();
            self.rebuild_resolved();
        }
        existed
    }

    /// Switch which literature dataset (`0..dataset_count`) is active for a formula in `source`.
    pub fn select_dataset(&mut self, source: DbSource, formula: &str, dataset_index: usize) -> bool {
        let valid = match source {
            DbSource::Main => self
                .main_datasets
                .get(formula)
                .map(|v| dataset_index < v.len())
                .unwrap_or(false),
            DbSource::Own => self
                .own_datasets
                .get(formula)
                .map(|v| dataset_index < v.len())
                .unwrap_or(false),
            DbSource::OpenDefault => false,
        };
        if valid {
            self.active_dataset_choice
                .insert((source, formula.to_string()), dataset_index);
            self.rebuild_resolved();
        }
        valid
    }

    /// Return all temperature datasets (including discontinuous alternative literature blocks)
    /// stored across layers for `formula`.
    pub fn get_all_datasets_for(&self, source: DbSource, formula: &str) -> Vec<SpeciesDataset> {
        match source {
            DbSource::Main => self.main_datasets.get(formula).cloned().unwrap_or_default(),
            DbSource::Own => self.own_datasets.get(formula).cloned().unwrap_or_default(),
            DbSource::OpenDefault => self
                .open_defaults
                .get(formula)
                .cloned()
                .into_iter()
                .collect(),
        }
    }

    /// Return source layer (`OwnDB`, `MainDB`, `Built-in`) currently resolved for `formula`.
    pub fn resolved_source_of(&self, formula: &str) -> Option<DbSource> {
        self.resolved_source.get(formula).copied()
    }

    /// Legacy-compatible MainDB read-only loader.
    ///
    /// When `overwrite == false`, existing `OpenDefault` species are kept for backwards regression parity
    /// while all `Maindb` species and alternative datasets are still indexed cleanly; when `overwrite == true`,
    /// `MainDB` supersedes `OpenDefault` (while still yielding to `OwnDB`).
    pub fn load_external_hsc_readonly<P: AsRef<Path>>(
        &mut self,
        path: P,
        overwrite: bool,
    ) -> Result<usize, SoryntelError> {
        let buf = fs::read(path.as_ref()).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot open {}: {e}", path.as_ref().display()))
        })?;
        let (records, n_recs) = parse_hsc_binary_records(&buf)?;
        let (order, grouped) = group_records_to_datasets(records, DbSource::Main);

        self.main_preserve_open_defaults = !overwrite;
        self.main_order = order;
        self.main_datasets = grouped;
        self.main_binary_records = n_recs;
        self.main_db_path = Some(path.as_ref().to_path_buf());

        if let Some(ref_path) = discover_companion_ref(path.as_ref()) {
            let _ = self.load_ref_dictionary(&ref_path);
        }

        self.rebuild_resolved();
        Ok(self.main_datasets.len())
    }

    /// Load active `MainDB` (`.HSC`) strictly superseding built-in defaults (`OwnDB > MainDB`).
    pub fn load_main_hsc<P: AsRef<Path>>(&mut self, path: P) -> Result<(usize, usize), SoryntelError> {
        let count = self.load_external_hsc_readonly(path, true)?;
        Ok((count, self.main_binary_records))
    }

    /// Load active `OwnDB` (`.HSC`, overwriting any prior in-memory `OwnDB` edits).
    pub fn load_own_hsc<P: AsRef<Path>>(&mut self, path: P) -> Result<(usize, usize), SoryntelError> {
        let buf = fs::read(path.as_ref()).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot open OwnDB {}: {e}", path.as_ref().display()))
        })?;
        let (records, n_recs) = parse_hsc_binary_records(&buf)?;
        let (order, grouped) = group_records_to_datasets(records, DbSource::Own);

        self.own_order = order;
        self.own_datasets = grouped;
        self.own_binary_records = n_recs;
        self.own_db_path = Some(path.as_ref().to_path_buf());
        self.rebuild_resolved();
        Ok((self.own_datasets.len(), n_recs))
    }

    /// Persist active `OwnDB` to a binary 256-byte `.HSC` file (plus refreshed `.CAT` index).
    pub fn save_own_hsc<P: AsRef<Path>>(&mut self, path: P) -> Result<usize, SoryntelError> {
        let ordered = self.collect_flat_datasets(DbSource::Own);
        let bytes = serialize_datasets_to_hsc(&ordered);
        fs::write(path.as_ref(), &bytes).map_err(|e| {
            SoryntelError::DatabaseError(format!(
                "Cannot write OwnDB {}: {e}",
                path.as_ref().display()
            ))
        })?;
        self.own_db_path = Some(path.as_ref().to_path_buf());
        let _ = write_catalogue_file(path.as_ref(), &ordered);
        Ok(ordered.len())
    }

    /// Create a fresh empty `OwnDB` `.HSC` file (valid 256-byte `'0'` sentinel matching `OWNDB6.HSC`).
    pub fn create_empty_own_hsc<P: AsRef<Path>>(&mut self, path: P) -> Result<(), SoryntelError> {
        let sentinel = build_empty_hsc_bytes();
        fs::write(path.as_ref(), &sentinel).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot create {}: {e}", path.as_ref().display()))
        })?;
        self.own_order.clear();
        self.own_datasets.clear();
        self.own_binary_records = 0;
        self.own_db_path = Some(path.as_ref().to_path_buf());
        self.rebuild_resolved();
        let _ = write_catalogue_file(path.as_ref(), &[]);
        Ok(())
    }

    /// Save active `MainDB` (as a custom copy or updated `.HSC`) to `path`.
    pub fn save_main_hsc_copy<P: AsRef<Path>>(&self, path: P) -> Result<usize, SoryntelError> {
        let ordered = self.collect_flat_datasets(DbSource::Main);
        let bytes = serialize_datasets_to_hsc(&ordered);
        fs::write(path.as_ref(), &bytes).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot write {}: {e}", path.as_ref().display()))
        })?;
        let _ = write_catalogue_file(path.as_ref(), &ordered);
        Ok(ordered.len())
    }

    /// Refresh `.CAT` index file next to the active `MainDB` or `OwnDB` `.HSC` file.
    pub fn refresh_catalogue(&self, source: DbSource) -> Result<PathBuf, SoryntelError> {
        let path = match source {
            DbSource::Own => self.own_db_path.as_ref(),
            DbSource::Main => self.main_db_path.as_ref(),
            DbSource::OpenDefault => None,
        }
        .ok_or_else(|| {
            SoryntelError::DatabaseError(format!("No active file path set for {}", source.label()))
        })?;
        let flat = self.collect_flat_datasets(source);
        write_catalogue_file(path, &flat)
    }

    /// Sort `OwnDB` or `MainDB` entries alphabetically / HSC element descending order (`CU-09` `Sort`).
    pub fn sort_database(&mut self, source: DbSource) {
        match source {
            DbSource::Own => {
                self.own_order
                    .sort_by(|a, b| formula_sort_key(a).cmp(&formula_sort_key(b)));
            }
            DbSource::Main => {
                self.main_order
                    .sort_by(|a, b| formula_sort_key(a).cmp(&formula_sort_key(b)));
            }
            DbSource::OpenDefault => {}
        }
        self.rebuild_resolved();
    }

    /// Load `MainDB6.REF` (`293`-byte bibliographic citation records).
    pub fn load_ref_dictionary<P: AsRef<Path>>(&mut self, path: P) -> Result<usize, SoryntelError> {
        let buf = fs::read(path.as_ref()).map_err(|e| {
            SoryntelError::DatabaseError(format!(
                "Cannot read reference file {}: {e}",
                path.as_ref().display()
            ))
        })?;
        let entries = parse_ref_binary(&buf);
        self.references.clear();
        self.reference_keys_ordered.clear();
        self.references_total = entries.len();
        for e in entries {
            let k_lower = e.key.to_ascii_lowercase();
            if !self.references.contains_key(&k_lower) {
                self.reference_keys_ordered.push(k_lower.clone());
            }
            self.references.entry(k_lower).or_default().push(e);
        }
        self.ref_db_path = Some(path.as_ref().to_path_buf());
        Ok(self.references_total)
    }

    /// Lookup citation entries referenced in a comma-separated interval `Reference` field
    /// (e.g. `"Barin 89, SGTE 94, Landolt 00"`).
    pub fn lookup_citations(&self, ref_field: &str) -> Vec<ReferenceEntry> {
        let mut out = Vec::new();
        for token in ref_field.split([',', ';']) {
            let k = token.trim().to_ascii_lowercase();
            if k.is_empty() {
                continue;
            }
            if let Some(entries) = self.references.get(&k) {
                out.extend(entries.iter().cloned());
            } else if let Some((_, entries)) =
                self.references.iter().find(|(rk, _)| rk.starts_with(&k))
            {
                // Partial match fallback for truncated 36-char `.HSC` reference field
                out.extend(entries.iter().cloned());
            }
        }
        out
    }

    /// Search `MainDB6.REF` citation dictionary by key or full author/journal text.
    pub fn search_references(&self, query: &str, limit: usize) -> Vec<ReferenceEntry> {
        let q = query.trim().to_ascii_lowercase();
        let mut res: Vec<ReferenceEntry> = self
            .reference_keys_ordered
            .iter()
            .filter_map(|k| self.references.get(k))
            .flatten()
            .filter(|e| {
                q.is_empty()
                    || e.key.to_ascii_lowercase().contains(&q)
                    || e.citation.to_ascii_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        res.truncate(limit);
        res
    }

    /// Inspect and/or merge `src_path` (`HSC1`) into `dst_path` (`HSC2`), writing sorted HSC 6.0 bytes.
    pub fn merge_hsc_files<P: AsRef<Path>, Q: AsRef<Path>>(
        src_path: P,
        dst_path: Q,
        decisions: &BTreeMap<String, MergeConflictDecision>,
        dry_run_conflicts_only: bool,
    ) -> Result<(usize, Vec<DbMergeConflict>), SoryntelError> {
        let src_bytes = fs::read(src_path.as_ref()).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot read {}: {e}", src_path.as_ref().display()))
        })?;
        let dst_bytes = if dst_path.as_ref().exists() {
            fs::read(dst_path.as_ref()).map_err(|e| {
                SoryntelError::DatabaseError(format!("Cannot read {}: {e}", dst_path.as_ref().display()))
            })?
        } else {
            build_empty_hsc_bytes()
        };

        let (src_recs, _) = parse_hsc_binary_records(&src_bytes)?;
        let (dst_recs, _) = parse_hsc_binary_records(&dst_bytes)?;
        let (s_ord, s_map) = group_records_to_datasets(src_recs, DbSource::Own);
        let (d_ord, d_map) = group_records_to_datasets(dst_recs, DbSource::Own);

        let src_flat: Vec<SpeciesDataset> =
            s_ord.into_iter().filter_map(|f| s_map.get(&f).cloned()).flatten().collect();
        let dst_flat: Vec<SpeciesDataset> =
            d_ord.into_iter().filter_map(|f| d_map.get(&f).cloned()).flatten().collect();

        let (merged, conflicts) = detect_and_merge_datasets(&src_flat, &dst_flat, decisions);
        if !dry_run_conflicts_only {
            let out_bytes = serialize_datasets_to_hsc(&merged);
            fs::write(dst_path.as_ref(), &out_bytes).map_err(|e| {
                SoryntelError::DatabaseError(format!(
                    "Cannot write {}: {e}",
                    dst_path.as_ref().display()
                ))
            })?;
            let _ = write_catalogue_file(dst_path.as_ref(), &merged);
        }
        Ok((merged.len(), conflicts))
    }

    /// Convert an older HSC 2.0–5.0 `.HSC` or CSV database to canonical sorted HSC 6.0 `.HSC` + `.CAT`.
    pub fn convert_legacy_to_hsc6<P: AsRef<Path>, Q: AsRef<Path>>(
        src_path: P,
        dst_path: Q,
    ) -> Result<usize, SoryntelError> {
        let bytes = fs::read(src_path.as_ref()).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot read {}: {e}", src_path.as_ref().display()))
        })?;
        let datasets = if bytes.len() >= 256 && bytes.len() % 256 == 0 {
            let (recs, _) = parse_hsc_binary_records(&bytes)?;
            let (ord, map) = group_records_to_datasets(recs, DbSource::Main);
            ord.into_iter().filter_map(|f| map.get(&f).cloned()).flatten().collect()
        } else {
            let txt = String::from_utf8_lossy(&bytes);
            import_datasets_csv(&txt, DbSource::Main)?
        };
        let upgraded = upgrade_legacy_datasets(datasets);
        let out_bytes = serialize_datasets_to_hsc(&upgraded);
        fs::write(dst_path.as_ref(), &out_bytes).map_err(|e| {
            SoryntelError::DatabaseError(format!("Cannot write {}: {e}", dst_path.as_ref().display()))
        })?;
        let _ = write_catalogue_file(dst_path.as_ref(), &upgraded);
        Ok(upgraded.len())
    }

    /// Export `source` database layer as UTF-8 CSV (`export_datasets_csv`).
    pub fn export_csv(&self, source: Option<DbSource>) -> String {
        let flat = match source {
            Some(s) => self.collect_flat_datasets(s),
            None => self.collect_resolved_datasets(),
        };
        export_datasets_csv(&flat)
    }

    /// Export `source` database layer as pretty-printed JSON.
    pub fn export_json(&self, source: Option<DbSource>) -> Result<String, SoryntelError> {
        let flat = match source {
            Some(s) => self.collect_flat_datasets(s),
            None => self.collect_resolved_datasets(),
        };
        serde_json::to_string_pretty(&flat)
            .map_err(|e| SoryntelError::DatabaseError(format!("JSON export failed: {e}")))
    }

    /// Import CSV species rows into `OwnDB`.
    pub fn import_csv_into_own(&mut self, csv_text: &str) -> Result<usize, SoryntelError> {
        let imported = import_datasets_csv(csv_text, DbSource::Own)?;
        let count = imported.len();
        for ds in imported {
            self.upsert_own_dataset(ds.species, ds.metadata, ds.raw_phase_code);
        }
        Ok(count)
    }

    /// Return database summary statistics and active file paths.
    pub fn status_summary(&self) -> DatabaseStatusSummary {
        DatabaseStatusSummary {
            main_path: self.main_db_path.clone(),
            own_path: self.own_db_path.clone(),
            ref_path: self.ref_db_path.clone(),
            main_unique_species: self.main_datasets.len(),
            main_binary_records: self.main_binary_records,
            own_unique_species: self.own_datasets.len(),
            own_binary_records: self.own_binary_records,
            resolved_total_species: self.species.len(),
            references_count: self.references_total,
        }
    }

    /// Collect flat ordered entries from one layer or the resolved view.
    fn collect_flat_datasets(&self, source: DbSource) -> Vec<SpeciesDataset> {
        match source {
            DbSource::Own => self
                .own_order
                .iter()
                .filter_map(|f| self.own_datasets.get(f))
                .flatten()
                .cloned()
                .collect(),
            DbSource::Main => self
                .main_order
                .iter()
                .filter_map(|f| self.main_datasets.get(f))
                .flatten()
                .cloned()
                .collect(),
            DbSource::OpenDefault => self
                .ordered_formulas
                .iter()
                .filter_map(|f| self.open_defaults.get(f))
                .cloned()
                .collect(),
        }
    }

    fn collect_resolved_datasets(&self) -> Vec<SpeciesDataset> {
        self.ordered_formulas
            .iter()
            .filter_map(|f| {
                let sp = self.species.get(f)?.clone();
                let metadata = self.metadata.get(f).cloned().unwrap_or_default();
                let source = self.resolved_source.get(f).copied().unwrap_or(DbSource::OpenDefault);
                let raw_phase_code = self.resolved_raw_phase.get(f).cloned().unwrap_or_default();
                Some(SpeciesDataset {
                    source,
                    species: sp,
                    metadata,
                    dataset_index: 0,
                    raw_phase_code,
                })
            })
            .collect()
    }

    fn make_entry_for(&self, formula: &str, source_filter: Option<DbSource>) -> Option<SpeciesCatalogEntry> {
        match source_filter {
            None => {
                let sp = self.species.get(formula)?.clone();
                let meta = self.metadata.get(formula).cloned().unwrap_or_default();
                let source = self
                    .resolved_source
                    .get(formula)
                    .copied()
                    .unwrap_or(DbSource::OpenDefault);
                let raw_phase_code = self
                    .resolved_raw_phase
                    .get(formula)
                    .cloned()
                    .unwrap_or_else(|| hsc_binary::phase_to_hsc_code(sp.phase, "").into());
                let dataset_count = match source {
                    DbSource::Main => self.main_datasets.get(formula).map(|v| v.len()).unwrap_or(1),
                    DbSource::Own => self.own_datasets.get(formula).map(|v| v.len()).unwrap_or(1),
                    DbSource::OpenDefault => 1,
                };
                Some(SpeciesCatalogEntry {
                    formula: formula.to_string(),
                    source,
                    species: sp,
                    metadata: meta,
                    dataset_count,
                    raw_phase_code,
                })
            }
            Some(src) => {
                let ds = self.pick_active_dataset(src, formula)?;
                let dataset_count = self.get_all_datasets_for(src, formula).len();
                Some(SpeciesCatalogEntry {
                    formula: formula.to_string(),
                    source: src,
                    species: ds.species,
                    metadata: ds.metadata,
                    dataset_count,
                    raw_phase_code: ds.raw_phase_code,
                })
            }
        }
    }

    fn entry_matches_filters(&self, entry: &SpeciesCatalogEntry, filters: &PhaseFilterFlags) -> bool {
        let is_ai = entry.raw_phase_code == "ai" || entry.species.formula.ends_with("(ia)");
        if !filters.matches_phase(entry.species.phase, is_ai) {
            return false;
        }
        if let Some(elems) = self.composition_cache.get(&entry.formula) {
            filters.matches_composition(elems)
        } else if let Ok((elems, _)) = parse_hsc_formula_elements(&entry.formula) {
            filters.matches_composition(&elems)
        } else {
            true
        }
    }

    fn formula_list_for_layer(&self, source_filter: Option<DbSource>) -> &[String] {
        match source_filter {
            None | Some(DbSource::OpenDefault) => &self.ordered_formulas,
            Some(DbSource::Main) => &self.main_order,
            Some(DbSource::Own) => &self.own_order,
        }
    }

    /// `CU-03` / `CU-04`: List & search by keyword across `Formula`, `StructuralFormula`, `ChemicalName`,
    /// `CommonName`, `CAS`, or `Any`, honoring source layer and phase/organic filters.
    pub fn search_advanced(
        &self,
        query: &str,
        field: KeywordSearchField,
        source_filter: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        let q = query.trim().to_ascii_lowercase();
        let formulas = self.formula_list_for_layer(source_filter);
        let mut out = Vec::new();
        for f in formulas {
            let Some(entry) = self.make_entry_for(f, source_filter) else {
                continue;
            };
            if !self.entry_matches_filters(&entry, filters) {
                continue;
            }
            let text_ok = if q.is_empty() {
                true
            } else {
                match field {
                    KeywordSearchField::Formula => entry.formula.to_ascii_lowercase().contains(&q),
                    KeywordSearchField::StructuralFormula => {
                        entry.metadata.structural_formula.to_ascii_lowercase().contains(&q)
                    }
                    KeywordSearchField::ChemicalName => {
                        entry.species.name.to_ascii_lowercase().contains(&q)
                    }
                    KeywordSearchField::CommonName => {
                        entry.metadata.common_name.to_ascii_lowercase().contains(&q)
                    }
                    KeywordSearchField::CasNumber => {
                        entry.metadata.cas_number.to_ascii_lowercase().contains(&q)
                    }
                    KeywordSearchField::Any => {
                        entry.formula.to_ascii_lowercase().contains(&q)
                            || entry.species.name.to_ascii_lowercase().contains(&q)
                            || entry.metadata.common_name.to_ascii_lowercase().contains(&q)
                            || entry.metadata.structural_formula.to_ascii_lowercase().contains(&q)
                            || entry.metadata.cas_number.to_ascii_lowercase().contains(&q)
                    }
                }
            };
            if text_ok {
                out.push(entry);
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    /// `CU-01`: Search by selected elements using `Some` (species whose elements are a subset of selected)
    /// or `All` (species containing every selected element), plus `Organic` & carbon-count limits (`28.5`).
    pub fn search_elements_filtered(
        &self,
        elements: &[String],
        mode: ElementSearchMode,
        source_filter: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Vec<SpeciesCatalogEntry> {
        let target: BTreeSet<String> = elements
            .iter()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty())
            .collect();
        if target.is_empty() {
            return Vec::new();
        }
        let formulas = self.formula_list_for_layer(source_filter);
        let mut out = Vec::new();
        for f in formulas {
            let Some(entry) = self.make_entry_for(f, source_filter) else {
                continue;
            };
            if !self.entry_matches_filters(&entry, filters) {
                continue;
            }
            let parsed = self
                .composition_cache
                .get(&entry.formula)
                .cloned()
                .or_else(|| parse_hsc_formula_elements(&entry.formula).ok().map(|(m, _)| m));
            let Some(elems) = parsed else {
                continue;
            };
            let matches = match mode {
                ElementSearchMode::Some => {
                    let mut has_real = false;
                    let mut subset = true;
                    for k in elems.keys() {
                        if k == "e-" {
                            continue;
                        }
                        has_real = true;
                        if !target.contains(k) {
                            subset = false;
                            break;
                        }
                    }
                    has_real && subset
                }
                ElementSearchMode::All => target.iter().all(|req| elems.contains_key(req)),
            };
            if matches {
                out.push(entry);
                if out.len() >= limit {
                    break;
                }
            }
        }
        out
    }

    /// `CU-02`: Stoichiometric search (`Find Stoichiometry`, `Fix Ratio ON/OFF` per `28.6`).
    pub fn search_stoichiometry(
        &self,
        query_formula: &str,
        fix_ratio: bool,
        source_filter: Option<DbSource>,
        filters: &PhaseFilterFlags,
        limit: usize,
    ) -> Result<Vec<SpeciesCatalogEntry>, SoryntelError> {
        let (target_elems, _) = parse_hsc_formula_elements(query_formula)?;
        let formulas = self.formula_list_for_layer(source_filter);
        let mut out = Vec::new();
        for f in formulas {
            let Some(entry) = self.make_entry_for(f, source_filter) else {
                continue;
            };
            if !self.entry_matches_filters(&entry, filters) {
                continue;
            }
            let cand = self
                .composition_cache
                .get(&entry.formula)
                .cloned()
                .or_else(|| parse_hsc_formula_elements(&entry.formula).ok().map(|(m, _)| m));
            if let Some(elems) = cand {
                if stoichiometry_matches(&elems, &target_elems, fix_ratio) {
                    out.push(entry);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<ChemicalSpecies> {
        let q = query.trim().to_lowercase();
        let mut res: Vec<ChemicalSpecies> = self
            .species
            .values()
            .filter(|sp| {
                q.is_empty()
                    || sp.formula.to_lowercase().contains(&q)
                    || sp.name.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        res.sort_by(|a, b| a.formula.cmp(&b.formula));
        res.truncate(limit);
        res
    }

    /// Searches all species whose constituent elements are a subset of `elements`
    /// (matching HSC's `From Elements` -> `Possible Species` filter).
    pub fn search_by_elements(
        &self,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        let flags = PhaseFilterFlags {
            solid: include_solid,
            liquid: include_liquid,
            gas: include_gas,
            aqua_ions: include_aqua_ions,
            aqua_neutrals: include_aqua_neutrals,
            include_ai: true,
            include_ao: true,
            organic: true,
            carbon_counts: Vec::new(),
        };
        let mut res: Vec<ChemicalSpecies> = self
            .search_elements_filtered(elements, ElementSearchMode::Some, None, &flags, usize::MAX)
            .into_iter()
            .map(|e| e.species)
            .collect();
        res.sort_by(|a, b| a.formula.cmp(&b.formula));
        res.truncate(limit);
        res
    }

    pub fn len(&self) -> usize {
        self.species.len()
    }

    pub fn is_empty(&self) -> bool {
        self.species.is_empty()
    }
}

impl SpeciesRepository for ThermoDatabase {
    fn get_species(&self, formula: &str) -> Option<ChemicalSpecies> {
        self.species.get(formula).cloned()
    }

    fn get_reference_element(&self, element_symbol: &str) -> Option<ChemicalSpecies> {
        let ref_formula = match element_symbol {
            "H" => "H2(g)",
            "O" => "O2(g)",
            "N" => "N2(g)",
            "Cl" => "Cl2(g)",
            "F" => "F2(g)",
            "Br" => "Br2(l)",
            "I" => "I2",
            other => other,
        };
        self.species.get(ref_formula).cloned()
    }
}

impl soryntel_thermo::SpeciesCatalogPort for ThermoDatabase {
    fn search_species(&self, query: &str, limit: usize) -> Vec<ChemicalSpecies> {
        self.search(query, limit)
    }

    fn search_species_by_elements(
        &self,
        elements: &[String],
        include_gas: bool,
        include_liquid: bool,
        include_solid: bool,
        include_aqua_ions: bool,
        include_aqua_neutrals: bool,
        limit: usize,
    ) -> Vec<ChemicalSpecies> {
        self.search_by_elements(
            elements,
            include_gas,
            include_liquid,
            include_solid,
            include_aqua_ions,
            include_aqua_neutrals,
            limit,
        )
    }

    fn insert_or_update_species(&mut self, sp: ChemicalSpecies) {
        self.insert_species(sp);
    }

    fn species_count(&self) -> usize {
        self.len()
    }

    fn get_species_metadata(&self, formula: &str) -> Option<SpeciesMetadata> {
        self.metadata.get(formula).cloned()
    }

    fn get_species_at(&self, index: usize) -> Option<ChemicalSpecies> {
        let f = self.ordered_formulas.get(index)?;
        self.species.get(f).cloned()
    }
}

fn default_metadata_for(formula: &str) -> SpeciesMetadata {
    match formula {
        "Al2O3" => SpeciesMetadata {
            structural_formula: "Al2O3".into(),
            common_name: "Corundum".into(),
            cas_number: "1344-28-1".into(),
            melting_point_k: 2327.0,
            boiling_point_k: 3253.0,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "TiO2" => SpeciesMetadata {
            structural_formula: "O=Ti=O".into(),
            common_name: "Rutile".into(),
            cas_number: "13463-67-7".into(),
            melting_point_k: 2116.0,
            boiling_point_k: 3245.0,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "Mg" => SpeciesMetadata {
            structural_formula: "Mg".into(),
            common_name: "Magnesium".into(),
            cas_number: "7439-95-4".into(),
            melting_point_k: 923.0,
            boiling_point_k: 1363.0,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "Ni" => SpeciesMetadata {
            structural_formula: "Ni".into(),
            common_name: "Nickel".into(),
            cas_number: "7440-02-0".into(),
            melting_point_k: 1728.0,
            boiling_point_k: 3186.0,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "CO(g)" => SpeciesMetadata {
            structural_formula: "C#O".into(),
            common_name: "Carbon monoxide".into(),
            cas_number: "630-08-0".into(),
            melting_point_k: 68.15,
            boiling_point_k: 81.65,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "O2(g)" => SpeciesMetadata {
            structural_formula: "O=O".into(),
            common_name: "Oxygen".into(),
            cas_number: "7782-44-7".into(),
            melting_point_k: 54.36,
            boiling_point_k: 90.19,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        "H2(g)" => SpeciesMetadata {
            structural_formula: "H-H".into(),
            common_name: "Hydrogen".into(),
            cas_number: "1333-74-0".into(),
            melting_point_k: 13.99,
            boiling_point_k: 20.27,
            color_code: 0,
            solubility: "-".into(),
            reliability_class: 1,
        },
        _ => SpeciesMetadata {
            structural_formula: formula.into(),
            solubility: "-".into(),
            reliability_class: 1,
            ..SpeciesMetadata::default()
        },
    }
}

fn cal_interval(
    t_min: f64,
    t_max: f64,
    phase: Phase,
    h_kcal: f64,
    s_cal: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    reference: &str,
) -> SpeciesThermoInterval {
    SpeciesThermoInterval {
        t_min,
        t_max,
        phase,
        kelley: KelleyCoefficients {
            a: a * CAL_TO_JOULE,
            b: b * CAL_TO_JOULE,
            c: c * CAL_TO_JOULE,
            d: d * CAL_TO_JOULE,
        },
        h_or_delta_h_tr: h_kcal * 1000.0 * CAL_TO_JOULE,
        s_or_delta_s_tr: s_cal * CAL_TO_JOULE,
        reference: reference.to_string(),
    }
}

/// Built-in open reference dataset for immediate standalone execution & regression tests.
fn open_reference_species() -> Vec<ChemicalSpecies> {
    vec![
        ChemicalSpecies {
            formula: "Al2O3".into(),
            name: "Aluminum oxide (corundum)".into(),
            phase: Phase::Solid,
            molecular_weight: 101.961,
            density_g_cm3: 3.965,
            charge: 0,
            intervals: vec![
                cal_interval(100.0, 800.0, Phase::Solid, -400.500, 12.177, 2.337, 70.441, -0.594, -47.365, "Barin 77, JANAF 98"),
                cal_interval(800.0, 2327.0, Phase::Solid, 0.0, 0.0, 27.719, 3.741, -10.585, -0.564, "Barin 77"),
                cal_interval(2327.0, 4000.0, Phase::Liquid, 26.550, 11.410, 46.000, 0.0, 0.0, 0.0, "JANAF 98"),
            ],
        },
        ChemicalSpecies {
            formula: "TiO2".into(),
            name: "Titanium(IV) oxide (Rutile)".into(),
            phase: Phase::Solid,
            molecular_weight: 79.865,
            density_g_cm3: 4.230,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 2143.0, Phase::Solid, -225.800, 12.020, 16.777, 1.810, -3.690, -0.364, "Barin 93, Saxena 93"),
                cal_interval(2143.0, 3000.0, Phase::Liquid, 16.000, 7.512, 24.000, 0.0, 0.0, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "Mg".into(),
            name: "Magnesium".into(),
            phase: Phase::Solid,
            molecular_weight: 24.305,
            density_g_cm3: 1.740,
            charge: 0,
            intervals: vec![
                cal_interval(20.0, 60.0, Phase::Solid, 0.0, 7.776, -1.791, 61.379, 0.003, -8.513, "Landolt 99, NASA 93"),
                cal_interval(60.0, 100.0, Phase::Solid, 0.0, 0.0, -6.085, 156.797, 0.030, -614.579, "NASA 93"),
                cal_interval(100.0, 180.0, Phase::Solid, 0.0, 0.0, 3.523, 17.250, -0.119, -31.676, "NASA 93"),
                cal_interval(180.0, 298.15, Phase::Solid, 0.0, 0.0, 5.947, 0.142, -0.266, 2.597, "NASA 93"),
                cal_interval(298.15, 923.0, Phase::Solid, 0.0, 0.0, 6.801, -1.541, -0.593, 2.852, "NASA 93"),
                cal_interval(923.0, 6000.0, Phase::Liquid, 2.026, 2.195, 8.198, 0.0, 0.0, 0.0, "NASA 93, Barin 77"),
            ],
        },
        ChemicalSpecies {
            formula: "Mg(g)".into(),
            name: "Magnesium gas".into(),
            phase: Phase::Gas,
            molecular_weight: 24.305,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 3000.0, Phase::Gas, 35.158, 35.527, 5.011, -0.065, -0.026, 0.022, "Landolt 99"),
            ],
        },
        ChemicalSpecies {
            formula: "Zn".into(),
            name: "Zinc".into(),
            phase: Phase::Solid,
            molecular_weight: 65.380,
            density_g_cm3: 7.140,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 692.68, Phase::Solid, 0.0, 9.950, 5.350, 2.400, 0.0, 0.0, "Barin 93"),
                cal_interval(692.68, 2500.0, Phase::Liquid, 1.750, 2.526, 7.500, 0.0, 0.0, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "Zn(g)".into(),
            name: "Zinc gas".into(),
            phase: Phase::Gas,
            molecular_weight: 65.380,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 3000.0, Phase::Gas, 31.170, 38.450, 4.968, 0.0, 0.0, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "Cd".into(),
            name: "Cadmium".into(),
            phase: Phase::Solid,
            molecular_weight: 112.411,
            density_g_cm3: 8.650,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 594.22, Phase::Solid, 0.0, 12.380, 5.310, 2.940, 0.0, 0.0, "Barin 93"),
                cal_interval(594.22, 2500.0, Phase::Liquid, 1.480, 2.491, 7.100, 0.0, 0.0, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "Cd(g)".into(),
            name: "Cadmium gas".into(),
            phase: Phase::Gas,
            molecular_weight: 112.411,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 3000.0, Phase::Gas, 26.720, 40.065, 4.968, 0.0, 0.0, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "Ni".into(),
            name: "Nickel".into(),
            phase: Phase::Solid,
            molecular_weight: 58.693,
            density_g_cm3: 8.908,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 631.0, Phase::Solid, 0.0, 7.137, 4.060, 7.040, 0.0, 0.0, "JANAF 98"),
                cal_interval(631.0, 1728.0, Phase::Solid, 0.0, 0.0, 6.000, 1.800, 0.0, 0.0, "JANAF 98"),
            ],
        },
        ChemicalSpecies {
            formula: "CO(g)".into(),
            name: "Carbon monoxide".into(),
            phase: Phase::Gas,
            molecular_weight: 28.010,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 2500.0, Phase::Gas, -26.417, 47.214, 6.790, 0.980, -0.110, 0.0, "JANAF 98"),
            ],
        },
        ChemicalSpecies {
            formula: "Ni(CO)4(g)".into(),
            name: "Nickel tetracarbonyl gas".into(),
            phase: Phase::Gas,
            molecular_weight: 170.735,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 1000.0, Phase::Gas, -144.100, 98.100, 34.700, 14.500, -4.000, 0.0, "Barin 93"),
            ],
        },
        ChemicalSpecies {
            formula: "O2(g)".into(),
            name: "Oxygen gas".into(),
            phase: Phase::Gas,
            molecular_weight: 31.999,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 3000.0, Phase::Gas, 0.0, 49.003, 7.160, 1.000, -0.400, 0.0, "JANAF 98"),
            ],
        },
        ChemicalSpecies {
            formula: "H2(g)".into(),
            name: "Hydrogen gas".into(),
            phase: Phase::Gas,
            molecular_weight: 2.016,
            density_g_cm3: 0.0,
            charge: 0,
            intervals: vec![
                cal_interval(298.15, 3000.0, Phase::Gas, 0.0, 31.207, 6.520, 0.780, 0.120, 0.0, "JANAF 98"),
            ],
        },
        ChemicalSpecies {
            formula: "H(+a)".into(),
            name: "Hydrogen ion (aq)".into(),
            phase: Phase::AqueousIon,
            molecular_weight: 1.008,
            density_g_cm3: 0.0,
            charge: 1,
            intervals: vec![
                cal_interval(298.15, 573.15, Phase::AqueousIon, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, "IUPAC / NBS 82"),
            ],
        },
        ChemicalSpecies {
            formula: "e-".into(),
            name: "Electron".into(),
            phase: Phase::AqueousIon,
            molecular_weight: 0.00054858,
            density_g_cm3: 0.0,
            charge: -1,
            intervals: vec![
                cal_interval(298.15, 5000.0, Phase::AqueousIon, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, "IUPAC SHE"),
            ],
        },
    ]
}
