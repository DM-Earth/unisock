//! UDP backend using a single socket.

use std::{
    future::Future,
    net::{SocketAddr, UdpSocket},
};

use async_io::Async;

/// Asynchronous UDP socket backend.
#[derive(Debug)]
pub struct Udp {
    sock: Async<UdpSocket>,

    // Treat as a blocking map for now. We don't need it to be async,
    // although scc supports async.
    occupied: scc::HashSet<SocketAddr, ahash::RandomState>,
}

impl unisock::AsyncBackend for Udp {
    type Error = std::io::Error;

    type Listener<'a> = &'a Self
    where
        Self: 'a;

    type Connection<'a> = Connection<'a>
    where
        Self: 'a;

    fn bind(addr: SocketAddr) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let sock = UdpSocket::bind(addr)?;
        Ok(Self {
            sock: Async::new(sock)?,
            occupied: Default::default(),
        })
    }

    #[inline]
    fn listen(&self) -> Result<Self::Listener<'_>, Self::Error> {
        Ok(self)
    }

    fn connect(
        &self,
        addr: SocketAddr,
    ) -> impl Future<Output = Result<Self::Connection<'_>, Self::Error>> {
        std::future::ready((|| {
            if self.occupied.insert(addr).is_err() {
                return Err(std::io::ErrorKind::AddrInUse.into());
            }
            Ok(Connection {
                back: self,
                peer: addr,
            })
        })())
    }
}

impl unisock::AsyncListener for &Udp {
    type Error = std::io::Error;

    type Connection<'a> = Connection<'a>
    where
        Self: 'a;

    async fn accept(&self) -> Result<(Self::Connection<'_>, SocketAddr), Self::Error> {
        loop {
            let Ok((_, peer)) = self.sock.peek_from(&mut []).await else {
                continue;
            };
            if self.occupied.insert(peer).is_ok() {
                return Ok((Connection { back: self, peer }, peer));
            }
        }
    }

    #[inline]
    fn close(self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}

/// Asynchronous UDP socket connection backend.
#[derive(Debug)]
pub struct Connection<'a> {
    back: &'a Udp,
    peer: SocketAddr,
}

impl unisock::AsyncConnection for Connection<'_> {
    type Error = std::io::Error;

    async fn read<'fut>(&'fut mut self, buf: &'fut mut [u8]) -> Result<usize, Self::Error> {
        let sock = &self.back.sock;
        loop {
            let Ok((_, addr)) = sock.peek_from(&mut []).await else {
                continue;
            };
            if addr == self.peer {
                return sock.recv(buf).await;
            }
        }
    }

    #[inline]
    fn write<'fut>(
        &'fut mut self,
        buf: &'fut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> + 'fut {
        self.back.sock.send_to(buf, self.peer)
    }

    #[inline]
    fn close(self) -> impl Future<Output = Result<(), Self::Error>> {
        std::future::ready(Ok(()))
    }
}

impl Drop for Connection<'_> {
    #[inline]
    fn drop(&mut self) {
        self.back.occupied.remove(&self.peer);
    }
}
