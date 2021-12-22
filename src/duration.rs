use midly::num::u28;

const TICKS_PER_BEAT: u16 = 100;

fn beats_to_ticks(x: u16) -> u32 {
    x as u32 * TICKS_PER_BEAT as u32
}

pub(crate) enum Dur {
    Ticks(u32),
    Beats(u16),
}

impl Dur {
    pub(crate) fn ticks<T>(x: T) -> Self
    where
        T: TryInto<u32, Error = std::num::TryFromIntError>,
    {
        Self::Ticks(x.try_into().unwrap())
    }

    pub(crate) fn beats(x: u16) -> Self {
        Self::Beats(x * TICKS_PER_BEAT)
    }

    pub fn as_int(&self) -> u32 {
        match *self {
            Dur::Ticks(x) => x,
            Dur::Beats(x) => x as u32,
        }
    }
}

impl std::ops::Mul for Dur {
    type Output = Dur;

    fn mul(self, rhs: Self) -> Self::Output {
        use Dur::*;
        match (self, rhs) {
            (Ticks(l), Ticks(r)) => Ticks(l * r),
            (Ticks(l), Beats(r)) => Ticks(l * beats_to_ticks(r)),
            (Beats(l), Ticks(r)) => Ticks(beats_to_ticks(l) * r),
            (Beats(l), Beats(r)) => Beats(l * r),
        }
    }
}

impl<T> From<T> for Dur
where
    T: Into<u32>,
{
    fn from(x: T) -> Self {
        Dur::Ticks(x.into())
    }
}

impl From<Dur> for u28 {
    fn from(x: Dur) -> Self {
        match x {
            Dur::Ticks(x) => x.into(),
            Dur::Beats(x) => (x as u32 * TICKS_PER_BEAT as u32).into(),
        }
    }
}
