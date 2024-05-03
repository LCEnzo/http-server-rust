use std::io::Read;
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

fn return_empty_http_response(stream: &mut TcpStream, code: u32, phrase: &[u8]) {
    let status_line = [b"HTTP/1.1 ", code.to_string().as_bytes(), b" ", phrase].concat();
    let headers = b"Content-Length: 0\r\nConnection: close\r\n";
    let message_body = b"";

    let buf: [&[u8]; 5] = [
        status_line.as_slice(),
        b"\r\n",
        headers,
        b"\r\n",
        message_body,
    ];
    let buf = buf.concat();

    if let Err(err) = stream.write_all(buf.as_slice()) {
        eprintln!("return_empty_404 failed due to {err}");
        return;
    }

    if let Err(err) = stream.flush() {
        eprintln!("TCP stream flush failed in return_empty_404: {err}");
    }
}

fn respond_via_http(stream: &mut TcpStream) {
    let mut buf: [u8; 1024] = [0; 1024];
    if let Err(err) = stream.read(&mut buf) {
        eprintln!("Error when reading string from TCP stream: {err}");
        return;
    }

    let buf = String::from_utf8_lossy(&buf);

    let path = buf
        .lines()
        .next()
        .and_then(|start_line| start_line.split_whitespace().nth(1))
        .unwrap_or_else(|| {
            eprintln!("Unable to parse path from TCP stream, defaulting to /");
            "/"
        });

    match path.trim() {
        "/" => return_empty_http_response(stream, 200, b"OK"),
        _ => return_empty_http_response(stream, 404, b"Not Found"),
    }
}

fn main() {
    println!("Test print for logging.");

    let listener = TcpListener::bind("127.0.0.1:4221")
        .expect("Expected to be able to bind TCP listener to port 4221");

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("Accepted new connection");
                respond_via_http(&mut _stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
