use std::env;
use std::io::ErrorKind;
use std::str::FromStr;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::Result as TIOResult;
use tokio::net::{TcpListener, TcpStream};

mod types;

use types::{HttpMethod, HttpRequest, HttpResponse};

static mut DIRECTORY: String = String::new();

async fn return_http_response(stream: &mut TcpStream, resp: &HttpResponse) {
    if let Err(err) = stream.write_all(resp.to_byte_string().as_slice()).await {
        eprintln!("return_empty_http_response failed due to {err}");
        return;
    }

    if let Err(err) = stream.flush().await {
        eprintln!("TCP stream flush failed in return_empty_http_response: {err}");
    }
}

async fn return_http_response_with_text(
    stream: &mut TcpStream,
    status_code: u16,
    phrase: Vec<u8>,
    msg: Vec<u8>,
) {
    let headers = vec![
        [b"Content-Length: ", msg.len().to_string().as_bytes()].concat(),
        b"Connection: close".to_vec(),
        b"Content-Type: text/plain".to_vec(),
    ];
    let resp = HttpResponse {
        status_code,
        phrase,
        headers,
        body: msg,
    };
    return_http_response(stream, &resp).await
}

async fn return_http_response_with_data(
    stream: &mut TcpStream,
    status_code: u16,
    phrase: Vec<u8>,
    msg: Vec<u8>,
) {
    let headers = vec![
        [b"Content-Length: ", msg.len().to_string().as_bytes()].concat(),
        b"Connection: close".to_vec(),
        b"Content-Type: application/octet-stream".to_vec(),
    ];
    let resp = HttpResponse {
        status_code,
        phrase,
        headers,
        body: msg,
    };
    return_http_response(stream, &resp).await
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

async fn return_path_http_response(stream: &mut TcpStream, path: String) {
    if !path.starts_with("/echo/") {
        return_empty_http_response(stream, 400, b"Bad Request".to_vec()).await;
        return;
    }

    let path = path.clone().split_off(6);
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

    return_http_response_with_text(stream, 200, b"OK".to_vec(), user_agent.as_bytes().to_vec())
        .await
}

async fn get_file(stream: &mut TcpStream, req: &HttpRequest) {
    if !req.path.starts_with("/files/") {
        return_empty_http_response(stream, 400, b"Bad Request".to_vec()).await;
        return;
    }

    let prefix = unsafe { DIRECTORY.clone() };
    let path = format!("{}/{}", prefix, req.path.clone().split_off(7));

    let read = fs::read(path.clone()).await;
    match read {
        Ok(content) => return_http_response_with_data(stream, 200, b"OK".to_vec(), content).await,
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return_http_response_with_text(
                    stream,
                    404,
                    b"Not Found".to_vec(),
                    b"File not found".to_vec(),
                )
                .await;
                return;
            }

            let msg = format!(
                "Encountered error while trying to read file ({}): {}",
                path, err
            );
            eprintln!("{}", msg);
            return_http_response_with_text(
                stream,
                500,
                b"Internal Server Error".to_vec(),
                msg.as_bytes().to_vec(),
            )
            .await;
        }
    }
}

async fn post_file(stream: &mut TcpStream, req: &HttpRequest) {
    if !req.path.starts_with("/files/") {
        return_empty_http_response(stream, 400, b"Bad Request".to_vec()).await;
        return;
    }

    let prefix = unsafe { DIRECTORY.clone() };
    let path = format!("{}/{}", prefix, req.path.clone().split_off(7));

    let written = fs::write(path.clone(), &req.body).await;
    match written {
        Ok(_) => {
            return_http_response_with_data(stream, 201, b"Created".to_vec(), b"".to_vec()).await
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return_http_response_with_text(
                    stream,
                    404,
                    b"Not Found".to_vec(),
                    b"File not found".to_vec(),
                )
                .await;
                return;
            }

            let msg = format!(
                "Encountered error while trying to write to file ({}): {}",
                path, err
            );
            eprintln!("{}", msg);
            return_http_response_with_text(
                stream,
                500,
                b"Internal Server Error".to_vec(),
                msg.as_bytes().to_vec(),
            )
            .await;
        }
    }
}

async fn respond_via_http(stream: &mut TcpStream) {
    let mut buf: [u8; 16 * 1024] = [0; 16 * 1024];
    if let Err(err) = stream.read(&mut buf).await {
        eprintln!("Error when reading string from TCP stream: {err}");
        return;
    }

    // eprintln!("raw buf has len: {}", buf.len());

    let buf = String::from_utf8_lossy(&buf);
    let buf = buf.trim_end_matches(|c| "\0\x03\x04".contains(c));
    // eprintln!("str buf has len: {}", buf.len());
    // eprintln!("Last char value: {}", buf.as_bytes().last().unwrap());
    // eprintln!("str buf: {}", buf);
    let request = HttpRequest::from_str(buf);
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
        _ if request.path.starts_with("/files/") => match request.method {
            HttpMethod::Get => get_file(stream, &request).await,
            HttpMethod::Post => post_file(stream, &request).await,
            _ => return_empty_http_response(stream, 404, b"Not Found".to_vec()).await,
        },
        _ if request.path.starts_with("/echo/") => {
            return_path_http_response(stream, request.path.clone()).await
        }
        _ => return_empty_http_response(stream, 404, b"Not Found".to_vec()).await,
    }
}

fn get_directory_cli_arg() {
    let mut args = env::args();

    while let Some(arg) = args.next() {
        if arg == "--directory" {
            let dir = args.next();
            if let Some(dir) = dir {
                unsafe { DIRECTORY.clone_from(&dir) };
                println!("Have set ({}) as DIRECTORY", dir);
                break;
            }

            panic!("Expect to have directory name/path after dir flag");
        }
    }
}

#[tokio::main]
async fn main() -> TIOResult<()> {
    get_directory_cli_arg();

    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    println!("TcpListener has bound 4221.");

    loop {
        match listener.accept().await {
            Ok((mut stream, _addr)) => {
                println!("Accepted new connection");
                tokio::spawn(async move { respond_via_http(&mut stream).await });
            }
            Err(e) => {
                println!("Failed to accept connection: {}", e);
            }
        }
    }
}
