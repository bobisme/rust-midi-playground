use std::{
    io::{stdin, stdout, Write},
    thread::sleep,
    time::Duration,
};

use eyre::{bail, eyre};
use midir::{MidiOutput, MidiOutputPort};
use midly::{num::u28, TrackEvent};

mod dsl;
mod duration;
mod notes;

use duration::Dur;

const TICKS_PER_BEAT: u16 = 100;
const ZERO_TICKS: u28 = u28::new(0);
const BEAT: u28 = u28::new(TICKS_PER_BEAT as u32);

fn play_midi() -> eyre::Result<()> {
    let midi_out = MidiOutput::new("My Test Output")?;

    // Get an output port (read from console if multiple are available)
    let out_ports = midi_out.ports();
    let out_port: &MidiOutputPort = match out_ports.len() {
        0 => return Err(eyre!("no output port found")),
        1 => {
            println!(
                "Choosing the only available output port: {}",
                midi_out.port_name(&out_ports[0]).unwrap()
            );
            &out_ports[0]
        }
        _ => {
            println!("\nAvailable output ports:");
            for (i, p) in out_ports.iter().enumerate() {
                println!("{}: {}", i, midi_out.port_name(p).unwrap());
            }
            let port = out_ports.iter().find(|p| {
                midi_out
                    .port_name(p)
                    .map(|name| name == "loopMIDI Port")
                    .unwrap()
            });
            if port.is_none() {
                bail!("could not grab a port");
            }
            port.unwrap()
            // print!("Please select output port: ");
            // stdout().flush()?;
            // let mut input = String::new();
            // stdin().read_line(&mut input)?;
            // out_ports
            //     .get(input.trim().parse::<usize>()?)
            //     .ok_or(eyre!("invalid output port selected"))?
        }
    };

    println!("\nOpening connection");
    let mut conn_out = midi_out.connect(out_port, "midir-test")?;
    println!("Connection open. Listen!");
    {
        // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
        let mut play_note = |note: u8, duration: u64| {
            const NOTE_ON_MSG: u8 = 0x90;
            const NOTE_OFF_MSG: u8 = 0x80;
            const VELOCITY: u8 = 0x64;
            // We're ignoring errors in here
            let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
            sleep(Duration::from_millis(duration * 150));
            let _ = conn_out.send(&[NOTE_OFF_MSG, note, VELOCITY]);
        };

        sleep(Duration::from_millis(4 * 150));

        play_note(66, 4);
        play_note(65, 3);
        play_note(63, 1);
        play_note(61, 6);
        play_note(59, 2);
        play_note(58, 4);
        play_note(56, 4);
        play_note(54, 4);
    }
    sleep(Duration::from_millis(150));
    println!("\nClosing connection");
    // This is optional, the connection would automatically be closed as soon as it goes out of scope
    conn_out.close();
    println!("Connection closed");
    Ok(())
}

fn main() {
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
    play_midi();
}
