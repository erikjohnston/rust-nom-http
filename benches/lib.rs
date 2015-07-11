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


macro_rules! benchmark_strings {
    ($($name:ident: $s:expr),+$(,)*) => {
        $(
            #[bench]
            fn $name(b: &mut test::Bencher) {
                let mut cb = TestHttpCallback::new();
                let mut http_parser = HttpParser::new();
                b.iter(
                    || http_parser.parse_http(&mut cb, $s)
                );
            }
        )*
    }
}

benchmark_strings! {
    bench_length: TEST_REQUEST_LENGTH,
    bench_chunked: TEST_REQUEST_CHUNKED,
    bench_chunked_params: TEST_REQUEST_CHUNKED_PARAMS
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

impl HttpCallbacks for TestHttpCallback {
    fn on_request_line(&mut self, request: &RequestLine) {
        // self.method = String::from_utf8(request.method.to_owned()).unwrap();
        // self.path = String::from_utf8(request.path.to_owned()).unwrap();
        // self.version = (
        //     util::dec_buf_to_int(request.version.0).unwrap(),
        //     util::dec_buf_to_int(request.version.1).unwrap(),
        // );
    }

    fn on_header(&mut self, name: &[u8], value: &[u8]) {
        // self.headers.insert(String::from_utf8(name.to_owned()).unwrap(), String::from_utf8(value.to_owned()).unwrap());
    }

    fn on_message_begin(&mut self, body_type: BodyType) {}

    fn on_chunk(&mut self, data: &[u8]) {
        // self.chunks.push_str(str::from_utf8(data).unwrap());
    }

    fn on_end(&mut self) {
        self.finished = true;
    }
}
