use std::{thread::sleep, time::Duration};

use eyre::{ensure, eyre, Result};
use midir::{MidiOutput, MidiOutputConnection};

use crate::notes::Note;

const NOTE_ON_MSG: u8 = 0x90;
const NOTE_OFF_MSG: u8 = 0x80;

pub struct Player<'a> {
    client_name: &'a str,
    tempo: f32,
    conn_out: Option<MidiOutputConnection>,
}

impl<'a> Player<'a> {
    pub fn new(client_name: &'a str) -> Result<Self> {
        Ok(Player {
            client_name,
            tempo: 120.0,
            conn_out: None,
        })
    }

    pub fn tempo(&self) -> f32 {
        self.tempo
    }

    pub fn set_tempo(&mut self, tempo: impl Into<f32>) {
        self.tempo = tempo.into();
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
