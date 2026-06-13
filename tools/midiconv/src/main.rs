// midiconv — convert a MIDI file into the ling song-data format consumed by
// soul-symphony-ling:
//   ฟังก์ชัน ข้อมูลเพลง_<Name>()  → [count, [glyphs], [freqs], [xs], [zs]]   (24 collectible lanes)
//   ฟังก์ชัน เวลาโน้ต_<Name>()   → [t0, t1, ...]   (note-onset timeline, seconds)
//   ฟังก์ชัน จำนวนโน้ต_<Name>()  → N
//
// <Name> is taken from the output filename stem (a trailing "_data" is stripped).
//
// Usage:  midiconv <in.mid> <out.ling>

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use midly::{Smf, TrackEventKind, MidiMessage, MetaMessage, Timing};

struct Note { start: f64, end: f64, midi: u8 }

fn freq_of(m: u8) -> f64 { 440.0 * 2f64.powf((m as f64 - 69.0) / 12.0) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: midiconv <in.mid> <out.ling>");
        std::process::exit(2);
    }

    let data = std::fs::read(&args[1]).unwrap_or_else(|e| {
        eprintln!("read {}: {e}", args[1]); std::process::exit(1);
    });
    let smf = Smf::parse(&data).unwrap_or_else(|e| {
        eprintln!("parse midi: {e}"); std::process::exit(1);
    });

    let tpb = match smf.header.timing {
        Timing::Metrical(t) => t.as_int() as f64,
        Timing::Timecode(fps, sub) => fps.as_f32() as f64 * sub as f64,
    };

    enum Ev { Tempo(u32), On(u8, u8), Off(u8, u8) }
    let mut events: Vec<(u64, usize, Ev)> = Vec::new();
    for track in &smf.tracks {
        let mut tick: u64 = 0;
        for (i, te) in track.iter().enumerate() {
            tick += te.delta.as_int() as u64;
            match te.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(us)) => {
                    events.push((tick, i, Ev::Tempo(us.as_int())));
                }
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    if ch == 9 { continue; }   // skip percussion
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            events.push((tick, i, Ev::On(ch, key.as_int())));
                        }
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            events.push((tick, i, Ev::Off(ch, key.as_int())));
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
    events.sort_by_key(|(t, i, _)| (*t, *i));

    let mut cur_tick: u64 = 0;
    let mut cur_sec: f64 = 0.0;
    let mut tempo: f64 = 500_000.0;
    let mut active: HashMap<(u8, u8), f64> = HashMap::new();
    let mut notes: Vec<Note> = Vec::new();

    for (tick, _, ev) in &events {
        cur_sec += (tick - cur_tick) as f64 * (tempo / 1_000_000.0) / tpb;
        cur_tick = *tick;
        match ev {
            Ev::Tempo(us) => tempo = *us as f64,
            Ev::On(ch, key) => { active.insert((*ch, *key), cur_sec); }
            Ev::Off(ch, key) => {
                if let Some(start) = active.remove(&(*ch, *key)) {
                    notes.push(Note { start, end: cur_sec.max(start + 0.04), midi: *key });
                }
            }
        }
    }
    for ((_, key), start) in active {
        notes.push(Note { start, end: start + 0.3, midi: key });
    }
    notes.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    if notes.is_empty() { eprintln!("no notes"); std::process::exit(1); }

    let song_len = notes.iter().map(|n| n.end).fold(0.0f64, f64::max) + 0.5;

    // ── Song name from the output filename stem (strip trailing "_data") ──
    let name = std::path::Path::new(&args[2])
        .file_stem().and_then(|s| s.to_str()).unwrap_or("Song")
        .trim_end_matches("_data").to_string();

    // ── 24 collectible "lanes": sample pitches evenly across the song ──
    const COUNT: usize = 24;
    let mut glyphs: Vec<i32> = Vec::with_capacity(COUNT);
    let mut freqs:  Vec<f64> = Vec::with_capacity(COUNT);
    let mut xs:     Vec<f64> = Vec::with_capacity(COUNT);
    let mut zs:     Vec<f64> = Vec::with_capacity(COUNT);
    for k in 0..COUNT {
        let idx = (k * notes.len() / COUNT).min(notes.len() - 1);
        let m = notes[idx].midi;
        glyphs.push((m as i32).rem_euclid(26));          // pitch → a..z glyph
        freqs.push(freq_of(m));
        let ang = k as f64 * 2.399963;                   // golden-angle scatter
        let rad = 6.0 + ((k % 6) as f64) * 4.0;
        xs.push(ang.cos() * rad);
        zs.push(ang.sin() * rad);
    }

    // ── Onset timeline (subsampled to keep the file modest) ──
    let mut times: Vec<f64> = notes.iter().map(|n| n.start).collect();
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    const MAX_T: usize = 1200;
    if times.len() > MAX_T {
        let step = times.len() as f64 / MAX_T as f64;
        times = (0..MAX_T).map(|i| times[(i as f64 * step) as usize]).collect();
    }

    let j_i = |v: &[i32]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
    let j_f = |v: &[f64], p: usize| v.iter().map(|x| format!("{:.*}", p, x)).collect::<Vec<_>>().join(",");

    // ── Emit ling ──
    let mut out = String::new();
    writeln!(out, "# auto-generated by midiconv — {name} ({song_len:.1}s).  Do not edit.").unwrap();
    writeln!(out, "ฟังก์ชัน ข้อมูลเพลง_{name}() {{").unwrap();
    writeln!(out, "    คืน [{COUNT}, [{}], [{}], [{}], [{}]]",
        j_i(&glyphs), j_f(&freqs, 1), j_f(&xs, 2), j_f(&zs, 2)).unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out, "ฟังก์ชัน เวลาโน้ต_{name}() {{ คืน [{}] }}", j_f(&times, 3)).unwrap();
    writeln!(out, "ฟังก์ชัน จำนวนโน้ต_{name}() {{ คืน {} }}", times.len()).unwrap();

    std::fs::write(&args[2], &out).unwrap();
    eprintln!("wrote {COUNT} lanes + {} onsets ({song_len:.1}s) → {}", times.len(), args[2]);
}
