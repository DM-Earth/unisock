//! Socket with custom implementations.
//!
//! This crate provides the core API for using custom implementations of sockets.

#![no_std]

use core::{future::Future, net::SocketAddr};

/// Asynchronous socket backend that provides
pub trait AsyncBackend {
    /// The error type that is returned by the backend.
    type Error: core::error::Error;

    /// The listener type that is used to accept incoming connections.
    type Listener<'a>: AsyncListener<Connection<'a> = Self::Connection<'a>, Error = Self::Error>
    where
        Self: 'a;

    /// The connection type that is used to send and receive data.
    type Connection<'a>: AsyncConnection<Error = Self::Error>
    where
        Self: 'a;

    /// Binds the socket to the specified address and returns the socket.
    fn bind(addr: SocketAddr) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// The connection type that is used to send and receive data.
    fn listen(&self) -> Result<Self::Listener<'_>, Self::Error>;

    /// Connects to a remote address and returns the connection.
    fn connect(&self, addr: SocketAddr) -> Result<Self::Connection<'_>, Self::Error>;
}

/// Asynchronous listener that accepts incoming connections.
pub trait AsyncListener {
    /// The error type that is returned by the listener.
    type Error: core::error::Error;

    /// The connection type that is accepted by the listener.
    type Connection<'a>: AsyncConnection<Error = Self::Error>
    where
        Self: 'a;

    /// Accepts a new connection on the listener. On success, returns the connection and the
    /// remote address.
    fn accept(
        &self,
    ) -> impl Future<Output = Result<(Self::Connection<'_>, SocketAddr), Self::Error>> + '_;

    /// Closes the listener.
    fn close(self) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Asynchronous connection that provides the basic API for sending and receiving data.
pub trait AsyncConnection {
    /// The error type that is returned by the connection.
    type Error: core::error::Error;

    /// Receives a single datagram message on the socket. On success, returns the number of bytes read.
    fn read<'fut>(
        &'fut mut self,
        buf: &'fut mut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> + 'fut;

    /// Sends data on the socket. On success, returns the number of bytes written.
    fn write<'fut>(
        &'fut mut self,
        buf: &'fut [u8],
    ) -> impl Future<Output = Result<usize, Self::Error>> + 'fut;

    /// Closes the connection.
    fn close(self) -> impl Future<Output = Result<(), Self::Error>>;
}
