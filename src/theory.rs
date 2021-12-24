use std::slice::Iter;

#[derive(Clone, Copy, Debug)]
pub enum Sign {
    DoubleFlat,
    Flat,
    Natural,
    Sharp,
    DoubleSharp,
}

impl Sign {
    fn rel(&self) -> i8 {
        match self {
            Self::DoubleFlat => -2,
            Self::Flat => -1,
            Self::Natural => 0,
            Self::Sharp => 1,
            Self::DoubleSharp => 2,
        }
    }
}

impl From<Option<Sign>> for Sign {
    fn from(s: Option<Sign>) -> Self {
        s.unwrap_or(Self::Natural)
    }
}

pub struct RelativeSemitone {
    inner: u8,
}

impl std::ops::Deref for RelativeSemitone {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub enum Degree {
    Num(u8),
    I,
    II,
    III,
    IV,
    V,
    VI,
    VII,
    NumWithSign(u8, Sign),
}

impl std::ops::Add<Sign> for Degree {
    type Output = Self;

    fn add(self, rhs: Sign) -> Self::Output {
        Self::NumWithSign(self.num(), rhs)
    }
}

trait DegreeNum {
    fn num(&self) -> u8;
}

impl Degree {
    fn sign(&self) -> Option<Sign> {
        match self {
            &Self::NumWithSign(_, s) => Some(s),
            _ => None,
        }
    }
}

impl DegreeNum for Degree {
    fn num(&self) -> u8 {
        match self {
            &Self::Num(x) => x,
            &Self::NumWithSign(x, _) => x,
            &Self::I => 1,
            &Self::II => 2,
            &Self::III => 3,
            &Self::IV => 4,
            &Self::V => 5,
            &Self::VI => 6,
            &Self::VII => 7,
        }
    }
}

pub enum Key {
    Midi(u8),
    A(Option<Sign>),
    B(Option<Sign>),
    C(Option<Sign>),
    D(Option<Sign>),
    E(Option<Sign>),
    F(Option<Sign>),
    G(Option<Sign>),
}

pub enum RelativeSemitones {
    Seven([u8; 7]),
}

impl RelativeSemitones {
    pub fn iter(&self) -> Iter<u8> {
        match self {
            RelativeSemitones::Seven(x) => x.iter(),
        }
    }
}

pub enum Mode {
    Ionian,
    Natural,
}

impl Mode {
    pub fn semitones(&self) -> RelativeSemitones {
        use RelativeSemitones::*;
        match self {
            Self::Ionian => Seven([0u8, 2, 4, 5, 7, 9, 11]),
            Self::Natural => Seven([0u8, 2, 3, 5, 7, 8, 10]),
        }
    }
}

pub struct Scale {
    key: Key,
    mode: Mode,
}

impl Scale {
    pub fn new(key: Key, mode: Mode) -> Self {
        Self { key, mode }
    }

    pub fn semitones(&self) -> [u8; 16] {
        let mut out = [u8::MAX; 16];
        let base = match self.key {
            Key::Midi(m) => m as i8,
            Key::A(s) => 8 + Sign::from(s).rel(),
            Key::B(s) => 10,
            Key::C(s) => 0,
            Key::D(s) => 2,
            Key::E(s) => 4,
            Key::F(s) => 5,
            Key::G(s) => 7,
        };
        self.mode
            .semitones()
            .iter()
            .enumerate()
            .for_each(|(i, tone)| {
                out[i] = (12 + base) as u8 + tone;
            });
        out
    }
}
