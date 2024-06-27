//! Socket with custom implementations.
//!
//! This crate provides the core API for using custom implementations of sockets.

use std::{future::Future, io::Result, net::SocketAddr};

/// Asynchronous socket backend that provides the basic API for sending and receiving data.
pub trait AsyncBackend {
    /// Creates a new instance of the backend bound to the given address.
    ///
    /// If `addr` yields multiple addresses, `bind` will be attempted with each of the
    /// addresses until one succeeds and returns the socket. If none of the addresses
    /// succeed in creating a socket, the error returned from the last attempt
    /// (the last address) is returned.
    fn bind(addr: SocketAddr) -> Result<Self>
    where
        Self: Sized;

    /// Connects this backend to a remote address, allowing the `send` and `recv` calls
    /// to be used to send data and also applies filters to only receive data from the
    /// specified address.
    fn connect(&mut self, addr: SocketAddr) -> Result<()>;

    /// Receives a single datagram message on the socket from the remote address to which
    /// it is connected. On success, returns the number of bytes read.
    ///
    /// *This method will fail if the socket is not connected.*
    fn recv<'fut>(&'fut self, buf: &'fut mut [u8]) -> impl Future<Output = Result<usize>> + 'fut;

    /// Receives a single datagram message on the socket, without removing it from the queue.
    /// On success, returns the number of bytes read and the origin.
    fn recv_from<'fut>(
        &'fut self,
        buf: &'fut mut [u8],
    ) -> impl Future<Output = Result<(usize, SocketAddr)>> + 'fut;

    /// Sends data to the connected peer and returns the number of bytes written.
    ///
    /// *This method will fail if the socket is not connected.*
    fn send<'fut>(&'fut self, buf: &'fut [u8]) -> impl Future<Output = Result<usize>> + 'fut;

    /// Sends data on the socket to the given address. On success, returns the number of
    /// bytes written.
    fn send_to<'fut>(
        &'fut self,
        buf: &'fut [u8],
        addr: SocketAddr,
    ) -> impl Future<Output = Result<usize>> + 'fut;
}
