//! Asynchronous WebSocket backend through `async-io` and `async-tungstenite`.

use std::{
    future::Future,
    io::Write,
    net::{SocketAddr, TcpListener, TcpStream},
    pin::Pin,
};

use async_io::Async;
use async_tungstenite::{
    tungstenite::{Error as WsError, Message},
    WebSocketStream,
};
use futures_lite::StreamExt;
use futures_sink::Sink as _;
use socket2::Socket;

/// Asynchronous TCP socket backend.
#[derive(Debug)]
pub struct WebSocket {
    local: SocketAddr,
}

impl unisock::AsyncBackend for WebSocket {
    type Error = WsError;

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

    async fn connect(&self, addr: SocketAddr) -> Result<Self::Connection<'_>, Self::Error> {
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
        let stream = async_tungstenite::accept_async(Async::new(stream)?).await?;
        Ok(Connection(stream))
    }
}

/// Asynchronous WebSocket listener.
#[derive(Debug)]
pub struct Listener(Async<TcpListener>);

impl unisock::AsyncListener for Listener {
    type Error = WsError;

    type Connection<'a> = Connection
    where
        Self: 'a;

    async fn accept(&self) -> Result<(Self::Connection<'_>, SocketAddr), Self::Error> {
        let (stream, addr) = self.0.accept().await?;
        let stream = async_tungstenite::accept_async(stream).await?;
        Ok((Connection(stream), addr))
    }

    #[inline]
    fn close(self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}

/// Asynchronous WebSocket connection.
#[derive(Debug)]
pub struct Connection(WebSocketStream<Async<TcpStream>>);

impl unisock::AsyncConnection for Connection {
    type Error = WsError;

    async fn read<'fut>(&'fut mut self, mut buf: &'fut mut [u8]) -> Result<usize, Self::Error> {
        let msg = self
            .0
            .try_next()
            .await?
            .ok_or(WsError::AlreadyClosed)?
            .into_data();
        buf.write(&msg).map_err(WsError::Io)
    }

    async fn write<'fut>(&'fut mut self, buf: &'fut [u8]) -> Result<usize, Self::Error> {
        let mut pinned = Pin::new(&mut self.0);
        futures_lite::future::poll_fn(|cx| pinned.as_mut().poll_ready(cx)).await?;
        let msg = Message::binary(buf);
        pinned.as_mut().start_send(msg)?;
        futures_lite::future::poll_fn(|cx| pinned.as_mut().poll_flush(cx)).await?;
        Ok(buf.len())
    }

    fn close(self) -> impl Future<Output = Result<(), Self::Error>> {
        let mut pinned = Box::pin(self.0);
        futures_lite::future::poll_fn(move |cx| pinned.as_mut().poll_close(cx))
    }
}
