use midly::{num::u28, TrackEvent};

mod duration;

const TICKS_PER_BEAT: u16 = 100;
const ZERO_TICKS: u28 = u28::new(0);
const BEAT: u28 = u28::new(TICKS_PER_BEAT as u32);

fn main() {
    let one_beat = duration::Dur::new(100);
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
            delta: one_beat.into(),
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
    println!("Hello, world!");
}
