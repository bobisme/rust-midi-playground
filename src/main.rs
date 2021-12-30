use std::{
    fs,
    io::{stdin, stdout, Write},
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::Duration,
};

use eyre::{bail, ensure, eyre, Result};
use midir::{MidiOutput, MidiOutputPort};
use midly::{num::u28, TrackEvent};

mod dsl;
mod duration;
mod midi;
mod notes;
pub mod player;
mod theory;
mod sequence;

use duration::Dur;
use notes::{Beats, Note};

const TICKS_PER_BEAT: u16 = 100;
const ZERO_TICKS: u28 = u28::new(0);
const BEAT: u28 = u28::new(TICKS_PER_BEAT as u32);

fn make_some_chains() -> Result<(markov::Chain<Note>, markov::Chain<Beats>)> {
    let data = fs::read("Beethoven-Moonlight-Sonata.mid")?;
    let parsed_midi = midi::parse(&data)?;
    let notes = parsed_midi.notes();
    ensure!(notes.len() != 0, "no parsed notes...");
    let mut note_chain = markov::Chain::of_order(2);
    note_chain.feed(notes);
    let mut beats_chain = markov::Chain::of_order(2);
    beats_chain.feed(notes.iter().map(|n| n.beats()).collect::<Vec<Beats>>());
    // note_chain
    //     .feed(vec![
    //         Note::try_from("c1")?,
    //         Note::try_from("c1")?,
    //         Note::try_from("d1")?,
    //         Note::try_from("c1")?,
    //         Note::try_from("f1")?,
    //         Note::try_from("e1")?,
    //     ])
    //     .feed([
    //         Note::try_from("c1")?,
    //         Note::try_from("c1")?,
    //         Note::try_from("d1")?,
    //         Note::try_from("c1")?,
    //         Note::try_from("a2")?,
    //         Note::try_from("f1")?,
    //     ])
    //     .feed([
    //         Note::try_from("c1")?,
    //         Note::try_from("c1")?,
    //         Note::try_from("c2")?,
    //         Note::try_from("a1")?,
    //         Note::try_from("f1")?,
    //         Note::try_from("e1")?,
    //         Note::try_from("d1")?,
    //     ])
    //     .feed([
    //         Note::try_from("c2")?,
    //         Note::try_from("c2")?,
    //         Note::try_from("a1")?,
    //         Note::try_from("f1")?,
    //         Note::try_from("g1")?,
    //         Note::try_from("f1")?,
    //     ]);
    // let mut beats_chain = markov::Chain::of_order(2);
    // beats_chain
    //     .feed(vec![
    //         2.into(),
    //         1.into(),
    //         2.into(),
    //         2.into(),
    //         2.into(),
    //         3.into(),
    //     ])
    //     .feed([2.into(), 1.into(), 2.into(), 2.into(), 2.into(), 3.into()])
    //     .feed([
    //         2.into(),
    //         1.into(),
    //         3.into(),
    //         3.into(),
    //         1.into(),
    //         2.into(),
    //         2.into(),
    //     ])
    //     .feed([2.into(), 1.into(), 3.into(), 1.into(), 2.into(), 3.into()]);
    Ok((note_chain, beats_chain))
}

fn main() -> Result<()> {
    let term = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term))?;

    println!("generating notes..");
    let (note_gen, beats_gen) = make_some_chains()?;
    let n_chain = note_gen.iter().flatten();
    let b_chain = beats_gen.iter().flatten();
    let chain = n_chain.zip(b_chain).map(|(n, b)| n.with_beats(b));
    // println!(
    //     "{}",
    //     chain
    //         .iter()
    //         .map(|x| x.to_string())
    //         .collect::<Vec<String>>()
    //         .join("; ")
    // );

    let mut player = player::Player::new("Bobs thing")?;
    player.set_tempo(50.0);
    player.connect("loopMIDI Port")?;
    let player = Arc::new(Mutex::new(player));

    {
        let player = player.clone();
        let term = term.clone();
        thread::spawn(move || {
            while !term.load(Ordering::Relaxed) {
                sleep(Duration::from_micros(50));
            }
            let mut guard = player.lock().unwrap();
            (0..127).for_each(|n| guard.play(&Note::off(n)).unwrap());
            std::process::exit(0);
        });
    }

    let data = fs::read("Beethoven-Moonlight-Sonata.mid")?;
    let parsed_midi = midi::parse(&data)?;
    let notes = parsed_midi.notes();
    ensure!(notes.len() != 0, "no parsed notes...");

    // for note in notes {
    //     if term.load(Ordering::Relaxed) {
    //         return Ok(());
    //     }
    //     println!("{}", note);
    //     let mut guard = player.lock().unwrap();
    //     guard.play(&note)?;
    // }

    println!("playing generated notes... ");
    for note in chain {
        if term.load(Ordering::Relaxed) {
            return Ok(());
        }
        println!("{}", note);
        let mut guard = player.lock().unwrap();
        guard.play(&note)?;
    }

    // let one_beat = Dur::Beats(1);
    let header = midly::Header {
        format: midly::Format::Sequential,
        timing: midly::Timing::Metrical(TICKS_PER_BEAT.into()),
    };
    let track_one: Vec<midly::TrackEvent> = vec![
        // wait 1 bar
        // dsl::note_off(Dur::Beats(4), 1, 0, 0),
        // dsl::note_on(Dur::Beats(0), 1, 60, 70),
        // dsl::note_off(Dur::Beats(1), 1, 60, 70),
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
    // play_midi();
    Ok(())
}
