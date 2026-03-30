//! Asynchronous WebSocket backend through `async-io` and `async-tungstenite`.

use std::{
    future::Future,
    io::Write as _,
    net::{SocketAddr, TcpListener},
};

use async_io::Async;
use async_net::TcpStream;
use async_tungstenite::{
    smol::connect_async,
    tungstenite::{Bytes, Message},
    WebSocketStream,
};
use futures_lite::StreamExt;

pub use async_tungstenite::tungstenite::Error as WsError;
use futures_util::stream::FusedStream;

/// Asynchronous TCP socket backend.
#[derive(Debug)]
pub struct WebSocket {
    local: SocketAddr,
}

impl unisock::AsyncBackend for WebSocket {
    type Error = WsError;

    type Listener<'a>
        = Listener
    where
        Self: 'a;

    type Connection<'a>
        = Connection
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
        let (stream, _) = connect_async(format!("ws://{}", addr)).await?;
        Ok(Connection(stream))
    }
}

/// Asynchronous WebSocket listener.
#[derive(Debug)]
pub struct Listener(Async<TcpListener>);

impl unisock::AsyncListener for Listener {
    type Error = WsError;

    type Connection<'a>
        = Connection
    where
        Self: 'a;

    async fn accept(&self) -> Result<(Self::Connection<'_>, SocketAddr), Self::Error> {
        let (stream, addr) = self.0.accept().await?;
        let stream = async_tungstenite::accept_async(stream.into()).await?;
        Ok((Connection(stream), addr))
    }

    #[inline]
    fn close(self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}

/// Asynchronous WebSocket connection.
#[derive(Debug)]
pub struct Connection(WebSocketStream<TcpStream>);

impl unisock::AsyncConnection for Connection {
    type Error = WsError;

    async fn read<'fut>(&'fut mut self, mut buf: &'fut mut [u8]) -> Result<usize, Self::Error> {
        let msg = self
            .0
            .next()
            .await
            .ok_or(WsError::AlreadyClosed)
            .flatten()?;
        buf.write(&msg.into_data()).map_err(WsError::Io)
    }

    async fn write<'fut>(&'fut mut self, buf: &'fut [u8]) -> Result<usize, Self::Error> {
        let bytes = Bytes::copy_from_slice(buf);
        self.0.send(Message::Binary(bytes)).await?;
        Ok(buf.len())
    }

    async fn close(mut self) -> Result<(), Self::Error> {
        self.0.close(None).await
    }

    #[inline]
    fn poll_readable(&self, _cx: &mut core::task::Context<'_>) -> bool {
        !self.0.is_terminated()
    }

    #[inline]
    fn poll_writable(&self, _cx: &mut core::task::Context<'_>) -> bool {
        !self.0.is_terminated()
    }
}
