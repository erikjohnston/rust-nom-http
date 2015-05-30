
#[derive(Debug)]
pub enum IntegerDecodeError { TooLong, InvalidChar }

use std::usize;

pub fn hex_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    if buf.len() * 4 >= usize::BITS {
        // Won't fit into a u64
        println!("Too long");
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
            d @ _ => {
                println!("invalid char: {}", d as char);
                return Err(IntegerDecodeError::InvalidChar)
            },
        }
    }
    Ok(size)
}

pub fn dec_buf_to_int(buf: &[u8]) -> Result<usize, IntegerDecodeError> {
    // 2^N > 10^X => N > X log2 (10) > 3.32 X > 3 X
    if buf.len() * 4 >= usize::BITS {
        // Won't fit into a u64
        println!("Too long");
        return Err(IntegerDecodeError::TooLong);
    }

    let mut size : usize = 0;
    for c in buf {
        size *= 10;
        match *c {
            d @ b'0' ... b'9' => {
                size += (d - b'0') as usize;
            },
            d @ _ => {
                println!("invalid char: {}", d as char);
                return Err(IntegerDecodeError::InvalidChar)
            },
        }
    }
    Ok(size)
}
