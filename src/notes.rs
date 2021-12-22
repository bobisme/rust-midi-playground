use eyre::bail;
use nom::{character::complete::one_of, combinator::opt, sequence::tuple};

pub struct Note {
    inner: u8,
}

impl std::ops::Deref for Note {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

fn note_from_str(x: &'static str) -> eyre::Result<u8> {
    let res: nom::IResult<_, _> = tuple((
        one_of("abcdefgABCDEFG"),
        opt(one_of("#b^vsfSF")),
        opt(one_of("+-")),
        one_of("01234"),
    ))(x);
    let (_, (note, sf, sign, oct)) = res?;
    let out = 24;
    let out: u8 = match note.to_ascii_uppercase() {
        'C' => out,
        'D' => out + 2,
        'E' => out + 4,
        'F' => out + 5,
        'G' => out + 7,
        'A' => out + 9,
        'B' => out + 11,
        _ => bail!("invalid note: {}", note),
    };
    let out = match sf {
        Some('#' | '^' | 's' | 'S') => out + 1,
        Some('b' | 'v' | 'f' | 'F') => out - 1,
        None => out,
        _ => bail!("invalid sharp or flat symbol: {}", sf.unwrap()),
    };
    let octave: i8 = match sign {
        Some('-') => -oct.to_string().parse()?,
        Some('+') | None => oct.to_string().parse()?,
        _ => bail!("invalid sign: {}", sign.unwrap()),
    };
    let out: u8 = (out as i8 + (12 * octave)).try_into()?;
    Ok(out)
}

impl Note {
    fn from<T: AsRef<str>>(code: T) -> Self {
        // let (note, modif, sign, )
        Note { inner: 0 }
    }
}

#[cfg(test)]
mod test_notes {
    use super::*;
    use hamcrest2::prelude::*;

        #[derive(Clone, Default, Debug)]
    struct Env {
        // ...
    }

    #[test]
    fn test_note_from_str() {
        rspec::run(&rspec::describe("note_from_str", Env::default(), |ctx| {

        }))
        let note = note_from_str("c3").unwrap();
        assert_eq!(note, 61u8);
    }
}
