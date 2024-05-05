use std::str::FromStr;
use tokio::io::Result as TIOResult;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

mod types;

use types::{HttpMethod, HttpRequest, HttpResponse};

async fn return_http_response(stream: &mut TcpStream, resp: &HttpResponse) {
    if let Err(err) = stream.write_all(resp.to_byte_string().as_slice()).await {
        eprintln!("return_empty_http_response failed due to {err}");
        return;
    }

    if let Err(err) = stream.flush().await {
        eprintln!("TCP stream flush failed in return_empty_http_response: {err}");
    }
}

async fn return_empty_http_response(stream: &mut TcpStream, status_code: u16, phrase: Vec<u8>) {
    let resp = HttpResponse {
        status_code,
        phrase,
        headers: vec![b"Content-Length: 0".to_vec(), b"Connection: close".to_vec()],
        body: vec![],
    };
    
    return_http_response(stream, &resp).await
}

async fn return_path_http_response(stream: &mut TcpStream, path: &mut String) {
    if !path.starts_with("/echo/") {
        return_empty_http_response(stream, 400, b"Bad Request".to_vec()).await;
        return;
    }

    let path = path.split_off(6);
    let len = path.len();

    let resp = HttpResponse {
        status_code: 200,
        phrase: b"OK".to_vec(),
        headers: vec![
            [b"Content-Length: ", len.to_string().as_bytes()].concat(),
            b"Connection: close".to_vec(),
            b"Content-Type: text/plain".to_vec(),
        ],
        body: path.as_bytes().to_vec(),
    };

    return_http_response(stream, &resp).await
}

async fn return_user_agent_response(stream: &mut TcpStream, req: &HttpRequest) {
    let user_agent = req
        .headers
        .get("User-Agent")
        .cloned()
        .unwrap_or_else(String::new);
    let resp = HttpResponse {
        status_code: 200,
        phrase: b"OK".to_vec(),
        headers: vec![
            [b"Content-Length: ", user_agent.len().to_string().as_bytes()].concat(),
            b"Connection: close".to_vec(),
            b"Content-Type: text/plain".to_vec(),
        ],
        body: user_agent.into_bytes().to_vec(),
    };

    return_http_response(stream, &resp).await
}

async fn respond_via_http(stream: &mut TcpStream) {
    let mut buf: [u8; 1024] = [0; 1024];
    if let Err(err) = stream.read(&mut buf).await {
        eprintln!("Error when reading string from TCP stream: {err}");
        return;
    }

    let buf = String::from_utf8_lossy(&buf);
    let request = HttpRequest::from_str(&buf);
    if let Err(err) = request {
        eprintln!("err: {err}");
        eprintln!("Expected to successfully parse request into a HttpRequest struct");
        return_empty_http_response(stream, 400, b"Bad Request".to_vec()).await;
        return;
    }
    let request = request.unwrap();

    match request.path.as_str() {
        "/" => return_empty_http_response(stream, 200, b"OK".to_vec()).await,
        "/user-agent" if request.method == HttpMethod::Get => {
            return_user_agent_response(stream, &request).await
        }
        _ if request.path.starts_with("/echo/") => {
            return_path_http_response(stream, &mut request.path.clone()).await
        }
        _ => return_empty_http_response(stream, 404, b"Not Found".to_vec()).await,
    }
}

#[tokio::main]
async fn main() -> TIOResult<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    println!("TcpListener has bound 4221.");

    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                println!("Accepted new connection");
                tokio::spawn(async move { 
                    respond_via_http(&mut stream).await
                });
            },
            Err(e) => {
                println!("Failed to accept connection: {}", e);
            }
        }
    }
}
