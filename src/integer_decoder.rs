use errors::IntegerDecodeError;


pub fn hex_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    if buf.len() >= 8 {  // TODO: Replace with usize::BITS
        return Err(IntegerDecodeError::TooLong(buf.len()));
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
            d @ _ => {
                return Err(IntegerDecodeError::InvalidChar(d))
            },
        }
    }
    Ok(size)
}

pub fn dec_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    // 2^N > 10^X => N > X log2 (10) > 3.32 X > 3 X
    if buf.len() >= 8 {
        return Err(IntegerDecodeError::TooLong(buf.len()));
    }

    let mut size : usize = 0;
    for c in buf {
        size *= 10;
        match *c {
            d @ b'0' ... b'9' => {
                size += (d - b'0') as usize;
            },
            d @ _ => {
                return Err(IntegerDecodeError::InvalidChar(d))
            },
        }
    }
    Ok(size)
}

#[test]
fn test_hex() {
    assert_eq!(245, hex_buf_to_int(b"F5").unwrap());
    assert_eq!(245, hex_buf_to_int(b"f5").unwrap());
    assert_eq!(9, hex_buf_to_int(b"9").unwrap());
}

#[test]
fn test_dec() {
    assert_eq!(245, dec_buf_to_int(b"245").unwrap());
    assert_eq!(9, dec_buf_to_int(b"9").unwrap());
}
