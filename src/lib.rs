
use std::ascii::AsciiExt;

#[macro_use]
extern crate nom;

pub mod util;
pub mod errors;

use errors::*;


mod parsers;
pub use parsers::{RequestLine, ResponseLine};
use nom::{IResult};

pub type HttpParserResult<T> = Result<T, HttpParserError>;


#[derive(PartialEq,Eq,Debug,Clone,Copy)]
enum ParserState {
    FirstLine,
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

enum BufferState { Ready(ParserState), Incomplete, }

struct ParserReturn<'r> (&'r [u8], BufferState);


pub trait HttpMessageCallbacks {
    fn on_header(&mut self, parser: &mut HttpParser, name: &[u8], value: &[u8]);
    fn on_message_begin(&mut self, parser: &mut HttpParser, body_type: BodyType);
    fn on_chunk(&mut self, parser: &mut HttpParser, data: &[u8]);
    fn on_end(&mut self, parser: &mut HttpParser);
}

pub trait HttpRequestCallbacks : HttpMessageCallbacks {
    fn on_request_line(&mut self, parser: &mut HttpParser, request: &RequestLine);
}

pub trait HttpResponseCallbacks: HttpMessageCallbacks {
    fn on_response_line(&mut self, parser: &mut HttpParser, request: &ResponseLine);
}


pub enum ParserType { Request, Response, }


pub struct HttpParser {
    pub body_type: BodyType,
    current_state: ParserState,
    body_finished: bool,
    parser_type: ParserType,
}

impl HttpParser {
    pub fn new(parser_type: ParserType) -> HttpParser {
        HttpParser {
            current_state: ParserState::FirstLine,
            body_type: BodyType::NoBody,
            body_finished: false,
            parser_type: parser_type,
        }
    }

    pub fn parse_request<T: HttpRequestCallbacks + HttpResponseCallbacks>(&mut self, cb: &mut T, input: &[u8])
    -> HttpParserResult<usize> {
        let mut curr_input = input;
        if let ParserState::FirstLine = self.current_state {
            let res = try!(self.parse_request_line(cb, curr_input));
            curr_input = res.0;

            match res.1 {
                BufferState::Ready(next_state) => {
                    self.current_state = next_state;
                },
                BufferState::Incomplete => {
                    return Ok(input.len() - curr_input.len());
                }
            }
        }

        self.parse_http(cb, curr_input)
    }

    pub fn parse_http<T: HttpRequestCallbacks + HttpResponseCallbacks>(&mut self, cb: &mut T, input: &[u8])
    -> HttpParserResult<usize> {
        let mut curr_input = input;
        loop {
            let res = match self.current_state {
                ParserState::FirstLine => match self.parser_type {
                    ParserType::Request => try!(self.parse_request_line(cb, curr_input)),
                    ParserType::Response => try!(self.parse_response_line(cb, curr_input)),
                },
                ParserState::Headers => try!(self.parse_header(cb, curr_input)),
                ParserState::HeaderEnd => try!(self.parse_header_end(cb, curr_input)),
                ParserState::Body(body_type) => try!(self.parse_body(cb, curr_input, body_type)),
                ParserState::Done => {
                    cb.on_end(self);
                    self.body_type = BodyType::NoBody;
                    self.body_finished = false;
                    self.current_state = ParserState::FirstLine;
                    return Ok(input.len() - curr_input.len());
                }
            };

            curr_input = res.0;

            match res.1 {
                BufferState::Ready(next_state) => {
                    self.current_state = next_state;
                },
                BufferState::Incomplete => {
                    return Ok(input.len() - curr_input.len());
                }
            }
        }
    }

    fn parse_request_line<'r, T: HttpRequestCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<ParserReturn<'r>> {
        Ok(match parsers::request_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadFirstLine),
            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
            IResult::Done(i, request) => {
                cb.on_request_line(self, &request);
                ParserReturn(i, BufferState::Ready(ParserState::Headers))
            }
        })
    }

    fn parse_response_line<'r, T: HttpResponseCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<ParserReturn<'r>> {
        Ok(match parsers::response_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadFirstLine),
            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
            IResult::Done(i, response) => {
                cb.on_response_line(self, &response);
                ParserReturn(i, BufferState::Ready(ParserState::Headers))
            }
        })
    }

    fn parse_header<'r, T: HttpMessageCallbacks>(&mut self, cb: &mut T, input: &'r[u8])
    -> HttpParserResult<ParserReturn<'r>> {
        let mut start = 0;
        loop {
            match parsers::header(&input[start..]) {
                IResult::Error(_) => {
                    return Ok(ParserReturn(&input[start..], BufferState::Ready(ParserState::HeaderEnd)))
                },
                IResult::Incomplete(_) => return Ok(ParserReturn(input, BufferState::Incomplete)),
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

    fn parse_header_end<'r, T: HttpMessageCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<ParserReturn<'r>> {
        Ok(match parsers::empty_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadHeader),
            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
            IResult::Done(i, _) => {
                if self.body_finished {
                    // We were dealing with trailing headers, so now we've finished.
                    ParserReturn(i, BufferState::Ready(ParserState::Done))
                } else {
                    let body_type = self.body_type;
                    cb.on_message_begin(self, body_type);

                    let body_state = match self.body_type {
                        BodyType::Chunked => BodyTypeState::Chunked(ChunkedState::Header),
                        BodyType::Length(len) => BodyTypeState::Lenth(len),
                        BodyType::EOF => BodyTypeState::EOF,
                        BodyType::NoBody => BodyTypeState::NoBody,
                    };

                    ParserReturn(i, BufferState::Ready(ParserState::Body(body_state)))
                }
            }
        })
    }

    fn parse_body<'r, T: HttpMessageCallbacks>(&mut self, cb: &mut T, input: &'r [u8], body_type: BodyTypeState)
    -> HttpParserResult<ParserReturn<'r>> {
        Ok(match body_type {
            BodyTypeState::Lenth(size) => {
                if input.len() < size {
                    cb.on_chunk(self, input);
                    ParserReturn(
                        b"",
                        BufferState::Ready(
                            ParserState::Body(BodyTypeState::Lenth(size - input.len()))
                        )
                    )
                } else {
                    if input.len() == 0 {
                        return Ok(ParserReturn(input, BufferState::Incomplete));
                    }

                    cb.on_chunk(self, &input[..size]);
                    ParserReturn(&input[size..], BufferState::Ready(ParserState::Done))
                }
            },
            BodyTypeState::Chunked(chunk_state) => {
                if input.len() == 0 {
                    return Ok(ParserReturn(input, BufferState::Incomplete));
                }

                match chunk_state {
                    ChunkedState::Header => {
                        match parsers::chunk_parser(input) {
                            IResult::Error(_) => return Err(HttpParserError::BadBodyChunkHeader),
                            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
                            IResult::Done(i, chunk_header) => {
                                // TODO: Handle chunk_header
                                ParserReturn(
                                    i,
                                    BufferState::Ready(
                                        ParserState::Body(
                                            BodyTypeState::Chunked(
                                                ChunkedState::Data(
                                                    chunk_header.size
                                                )
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
                            ParserReturn(
                                b"",
                                BufferState::Ready(
                                    ParserState::Body(
                                        BodyTypeState::Chunked(
                                            ChunkedState::Data(
                                                size - input.len()
                                            )
                                        )
                                    )
                                )
                            )
                        } else {
                            if size > 0 {
                                cb.on_chunk(self, &input[..size]);
                                ParserReturn(
                                    &input[size..],
                                    BufferState::Ready(
                                        ParserState::Body(
                                            BodyTypeState::Chunked(ChunkedState::DataEnd)
                                        )
                                    )
                                )
                            } else {
                                self.body_finished = true;
                                ParserReturn(
                                    &input[size..],
                                    BufferState::Ready(
                                        ParserState::Headers
                                    )
                                )
                            }
                        }
                    },
                    ChunkedState::DataEnd => {
                        match parsers::empty_line(input) {
                            IResult::Error(_) => return Err(HttpParserError::BadBodyChunkHeader),
                            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
                            IResult::Done(i, _) => {
                                ParserReturn(
                                    i,
                                    BufferState::Ready(
                                        ParserState::Body(
                                            BodyTypeState::Chunked(ChunkedState::Header)
                                        )
                                    )
                                )
                            }
                        }
                    },
                }
            },
            BodyTypeState::EOF => {
                if input.len() == 0 {
                    return Ok(ParserReturn(input, BufferState::Incomplete));
                }

                cb.on_chunk(self, input);
                ParserReturn(b"", BufferState::Incomplete)
            },
            BodyTypeState::NoBody => ParserReturn(input, BufferState::Ready(ParserState::Done)),
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
