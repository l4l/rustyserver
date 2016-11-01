#[macro_use]
extern crate log;

use std::io::{Read, Write, Result};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::collections::HashMap;
use std::str;
use std::fs::File;
use std::borrow::Cow;
use std::ops::Deref;

mod logger;

type Map<T> = Option<HashMap<T, T>>;

const ERR404: &'static str = "HTTP 404 Not Found";

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
            info!("{:?}", v);
        } else {
            dict.insert(v[0], v[1]);
        }
    }
    // TODO: implement body parsing
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

fn handle_get<'a>(http: &Http) -> Cow<'a, str> {
    // TODO: get request handling
    Cow::Borrowed(ERR404)
}

fn handle_post<'a>(http: &Http) -> Cow<'a, str> {
    if let Some(ref h) = http.headers {
        if let Some(&pass) = h.get("Auth") {
            if http.path == "/flag" && pass == "OylFIrcuIk8KN1sJCEADaDFd7fi4TmKz" {
                let mut f = File::open("the flag").unwrap();
                let mut s = String::new();
                let _ = f.read_to_string(&mut s).unwrap();
                return Cow::Owned(format!("{}", s));
            }
        }
    }
    Cow::Borrowed(ERR404)
}

fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 1028];
    let _ = stream.read(&mut buf);
    let (request, http) = parse(&buf);
    match request {
        Request::Get => {
            info!("GET {}", http.path);
            let _ = stream.write(handle_get(&http).deref().as_bytes());
        }
        Request::Post => {
            info!("POST {}", http.path);
            let _ = stream.write(handle_post(&http).deref().as_bytes());
        }
        Request::Undefined => {
            let _ = stream.write(ERR404.as_bytes());
            error!("Malformed request: {}", String::from_utf8_lossy(&buf));
        }
    }
}
