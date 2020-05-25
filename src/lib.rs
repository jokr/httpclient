use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::ToSocketAddrs;

extern crate anyhow;
use anyhow::Result;

extern crate thiserror;
use thiserror::Error;

#[macro_use]
extern crate log;

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

#[derive(Error, Debug)]
pub enum HttpClientError {
    #[error("invalid url")]
    InvalidUrl,
    #[error("invalid http status line")]
    InvalidHttpStatusLine,
}

pub fn request(url: &str) -> Result<Response> {
    let url = URL::parse(url).ok_or(HttpClientError::InvalidUrl)?;

    let addrs: Vec<SocketAddr> = (&*url.host, url.port.unwrap_or(80))
        .to_socket_addrs()?
        .collect();

    let mut stream = TcpStream::connect(&addrs[..]).expect("Could not connect to server.");
    info!("Connected to {:?}\n", stream.peer_addr().unwrap());

    write!(
        stream,
        "GET {} HTTP/1.1\r\n",
        url.path.unwrap_or(String::from("/"))
    )?;
    let headers = [("Host", url.host)];
    for header in headers.iter() {
        write!(stream, "{}: {}\n", header.0, header.1)?;
    }
    stream.write("\n".as_bytes())?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    debug!("{}", buf);
    let status = buf
        .split(' ')
        .nth(1)
        .map(|code| code.parse::<u16>().ok())
        .flatten()
        .ok_or(HttpClientError::InvalidHttpStatusLine)?;
    buf.clear();
    info!("Received {} response.", status);

    let mut headers = Vec::new();
    let mut content_length = None;
    while reader.read_line(&mut buf)? > 0 {
        debug!("{}", buf);
        let idx = buf.find(":");
        if idx.is_none() {
            break;
        }
        let idx_val = idx.unwrap();
        let header = (
            buf[..idx_val].to_lowercase(),
            buf[idx_val + 1..].trim().to_string(),
        );
        if header.0 == "content-length" {
            content_length = Some(header.1.parse::<usize>().unwrap());
        }
        headers.push(header);
        buf.clear();
    }

    Ok(Response {
        status,
        headers,
        body: match content_length {
            Some(0) => None,
            Some(content_length) => {
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body)?;
                Some(body)
            }
            None => {
                let mut body = Vec::new();
                reader.read_to_end(&mut body)?;
                Some(body)
            }
        },
    })
}

#[derive(Debug)]
struct URL {
    protocol: String,
    port: Option<u16>,
    host: String,
    path: Option<String>,
}

impl URL {
    fn parse(url: &str) -> Option<URL> {
        let (protocol, url) = match url.find("://") {
            Some(idx) => (url[0..idx].to_string(), url[idx + 3..].to_string()),
            None => return None,
        };
        let (host_with_port, path) = match url.find("/") {
            Some(idx) => (url[0..idx].to_string(), Some(url[idx..].to_string())),
            None => (url, None),
        };
        let (host, port) = match host_with_port.find(":") {
            Some(idx) => (
                host_with_port[0..idx].to_string(),
                Some((host_with_port[idx + 1..]).parse::<u16>().unwrap()),
            ),
            None => (host_with_port, None),
        };
        Some(URL {
            protocol,
            host,
            path,
            port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let url_option = URL::parse("http://www.example.com");
        assert_eq!(url_option.is_some(), true);
        let url = url_option.unwrap();
        assert_eq!(url.protocol, "http");
        assert_eq!(url.host, "www.example.com");
        assert_eq!(url.port, None);
        assert_eq!(url.path, None);

        let url_option = URL::parse("https://example.com:8080/hello_world");
        assert_eq!(url_option.is_some(), true);
        let url = url_option.unwrap();
        assert_eq!(url.protocol, "https");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, Some(String::from("/hello_world")));
    }
}
