// phrasebake — bake a single shaped phrase (any script) into one self-contained
// ling draw function: วาด<Name>(ox, oy, sc).
//
// rustybuzz shapes the phrase (handles Thai reordering/marks + CJK), fontdue
// rasterises each shaped glyph, the coverage is run-length → rectangles →
// filled triangles, positioned by the shaped pen advances and centred.
//
//   phrasebake <font.ttf> <Name> <out_file.ling> "<phrase>"

use std::fs;
use std::fmt::Write as _;

const PX: f32 = 64.0; // raster height (pixels per em)
const THRESH: u8 = 100; // coverage cutoff

fn fmt_coef(c: f32) -> String {
    if c < 0.0 { format!("(0.0 - {:.5})", -c) } else { format!("{:.5}", c) }
}

#[derive(Clone)]
struct Rect { x0: usize, x1: usize, y0: usize, y1: usize }

// Run-length per row, greedily merge vertically adjacent identical spans.
fn rects_of(w: usize, h: usize, bitmap: &[u8]) -> Vec<Rect> {
    let mut rects: Vec<Rect> = Vec::new();
    let mut open: Vec<Rect> = Vec::new();
    for row in 0..h {
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut c = 0usize;
        while c < w {
            if bitmap[row * w + c] >= THRESH {
                let start = c;
                while c < w && bitmap[row * w + c] >= THRESH { c += 1; }
                spans.push((start, c));
            } else { c += 1; }
        }
        let mut used = vec![false; spans.len()];
        let mut next_open: Vec<Rect> = Vec::new();
        for mut r in open.drain(..) {
            if let Some(i) = spans.iter().enumerate()
                .position(|(i, &(a, b))| !used[i] && a == r.x0 && b == r.x1)
            {
                used[i] = true; r.y1 = row + 1; next_open.push(r);
            } else { rects.push(r); }
        }
        for (i, &(a, b)) in spans.iter().enumerate() {
            if !used[i] { next_open.push(Rect { x0: a, x1: b, y0: row, y1: row + 1 }); }
        }
        open = next_open;
    }
    rects.extend(open.drain(..));
    rects
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: phrasebake <font.ttf> <Name> <out_file.ling> \"<phrase>\"");
        std::process::exit(2);
    }
    let (font_path, name, out_file, phrase) = (&args[1], &args[2], &args[3], &args[4]);

    let data = fs::read(font_path).unwrap_or_else(|e| { eprintln!("read {font_path}: {e}"); std::process::exit(1); });
    let rb_face = rustybuzz::Face::from_slice(&data, 0).unwrap_or_else(|| { eprintln!("rustybuzz: bad font"); std::process::exit(1); });
    let fd_font = fontdue::Font::from_bytes(data.as_slice(), fontdue::FontSettings::default())
        .unwrap_or_else(|e| { eprintln!("fontdue: {e}"); std::process::exit(1); });

    let upm = rb_face.units_per_em() as f32;

    // shape
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(phrase);
    let shaped = rustybuzz::shape(&rb_face, &[], buffer);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();

    // First pass: total advance (em units) for centring.
    let mut total_adv_em = 0.0f32;
    for p in positions { total_adv_em += p.x_advance as f32 / upm; }
    let center_x = total_adv_em * 0.5;
    let center_y = 0.35; // rough vertical centre (cap-height-ish above baseline)

    // Build triangles.
    let mut tris = String::new();
    let mut n_tri = 0usize;
    let mut cursor_em = 0.0f32; // pen x in em units
    for (info, pos) in infos.iter().zip(positions.iter()) {
        let gid = info.glyph_id as u16;
        let (m, bitmap) = fd_font.rasterize_indexed(gid, PX);
        let (w, h) = (m.width, m.height);
        let pen_x_em = cursor_em + pos.x_offset as f32 / upm;
        let pen_y_em = pos.y_offset as f32 / upm;
        if w != 0 && h != 0 && bitmap.iter().any(|&c| c >= THRESH) {
            // glyph origin (left/bottom bearing) in em units, fontdue px → em via /PX
            let gl = pen_x_em + m.xmin as f32 / PX;
            let gb = pen_y_em + m.ymin as f32 / PX;
            for r in rects_of(w, h, &bitmap) {
                let lx_l = gl + r.x0 as f32 / PX - center_x;
                let lx_r = gl + r.x1 as f32 / PX - center_x;
                let ly_t = gb + (h - r.y0) as f32 / PX - center_y; // top row → larger y
                let ly_b = gb + (h - r.y1) as f32 / PX - center_y;
                // billboard: right=(1,0,0), up=(0,-1,0) → world = (ox+lx*sc, oy-ly*sc, 0)
                let p = |x: f32, y: f32| format!("ox+{}*sc, oy-{}*sc, 0.0", fmt_coef(x), fmt_coef(y));
                writeln!(tris, "    วาดสามเหลี่ยม3มิติ({},  {},  {})", p(lx_l, ly_t), p(lx_r, ly_t), p(lx_r, ly_b)).unwrap();
                writeln!(tris, "    วาดสามเหลี่ยม3มิติ({},  {},  {})", p(lx_l, ly_t), p(lx_r, ly_b), p(lx_l, ly_b)).unwrap();
                n_tri += 2;
            }
        }
        cursor_em += pos.x_advance as f32 / upm;
    }

    let mut out = String::new();
    writeln!(out, "# {name} — baked phrase \"{phrase}\" (auto-gen by phrasebake; do not edit)").unwrap();
    writeln!(out, "ฟังก์ชัน วาด{name}(ox, oy, sc) {{").unwrap();
    out.push_str(&tris);
    writeln!(out, "}}").unwrap();
    fs::write(out_file, out).unwrap();
    eprintln!("wrote {name} ({} glyphs, {n_tri} tris, width {:.2}em) → {out_file}", infos.len(), total_adv_em);
}
