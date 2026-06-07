// Soul Symphony Ling — ทางเข้า Rust
// โหลดไฟล์ .ling ทั้งหมดจากไดเรกทอรี่ เกม/ ตามลำดับการประกาศ
// ต่อแฟ้มเป็นโปรแกรมเดียวแล้วรันผ่านตัวแปลภาษา ling

fn main() {
    let files = &[
        "game/color.ling",
        "game/shape.ling",
        "game/audio.ling",
        "game/title.ling",
        "game/play.ling",
        "game/main.ling",
    ];
}

    let mut source = String::new();
    for path in files {
        match std::fs::read_to_string(path) {
            Ok(s) => {
                source.push_str(&s);
                source.push('\n');
            }
            Err(e) => {
                eprintln!("[soul-symphony-ling] ไม่สามารถโหลด {path}: {e}");
                std::process::exit(1);
            }
        }
    }

    let lang = ling::detect_language(&source);
    if lang != "English" {
        eprintln!("[soul-symphony-ling] ตรวจพบภาษา: {lang}");
    }

    if let Err(e) = ling::run(&source) {
        eprintln!("[soul-symphony-ling] ข้อผิดพลาด: {e}");
        std::process::exit(1);
    }
}
