// Soul Symphony Ling — ทางเข้า Rust
// โหลดไฟล์ .ling ทั้งหมดจากไดเรกทอรี่ เกม/ ตามลำดับการประกาศ
// ต่อแฟ้มเป็นโปรแกรมเดียวแล้วรันผ่านตัวแปลภาษา ling

use std::path::{Path, PathBuf};

// หาโฟลเดอร์ฐานที่มี game/ ฟอนต์ และเพลง — รองรับทั้งรันจากรากโปรเจกต์
// (ตอนพัฒนา) และรันไฟล์ .exe ที่ build แล้วโดยมี asset คัดลอกไว้ข้าง ๆ
fn find_base() -> PathBuf {
    // 1) ไดเรกทอรี่ปัจจุบัน ถ้ามี game/ (รันจากรากโปรเจกต์)
    if Path::new("game").is_dir() {
        return std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    }
    // 2) ข้าง ๆ ไฟล์ปฏิบัติการ (release: asset ถูกคัดลอกไว้ข้าง .exe)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("game").is_dir() {
                return dir.to_path_buf();
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn main() {
    let base = find_base();
    // โหลด asset ตอนรันไทม์ (เพลง WAV, เซฟ) อ้างอิงจากไดเรกทอรี่ปัจจุบัน
    // จึงตั้ง CWD = base ให้พาธเช่น "music/..." ชี้ถูกที่
    let _ = std::env::set_current_dir(&base);

    let game_dir = base.join("game");
    let files = &[
        "color.ling",
        "shape.ling",
        "audio.ling",
        "title.ling",
        "play.ling",
        "main.ling",
    ];

    let mut source = String::new();
    for path in files {
        let full = game_dir.join(path);
        match std::fs::read_to_string(&full) {
            Ok(s) => {
                source.push_str(&s);
                source.push('\n');
            }
            Err(e) => {
                eprintln!("[soul-symphony-ling] ไม่สามารถโหลด {}: {e}", full.display());
                std::process::exit(1);
            }
        }
    }

    let lang = ling::detect_language(&source);
    if lang != "English" {
        eprintln!("[soul-symphony-ling] ตรวจพบภาษา: {lang}");
    }

    // คำสั่ง `use "../font/..."` ในไฟล์เกมเขียนแบบสัมพัทธ์กับ game/
    // จึงส่ง game/ เป็น source_dir เพื่อให้ import ฟอนต์/เพลงชี้ถูกที่
    // รันผ่าน Cranelift JIT (ไม่ใช้ตัวแปลภาษา) เพื่อความเร็วระดับ native
    if let Err(e) = ling::run_jit(&source, Some(game_dir), Some("main.ling")) {
        eprintln!("[soul-symphony-ling] ข้อผิดพลาด: {e}");
        std::process::exit(1);
    }
}
