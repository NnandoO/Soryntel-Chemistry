//! Soryntel Chemistry — Reaction Equations CLI & Local Web UI Server.

use serde::{Deserialize, Serialize};
use soryntel_core::{EnergyUnit, ResultFormat, TemperatureUnit};
use soryntel_db::ThermoDatabase;
use soryntel_solver::balance_reaction;
use soryntel_thermo::{calculate, CalculationRequest};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

const UI_HTML: &str = include_str!("ui.html");

#[derive(Deserialize)]
struct BalanceReq {
    equation: String,
    smallest_coeff: f64,
}

#[derive(Serialize)]
struct BalanceResp {
    balanced: String,
}

#[derive(Serialize)]
struct ErrorResp {
    error: String,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut db = ThermoDatabase::with_open_defaults();

    // Optional external read-only .HSC database path via --hsc-db <path>
    let mut hsc_db_path: Option<String> = None;
    let mut pos_args = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--hsc-db" && i + 1 < args.len() {
            hsc_db_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            pos_args.push(args[i].clone());
            i += 1;
        }
    }

    if let Some(path) = hsc_db_path {
        match db.load_external_hsc_readonly(&path, false) {
            Ok(count) => eprintln!("[soryntel-db] Loaded {count} species from {path} (read-only)"),
            Err(e) => eprintln!("[soryntel-db] Warning: {e}"),
        }
    }

    match pos_args.first().map(|s| s.as_str()) {
        Some("balance") => {
            let eq = pos_args.get(1).expect("Usage: soryntel-app balance \"<equation>\"");
            match balance_reaction(eq, 1.0) {
                Ok((b, _)) => println!("{b}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Some("calc") => {
            let input = pos_args.get(1).expect("Usage: soryntel-app calc \"<formula_or_reaction>\" [from] [to] [step]");
            let t_from: f64 = pos_args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            let t_to: f64 = pos_args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1000.0);
            let t_step: f64 = pos_args.get(4).and_then(|s| s.parse().ok()).unwrap_or(100.0);
            let req = CalculationRequest {
                input: input.clone(),
                t_from,
                t_to,
                t_step,
                temp_unit: TemperatureUnit::Celsius,
                energy_unit: EnergyUnit::Kilojoules,
                result_format: ResultFormat::Normal,
                show_transitions: true,
                criss_cobble: false,
            };
            match calculate(&req, &db) {
                Ok(resp) => println!("{}", serde_json::to_string_pretty(&resp).unwrap()),
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            let port = pos_args
                .iter()
                .position(|s| s == "--port")
                .and_then(|idx| pos_args.get(idx + 1))
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8765);
            run_server(Arc::new(db), port);
        }
    }
}

fn run_server(db: Arc<ThermoDatabase>, port: u16) {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).expect("Failed to bind HTTP server");
    println!("Soryntel Chemistry (Módulo 01: Reaction Equations) listo en: http://{addr}");
    println!("Especies cargadas: {}. Presiona Ctrl+C para salir.", db.len());

    for stream in listener.incoming().flatten() {
        let db_ref = Arc::clone(&db);
        std::thread::spawn(move || handle_client(stream, &db_ref));
    }
}

fn handle_client(mut stream: TcpStream, db: &ThermoDatabase) {
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let req_str = String::from_utf8_lossy(&buf[..n]);
    let first_line = req_str.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");
    let body = req_str.split("\r\n\r\n").nth(1).unwrap_or("");

    let (status, content_type, resp_body) = match (method, path) {
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", UI_HTML.to_string()),
        ("GET", "/api/info") => (
            "200 OK",
            "application/json",
            format!(r#"{{"species_count":{}}}"#, db.len()),
        ),
        ("GET", p) if p.starts_with("/api/species") => {
            let q = p.split("q=").nth(1).unwrap_or("");
            let decoded = url_decode(q);
            let list = db.search(&decoded, 50);
            ("200 OK", "application/json", serde_json::to_string(&list).unwrap())
        }
        ("POST", "/api/balance") => match serde_json::from_str::<BalanceReq>(body) {
            Ok(breq) => match balance_reaction(&breq.equation, breq.smallest_coeff) {
                Ok((balanced, _)) => (
                    "200 OK",
                    "application/json",
                    serde_json::to_string(&BalanceResp { balanced }).unwrap(),
                ),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::to_string(&ErrorResp { error: e.to_string() }).unwrap(),
                ),
            },
            Err(e) => (
                "400 Bad Request",
                "application/json",
                serde_json::to_string(&ErrorResp { error: e.to_string() }).unwrap(),
            ),
        },
        ("POST", "/api/calculate") => match serde_json::from_str::<CalculationRequest>(body) {
            Ok(creq) => match calculate(&creq, db) {
                Ok(resp) => ("200 OK", "application/json", serde_json::to_string(&resp).unwrap()),
                Err(e) => (
                    "400 Bad Request",
                    "application/json",
                    serde_json::to_string(&ErrorResp { error: e.to_string() }).unwrap(),
                ),
            },
            Err(e) => (
                "400 Bad Request",
                "application/json",
                serde_json::to_string(&ErrorResp { error: e.to_string() }).unwrap(),
            ),
        },
        _ => ("404 Not Found", "text/plain", "Not Found".to_string()),
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
        resp_body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn url_decode(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(hex as char);
                i += 3;
                continue;
            }
            out.push('%');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}
