use std::io::{self, IoSlice, IoSliceMut, Read, Write};

#[derive(Debug)]
pub(crate) enum TcpStream {
    #[cfg(target_arch = "wasm32")]
    Lunatic(lunatic::net::TcpStream),
    Std(std::net::TcpStream),
}

impl TcpStream {
    #[inline]
    pub(crate) fn try_clone(&self) -> Result<Self, io::Error> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => Ok(TcpStream::Lunatic(tcp_stream.clone())),
            TcpStream::Std(tcp_stream) => Ok(TcpStream::Std(tcp_stream.try_clone()?)),
        }
    }
}

impl From<std::net::TcpStream> for TcpStream {
    #[inline]
    fn from(tcp_stream: std::net::TcpStream) -> Self {
        TcpStream::Std(tcp_stream)
    }
}

#[cfg(target_arch = "wasm32")]
impl From<lunatic::net::TcpStream> for TcpStream {
    #[inline]
    fn from(tcp_stream: lunatic::net::TcpStream) -> Self {
        TcpStream::Lunatic(tcp_stream)
    }
}

impl Read for TcpStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => tcp_stream.read(buf),
            TcpStream::Std(tcp_stream) => tcp_stream.read(buf),
        }
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => tcp_stream.read_vectored(bufs),
            TcpStream::Std(tcp_stream) => tcp_stream.read_vectored(bufs),
        }
    }
}

impl Write for TcpStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => tcp_stream.write(buf),
            TcpStream::Std(tcp_stream) => tcp_stream.write(buf),
        }
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => tcp_stream.write_vectored(bufs),
            TcpStream::Std(tcp_stream) => tcp_stream.write_vectored(bufs),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(target_arch = "wasm32")]
            TcpStream::Lunatic(tcp_stream) => tcp_stream.flush(),
            TcpStream::Std(tcp_stream) => tcp_stream.flush(),
        }
    }
}
