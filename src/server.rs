use std::{
    collections::HashMap,
    io::{self, prelude::*, BufReader, Error, ErrorKind},
    marker::PhantomData,
    net::{TcpListener, TcpStream},
};

type Callback = fn(&TcpStream) -> io::Result<String>;
type RouteMap = HashMap<String, Callback>;

#[derive(Default)]
pub struct Server {
    host: String,
    routes: RouteMap,
}

impl Server {
    fn respond(&self, stream: &mut TcpStream, method: &str, endpoint: &str) -> io::Result<()> {
        if method.to_lowercase() != "get" {
            stream.write_all("HTTP/1.1 405 Method Not Allowed\r\n\r\n".as_bytes())?;
            return Ok(());
        }

        if let Some(cb) = self.routes.get(endpoint) {
            let contents = cb(stream)?;
            let length = contents.len();

            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{contents}");

            stream.write_all(response.as_bytes())
        } else {
            stream.write_all("HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes())
        }
    }

    fn handle_stream(&self, mut stream: TcpStream) -> io::Result<()> {
        let reader = BufReader::new(&mut stream);

        if let Some(Ok(request)) = reader.lines().next() {
            println!("{request}\n{stream:#?}");

            let mut split = request.split(' ');

            let method = split.next().ok_or(Error::from(ErrorKind::InvalidData))?;
            let endpoint = split.next().ok_or(Error::from(ErrorKind::InvalidData))?;

            self.respond(&mut stream, method, endpoint)
        } else {
            stream.write_all("HTTP/1.1 400 BAD REQUEST\r\n\r\n".as_bytes())
        }
    }

    /// # Errors
    ///
    /// Will error if the request has invalid data or if writing to the `TcpStream` fails.
    pub fn run(&self) -> io::Result<()> {
        debug_assert!(!self.host.is_empty());
        let listener = TcpListener::bind(&self.host)?;

        for stream in listener.incoming() {
            self.handle_stream(stream?)?;
        }

        Ok(())
    }
}

pub struct NoHost;
pub struct Host;

pub struct Builder<T> {
    host: String,
    routes: RouteMap,
    _marker: PhantomData<T>,
}

impl Default for Builder<NoHost> {
    fn default() -> Self {
        Builder {
            host: String::default(),
            routes: RouteMap::default(),
            _marker: PhantomData,
        }
    }
}

impl Builder<NoHost> {
    /// # Panics
    ///
    /// Will panic if host is empty.
    #[must_use]
    pub fn add_host(self, host: &str) -> Builder<Host> {
        assert!(!host.is_empty());

        Builder {
            host: host.to_string(),
            routes: self.routes,
            _marker: PhantomData,
        }
    }
}

impl Builder<Host> {
    /// # Panics
    ///
    /// Will panic if the endpoint doesn't start with a '/'.
    /// The route has to be relative to the root.
    #[allow(clippy::return_self_not_must_use)]
    pub fn add_route(mut self, endpoint: &str, callback: Callback) -> Self {
        assert!(endpoint.starts_with('/'));

        self.routes.insert(endpoint.to_string(), callback);
        self
    }

    #[must_use]
    pub fn build(self) -> Server {
        Server {
            host: self.host,
            routes: self.routes,
        }
    }
}
