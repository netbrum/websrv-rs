use std::{
    collections::HashMap,
    io::{self, prelude::*, BufReader, Error, ErrorKind},
    marker::PhantomData,
    net::{TcpListener, TcpStream},
};

use crate::Pool;

type Callback = fn(&TcpStream) -> io::Result<String>;
type RouteMap = HashMap<String, Callback>;

pub struct Server {
    host: String,
    routes: RouteMap,
    pool: Pool,
}

impl Server {
    fn respond(stream: &mut TcpStream, cb: Callback) -> io::Result<()> {
        let contents = cb(stream)?;
        let length = contents.len();

        let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes())
    }

    fn handle_stream(&self, mut stream: TcpStream) -> io::Result<()> {
        let reader = BufReader::new(&mut stream);

        if let Some(Ok(request)) = reader.lines().next() {
            let mut split = request.split(' ');

            let method = split.next().ok_or(Error::from(ErrorKind::InvalidData))?;
            let endpoint = split.next().ok_or(Error::from(ErrorKind::InvalidData))?;

            if method.to_lowercase() != "get" {
                return stream.write_all("HTTP/1.1 405 Method Not Allowed\r\n\r\n".as_bytes());
            }

            if let Some(cb) = self.routes.get(endpoint) {
                let cb_c = *cb;

                self.pool.execute(move || {
                    println!("{request}\n{stream:#?}");
                    Self::respond(&mut stream, cb_c).expect("valid callback");
                });

                Ok(())
            } else {
                stream.write_all("HTTP/1.1 404 NOT FOUND\r\n\r\n".as_bytes())
            }
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
    pool_size: usize,
    _marker: PhantomData<T>,
}

const DEFAULT_POOL_SIZE: usize = 5;

impl Default for Builder<NoHost> {
    fn default() -> Self {
        Builder {
            host: String::default(),
            routes: RouteMap::default(),
            pool_size: DEFAULT_POOL_SIZE,
            _marker: PhantomData,
        }
    }
}

impl<T> Builder<T> {
    #[allow(clippy::must_use_candidate, clippy::return_self_not_must_use)]
    pub fn set_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
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
            pool_size: self.pool_size,
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
            pool: Pool::new(self.pool_size),
        }
    }
}
