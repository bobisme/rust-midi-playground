use std::{thread::sleep, time::Duration};

use eyre::{ensure, eyre, Result};
use midir::{MidiOutput, MidiOutputConnection};
use num::Zero;

use crate::{notes::Note, sequence::Event};

const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;

pub struct Player<'a> {
    client_name: &'a str,
    tempo: f32,
    ticks_per_beat: u32,
    tick_dur: Duration,
    conn_out: Option<MidiOutputConnection>,

    ticks_played: u32,
    notes_on: [Option<u32>; 128],
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

    fn play_key(&mut self, key: u8, vel: u8) {
        let conn = self.conn_out.as_mut().unwrap();
        self.notes_on[key as usize] = Some(self.ticks_played);
        conn.send(&[NOTE_ON_MSG, key, vel]).unwrap();
    }

    fn stop_key(&mut self, key: u8) {
        self.notes_on[key as usize] = None;
        let conn = self.conn_out.as_mut().unwrap();
        conn.send(&[NOTE_OFF_MSG, key, 0]).unwrap();
    }

    pub fn event(&mut self, event: &Event) -> Result<()> {
        ensure!(self.conn_out.is_some(), "not connected to out port");
        match event {
            Event::PlayNote { key, dynamic } => {
                println!("playing {} @ {:?}", key, dynamic);
                self.play_key(key.as_int(), dynamic.vel());
            }
            Event::StopNote { key } => {
                println!("stopping {}", key);
                self.stop_key(key.as_int());
            }
            Event::Wait { ticks } => {
                let dur = self.tick_dur * *ticks;
                println!("waiting {} ticks ({:?})", ticks, dur);
                sleep(dur);
                self.ticks_played += *ticks;
            }
        }
        let max_len = self.ticks_per_beat * 8;
        let threshold = self.ticks_played.checked_sub(max_len).unwrap_or(0);
        let ringing_notes = self
            .notes_on
            .iter()
            .enumerate()
            .filter(|&(key, start)| start.is_some())
            .map(|(k, &s)| (k, s.unwrap()))
            .filter(|&(k, s)| s < threshold)
            .map(|(k, s)| k as u8)
            .collect::<Vec<_>>();
        for key in ringing_notes {
            self.stop_key(key);
        }
        Ok(())
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
