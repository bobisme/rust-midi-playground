use std::{
    cell::{Ref, RefCell},
    convert::Infallible,
};

use eyre::{bail, ensure, eyre, Result};
use midly::{num::u7, MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use num::{bigint::ParseBigIntError, Zero};

use crate::{
    notes::{Beats, Note},
    sequence::Event,
};

pub struct Midi<'a> {
    smf: Smf<'a>,
    notes: Vec<Note>,
}

impl<'a> Midi<'a> {
    pub fn notes(&self) -> &Vec<Note> {
        &self.notes
    }
}

fn calc_ticks_per_beat(timing: &Timing, tempo: f32) -> f64 {
    use midly::Timing::*;
    let out = match *timing {
        Metrical(x) => x.as_int() as f32,
        Timecode(f, x) => f.as_f32() * x as f32 / (tempo / 60.0),
    };
    out as f64
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

pub struct MidiSequence {
    pub events: Vec<Event>,
    ticks_per_beat: u32,
}

impl MidiSequence {
    /// Get a reference to the midi sequence's ticks per beat.
    pub fn ticks_per_beat(&self) -> u32 {
        self.ticks_per_beat
    }
}

impl IntoIterator for MidiSequence {
    type Item = Event;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

pub struct Parser {
    tempo: RefCell<f32>,
    ticks_per_beat: u32,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            tempo: RefCell::new(120.0),
            ticks_per_beat: 12,
        }
    }
}

impl Parser {
    pub fn with_ticks_per_beat(&self, t: u32) -> Self {
        Self {
            ticks_per_beat: t.into(),
            tempo: RefCell::new(*self.tempo.borrow()),
        }
    }

    pub fn parse_seq(&self, data: &[u8], track_i: usize) -> Result<MidiSequence> {
        let smf = Smf::parse(data)?;
        let track = smf
            .tracks
            .get(track_i)
            .ok_or_else(|| eyre!("could not get track {}", track_i))?;

        let mut tempo = RefCell::new(50.0);

        let things = track
            .iter()
            .map(|ev| match ev.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                    tempo.replace(t.as_int() as f32);
                    (ev.delta, None)
                }
                TrackEventKind::Midi { channel, message } => {
                    use MidiMessage::*;
                    match message {
                        NoteOff { key, vel } => (ev.delta, Some(Event::stop(key))),
                        NoteOn { key, vel } if vel == 0 => (ev.delta, Some(Event::stop(key))),
                        NoteOn { key, vel } => (ev.delta, Some(Event::play(key, vel))),
                        _ => (ev.delta, None),
                    }
                }
                _ => (ev.delta, None),
            })
            // .inspect(|(d, e)| println!("d={:?}, e={:?}", d, e))
            .scan(0u32, |state, (delta, event)| {
                *state = *state + delta.as_int();
                match event {
                    None => Some([None, None]),
                    Some(ev) => {
                        let d = *state as f64;
                        let tpb = calc_ticks_per_beat(&smf.header.timing, *tempo.borrow());
                        let beats = d / tpb;
                        let ticks = (beats * self.ticks_per_beat as f64).round() as u32;
                        *state = 0;
                        Some([Some(Event::wait(ticks)), event])
                    }
                }
            })
            .flatten()
            .filter_map(|e| match e {
                Some(Event::Wait { ticks }) if ticks == 0 => None,
                _ => e,
            });

        Ok(MidiSequence {
            events: things.collect(),
            ticks_per_beat: self.ticks_per_beat,
        })
    }
}
