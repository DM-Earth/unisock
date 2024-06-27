use std::net::UdpSocket;

use async_io::Async;
use socket2::Socket;

/// Asynchronous UDP socket backend.
#[derive(Debug)]
pub struct Udp(Async<UdpSocket>);

impl unisock::AsyncBackend for Udp {
    fn bind(addr: std::net::SocketAddr) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        let socket = Socket::new(
            match addr {
                std::net::SocketAddr::V4(_) => socket2::Domain::IPV4,
                std::net::SocketAddr::V6(_) => socket2::Domain::IPV6,
            },
            socket2::Type::DGRAM,
            None,
        )?;
        socket.set_reuse_address(true)?;
        socket.bind(&addr.into())?;

        let socket: UdpSocket = socket.into();
        Async::new(socket).map(Self)
    }

    #[inline]
    fn connect(&mut self, addr: std::net::SocketAddr) -> std::io::Result<()> {
        self.0.get_ref().connect(addr)
    }

    #[inline]
    fn recv<'fut>(
        &'fut self,
        buf: &'fut mut [u8],
    ) -> impl std::future::Future<Output = std::io::Result<usize>> + 'fut {
        self.0.recv(buf)
    }

    #[inline]
    fn recv_from<'fut>(
        &'fut self,
        buf: &'fut mut [u8],
    ) -> impl std::future::Future<Output = std::io::Result<(usize, std::net::SocketAddr)>> + 'fut
    {
        self.0.recv_from(buf)
    }

    #[inline]
    fn send<'fut>(
        &'fut self,
        buf: &'fut [u8],
    ) -> impl std::future::Future<Output = std::io::Result<usize>> + 'fut {
        self.0.send(buf)
    }

    #[inline]
    fn send_to<'fut>(
        &'fut self,
        buf: &'fut [u8],
        addr: std::net::SocketAddr,
    ) -> impl std::future::Future<Output = std::io::Result<usize>> + 'fut {
        self.0.send_to(buf, addr)
    }
}
