
pub use nom_parsers::{RequestLine, ResponseLine};
use std::collections::HashMap;
use std::str;
use std::convert::From;

use parser::*;


#[derive(PartialEq,Eq,Debug)]
pub struct FullRequest {
    pub method: String,
    pub path: String,
    pub version: (u8, u8),
    pub headers: HashMap<String, Vec<u8>>,
    pub chunks: Vec<u8>,
    pub finished: bool,
}

impl <'a> From<FullRequestCallback<'a>> for FullRequest {
    fn from(r: FullRequestCallback<'a>) -> FullRequest {
        let mut headers = HashMap::with_capacity(r.headers.len());
        for (key, value) in r.headers {
            headers.insert(key.into(), value.into());
        }
        FullRequest {
            method: r.method.to_owned(),
            path: r.path.to_owned(),
            version: r.version,
            headers: headers,
            chunks: r.chunks,
            finished: r.finished,
        }
    }
}


#[derive(PartialEq,Eq,Debug)]
pub struct FullRequestCallback<'r> {
    pub method: &'r str,
    pub path: &'r str,
    pub version: (u8, u8),
    pub headers: HashMap< &'r str,  &'r [u8]>,
    pub chunks: Vec<u8>,
    pub finished: bool,
}

impl <'r> FullRequestCallback<'r> {
    pub fn new() -> FullRequestCallback<'r> {
        FullRequestCallback{
            method: "",
            path: "",
            version: (0, 0),
            headers: HashMap::new(),
            chunks: Vec::new(),
            finished: false,
        }
    }
}


impl <'r> HttpRequestCallbacks<'r> for FullRequestCallback<'r> {
    fn on_request_line(&mut self, _: &mut HttpParser, request: RequestLine<'r>) {
        self.method = str::from_utf8(request.method).unwrap();
        self.path = str::from_utf8(request.path).unwrap();
        self.version = (
            request.version.0,
            request.version.1,
        );
    }

}

impl <'r> HttpMessageCallbacks<'r> for FullRequestCallback<'r> {
    fn on_header(&mut self, _: &mut HttpParser, name: &'r [u8], value: &'r [u8]) {
        self.headers.insert(str::from_utf8(name).unwrap(), value);
    }
    fn on_headers_finished(&mut self, _: &mut HttpParser, _: BodyType) -> ExpectBody {
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
