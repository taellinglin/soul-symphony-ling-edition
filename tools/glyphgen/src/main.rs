// glyphgen — convert an OTF/TTF font into per-letter ling "fill shape" files.
//
// For each lowercase letter a..z it rasterises the glyph with fontdue,
// run-length encodes the coverage into horizontal spans, greedily merges
// vertically adjacent identical spans into rectangles, and emits a .ling
// function that draws those rectangles as filled 3-D triangles oriented on
// an arbitrary plane (origin + right/up basis vectors + scale).
//
// Usage:
//   glyphgen <font.otf> <FontName> <out_root_dir>
//
// Output:
//   <out_root>/<FontName>/<letter>.ling     one fn per non-blank glyph
//   <out_root>/<FontName>.ling              aggregator + วาดอักขระ<FontName> dispatcher

use std::fs;
use std::fmt::Write as _;
use std::path::Path;

const PX: f32 = 56.0; // raster height
const THRESH: u8 = 100; // coverage cutoff

fn fmt_coef(c: f32) -> String {
    // ling negative-literal-safe: wrap negatives as (0.0 - x)
    if c < 0.0 {
        format!("(0.0 - {:.5})", -c)
    } else {
        format!("{:.5}", c)
    }
}

// One world-space coordinate expression for a local glyph point (lx, ly).
// axis is 'x' | 'y' | 'z'; uses params ox/oy/oz, rx/ry/rz (right), ux/uy/uz (up), s (scale).
fn coord(axis: char, lx: f32, ly: f32) -> String {
    format!(
        "o{a} + {LX}*s*r{a} + {LY}*s*u{a}",
        a = axis,
        LX = fmt_coef(lx),
        LY = fmt_coef(ly)
    )
}

fn corner(lx: f32, ly: f32) -> (String, String, String) {
    (coord('x', lx, ly), coord('y', lx, ly), coord('z', lx, ly))
}

#[derive(Clone)]
struct Rect {
    x0: usize,
    x1: usize, // [x0, x1)  exclusive
    y0: usize, // top row
    y1: usize, // [y0, y1)  exclusive, bottom
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: glyphgen <font.otf> <FontName> <out_root_dir>");
        std::process::exit(2);
    }
    let font_path = &args[1];
    let font_name = &args[2];
    let out_root = &args[3];

    let bytes = fs::read(font_path).unwrap_or_else(|e| {
        eprintln!("cannot read {font_path}: {e}");
        std::process::exit(1);
    });
    let font = fontdue::Font::from_bytes(bytes.as_slice(), fontdue::FontSettings::default())
        .unwrap_or_else(|e| {
            eprintln!("cannot parse font: {e}");
            std::process::exit(1);
        });

    let glyph_dir = Path::new(out_root).join(font_name);
    fs::create_dir_all(&glyph_dir).unwrap();

    let mut present: Vec<char> = Vec::new();

    for letter in b'a'..=b'z' {
        let ch = letter as char;
        let (metrics, bitmap) = font.rasterize(ch, PX);
        let w = metrics.width;
        let h = metrics.height;

        if w == 0 || h == 0 || bitmap.iter().all(|&c| c < THRESH) {
            // blank glyph — skip entirely
            continue;
        }

        if std::env::var("GLYPH_PREVIEW").is_ok() {
            eprintln!("--- '{ch}'  {w}x{h}  xmin={} ymin={} adv={:.1}", metrics.xmin, metrics.ymin, metrics.advance_width);
            for row in 0..h {
                let line: String = (0..w)
                    .map(|cc| if bitmap[row * w + cc] >= THRESH { '#' } else { '.' })
                    .collect();
                eprintln!("{line}");
            }
        }

        // ── run-length per row, greedy vertical merge into rectangles ──
        let mut rects: Vec<Rect> = Vec::new();
        let mut open: Vec<Rect> = Vec::new(); // rects still growing downward

        for row in 0..h {
            // spans in this row
            let mut spans: Vec<(usize, usize)> = Vec::new();
            let mut c = 0usize;
            while c < w {
                if bitmap[row * w + c] >= THRESH {
                    let start = c;
                    while c < w && bitmap[row * w + c] >= THRESH {
                        c += 1;
                    }
                    spans.push((start, c));
                } else {
                    c += 1;
                }
            }

            let mut used = vec![false; spans.len()];
            let mut next_open: Vec<Rect> = Vec::new();

            for mut r in open.drain(..) {
                // continue this rect if an identical span exists in this row
                if let Some(i) = spans
                    .iter()
                    .enumerate()
                    .position(|(i, &(a, b))| !used[i] && a == r.x0 && b == r.x1)
                {
                    used[i] = true;
                    r.y1 = row + 1;
                    next_open.push(r);
                } else {
                    rects.push(r); // closed
                }
            }
            for (i, &(a, b)) in spans.iter().enumerate() {
                if !used[i] {
                    next_open.push(Rect { x0: a, x1: b, y0: row, y1: row + 1 });
                }
            }
            open = next_open;
        }
        rects.extend(open.drain(..));

        // ── normalisation: centre glyph bbox at local origin, height ≈ h/PX ──
        let xmin = metrics.xmin as f32;
        let ymin = metrics.ymin as f32;
        let cx = (xmin + w as f32 * 0.5) / PX;
        let cy = (ymin + h as f32 * 0.5) / PX;

        let mut body = String::new();
        writeln!(
            body,
            "# ─ {font_name} '{ch}' — รูปอักขระเวกเตอร์ (สร้างอัตโนมัติจาก {fp}) ─",
            fp = Path::new(font_path).file_name().unwrap().to_string_lossy()
        )
        .unwrap();
        writeln!(
            body,
            "ฟังก์ชัน อักขระ_{font_name}_{ch}(ox,oy,oz, rx,ry,rz, ux,uy,uz, s) {{"
        )
        .unwrap();

        for r in &rects {
            let lx_l = (xmin + r.x0 as f32) / PX - cx;
            let lx_r = (xmin + r.x1 as f32) / PX - cx;
            let ly_t = (ymin + (h - r.y0) as f32) / PX - cy; // top row -> larger y
            let ly_b = (ymin + (h - r.y1) as f32) / PX - cy;

            let (tlx, tly, tlz) = corner(lx_l, ly_t);
            let (trx, tryy, trz) = corner(lx_r, ly_t);
            let (brx, bry, brz) = corner(lx_r, ly_b);
            let (blx, bly, blz) = corner(lx_l, ly_b);

            writeln!(
                body,
                "    วาดสามเหลี่ยม3มิติ({tlx},{tly},{tlz},  {trx},{tryy},{trz},  {brx},{bry},{brz})"
            )
            .unwrap();
            writeln!(
                body,
                "    วาดสามเหลี่ยม3มิติ({tlx},{tly},{tlz},  {brx},{bry},{brz},  {blx},{bly},{blz})"
            )
            .unwrap();
        }

        writeln!(body, "}}").unwrap();

        let file = glyph_dir.join(format!("{ch}.ling"));
        fs::write(&file, body).unwrap();
        present.push(ch);
        eprintln!("  {ch}: {} rects", rects.len());
    }

    // ── aggregator + dispatcher ──
    let mut agg = String::new();
    writeln!(
        agg,
        "# ───────────────────────────────────────────────────────────────"
    )
    .unwrap();
    writeln!(
        agg,
        "# {font_name}.ling — ดัชนีรูปอักขระเวกเตอร์ (สร้างอัตโนมัติ — อย่าแก้ด้วยมือ)"
    )
    .unwrap();
    writeln!(
        agg,
        "# วาดอักขระ{font_name}(ดัชนี, ox,oy,oz, rx,ry,rz, ux,uy,uz, s)"
    )
    .unwrap();
    writeln!(
        agg,
        "# ───────────────────────────────────────────────────────────────"
    )
    .unwrap();
    for ch in &present {
        writeln!(agg, "ใช้ \"{font_name}/{ch}\"").unwrap();
    }
    writeln!(agg).unwrap();
    writeln!(
        agg,
        "ฟังก์ชัน วาดอักขระ{font_name}(ดัชนี, ox,oy,oz, rx,ry,rz, ux,uy,uz, s) {{"
    )
    .unwrap();
    for ch in &present {
        let idx = (*ch as u8 - b'a') as usize;
        writeln!(
            agg,
            "    ถ้า ดัชนี == {idx} {{ อักขระ_{font_name}_{ch}(ox,oy,oz, rx,ry,rz, ux,uy,uz, s) }}"
        )
        .unwrap();
    }
    writeln!(agg, "}}").unwrap();

    let agg_file = Path::new(out_root).join(format!("{font_name}.ling"));
    fs::write(&agg_file, agg).unwrap();

    eprintln!(
        "wrote {} glyphs + {}.ling dispatcher to {}",
        present.len(),
        font_name,
        out_root
    );
}
