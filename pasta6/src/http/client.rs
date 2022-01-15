use crate::http::connection::Connection;
use crate::http::{Method, Response};
use crate::net::TcpStream;
use std::io::{self, Write};

pub(crate) struct Client {
    tcp_stream: TcpStream,
    connection: Connection,
}

pub(crate) type ClientResult<T> = Result<T, ClientError>;

#[derive(Debug)]
pub(crate) struct ClientError {
    _source: Box<dyn std::error::Error>,
}

impl Client {
    #[inline]
    pub(crate) fn new(tcp_stream: TcpStream) -> ClientResult<Self> {
        let connection = Connection::new(tcp_stream.try_clone()?);
        Ok(Self {
            tcp_stream,
            connection,
        })
    }

    #[inline]
    pub(crate) fn request(&mut self, method: Method, path: &str) -> ClientResult<Response> {
        write!(
            self.tcp_stream,
            "{} {} HTTP/1.1\r\n\r\n",
            method.as_ref(),
            path
        )?;
        let response = self.connection.next_response()?;
        Ok(response)
    }
}

impl<E> From<E> for ClientError
where
    E: std::error::Error + 'static,
{
    #[inline]
    fn from(source: E) -> Self {
        Self {
            _source: Box::new(source),
        }
    }
}
