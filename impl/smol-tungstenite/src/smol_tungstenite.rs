//! smol runtime integration with async-tungstenite
//!
//! Migrated from [`async-tungstenite/src/smol.rs`](https://github.com/sdroege/async-tungstenite/blob/main/src/smol.rs)
//! for 1.78 compiler support.
//!
//! Should be removed once macports provide 1.83 or later versions of Rust toolcahin.

use async_net::TcpStream;
use async_tungstenite::{
    tungstenite::{
        self,
        client::IntoClientRequest,
        handshake::client::{Request, Response},
        protocol::WebSocketConfig,
        Error,
    },
    WebSocketStream,
};

fn domain(request: &Request) -> Result<String, Error> {
    request
        .uri()
        .host()
        .map(|host| {
            // If host is an IPv6 address, it might be surrounded by brackets. These brackets are
            // *not* part of a valid IP, so they must be stripped out.
            //
            // The URI from the request is guaranteed to be valid, so we don't need a separate
            // check for the closing bracket.
            let host = if host.starts_with('[') {
                &host[1..host.len() - 1]
            } else {
                host
            };

            host.to_owned()
        })
        .ok_or(Error::Url(tungstenite::error::UrlError::NoHostName))
}

fn port(request: &Request) -> Result<u16, Error> {
    request
        .uri()
        .port_u16()
        .or_else(|| match request.uri().scheme_str() {
            Some("wss") => Some(443),
            Some("ws") => Some(80),
            _ => None,
        })
        .ok_or(Error::Url(
            tungstenite::error::UrlError::UnsupportedUrlScheme,
        ))
}

mod dummy_tls {
    use async_tungstenite::{
        client_async_with_config,
        tungstenite::{
            self,
            client::{uri_mode, IntoClientRequest},
            handshake::client::{Request, Response},
            protocol::WebSocketConfig,
            stream::Mode,
            Error,
        },
        WebSocketStream,
    };
    use futures_lite::{AsyncRead, AsyncWrite};

    pub type AutoStream<S> = S;
    type Connector = ();

    async fn wrap_stream<S>(
        socket: S,
        _domain: String,
        _connector: Option<()>,
        mode: Mode,
    ) -> Result<AutoStream<S>, Error>
    where
        S: 'static + AsyncRead + AsyncWrite + Unpin,
    {
        match mode {
            Mode::Plain => Ok(socket),
            Mode::Tls => Err(Error::Url(
                tungstenite::error::UrlError::TlsFeatureNotEnabled,
            )),
        }
    }

    /// Creates a WebSocket handshake from a request and a stream,
    /// upgrading the stream to TLS if required and using the given
    /// connector and WebSocket configuration.
    pub async fn client_async_tls_with_connector_and_config<R, S>(
        request: R,
        stream: S,
        connector: Option<Connector>,
        config: Option<WebSocketConfig>,
    ) -> Result<(WebSocketStream<AutoStream<S>>, Response), Error>
    where
        R: IntoClientRequest + Unpin,
        S: 'static + AsyncRead + AsyncWrite + Unpin,
        AutoStream<S>: Unpin,
    {
        let request: Request = request.into_client_request()?;

        let domain = super::domain(&request)?;

        // Make sure we check domain and mode first. URL must be valid.
        let mode = uri_mode(request.uri())?;

        let stream = wrap_stream(stream, domain, connector, mode).await?;
        client_async_with_config(request, stream, config).await
    }
}

pub type ClientStream<S> = dummy_tls::AutoStream<S>;
pub type ConnectStream = ClientStream<TcpStream>;

pub async fn connect_async<R>(
    request: R,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    connect_async_with_config(request, None).await
}

pub async fn connect_async_with_config<R>(
    request: R,
    config: Option<WebSocketConfig>,
) -> Result<(WebSocketStream<ConnectStream>, Response), Error>
where
    R: IntoClientRequest + Unpin,
{
    let request: Request = request.into_client_request()?;

    let domain = domain(&request)?;
    let port = port(&request)?;

    let try_socket = TcpStream::connect((domain.as_str(), port)).await;
    let socket = try_socket.map_err(Error::Io)?;
    dummy_tls::client_async_tls_with_connector_and_config(request, socket, None, config).await
}
