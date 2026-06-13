// build.rs — bundle runtime assets next to the binary + embed the exe icon.
//
// Reads Cargo.toml metadata:
//   [package.metadata.assets] dirs = [...]   → copied (incrementally) into
//                                              target/<profile>/<dir> so the
//                                              .exe runs standalone.
//   [package.metadata.icon]   path = "..."   → embedded as the Windows exe icon.
//
// The music/ folder is ~1 GB, so copying is incremental (size + mtime check).

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let cargo_toml = fs::read_to_string(manifest_dir.join("Cargo.toml")).unwrap_or_default();
    let parsed: toml::Value = cargo_toml.parse().unwrap_or(toml::Value::Table(Default::default()));
    let meta = parsed.get("package").and_then(|p| p.get("metadata"));

    // target/<profile> (where the binary lands): OUT_DIR is
    // target/<profile>/build/<crate-hash>/out → 3 levels up.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            manifest_dir
                .join("target")
                .join(env::var("PROFILE").unwrap_or_else(|_| "debug".into()))
        });

    if let Some(dirs) = meta
        .and_then(|m| m.get("assets"))
        .and_then(|a| a.get("dirs"))
        .and_then(|d| d.as_array())
    {
        for d in dirs {
            if let Some(name) = d.as_str() {
                let src = manifest_dir.join(name);
                if src.is_dir() {
                    copy_dir_incremental(&src, &profile_dir.join(name));
                }
                println!("cargo:rerun-if-changed={}", src.display());
            }
        }
    }

    #[cfg(windows)]
    {
        if let Some(icon) = meta
            .and_then(|m| m.get("icon"))
            .and_then(|i| i.get("path"))
            .and_then(|p| p.as_str())
        {
            let full = manifest_dir.join(icon);
            if full.exists() {
                let mut res = winresource::WindowsResource::new();
                res.set_icon(full.to_str().unwrap());
                let _ = res.compile();
                println!("cargo:rerun-if-changed={}", full.display());
            }
        }
    }

    println!("cargo:rerun-if-changed=Cargo.toml");
}

/// Recursively copy `src` into `dst`, skipping files whose destination already
/// matches by size + mtime (keeps the ~1 GB music copy cheap on rebuilds).
fn copy_dir_incremental(src: &Path, dst: &Path) {
    if fs::create_dir_all(dst).is_err() {
        return;
    }
    let entries = match fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_incremental(&path, &target);
        } else {
            let need = match (fs::metadata(&path), fs::metadata(&target)) {
                (Ok(s), Ok(t)) => match (s.modified(), t.modified()) {
                    (Ok(sm), Ok(tm)) => sm > tm || s.len() != t.len(),
                    _ => true,
                },
                _ => true,
            };
            if need {
                let _ = fs::copy(&path, &target);
            }
        }
    }
}
