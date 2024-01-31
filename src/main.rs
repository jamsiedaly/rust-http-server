extern crate core;

use clap::Parser;
use std::fmt::Display;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    directory: Option<String>,
}

fn main() {
    let args = Args::parse();

    let directory = match args.directory {
        None => {
            Arc::new("files".to_owned())
        }
        Some(dir) => {
            Arc::new(dir)
        }
    };

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let directory = directory.clone();
                std::thread::spawn(move || {
                    let mut buf: [u8; 256] = [0; 256];
                    if let Ok(message_length) = stream.read(&mut buf) {
                        let message = parse_request(&buf, message_length);

                        if message.path == "/" {
                            stream.write(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
                        } else if message.path.starts_with("/echo/") {
                            let response = echo_response(&message);
                            stream.write(response.to_string().as_bytes()).unwrap();
                        } else if message.path.starts_with("/user-agent") {
                            let response = user_agent_request(message);
                            stream.write(response.to_string().as_bytes()).unwrap();
                        } else if message.path.starts_with("/files/") {
                            let response: Response = if message.method == "GET" {
                                get_file_response(message, &directory)
                            } else if message.method == "POST" {
                                let unsanitized_filename = message.path.replace("/files/", "");
                                match File::create(format!("{}/{}", directory, unsanitized_filename)) {
                                    Ok(mut file) => {
                                        match file.write_all(message.content.as_bytes()) {
                                            Ok(_) => {
                                                Response {
                                                    status_code: 201,
                                                    headers: vec!["Content-Type: text/plain".to_owned(), "Content-Length: 0".to_owned()],
                                                    body: "".to_owned(),
                                                }
                                            }
                                            Err(_) => {
                                                Response {
                                                    status_code: 500,
                                                    headers: vec!["Content-Type: text/plain".to_owned(), "Content-Length: 0".to_owned()],
                                                    body: "".to_owned(),
                                                }
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        Response {
                                            status_code: 500,
                                            headers: vec!["Content-Type: text/plain".to_owned(), "Content-Length: 0".to_owned()],
                                            body: "".to_owned(),
                                        }
                                    }
                                }
                            } else {
                                Response {
                                    status_code: 405,
                                    headers: vec!["Content-Type: text/plain".to_owned(), "Content-Length: 0".to_owned()],
                                    body: "".to_owned(),
                                }
                            };
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

fn get_file_response(request: Request, directory: &str) -> Response {
    let unsanitized_filename = request.path.replace("/files/", "");
    let filename = &sanitize_filename::sanitize(unsanitized_filename);
    let file_location = format!("{}/{}", directory, filename);
    let path = Path::new(&file_location);

    let response = if Path::exists(path) {
        let file = fs::read_to_string(path).unwrap();
        let content_length = format!("Content-Length: {}", file.len());
        Response {
            status_code: 200,
            headers: vec![
                "Content-Type: application/octet-stream".to_owned(),
                content_length,
            ],
            body: file,
        }
    } else {
        Response {
            status_code: 404,
            headers: vec!["Content-Type: text/plain".to_owned(), "Content-Length: 0".to_owned()],
            body: "".to_owned(),
        }
    };

    return response;
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
    content: String,
}

fn parse_request(request: &[u8; 256], message_length: usize) -> Request {
    let request = &request[0..message_length];

    let header_section = String::from_utf8_lossy(request);

    let mut lines = header_section.lines();
    let first_line = lines.next().unwrap();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap().to_owned();
    let path = parts.next().unwrap().to_owned();
    let http_version = parts.next().unwrap().to_owned();

    let mut headers = Vec::new();

    let mut parsing_headers = true;
    let mut content_length: Option<usize> = None;
    let mut content = String::new();
    for line in lines {
        if parsing_headers {
            if line.is_empty() {
                parsing_headers = false;
            } else {
                if line.starts_with("Content-Length") {
                    content_length = line.split(":")
                        .collect::<Vec<&str>>()[1]
                        .trim()
                        .parse::<usize>()
                        .ok();
                }
                headers.push(line.to_owned());
            }
        } else if content_length.is_some() {
            content.push_str(line);
        }
    }

    return Request {
        method,
        headers,
        path,
        http_version,
        content: content.to_owned(),
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

impl Into<Vec<u8>> for Response {
    fn into(self) -> Vec<u8> {
        return self.to_string().into_bytes();
    }
}