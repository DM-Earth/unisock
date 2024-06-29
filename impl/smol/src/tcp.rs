//! Asynchronous TCP socket backend.

use std::net::{SocketAddr, TcpListener, TcpStream};

use async_io::Async;
use futures_lite::{AsyncReadExt as _, AsyncWriteExt as _};
use socket2::Socket;

/// Asynchronous TCP socket backend.
#[derive(Debug)]
pub struct Tcp {
    local: SocketAddr,
}

impl unisock::AsyncBackend for Tcp {
    type Error = std::io::Error;

    type Listener<'a> = Listener
    where
        Self: 'a;

    type Connection<'a> = Connection
    where
        Self: 'a;

    #[inline]
    fn bind(addr: SocketAddr) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self { local: addr })
    }

    #[inline]
    fn listen(&self) -> Result<Self::Listener<'_>, Self::Error> {
        let listener = TcpListener::bind(self.local)?;
        Ok(Listener(Async::new(listener)?))
    }

    fn connect(&self, addr: SocketAddr) -> Result<Self::Connection<'_>, Self::Error> {
        let sock = Socket::new(
            match self.local {
                SocketAddr::V4(_) => socket2::Domain::IPV4,
                SocketAddr::V6(_) => socket2::Domain::IPV6,
            },
            socket2::Type::STREAM,
            None,
        )?;
        sock.bind(&self.local.into())?;
        sock.connect(&addr.into())?;
        let stream: TcpStream = sock.into();
        Ok(Connection(Async::new(stream)?))
    }
}

/// Asynchronous TCP listener.
#[derive(Debug)]
pub struct Listener(Async<TcpListener>);

impl unisock::AsyncListener for Listener {
    type Error = std::io::Error;

    type Connection<'a> = Connection
    where
        Self: 'a;

    #[inline]
    async fn accept(&self) -> Result<(Self::Connection<'_>, SocketAddr), Self::Error> {
        let (stream, addr) = self.0.accept().await?;
        Ok((Connection(stream), addr))
    }

    #[inline]
    fn close(self) -> impl futures_lite::Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}

/// Asynchronous TCP connection.
#[derive(Debug)]
pub struct Connection(Async<TcpStream>);

impl unisock::AsyncConnection for Connection {
    type Error = std::io::Error;

    #[inline]
    fn read<'fut>(
        &'fut mut self,
        buf: &'fut mut [u8],
    ) -> impl futures_lite::Future<Output = Result<usize, Self::Error>> {
        self.0.read(buf)
    }

    #[inline]
    async fn write<'fut>(&'fut mut self, buf: &'fut [u8]) -> Result<usize, Self::Error> {
        self.0.write(buf).await
    }

    #[inline]
    fn close(self) -> impl futures_lite::Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}
