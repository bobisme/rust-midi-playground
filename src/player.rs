use std::{sync::atomic::AtomicU32, thread::sleep, time::Duration};

use eyre::{ensure, eyre, Result};
use midir::{MidiOutput, MidiOutputConnection};

use crate::{
    notes::Note,
    sequence::{Dynamic, Event},
};

const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;
const HUMAN_MS_RANGE: f64 = 30.0;
const HUMAN_VEL_RANGE: f64 = 12.0;
/// Max number of beats the player will let a note ring for.
const MAX_NOTE_BEATS: u32 = 4;

pub struct Player<'a> {
    client_name: &'a str,
    tempo: f32,
    ticks_per_beat: u32,
    tick_dur: Duration,
    conn_out: Option<MidiOutputConnection>,

    ticks_played: u32,
    // all keys including the tick # after which they are expected to stop
    notes_on: [Option<u32>; 128],
    // timeshift is to correct a prior humanization delay
    timeshift: f64,
    human_ms_range: f64,
    human_vel_range: f64,
}

fn add_ms(dur: Duration, ms: f64) -> Duration {
    match ms {
        x if x >= 0.0 => dur + Duration::from_millis(ms as u64),
        _ => dur.saturating_sub(Duration::from_millis(-ms as u64)),
    }
}

impl<'a> Player<'a> {
    pub fn new(client_name: &'a str) -> Self {
        let tpb = 12;
        Player {
            client_name,
            tempo: 120.0,
            tick_dur: Duration::from_secs_f32(60.0 / 120.0 / tpb as f32),
            ticks_per_beat: tpb,
            conn_out: None,
            ticks_played: 0,
            notes_on: [None; 128],
            timeshift: 0.0,
            human_ms_range: HUMAN_MS_RANGE,
            human_vel_range: HUMAN_VEL_RANGE,
        }
    }

    /// Set the player's ticks per beat.
    pub fn set_ticks_per_beat(&mut self, ticks_per_beat: impl Into<u32>) {
        self.ticks_per_beat = ticks_per_beat.into();
    }

    pub fn tempo(&self) -> f32 {
        self.tempo
    }

    pub fn set_tempo(&mut self, tempo: impl Into<f32>) {
        self.tempo = tempo.into();
        self.tick_dur = Duration::from_secs_f32(60.0 / self.tempo / self.ticks_per_beat as f32);
    }

    pub fn connect(&mut self, port_name: &str) -> Result<&Self> {
        let midi_out = MidiOutput::new(self.client_name)?;
        let out_ports = midi_out.ports();
        let port = out_ports
            .iter()
            .find(|p| {
                midi_out
                    .port_name(p)
                    .map(|name| name == port_name)
                    .unwrap_or(false)
            })
            .ok_or_else(|| eyre!("could not find port"))?;
        self.conn_out = Some(midi_out.connect(port, "midir-test")?);
        Ok(self)
    }

    pub fn play(&mut self, note: &Note) -> Result<()> {
        ensure!(self.conn_out.is_some(), "not connected to out port");
        let conn = self.conn_out.as_mut().unwrap();
        let _ = conn.send(&[NOTE_ON_MSG, note.note(), note.vel()]);
        sleep(note.beats().from_bpm(self.tempo));
        let _ = conn.send(&[NOTE_OFF_MSG, note.note(), note.vel()]);
        Ok(())
    }

    fn play_key(&mut self, key: u8, dynamic: Dynamic, max_ticks: u32) {
        let conn = self.conn_out.as_mut().unwrap();
        let off_tick = self.ticks_played + max_ticks;
        self.notes_on[key as usize] = Some(off_tick);
        let vel_range = self.human_vel_range;
        let human = (rand::random::<f64>() * vel_range - vel_range).round() as i8;
        let vel = match human {
            x if x >= 0 => dynamic.vel().saturating_add(human as u8),
            _ => dynamic.vel().saturating_sub(-human as u8),
        };
        // println!(
        //     "playing {} @ {:?} ({}) for at least {} ticks",
        //     key, dynamic, vel, max_ticks
        // );
        conn.send(&[NOTE_ON_MSG, key, vel]).unwrap();
    }

    fn stop_key(&mut self, key: u8) {
        self.notes_on[key as usize] = None;
        let conn = self.conn_out.as_mut().unwrap();
        conn.send(&[NOTE_OFF_MSG, key, 0]).unwrap();
    }

    fn wait(&mut self, ticks: u32) {
        let dur = self.tick_dur * ticks;
        let dur = add_ms(dur, self.timeshift);
        let ms_range = self.human_ms_range;
        let shift_ms = (rand::random::<f64>() * ms_range - ms_range).round();
        self.timeshift = -shift_ms;
        let dur = add_ms(dur, shift_ms);
        // println!("waiting {} ticks ({:?})", ticks, dur);
        sleep(dur);
        self.ticks_played += ticks;
    }

    pub fn event(&mut self, event: &Event) -> Result<()> {
        ensure!(self.conn_out.is_some(), "not connected to out port");
        match event {
            &Event::PlayNote { key, dynamic } => {
                self.play_key(key.as_int(), dynamic, self.ticks_per_beat * MAX_NOTE_BEATS);
            }
            &Event::PlayNoteTicks {
                key,
                dynamic,
                ticks,
            } => {
                self.play_key(key.as_int(), dynamic, ticks);
            }
            Event::StopNote { key } => {
                println!("stopping {}", key);
                self.stop_key(key.as_int());
            }
            Event::Wait { ticks } => {
                self.wait(*ticks);
            }
        }
        self.notes_on
            .iter()
            .enumerate()
            .filter(|&(_, end)| end.is_some())
            .filter(|&(_, &end)| end.unwrap() < self.ticks_played)
            .map(|(key, _)| key as u8)
            .collect::<Vec<u8>>()
            .iter()
            .for_each(|&key| self.stop_key(key));
        Ok(())
    }

    /// Set the player's human ms range.
    pub fn set_human_ms_range(&mut self, human_ms_range: f64) {
        self.human_ms_range = human_ms_range;
    }

    /// Set the player's human vel range.
    pub fn set_human_vel_range(&mut self, human_vel_range: f64) {
        self.human_vel_range = human_vel_range;
    }

    /// Get a reference to the player's ticks played.
    pub fn ticks_played(&self) -> u32 {
        self.ticks_played
    }

    /// Get a reference to the player's tick duration.
    pub fn tick_dur(&self) -> Duration {
        self.tick_dur
    }
}

impl<'a> Drop for Player<'a> {
    fn drop(&mut self) {
        if self.conn_out.is_none() {
            return;
        }

        let conn = self.conn_out.take().unwrap();
        conn.close();
    }
}
