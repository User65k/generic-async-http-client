use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
#[cfg(feature = "rustls")]
use std::{convert::TryFrom, sync::Arc};

#[cfg(feature = "proxies")]
mod socks5;
#[cfg(feature = "proxies")]
use socks5::connect_via_socks_prx;
#[cfg(feature = "proxies")]
mod http;
#[cfg(feature = "proxies")]
use http::connect_via_http_prx;

#[cfg(feature = "use_async_h1")]
use async_std::{
    io::{Read, Write},
    net::TcpStream,
};
#[cfg(all(feature = "use_async_h1", feature = "proxies"))]
use http_types::Url as Uri;
#[cfg(all(feature = "use_hyper", feature = "proxies"))]
use hyper::http::uri::Uri;
#[cfg(feature = "use_hyper")]
use hyper::rt::{Read, ReadBufCursor, Write};
#[cfg(feature = "use_hyper")]
use tokio::{
    io::{AsyncRead as _, AsyncWrite as _},
    net::TcpStream,
};

#[cfg(any(feature = "async_native_tls", feature = "hyper_native_tls"))]
use async_native_tls::{TlsConnector, TlsStream};
#[cfg(all(feature = "rustls", feature = "use_async_h1"))]
use futures_rustls::{
    client::TlsStream,
    rustls::{pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};
#[cfg(all(feature = "rustls", feature = "use_hyper"))]
use tokio_rustls::{
    client::TlsStream,
    rustls::{pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};
#[cfg(feature = "rustls")]
use webpki_roots::TLS_SERVER_ROOTS;

pub struct Stream {
    state: State,
}
enum State {
    #[cfg(any(
        feature = "rustls",
        feature = "hyper_native_tls",
        feature = "async_native_tls"
    ))]
    Tls(TlsStream<TcpStream>),
    Plain(TcpStream),
}

#[cfg(feature = "proxies")]
pub mod proxy {
    use super::*;
    use async_trait::async_trait;

    /// Sets the global proxy to a `&'static Proxy`.
    pub fn set_proxy(proxy: &'static dyn Proxy) {
        unsafe {
            GLOBAL_PROXY = proxy;
        }
    }
    /// Sets the global proxy to a `Box<Proxy>`.
    ///
    /// This is a simple convenience wrapper over `set_proxy`, which takes a
    /// `Box<Proxy>` rather than a `&'static Proxy`. See the documentation for
    /// [`set_proxy`] for more details.
    pub fn set_boxed_proxy(proxy: Box<dyn Proxy>) {
        set_proxy(Box::leak(proxy))
    }
    /// Returns a reference to the proxy.
    pub fn proxy() -> &'static dyn Proxy {
        unsafe { GLOBAL_PROXY }
    }
    static mut GLOBAL_PROXY: &dyn Proxy = &EnvProxy;

    #[async_trait]
    pub trait Proxy: Sync + Send {
        async fn connect_w_proxy(&self, host: &str, port: u16, tls: bool) -> io::Result<TcpStream>;
    }

    pub struct NoProxy;
    #[async_trait]
    impl Proxy for NoProxy {
        async fn connect_w_proxy(
            &self,
            host: &str,
            port: u16,
            _tls: bool,
        ) -> io::Result<TcpStream> {
            TcpStream::connect((host, port)).await
        }
    }
    ///
    /// `http_proxy`, `HTTPS_PROXY` should be set for protocol-specific proxies.
    /// General proxy should be set with `ALL_PROXY`
    ///
    /// A comma-separated list of host names that shouldn't go through any proxy is
    /// set in (only an asterisk, '*' matches all hosts) `NO_PROXY`
    pub struct EnvProxy;
    #[async_trait]
    impl Proxy for EnvProxy {
        async fn connect_w_proxy(&self, host: &str, port: u16, tls: bool) -> io::Result<TcpStream> {
            let mut prx = std::env::var("ALL_PROXY")
                .or_else(|_| std::env::var("all_proxy"))
                .ok();
            if prx.is_none() && tls {
                prx = std::env::var("HTTPS_PROXY")
                    .or_else(|_| std::env::var("https_proxy"))
                    .ok();
            }
            if prx.is_none() && !tls {
                prx = std::env::var("HTTP_PROXY")
                    .or_else(|_| std::env::var("http_proxy"))
                    .ok();
            }
            if let Ok(no_proxy) = std::env::var("NO_PROXY").or_else(|_| std::env::var("no_proxy")) {
                for h in no_proxy.split(',') {
                    match h.trim() {
                        a if a == host => {}
                        "*" => {}
                        _ => continue,
                    }
                    log::debug!("using no proxy due to env NO_PROXY");
                    prx = None;
                    break;
                }
            }
            match prx {
                None => TcpStream::connect((host, port)).await,
                Some(proxy) => {
                    let url = proxy
                        .parse::<Uri>()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                    #[cfg(feature = "use_hyper")]
                    let (phost, scheme) = (url.host(), url.scheme_str());
                    #[cfg(feature = "use_async_h1")]
                    let (phost, scheme) = (url.host_str(), Some(url.scheme()));

                    let phost = match phost {
                        Some(s) => s,
                        None => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "missing proxy host",
                            ));
                        }
                    };
                    #[cfg(feature = "use_hyper")]
                    let pport = url.port().map(|p| p.as_u16());
                    #[cfg(feature = "use_async_h1")]
                    let pport = url.port();

                    let pport = match pport {
                        Some(port) => port,
                        None => match scheme {
                            Some("https") => 443,
                            Some("http") => 80,
                            Some("socks5") => 1080,
                            Some("socks5h") => 1080,
                            _ => {
                                return Err(io::Error::new(
                                    io::ErrorKind::InvalidInput,
                                    "missing proxy port",
                                ))
                            }
                        },
                    };
                    log::info!("using proxy {}:{}", phost, pport);
                    match scheme {
                        Some("http") => connect_via_http_prx(host, port, phost, pport).await,
                        Some(socks5) if socks5 == "socks5" || socks5 == "socks5h" => {
                            connect_via_socks_prx(host, port, phost, pport, socks5 == "socks5h")
                                .await
                        }
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidInput,
                                "unsupported proxy scheme",
                            ))
                        }
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::tests::{
            assert_stream, block_on, listen_somewhere, spawn, TcpListener, WriteExt,
        };
        #[test]
        fn prx_from_env() {
            async fn server(listener: TcpListener) -> std::io::Result<bool> {
                let (mut stream, _) = listener.accept().await?;

                assert_stream(
                    &mut stream,
                    format!("CONNECT whatever:80 HTTP/1.1\r\nHost: whatever:80\r\n\r\n").as_bytes(),
                )
                .await?;
                stream.write_all(b"HTTP/1.1 200 Connected\r\n\r\n").await?;

                assert_stream(
                    &mut stream,
                    format!("GET /bla HTTP/1.1\r\nhost: whatever\r\ncontent-length: 0\r\n\r\n")
                        .as_bytes(),
                )
                .await?;
                stream
                    .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\nabc")
                    .await?;

                Ok(true)
            }
            block_on(async {
                let (listener, pport, phost) = listen_somewhere().await?;
                std::env::set_var("HTTP_PROXY", format!("http://{phost}:{pport}/"));
                std::env::set_var("NO_PROXY", &phost);
                let t = spawn(server(listener));

                let r = crate::Request::get("http://whatever/bla");
                let mut aw = r.exec().await?;

                assert_eq!(aw.status_code(), 200, "wrong status");
                assert_eq!(aw.text().await?, "abc", "wrong text");
                assert!(t.await?, "not cool");
                Ok(())
            })
            .unwrap();
        }
    }
}

#[cfg(any(
    feature = "rustls",
    feature = "hyper_native_tls",
    feature = "async_native_tls"
))]
fn get_tls_connector() -> io::Result<TlsConnector> {
    #[cfg(feature = "rustls")]
    {
        let mut root_store = RootCertStore::empty();
        root_store.extend(TLS_SERVER_ROOTS.iter().cloned());

        let mut config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        #[cfg(feature = "use_hyper")]
        config.alpn_protocols.push(b"h2".to_vec());
        config.alpn_protocols.push(b"http/1.1".to_vec());

        Ok(TlsConnector::from(Arc::new(config)))
    }
    #[cfg(any(feature = "async_native_tls", feature = "hyper_native_tls"))]
    return Ok(TlsConnector::new());
}

impl Stream {
    pub async fn connect(host: &str, port: u16, tls: bool) -> io::Result<Stream> {
        #[cfg(feature = "proxies")]
        let tcp = proxy::proxy().connect_w_proxy(host, port, tls).await?;
        #[cfg(not(feature = "proxies"))]
        let tcp = TcpStream::connect((host, port)).await?;
        log::trace!("connected to {}:{}", host, port);

        if tls {
            #[cfg(any(
                feature = "hyper_native_tls",
                feature = "async_native_tls",
                feature = "rustls"
            ))]
            {
                #[cfg(feature = "rustls")]
                let host = ServerName::try_from(host)
                    .map_err(|_e| io::Error::new(io::ErrorKind::InvalidInput, "Invalid DNS name"))?
                    .to_owned();
                let tlsc = get_tls_connector()?;

                let tls = tlsc.connect(host, tcp).await;
                return match tls {
                    Ok(stream) => {
                        log::trace!("wrapped TLS");
                        Ok(Stream {
                            state: State::Tls(stream),
                        })
                    }
                    Err(e) => {
                        log::error!("TLS Handshake: {}", e);
                        #[cfg(feature = "rustls")]
                        {
                            Err(e)
                        }
                        #[cfg(any(feature = "hyper_native_tls", feature = "async_native_tls"))]
                        Err(io::Error::new(io::ErrorKind::InvalidInput, e))
                    }
                };
            }
            #[cfg(not(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            )))]
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "no TLS backend available",
            ));
        } else {
            return Ok(Stream {
                state: State::Plain(tcp),
            });
        }
    }
}

#[cfg(feature = "use_hyper")]
impl Stream {
    pub fn get_proto(&self) -> hyper::Version {
        #[cfg(feature = "rustls")]
        if let State::Tls(ref t) = self.state {
            let (_, s) = t.get_ref();
            if Some(&b"h2"[..]) == s.alpn_protocol() {
                return hyper::Version::HTTP_2;
            }
        }
        hyper::Version::HTTP_11
    }
}

impl Write for Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            ))]
            State::Tls(ref mut t) => Pin::new(t).poll_write(cx, buf),
            State::Plain(ref mut t) => Pin::new(t).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            ))]
            State::Tls(ref mut t) => Pin::new(t).poll_flush(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_flush(cx),
        }
    }

    #[cfg(feature = "use_async_h1")]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            ))]
            State::Tls(ref mut t) => Pin::new(t).poll_close(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_close(cx),
        }
    }

    #[cfg(feature = "use_hyper")]
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::result::Result<(), std::io::Error>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            ))]
            State::Tls(ref mut t) => Pin::new(t).poll_shutdown(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_shutdown(cx),
        }
    }
}
impl Read for Stream {
    #[cfg(feature = "use_async_h1")]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(any(
                feature = "rustls",
                feature = "hyper_native_tls",
                feature = "async_native_tls"
            ))]
            State::Tls(ref mut t) => Pin::new(t).poll_read(cx, buf),
            State::Plain(ref mut t) => Pin::new(t).poll_read(cx, buf),
        }
    }
    #[cfg(feature = "use_hyper")]
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        mut buf: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        let f = {
            let mut tbuf = tokio::io::ReadBuf::uninit(unsafe { buf.as_mut() });
            let p = match pin.state {
                #[cfg(any(
                    feature = "rustls",
                    feature = "hyper_native_tls",
                    feature = "async_native_tls"
                ))]
                State::Tls(ref mut t) => Pin::new(t).poll_read(cx, &mut tbuf),
                State::Plain(ref mut t) => Pin::new(t).poll_read(cx, &mut tbuf),
            };
            match p {
                Poll::Ready(Ok(())) => tbuf.filled().len(),
                o => return o,
            }
        };
        unsafe {
            buf.advance(f);
        }
        Poll::Ready(Ok(()))
    }
}
