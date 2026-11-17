//! Local HTTP Bridge for Cross-Suite HSC Excel Add-Ins (Microsoft Excel VBA/WEBSERVICE,
//! LibreOffice Calc `.oxt` Python/Basic, ONLYOFFICE Plugin, WPS Office, and headless scripts).
//!
//! Runs synchronously on `127.0.0.1:<port>` using `std::net::TcpListener` with CORS headers so
//! standalone Soryntel GUI, `--addin-bridge` daemon mode, and external spreadsheets share
//! the exact same thermodynamic engine and active `AddInUnits`.

use serde_json::{json, Value};
use soryntel_application::SoryntelApplication;
use soryntel_db::ThermoDatabase;
use soryntel_thermo::excel_addins::{
    AddInOrigin, AddInUnits, ArgValue, CellEvalValue, ADDIN_FUNCTION_CATALOG,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const MAX_REQUEST_BYTES: usize = 1024 * 1024;

/// Shared telemetry counters for the live Add-In HTTP Bridge.
#[derive(Debug, Default)]
pub struct BridgeStats {
    pub requests_served: AtomicU64,
    pub last_call: Mutex<Option<String>>,
    pub last_error: Mutex<Option<String>>,
}

impl BridgeStats {
    fn record_call(&self, summary: String) {
        self.requests_served.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut guard) = self.last_call.lock() {
            *guard = Some(summary);
        }
    }

    fn record_error(&self, err: String) {
        if let Ok(mut guard) = self.last_error.lock() {
            *guard = Some(err);
        }
    }
}

/// Handle managing a background HTTP Bridge thread spawned from the desktop UI.
pub struct BridgeHandle {
    port: u16,
    stop_flag: Arc<AtomicBool>,
    stats: Arc<BridgeStats>,
    join_handle: Option<JoinHandle<()>>,
}

impl BridgeHandle {
    /// Start the HTTP bridge on `127.0.0.1:<port>` in a dedicated background thread.
    pub fn start(
        db: Arc<ThermoDatabase>,
        units: Arc<Mutex<AddInUnits>>,
        port: u16,
    ) -> Result<Self, String> {
        let addr = format!("127.0.0.1:{port}");
        let listener = TcpListener::bind(&addr)
            .map_err(|e| format!("No se pudo abrir HTTP Bridge en {addr}: {e}"))?;
        let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("Error configurando socket no bloqueante: {e}"))?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(BridgeStats::default());

        let stop_clone = Arc::clone(&stop_flag);
        let stats_clone = Arc::clone(&stats);
        let join_handle = thread::Builder::new()
            .name(format!("soryntel-addin-bridge-{bound_port}"))
            .spawn(move || {
                run_accept_loop(listener, db, units, stop_clone, stats_clone, bound_port);
            })
            .map_err(|e| format!("No se pudo lanzar hilo HTTP Bridge: {e}"))?;

        Ok(Self {
            port: bound_port,
            stop_flag,
            stats,
            join_handle: Some(join_handle),
        })
    }

    /// Signal the background bridge thread to stop and join it.
    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn is_running(&self) -> bool {
        !self.stop_flag.load(Ordering::Relaxed) && self.join_handle.is_some()
    }

    pub fn requests_served(&self) -> u64 {
        self.stats.requests_served.load(Ordering::Relaxed)
    }

    pub fn last_call(&self) -> Option<String> {
        self.stats.last_call.lock().ok().and_then(|g| g.clone())
    }
}

impl Drop for BridgeHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Run the HTTP Bridge synchronously in the current thread (`--addin-bridge` headless CLI mode).
pub fn run_bridge_blocking(
    db: Arc<ThermoDatabase>,
    units: Arc<Mutex<AddInUnits>>,
    port: u16,
) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr)?;
    let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    listener.set_nonblocking(true)?;
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stats = Arc::new(BridgeStats::default());
    eprintln!(
        "Soryntel HSC Excel Add-In HTTP Bridge listening on http://127.0.0.1:{bound_port} ({} species, {} functions)",
        db.len(),
        ADDIN_FUNCTION_CATALOG
            .iter()
            .filter(|m| m.origin != AddInOrigin::HscSimOnly)
            .count()
    );
    run_accept_loop(listener, db, units, stop_flag, stats, bound_port);
    Ok(())
}

fn run_accept_loop(
    listener: TcpListener,
    db: Arc<ThermoDatabase>,
    units: Arc<Mutex<AddInUnits>>,
    stop_flag: Arc<AtomicBool>,
    stats: Arc<BridgeStats>,
    port: u16,
) {
    while !stop_flag.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                let db_ref = Arc::clone(&db);
                let units_ref = Arc::clone(&units);
                let stats_ref = Arc::clone(&stats);
                thread::spawn(move || {
                    handle_client_stream(&mut stream, &db_ref, &units_ref, &stats_ref, port);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(25));
            }
            Err(e) => {
                stats.record_error(format!("Accept error: {e}"));
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

fn handle_client_stream(
    stream: &mut TcpStream,
    db: &ThermoDatabase,
    units: &Arc<Mutex<AddInUnits>>,
    stats: &BridgeStats,
    port: u16,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_nonblocking(false);

    let req = match read_http_request(stream) {
        Ok(Some(r)) => r,
        Ok(None) => return,
        Err(msg) => {
            stats.record_error(msg.clone());
            let resp = HttpResponse::json(400, json!({ "ok": false, "error": msg }));
            let _ = write_http_response(stream, &resp);
            return;
        }
    };

    let (resp, summary) = handle_bridge_request(&req, db, units, port);
    if !summary.is_empty() {
        stats.record_call(summary);
    }
    let _ = write_http_response(stream, &resp);
}

/// Minimal parsed HTTP/1.1 request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub query: String,
    pub body: String,
}

/// Minimal HTTP/1.1 response with automatic CORS headers.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl HttpResponse {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: body.into(),
        }
    }

    pub fn json(status: u16, val: Value) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: val.to_string(),
        }
    }

    pub fn empty(status: u16) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8",
            body: String::new(),
        }
    }
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    }
}

fn write_http_response(stream: &mut impl Write, resp: &HttpResponse) -> std::io::Result<()> {
    let bytes = resp.body.as_bytes();
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        resp.status,
        status_reason(resp.status),
        resp.content_type,
        bytes.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

fn read_http_request(stream: &mut impl Read) -> Result<Option<HttpRequest>, String> {
    let mut buf = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    let mut header_end: Option<(usize, usize)> = None;

    loop {
        let n = match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                if buf.is_empty() {
                    return Ok(None);
                }
                break;
            }
            Err(e) => return Err(format!("Socket read error: {e}")),
        };
        if buf.len() + n > MAX_REQUEST_BYTES {
            return Err("Request exceeds 1 MiB limit".into());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = find_header_end(&buf) {
            header_end = Some(pos);
            break;
        }
    }

    if buf.is_empty() {
        return Ok(None);
    }

    let (hdr_bytes_len, sep_len) = header_end.ok_or_else(|| "Malformed HTTP headers".to_string())?;
    let header_str = String::from_utf8_lossy(&buf[..hdr_bytes_len]).to_string();
    let content_length = parse_content_length(&header_str);

    let body_start = hdr_bytes_len + sep_len;
    let total_needed = body_start + content_length;
    if total_needed > MAX_REQUEST_BYTES {
        return Err("Request body exceeds 1 MiB limit".into());
    }
    while buf.len() < total_needed {
        let n = match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => return Err(format!("Body read error: {e}")),
        };
        buf.extend_from_slice(&chunk[..n]);
    }

    let body_end = total_needed.min(buf.len());
    let body = String::from_utf8_lossy(&buf[body_start..body_end]).to_string();
    Ok(Some(parse_http_request_head(&header_str, body)?))
}

fn find_header_end(buf: &[u8]) -> Option<(usize, usize)> {
    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
        return Some((i, 4));
    }
    if let Some(i) = buf.windows(2).position(|w| w == b"\n\n") {
        return Some((i, 2));
    }
    None
}

fn parse_content_length(header_str: &str) -> usize {
    for line in header_str.lines().skip(1) {
        if let Some((name, val)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                return val.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }
    0
}

/// Parse an HTTP request line + header block + body into `HttpRequest`.
pub fn parse_http_request_head(header_str: &str, body: String) -> Result<HttpRequest, String> {
    let first_line = header_str
        .lines()
        .next()
        .ok_or_else(|| "Empty HTTP request".to_string())?
        .trim();
    let mut parts = first_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "Missing HTTP method".to_string())?
        .to_ascii_uppercase();
    let target = parts.next().unwrap_or("/");
    let (raw_path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q.to_string()),
        None => (target, String::new()),
    };
    Ok(HttpRequest {
        method,
        path: raw_path.to_string(),
        query,
        body,
    })
}

/// Decode URL percent-encoded strings (`%XX` and `+` -> space).
pub fn url_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse URL query parameters into ordered `(decoded_key, decoded_val)` pairs.
pub fn parse_query_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| match part.split_once('=') {
            Some((k, v)) => (url_decode(k), url_decode(v)),
            None => (url_decode(part), String::new()),
        })
        .collect()
}

fn query_map(query: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for (k, v) in parse_query_pairs(query) {
        map.insert(k.to_ascii_lowercase(), v);
    }
    map
}

fn apply_unit_overrides(
    units: &mut AddInUnits,
    temp: Option<&str>,
    energy: Option<&str>,
    pressure: Option<&str>,
) {
    if let Some(t) = temp.map(str::trim).filter(|s| !s.is_empty()) {
        units.temp_unit = t.trim_start_matches('°').to_string();
    }
    if let Some(e) = energy.map(str::trim).filter(|s| !s.is_empty()) {
        units.energy_unit = e.to_string();
    }
    if let Some(p) = pressure.map(str::trim).filter(|s| !s.is_empty()) {
        units.pressure_unit = p.to_string();
    }
}

fn units_json(u: &AddInUnits) -> Value {
    json!({
        "temp": u.temp_unit,
        "energy": u.energy_unit,
        "pressure": u.pressure_unit
    })
}

fn is_range_slot(fn_upper: &str, arg_idx: usize) -> bool {
    if fn_upper.starts_with("STREAM") || fn_upper.starts_with("FLOW") || fn_upper == "DENSITYA" {
        arg_idx <= 1 && !matches!(fn_upper, "STREAMX" | "STREAMMASS" | "STREAMVOL")
    } else {
        matches!(fn_upper, "SUM" | "AVERAGE" | "MIN" | "MAX")
    }
}

fn parse_bridge_arg(fn_upper: &str, arg_idx: usize, raw: &str) -> ArgValue {
    let t = raw.trim();
    if t.contains('|') {
        let items = t.split('|').map(|s| s.trim().to_string()).collect();
        return ArgValue::Range(items);
    }
    if is_range_slot(fn_upper, arg_idx) {
        let sep = if t.contains(';') {
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
        return ArgValue::Range(vec![t.to_string()]);
    }
    ArgValue::Scalar(t.to_string())
}

fn json_val_to_arg(fn_upper: &str, arg_idx: usize, v: &Value) -> ArgValue {
    match v {
        Value::Array(items) => ArgValue::Range(
            items
                .iter()
                .map(|x| match x {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                })
                .collect(),
        ),
        Value::String(s) => parse_bridge_arg(fn_upper, arg_idx, s),
        Value::Number(n) => ArgValue::Scalar(n.to_string()),
        Value::Bool(b) => ArgValue::Scalar(b.to_string()),
        Value::Null => ArgValue::Scalar(String::new()),
        other => ArgValue::Scalar(other.to_string()),
    }
}

fn ordered_query_args(params: &BTreeMap<String, String>) -> Vec<String> {
    // Support `&a=&b=&c=...` AND `&a0=&a1=...` AND `&arg0=&arg1=...` AND `&p0=&p1=...`
    const KEY_SLOTS: [&[&str]; 8] = [
        &["a", "a0", "arg0", "p0"],
        &["b", "a1", "arg1", "p1"],
        &["c", "a2", "arg2", "p2"],
        &["d", "a3", "arg3", "p3"],
        &["e", "a4", "arg4", "p4"],
        &["f", "a5", "arg5", "p5"],
        &["g", "a6", "arg6", "p6"],
        &["h", "a7", "arg7", "p7"],
    ];

    let mut slots: [Option<String>; 8] = Default::default();
    let mut highest: Option<usize> = None;
    for (idx, aliases) in KEY_SLOTS.iter().enumerate() {
        for &alias in *aliases {
            if let Some(v) = params.get(alias) {
                slots[idx] = Some(v.clone());
                highest = Some(idx);
                break;
            }
        }
    }
    let Some(last_idx) = highest else {
        return Vec::new();
    };
    (0..=last_idx)
        .map(|i| slots[i].clone().unwrap_or_default())
        .collect()
}

fn cell_value_to_json(
    fn_name: &str,
    val: &CellEvalValue,
    units: &AddInUnits,
    decimals: usize,
) -> Value {
    let u = units_json(units);
    match val {
        CellEvalValue::Number(n) => json!({
            "ok": true,
            "fn": fn_name,
            "kind": "number",
            "value": n,
            "display": val.as_display_string(decimals),
            "units": u
        }),
        CellEvalValue::Text(s) => json!({
            "ok": true,
            "fn": fn_name,
            "kind": "text",
            "value": s,
            "display": s,
            "units": u
        }),
        CellEvalValue::Array(arr) => {
            let displays: Vec<String> = arr.iter().map(|v| v.as_display_string(8)).collect();
            let raw_vals: Vec<Value> = arr
                .iter()
                .map(|v| match v {
                    CellEvalValue::Number(x) if x.is_finite() => json!(x),
                    CellEvalValue::Text(t) => json!(t),
                    CellEvalValue::Error(e) => json!(e),
                    _ => json!(v.as_display_string(8)),
                })
                .collect();
            json!({
                "ok": true,
                "fn": fn_name,
                "kind": "array",
                "values": raw_vals,
                "displays": displays,
                "display": val.as_multiline_string(8),
                "units": u
            })
        }
        CellEvalValue::Error(err) => json!({
            "ok": false,
            "fn": fn_name,
            "kind": "error",
            "error": err,
            "display": err,
            "units": u
        }),
    }
}

fn eval_query_params(
    params: &BTreeMap<String, String>,
    db: &ThermoDatabase,
    units_lock: &Arc<Mutex<AddInUnits>>,
) -> (String, CellEvalValue, AddInUnits, usize) {
    let mut decimals = params
        .get("decimals")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(6)
        .min(12);

    let has_override = params.contains_key("t") || params.contains_key("e") || params.contains_key("p");
    let mut guard = units_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut local_units = guard.clone();
    if has_override {
        apply_unit_overrides(
            &mut local_units,
            params.get("t").map(String::as_str),
            params.get("e").map(String::as_str),
            params.get("p").map(String::as_str),
        );
    }

    let active_units = if has_override { &mut local_units } else { &mut *guard };

    if let Some(formula) = params.get("formula").filter(|s| !s.trim().is_empty()) {
        let val = SoryntelApplication::eval_addin_formula(db, active_units, formula, |_, _| {
            String::new()
        });
        if matches!(val, CellEvalValue::Array(_)) && !params.contains_key("decimals") {
            decimals = 8;
        }
        let u_out = active_units.clone();
        return (formula.clone(), val, u_out, decimals);
    }

    let fn_name = params.get("fn").cloned().unwrap_or_default();
    let fn_upper = fn_name.trim().to_ascii_uppercase();
    let raw_args = ordered_query_args(params);
    let args: Vec<ArgValue> = raw_args
        .iter()
        .enumerate()
        .map(|(i, s)| parse_bridge_arg(&fn_upper, i, s))
        .collect();
    let val = SoryntelApplication::eval_addin_call(db, active_units, &fn_name, &args);
    if (fn_upper == "STREAMEQ" || matches!(val, CellEvalValue::Array(_)))
        && !params.contains_key("decimals")
    {
        decimals = 8;
    }
    let summary = format!("{}({})", fn_name.trim(), raw_args.join(", "));
    let u_out = active_units.clone();
    (summary, val, u_out, decimals)
}

/// Pure HTTP request router shared by the background server, CLI bridge, and unit tests.
pub fn handle_bridge_request(
    req: &HttpRequest,
    db: &ThermoDatabase,
    units: &Arc<Mutex<AddInUnits>>,
    port: u16,
) -> (HttpResponse, String) {
    if req.method == "OPTIONS" {
        return (HttpResponse::empty(204), String::new());
    }

    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/health") | ("GET", "/") => {
            let guard = units.lock().unwrap_or_else(|e| e.into_inner());
            let active_count = ADDIN_FUNCTION_CATALOG
                .iter()
                .filter(|m| m.origin != AddInOrigin::HscSimOnly)
                .count();
            let resp = HttpResponse::json(
                200,
                json!({
                    "ok": true,
                    "service": "soryntel-hsc-bridge",
                    "version": env!("CARGO_PKG_VERSION"),
                    "port": port,
                    "species_count": db.len(),
                    "functions_count": active_count,
                    "units": units_json(&guard)
                }),
            );
            (resp, "GET /health".into())
        }
        ("GET", "/catalog") => {
            let items: Vec<Value> = ADDIN_FUNCTION_CATALOG
                .iter()
                .map(|m| {
                    json!({
                        "name": m.name,
                        "syntax": m.syntax,
                        "group": m.group,
                        "origin": m.origin.label(),
                        "active": m.origin != AddInOrigin::HscSimOnly,
                        "description": m.description,
                        "example": m.example
                    })
                })
                .collect();
            (HttpResponse::json(200, Value::Array(items)), "GET /catalog".into())
        }
        ("GET", "/units") => {
            let params = query_map(&req.query);
            let mut guard = units.lock().unwrap_or_else(|e| e.into_inner());
            if params.contains_key("temp")
                || params.contains_key("energy")
                || params.contains_key("pressure")
            {
                apply_unit_overrides(
                    &mut guard,
                    params.get("temp").map(String::as_str),
                    params.get("energy").map(String::as_str),
                    params.get("pressure").map(String::as_str),
                );
            }
            (
                HttpResponse::json(200, json!({ "ok": true, "units": units_json(&guard) })),
                "GET /units".into(),
            )
        }
        ("POST", "/units") => {
            let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
                return (
                    HttpResponse::json(400, json!({ "ok": false, "error": "Invalid JSON body" })),
                    String::new(),
                );
            };
            let mut guard = units.lock().unwrap_or_else(|e| e.into_inner());
            apply_unit_overrides(
                &mut guard,
                payload.get("temp").and_then(Value::as_str),
                payload.get("energy").and_then(Value::as_str),
                payload.get("pressure").and_then(Value::as_str),
            );
            (
                HttpResponse::json(200, json!({ "ok": true, "units": units_json(&guard) })),
                format!("POST /units ({}, {}, {})", guard.temp_unit, guard.energy_unit, guard.pressure_unit),
            )
        }
        ("GET", "/eval") => {
            let params = query_map(&req.query);
            let (summary, val, _, decimals) = eval_query_params(&params, db, units);
            let txt = val.as_multiline_string(decimals);
            (HttpResponse::text(200, txt), summary)
        }
        ("GET", "/json") => {
            let params = query_map(&req.query);
            let fn_name = params.get("fn").cloned().unwrap_or_else(|| "FORMULA".into());
            let (summary, val, used_units, decimals) = eval_query_params(&params, db, units);
            let body = cell_value_to_json(&fn_name, &val, &used_units, decimals);
            (HttpResponse::json(200, body), summary)
        }
        ("POST", "/eval") => handle_post_eval(&req.body, db, units),
        ("POST", "/streameq") => handle_post_streameq(&req.body, db, units),
        _ => (
            HttpResponse::json(404, json!({ "ok": false, "error": "Unknown route" })),
            String::new(),
        ),
    }
}

fn handle_post_eval(
    body: &str,
    db: &ThermoDatabase,
    units_lock: &Arc<Mutex<AddInUnits>>,
) -> (HttpResponse, String) {
    let Ok(payload) = serde_json::from_str::<Value>(body) else {
        return (
            HttpResponse::json(400, json!({ "ok": false, "error": "Invalid JSON body" })),
            String::new(),
        );
    };
    let decimals = payload
        .get("decimals")
        .and_then(Value::as_u64)
        .map(|d| d as usize)
        .unwrap_or(6)
        .min(12);

    let mut guard = units_lock.lock().unwrap_or_else(|e| e.into_inner());
    let unit_override = payload.get("units");
    let mut local_units = guard.clone();
    if let Some(u) = unit_override {
        apply_unit_overrides(
            &mut local_units,
            u.get("temp").and_then(Value::as_str),
            u.get("energy").and_then(Value::as_str),
            u.get("pressure").and_then(Value::as_str),
        );
    }
    let active_units = if unit_override.is_some() {
        &mut local_units
    } else {
        &mut *guard
    };

    if let Some(formula) = payload.get("formula").and_then(Value::as_str) {
        let val = SoryntelApplication::eval_addin_formula(db, active_units, formula, |_, _| {
            String::new()
        });
        let dec = if matches!(val, CellEvalValue::Array(_)) { 8 } else { decimals };
        let j = cell_value_to_json("FORMULA", &val, active_units, dec);
        return (HttpResponse::json(200, j), format!("POST /eval {formula}"));
    }

    let fn_name = payload
        .get("fn")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if fn_name.is_empty() {
        return (
            HttpResponse::json(400, json!({ "ok": false, "error": "Missing 'fn' or 'formula'" })),
            String::new(),
        );
    }
    let fn_upper = fn_name.to_ascii_uppercase();
    let args: Vec<ArgValue> = payload
        .get("args")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .enumerate()
                .map(|(i, v)| json_val_to_arg(&fn_upper, i, v))
                .collect()
        })
        .unwrap_or_default();

    let val = SoryntelApplication::eval_addin_call(db, active_units, &fn_name, &args);
    let dec = if fn_upper == "STREAMEQ" || matches!(val, CellEvalValue::Array(_)) {
        8
    } else {
        decimals
    };
    let j = cell_value_to_json(&fn_name, &val, active_units, dec);
    (HttpResponse::json(200, j), format!("POST /eval {fn_name}"))
}

fn handle_post_streameq(
    body: &str,
    db: &ThermoDatabase,
    units_lock: &Arc<Mutex<AddInUnits>>,
) -> (HttpResponse, String) {
    let Ok(payload) = serde_json::from_str::<Value>(body) else {
        return (
            HttpResponse::json(400, json!({ "ok": false, "error": "Invalid JSON body" })),
            String::new(),
        );
    };

    let guard = units_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut local_units = guard.clone();
    if let Some(u) = payload.get("units") {
        apply_unit_overrides(
            &mut local_units,
            u.get("temp").and_then(Value::as_str),
            u.get("energy").and_then(Value::as_str),
            u.get("pressure").and_then(Value::as_str),
        );
    }
    let active_units = if payload.get("units").is_some() {
        &local_units
    } else {
        &*guard
    };

    let extract_str_vec = |v: &Value| -> Vec<String> {
        v.as_array()
            .map(|arr| {
                arr.iter()
                    .map(|x| match x {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let species = payload
        .get("species")
        .map(extract_str_vec)
        .unwrap_or_default();
    let inputs = payload
        .get("inputs")
        .map(extract_str_vec)
        .unwrap_or_default();
    let t_val = payload.get("t").and_then(Value::as_f64).unwrap_or(1500.0);
    let p_val = payload.get("p").and_then(Value::as_f64).unwrap_or(1.0);

    let val = if !species.is_empty() {
        SoryntelApplication::eval_addin_streameq(db, active_units, &species, &inputs, t_val, p_val)
    } else if let Some(args_arr) = payload.get("args").and_then(Value::as_array) {
        let mut u_mut = active_units.clone();
        let args: Vec<ArgValue> = args_arr
            .iter()
            .enumerate()
            .map(|(i, v)| json_val_to_arg("STREAMEQ", i, v))
            .collect();
        SoryntelApplication::eval_addin_call(db, &mut u_mut, "StreamEQ", &args)
    } else {
        CellEvalValue::Error("#VALUE!".into())
    };

    let mut j = cell_value_to_json("StreamEQ", &val, active_units, 8);
    if let (Some(obj), CellEvalValue::Array(arr)) = (j.as_object_mut(), &val) {
        obj.insert(
            "outputs".into(),
            Value::Array(
                arr.iter()
                    .map(|v| match v {
                        CellEvalValue::Number(n) if n.is_finite() => json!(n),
                        other => json!(other.as_display_string(8)),
                    })
                    .collect(),
            ),
        );
    }
    let summary = format!("POST /streameq ({} rows, T={t_val}, P={p_val})", species.len());
    (HttpResponse::json(200, j), summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    fn test_db_and_units() -> (Arc<ThermoDatabase>, Arc<Mutex<AddInUnits>>) {
        (
            Arc::new(ThermoDatabase::with_open_defaults()),
            Arc::new(Mutex::new(AddInUnits::default())),
        )
    }

    #[test]
    fn url_decode_and_query_parsing() {
        let q = "fn=G&a=C+%2B+O2%28g%29+%3D+CO2%28g%29&b=1000";
        let map = query_map(q);
        assert_eq!(map.get("fn").map(String::as_str), Some("G"));
        assert_eq!(map.get("a").map(String::as_str), Some("C + O2(g) = CO2(g)"));
        assert_eq!(map.get("b").map(String::as_str), Some("1000"));
    }

    #[test]
    fn http_get_health_catalog_units_and_eval() {
        let (db, units) = test_db_and_units();

        let (health, _) = handle_bridge_request(
            &HttpRequest {
                method: "GET".into(),
                path: "/health".into(),
                query: String::new(),
                body: String::new(),
            },
            &db,
            &units,
            8765,
        );
        assert_eq!(health.status, 200);
        let hjson: Value = serde_json::from_str(&health.body).unwrap();
        assert_eq!(hjson["ok"], true);
        assert_eq!(hjson["functions_count"], 83);

        let (eval_mw, _) = handle_bridge_request(
            &HttpRequest {
                method: "GET".into(),
                path: "/eval".into(),
                query: "fn=MW&a=Al2O3".into(),
                body: String::new(),
            },
            &db,
            &units,
            8765,
        );
        assert_eq!(eval_mw.status, 200);
        let mw: f64 = eval_mw.body.trim().parse().expect("numeric MW");
        assert!((mw - 101.961).abs() < 0.1, "got {mw}");

        let (eval_min, _) = handle_bridge_request(
            &HttpRequest {
                method: "GET".into(),
                path: "/json".into(),
                query: "fn=MineralCode&a=Pyrite".into(),
                body: String::new(),
            },
            &db,
            &units,
            8765,
        );
        assert_eq!(eval_min.status, 200);
        let mjson: Value = serde_json::from_str(&eval_min.body).unwrap();
        assert_eq!(mjson["ok"], true);
        assert_eq!(mjson["value"], "Py");
    }

    #[test]
    fn http_post_eval_units_and_streameq() {
        let (db, units) = test_db_and_units();

        // Update global units via POST /units
        let (u_resp, _) = handle_bridge_request(
            &HttpRequest {
                method: "POST".into(),
                path: "/units".into(),
                query: String::new(),
                body: r#"{"temp":"K","energy":"kJ","pressure":"atm"}"#.into(),
            },
            &db,
            &units,
            8765,
        );
        assert_eq!(u_resp.status, 200);
        assert_eq!(units.lock().unwrap().energy_unit, "kJ");

        // Evaluate EnergyU() to confirm updated shared unit
        let (eu, _) = handle_bridge_request(
            &HttpRequest {
                method: "POST".into(),
                path: "/eval".into(),
                query: String::new(),
                body: r#"{"fn":"EnergyU","args":[]}"#.into(),
            },
            &db,
            &units,
            8765,
        );
        let ejson: Value = serde_json::from_str(&eu.body).unwrap();
        assert_eq!(ejson["value"], "kJ");

        // Evaluate StreamEQ via POST /streameq on open-default binary Mg / Mg(g)
        let (seq, _) = handle_bridge_request(
            &HttpRequest {
                method: "POST".into(),
                path: "/streameq".into(),
                query: String::new(),
                body: r#"{"species":["§ PHASE 1:","Mg(g)","§ PHASE 2:","Mg"],"inputs":["25","0","25","1"],"t":1500,"p":1,"units":{"temp":"C","energy":"kcal","pressure":"bar"}}"#.into(),
            },
            &db,
            &units,
            8765,
        );
        assert_eq!(seq.status, 200);
        let sjson: Value = serde_json::from_str(&seq.body).unwrap();
        assert_eq!(sjson["ok"], true, "sjson={sjson}");
        let arr = sjson["outputs"].as_array().expect("outputs array");
        assert_eq!(arr.len(), 4);
    }

    #[test]
    fn live_tcp_listener_roundtrip_eval_and_streameq() {
        let (db, units) = test_db_and_units();
        let mut handle = BridgeHandle::start(db, units, 0).expect("start ephemeral HTTP bridge");
        let port = handle.port();
        assert!(handle.is_running());

        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect bridge");
        stream
            .write_all(
                format!(
                    "GET /eval?fn=MW&a=CO(g) HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            )
            .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        assert!(raw.contains("200 OK"), "resp: {raw}");
        assert!(raw.contains("28.01"), "resp: {raw}");
        assert!(raw.contains("Access-Control-Allow-Origin: *"), "resp: {raw}");

        handle.stop();
        assert!(!handle.is_running());
    }
}
