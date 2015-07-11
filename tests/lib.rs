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


macro_rules! create_test {
    ($( $expected:expr => {$($i:ident => $input_expr:expr),+$(,)*} ),+ $(,)* ) => {
        $($(
            #[test]
            fn $i() {
                // TODO: These should be split into multiple tests, but there's no nice way
                // to concat identifiers
                let input = $input_expr;
                {
                    let mut cb = TestHttpCallback::new();
                    let mut http_parser = HttpParser::new();
                    http_parser.parse_http(
                        &mut cb,
                        input
                    ).unwrap();
                    assert_eq!($expected, cb);
                }
                {
                    let mut cb = TestHttpCallback::new();
                    let mut http_parser = HttpParser::new();

                    let mut start = 0;
                    for i in 1..input.len() + 1 {
                        println!("Input: {:?}", String::from_utf8_lossy(&input[start..i]));
                        start += http_parser.parse_http(
                            &mut cb,
                            &input[start..i],
                        ).unwrap();
                    }
                    assert_eq!($expected, cb);
                }
            }
        )*)*
    }
}

create_test!{
    TestHttpCallback{
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
    }
}


#[test]
fn test_consumer() {
    let mut cb = TestHttpCallback::new();
    let mut http_parser = HttpParser::new();
    http_parser.parse_http(
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
struct TestHttpCallback {
    method: String,
    path: String,
    version: (usize, usize),
    headers: HashMap<String, String>,
    chunks: String,
    finished: bool,
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
        println!("on_request_line");
        self.method = String::from_utf8(request.method.to_owned()).unwrap();
        self.path = String::from_utf8(request.path.to_owned()).unwrap();
        self.version = (
            util::dec_buf_to_int(request.version.0).unwrap(),
            util::dec_buf_to_int(request.version.1).unwrap(),
        );
    }
    fn on_header(&mut self, name: &[u8], value: &[u8]) {
        println!(
            "on_header name: {:?}, value: {:?}",
            String::from_utf8_lossy(name),
            String::from_utf8_lossy(value),
        );
        self.headers.insert(String::from_utf8(name.to_owned()).unwrap(), String::from_utf8(value.to_owned()).unwrap());
    }
    fn on_message_begin(&mut self, body_type: BodyType) {
        println!("on_message_begin");
        println!("BodyType: {:?}", body_type);
    }
    fn on_chunk(&mut self, data: &[u8]) {
        println!("on_chunk");

        self.chunks.push_str(str::from_utf8(data).unwrap());
    }
    fn on_end(&mut self) {
        println!("on_end");
        self.finished = true;
    }
}
