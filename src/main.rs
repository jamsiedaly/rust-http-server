use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut buf: [u8; 128] = [0; 128];
                if let Ok(message_length) = stream.read(&mut buf) {
                    let request = String::from_utf8_lossy(&buf[..message_length]);

                    let message = parse_request(&request);

                    if message.path == "/" {
                        stream.write(b"HTTP/1.1 200 OK").unwrap();
                    } else {
                        stream.write(b"HTTP/1.1 404 NOT FOUND").unwrap();
                    }

                }

                println!("accepted new connection");
                stream.write(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}


#[derive(Debug)]
struct Message {
    method: String,
    headers: Vec<String>,
    path: String,
    http_version: String,
}

fn parse_request(request: &str) -> Message {
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
    headers.remove(headers.len() - 1);

    return Message {
        method,
        headers,
        path,
        http_version,
    };
}
