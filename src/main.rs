use std::fmt::Display;
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                std::thread::spawn(move || {
                    let mut buf: [u8; 128] = [0; 128];
                    if let Ok(message_length) = stream.read(&mut buf) {
                        let request = String::from_utf8_lossy(&buf[..message_length]);

                        let message = parse_request(&request);

                        if message.path == "/" {
                            stream.write(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
                        } else if message.path.starts_with("/echo/") {
                            let response = echo_response(&message);
                            stream.write(response.to_string().as_bytes()).unwrap();
                        } else if message.path.starts_with("/user-agent") {
                            let response = user_agent_request(message);
                            stream.write(response.to_string().as_bytes()).unwrap();
                        } else {
                            stream.write(b"HTTP/1.1 404 NOT FOUND\r\n\r\n").unwrap();
                        }
                    }
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn echo_response(message: &Request) -> Response {
    let echo_message = message.path.replace("/echo/", "");

    let response = Response {
        status_code: 200,
        headers: vec!["Content-Type: text/plain".to_owned(), format!("Content-Length: {}", echo_message.len())],
        body: echo_message,
    };
    response
}

fn user_agent_request(message: Request) -> Response {
    let user_agent = message.headers.iter().find(
        |header| header.starts_with("User-Agent")
    ).unwrap().split(":").collect::<Vec<&str>>()[1].trim().to_owned();

    let response = Response {
        status_code: 200,
        headers: vec!["Content-Type: text/plain".to_owned(), format!("Content-Length: {}", user_agent.len())],
        body: user_agent,
    };
    response
}


#[derive(Debug)]
#[allow(dead_code)]
struct Request {
    method: String,
    headers: Vec<String>,
    path: String,
    http_version: String,
}

fn parse_request(request: &str) -> Request {
    let mut lines = request.lines();
    let first_line = lines.next().unwrap();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap().to_owned();
    let path = parts.next().unwrap().to_owned();
    let http_version = parts.next().unwrap().to_owned();

    let mut headers = Vec::new();
    lines.into_iter().for_each(|line| {
        if !line.is_empty() {
            headers.push(line.to_string());
        }
    });
    if headers.len() > 1 {
        headers.remove(headers.len() - 1);
    }

    return Request {
        method,
        headers,
        path,
        http_version,
    };
}

#[derive(Debug)]
struct Response {
    status_code: u16,
    headers: Vec<String>,
    body: String,
}

impl Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut response = format!("HTTP/1.1 {}\r\n", self.status_code);
        self.headers.iter().for_each(|header| {
            response.push_str(&format!("{}\r\n", header));
        });
        response.push_str("\r\n");
        response.push_str(&self.body);
        return write!(f, "{}", response);
    }
}