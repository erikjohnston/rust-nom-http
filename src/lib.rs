
use std::ascii::AsciiExt;

#[macro_use]
extern crate nom;

pub mod util;
pub mod errors;

use errors::*;


mod parsers;
pub use parsers::RequestLine;
use nom::{IResult};

pub type HttpParserResult = Result<usize, HttpParserError>;


#[derive(PartialEq,Eq,Debug,Clone,Copy)]
enum ParserState {
    RequestLine,
    Headers,
    HeaderEnd,
    Body(BodyTypeState),
    Done,
}

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
enum ChunkedState { Header, Data(usize), DataEnd }

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
enum BodyTypeState { Lenth(usize), Chunked(ChunkedState), EOF, NoBody }

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
pub enum BodyType { Length(usize), Chunked, EOF, NoBody }


pub trait HttpCallbacks {
    fn on_request_line(&mut self, parser: &mut HttpParser, request: &RequestLine);
    fn on_header(&mut self, parser: &mut HttpParser, name: &[u8], value: &[u8]);
    fn on_message_begin(&mut self, parser: &mut HttpParser, body_type: BodyType);
    fn on_chunk(&mut self, parser: &mut HttpParser, data: &[u8]);
    fn on_end(&mut self, parser: &mut HttpParser);
}


pub struct HttpParser {
    pub body_type: BodyType,
    current_state: ParserState,
    body_finished: bool
}

impl HttpParser {
    pub fn new() -> HttpParser {
        HttpParser{
            current_state: ParserState::RequestLine,
            body_type: BodyType::NoBody,
            body_finished: false
        }
    }

    pub fn parse_http<T: HttpCallbacks>(&mut self, cb: &mut T, input: &[u8]) -> HttpParserResult {
        let mut curr_input = input;
        loop {
            let (size, next_state) = match self.current_state {
                ParserState::RequestLine => try!(self.parse_request_line(cb, curr_input)),
                ParserState::Headers => try!(self.parse_header(cb, curr_input)),
                ParserState::HeaderEnd => try!(self.parse_header_end(cb, curr_input)),
                ParserState::Body(body_type) => try!(self.parse_body(cb, curr_input, body_type)),
                ParserState::Done => {
                    // TODO: Reset things.
                    cb.on_end(self);
                    self.body_type = BodyType::NoBody;
                    self.body_finished = false;
                    self.current_state = ParserState::RequestLine;
                    return Ok(input.len() - curr_input.len());
                }
            };
            if size > 0 || next_state != self.current_state {
                self.current_state = next_state;
                curr_input = &curr_input[size..];
            } else {
                return Ok(input.len() - curr_input.len());
            }
        }
    }

    fn parse_request_line<T: HttpCallbacks>(&mut self, cb: &mut T, input: &[u8]) -> Result<(usize, ParserState), HttpParserError> {
        Ok(match parsers::request_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadFirstLine),
            IResult::Incomplete(_) => (0, self.current_state),
            IResult::Done(i, request) => {
                cb.on_request_line(self, &request);
                (input.len() - i.len(), ParserState::Headers)
            }
        })
    }

    fn parse_header<T: HttpCallbacks>(&mut self, cb: &mut T, input: &[u8]) -> Result<(usize, ParserState), HttpParserError> {
        let mut start = 0;
        loop {
            match parsers::header(&input[start..]) {
                IResult::Error(_) => {
                    return Ok((start, ParserState::HeaderEnd))
                },
                IResult::Incomplete(_) => return Ok((0, self.current_state)),
                IResult::Done(i, (name, value)) => {
                    cb.on_header(self, name, value);
                    if let Some(body_type) = try!(body_type_from_header(name, value)) {
                        self.body_type = body_type;
                    }

                    start = input.len() - i.len();
                }
            }
        }
    }

    fn parse_header_end<T: HttpCallbacks>(&mut self, cb: &mut T, input: &[u8]) -> Result<(usize, ParserState), HttpParserError> {
        Ok(match parsers::empty_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadHeader),
            IResult::Incomplete(_) => (0, self.current_state),
            IResult::Done(i, _) => {
                if self.body_finished {
                    // We were dealing with trailing headers, so now we've finished.
                    (input.len() - i.len(), ParserState::Done)
                } else {
                    let body_type = self.body_type;
                    cb.on_message_begin(self, body_type);

                    let body_state = match self.body_type {
                        BodyType::Chunked => BodyTypeState::Chunked(ChunkedState::Header),
                        BodyType::Length(len) => BodyTypeState::Lenth(len),
                        BodyType::EOF => BodyTypeState::EOF,
                        BodyType::NoBody => BodyTypeState::NoBody,
                    };

                    (input.len() - i.len(), ParserState::Body(body_state))
                }
            }
        })
    }

    fn parse_body<T: HttpCallbacks>(&mut self, cb: &mut T, input: &[u8], body_type: BodyTypeState) -> Result<(usize, ParserState), HttpParserError> {
        Ok(match body_type {
            BodyTypeState::Lenth(size) => {
                if input.len() < size {
                    cb.on_chunk(self, input);
                    (
                        input.len(),
                        ParserState::Body(BodyTypeState::Lenth(size - input.len()))
                    )
                } else {
                    cb.on_chunk(self, &input[..size]);
                    (size, ParserState::Done)
                }
            },
            BodyTypeState::Chunked(chunk_state) => {
                match chunk_state {
                    ChunkedState::Header => {
                        match parsers::chunk_parser(input) {
                            IResult::Error(_) => return Err(HttpParserError::BadBodyChunkHeader),
                            IResult::Incomplete(_) => (0, self.current_state),
                            IResult::Done(i, chunk_header) => {
                                // TODO: Handle chunk_header
                                (
                                    input.len() - i.len(),
                                    ParserState::Body(
                                        BodyTypeState::Chunked(
                                            ChunkedState::Data(
                                                chunk_header.size
                                            )
                                        )
                                    )
                                )
                            }
                        }
                    },
                    ChunkedState::Data(size) => {
                        if input.len() < size {
                            cb.on_chunk(self, input);
                            (
                                input.len(),
                                ParserState::Body(
                                    BodyTypeState::Chunked(
                                        ChunkedState::Data(
                                            size - input.len()
                                        )
                                    )
                                )
                            )
                        } else {
                            if size > 0 {
                                cb.on_chunk(self, &input[..size]);
                                (
                                    size,
                                    ParserState::Body(
                                        BodyTypeState::Chunked(ChunkedState::DataEnd)
                                    )
                                )
                            } else {
                                self.body_finished = true;
                                (
                                    size,
                                    ParserState::Headers
                                )
                            }
                        }
                    },
                    ChunkedState::DataEnd => {
                        match parsers::empty_line(input) {
                            IResult::Error(_) => return Err(HttpParserError::BadBodyChunkHeader),
                            IResult::Incomplete(_) => (0, self.current_state),
                            IResult::Done(i, _) => {
                                (
                                    input.len() - i.len(),
                                    ParserState::Body(
                                        BodyTypeState::Chunked(ChunkedState::Header)
                                    )
                                )
                            }
                        }
                    },
                }
            },
            BodyTypeState::EOF => {
                cb.on_chunk(self, input);
                (input.len(), self.current_state)
            },
            BodyTypeState::NoBody => (0, ParserState::Done),
        })
    }
}


fn body_type_from_header(name: &[u8], value: &[u8]) -> Result<Option<BodyType>, HttpHeaderParseError> {
    if b"transfer-encoding".eq_ignore_ascii_case(name) {
        // TODO: Comparison isn't this simple.
        if b"chunked".eq_ignore_ascii_case(value) {
            return Ok(Some(BodyType::Chunked))
        }
    } else if b"content-length".eq_ignore_ascii_case(name) {
        return match util::dec_buf_to_int(value) {
            Ok(size) => Ok(Some(BodyType::Length(size))),
            Err(e) => Err(HttpHeaderParseError::ContentLength(e)),
        };
    }
    Ok(None)
}
