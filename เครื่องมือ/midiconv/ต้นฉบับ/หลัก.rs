// midiconv — convert a MIDI file into a pre-rasterized ling lookup table.
//
// Emits per-game-frame arrays so the ling player is fully stateless (O(1)):
//   slot = กรอบ mod song_frames
//   freq = list_get(song_freq_N, slot)   (0.0 = silence)
//   key  = list_get(song_key_N,  slot)   (-1  = silence)
//
// Usage:  midiconv <in.mid> <out.ling> [fps]   default fps = 30

use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use midly::{Smf, TrackEventKind, MidiMessage, MetaMessage, Timing};

const LANES: usize = 8;

struct Note { start: f64, end: f64, midi: u8 }

fn freq_of(m: u8) -> f64 { 440.0 * 2f64.powf((m as f64 - 69.0) / 12.0) }

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: midiconv <in.mid> <out.ling> [fps]");
        std::process::exit(2);
    }
    let fps: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(30.0);

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
                    if ch == 9 { continue; }
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

    // Key mapping: spread song's pitch range → 0..255
    let min_n = notes.iter().map(|n| n.midi).min().unwrap() as f64;
    let max_n = notes.iter().map(|n| n.midi).max().unwrap() as f64;
    let span = (max_n - min_n).max(1.0);
    let key_of = |m: u8| -> i32 {
        (((m as f64 - min_n) / span) * 255.0).round().clamp(0.0, 255.0) as i32
    };

    // Greedy lane assignment
    let mut lane_end = [f64::NEG_INFINITY; LANES];
    let mut lanes: Vec<Vec<&Note>> = vec![Vec::new(); LANES];
    for n in &notes {
        let mut placed = false;
        for l in 0..LANES {
            if lane_end[l] <= n.start + 1e-6 {
                lanes[l].push(n);
                lane_end[l] = n.end;
                placed = true;
                break;
            }
        }
        if !placed { /* drop */ }
    }

    // Pre-rasterize at `fps` frames per second
    let n_frames = (song_len * fps).ceil() as usize;
    let mut freq_grid: Vec<Vec<f64>> = vec![vec![0.0; n_frames]; LANES];
    let mut key_grid:  Vec<Vec<i32>> = vec![vec![-1;  n_frames]; LANES];

    for (l, lane) in lanes.iter().enumerate() {
        for n in lane {
            let f_start = (n.start * fps).floor() as usize;
            let f_end   = ((n.end * fps).ceil() as usize).min(n_frames);
            for fi in f_start..f_end {
                freq_grid[l][fi] = freq_of(n.midi);
                key_grid[l][fi]  = key_of(n.midi);
            }
        }
    }

    // Emit ling
    let mut out = String::new();
    writeln!(out, "# ── SubsynthWinter_3.ling — pre-rasterised MIDI lookup ({fps} fps) ──").unwrap();
    writeln!(out, "# Do not edit by hand.  Regenerate with midiconv.").unwrap();
    writeln!(out, "ผูก song_len = {song_len:.4}").unwrap();
    writeln!(out, "ผูก song_fps = {fps:.1}").unwrap();
    writeln!(out, "ผูก song_frames = {n_frames}").unwrap();

    for l in 0..LANES {
        let freqs: String = freq_grid[l].iter()
            .map(|f| format!("{f:.2}"))
            .collect::<Vec<_>>().join(",");
        let keys: String = key_grid[l].iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>().join(",");
        writeln!(out, "ผูก song_freq_{l} = [{freqs}]").unwrap();
        writeln!(out, "ผูก song_key_{l}  = [{keys}]").unwrap();
    }

    std::fs::write(&args[2], &out).unwrap();
    eprintln!("wrote {n_frames} frames × {LANES} lanes  ({song_len:.1}s @ {fps}fps) → {}", args[2]);
}
