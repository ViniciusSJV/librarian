//! Minimal HTTP/1.1 for a local tool: one request per connection, bodies by Content-Length.
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

pub struct Request {
    pub method: String,
    pub path: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        let found = self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name));
        found.map(|(_, v)| v.as_str())
    }
    pub fn param(&self, name: &str) -> Option<&str> {
        self.query.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    pub fn json(&self) -> Result<Value, String> {
        serde_json::from_slice(&self.body).map_err(|e| format!("Corpo JSON inválido: {e}"))
    }
}

pub struct Response {
    pub status: u16,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

impl Response {
    pub fn json(status: u16, value: &Value) -> Self {
        let body = serde_json::to_vec(value).unwrap_or_default();
        Self { status, content_type: "application/json; charset=utf-8", body }
    }
    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &serde_json::json!({ "error": message.into() }))
    }
    pub fn html(body: Vec<u8>) -> Self {
        Self { status: 200, content_type: "text/html; charset=utf-8", body }
    }
}

pub fn read(stream: &TcpStream) -> Result<Request, String> {
    let mut reader = BufReader::new(stream);
    let mut head = Vec::new();
    loop {
        let mut line = Vec::new();
        let n = reader.read_until(b'\n', &mut line).map_err(|e| e.to_string())?;
        if n == 0 || line == b"\r\n" || line == b"\n" {
            break;
        }
        head.push(String::from_utf8_lossy(&line).trim_end().to_owned());
        if head.iter().map(String::len).sum::<usize>() > MAX_HEADER_BYTES {
            return Err("Cabeçalho grande demais".into());
        }
    }
    let first = head.first().ok_or("Requisição vazia")?;
    let mut parts = first.split(' ');
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().ok_or("Requisição sem caminho")?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            (decode(k), decode(v))
        })
        .collect();
    let headers: Vec<(String, String)> = head[1..]
        .iter()
        .filter_map(|h| h.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();
    let length = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map(|(_, v)| v.parse::<usize>().map_err(|_| "Content-Length inválido"))
        .transpose()?
        .unwrap_or(0);
    if length > MAX_BODY_BYTES {
        return Err("Corpo grande demais".into());
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok(Request { method, path: decode(path), query, headers, body })
}

pub fn write(mut stream: &TcpStream, response: &Response) -> std::io::Result<()> {
    let reason = match response.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        501 => "Not Implemented",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        response.status,
        response.content_type,
        response.body.len()
    )?;
    stream.write_all(&response.body)?;
    stream.flush()
}

/// Percent-decoding for paths and query values ('+' is a space in query strings).
fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                        continue;
                    }
                    _ => out.push(b'%'),
                }
            }
            b'+' => out.push(b' '),
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_percent_and_plus_and_keeps_bad_escapes() {
        assert_eq!(super::decode("C%3A%5CUsers%5Cbeatl"), r"C:\Users\beatl");
        assert_eq!(super::decode("dire%C3%A7%C3%A3o+dos+raios"), "direção dos raios");
        assert_eq!(super::decode("100%"), "100%");
        assert_eq!(super::decode("%zz"), "%zz");
    }
}
