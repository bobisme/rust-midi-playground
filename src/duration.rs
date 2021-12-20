use std::num::TryFromIntError;

use midly::num::u28;

#[derive(Default)]
pub(crate) struct Dur {
    pub(crate) ticks: u28,
}

impl Dur {
    pub(crate) fn new<T>(ticks: T) -> Self
    where
        T: TryInto<u32, Error = std::num::TryFromIntError>,
    {
        let t = ticks.try_into().unwrap();
        Self { ticks: t.into() }
    }

    pub fn as_int(self) -> u32 {
        self.ticks.as_int()
    }
}

impl std::ops::Deref for Dur {
    type Target = u28;

    fn deref(&self) -> &Self::Target {
        &self.ticks
    }
}

impl std::ops::Mul for Dur {
    type Output = Dur;

    fn mul(self, rhs: Self) -> Self::Output {
        Dur {
            ticks: u28::new(self.as_int() * rhs.as_int()),
        }
    }
}

impl<T> From<T> for Dur
where
    T: Into<u32>,
{
    fn from(x: T) -> Self {
        Dur {
            ticks: x.into().into(),
        }
    }
}

impl From<Dur> for u28 {
    fn from(x: Dur) -> Self {
        x.ticks
    }
}
