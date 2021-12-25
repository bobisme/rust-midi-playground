use std::{cell::RefCell, convert::Infallible};

use eyre::{ensure, Result};
use midly::{num::u7, MetaMessage, MidiMessage, Smf, TrackEventKind};
use num::Zero;

use crate::notes::{Beats, Note};

pub struct Midi<'a> {
    smf: Smf<'a>,
    notes: Vec<Note>,
}

impl<'a> Midi<'a> {
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }
}

pub fn parse(data: &[u8]) -> Result<Midi> {
    let smf = Smf::parse(data)?;
    ensure!(!smf.tracks.len().is_zero(), "no tracks found!");
    for (i, track) in smf.tracks.iter().enumerate() {
        println!("track: {}, events: {}", i, track.len());
    }
    let track = smf.tracks.get(1).unwrap();
    let mut notes = Vec::<Note>::new();
    let mut tempo = RefCell::new(25.0);
    let ticks_per_beat = || {
        let out = match smf.header.timing {
            midly::Timing::Metrical(x) => x.as_int() as f32,
            midly::Timing::Timecode(f, x) => f.as_f32() * x as f32 / (*tempo.borrow() / 60.0),
        } as f64;
        out
    };
    // let mut ticks_per_beat = match smf.header.timing {
    //     midly::Timing::Metrical(x) => x.as_int() as f32,
    //     midly::Timing::Timecode(f, x) => f.as_f32() * x as f32 / (tempo / 60.0),
    // } as f64;
    for (i, event) in track.iter().enumerate() {
        match event.kind {
            TrackEventKind::Midi { channel, message } => {
                use MidiMessage::*;
                match message {
                    NoteOff { key, vel } => {
                        // println!("got note off: key={} vel={}", key, vel);
                    }
                    NoteOn { key, vel } if vel == 0 => {}
                    NoteOn { key, vel } => {
                        // println!("got note on: key={} vel={}", key, vel);
                        let off = track.iter().skip(i + 1).find(|&e| match e.kind {
                            TrackEventKind::Midi {
                                channel: ch,
                                message: NoteOn { key: k, vel: v },
                            } => ch == channel && k == key && v == 0,
                            TrackEventKind::Midi {
                                channel: ch,
                                message: NoteOff { key: k, vel: _ },
                            } => ch == channel && k == key,
                            _ => false,
                        });
                        if let Some(ev) = off {
                            notes.push(Note::from(key.as_int()).with_vel(vel.as_int()).with_beats(
                                Beats::from(ev.delta.as_int() as f64 / ticks_per_beat()),
                            ));
                        }
                    }
                    Aftertouch { key, vel } => {}
                    Controller { controller, value } => {}
                    ProgramChange { program } => {}
                    ChannelAftertouch { vel } => {}
                    PitchBend { bend } => {}
                }
            }
            TrackEventKind::SysEx(_) => {}
            TrackEventKind::Escape(_) => {}
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                tempo.replace(t.as_int() as f32);
                // ticks_per_beat = match smf.header.timing {
                //     midly::Timing::Metrical(x) => x.as_int() as f32,
                //     midly::Timing::Timecode(f, x) => f.as_f32() * x as f32 / (tempo / 60.0),
                // } as f64;
            }
            TrackEventKind::Meta(_) => {}
        }
    }
    Ok(Midi { smf, notes })
}
