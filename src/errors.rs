
use std::fmt;
use std::error;
use std::convert;

#[derive(Debug)]
pub enum IntegerDecodeError { TooLong(usize), InvalidChar(u8) }

#[derive(Debug)]
pub enum HttpParserError {
    BadFirstLine,
    BadHeader,
    BadHeaderValue(HttpHeaderParseError),
    BadBodyChunkHeader,
}

#[derive(Debug)]
pub enum HttpHeaderParseError {
    ContentLength(IntegerDecodeError),
    UnrecognizedTransferEncoding,
}



impl convert::From<HttpHeaderParseError> for HttpParserError {
    fn from(e: HttpHeaderParseError) -> Self {
        HttpParserError::BadHeaderValue(e)
    }
}


impl error::Error for IntegerDecodeError {
    fn description(&self) -> &str {
        "failed to parse integer"
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl fmt::Display for IntegerDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IntegerDecodeError::TooLong(len) => write!(
                f,
                "Could not parse int: The supplied buffer was too long. Input size {} bytes",
                len,
            ),
            IntegerDecodeError::InvalidChar(chr) => write!(
                f,
                "Could not parse int: Buffer included invalid character '{:X}'.",
                chr,
            ),
        }
    }
}


impl error::Error for HttpHeaderParseError {
    fn description(&self) -> &str {
        "failed to parse header value"
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &HttpHeaderParseError::ContentLength(ref e) => Some(e),
            &HttpHeaderParseError::UnrecognizedTransferEncoding => None,
        }
    }
}

impl fmt::Display for HttpHeaderParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &HttpHeaderParseError::ContentLength(ref e) => write!(
                f, "HttpHeaderParseError: Failed to parse Content-Length: {}", e
            ),
            &HttpHeaderParseError::UnrecognizedTransferEncoding => write!(
                f, "HttpHeaderParseError: Unrecognized Transfer-Encoding.",
            ),
        }
    }
}



impl error::Error for HttpParserError {
    fn description(&self) -> &str {
        "failed to parse HTTP message"
    }

    fn cause(&self) -> Option<&error::Error> {
        match self {
            &HttpParserError::BadFirstLine => None,
            &HttpParserError::BadHeader => None,
            &HttpParserError::BadHeaderValue(ref err) => Some(err),
            &HttpParserError::BadBodyChunkHeader => None,
        }
    }
}

impl fmt::Display for HttpParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &HttpParserError::BadFirstLine => write!(
                f, "HttpParserError: Invalid first line."
            ),
            &HttpParserError::BadHeader => write!(
                f, "HttpParserError: Invalid header line."
            ),
            &HttpParserError::BadHeaderValue(ref err) => write!(
                f, "HttpParserError: {}", err
            ),
            &HttpParserError::BadBodyChunkHeader => write!(
                f, "HttpParserError: Invalid chunked header."
            ),
        }
    }
}
