use std::collections::HashMap;
use std::fmt::Display;
use std::io::Error;
use std::io::ErrorKind::InvalidInput;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct HttpResponse {
    pub status_code: u16,
    pub phrase: Vec<u8>,
    pub headers: Vec<Vec<u8>>,
    pub body: Vec<u8>,
}

impl Display for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HTTP/1.1 {} {} \\r\\n {:?} \\r\\n {} \\r\\n",
            self.status_code,
            String::from_utf8_lossy(self.phrase.as_slice()),
            self.headers,
            String::from_utf8_lossy(self.body.as_slice())
        )
    }
}

impl HttpResponse {
    pub fn to_byte_string(&self) -> Vec<u8> {
        let mut result = vec![];

        // Status line
        result.extend_from_slice(b"HTTP/1.1 ");
        result.extend_from_slice(self.status_code.to_string().as_bytes());
        result.extend_from_slice(b" ");
        result.extend_from_slice(self.phrase.as_slice());
        result.extend_from_slice(b"\r\n");

        // Headers
        for header in self.headers.iter() {
            result.extend_from_slice(header);
            result.extend_from_slice(b"\r\n");
        }
        result.extend_from_slice(b"\r\n");

        // Body
        result.extend_from_slice(self.body.as_slice());

        result
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum HttpMethod {
    Get,
    Put,
    Post,
    Patch,
    Delete,
    Head,
    Options,
    Connect,
}

impl FromStr for HttpMethod {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "GET" => Ok(HttpMethod::Get),
            "PUT" => Ok(HttpMethod::Put),
            "POST" => Ok(HttpMethod::Post),
            "PATCH" => Ok(HttpMethod::Patch),
            "DELETE" => Ok(HttpMethod::Delete),
            "HEAD" => Ok(HttpMethod::Head),
            "OPTIONS" => Ok(HttpMethod::Options),
            "CONNECT" => Ok(HttpMethod::Connect),
            _ => Err(Error::new(InvalidInput, "Invalid HTTP Method")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl FromStr for HttpRequest {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();

        // parse status line
        let status_line = lines
            .next()
            .ok_or_else(|| Error::new(InvalidInput, "Status line missing"))?;
        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(Error::new(
                InvalidInput,
                "Invalid status line, expected to split it into 3 parts by whitespace",
            ));
        }

        let method = parts[0].parse()?;
        let path = parts[1].to_string();
        let version = parts[2].to_string();

        // parse headers
        let mut headers = HashMap::new();
        for line in lines.by_ref() {
            if line == "\r\n" {
                break;
            }

            if line.is_empty() {
                break;
            }

            let colon_pos = line
                .find(':')
                .ok_or_else(|| Error::new(InvalidInput, "Invalid header format"))?;
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            headers.insert(key, value);
        }

        // everything left is the body
        let body_lines = lines.collect::<Vec<&str>>();
        let blc = body_lines.concat();
        let body = blc.as_bytes();

        Ok(HttpRequest {
            method,
            path,
            version,
            headers,
            body: body.to_vec(),
        })
    }
}
