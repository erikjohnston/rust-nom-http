
use std::fmt;
use std::error;

#[derive(Debug)]
pub enum IntegerDecodeError { TooLong, InvalidChar }

pub fn hex_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    if buf.len() >= 8 {  // TODO: Replace with usize::BITS
        return Err(IntegerDecodeError::TooLong);
    }

    let mut size : usize = 0;
    for c in buf {
        size *= 16;
        match *c {
            d @ b'0' ... b'9' => {
                size += (d - b'0') as usize;
            },
            d @ b'A' ... b'F' => {
                size += (d - b'A' + 10) as usize;
            },
            d @ b'a' ... b'f' => {
                size += (d - b'a' + 10) as usize;
            },
            _ => {
                return Err(IntegerDecodeError::InvalidChar)
            },
        }
    }
    Ok(size)
}

pub fn dec_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    // 2^N > 10^X => N > X log2 (10) > 3.32 X > 3 X
    if buf.len() >= 8 {
        return Err(IntegerDecodeError::TooLong);
    }

    let mut size : usize = 0;
    for c in buf {
        size *= 10;
        match *c {
            d @ b'0' ... b'9' => {
                size += (d - b'0') as usize;
            },
            _ => {
                return Err(IntegerDecodeError::InvalidChar)
            },
        }
    }
    Ok(size)
}

impl error::Error for IntegerDecodeError {
    fn description(&self) -> &str {
        match *self {
            IntegerDecodeError::TooLong => "The supplied buffer was too long.",
            IntegerDecodeError::InvalidChar => "Buffer included invalid character.",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl fmt::Display for IntegerDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IntegerDecodeError::TooLong => write!(
                f, "Could not parse int: The supplied buffer was too long."
            ),
            IntegerDecodeError::InvalidChar => write!(
                f, "Could not parse int: Buffer included invalid character."
            ),
        }
    }
}
