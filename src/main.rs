use std::{
    collections::VecDeque,
    fs,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    },
};

use clap::Parser;
use eyre::{ensure, Result};
use itertools::Itertools;
use midly::{num::u28, TrackEvent};

mod dsl;
mod duration;
mod midi;
mod notes;
mod player;
mod sequence;
mod theory;

use duration::Dur;
use player::Player;
use sixtyfps::Model;

use crate::sequence::Event;

const TICKS_PER_BEAT: u16 = 100;
const ZERO_TICKS: u28 = u28::new(0);
const BEAT: u28 = u28::new(TICKS_PER_BEAT as u32);

fn write_some_midi() {
    let header = midly::Header {
        format: midly::Format::Sequential,
        timing: midly::Timing::Metrical(TICKS_PER_BEAT.into()),
    };
    let track_one: Vec<midly::TrackEvent> = vec![
        TrackEvent {
            delta: ZERO_TICKS,
            kind: midly::TrackEventKind::Midi {
                channel: 1.into(),
                message: midly::MidiMessage::NoteOn {
                    key: 60.into(),
                    vel: 70.into(),
                },
            },
        },
        TrackEvent {
            delta: Dur::beats(1).into(),
            kind: midly::TrackEventKind::Midi {
                channel: 1.into(),
                message: midly::MidiMessage::NoteOff {
                    key: 60.into(),
                    vel: 70.into(),
                },
            },
        },
    ];
    let smf = midly::Smf {
        header,
        tracks: vec![track_one],
    };
    smf.save("out.mid").unwrap();
    println!("Wrote out.mid!");
}

#[derive(Debug, Parser)]
#[clap(about, version, author)]
struct Args {
    /// Path to MIDI file to rip off
    path: String,
    /// Order of the markov chain
    #[clap(long, default_value_t = 1)]
    order: usize,
    /// Tempo
    #[clap(long, default_value_t = 120)]
    tempo: usize,
    /// Ticks per beat
    #[clap(long, default_value_t = 24)]
    ticks_per_beat: u32,
    /// Number of events to use per chunk
    #[clap(long, default_value_t = usize::MAX)]
    chunk_size: usize,
    /// The humanization delay range (±ms/2)
    #[clap(long)]
    human_ms: Option<u8>,
    /// The humanization velocity range (±vel/2)
    #[clap(long)]
    human_vel: Option<u8>,
    /// Show the UI
    #[clap(long)]
    ui: bool,
    /// Show the UI
    #[clap(long)]
    dot_file: Option<String>,
}

// #[derive(Clone, Debug)]
// pub struct UIKey {
//     key: i32,
//     on: bool,
//     octave: i32,
//     octave_key: i32,
//     is_black: bool,
// }

fn key_offset(key: u8) -> i32 {
    let oct = key / 12;
    let oct_k = key % 12;
    match oct_k {
        1 | 3 | 6 | 8 | 10 => oct_k / 2,
        x if x < 5 => oct_k / 2,
        _ => oct_k / 2 + 1,
    }
    .into()
}

struct UIPlayer<'a> {
    pub player: Player<'a>,
    pub notes_on: [(u8, u32, u32); 128], // (vel, start_tick, end_tick)
    pub history_size: usize,
    pub note_history: VecDeque<PlayedNote>,
}

impl<'a> UIPlayer<'a> {
    fn new(player: Player<'a>) -> Self {
        Self {
            player,
            notes_on: [(0, 0, 0); 128],
            history_size: 128,
            note_history: VecDeque::with_capacity(128),
        }
    }

    pub fn event(&mut self, event: &Event) -> Result<()> {
        self.player.event(event)?;
        let ticks_played = self.player.ticks_played();
        match *event {
            Event::PlayNoteTicks {
                key,
                dynamic,
                ticks,
            } => {
                let k = key.as_int() as usize;
                let end_ticks = ticks_played + ticks;
                self.notes_on[k] = (dynamic.as_int(), ticks_played, end_ticks);
                while self.note_history.len() >= self.note_history.capacity() {
                    self.note_history.pop_front();
                }
                self.note_history.push_back(PlayedNote {
                    end: end_ticks.try_into()?,
                    key: key.as_int().into(),
                    offset: key_offset(key.as_int()),
                    start: ticks_played.try_into()?,
                    oct: (key.as_int() / 12) as i32 - 2,
                    oct_k: (key.as_int() % 12).into(),
                    vel: dynamic.as_int().into(),
                });
            }
            _ => {}
        }
        self.notes_on
            .iter()
            .enumerate()
            .filter(|(_, &(vel, _, stop))| vel > 0 && stop < ticks_played)
            .map(|(key, &(_, start, stop))| (key, start, stop))
            .collect::<Vec<_>>()
            .iter()
            .for_each(|&(key, start, stop)| self.notes_on[key] = (0, start, stop));
        Ok(())
    }

    pub fn ticks_played(&self) -> u32 {
        self.player.ticks_played()
    }

    pub fn tick_dur(&self) -> std::time::Duration {
        self.player.tick_dur()
    }
}

sixtyfps::include_modules!();
fn main() -> Result<()> {
    let args = Args::parse();

    println!("generating with order {} chain", args.order);

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;

    // let data = fs::read("1st Mvmt Sonata No.14, Opus 27, No.2.mid")?;
    let data = fs::read(args.path)?;
    let midi_parser = midi::Parser::default().with_ticks_per_beat(args.ticks_per_beat);
    let seq = midi_parser.parse_seq(&data, 0)?;

    let mut player = player::Player::new("Bobs thing");
    player.set_ticks_per_beat(seq.ticks_per_beat());
    player.set_tempo(args.tempo as f32);
    player.connect("loopMIDI Port")?;
    if let Some(ms) = args.human_ms {
        player.set_human_ms_range(ms as f64);
    }
    if let Some(vel) = args.human_vel {
        player.set_human_vel_range(vel as f64);
    }

    ensure!(!seq.events.is_empty(), "no events");

    // // play the original
    // for ev in &seq.events {
    //     if term.load(Ordering::Relaxed) {
    //         break;
    //     }
    //     player.event(ev);
    // }

    // generate some new material
    let mut seq_chain = markov::Chain::of_order(args.order);
    let iter = seq.events.into_iter();
    let rev_iter = iter.clone().rev();
    let iter = iter.chain(rev_iter);
    // let quieter = iter.clone().map(|e| {
    //     if let Event::PlayNote { key, dynamic } = e {
    //         Event::play(key, dynamic.down())
    //     } else {
    //         e
    //     }
    // });
    // let iter = iter.chain(quieter);
    let chunk_size: usize = args.chunk_size;
    for chunk in &iter.chunks(chunk_size) {
        let tokens = chunk.collect::<Vec<_>>();
        seq_chain.feed(tokens);
    }

    if let Some(path) = args.dot_file {
        let graph = seq_chain.graph();
        let dot = petgraph::dot::Dot::with_config(&graph, &[]);
        std::fs::write(path, format!("{:?}", dot)).unwrap();
    }

    if args.ui {
        let main = MainWindow::new();
        // let mut keys = main.get_keys().iter().collect();
        let abc = [
            "C", "C#/Db", "D", "D#/Eb", "E", "F", "F#/Gb", "G", "G#/Ab", "A", "A#/Bb", "B",
        ];
        let keys: Vec<UIKey> = (0usize..=127)
            .map(|k| (k, k % 12))
            .map(|(k, oct_k)| UIKey {
                key: k as i32,
                vel: 0,
                oct: (k / 12) as i32 - 2,
                note: sixtyfps::SharedString::from(abc[oct_k]),
                black: [1, 3, 6, 8, 10].contains(&oct_k),
                offset: match oct_k {
                    1 | 3 | 6 | 8 | 10 => oct_k / 2,
                    x if x < 5 => oct_k / 2,
                    _ => oct_k / 2 + 1,
                } as i32,
            })
            .sorted_by_key(|x| x.black)
            .collect();
        // for k in &keys {
        //     println!("\t\t{:?},", k);
        // }
        let keys_model = Rc::new(sixtyfps::VecModel::from(keys));
        main.set_keys(sixtyfps::ModelHandle::new(keys_model.clone()));
        let mut player = UIPlayer::new(player);
        let clocked_ticks = Arc::new(AtomicU32::new(0));
        let tick_dur = player.tick_dur();

        let thread_arc = clocked_ticks.clone();
        let handle_weak = main.as_weak();
        std::thread::spawn(move || {
            let clocked_ticks = thread_arc;
            loop {
                std::thread::sleep(tick_dur);
                let prev = clocked_ticks.fetch_add(1, Ordering::Relaxed);
                let main_copy = handle_weak.clone();
                sixtyfps::invoke_from_event_loop(move || {
                    let main = main_copy.unwrap();
                    main.set_ticks_played((prev + 1) as i32);
                });
            }
        });

        let handle_weak = main.as_weak();
        let thread_arc = clocked_ticks.clone();
        std::thread::spawn(move || {
            let clocked_ticks = thread_arc;
            for ev in seq_chain.iter().flatten() {
                if term.load(Ordering::Relaxed) {
                    break;
                }
                player.event(&ev).expect("failed playing event");
                let ticks_played = player.ticks_played();
                if let Event::Wait { ticks: _ } = &ev {
                    clocked_ticks.store(ticks_played, Ordering::Relaxed);
                }
                let clocked = clocked_ticks.load(Ordering::Relaxed);
                let main_copy = handle_weak.clone();
                let history: Vec<PlayedNote> = player.note_history.clone().into();
                sixtyfps::invoke_from_event_loop(move || {
                    let main = main_copy.unwrap();
                    let keys_model = main.get_keys();
                    keys_model
                        .iter()
                        .enumerate()
                        .map(|(i, uik)| {
                            let (vel, _, _) = player.notes_on[uik.key as usize];
                            (
                                i,
                                UIKey {
                                    vel: vel as i32,
                                    ..uik
                                },
                            )
                        })
                        .for_each(|(i, uik)| {
                            keys_model.set_row_data(i, uik);
                        });
                    let hist_model = Rc::new(sixtyfps::VecModel::from(history));
                    main.set_history(sixtyfps::ModelHandle::new(hist_model));
                    main.set_ticks_played(clocked as i32);
                });
            }
        });
        main.run();
    }

    Ok(())
}
