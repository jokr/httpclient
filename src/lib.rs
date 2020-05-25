use std::io::Read;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::net::ToSocketAddrs;

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

pub fn request(url: &str) -> Result<Response, std::io::Error> {
    let url = parse_url(url).expect(&format!("Could not parse url {}", url));
    println!("{:?}", url);

    let addrs: Vec<SocketAddr> = (&*url.host, url.port.unwrap_or(80))
        .to_socket_addrs()
        .expect(&format!("Could not resolve host {}", url.host))
        .collect();
    println!("{:?}", addrs);

    let mut stream = TcpStream::connect(&addrs[..]).expect("Could not connect to server.");
    println!("Connected to {:?}\n", stream.peer_addr().unwrap());
    let request = format!("GET {} HTTP/1.1\n", url.path.unwrap_or(String::from("/")));
    print!("{}", request);
    stream.write(request.as_bytes()).unwrap();
    let headers = [("Host", url.host)];
    for header in headers.iter() {
        let header_line = format!("{}: {}\n", header.0, header.1);
        print!("{}", header_line);
        stream.write(header_line.as_bytes()).unwrap();
    }
    print!("\n");
    stream.write("\n".as_bytes()).unwrap();
    stream.flush().unwrap();

    let mut reader = BufReader::new(stream);
    let mut buf = String::new();
    reader.read_line(&mut buf)?;
    print!("{}", buf);
    let status = buf.split(' ')
        .nth(1)
        .map(|code| { code.parse::<u16>().ok() })
        .flatten()
        .expect(&format!("Invalid HTTP response status line {}", buf));
    buf.clear();

    let mut headers = Vec::new();
    let mut content_length = None;
    while reader.read_line(&mut buf)? > 0 {
        print!("{}", buf);
        let idx = buf.find(":");
        if idx.is_none() {
            break;
        }
        let idx_val = idx.unwrap();
        let header = (buf[..idx_val].to_string(), buf[idx_val + 1..].to_string());
        if header.0 == "Content-Length" {
            content_length = Some(header.1.trim().parse::<usize>().unwrap());
        }
        headers.push(header);
        buf.clear();
    }

    match content_length {
        Some(0) => {
            return Ok(Response { status, headers, body: None});
        },
        Some(content_length) => {
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body)?;
            return Ok(Response { status, headers, body: Some(body)});
        },
        None => {
            let mut body = Vec::new();
            reader.read_to_end(&mut body)?;
            return Ok(Response { status, headers, body: Some(body)});
        },
    };
}

#[derive(Debug)]
struct URL {
    protocol: String,
    port: Option<u16>,
    host: String,
    path: Option<String>,
}

fn parse_url(url: &str) -> Option<URL> {
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

#[test]
fn test_parse_url() {
    let url_option = parse_url("http://www.example.com");
    assert_eq!(url_option.is_some(), true);
    let url = url_option.unwrap();
    assert_eq!(url.protocol, "http");
    assert_eq!(url.host, "www.example.com");
    assert_eq!(url.port, None);
    assert_eq!(url.path, None);

    let url_option = parse_url("https://example.com:8080/hello_world");
    assert_eq!(url_option.is_some(), true);
    let url = url_option.unwrap();
    assert_eq!(url.protocol, "https");
    assert_eq!(url.host, "example.com");
    assert_eq!(url.port, Some(8080));
    assert_eq!(url.path, Some(String::from("/hello_world")));
}