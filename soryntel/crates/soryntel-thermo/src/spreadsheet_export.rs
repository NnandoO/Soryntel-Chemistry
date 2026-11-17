//! Cross-suite spreadsheet exporter and add-in package generator.
//!
//! Generates:
//! - Valid ECMA-376 `.xlsx` workbooks (with both Excel formulas and cached values)
//! - Valid OASIS OpenDocument `.ods` workbooks for LibreOffice Calc / Collabora / ONLYOFFICE
//! - LibreOffice Calc `.oxt` extension package (Python UNO / StarBasic bridge)
//! - ONLYOFFICE Desktop/Docs plugin bundle (`.zip`: `config.json`, `index.html`, `code.js`)
//! - Microsoft Excel & WPS Office portable VBA module (`.bas`)
//!
//! Uses an embedded, zero-dependency PKZIP Stored (method 0) + IEEE 802.3 CRC32 writer.

use crate::excel_addins::{AddInOrigin, ADDIN_FUNCTION_CATALOG};
use std::collections::BTreeMap;

/// Cell value representation for cross-suite workbook export.
#[derive(Debug, Clone)]
pub enum ExportCellValue {
    Empty,
    Text(String),
    Number(f64),
    Formula {
        formula: String,
        cached_number: Option<f64>,
        cached_text: Option<String>,
        array_rows: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportCellStyle {
    Default = 0,
    Header = 1,
    InputRed = 2,
    BoldBox = 3,
    Olive = 4,
    Green = 5,
    Teal = 6,
    YellowNote = 7,
}

#[derive(Debug, Clone)]
pub struct ExportCell {
    pub row: usize, // 0-based
    pub col: usize, // 0-based
    pub value: ExportCellValue,
    pub style: ExportCellStyle,
}

#[derive(Debug, Clone)]
pub struct ExportSheet {
    pub name: String,
    pub cells: Vec<ExportCell>,
    pub col_widths: Vec<f64>,
}

/// Minimal PKZIP archive builder (Stored method 0) with standard CRC32 IEEE.
pub struct ZipBuilder {
    entries: Vec<(String, Vec<u8>, u32)>,
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: impl Into<String>, data: impl Into<Vec<u8>>) {
        let bytes = data.into();
        let crc = crc32_ieee(&bytes);
        self.entries.push((path.into(), bytes, crc));
    }

    pub fn finish(self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        let mut offset = 0u32;

        for (name, data, crc) in &self.entries {
            let nb = name.as_bytes();
            let sz = data.len() as u32;
            let nlen = nb.len() as u16;

            // Local file header
            out.extend_from_slice(&0x04034b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0u16.to_le_bytes()); // flags
            out.extend_from_slice(&0u16.to_le_bytes()); // method 0 = Stored
            out.extend_from_slice(&0u16.to_le_bytes()); // mod time
            out.extend_from_slice(&0x5421u16.to_le_bytes()); // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&sz.to_le_bytes());
            out.extend_from_slice(&sz.to_le_bytes());
            out.extend_from_slice(&nlen.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra len
            out.extend_from_slice(nb);
            out.extend_from_slice(data);

            // Central directory header
            central.extend_from_slice(&0x02014b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes());
            central.extend_from_slice(&0x5421u16.to_le_bytes());
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&sz.to_le_bytes());
            central.extend_from_slice(&sz.to_le_bytes());
            central.extend_from_slice(&nlen.to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra
            central.extend_from_slice(&0u16.to_le_bytes()); // comment
            central.extend_from_slice(&0u16.to_le_bytes()); // disk start
            central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(nb);

            offset = out.len() as u32;
        }

        let cd_offset = out.len() as u32;
        let cd_size = central.len() as u32;
        let count = self.entries.len() as u16;
        out.extend_from_slice(&central);

        // End of central directory record
        out.extend_from_slice(&0x06054b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&count.to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }
}

fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320u32 & mask);
        }
    }
    !crc
}

pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() && c != '\n' && c != '\r' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

pub fn col_to_a1(col: usize) -> String {
    let mut c = col + 1;
    let mut out = String::new();
    while c > 0 {
        let rem = (c - 1) % 26;
        out.insert(0, (b'A' + rem as u8) as char);
        c = (c - 1) / 26;
    }
    out
}

fn normalize_excel_formula(raw: &str) -> String {
    let s = raw.trim().trim_start_matches('=');
    // OpenXML requires comma `,` argument separators inside <f>...</f> (localized UI adapts automatically).
    let mut out = String::with_capacity(s.len());
    let mut in_quote = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quote = !in_quote;
            out.push(ch);
        } else if ch == ';' && !in_quote {
            out.push(',');
        } else {
            out.push(ch);
        }
    }
    out
}

/// Build an ECMA-376 `.xlsx` workbook compatible with Excel, LibreOffice Calc, ONLYOFFICE, and WPS.
pub fn build_xlsx(sheets: &[ExportSheet]) -> Vec<u8> {
    let mut zip = ZipBuilder::new();

    // [Content_Types].xml
    let mut ct = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
"#,
    );
    for i in 0..sheets.len() {
        ct.push_str(&format!(
            "  <Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\n",
            i + 1
        ));
    }
    ct.push_str("</Types>");
    zip.add_file("[Content_Types].xml", ct.into_bytes());

    // _rels/.rels
    zip.add_file(
        "_rels/.rels",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
            .as_bytes()
            .to_vec(),
    );

    // xl/workbook.xml & xl/_rels/workbook.xml.rels
    let mut wb = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
"#,
    );
    let mut wbrels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#,
    );
    for (i, sh) in sheets.iter().enumerate() {
        let sid = i + 1;
        wb.push_str(&format!(
            "    <sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>\n",
            xml_escape(&sh.name),
            sid,
            sid
        ));
        wbrels.push_str(&format!(
            "  <Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{}.xml\"/>\n",
            sid, sid
        ));
    }
    wbrels.push_str(&format!(
        "  <Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>\n</Relationships>",
        sheets.len() + 1
    ));
    wb.push_str("  </sheets>\n</workbook>");
    zip.add_file("xl/workbook.xml", wb.into_bytes());
    zip.add_file("xl/_rels/workbook.xml.rels", wbrels.into_bytes());

    // xl/styles.xml (8 styles matching ExportCellStyle 0..7)
    let styles = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="4">
    <font><sz val="10"/><name val="Calibri"/></font>
    <font><b/><sz val="10"/><name val="Calibri"/></font>
    <font><sz val="10"/><color rgb="FFCC0000"/><name val="Calibri"/></font>
    <font><b/><sz val="10"/><color rgb="FFFFFFFF"/><name val="Calibri"/></font>
  </fonts>
  <fills count="8">
    <fill><patternFill patternType="none"/></fill>
    <fill><patternFill patternType="gray125"/></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FFEAF1FB"/></patternFill></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FF808000"/></patternFill></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FF168232"/></patternFill></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FF1E78B4"/></patternFill></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FFFFFFC0"/></patternFill></fill>
    <fill><patternFill patternType="solid"><fgColor rgb="FFF3F3F3"/></patternFill></fill>
  </fills>
  <borders count="2">
    <border><left/><right/><top/><bottom/><diagonal/></border>
    <border><left style="thin"/><right style="thin"/><top style="thin"/><bottom style="thin"/><diagonal/></border>
  </borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs count="8">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0"/>
    <xf numFmtId="0" fontId="1" fillId="2" borderId="1" xfId="0" applyFont="1" applyFill="1"/>
    <xf numFmtId="0" fontId="2" fillId="0" borderId="1" xfId="0" applyFont="1"/>
    <xf numFmtId="0" fontId="1" fillId="7" borderId="1" xfId="0" applyFont="1" applyFill="1"/>
    <xf numFmtId="0" fontId="3" fillId="3" borderId="1" xfId="0" applyFont="1" applyFill="1"/>
    <xf numFmtId="0" fontId="3" fillId="4" borderId="1" xfId="0" applyFont="1" applyFill="1"/>
    <xf numFmtId="0" fontId="3" fillId="5" borderId="1" xfId="0" applyFont="1" applyFill="1"/>
    <xf numFmtId="0" fontId="0" fillId="6" borderId="1" xfId="0" applyFill="1"/>
  </cellXfs>
</styleSheet>"#;
    zip.add_file("xl/styles.xml", styles.as_bytes().to_vec());

    // Worksheets
    for (idx, sh) in sheets.iter().enumerate() {
        let mut by_row: BTreeMap<usize, Vec<&ExportCell>> = BTreeMap::new();
        for c in &sh.cells {
            by_row.entry(c.row).or_default().push(c);
        }
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
"#,
        );
        if !sh.col_widths.is_empty() {
            xml.push_str("  <cols>\n");
            for (ci, w) in sh.col_widths.iter().enumerate() {
                xml.push_str(&format!(
                    "    <col min=\"{}\" max=\"{}\" width=\"{:.1}\" customWidth=\"1\"/>\n",
                    ci + 1,
                    ci + 1,
                    w.max(6.0)
                ));
            }
            xml.push_str("  </cols>\n");
        }
        xml.push_str("  <sheetData>\n");
        for (r, mut rcells) in by_row {
            rcells.sort_by_key(|c| c.col);
            xml.push_str(&format!("    <row r=\"{}\">\n", r + 1));
            for c in rcells {
                let rref = format!("{}{}", col_to_a1(c.col), c.row + 1);
                let sty = c.style as usize;
                match &c.value {
                    ExportCellValue::Empty => {}
                    ExportCellValue::Number(v) => {
                        if v.is_finite() {
                            xml.push_str(&format!(
                                "      <c r=\"{rref}\" s=\"{sty}\"><v>{v}</v></c>\n"
                            ));
                        }
                    }
                    ExportCellValue::Text(s) => {
                        xml.push_str(&format!(
                            "      <c r=\"{rref}\" s=\"{sty}\" t=\"inlineStr\"><is><t>{}</t></is></c>\n",
                            xml_escape(s)
                        ));
                    }
                    ExportCellValue::Formula {
                        formula,
                        cached_number,
                        cached_text,
                        array_rows,
                    } => {
                        let fx = xml_escape(&normalize_excel_formula(formula));
                        let arr_attr = if *array_rows > 1 {
                            let end_ref = format!("{}{}", col_to_a1(c.col), c.row + *array_rows);
                            format!(" t=\"array\" ref=\"{rref}:{end_ref}\"")
                        } else {
                            String::new()
                        };
                        if let Some(num) = cached_number.filter(|v| v.is_finite()) {
                            xml.push_str(&format!(
                                "      <c r=\"{rref}\" s=\"{sty}\"><f{arr_attr}>{fx}</f><v>{num}</v></c>\n"
                            ));
                        } else if let Some(txt) = cached_text {
                            let is_std_err = matches!(
                                txt.as_str(),
                                "#DIV/0!" | "#N/A" | "#NAME?" | "#NULL!" | "#NUM!" | "#REF!" | "#VALUE!"
                            );
                            let c_type = if is_std_err { "e" } else { "str" };
                            xml.push_str(&format!(
                                "      <c r=\"{rref}\" s=\"{sty}\" t=\"{c_type}\"><f{arr_attr}>{fx}</f><v>{}</v></c>\n",
                                xml_escape(txt)
                            ));
                        } else {
                            xml.push_str(&format!(
                                "      <c r=\"{rref}\" s=\"{sty}\"><f{arr_attr}>{fx}</f></c>\n"
                            ));
                        }
                    }
                }
            }
            xml.push_str("    </row>\n");
        }
        xml.push_str("  </sheetData>\n</worksheet>");
        zip.add_file(format!("xl/worksheets/sheet{}.xml", idx + 1), xml.into_bytes());
    }

    zip.finish()
}

fn to_openformula_refs(formula_body: &str) -> String {
    let mut out = String::with_capacity(formula_body.len() + 16);
    let chars: Vec<char> = formula_body.chars().collect();
    let n = chars.len();
    let mut i = 0usize;
    let mut in_quote = false;

    let parse_cell_token = |start: usize| -> Option<(usize, String)> {
        let mut j = start;
        let mut tok = String::new();
        if j < n && chars[j] == '$' {
            tok.push('$');
            j += 1;
        }
        let col_start = j;
        while j < n && chars[j].is_ascii_alphabetic() && (j - col_start) < 3 {
            tok.push(chars[j]);
            j += 1;
        }
        if j == col_start {
            return None;
        }
        if j < n && chars[j] == '$' {
            tok.push('$');
            j += 1;
        }
        let row_start = j;
        while j < n && chars[j].is_ascii_digit() && (j - row_start) < 6 {
            tok.push(chars[j]);
            j += 1;
        }
        if j == row_start {
            return None;
        }
        if j < n && (chars[j].is_ascii_alphanumeric() || chars[j] == '(' || chars[j] == '.') {
            return None;
        }
        Some((j, tok))
    };

    while i < n {
        let ch = chars[i];
        if ch == '"' {
            in_quote = !in_quote;
            out.push(ch);
            i += 1;
            continue;
        }
        if in_quote {
            out.push(ch);
            i += 1;
            continue;
        }
        if ch == ',' {
            out.push(';');
            i += 1;
            continue;
        }
        let prev_is_word = i > 0 && (chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '.');
        if !prev_is_word {
            if let Some((j, lhs)) = parse_cell_token(i) {
                if j < n && chars[j] == ':' {
                    if let Some((k, rhs)) = parse_cell_token(j + 1) {
                        out.push_str(&format!("[.{lhs}:.{rhs}]"));
                        i = k;
                        continue;
                    }
                }
                out.push_str(&format!("[.{lhs}]"));
                i = j;
                continue;
            }
        }
        out.push(ch);
        i += 1;
    }
    out
}

/// Build an OASIS OpenDocument `.ods` workbook compatible with LibreOffice Calc, Collabora, ONLYOFFICE, and WPS.
pub fn build_ods(sheets: &[ExportSheet]) -> Vec<u8> {
    let mut zip = ZipBuilder::new();
    // OASIS ODF spec requires mimetype as the very first uncompressed entry.
    zip.add_file(
        "mimetype",
        b"application/vnd.oasis.opendocument.spreadsheet".to_vec(),
    );
    zip.add_file(
        "META-INF/manifest.xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
  <manifest:file-entry manifest:full-path="/" manifest:version="1.2" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
  <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
  <manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#
            .to_vec(),
    );
    zip.add_file(
        "styles.xml",
        br##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
  office:version="1.2">
  <office:styles>
    <style:default-style style:family="table-cell">
      <style:text-properties fo:font-family="Arial" fo:font-size="10pt" fo:color="#111827"/>
    </style:default-style>
  </office:styles>
</office:document-styles>"##
            .to_vec(),
    );

    let mut content = String::from(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content
  xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
  xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
  xmlns:of="urn:oasis:names:tc:opendocument:xmlns:of:1.2"
  office:version="1.2">
  <office:automatic-styles>
    <style:style style:name="ce0" style:family="table-cell">
      <style:text-properties fo:color="#111827" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce1" style:family="table-cell">
      <style:table-cell-properties fo:background-color="#E8ECF2" fo:border="0.5pt solid #B4BDC9"/>
      <style:text-properties fo:color="#0F172A" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce2" style:family="table-cell">
      <style:text-properties fo:color="#C00000" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce3" style:family="table-cell">
      <style:table-cell-properties fo:border="0.5pt solid #B4BDC9"/>
      <style:text-properties fo:color="#1F2937" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce4" style:family="table-cell">
      <style:table-cell-properties fo:background-color="#F3F6E8" fo:border="0.5pt solid #B4BDC9"/>
      <style:text-properties fo:color="#111827" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce5" style:family="table-cell">
      <style:table-cell-properties fo:background-color="#E9F5E9" fo:border="0.5pt solid #B4BDC9"/>
      <style:text-properties fo:color="#111827" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce6" style:family="table-cell">
      <style:table-cell-properties fo:background-color="#E6F3F6" fo:border="0.5pt solid #B4BDC9"/>
      <style:text-properties fo:color="#111827" fo:font-weight="bold" fo:font-size="10pt"/>
    </style:style>
    <style:style style:name="ce7" style:family="table-cell">
      <style:table-cell-properties fo:background-color="#FFFDE0" fo:border="0.5pt solid #D1D5DB"/>
      <style:text-properties fo:color="#111827" fo:font-size="9.5pt"/>
    </style:style>
  </office:automatic-styles>
  <office:body>
    <office:spreadsheet>
"##,
    );
    for sh in sheets {
        content.push_str(&format!(
            "      <table:table table:name=\"{}\">\n",
            xml_escape(&sh.name)
        ));
        let mut grid: BTreeMap<usize, BTreeMap<usize, &ExportCell>> = BTreeMap::new();
        let mut max_row = 0usize;
        let mut max_col = 0usize;
        for c in &sh.cells {
            max_row = max_row.max(c.row);
            max_col = max_col.max(c.col);
            grid.entry(c.row).or_default().insert(c.col, c);
        }
        content.push_str(&format!(
            "        <table:table-column table:number-columns-repeated=\"{}\"/>\n",
            max_col + 1
        ));
        for r in 0..=max_row {
            content.push_str("        <table:table-row>\n");
            let row_map = grid.get(&r);
            for col in 0..=max_col {
                if let Some(cell) = row_map.and_then(|m| m.get(&col)) {
                    let sty = cell.style as usize;
                    let s_attr = format!(" table:style-name=\"ce{sty}\"");
                    match &cell.value {
                        ExportCellValue::Empty => {
                            content.push_str(&format!("          <table:table-cell{s_attr}/>\n"));
                        }
                        ExportCellValue::Number(v) => {
                            content.push_str(&format!(
                                "          <table:table-cell{s_attr} office:value-type=\"float\" office:value=\"{v}\"><text:p>{v}</text:p></table:table-cell>\n"
                            ));
                        }
                        ExportCellValue::Text(s) => {
                            let xs = xml_escape(s);
                            content.push_str(&format!(
                                "          <table:table-cell{s_attr} office:value-type=\"string\"><text:p>{xs}</text:p></table:table-cell>\n"
                            ));
                        }
                        ExportCellValue::Formula {
                            formula,
                            cached_number,
                            cached_text,
                            array_rows,
                        } => {
                            let of_body = to_openformula_refs(formula.trim().trim_start_matches('='));
                            let fx = xml_escape(&format!("of:={of_body}"));
                            let matrix_attr = if *array_rows > 1 {
                                format!(" table:number-matrix-rows-spanned=\"{array_rows}\" table:number-matrix-columns-spanned=\"1\"")
                            } else {
                                String::new()
                            };
                            if let Some(num) = cached_number.filter(|v| v.is_finite()) {
                                content.push_str(&format!(
                                    "          <table:table-cell{s_attr} table:formula=\"{fx}\"{matrix_attr} office:value-type=\"float\" office:value=\"{num}\"><text:p>{num}</text:p></table:table-cell>\n"
                                ));
                            } else if let Some(txt) = cached_text {
                                let xt = xml_escape(txt);
                                content.push_str(&format!(
                                    "          <table:table-cell{s_attr} table:formula=\"{fx}\"{matrix_attr} office:value-type=\"string\" office:string-value=\"{xt}\"><text:p>{xt}</text:p></table:table-cell>\n"
                                ));
                            } else {
                                content.push_str(&format!(
                                    "          <table:table-cell{s_attr} table:formula=\"{fx}\"{matrix_attr}/>\n"
                                ));
                            }
                        }
                    }
                } else {
                    content.push_str("          <table:table-cell/>\n");
                }
            }
            content.push_str("        </table:table-row>\n");
        }
        content.push_str("      </table:table>\n");
    }
    content.push_str(
        "    </office:spreadsheet>\n  </office:body>\n</office:document-content>",
    );
    zip.add_file("content.xml", content.into_bytes());
    zip.finish()
}

#[derive(Clone, Copy)]
enum UdfParamKind {
    Str,
    Num,
    Range,
    OptionalVariant,
}

#[derive(Clone, Copy)]
struct UdfSpec {
    params: &'static [(&'static str, UdfParamKind)],
    volatile: bool,
    is_array_streameq: bool,
}

fn udf_spec_for(fn_name: &str) -> UdfSpec {
    use UdfParamKind::*;
    match fn_name.to_ascii_uppercase().as_str() {
        "ENERGYU" | "TEMPU" | "PRESU" => UdfSpec {
            params: &[],
            volatile: true,
            is_array_streameq: false,
        },
        "UNITS" => UdfSpec {
            params: &[
                ("TUnit", Str),
                ("EUnit", Str),
                ("PUnit", OptionalVariant),
            ],
            volatile: true,
            is_array_streameq: false,
        },
        "BAL" => UdfSpec {
            params: &[("Equation", Str), ("Mult", OptionalVariant)],
            volatile: false,
            is_array_streameq: false,
        },
        "SPECIES" => UdfSpec {
            params: &[("DBNo", Num), ("Pos", Num)],
            volatile: false,
            is_array_streameq: false,
        },
        "H" | "HKG" | "HNM3" | "HCM" | "HLAT" | "S" | "CP" | "G" | "K" | "TATH"
        | "TATHNM3" | "TATHCM" | "TATHKG" | "TATHLAT" | "TATS" | "TATCP" | "TATG" => UdfSpec {
            params: &[("Target", Str), ("ValOrT", Num)],
            volatile: false,
            is_array_streameq: false,
        },
        "MOLP" | "WTP" => UdfSpec {
            params: &[("Species1", Str), ("Species2", Str)],
            volatile: false,
            is_array_streameq: false,
        },
        "MINERALC" | "MINERALELEMENTWT" => UdfSpec {
            params: &[("Mineral", Str), ("Element", Str)],
            volatile: false,
            is_array_streameq: false,
        },
        "MINERALPROPERTY" => UdfSpec {
            params: &[("Mineral", Str), ("Prop", Str)],
            volatile: false,
            is_array_streameq: false,
        },
        "FLOWH" | "FLOWHLAT" | "FLOWHKG" | "FLOWHNM3" | "FLOWS" | "FLOWCP" | "FLOWG"
        | "STREAMH" | "STREAMHLAT" | "STREAMHKG" | "STREAMHNM3" | "STREAMS"
        | "STREAMCP" | "STREAMG" | "DENSITYA" => UdfSpec {
            params: &[("SpRange", Range), ("AmtRange", Range), ("Temp", Num)],
            volatile: false,
            is_array_streameq: false,
        },
        "STREAMTH" | "STREAMTHKG" | "STREAMTHLAT" | "STREAMTHNM3" | "STREAMTS"
        | "STREAMTCP" | "STREAMTG" | "FLOWTH" | "FLOWTS" | "FLOWTCP" | "FLOWTG" => UdfSpec {
            params: &[
                ("SpRange", Range),
                ("AmtRange", Range),
                ("TargetY", Num),
                ("Tmin", Num),
                ("Tmax", Num),
            ],
            volatile: false,
            is_array_streameq: false,
        },
        "STREAMEQ" => UdfSpec {
            params: &[
                ("SpRange", Range),
                ("AmtRange", Range),
                ("Temp", Num),
                ("Pres", Num),
            ],
            volatile: false,
            is_array_streameq: true,
        },
        _ => UdfSpec {
            params: &[("Target", Str)],
            volatile: false,
            is_array_streameq: false,
        },
    }
}

fn active_addin_names() -> impl Iterator<Item = &'static str> {
    ADDIN_FUNCTION_CATALOG
        .iter()
        .filter(|m| m.origin != AddInOrigin::HscSimOnly)
        .map(|m| m.name)
}

fn emit_vba_or_basic_udfs(call_fn: &str, byval_prefix: bool) -> String {
    use UdfParamKind::*;
    let mut out = String::new();
    for name in active_addin_names() {
        let spec = udf_spec_for(name);
        if spec.is_array_streameq {
            continue;
        }
        let fn_ident = if byval_prefix {
            name.to_string()
        } else {
            name.to_ascii_uppercase()
        };
        let sig_args: Vec<String> = spec
            .params
            .iter()
            .map(|(p, k)| match k {
                OptionalVariant => {
                    if byval_prefix {
                        format!("Optional ByVal {p} As Variant")
                    } else {
                        format!("Optional {p} As Variant")
                    }
                }
                Str => {
                    if byval_prefix {
                        format!("ByVal {p} As String")
                    } else {
                        format!("{p} As String")
                    }
                }
                Num => {
                    if byval_prefix {
                        format!("ByVal {p} As Double")
                    } else {
                        format!("{p} As Double")
                    }
                }
                Range => {
                    if byval_prefix {
                        format!("ByVal {p} As Variant")
                    } else {
                        format!("{p} As Variant")
                    }
                }
            })
            .collect();

        let has_opt = spec.params.iter().any(|(_, k)| matches!(k, OptionalVariant));
        out.push_str(&format!(
            "Public Function {fn_ident}({}) As Variant\n",
            sig_args.join(", ")
        ));
        if byval_prefix && spec.volatile {
            out.push_str("    Application.Volatile\n");
        }
        if has_opt {
            let opt_name = spec.params.last().map(|(p, _)| *p).unwrap_or("Opt");
            let req_names: Vec<&str> = spec
                .params
                .iter()
                .take(spec.params.len() - 1)
                .map(|(p, _)| *p)
                .collect();
            let full_names: Vec<&str> = spec.params.iter().map(|(p, _)| *p).collect();
            let req_call = format!("\"{name}\", {}", req_names.join(", "));
            let full_call = format!("\"{name}\", {}", full_names.join(", "));
            out.push_str(&format!(
                "    If IsMissing({opt_name}) Then {fn_ident} = {call_fn}({req_call}) Else {fn_ident} = {call_fn}({full_call})\n"
            ));
        } else {
            let arg_names: Vec<&str> = spec.params.iter().map(|(p, _)| *p).collect();
            let call_args = if arg_names.is_empty() {
                format!("\"{name}\"")
            } else {
                format!("\"{name}\", {}", arg_names.join(", "))
            };
            out.push_str(&format!("    {fn_ident} = {call_fn}({call_args})\n"));
        }
        out.push_str("End Function\n");
    }
    out
}

fn emit_python_udfs() -> String {
    use UdfParamKind::*;
    let mut out = String::new();
    for name in active_addin_names() {
        let spec = udf_spec_for(name);
        if spec.is_array_streameq {
            continue;
        }
        let py_name = name.to_ascii_uppercase();
        let params: Vec<String> = spec
            .params
            .iter()
            .map(|(p, k)| {
                let p_low = p.to_ascii_lowercase();
                match k {
                    OptionalVariant => format!("{p_low}=None"),
                    _ => p_low,
                }
            })
            .collect();
        let call_args: Vec<String> = spec
            .params
            .iter()
            .map(|(p, _)| p.to_ascii_lowercase())
            .collect();
        let call_tail = if call_args.is_empty() {
            String::new()
        } else {
            format!(", {}", call_args.join(", "))
        };
        out.push_str(&format!(
            "def {py_name}({}): return soryntel_eval(\"{name}\"{call_tail})\n",
            params.join(", ")
        ));
    }
    out
}

fn emit_onlyoffice_js_udfs() -> (String, String) {
    let mut defs = String::new();
    let mut exports = Vec::new();
    for name in active_addin_names() {
        let spec = udf_spec_for(name);
        exports.push(format!("{name}: {name}"));
        if spec.is_array_streameq {
            continue;
        }
        let params: Vec<String> = spec
            .params
            .iter()
            .map(|(p, _)| {
                let mut chars = p.chars();
                match chars.next() {
                    Some(c) => format!("{}{}", c.to_ascii_lowercase(), chars.as_str()),
                    None => String::new(),
                }
            })
            .collect();
        let call_tail = if params.is_empty() {
            String::new()
        } else {
            format!(", {}", params.join(", "))
        };
        defs.push_str(&format!(
            "  /** @customfunction */\n  function {name}({}) {{ return callBridge(\"{name}\"{call_tail}); }}\n",
            params.join(", ")
        ));
    }
    (defs, exports.join(", "))
}

/// Portable Excel & WPS Spreadsheets VBA module (`SoryntelHSC.bas`) calling the local HTTP bridge.
/// Supports Excel `Range` parameters (joined with `|` for `Stream*`, `Flow*`, `DensityA`, and `StreamEQ`),
/// full UTF-8 URL encoding, and every HSC6 + HSC10 + Mineralogy UDF (all 83 active functions).
pub fn generate_excel_wps_vba(port: u16) -> String {
    let generated_udfs = emit_vba_or_basic_udfs("SoryntelCall", true);
    format!(
        r##"Attribute VB_Name = "SoryntelHSC"
' Soryntel Thermochemical Add-In Module for Microsoft Excel & WPS Office
' Compatible with Excel 2010-2024 and WPS Spreadsheets (VBA Bridge)
Option Explicit

Private Const SORYNTEL_BASE As String = "http://127.0.0.1:{port}"

Private Function HexByte(ByVal b As Long) As String
    HexByte = "%" & Right$("0" & Hex$(b And &HFF), 2)
End Function

Private Function UrlEnc(ByVal s As String) As String
    Dim i As Long, ch As String, code As Long, res As String
    For i = 1 To Len(s)
        ch = Mid$(s, i, 1)
        code = AscW(ch)
        If code < 0 Then code = code + 65536
        If (code >= 48 And code <= 57) Or (code >= 65 And code <= 90) Or (code >= 97 And code <= 122) Or ch = "-" Or ch = "_" Or ch = "." Then
            res = res & ch
        ElseIf code < &H80 Then
            res = res & HexByte(code)
        ElseIf code < &H800 Then
            res = res & HexByte(&HC0 Or (code \ 64)) & HexByte(&H80 Or (code And &H3F))
        Else
            res = res & HexByte(&HE0 Or (code \ 4096)) & HexByte(&H80 Or ((code \ 64) And &H3F)) & HexByte(&H80 Or (code And &H3F))
        End If
    Next i
    UrlEnc = res
End Function

Private Function ArgToStr(ByVal v As Variant) As String
    If IsMissing(v) Then
        ArgToStr = ""
        Exit Function
    End If
    If IsObject(v) Then
        On Error Resume Next
        Dim arr As Variant
        arr = v.Value
        If IsArray(arr) Then
            v = arr
        Else
            ArgToStr = CStr(v.Value)
            Exit Function
        End If
        On Error GoTo 0
    End If
    If IsArray(v) Then
        Dim r As Long, c As Long, out As String, item As String
        out = ""
        For r = LBound(v, 1) To UBound(v, 1)
            On Error Resume Next
            Dim cLo As Long, cHi As Long
            cLo = LBound(v, 2)
            cHi = UBound(v, 2)
            If Err.Number = 0 Then
                For c = cLo To cHi
                    item = CStr(v(r, c))
                    If Len(out) > 0 Then out = out & "|"
                    out = out & item
                Next c
            Else
                Err.Clear
                item = CStr(v(r))
                If Len(out) > 0 Then out = out & "|"
                out = out & item
            End If
            On Error GoTo 0
        Next r
        ArgToStr = out
    Else
        ArgToStr = CStr(v)
    End If
End Function

Private Function SoryntelRaw(ByVal fnName As String, Optional ByVal a1 As Variant, Optional ByVal a2 As Variant, Optional ByVal a3 As Variant, Optional ByVal a4 As Variant, Optional ByVal a5 As Variant) As String
    On Error GoTo ErrHandler
    Dim http As Object
    Set http = CreateObject("MSXML2.XMLHTTP")
    Dim url As String
    url = SORYNTEL_BASE & "/eval?fn=" & UrlEnc(fnName)
    If Not IsMissing(a1) Then url = url & "&a=" & UrlEnc(ArgToStr(a1))
    If Not IsMissing(a2) Then url = url & "&b=" & UrlEnc(ArgToStr(a2))
    If Not IsMissing(a3) Then url = url & "&c=" & UrlEnc(ArgToStr(a3))
    If Not IsMissing(a4) Then url = url & "&d=" & UrlEnc(ArgToStr(a4))
    If Not IsMissing(a5) Then url = url & "&e=" & UrlEnc(ArgToStr(a5))
    http.Open "GET", url, False
    http.Send
    SoryntelRaw = Trim$(http.responseText)
    Exit Function
ErrHandler:
    SoryntelRaw = "#BRIDGE_OFFLINE!"
End Function

Private Function SoryntelCall(ByVal fnName As String, Optional ByVal a1 As Variant, Optional ByVal a2 As Variant, Optional ByVal a3 As Variant, Optional ByVal a4 As Variant, Optional ByVal a5 As Variant) As Variant
    Dim txt As String
    txt = SoryntelRaw(fnName, a1, a2, a3, a4, a5)
    If Left$(txt, 1) = "#" Then
        SoryntelCall = txt
    ElseIf IsNumeric(txt) Then
        SoryntelCall = CDbl(txt)
    Else
        SoryntelCall = txt
    End If
End Function

Public Sub HSC_RecalculateAll()
    Application.CalculateFullRebuild
End Sub

Public Sub HSC_ChangeMeasurementUnits(ByVal TUnit As String, ByVal EUnit As String, Optional ByVal PUnit As String = "bar")
    Dim res As Variant
    res = SoryntelCall("UNITS", TUnit, EUnit, PUnit)
    Application.CalculateFullRebuild
End Sub

{generated_udfs}
Public Function StreamEQ(ByVal SpRange As Variant, ByVal AmtRange As Variant, ByVal T As Double, ByVal P As Double) As Variant
    Dim raw As String
    raw = SoryntelRaw("StreamEQ", SpRange, AmtRange, T, P)
    If Left$(raw, 1) = "#" Then
        StreamEQ = raw
        Exit Function
    End If
    Dim lines() As String
    lines = Split(Replace(raw, vbCrLf, vbLf), vbLf)
    Dim n As Long, i As Long
    n = UBound(lines) - LBound(lines) + 1
    Dim res() As Variant
    ReDim res(1 To n, 1 To 1)
    For i = 0 To n - 1
        If IsNumeric(lines(i)) Then res(i + 1, 1) = CDbl(lines(i)) Else res(i + 1, 1) = lines(i)
    Next i
    StreamEQ = res
End Function
"##
    )
}

/// Generate a LibreOffice Calc / Collabora `.oxt` Extension Package containing BOTH
/// a Basic library (`SoryntelHSC/` via StarBasic `SimpleFileAccess`) and Python script (`Scripts/python/`).
pub fn build_libreoffice_oxt(port: u16) -> Vec<u8> {
    let mut zip = ZipBuilder::new();
    zip.add_file(
        "META-INF/manifest.xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
  <manifest:file-entry manifest:media-type="application/vnd.sun.star.basic-library" manifest:full-path="SoryntelHSC/"/>
  <manifest:file-entry manifest:media-type="application/vnd.sun.star.framework-script" manifest:full-path="Scripts/"/>
</manifest:manifest>"#
            .to_vec(),
    );
    zip.add_file(
        "description.xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<description xmlns="http://openoffice.org/extensions/description/2006" xmlns:xlink="http://www.w3.org/1999/xlink">
  <identifier value="org.soryntel.chemistry.calc.addin"/>
  <version value="1.1.0"/>
  <display-name><name lang="en">Soryntel Chemistry Add-In (Basic + Python) for LibreOffice Calc</name></display-name>
</description>"#
            .to_vec(),
    );
    zip.add_file(
        "SoryntelHSC/script.xlb",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE library:library PUBLIC "-//OpenOffice.org//DTD OfficeDocument 1.0//EN" "library.dtd">
<library:library xmlns:library="http://openoffice.org/2000/library" library:name="SoryntelHSC" library:readonly="false" library:passwordprotected="false">
  <library:element library:name="SoryntelHSC"/>
</library:library>"#
            .to_vec(),
    );

    let basic_udfs = emit_vba_or_basic_udfs("SORYNTEL_EVAL", false);
    let basic_src = format!(
        r##"REM Soryntel Chemistry StarBasic Library for LibreOffice Calc (All 83 Active Functions)
Option Compatible
Private Const SORYNTEL_URL As String = "http://127.0.0.1:{port}/eval"

Private Function JoinArg(v As Variant) As String
    If IsArray(v) Then
        Dim r As Long, c As Long, out As String
        out = ""
        For r = LBound(v, 1) To UBound(v, 1)
            On Local Error Resume Next
            c = LBound(v, 2)
            If Err = 0 Then
                Dim col As Long
                For col = LBound(v, 2) To UBound(v, 2)
                    If Len(out) > 0 Then out = out & "|"
                    out = out & CStr(v(r, col))
                Next col
            Else
                Err.Clear
                If Len(out) > 0 Then out = out & "|"
                out = out & CStr(v(r))
            End If
            On Local Error GoTo 0
        Next r
        JoinArg = out
    ElseIf IsMissing(v) Then
        JoinArg = ""
    Else
        JoinArg = CStr(v)
    End If
End Function

Private Function PctEnc(s As String) As String
    Dim i As Long, ch As String, code As Long, res As String
    For i = 1 To Len(s)
        ch = Mid(s, i, 1)
        code = Asc(ch)
        If (code >= 48 And code <= 57) Or (code >= 65 And code <= 90) Or (code >= 97 And code <= 122) Or ch = "-" Or ch = "_" Or ch = "." Then
            res = res & ch
        Else
            res = res & "%" & Right("0" & Hex(code And &HFF), 2)
        End If
    Next i
    PctEnc = res
End Function

Function SORYNTEL_EVAL(fnName As String, Optional a1 As Variant, Optional a2 As Variant, Optional a3 As Variant, Optional a4 As Variant, Optional a5 As Variant) As Variant
    On Local Error GoTo ErrBridge
    Dim u As String
    u = SORYNTEL_URL & "?fn=" & PctEnc(fnName)
    If Not IsMissing(a1) Then u = u & "&a=" & PctEnc(JoinArg(a1))
    If Not IsMissing(a2) Then u = u & "&b=" & PctEnc(JoinArg(a2))
    If Not IsMissing(a3) Then u = u & "&c=" & PctEnc(JoinArg(a3))
    If Not IsMissing(a4) Then u = u & "&d=" & PctEnc(JoinArg(a4))
    If Not IsMissing(a5) Then u = u & "&e=" & PctEnc(JoinArg(a5))
    Dim sfa As Object, stm As Object, tis As Object, txt As String
    sfa = CreateUnoService("com.sun.star.ucb.SimpleFileAccess")
    stm = sfa.openFileRead(u)
    tis = CreateUnoService("com.sun.star.io.TextInputStream")
    tis.setInputStream(stm)
    txt = tis.readString(Array(), True)
    tis.closeInput()
    txt = Trim(txt)
    If Left(txt, 1) = "#" Then
        SORYNTEL_EVAL = txt
    ElseIf IsNumeric(txt) Then
        SORYNTEL_EVAL = CDbl(txt)
    Else
        SORYNTEL_EVAL = txt
    End If
    Exit Function
ErrBridge:
    SORYNTEL_EVAL = "#BRIDGE_OFFLINE!"
End Function

{basic_udfs}
Function STREAMEQ(sp As Variant, amt As Variant, temp As Double, pres As Double) As Variant
    Dim raw As String
    raw = CStr(SORYNTEL_EVAL("StreamEQ", sp, amt, temp, pres))
    If Left(raw, 1) = "#" Then
        STREAMEQ = raw
        Exit Function
    End If
    Dim lines As Variant, i As Long
    lines = Split(raw, Chr(10))
    Dim out() As Variant
    ReDim out(UBound(lines), 0)
    For i = 0 To UBound(lines)
        If IsNumeric(lines(i)) Then out(i, 0) = CDbl(lines(i)) Else out(i, 0) = lines(i)
    Next i
    STREAMEQ = out
End Function
"##
    );
    let xba = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE script:module PUBLIC \"-//OpenOffice.org//DTD OfficeDocument 1.0//EN\" \"module.dtd\">\n\
         <script:module xmlns:script=\"http://openoffice.org/2000/script\" script:name=\"SoryntelHSC\" script:language=\"StarBasic\">\n\
         {}\n\
         </script:module>\n",
        xml_escape(&basic_src)
    );
    zip.add_file("SoryntelHSC/SoryntelHSC.xba", xba.into_bytes());

    let py_udfs = emit_python_udfs();
    let py = format!(
        r##"# Soryntel Chemistry Python Script for LibreOffice Calc (and standalone Python automation — All 83 Active Functions)
import urllib.parse
import urllib.request

SORYNTEL_URL = "http://127.0.0.1:{port}/eval"

def _flat(v):
    if isinstance(v, (list, tuple)):
        out = []
        for item in v:
            if isinstance(item, (list, tuple)):
                out.extend(str(x) for x in item)
            else:
                out.append(str(item))
        return "|".join(out)
    return "" if v is None else str(v)

def soryntel_eval(fn_name, a=None, b=None, c=None, d=None, e=None):
    params = {{"fn": fn_name}}
    for key, val in (("a", a), ("b", b), ("c", c), ("d", d), ("e", e)):
        if val is not None:
            params[key] = _flat(val)
    qs = urllib.parse.urlencode(params)
    try:
        with urllib.request.urlopen(f"{{SORYNTEL_URL}}?{{qs}}", timeout=5) as resp:
            txt = resp.read().decode("utf-8").strip()
    except Exception:
        return "#BRIDGE_OFFLINE!"
    try:
        return float(txt)
    except ValueError:
        return txt

{py_udfs}
def STREAMEQ(sp, amt, temp, pres):
    raw = str(soryntel_eval("StreamEQ", sp, amt, temp, pres))
    if raw.startswith("#"):
        return [[raw]]
    rows = []
    for x in raw.splitlines():
        try:
            rows.append([float(x)])
        except ValueError:
            rows.append([x])
    return rows
"##
    );
    zip.add_file("Scripts/python/soryntel_hsc.py", py.into_bytes());
    zip.add_file(
        "README.txt",
        format!(
            "Soryntel Chemistry Add-In for LibreOffice Calc / Collabora\n\
             ==========================================================\n\
             1. Install this .oxt via Tools -> Extension Manager -> Add...\n\
             2. Copy/expose SoryntelHSC module functions to Standard library OR use macro-free =WEBSERVICE(\"http://127.0.0.1:{port}/eval?fn=H&a=CO2(g)&b=100\").\n\
             3. Keep Soryntel Add-In HTTP Bridge running on 127.0.0.1:{port} (GUI or `soryntel-desktop --addin-bridge {port}`).\n"
        )
        .into_bytes(),
    );
    zip.finish()
}

/// Generate an ONLYOFFICE Desktop / Docs Plugin + Custom Functions Library package
/// (`.zip` with `config.json`, live status `index.html`, `code.js`, and all 83 `@customfunction` UDFs in `functions.js`).
pub fn build_onlyoffice_plugin_zip(port: u16) -> Vec<u8> {
    let mut zip = ZipBuilder::new();
    zip.add_file(
        "config.json",
        br#"{
  "name": "Soryntel Chemistry Add-In",
  "guid": "asc.{7F3B5D9E-4C12-4A89-9E11-5097E1000015}",
  "baseUrl": "",
  "variations": [
    {
      "description": "83 Thermochemical & Mineralogy Custom Functions compatible with HSC Add-Ins",
      "url": "index.html",
      "isViewer": false,
      "EditorsSupport": ["cell"],
      "isVisual": true,
      "isSystem": false,
      "initDataType": "none",
      "initData": "",
      "buttons": [{ "text": "Close", "primary": true }]
    }
  ]
}"#
        .to_vec(),
    );
    zip.add_file(
        "index.html",
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Soryntel Chemistry Add-In</title>
  <style>
    body {{ font-family: sans-serif; font-size: 13px; margin: 12px; color: #1f2937; }}
    code {{ background: #f3f4f6; padding: 2px 5px; border-radius: 3px; }}
    .ok {{ color: #166534; font-weight: 600; }}
    .err {{ color: #991b1b; font-weight: 600; }}
  </style>
  <script type="text/javascript" src="functions.js"></script>
  <script type="text/javascript" src="code.js"></script>
</head>
<body>
  <h3>Soryntel HSC Chemistry Add-In (83 Functions)</h3>
  <p>Bridge URL: <code>http://127.0.0.1:{port}</code> — Status: <span id="bridge-status">checking...</span></p>
  <p>Import <code>functions.js</code> under Plugins -&gt; Macros / Custom Functions in ONLYOFFICE.</p>
  <p>Examples: <code>=H("CO2(g)";100)</code>, <code>=StreamEQ(A4:A15;B4:B15;C1;C2)</code>, <code>=MineralCode("Pyrite")</code></p>
  <script>
    fetch("http://127.0.0.1:{port}/health").then(function(r){{ return r.json(); }}).then(function(d){{
      var el = document.getElementById("bridge-status");
      el.className = "ok";
      el.textContent = "Online (" + d.species_count + " species, units: " + d.units.temp + " / " + d.units.energy + " / " + d.units.pressure + ")";
    }}).catch(function(){{
      var el = document.getElementById("bridge-status");
      el.className = "err";
      el.textContent = "Offline (start Soryntel HTTP Bridge)";
    }});
  </script>
</body>
</html>"#
        )
        .into_bytes(),
    );
    let (js_defs, js_exports) = emit_onlyoffice_js_udfs();
    let custom_funcs = format!(
        r##"/**
 * Soryntel Chemistry Custom Functions Library for ONLYOFFICE Spreadsheet Editor
 * Exposes all 83 active HSC6 + HSC10 + Mineralogy functions.
 */
(function() {{
  var BASE = "http://127.0.0.1:{port}";
  function flat(v) {{
    if (Array.isArray(v)) {{
      var out = [];
      for (var i = 0; i < v.length; i++) {{
        if (Array.isArray(v[i])) {{
          for (var j = 0; j < v[i].length; j++) out.push(String(v[i][j] === undefined ? "" : v[i][j]));
        }} else {{
          out.push(String(v[i] === undefined ? "" : v[i]));
        }}
      }}
      return out.join("|");
    }}
    return v === undefined || v === null ? "" : String(v);
  }}
  function callBridge(fnName, a, b, c, d, e) {{
    try {{
      var xhr = new XMLHttpRequest();
      var q = BASE + "/eval?fn=" + encodeURIComponent(fnName);
      if (a !== undefined) q += "&a=" + encodeURIComponent(flat(a));
      if (b !== undefined) q += "&b=" + encodeURIComponent(flat(b));
      if (c !== undefined) q += "&c=" + encodeURIComponent(flat(c));
      if (d !== undefined) q += "&d=" + encodeURIComponent(flat(d));
      if (e !== undefined) q += "&e=" + encodeURIComponent(flat(e));
      xhr.open("GET", q, false);
      xhr.send(null);
      var txt = (xhr.responseText || "").trim();
      if (txt.charAt(0) === "#") return txt;
      var num = Number(txt);
      return (!isNaN(num) && txt !== "") ? num : txt;
    }} catch (_err) {{
      return "#BRIDGE_OFFLINE!";
    }}
  }}

{js_defs}
  /** @customfunction */
  function StreamEQ(speciesRange, amountRange, temp, press) {{
    try {{
      var xhr = new XMLHttpRequest();
      var q = BASE + "/eval?fn=StreamEQ&a=" + encodeURIComponent(flat(speciesRange))
        + "&b=" + encodeURIComponent(flat(amountRange))
        + "&c=" + encodeURIComponent(flat(temp))
        + "&d=" + encodeURIComponent(flat(press));
      xhr.open("GET", q, false);
      xhr.send(null);
      var txt = (xhr.responseText || "").trim();
      if (txt.charAt(0) === "#") return txt;
      return txt.split(/\r?\n/).map(function(line) {{
        var n = Number(line);
        return [(!isNaN(n) && line.trim() !== "") ? n : line];
      }});
    }} catch (_err) {{
      return "#BRIDGE_OFFLINE!";
    }}
  }}

  var exported = {{ {js_exports} }};
  if (typeof window !== "undefined") {{
    window.SoryntelFunctions = exported;
  }}
  if (typeof Api !== "undefined" && typeof Api.AddCustomFunctionLibrary === "function") {{
    Api.AddCustomFunctionLibrary("SoryntelHSC", exported);
  }}
}})();
"##
    );
    zip.add_file("functions.js", custom_funcs.into_bytes());

    let code_js = format!(
        r#"(function(window, undefined) {{
  if (window.Asc && window.Asc.plugin) {{
    window.Asc.plugin.init = function() {{
      if (window.Api && typeof window.Api.AddCustomFunctionLibrary === "function" && window.SoryntelFunctions) {{
        window.Api.AddCustomFunctionLibrary("SoryntelHSC", window.SoryntelFunctions);
      }}
    }};
    window.Asc.plugin.button = function() {{
      this.executeCommand("close", "");
    }};
  }}
}})(window, undefined);"#
    );
    let _ = port;
    zip.add_file("code.js", code_js.into_bytes());
    zip.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel_addins::{AddInOrigin, ADDIN_FUNCTION_CATALOG};

    #[test]
    fn vba_oxt_and_onlyoffice_expose_all_83_active_functions() {
        let vba = generate_excel_wps_vba(8765);
        let oxt = build_libreoffice_oxt(8765);
        let oo = build_onlyoffice_plugin_zip(8765);

        let oxt_str = String::from_utf8_lossy(&oxt);
        let oo_str = String::from_utf8_lossy(&oo);

        let active: Vec<_> = ADDIN_FUNCTION_CATALOG
            .iter()
            .filter(|m| m.origin != AddInOrigin::HscSimOnly)
            .collect();
        assert_eq!(active.len(), 83);

        for meta in active {
            assert!(
                vba.contains(&format!("Function {}(", meta.name)),
                "VBA missing {}",
                meta.name
            );
            assert!(
                oxt_str.contains(&format!("Function {}(", meta.name.to_ascii_uppercase())),
                "OXT StarBasic missing {}",
                meta.name
            );
            assert!(
                oxt_str.contains(&format!("def {}(", meta.name.to_ascii_uppercase())),
                "OXT Python missing {}",
                meta.name
            );
            assert!(
                oo_str.contains(&format!("function {}(", meta.name)),
                "ONLYOFFICE JS missing {}",
                meta.name
            );
        }
    }

    #[test]
    fn xlsx_marks_standard_excel_errors_with_error_cell_type() {
        let sheet = ExportSheet {
            name: "ErrTest".into(),
            col_widths: vec![16.0],
            cells: vec![
                ExportCell {
                    row: 0,
                    col: 0,
                    style: ExportCellStyle::Default,
                    value: ExportCellValue::Formula {
                        formula: "=MW(\"NO_SUCH_SPECIES\")".into(),
                        cached_number: None,
                        cached_text: Some("#NAME?".into()),
                        array_rows: 1,
                    },
                },
                ExportCell {
                    row: 1,
                    col: 0,
                    style: ExportCellStyle::Default,
                    value: ExportCellValue::Formula {
                        formula: "=StreamX(\"A\";B1;\"SiO2\")".into(),
                        cached_number: None,
                        cached_text: Some("#HSC_SIM_ONLY".into()),
                        array_rows: 1,
                    },
                },
            ],
        };
        let bytes = build_xlsx(&[sheet]);
        let s = String::from_utf8_lossy(&bytes);
        assert!(s.contains("<c r=\"A1\" s=\"0\" t=\"e\">"), "missing t=\"e\" on #NAME?");
        assert!(
            s.contains("<c r=\"A2\" s=\"0\" t=\"str\">"),
            "custom #HSC_SIM_ONLY should keep t=\"str\""
        );
    }
}
