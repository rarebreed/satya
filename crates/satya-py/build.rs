//! Build script for `satya-py`.
//!
//! When `PYO3_NO_PYTHON=1` is active on Windows MSVC (such as in a sandboxed offline build),
//! generates a PEP 384 `python3.lib` import library in `OUT_DIR` from the `pyo3-ffi` symbol
//! declarations using MSVC `lib.exe` so both `cdylib` (`satya_py.dll` / `.pyd`) and `cargo test`
//! link cleanly without requiring outside-workspace access to a system Python installation.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn extract_ident(after_kw: &str) -> Option<&str> {
    let trimmed = after_kw.trim_start();
    let end = trimmed
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(trimmed.len());
    let ident = &trimmed[..end];
    if ident.starts_with("Py") || ident.starts_with("_Py") {
        Some(ident)
    } else {
        None
    }
}

fn find_pyo3_ffi_src() -> Option<PathBuf> {
    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)?;
    let reg_src = home.join(".cargo").join("registry").join("src");
    let entries = fs::read_dir(reg_src).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("pyo3-ffi-0.27.2").join("src");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn find_msvc_lib_exe() -> PathBuf {
    let known = PathBuf::from(
        r"C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\HostX64\x64\lib.exe",
    );
    if known.exists() {
        return known;
    }
    PathBuf::from("lib.exe")
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let Some(out_dir) = env::var_os("OUT_DIR").map(PathBuf::from) else {
        return;
    };
    let lib_path = out_dir.join("python3.lib");
    if let Some(ffi_src) = find_pyo3_ffi_src() {
        let mut rs_files = Vec::new();
        collect_rs_files(&ffi_src, &mut rs_files);

        let mut fn_syms = BTreeSet::new();
        let mut data_syms = BTreeSet::new();

        for extra_fn in ["_Py_DecRef", "_Py_IncRef", "Py_DecRef", "Py_IncRef"] {
            fn_syms.insert(extra_fn.to_string());
        }
        for extra_data in [
            "_Py_NoneStruct",
            "_Py_TrueStruct",
            "_Py_FalseStruct",
            "_Py_NotImplementedStruct",
            "_Py_EllipsisObject",
            "_PyWeakref_ProxyType",
            "_PyWeakref_CallableProxyType",
            "_PyByteArray_empty_string",
        ] {
            data_syms.insert(extra_data.to_string());
        }

        for file in rs_files {
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            for line in text.lines() {
                let s = line.trim();
                if let Some(rest) = s.strip_prefix("pub fn ") {
                    if let Some(id) = extract_ident(rest) {
                        fn_syms.insert(id.to_string());
                    }
                } else if let Some(rest) = s.strip_prefix("pub unsafe fn ") {
                    if let Some(id) = extract_ident(rest) {
                        fn_syms.insert(id.to_string());
                    }
                } else if let Some(rest) = s.strip_prefix("pub static mut ") {
                    if let Some(id) = extract_ident(rest) {
                        data_syms.insert(id.to_string());
                    }
                } else if let Some(rest) = s.strip_prefix("pub static ")
                    && let Some(id) = extract_ident(rest)
                {
                    data_syms.insert(id.to_string());
                }
            }
        }

        let mut def_contents = String::from("LIBRARY python3.dll\nEXPORTS\n");
        for sym in &fn_syms {
            if !data_syms.contains(sym) {
                def_contents.push_str("    ");
                def_contents.push_str(sym);
                def_contents.push('\n');
            }
        }
        for sym in &data_syms {
            def_contents.push_str("    ");
            def_contents.push_str(sym);
            def_contents.push_str(" DATA\n");
        }

        let def_path = out_dir.join("python3.def");
        let _ = fs::write(&def_path, def_contents);

        let lib_exe = find_msvc_lib_exe();
        let _ = Command::new(lib_exe)
            .arg("/NOLOGO")
            .arg("/MACHINE:X64")
            .arg(format!("/DEF:{}", def_path.display()))
            .arg(format!("/OUT:{}", lib_path.display()))
            .status();
    }

    if lib_path.exists() {
        println!("cargo:rustc-link-search=native={}", out_dir.display());
    }
}
