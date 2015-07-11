
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
enum BodyTypeState { Lenth(usize), Chunked(ChunkedState), EOF }

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
pub enum BodyType { Length(usize), Chunked, EOF }


pub trait HttpCallbacks {
    fn on_request_line(&mut self, request: &RequestLine);
    fn on_header(&mut self, name: &[u8], value: &[u8]);
    fn on_message_begin(&mut self, body_type: BodyType);
    fn on_chunk(&mut self, data: &[u8]);
    fn on_end(&mut self);
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
            body_type: BodyType::EOF,
            body_finished: false
        }
    }

    pub fn parse_http(&mut self, cb: &mut HttpCallbacks, input: &[u8]) -> HttpParserResult {
        let mut curr_input = input;
        loop {
            let (size, next_state) = try!(self.parse_http_inner(cb, curr_input));
            if size > 0 || next_state != self.current_state {
                self.current_state = next_state;
                curr_input = &curr_input[size..];
            } else {
                return Ok(input.len() - curr_input.len());
            }
        }
    }

    fn parse_http_inner(&mut self, cb: &mut HttpCallbacks, input: &[u8]) -> Result<(usize, ParserState), HttpParserError> {
        let res = match self.current_state {
            ParserState::RequestLine => {
                match parsers::request_line(input) {
                    IResult::Error(_) => return Err(HttpParserError::BadFirstLine),
                    IResult::Incomplete(_) => (0, self.current_state),
                    IResult::Done(i, request) => {
                        cb.on_request_line(&request);
                        (input.len() - i.len(), ParserState::Headers)
                    }
                }
            },
            ParserState::Headers => {
                match parsers::header(input) {
                    IResult::Error(_) => {
                        (0, ParserState::HeaderEnd)
                    },
                    IResult::Incomplete(_) => (0, self.current_state),
                    IResult::Done(i, (name, value)) => {
                        cb.on_header(name, value);
                        if let Some(body_type) = try!(body_type_from_header(name, value)) {
                            self.body_type = body_type;
                        }
                        (input.len() - i.len(), ParserState::Headers)
                    }
                }
            },
            ParserState::HeaderEnd => {
                match parsers::empty_line(input) {
                    IResult::Error(_) => return Err(HttpParserError::BadHeader),
                    IResult::Incomplete(_) => (0, self.current_state),
                    IResult::Done(i, _) => {
                        if self.body_finished {
                            // We were dealing with trailing headers, so now we've finished.
                            (input.len() - i.len(), ParserState::Done)
                        } else {
                            cb.on_message_begin(self.body_type);

                            let body_state = match self.body_type {
                                BodyType::Chunked => BodyTypeState::Chunked(ChunkedState::Header),
                                BodyType::Length(len) => BodyTypeState::Lenth(len),
                                BodyType::EOF => BodyTypeState::EOF,
                            };

                            (input.len() - i.len(), ParserState::Body(body_state))
                        }
                    }
                }
            },
            ParserState::Body(body_type) => {
                match body_type {
                    BodyTypeState::Lenth(size) => {
                        if input.len() < size {
                            cb.on_chunk(input);
                            (
                                input.len(),
                                ParserState::Body(BodyTypeState::Lenth(size - input.len()))
                            )
                        } else {
                            cb.on_chunk(&input[..size]);
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
                                    cb.on_chunk(input);
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
                                        cb.on_chunk(&input[..size]);
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
                        cb.on_chunk(input);
                        (input.len(), self.current_state)
                    },
                }
            },
            ParserState::Done => {
                // TODO: Reset things.
                cb.on_end();
                self.body_type = BodyType::EOF;
                self.body_finished = false;
                (0, ParserState::RequestLine)
            }
        };

        Ok(res)
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
