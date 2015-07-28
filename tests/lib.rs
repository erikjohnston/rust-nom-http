extern crate nom_http;

use nom_http::*;
use std::collections::HashMap;
use std::str;


macro_rules! create_map {
    ( $( $n:expr => $v:expr ),* ) => {{
        let mut hash_map : HashMap<String, String> = ::std::collections::HashMap::new();
        $( hash_map.insert(String::from($n), String::from($v)); )*
        hash_map
    }}
}


macro_rules! create_request_test {
    ($( $expected:expr => {$($i:ident => $input_expr:expr),+$(,)*} ),+ $(,)* ) => {
        $($(
            #[test]
            fn $i() {
                // TODO: These should be split into multiple tests, but there's no nice way
                // to concat identifiers
                let input = $input_expr;
                {
                    let mut http_parser = HttpParser::new(ParserType::Request);
                    for _ in 0..3 {  // Tests to ensure we can run the parser multiple times
                        let mut cb = TestRequestHttpCallback::new();
                        http_parser.parse_request(
                            &mut cb,
                            input
                        ).unwrap();
                        // assert_eq!($expected, cb);
                        // method: String,
                        // path: String,
                        // version: (usize, usize),
                        // headers: HashMap<String, String>,
                        // chunks: String,
                        // finished: bool,

                        assert_eq!($expected, cb);
                    }
                }
                println!("Passed full buffer tests.");
                {
                    let mut http_parser = HttpParser::new(ParserType::Request);

                    for _ in 0..3 {  // Tests to ensure we can run the parser multiple times
                        let mut cb = TestRequestHttpCallback::new();
                        let mut curr_input = &input[..];
                        for _ in 1..input.len() + 1 {
                            println!("Input: {:?}", String::from_utf8_lossy(curr_input));
                            curr_input = http_parser.parse_request(
                                &mut cb,
                                curr_input,
                            ).unwrap();
                        }
                        assert_eq!($expected, cb);
                    }
                }
            }
        )*)*
    }
}

macro_rules! create_response_test {
    ($( $expected:expr => {$($i:ident => $input_expr:expr),+$(,)*} ),+ $(,)* ) => {
        $($(
            #[test]
            fn $i() {
                // TODO: These should be split into multiple tests, but there's no nice way
                // to concat identifiers
                let input = $input_expr;
                {
                    let mut http_parser = HttpParser::new(ParserType::Response);
                    for _ in 0..3 {  // Tests to ensure we can run the parser multiple times
                        let mut cb = TestResponseHttpCallback::new($expected.expect_body);
                        http_parser.parse_response(
                            &mut cb,
                            input
                        ).unwrap();
                        assert_eq!($expected, cb);
                    }
                }
                println!("Passed full buffer tests.");
                {
                    let mut http_parser = HttpParser::new(ParserType::Response);

                    for _ in 0..3 {  // Tests to ensure we can run the parser multiple times
                        let mut cb = TestResponseHttpCallback::new($expected.expect_body);
                        let mut curr_input = &input[..];
                        for _ in 1..input.len() + 1 {
                            println!("Input: {:?}", String::from_utf8_lossy(curr_input));
                            curr_input = http_parser.parse_response(
                                &mut cb,
                                curr_input,
                            ).unwrap();
                        }
                        assert_eq!($expected, cb);
                    }
                }
            }
        )*)*
    }
}

create_request_test!{
    TestRequestHttpCallback{
        method: "GET".to_owned(),
        path: "/test_url/".to_owned(),
        version: (1, 0),
        headers: create_map!{
            "Transfer-Encoding" => "chunked",
            "TestName" => "TestValue"
        },
        chunks: "Hello".to_owned(),
        finished: true,
    } => {
        chunked_trailing =>
b"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked

5
Hello
0
TestName: TestValue

",
        chunked_no_trailing =>
b"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked
TestName: TestValue

5
Hello
0

",
        chunked_multi_chunked =>
b"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked
TestName: TestValue

2
He
3
llo
0

",
        chunked_params =>
br#"GET /test_url/ HTTP/1.0
Transfer-Encoding: chunked
TestName: TestValue

5;foo=bar;another=param;no_val_param;q="Quoted Param"
Hello
0

"#,
    },
    TestRequestHttpCallback{
        method: "GET".to_owned(),
        path: "/joyent/http-parser/".to_owned(),
        version: (1, 1),
        headers: create_map!{
            "Host" => "github.com",
            "Connection" => "keep-alive",
            "Cache-Control" => "max-age=0",
            "Accept" => "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8",
            "User-Agent" => "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/44.0.2403.9 Safari/537.36",
            "Referer" => "https://github.com/joyent/http-parser",
            "Accept-Encoding" => "gzip, deflate, sdch",
            "Accept-Language" => "en-GB,en-US;q=0.8,en;q=0.6,nb;q=0.4"
        },
        chunks: "".to_owned(),
        finished: true,
    } => {
        github_request =>
br#"GET /joyent/http-parser/ HTTP/1.1
Host: github.com
Connection: keep-alive
Cache-Control: max-age=0
Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8
User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/44.0.2403.9 Safari/537.36
Referer: https://github.com/joyent/http-parser
Accept-Encoding: gzip, deflate, sdch
Accept-Language: en-GB,en-US;q=0.8,en;q=0.6,nb;q=0.4

"#,
    }
}

create_response_test!{
    TestResponseHttpCallback{
        version: (1, 0),
        code: 200,
        phrase: "OK".to_owned(),
        headers: create_map!{
            "Transfer-Encoding" => "chunked",
            "TestName" => "TestValue"
        },
        chunks: "Hello".to_owned(),
        finished: true,
        expect_body: ExpectBody::Maybe,
    } => {
        resp_chunked_trailing =>
b"HTTP/1.0 200 OK
Transfer-Encoding: chunked

5
Hello
0
TestName: TestValue

",
    },
    TestResponseHttpCallback{
        version: (1, 0),
        code: 200,
        phrase: "OK".to_owned(),
        headers: create_map!{
            "Content-Length" => "52",
            "TestName" => "TestValue"
        },
        chunks: "".to_owned(),
        finished: true,
        expect_body: ExpectBody::No,
    } => {
        resp_head =>
b"HTTP/1.0 200 OK
Content-Length: 52
TestName: TestValue

",
    }

}


#[test]
fn test_consumer() {
    let mut cb = TestRequestHttpCallback::new();
    let mut http_parser = HttpParser::new(ParserType::Request);
    http_parser.parse_request(
        &mut cb, b"GET /test_url/ HTTP/1.0\r\nContent-Length: 5\r\n\r\nHello"
    ).unwrap();

    assert_eq!("GET", cb.method);
    assert_eq!("/test_url/", &cb.path);
    assert_eq!((1,0), cb.version);


    assert_eq!(1, cb.headers.len());
    for (name, value) in &cb.headers {
        assert_eq!("Content-Length", name);
        assert_eq!("5", value);
    }

    assert_eq!("Hello", cb.chunks);
    assert!(cb.finished);
}


// END TESTS

#[derive(PartialEq,Eq,Debug)]
struct TestRequestHttpCallback {
    method: String,
    path: String,
    version: (usize, usize),
    headers: HashMap<String, String>,
    chunks: String,
    finished: bool,
}

impl TestRequestHttpCallback {
    fn new() -> TestRequestHttpCallback {
        TestRequestHttpCallback{
            method: String::new(),
            path: String::new(),
            version: (0,0),
            headers: HashMap::new(),
            chunks: String::new(),
            finished: false,
        }
    }
}

impl HttpRequestCallbacks for TestRequestHttpCallback {
    fn on_request_line(&mut self, _: &mut HttpParser, request: &RequestLine) {
        println!("on_request_line");
        self.method = String::from_utf8(request.method.to_owned()).unwrap();
        self.path = String::from_utf8(request.path.to_owned()).unwrap();
        self.version = (
            util::dec_buf_to_int(request.version.0).unwrap(),
            util::dec_buf_to_int(request.version.1).unwrap(),
        );
    }

}

impl HttpMessageCallbacks for TestRequestHttpCallback {
    fn on_header(&mut self, _: &mut HttpParser, name: &[u8], value: &[u8]) {
        println!(
            "on_header name: {:?}, value: {:?}",
            String::from_utf8_lossy(name),
            String::from_utf8_lossy(value),
        );
        self.headers.insert(String::from_utf8(name.to_owned()).unwrap(), String::from_utf8(value.to_owned()).unwrap());
    }
    fn on_headers_finished(&mut self, _: &mut HttpParser, body_type: BodyType) -> ExpectBody {
        println!("on_headers_finished");
        println!("BodyType: {:?}", body_type);
        ExpectBody::Maybe
    }
    fn on_chunk(&mut self, _: &mut HttpParser, data: &[u8]) {
        println!("on_chunk");

        self.chunks.push_str(str::from_utf8(data).unwrap());
    }
    fn on_end(&mut self, _: &mut HttpParser) {
        println!("on_end");
        self.finished = true;
    }
}

#[derive(PartialEq,Eq,Debug)]
struct TestResponseHttpCallback {
    version: (usize, usize),
    code: u16,
    phrase: String,
    headers: HashMap<String, String>,
    chunks: String,
    finished: bool,
    expect_body: ExpectBody,
}

impl TestResponseHttpCallback {
    fn new(expect_body: ExpectBody) -> TestResponseHttpCallback {
        TestResponseHttpCallback{
            version: (0,0),
            code: 0,
            phrase: String::new(),
            headers: HashMap::new(),
            chunks: String::new(),
            finished: false,
            expect_body: expect_body,
        }
    }
}

impl HttpResponseCallbacks for TestResponseHttpCallback {
    fn on_response_line(&mut self, _: &mut HttpParser, response: &ResponseLine) {
        println!("on_response_line");
        self.version = (
            util::dec_buf_to_int(response.version.0).unwrap(),
            util::dec_buf_to_int(response.version.1).unwrap(),
        );
        self.code = response.code;
        self.phrase = String::from_utf8(response.phrase.to_owned()).unwrap();
    }

}

impl HttpMessageCallbacks for TestResponseHttpCallback {
    fn on_header(&mut self, _: &mut HttpParser, name: &[u8], value: &[u8]) {
        println!(
            "on_header name: {:?}, value: {:?}",
            String::from_utf8_lossy(name),
            String::from_utf8_lossy(value),
        );
        self.headers.insert(String::from_utf8(name.to_owned()).unwrap(), String::from_utf8(value.to_owned()).unwrap());
    }
    fn on_headers_finished(&mut self, _: &mut HttpParser, body_type: BodyType) -> ExpectBody {
        println!("on_headers_finished");
        println!("BodyType: {:?}", body_type);
        self.expect_body
    }
    fn on_chunk(&mut self, _: &mut HttpParser, data: &[u8]) {
        println!("on_chunk");

        self.chunks.push_str(str::from_utf8(data).unwrap());
    }
    fn on_end(&mut self, _: &mut HttpParser) {
        println!("on_end");
        self.finished = true;
    }
}
