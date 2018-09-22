extern crate url;

use bot;
use bot::{BotGlobals, BotRequest};
use std::{thread, time};
use std::collections::HashMap;
use std::io::*;
use std::net::{Shutdown, TcpListener, TcpStream};

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
    // 正常应该读出 header 之后取其中的 Content-Length 判断报文总长度再循环读出来，太麻烦了我不想写，先sleep一段时间等字节都进到网络缓冲区再读出来
    thread::sleep(time::Duration::from_millis(200));

    // 懒得管那些 http 协议了，直接把 post 的报文体抓出来，出错就错了反正也无所谓
    let mut buf = [0; 2048];
    let size = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[0..size]);
    let pos = req.find("\r\n\r\n").unwrap_or(req.len() - 4);

    let req = String::from(&req[pos + 4..]);
    let req = url::form_urlencoded::parse(req.as_bytes());

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
            println!("msg: {:?}", http_req.params);
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
    println!("{:?}", bot_req);

    // handle
    let bot_resps = bot::process_request(&mut bot_req, globals);
    println!("{:?}", bot_resps);

    // build http response
    let mut body = String::new();
    for bot_resp in bot_resps {
        body.push_str(format!("<&&>{:?}<&>{}<&>{}\r\n", bot_resp.resp_type, bot_resp.target_id, bot_resp.text).as_str());
    }
    println!("{}", body);

    return HttpResponse::new(body);
}
