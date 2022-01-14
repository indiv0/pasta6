use crate::http::connection::Connection;
use crate::http::{Method, Response};
use std::io::Write;

pub(crate) type ClientResult<T> = Result<T, ClientError>;

pub(crate) struct Client {
    tcp_stream: lunatic::net::TcpStream,
    connection: Connection,
}

pub(crate) struct ClientError {
    _source: Box<dyn std::error::Error>,
}

#[cfg(target_arch = "wasm32")]
impl Client {
    #[inline]
    pub(crate) fn new(host: &str) -> ClientResult<Self> {
        let tcp_stream = lunatic::net::TcpStream::connect(host)?;
        let connection = Connection::new(tcp_stream.clone());
        Ok(Self {
            tcp_stream,
            connection,
        })
    }

    #[inline]
    pub(crate) fn get(&mut self, method: Method, path: &str) -> ClientResult<Response> {
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
