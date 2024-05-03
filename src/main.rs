use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

fn return_empty_200(stream: &mut TcpStream) {
    let status_line = b"HTTP/1.1 200 OK";
    let headers = b""; //b"Content-Length: 0\r\nConnection: close\r\n";
    let message_body = b"";

    let buf: [&[u8]; 5] = [status_line, b"\r\n", headers, b"\r\n", message_body];
    let buf = buf.concat();

    if let Err(err) = stream.write_all(buf.as_slice()) {
        eprintln!("return_empty_200 failed due to {err}");
    }

    if let Err(err) = stream.flush() {
        eprintln!("TCP stream flush failed in return_empty_200: {err}");
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
                return_empty_200(&mut _stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}
