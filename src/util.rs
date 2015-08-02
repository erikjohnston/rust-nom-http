
pub use nom_parsers::{RequestLine, ResponseLine};
use std::collections::HashMap;
use std::str;

use errors::IntegerDecodeError;

use parser::*;


#[derive(PartialEq,Eq,Debug)]
pub struct SimpleRequestCallback<'r> {
    pub method: &'r str,
    pub path: &'r str,
    pub version: (u8, u8),
    pub headers: HashMap< &'r str,  &'r [u8]>,
    pub chunks: Vec<u8>,
    pub finished: bool,
}

impl <'r> SimpleRequestCallback<'r> {
    pub fn new() -> SimpleRequestCallback<'r> {
        SimpleRequestCallback{
            method: "",
            path: "",
            version: (0, 0),
            headers: HashMap::new(),
            chunks: Vec::new(),
            finished: false,
        }
    }
}

impl <'r> HttpRequestCallbacks<'r> for SimpleRequestCallback<'r> {
    fn on_request_line(&mut self, _: &mut HttpParser, request: RequestLine<'r>) {
        self.method = str::from_utf8(request.method).unwrap();
        self.path = str::from_utf8(request.path).unwrap();
        self.version = (
            request.version.0,
            request.version.1,
        );
    }

}

impl <'r> HttpMessageCallbacks<'r> for SimpleRequestCallback<'r> {
    fn on_header(&mut self, _: &mut HttpParser, name: &'r [u8], value: &'r [u8]) {
        self.headers.insert(str::from_utf8(name).unwrap(), value);
    }
    fn on_headers_finished(&mut self, _: &mut HttpParser, body_type: BodyType) -> ExpectBody {
        ExpectBody::Maybe
    }
    fn on_chunk(&mut self, _: &mut HttpParser, data: &[u8]) {
        // TODO: push_all?
        for d in data {
            self.chunks.push(*d);
        }
    }
    fn on_end(&mut self, _: &mut HttpParser) {
        self.finished = true;
    }
}


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
