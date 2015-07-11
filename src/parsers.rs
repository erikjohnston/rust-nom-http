
use nom::{IResult, Needed, Err, space, digit};

use util::hex_buf_to_int;


// named!(not_space, take_until_either!(" \t"));
// named!(not_space_or_colon, take_until_either!(" \t:"));
// named!(
//     token,
//     is_a!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&'*+-^_`|-")
// );


fn not_space(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for idx in 0..input.len() {
        match input[idx] {
            b' ' | b'\t' => return IResult::Done(&input[idx..], &input[..idx]),
            b'\n' | b'\r' => return IResult::Error(Err::Code(0)),
            _ => continue,
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for idx in 0..input.len() {
        match input[idx] {
            33 | 35...39 | 42 | 43 | 45 | 48...57 | 65...90 | 94...122 | 124 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn hexdigit(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for idx in 0..input.len() {
        match input[idx] {
            48...57 | 65...90 | 94...122 => continue,
            _ => return IResult::Done(&input[idx..], &input[..idx]),
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

fn not_space_or_semicolon(input: &[u8]) -> IResult<&[u8], &[u8]> {
    for idx in 0..input.len() {
        match input[idx] {
            b' ' | b'\t' | b';' | b'\n' | b'\r' => return IResult::Done(&input[idx..], &input[..idx]),
            _ => continue,
        }
    }
    IResult::Incomplete(Needed::Size(1))
}


#[derive(Debug, PartialEq, Clone)]
pub struct RequestLine<'r> {
    pub method: &'r [u8],
    pub path: &'r [u8],
    pub version: (&'r [u8], &'r [u8]),
}

named!(
    pub request_line <&[u8], RequestLine>,
    chain!(
        method: not_space   ~
        space               ~
        path: not_space     ~
        space               ~
        tag!("HTTP/")       ~
        major: digit        ~
        tag!(".")           ~
        minor: digit        ~
        space?              ~
        tag!("\r")?         ~
        tag!("\n")          ,
        || {RequestLine{method: method, path:path, version: (major, minor)}}
    )
);

// We need this to deal with the insanity of obs-fold
fn take_header_value(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut end_pos = 0;
    let mut idx = 0;
    while idx < buf.len() {
        match buf[idx] {
            b'\n' => {
                idx += 1;
                if idx >= buf.len() {
                    return IResult::Incomplete(Needed::Size(1));
                }
                match buf[idx] {
                    b' ' | b'\t' => {
                        idx += 1;
                        continue;
                    },
                    _ => if end_pos > 0 {
                        return IResult::Done(&buf[end_pos..], &buf[..end_pos])
                    } else {
                        return IResult::Error(Err::Code(0))
                    },
                }
            },
            b' ' | b'\t' | b'\r' => {},
            _ => end_pos = idx + 1,
        }
        idx += 1;
    }
    IResult::Incomplete(Needed::Size(1))
}

named!(
    pub header <&[u8], (&[u8], &[u8])>,
    chain!(
        name: token                 ~
        space?                      ~
        tag!(":")                   ~
        space?                      ~
        value: take_header_value    ~
        tag!("\r")?                 ~
        tag!("\n")                  ,
        || {(name, value)}
    )
);


fn quoted_string_or_token(buf: &[u8]) -> IResult<&[u8], &[u8]> {
    if buf.len() == 0 {
        return IResult::Incomplete(Needed::Size(1));
    }

    if buf[0] != b'"' {
        return not_space_or_semicolon(buf);
    }

    let mut idx = 1;
    while idx < buf.len() {
        match buf[idx] {
            b'\\' => {
                idx += 2;
                continue;
            },
            b'"' => {
                return IResult::Done(&buf[idx+1..], &buf[1..idx]);
            }
            b' ' | b'\t' | 0x21 | 0x23...0x5b | 0x5D...0x7E | 0x80...0xFF => {
                idx += 1;
                continue;
            },
            _ => {
                return IResult::Error(Err::Code(0));
            }
        }
    }
    IResult::Incomplete(Needed::Size(1))
}

named!(
    chunk_parameter_value,
    chain!(
        tag!("=")   ~
        space?      ~
        value: quoted_string_or_token,
        || value
    )
);

named!(
    pub empty_line,
    chain!(
        tag!("\r")? ~
        tag!("\n")  ,
        || b""
    )
);

named!(
    chunk_parameter<&[u8], (&[u8], Option<&[u8]>)>,
    chain!(
        tag!(";")                           ~
        space?                              ~
        name: token                         ~
        space?                              ~
        value: opt!(chunk_parameter_value)  ~
        space?                              ,
        || (name, value)
    )
);

fn many_chunk_params(input: &[u8]) -> IResult<&[u8], Vec<(&[u8], Option<&[u8]>)>> {
    let mut begin = 0;
    let mut remaining = input.len();
    let mut res = Vec::new();
    loop {
        match chunk_parameter(&input[begin..]) {
            IResult::Done(i, o) => {
                if i.len() == input[begin..].len() {
                    break;
                }
                res.push(o);
                begin += remaining - i.len();
                remaining = i.len();
            },
            IResult::Incomplete(s) => {
                return IResult::Incomplete(s);
            },
            _ => {
                break;
            }
        }
    }
    IResult::Done(&input[begin..], res)
}

#[derive(Debug, PartialEq)]
pub struct ChunkHeader<'r> {
    pub parameters: Vec<(&'r[u8], Option<&'r[u8]>)>,
    pub size: usize,
}

named!(
    pub chunk_parser <&[u8], ChunkHeader>,
    chain!(
        size: map_res!(
            hexdigit,
            hex_buf_to_int
        )                                       ~
        space?                                  ~
        values: many_chunk_params               ~
        space?                                  ~
        tag!("\r")?                             ~
        tag!("\n")                              ,
        || ChunkHeader{parameters: values, size: size}
    )
);


// ***************************************
// **************** TESTS ****************
// ***************************************

macro_rules! test_parser {
    ($f:ident, $($r:expr => [$($i:ident => $e:expr),+$(,)*]),+$(,)*) => {
        $($(
            #[test]
            fn $i() {
                match $f($e) {
                    IResult::Done(_, res) => {
                        assert_eq!(res, $r);
                    },
                    IResult::Incomplete(_) => panic!("Incomplete"),
                    IResult::Error(err) => panic!("Err {:?}", err),
                };
            }
        )*)*
    }
}

test_parser!(
    chunk_parameter_value,
    b"foo" => [
        test_chunk_val_1 => b"= foo ",
        test_chunk_val_2 => b"=foo\n",
        test_chunk_val_3 => b"=foo;",
        test_chunk_val_4 => b"=\"foo\";",
    ],
    b"wibble wobble" => [
        test_chunk_val_5 => b"=\"wibble wobble\";",
    ],
);


test_parser!(
    chunk_parameter,
    (&b"foo"[..], Option::Some(&b"bar"[..])) => [
        test_chunk_1 => b";foo = bar ",
        test_chunk_2 => b";foo=bar\n",
        test_chunk_3 => b";foo=bar;",
        test_chunk_4 => b";foo=\"bar\";",
    ],
    (&b"foo"[..], Option::None) => [
        test_chunk_no_val_1 => b";  foo \r\n",
        test_chunk_no_val_12=> b";foo\n",
        test_chunk_no_val_3 => b";foo;",
    ],
);

test_parser!(
    chunk_parser,
    ChunkHeader{parameters: vec![], size: 248} => [
        test_chunk_header_1 => b"F8\n",
        test_chunk_header_2 => b"F8 \r\n",
    ],
    ChunkHeader{parameters: vec![(b"name", Option::Some(b"val"))], size: 248} => [
        test_chunk_header_3 => b"F8;name=val\n",
        test_chunk_header_4 => b"F8 ; name = val \r\n",
    ],
);

test_parser!(
    request_line,
    RequestLine{method: b"GET", path: b"/test_url/", version: (b"1", b"0")} => [
        test_re_l_1 => b"GET /test_url/ HTTP/1.0\r\n",
        test_re_l_2 => b"GET /test_url/ HTTP/1.0\n",
        test_re_l_3 => b"GET  /test_url/ \t HTTP/1.0\t  \n",
    ]
);

test_parser!(
    header,
    (&b"Content-Length"[..], &b"52"[..]) => [
        test_header_1 => b"Content-Length: 52\r\nfoo...",
        test_header_2 => b"Content-Length: 52\nfoo...",
        test_header_3 => b"Content-Length   : 52\nfoo...",
        test_header_4 => b"Content-Length\t:52\nfoo...",
    ],
    (&b"Content-Length"[..], &b"test\r\n and\r\n another"[..]) => [
        test_header_obs => b"Content-Length: test\r\n and\r\n another\r\n\r\n",
    ],
);

#[test]
fn test_take_header() {
    assert_eq!(
        take_header_value(b"wibble\r\nabc"),
        IResult::Done(&b"\r\nabc"[..], &b"wibble"[..])
    );
}

#[test]
fn test_request_line_incomplete() {
    let incomplete = [
        &b"G"[..],
        &b"GET"[..],
        &b"GET /"[..],
        &b"GET /asad/ HTTP/1.1"[..],
    ];

    for i in incomplete.iter() {
        let res = request_line(i);
        match res {
            IResult::Incomplete(_) => {},
            d@ _ => panic!("Not incomplete: {:?}: {:?}", String::from_utf8_lossy(i), d)
        }
    }
}

#[test]
fn test_chunk_param_incomplete() {
    let incomplete = [
        &b"5;"[..],
        &b"5; fo"[..],
        &b"5; foo="[..],
    ];

    for i in incomplete.iter() {
        let res = chunk_parser(i);
        match res {
            IResult::Incomplete(_) => {},
            d@ _ => panic!("Not incomplete: {:?}: {:?}", String::from_utf8_lossy(i), d)
        }
    }
}
