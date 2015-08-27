
pub use nom_parsers::{RequestLine, ResponseLine};
use std::collections::HashMap;
use std::str;
use std::convert::From;

use parser::*;

use std::borrow::Borrow;


pub trait BufferedRequest {
    fn method(&self) -> &str;
    fn path(&self) -> &str;
    fn version(&self) -> (u8, u8);
    fn header(&self, &str) -> Option<&[u8]>;
}


#[derive(PartialEq,Eq,Debug)]
pub struct BufferedRequestOwned {
    pub method: String,
    pub path: String,
    pub version: (u8, u8),
    pub headers: HashMap<String, Vec<u8>>,
    pub chunks: Vec<u8>,
    pub finished: bool,
}

impl BufferedRequestOwned {
    pub fn new() -> BufferedRequestOwned {
        BufferedRequestOwned{
            method: String::new(),
            path: String::new(),
            version: (0, 0),
            headers: HashMap::new(),
            chunks: Vec::new(),
            finished: false,
        }
    }
}

impl <'a> From<BufferedRequestCallback<'a>> for BufferedRequestOwned {
    fn from(r: BufferedRequestCallback<'a>) -> BufferedRequestOwned {
        let mut headers = HashMap::with_capacity(r.headers.len());
        for (key, value) in r.headers {
            headers.insert(key.into(), value.into());
        }
        BufferedRequestOwned {
            method: r.method.to_owned(),
            path: r.path.to_owned(),
            version: r.version,
            headers: headers,
            chunks: r.chunks,
            finished: r.finished,
        }
    }
}

impl BufferedRequest for BufferedRequestOwned {
    fn path(&self) -> &str {self.path.borrow()}
    fn method(&self) -> &str {self.method.borrow()}
    fn version(&self) -> (u8, u8) {self.version}

    fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers.get(name).map(|v| v.borrow())
    }
}

impl <'r> HttpRequestCallbacks<'r> for BufferedRequestOwned {
    fn on_request_line(&mut self, _: &mut HttpParser, request: RequestLine<'r>) {
        self.method = str::from_utf8(request.method).unwrap().to_owned();
        self.path = str::from_utf8(request.path).unwrap().to_owned();
        self.version = (
            request.version.0,
            request.version.1,
        );
    }

}

impl <'r> HttpMessageCallbacks<'r> for BufferedRequestOwned {
    fn on_header(&mut self, _: &mut HttpParser, name: &'r [u8], value: &'r [u8]) {
        self.headers.insert(
            str::from_utf8(name).unwrap().to_owned(),
            value.to_owned()
        );
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


#[derive(PartialEq,Eq,Debug)]
pub struct BufferedRequestCallback<'r> {
    pub method: &'r str,
    pub path: &'r str,
    pub version: (u8, u8),
    pub headers: HashMap< &'r str,  &'r [u8]>,
    pub chunks: Vec<u8>,
    pub finished: bool,
}

impl <'r> BufferedRequestCallback<'r> {
    pub fn new() -> BufferedRequestCallback<'r> {
        BufferedRequestCallback{
            method: "",
            path: "",
            version: (0, 0),
            headers: HashMap::new(),
            chunks: Vec::new(),
            finished: false,
        }
    }
}


impl <'r> BufferedRequest for BufferedRequestCallback<'r> {
    fn path(&self) -> &str {self.path}
    fn method(&self) -> &str {self.method}
    fn version(&self) -> (u8, u8) {self.version}

    fn header(&self, name: &str) -> Option<&[u8]> {
        self.headers.get(name).map(|v| *v)
    }
}


impl <'r> HttpRequestCallbacks<'r> for BufferedRequestCallback<'r> {
    fn on_request_line(&mut self, _: &mut HttpParser, request: RequestLine<'r>) {
        self.method = str::from_utf8(request.method).unwrap();
        self.path = str::from_utf8(request.path).unwrap();
        self.version = (
            request.version.0,
            request.version.1,
        );
    }

}

impl <'r> HttpMessageCallbacks<'r> for BufferedRequestCallback<'r> {
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
