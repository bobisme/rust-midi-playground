use midly::num::u7;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Dynamic {
    VerySoft,
    Soft,
    Medium,
    Loud,
    VeryLoud,
}

impl Dynamic {
    pub fn to_int(&self) -> u8 {
        match self {
            Self::VerySoft => 16,
            Self::Soft => 40,
            Self::Medium => 64,
            Self::Loud => 88,
            Self::VeryLoud => 112,
        }
    }

    pub fn vel(&self) -> u8 {
        self.to_int()
    }
}

impl From<u8> for Dynamic {
    fn from(x: u8) -> Self {
        match x {
            0..=24 => Self::VerySoft,
            25..=49 => Self::Soft,
            50..=76 => Self::Medium,
            77..=101 => Self::Loud,
            102.. => Self::VeryLoud,
        }
    }
}
impl From<u7> for Dynamic {
    fn from(x: u7) -> Self {
        Dynamic::from(x.as_int())
    }
}

impl Default for Dynamic {
    fn default() -> Self {
        Self::Medium
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    PlayNote { key: u7, dynamic: Dynamic },
    StopNote { key: u7 },
    Wait { ticks: u32 },
}

impl Event {
    pub fn p(key: impl Into<u7>) -> Self {
        Self::PlayNote {
            key: key.into(),
            dynamic: Default::default(),
        }
    }

    pub fn play(key: impl Into<u7>, dynamic: impl Into<Dynamic>) -> Self {
        Self::PlayNote {
            key: key.into(),
            dynamic: dynamic.into(),
        }
    }

    pub fn stop(key: impl Into<u7>) -> Self {
        Self::StopNote { key: key.into() }
    }

    pub fn wait(ticks: impl Into<u32>) -> Self {
        Self::Wait {
            ticks: ticks.into(),
        }
    }
}
