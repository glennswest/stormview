//! Just enough HTTP/1.1 and WebSocket client for reading a components feed:
//! `GET` with `Connection: close`, and a websocket upgrade that reads the
//! first text frame (the daemons push a full snapshot on connect). Plain
//! TCP only — the node daemons serve the feed without TLS.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub struct Resp {
    pub status: u16,
    pub body: Vec<u8>,
}

/// Why a request didn't produce a response.
#[derive(Debug)]
pub enum Error {
    /// Nothing listening, no route, or the connect timed out: the feed is
    /// not there.
    Connect(String),
    /// Connected, but the exchange broke or wasn't HTTP.
    Protocol(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Connect(e) => write!(f, "connect: {e}"),
            Error::Protocol(e) => write!(f, "{e}"),
        }
    }
}

fn connect(hostport: &str, timeout: Duration) -> Result<TcpStream, Error> {
    let addr = hostport
        .to_socket_addrs()
        .map_err(|e| Error::Connect(format!("{hostport}: {e}")))?
        .next()
        .ok_or_else(|| Error::Connect(format!("{hostport} resolves to nothing")))?;
    let s = TcpStream::connect_timeout(&addr, timeout).map_err(|e| Error::Connect(format!("{hostport}: {e}")))?;
    s.set_read_timeout(Some(timeout)).ok();
    s.set_write_timeout(Some(timeout)).ok();
    s.set_nodelay(true).ok();
    Ok(s)
}

/// Status line and headers (names lowercased).
fn head(r: &mut impl BufRead) -> Result<(u16, Vec<(String, String)>), Error> {
    let p = |e: std::io::Error| Error::Protocol(format!("reading the response: {e}"));
    let mut line = String::new();
    r.read_line(&mut line).map_err(p)?;
    let status = line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .filter(|_| line.starts_with("HTTP/1."))
        .ok_or_else(|| Error::Protocol(format!("not an HTTP response: {:?}", line.trim_end())))?;
    let mut headers = Vec::new();
    loop {
        line.clear();
        if r.read_line(&mut line).map_err(p)? == 0 {
            return Err(Error::Protocol("connection closed in the headers".into()));
        }
        let l = line.trim_end();
        if l.is_empty() {
            return Ok((status, headers));
        }
        if let Some((k, v)) = l.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
}

fn header<'a>(h: &'a [(String, String)], name: &str) -> Option<&'a str> {
    h.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

/// Decode a chunked body.
pub fn dechunk(r: &mut impl BufRead) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        r.read_line(&mut line).map_err(|e| format!("chunk size: {e}"))?;
        let size = usize::from_str_radix(line.trim().split(';').next().unwrap_or(""), 16)
            .map_err(|_| format!("bad chunk size {:?}", line.trim()))?;
        if size == 0 {
            return Ok(body);
        }
        let at = body.len();
        body.resize(at + size, 0);
        r.read_exact(&mut body[at..]).map_err(|e| format!("chunk: {e}"))?;
        line.clear();
        r.read_line(&mut line).map_err(|e| format!("chunk end: {e}"))?;
    }
}

/// `GET http://<hostport><path>`.
pub fn get(hostport: &str, path: &str, timeout: Duration) -> Result<Resp, Error> {
    let mut s = connect(hostport, timeout)?;
    let req = format!("GET {path} HTTP/1.1\r\nHost: {hostport}\r\nAccept: application/json\r\nUser-Agent: stormview-test\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes()).map_err(|e| Error::Protocol(format!("sending: {e}")))?;
    let mut r = BufReader::new(s);
    let (status, h) = head(&mut r)?;
    let body = if header(&h, "transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
        dechunk(&mut r).map_err(Error::Protocol)?
    } else if let Some(n) = header(&h, "content-length").and_then(|v| v.parse::<usize>().ok()) {
        let mut b = vec![0; n];
        r.read_exact(&mut b).map_err(|e| Error::Protocol(format!("body: {e}")))?;
        b
    } else {
        let mut b = Vec::new();
        r.read_to_end(&mut b).map_err(|e| Error::Protocol(format!("body: {e}")))?;
        b
    };
    Ok(Resp { status, body })
}

/// RFC 6455 §1.3's sample key and the accept value it must produce, so the
/// handshake is checked without carrying a SHA-1.
const WS_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const WS_ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

/// What a websocket upgrade came to.
pub enum Ws {
    /// The first text message.
    Text(Vec<u8>),
    /// The server answered the upgrade with a plain status (404: no
    /// websocket here; 401: it wants a session).
    Refused(u16),
}

/// Upgrade `path` to a websocket and return the first text message.
pub fn ws_first_text(hostport: &str, path: &str, timeout: Duration) -> Result<Ws, Error> {
    let mut s = connect(hostport, timeout)?;
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {hostport}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {WS_KEY}\r\nSec-WebSocket-Version: 13\r\nUser-Agent: stormview-test\r\n\r\n"
    );
    s.write_all(req.as_bytes()).map_err(|e| Error::Protocol(format!("sending: {e}")))?;
    let mut r = BufReader::new(s.try_clone().map_err(|e| Error::Protocol(e.to_string()))?);
    let (status, h) = head(&mut r)?;
    if status != 101 {
        return Ok(Ws::Refused(status));
    }
    if header(&h, "sec-websocket-accept") != Some(WS_ACCEPT) {
        return Err(Error::Protocol(format!("bad Sec-WebSocket-Accept {:?}", header(&h, "sec-websocket-accept"))));
    }
    let text = read_message(&mut r).map_err(Error::Protocol);
    // A masked, empty close frame; the server may already be gone.
    let _ = s.write_all(&[0x88, 0x80, 0, 0, 0, 0]);
    text.map(Ws::Text)
}

/// Read frames until one whole text message (fragments joined); control
/// frames other than close are passed over.
pub fn read_message(r: &mut impl Read) -> Result<Vec<u8>, String> {
    let mut msg = Vec::new();
    let mut in_text = false;
    loop {
        let mut h = [0u8; 2];
        r.read_exact(&mut h).map_err(|e| format!("frame header: {e}"))?;
        let fin = h[0] & 0x80 != 0;
        let op = h[0] & 0x0f;
        let masked = h[1] & 0x80 != 0;
        let len = match h[1] & 0x7f {
            126 => {
                let mut b = [0u8; 2];
                r.read_exact(&mut b).map_err(|e| e.to_string())?;
                u16::from_be_bytes(b) as u64
            }
            127 => {
                let mut b = [0u8; 8];
                r.read_exact(&mut b).map_err(|e| e.to_string())?;
                u64::from_be_bytes(b)
            }
            n => n as u64,
        };
        if len > 256 << 20 {
            return Err(format!("a {len}-byte frame"));
        }
        let mut mask = [0u8; 4];
        if masked {
            r.read_exact(&mut mask).map_err(|e| e.to_string())?;
        }
        let mut payload = vec![0u8; len as usize];
        r.read_exact(&mut payload).map_err(|e| format!("frame payload: {e}"))?;
        if masked {
            for (i, b) in payload.iter_mut().enumerate() {
                *b ^= mask[i % 4];
            }
        }
        match op {
            0x1 => {
                in_text = true;
                msg = payload;
            }
            0x0 if in_text => msg.extend_from_slice(&payload),
            0x8 => return Err("the server closed the websocket before a snapshot".into()),
            0x2 => return Err("a binary frame where a JSON snapshot was expected".into()),
            _ => continue, // ping, pong
        }
        if fin {
            return Ok(msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunked_bodies_join() {
        let mut r = BufReader::new(&b"5\r\nhello\r\n6;x=y\r\n world\r\n0\r\n\r\n"[..]);
        assert_eq!(dechunk(&mut r).unwrap(), b"hello world");
    }

    #[test]
    fn frames_decode_with_fragments_masks_and_pings() {
        let mut f = vec![0x89, 0x00]; // ping, skipped
        f.extend([0x01, 0x03]);
        f.extend(b"[1,");
        f.extend([0x80, 0x82, 1, 2, 3, 4, b'2' ^ 1, b']' ^ 2]); // masked continuation
        assert_eq!(read_message(&mut &f[..]).unwrap(), b"[1,2]");
    }

    #[test]
    fn long_frames_use_extended_lengths() {
        let body = vec![b'x'; 300];
        let mut f = vec![0x81, 126, 1, 44];
        f.extend(&body);
        assert_eq!(read_message(&mut &f[..]).unwrap(), body);
    }

    #[test]
    fn a_close_before_text_is_an_error() {
        assert!(read_message(&mut &[0x88u8, 0x00][..]).is_err());
    }
}
