#[macro_use]
extern crate log;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::str;
use std::fs;
use std::fs::File;
use std::borrow::Cow;
use std::ops::Deref;

mod logger;

type Map<T> = Option<HashMap<T, T>>;

const RESP404: &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";
const RESP520: &'static str = "HTTP/1.1 520 Unknown Error\r\n\r\n";
const RESP200: &'static str = "HTTP/1.1 200\n";

#[derive(Debug)]
enum Request {
    Undefined,
    Get,
    Post,
}

#[derive(Debug)]
struct Http<'a> {
    path: &'a str,
    headers: Map<&'a str>,
    body: &'a str,
}

impl<'a> Http<'a> {
    fn new() -> Http<'a> {
        Http {
            path: "",
            headers: None,
            body: "",
        }
    }
}

#[derive(Debug)]
enum HttpError {
    NotFound,
    Unknown,
}

fn main() {
    let _ = logger::init();
    let listener = TcpListener::bind("0.0.0.0:8765").unwrap();

    for l in listener.incoming() {
        match l {
            Ok(stream) => {
                info!("connection from {} to {}",
                      stream.peer_addr().unwrap().to_string(),
                      stream.local_addr().unwrap().to_string());

                thread::spawn(move || {
                    handle(stream);
                });
            }
            Err(e) => {
                warn!("{:?}", e.to_string());
            }
        }
    }
}

fn parse_type(s: &str) -> Request {
    match s {
        "GET" => Request::Get,
        "POST" => Request::Post,
        _ => Request::Undefined,
    }
}

fn parse_path(s: &str) -> &str {
    match s.split_whitespace().next() {
        Some(s) => s,
        None => "",
    }
}

// bad lifetimes :(
fn parse_headers_and_body<'a>(s: &'a str) -> (Map<&'a str>, &'a str) {
    let mut dict = HashMap::new();
    for line in s.lines() {
        let v: Vec<&str> = line.splitn(2, ": ").collect();
        if v.len() != 2 {
            // debug!("{:?}", v);
        } else {
            dict.insert(v[0], v[1]);
        }
    }
    // TODO [or not todo]: implement body parsing
    (Some(dict), "")
}

fn parse<'a>(buf: &'a [u8]) -> (Request, Http<'a>) {
    let mut req = Request::Undefined;
    let mut http = Http::new();
    if let Ok(s) = str::from_utf8(buf) {
        let a: Vec<&str> = s.splitn(2, ' ').collect();
        req = parse_type(a[0]);
        let a: Vec<&str> = a[1].splitn(2, '\n').collect();
        match req {
            Request::Get | Request::Post => {
                http.path = parse_path(a[0]);
                let (h, b) = parse_headers_and_body(a[1]);
                http.headers = h;
                http.body = b;
            }
            _ => {}
        }
    }
    (req, http)
}

fn handle_get<'a>(http: &Http) -> Result<Cow<'a, str>, HttpError> {
    let mut path: String = String::from("www");
    path.push_str(http.path);
    let mut meta = if let Ok(m) = fs::metadata(&path) {
        m
    } else {
        return Err(HttpError::NotFound);
    };

    if meta.is_dir() {
        let mut p: String = path.clone();
        p.push_str("/index.html");
        if let Ok(m) = fs::metadata(&p) {
            path = p;
            meta = m;
        }
    }

    return if meta.is_dir() {
        let mut s: String = String::new();

        if let Ok(mut iter) = fs::read_dir(path) {
            while let Some(Ok(en)) = iter.next() {
                if en.file_name().to_string_lossy().deref().as_bytes()[0] != '.' as u8 {
                    s.push_str(en.path().to_string_lossy().deref());
                    s.push('\n');
                }
            }
        }

        Ok(Cow::Owned(s))
    } else if meta.is_file() {
        let mut f = File::open(path).unwrap();
        let mut buf: String = String::new();
        let _ = f.read_to_string(&mut buf).unwrap();
        Ok(Cow::Owned(buf))
    } else {
        Err(HttpError::NotFound)
    };
}

fn handle_post<'a>(http: &Http) -> Result<Cow<'a, str>, HttpError> {
    if let Some(ref h) = http.headers {
        if let Some(&pass) = h.get("Auth") {
            // should be pretty secure
            if http.path == "/flag" && pass == "OylFIrcuIk8KN1sJCEADaDFd7fi4TmKz" {
                let mut f = File::open("the flag").unwrap();
                let mut s = String::new();
                let _ = f.read_to_string(&mut s).unwrap();
                return Ok(Cow::Owned(format!("{}", s)));
            }
        }
    }
    Err(HttpError::NotFound)
}

fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 1028];
    let _ = stream.read(&mut buf);
    let (request, http) = parse(&buf);
    let responce = match request {
        Request::Get => {
            info!("GET {}", http.path);
            handle_get(&http)
        }
        Request::Post => {
            info!("POST {}", http.path);
            handle_post(&http)
        }
        Request::Undefined => Err(HttpError::NotFound),
    };

    let (msg, header_len) = match responce {
        Ok(msg) => (msg, stream.write(RESP200.as_bytes())),
        Err(e) => {
            match e {
                HttpError::NotFound => {
                    (Cow::Borrowed("Not found"), stream.write(RESP404.as_bytes()))
                }
                _ => (Cow::Borrowed("Unknown"), stream.write(RESP520.as_bytes())),
            }
        }
    };

    if let (Ok(header_len), Ok(content_len), Ok(body_len)) =
           (header_len,
            stream.write(format!("Content-Length: {}\r\n\r\n", msg.len()).as_bytes()),
            stream.write(msg.deref().as_bytes())) {
        info!("Sent {} bytes", header_len + content_len + body_len);
    } else {
        warn!("Error in responcing");
    }
}
