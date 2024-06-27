use std::net::{SocketAddr, TcpListener, TcpStream};

use async_io::Async;
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use socket2::Socket;

/// Asynchronous TCP socket backend.
///
/// This backend contains a listener, and all sockets needed are contained
/// in futures returned, but not in the backend itself.
#[derive(Debug)]
pub struct Tcp {
    listener: Async<TcpListener>,
    local: SocketAddr,
    peer: Option<SocketAddr>,
}

impl Tcp {
    #[inline(always)]
    fn socket(&self, peer: SocketAddr) -> std::io::Result<Socket> {
        let socket = Socket::new(
            match self.local {
                SocketAddr::V4(_) => socket2::Domain::IPV4,
                SocketAddr::V6(_) => socket2::Domain::IPV6,
            },
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;

        socket.set_reuse_address(true)?;
        socket.bind(&self.local.into())?;
        socket.connect(&peer.into())?;
        Ok(socket)
    }
}

impl unisock::AsyncBackend for Tcp {
    #[inline]
    fn bind(addr: SocketAddr) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let listener = <Async<TcpListener>>::bind(addr)?;
        Ok(Self {
            listener,
            local: addr,
            peer: None,
        })
    }

    #[inline]
    fn connect(&mut self, addr: SocketAddr) -> std::io::Result<()> {
        self.peer = Some(addr);
        Ok(())
    }

    async fn recv<'fut>(&'fut self, buf: &'fut mut [u8]) -> std::io::Result<usize> {
        let peer = self.peer.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotConnected, "tcp socket not connected")
        })?;
        let socket: TcpStream = self.socket(peer)?.into();
        let mut socket = Async::new(socket)?;
        socket.readable().await?;
        socket.read(buf).await
    }

    #[inline]
    async fn recv_from<'fut>(
        &'fut self,
        buf: &'fut mut [u8],
    ) -> std::io::Result<(usize, SocketAddr)> {
        let (mut stream, peer) = self.listener.accept().await?;
        Ok((stream.read(buf).await?, peer))
    }

    async fn send<'fut>(&'fut self, buf: &'fut [u8]) -> std::io::Result<usize> {
        let peer = self.peer.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotConnected, "tcp socket not connected")
        })?;
        let socket: TcpStream = self.socket(peer)?.into();
        let mut socket = Async::new(socket)?;
        socket.writable().await?;
        socket.write(buf).await
    }

    async fn send_to<'fut>(
        &'fut self,
        buf: &'fut [u8],
        addr: SocketAddr,
    ) -> std::io::Result<usize> {
        let socket: TcpStream = self.socket(addr)?.into();
        let mut socket = Async::new(socket)?;
        socket.writable().await?;
        socket.write(buf).await
    }
}
