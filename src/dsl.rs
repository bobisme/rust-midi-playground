use midly::{MidiMessage, TrackEvent, TrackEventKind::Midi};

use crate::duration::Dur;

// pub(crate) fn note_on<'a>(delta: Dur, chan: u8, key: u8, vel: u8) -> TrackEvent<'a> {
//     TrackEvent {
//         delta: delta.into(),
//         kind: Midi {
//             channel: chan.into(),
//             message: MidiMessage::NoteOn {
//                 key: key.into(),
//                 vel: vel.into(),
//             },
//         },
//     }
// }

// pub(crate) fn note_off<'a>(delta: Dur, chan: u8, key: u8, vel: u8) -> TrackEvent<'a> {
//     TrackEvent {
//         delta: delta.into(),
//         kind: Midi {
//             channel: chan.into(),
//             message: MidiMessage::NoteOff {
//                 key: key.into(),
//                 vel: vel.into(),
//             },
//         },
//     }
// }
