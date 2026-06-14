// build.rs — bundle runtime assets next to the binary + embed the exe icon, so
// the built .exe runs standalone (it runs with CWD = target/<profile>/).
//
//   ASSET_DIRS  → copied (incrementally) into target/<profile>/<dir>.
//   ASSET_FILES → copied only if absent at the destination, so the exe's
//                 runtime-saved configs (options.cfg / net.cfg) survive rebuilds.
//   ICON        → embedded as the Windows exe icon.
//
// (These mirror [package.metadata.assets] / [package.metadata.icon] in Cargo.toml,
// kept as plain constants here to avoid a TOML-parser build dependency.)

use std::path::{Path, PathBuf};
use std::{env, fs};

const ASSET_DIRS: &[&str] = &["game", "font", "music"];
const ASSET_FILES: &[&str] = &["options.cfg", "net.cfg", "saves.dat"];
const ICON: &str = "assets/ling.ico";

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

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

    for name in ASSET_DIRS {
        let src = manifest_dir.join(name);
        if src.is_dir() {
            copy_dir_incremental(&src, &profile_dir.join(name));
        }
        println!("cargo:rerun-if-changed={}", src.display());
    }

    for name in ASSET_FILES {
        let src = manifest_dir.join(name);
        let dst = profile_dir.join(name);
        if src.is_file() && !dst.exists() {
            let _ = fs::copy(&src, &dst);
        }
        println!("cargo:rerun-if-changed={}", src.display());
    }

    #[cfg(windows)]
    {
        let icon = manifest_dir.join(ICON);
        if icon.exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon(icon.to_str().unwrap());
            let _ = res.compile();
            println!("cargo:rerun-if-changed={}", icon.display());
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
