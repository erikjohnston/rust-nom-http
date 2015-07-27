#![feature(test)]

extern crate nom_http;
extern crate test;

use nom_http::*;
use std::collections::HashMap;
use std::str;


const TEST_REQUEST_CHUNKED : &'static [u8] =
b"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked
TestName: TestValue

2
He
3
llo
0

";

const TEST_REQUEST_LENGTH : &'static [u8] =
b"GET /test_url/ HTTP/1.0
Content-Length: 5

Hello";


const TEST_REQUEST_CHUNKED_PARAMS : &'static [u8] =
br#"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked
TestName: TestValue

5;foo=bar;another=param;no_val_param;q="Quoted Param"
Hello
0

"#;

const TEST_REQUEST_GITHUB_GET : &'static [u8] =
br#"GET /joyent/http-parser/ HTTP/1.1
Host: github.com
Connection: keep-alive
Cache-Control: max-age=0
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8
User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/44.0.2403.9 Safari/537.36
Referer: https://github.com/joyent/http-parser
Accept-Encoding: gzip, deflate, sdch
Accept-Language: en-GB,en-US;q=0.8,en;q=0.6,nb;q=0.4

"#;

// Shamelessly taken from: https://github.com/joyent/http-parser/blob/master/bench.c
const TEST_REQUEST_JOYENT_HTTP_PARSER_BENCH : &'static [u8] =
b"POST /joyent/http-parser HTTP/1.1\r\n\
Host: github.com\r\n\
DNT: 1\r\n\
Accept-Encoding: gzip, deflate, sdch\r\n\
Accept-Language: ru-RU,ru;q=0.8,en-US;q=0.6,en;q=0.4\r\n\
User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_10_1) \
    AppleWebKit/537.36 (KHTML, like Gecko) \
    Chrome/39.0.2171.65 Safari/537.36\r\n\
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,\
    image/webp,*/*;q=0.8\r\n\
Referer: https://github.com/joyent/http-parser\r\n\
Connection: keep-alive\r\n\
Transfer-Encoding: chunked\r\n\
Cache-Control: max-age=0\r\n\r\nb\r\nhello world\r\n0\r\n\r\n";


macro_rules! benchmark_strings {
    ($($name:ident: $s:expr),+$(,)*) => {
        $(
            #[bench]
            fn $name(b: &mut test::Bencher) {
                let mut cb = TestHttpCallback::new();
                let mut http_parser = HttpParser::new(ParserType::Request);
                b.iter(
                    || http_parser.parse_request(&mut cb, $s)
                );
            }
        )*
    }
}

benchmark_strings! {
    bench_length: TEST_REQUEST_LENGTH,
    bench_chunked: TEST_REQUEST_CHUNKED,
    bench_chunked_params: TEST_REQUEST_CHUNKED_PARAMS,
    bench_github_get: TEST_REQUEST_GITHUB_GET,
    joyent_http_parser: TEST_REQUEST_JOYENT_HTTP_PARSER_BENCH,
}


// END TESTS


#[derive(PartialEq,Eq,Debug)]
pub struct TestHttpCallback {
    pub method: String,
    pub path: String,
    pub version: (usize, usize),
    pub headers: HashMap<String, String>,
    pub chunks: String,
    pub finished: bool,
}

impl TestHttpCallback {
    fn new() -> TestHttpCallback {
        TestHttpCallback{
            method: String::new(),
            path: String::new(),
            version: (0,0),
            headers: HashMap::new(),
            chunks: String::new(),
            finished: false,
        }
    }
}

impl HttpRequestCallbacks for TestHttpCallback {
    fn on_request_line(&mut self, parser: &mut HttpParser, request: &RequestLine) {
        // self.method = String::from_utf8(request.method.to_owned()).unwrap();
        // self.path = String::from_utf8(request.path.to_owned()).unwrap();
        // self.version = (
        //     util::dec_buf_to_int(request.version.0).unwrap(),
        //     util::dec_buf_to_int(request.version.1).unwrap(),
        // );
    }
}

impl HttpMessageCallbacks for TestHttpCallback {
    fn on_header(&mut self, parser: &mut HttpParser, name: &[u8], value: &[u8]) {
        // self.headers.insert(String::from_utf8(name.to_owned()).unwrap(), String::from_utf8(value.to_owned()).unwrap());
    }

    fn on_message_begin(&mut self, parser: &mut HttpParser, body_type: BodyType) {}

    fn on_chunk(&mut self, parser: &mut HttpParser, data: &[u8]) {
        // self.chunks.push_str(str::from_utf8(data).unwrap());
    }

    fn on_end(&mut self, parser: &mut HttpParser) {
        self.finished = true;
    }
}
