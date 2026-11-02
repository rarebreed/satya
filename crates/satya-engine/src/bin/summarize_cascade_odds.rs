//! CLI binary for summarizing Satya Two-List Cascading Dice odds across $K \in \{10, 20, 100\}$ and Skill $N$.
//!
//! Ported from `src/satya_ml/summarize_cascade_odds.py`.
//!
//! Usage:
//! ```bash
//! cargo run -p satya-engine --bin summarize-cascade-odds -- --calibration k20_full
//! cargo run -p satya-engine --bin summarize-cascade-odds -- --calibration c_vs_n
//! cargo run -p satya-engine --bin summarize-cascade-odds -- --calibration k10_all
//! ```

use satya_engine::summarize_cascade_odds::{CalibrationPreset, run_calibration_report};
use std::env;
use std::process;
use std::str::FromStr;

fn print_usage(prog: &str) {
    eprintln!(
        "Usage: {prog} [--calibration {{midpoint_2c|raw_2k_minus_c|both|k20_full|c_vs_n|k10_all}}]"
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = args
        .first()
        .map(String::as_str)
        .unwrap_or("summarize-cascade-odds");

    let mut preset = CalibrationPreset::Both;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage(prog);
                return;
            }
            "--calibration" => {
                i += 1;
                let Some(val) = args.get(i) else {
                    eprintln!("Error: --calibration requires an argument.");
                    print_usage(prog);
                    process::exit(2);
                };
                match CalibrationPreset::from_str(val) {
                    Ok(p) => preset = p,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        print_usage(prog);
                        process::exit(2);
                    }
                }
            }
            arg if arg.starts_with("--calibration=") => {
                let val = &arg["--calibration=".len()..];
                match CalibrationPreset::from_str(val) {
                    Ok(p) => preset = p,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        print_usage(prog);
                        process::exit(2);
                    }
                }
            }
            other => {
                eprintln!("Error: Unrecognized argument '{other}'.");
                print_usage(prog);
                process::exit(2);
            }
        }
        i += 1;
    }

    println!("{}", run_calibration_report(preset));
}
