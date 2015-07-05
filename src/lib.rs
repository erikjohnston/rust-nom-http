
use std::ascii::AsciiExt;

#[macro_use]
extern crate nom;

pub mod util;

mod parsers;
pub use parsers::RequestLine;
use nom::{IResult};


#[derive(PartialEq,Eq,Debug,Clone,Copy)]
enum State {
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
enum ParserState { Consumed(usize, State), Error }

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
    current_state: State,
    body_finished: bool
}

impl HttpParser {
    pub fn new() -> HttpParser {
        HttpParser{
            current_state: State::RequestLine,
            body_type: BodyType::EOF,
            body_finished: false
        }
    }

    pub fn parse_http(&mut self, cb: &mut HttpCallbacks, mut input: &[u8]) {
        loop {
            let parser_state = self.parse_http_inner(cb, input);
            if let ParserState::Consumed(size, next_state) = parser_state {
                if size > 0 || next_state != self.current_state {
                    self.current_state = next_state;
                    input = &input[size..];

                    // if input.len() == 0 {
                    //     return parser_state;
                    // }
                } else {
                    return;
                }
            } else {
                return;
            }
        }
    }

    fn parse_http_inner(&mut self, cb: &mut HttpCallbacks, input: &[u8]) -> ParserState {
        match self.current_state {
            State::RequestLine => {
                match parsers::request_line(input) {
                    IResult::Error(_) => ParserState::Error,
                    IResult::Incomplete(_) => ParserState::Consumed(0, self.current_state),
                    IResult::Done(i, request) => {
                        cb.on_request_line(&request);
                        ParserState::Consumed(input.len() - i.len(), State::Headers)
                    }
                }
            },
            State::Headers => {
                match parsers::header(input) {
                    IResult::Error(_) => {
                        ParserState::Consumed(0, State::HeaderEnd)
                    },
                    IResult::Incomplete(_) => ParserState::Consumed(0, self.current_state),
                    IResult::Done(i, (name, value)) => {
                        cb.on_header(name, value);
                        if let Some(body_type) = body_type_from_header(name, value) {
                            self.body_type = body_type;
                        }
                        ParserState::Consumed(input.len() - i.len(), State::Headers)
                    }
                }
            },
            State::HeaderEnd => {
                match parsers::empty_line(input) {
                    IResult::Error(_) => ParserState::Error,
                    IResult::Incomplete(_) => ParserState::Consumed(0, self.current_state),
                    IResult::Done(i, _) => {
                        if self.body_finished {
                            // We were dealing with trailing headers, so now we've finished.
                            cb.on_end();
                            ParserState::Consumed(input.len() - i.len(), State::Done)
                        } else {
                            cb.on_message_begin(self.body_type);

                            let body_state = match self.body_type {
                                BodyType::Chunked => BodyTypeState::Chunked(ChunkedState::Header),
                                BodyType::Length(len) => BodyTypeState::Lenth(len),
                                BodyType::EOF => BodyTypeState::EOF,
                            };

                            ParserState::Consumed(
                                input.len() - i.len(), State::Body(body_state)
                            )
                        }
                    }
                }
            },
            State::Body(body_type) => {
                match body_type {
                    BodyTypeState::Lenth(size) => {
                        if input.len() < size {
                            cb.on_chunk(input);
                            ParserState::Consumed(
                                input.len(),
                                State::Body(BodyTypeState::Lenth(size - input.len()))
                            )
                        } else {
                            cb.on_chunk(&input[..size]);
                            cb.on_end();
                            ParserState::Consumed(size, State::Done)
                        }
                    },
                    BodyTypeState::Chunked(chunk_state) => {
                        match chunk_state {
                            ChunkedState::Header => {
                                match parsers::chunk_parser(input) {
                                    IResult::Error(_) => ParserState::Error,
                                    IResult::Incomplete(_) => ParserState::Consumed(0, self.current_state),
                                    IResult::Done(i, chunk_header) => {
                                        // TODO: Handle chunk_header
                                        ParserState::Consumed(
                                            input.len() - i.len(),
                                            State::Body(
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
                                    ParserState::Consumed(
                                        input.len(),
                                        State::Body(
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
                                        ParserState::Consumed(
                                            size,
                                            State::Body(
                                                BodyTypeState::Chunked(ChunkedState::DataEnd)
                                            )
                                        )
                                    } else {
                                        self.body_finished = true;
                                        ParserState::Consumed(
                                            size,
                                            State::Headers
                                        )
                                    }
                                }
                            },
                            ChunkedState::DataEnd => {
                                match parsers::empty_line(input) {
                                    IResult::Error(_) => ParserState::Error,
                                    IResult::Incomplete(_) => ParserState::Consumed(0, self.current_state),
                                    IResult::Done(i, _) => {
                                        ParserState::Consumed(
                                            input.len() - i.len(),
                                            State::Body(
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
                        ParserState::Consumed(input.len(), self.current_state)
                    },
                }
            },
            State::Done => {
                // this should not be called
                ParserState::Error
            }
        }
    }
}


fn body_type_from_header(name: &[u8], value: &[u8]) -> Option<BodyType> {
    if b"transfer-encoding".eq_ignore_ascii_case(name) {
        if b"chunked".eq_ignore_ascii_case(value) {
            return Some(BodyType::Chunked)
        }
    } else if b"content-length".eq_ignore_ascii_case(name) {
        // TODO: This should error!
        if let Some(size) = util::dec_buf_to_int(value).ok() {
            return Some(BodyType::Length(size))
        }
    }
    None
}
