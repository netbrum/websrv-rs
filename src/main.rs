use std::{fs, io, net::TcpStream};
use websrv_rs::Builder;

fn hello_ip(request: &TcpStream) -> io::Result<String> {
    Ok(format!("Hello, {}", request.peer_addr()?))
}

fn html(_request: &TcpStream) -> io::Result<String> {
    let contents = fs::read_to_string("example.html")?;

    Ok(contents)
}

#[allow(clippy::unnecessary_wraps)]
fn test(_request: &TcpStream) -> io::Result<String> {
    Ok(String::from("Test"))
}

fn main() {
    let server = Builder::default()
        .add_host("127.0.0.1:3002")
        .add_route("/", hello_ip)
        .add_route("/html", html)
        .add_route("/test", test)
        .build();

    match server.run() {
        Ok(()) => {}
        Err(e) => eprintln!("{e:#?}"),
    }
}
