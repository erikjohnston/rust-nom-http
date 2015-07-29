
use std::ascii::AsciiExt;


use errors::*;
use util;

use nom_parsers;
use nom_parsers::{RequestLine, ResponseLine};
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

#[derive(PartialEq,Eq,Debug,Clone,Copy)]
pub enum ExpectBody { Maybe, No }


pub trait HttpMessageCallbacks {
    fn on_header(&mut self, parser: &mut HttpParser, name: &[u8], value: &[u8]);
    fn on_headers_finished(&mut self, parser: &mut HttpParser, body_type: BodyType) -> ExpectBody;
    fn on_chunk(&mut self, parser: &mut HttpParser, data: &[u8]);
    fn on_end(&mut self, parser: &mut HttpParser);
}

pub trait HttpRequestCallbacks : HttpMessageCallbacks {
    fn on_request_line(&mut self, parser: &mut HttpParser, request: &RequestLine);
}

pub trait HttpResponseCallbacks: HttpMessageCallbacks {
    fn on_response_line(&mut self, parser: &mut HttpParser, response: &ResponseLine);
}


pub enum ParserType { Request, Response, }

/// The HttpParser object.
///
/// This stores the current state of the parsing of a stream of bytes. Each stream of bytes
/// has its own parser.
///
/// Each HttpParser can only process either requests or responses, not both.
pub struct HttpParser {
    pub body_type: BodyType,
    current_state: ParserState,
    body_finished: bool,
    expect_body: ExpectBody,
    parser_type: ParserType,
}

impl HttpParser {
    /// Constructs a new HttpParser.
    ///
    /// Must be told whether the HttpParser will be used to parse requests or responses.
    pub fn new(parser_type: ParserType) -> HttpParser {
        HttpParser {
            current_state: ParserState::FirstLine,
            body_type: match parser_type {
                ParserType::Request => BodyType::NoBody,
                ParserType::Response => BodyType::EOF,
            },
            body_finished: false,
            parser_type: parser_type,
            expect_body: ExpectBody::Maybe,
        }
    }

    pub fn parse_request<'r, T: HttpRequestCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<&'r [u8]> {
        let mut curr_input = input;
        if let ParserState::FirstLine = self.current_state {
            let res = try!(self.parse_request_line(cb, curr_input));
            curr_input = res.0;

            match res.1 {
                BufferState::Ready(next_state) => {
                    self.current_state = next_state;
                },
                BufferState::Incomplete => {
                    return Ok(curr_input);
                }
            }
        }

        self.parse_http(cb, curr_input)
    }

    pub fn parse_response<'r, T: HttpResponseCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<&'r [u8]> {
        let mut curr_input = input;
        if let ParserState::FirstLine = self.current_state {
            let res = try!(self.parse_response_line(cb, curr_input));
            curr_input = res.0;

            match res.1 {
                BufferState::Ready(next_state) => {
                    self.current_state = next_state;
                },
                BufferState::Incomplete => {
                    return Ok(curr_input);
                }
            }
        }

        self.parse_http(cb, curr_input)
    }

    fn parse_http<'r, T: HttpMessageCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<&'r [u8]> {
        let mut curr_input = input;
        loop {
            let res = match self.current_state {
                ParserState::FirstLine => unreachable!(),
                ParserState::Headers => try!(self.parse_header(cb, curr_input)),
                ParserState::HeaderEnd => try!(self.parse_header_end(cb, curr_input)),
                ParserState::Body(body_type) => try!(self.parse_body(cb, curr_input, body_type)),
                ParserState::Done => {
                    cb.on_end(self);
                    self.body_type = match self.parser_type {
                        ParserType::Request => BodyType::NoBody,
                        ParserType::Response => BodyType::EOF,
                    };
                    self.body_finished = false;
                    self.current_state = ParserState::FirstLine;
                    self.expect_body = ExpectBody::Maybe;
                    return Ok(curr_input);
                }
            };

            curr_input = res.0;

            match res.1 {
                BufferState::Ready(next_state) => {
                    self.current_state = next_state;
                },
                BufferState::Incomplete => {
                    return Ok(curr_input);
                }
            }
        }
    }

    fn parse_request_line<'r, T: HttpRequestCallbacks>(&mut self, cb: &mut T, input: &'r [u8])
    -> HttpParserResult<ParserReturn<'r>> {
        Ok(match nom_parsers::request_line(input) {
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
        Ok(match nom_parsers::response_line(input) {
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
            match nom_parsers::header(&input[start..]) {
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
        Ok(match nom_parsers::empty_line(input) {
            IResult::Error(_) => return Err(HttpParserError::BadHeader),
            IResult::Incomplete(_) => ParserReturn(input, BufferState::Incomplete),
            IResult::Done(i, _) => {
                if self.body_finished {
                    // We were dealing with trailing headers, so now we've finished.
                    ParserReturn(i, BufferState::Ready(ParserState::Done))
                } else {
                    let body_type = self.body_type;
                    self.expect_body = cb.on_headers_finished(self, body_type);

                    let body_state = match self.expect_body {
                        ExpectBody::Maybe => match self.body_type {
                            BodyType::Chunked => BodyTypeState::Chunked(ChunkedState::Header),
                            BodyType::Length(len) => BodyTypeState::Lenth(len),
                            BodyType::EOF => BodyTypeState::EOF,
                            BodyType::NoBody => BodyTypeState::NoBody,
                        },
                        ExpectBody::No => BodyTypeState::NoBody,
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
                if input.len() == 0 && size != 0 {
                    return Ok(ParserReturn(input, BufferState::Incomplete));
                } else if input.len() < size {
                    cb.on_chunk(self, input);
                    ParserReturn(
                        b"",
                        BufferState::Ready(
                            ParserState::Body(BodyTypeState::Lenth(size - input.len()))
                        )
                    )
                } else {
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
                        match nom_parsers::chunk_parser(input) {
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
                        match nom_parsers::empty_line(input) {
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
