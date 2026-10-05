use crate::model::Result;
use std::io::{self, BufRead, Cursor, Read};

#[derive(Clone, Copy)]
pub(super) enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn declarations(self) -> &'static [&'static [u8]] {
        match self {
            Self::Utf8 => &[b"utf-8"],
            Self::Utf16Le => &[b"utf-16", b"utf-16le"],
            Self::Utf16Be => &[b"utf-16", b"utf-16be"],
        }
    }
}

pub(super) struct Input<R> {
    inner: R,
    pub encoding: Encoding,
    decoded: Cursor<Vec<u8>>,
    pending: Option<u16>,
}

impl<R: BufRead> Input<R> {
    pub fn new(mut inner: R) -> Result<Self> {
        let mut prefix = [0; 2];
        inner.read_exact(&mut prefix)?;
        let encoding = match prefix {
            [0xff, 0xfe] => Encoding::Utf16Le,
            [0xfe, 0xff] => Encoding::Utf16Be,
            _ => Encoding::Utf8,
        };
        let decoded = if prefix == [0xef, 0xbb] {
            let mut last = [0];
            inner.read_exact(&mut last)?;
            if last[0] == 0xbf {
                Vec::new()
            } else {
                vec![prefix[0], prefix[1], last[0]]
            }
        } else if matches!(encoding, Encoding::Utf8) {
            prefix.to_vec()
        } else {
            Vec::new()
        };
        Ok(Self {
            inner,
            encoding,
            decoded: Cursor::new(decoded),
            pending: None,
        })
    }
}

impl<R: BufRead> Read for Input<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() {
            return Ok(0);
        }
        loop {
            let count = self.decoded.read(output)?;
            if count > 0 {
                return Ok(count);
            }
            if matches!(self.encoding, Encoding::Utf8) {
                return self.inner.read(output);
            }
            let mut bytes = [0; 8192];
            let mut count = self.inner.read(&mut bytes)?;
            if count == 0 {
                return if self.pending.is_some() {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UTF-16 termina con un sustituto sin pareja",
                    ))
                } else {
                    Ok(0)
                };
            }
            if count % 2 != 0 {
                self.inner
                    .read_exact(&mut bytes[count..count + 1])
                    .map_err(|error| {
                        if error.kind() == io::ErrorKind::UnexpectedEof {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                "UTF-16 termina con un byte incompleto",
                            )
                        } else {
                            error
                        }
                    })?;
                count += 1;
            }
            let mut words = Vec::with_capacity(count / 2 + 1);
            words.extend(self.pending.take());
            for pair in bytes[..count].as_chunks::<2>().0 {
                let pair = [pair[0], pair[1]];
                words.push(match self.encoding {
                    Encoding::Utf16Le => u16::from_le_bytes(pair),
                    Encoding::Utf16Be => u16::from_be_bytes(pair),
                    Encoding::Utf8 => unreachable!(),
                });
            }
            if words
                .last()
                .is_some_and(|word| (0xd800..=0xdbff).contains(word))
            {
                self.pending = words.pop();
            }
            let text = String::from_utf16(&words).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "secuencia UTF-16 inválida")
            })?;
            self.decoded = Cursor::new(text.into_bytes());
        }
    }
}
