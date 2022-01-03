use std::{
    fs,
    sync::{
        atomic::{AtomicBool, Ordering},
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

    // Number of events to use per chunk
    #[clap(long, default_value_t = 1<<16)]
    chunk_size: usize,

    // The humanization delay range (±ms/2)
    #[clap(long)]
    human_ms: Option<u8>,
    // The humanization velocity range (±vel/2)
    #[clap(long)]
    human_vel: Option<u8>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("generating with order {} chain", args.order);

    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;

    // let data = fs::read("1st Mvmt Sonata No.14, Opus 27, No.2.mid")?;
    let data = fs::read(args.path)?;
    let midi_parser = midi::Parser::default().with_ticks_per_beat(24);
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
    let quieter = iter.clone().map(|e| {
        if let Event::PlayNote { key, dynamic } = e {
            Event::play(key, dynamic.down())
        } else {
            e
        }
    });
    let iter = iter.chain(quieter);
    let chunk_size: usize = args.chunk_size;
    for chunk in &iter.chunks(chunk_size) {
        let tokens = chunk.collect::<Vec<_>>();
        seq_chain.feed(tokens);
    }
    for ev in seq_chain.iter().flatten() {
        if term.load(Ordering::Relaxed) {
            break;
        }
        player.event(&ev)?;
    }

    Ok(())
}
