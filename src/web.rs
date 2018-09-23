extern crate url;

use bot;
use bot::{BotGlobals, BotRequest};
use std::collections::HashMap;
use std::io::*;
use std::net::{Shutdown, TcpListener, TcpStream};

const HEADER_CONTENT_LENGTH: &'static str = "Content-Length: ";

pub fn start(host: String, port: String, globals: &mut BotGlobals) {
    let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();

    // handle tcp socket
    for stream in listener.incoming() {
        if stream.is_err() {
            continue;
        }
        let mut stream = stream.unwrap();

        // read request
        let req = read_http_request(&mut stream);
        if req.is_err() {
            close_stream(&mut stream);
            continue;
        }
        let req = req.unwrap();

        // build response
        let resp = handle_http_request(&req, globals);
        if resp.is_err() {
            close_stream(&mut stream);
            continue;
        }
        let resp = resp.unwrap();

        // send response & close socket
        write_http_response(&mut stream, &resp);
    }
}

struct HttpRequest {
    params: HashMap<String, String>,
}

struct HttpResponse {
    status: u16,
    body: String,
}

impl HttpResponse {
    fn new(body: String) -> Result<HttpResponse> { Ok(HttpResponse { status: 200, body }) }
    fn empty() -> Result<HttpResponse> { Ok(HttpResponse { status: 200, body: String::new() }) }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest> {
    let body = read_http_body(stream)?;

    let req = url::form_urlencoded::parse(body.as_bytes());
    let mut params = HashMap::new();
    for item in req {
        params.insert(String::from(item.0), String::from(item.1));
    }
    return Ok(HttpRequest { params });
}

fn write_http_response(stream: &mut TcpStream, resp: &HttpResponse) {
    let body = format!("HTTP/1.1 {}\r\n\r\n{}", resp.status, resp.body);

    stream.write_all(body.as_bytes()).unwrap_or(());
    stream.flush().unwrap_or(());

    close_stream(stream);
}

fn close_stream(stream: &mut TcpStream) {
    stream.shutdown(Shutdown::Both).unwrap_or(());
}

fn handle_http_request(http_req: &HttpRequest, globals: &mut BotGlobals) -> Result<HttpResponse> {
    // build bot request
    let event = http_req.params.get("Event");
    if event.is_none() {
        return HttpResponse::empty();
    }
    let event = event.unwrap().as_str();
    let bot_req: Option<BotRequest> = match event {
        "KeepAlive" | "StatusChanged" => None,
        "ReceiveNormalIM" | "ReceiveClusterIM" => {
            Some(BotRequest::new(&http_req.params, globals))
        }
        _ => {
            println!("Unknown event: {}, msg: {:?}", event, http_req.params);
            return HttpResponse::empty();
        }
    };
    if bot_req.is_none() {
        return HttpResponse::empty();
    }
    let mut bot_req = bot_req.unwrap();

    // handle
    let bot_resps = bot::process_request(&mut bot_req, globals);

    // build http response
    let mut body = String::new();
    for bot_resp in bot_resps {
        body.push_str(format!("<&&>{:?}<&>{}<&>{}\r\n", bot_resp.resp_type, bot_resp.target_id, bot_resp.text).as_str());
    }

    return HttpResponse::new(body);
}

fn read_http_body(stream: &mut TcpStream) -> Result<String> {
    let mut line = String::new();
    let mut reader = BufReader::new(stream);
    let mut content_length = 0usize;

    loop {
        reader.read_line(&mut line)?;

        if line == "\r\n" {
            break;
        }

        if line.starts_with(HEADER_CONTENT_LENGTH) {
            let xxx = &line[HEADER_CONTENT_LENGTH.len()..line.len() - 2];
            content_length = xxx.parse::<usize>().unwrap();
        }

        line.clear();
    }

    let mut body_bytes = vec!(0u8; content_length);
    reader.read_exact(&mut body_bytes[..])?;
    let body = String::from_utf8(body_bytes);
    return match body {
        Ok(t) => Ok(t),
        Err(t) => Err(Error::new(ErrorKind::InvalidData, t.to_string()))
    };
}
