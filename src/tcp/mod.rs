use std::{sync::Arc};
use std::io;
use std::pin::Pin;
use std::task::{Poll, Context};

#[cfg(feature = "proxies")]
mod socks5;
#[cfg(feature = "proxies")]
use socks5::connect_via_socks_prx;
#[cfg(feature = "proxies")]
mod http;
#[cfg(feature = "proxies")]
use http::connect_via_http_prx;

#[cfg(feature = "use_async_h1")]
use async_std::{net::TcpStream,
    io::{Read, Write}
};
#[cfg(feature = "use_async_h1")]
use http_types::Url as Uri;
#[cfg(feature = "use_hyper")]
use tokio::{
    net::{TcpStream},
    io::{ReadBuf, AsyncRead, AsyncWrite as Write}
};
#[cfg(feature = "use_hyper")]
use hyper::{client::connect::Connection, http::uri::{Uri}};

#[cfg(all(feature = "rustls", feature = "use_async_h1"))]
use async_rustls::{rustls::ClientConfig, webpki::{DNSNameRef}, TlsConnector, client::TlsStream};
#[cfg(all(feature = "rustls", feature = "use_hyper"))]
use tokio_rustls::{rustls::{ClientConfig, Session}, webpki::{DNSNameRef}, TlsConnector, client::TlsStream};
#[cfg(feature = "rustls")]
use webpki_roots::TLS_SERVER_ROOTS;

pub struct Stream {
    state: State
}
enum State{
    #[cfg(feature = "rustls")]
    Tls(TlsStream<TcpStream>),
    Plain(TcpStream),
}

//static connect : Box<fn(&str, u16) -> dyn Future<Output = io::Result<TcpStream>>> = Box::new(connect_w_proxy);

/*
    http_proxy, HTTPS_PROXY

They should be set for protocol-specific proxies. General proxy should be
set with

    ALL_PROXY

A comma-separated list of host names that shouldn't go through any proxy is
set in (only an asterisk, '*' matches all hosts)

    NO_PROXY
*/
#[cfg(feature = "proxies")]
pub async fn connect_w_proxy(host: &str, port: u16, tls: bool) -> io::Result<TcpStream> {
    let mut prx = std::env::var("ALL_PROXY").or_else(|_|std::env::var("all_proxy")).ok();
    if prx==None && tls {
        prx = std::env::var("HTTPS_PROXY").or_else(|_|std::env::var("https_proxy")).ok();
    }
    if prx==None && !tls {
        prx = std::env::var("HTTP_PROXY").or_else(|_|std::env::var("http_proxy")).ok();
    }
    if let Ok(no_proxy) = std::env::var("NO_PROXY").or_else(|_|std::env::var("no_proxy")) {
        for h in no_proxy.split(",") {
            match h.trim() {
                a if a==host => {},
                "*" => {},
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
            let url = proxy.parse::<Uri>().map_err(|e|io::Error::new(io::ErrorKind::InvalidInput,e))?;

            #[cfg(feature = "use_hyper")]
            let (phost, scheme) = (url.host(), url.scheme_str());
            #[cfg(feature = "use_async_h1")]
            let (phost, scheme) = (url.host_str(), Some(url.scheme()));

            let phost = match phost {
                Some(s) => s,
                None => {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing proxy host"));
                }
            };
            #[cfg(feature = "use_hyper")]
            let pport = url.port().map(|p|p.as_u16());
            #[cfg(feature = "use_async_h1")]
            let pport = url.port();

            let pport = match pport {
                Some(port) => port,
                None => {
                    match scheme {
                        Some("https") => 443,
                        Some("http") => 80,
                        Some("socks5") => 1080,
                        Some("socks5h") => 1080,
                        _ => return Err(io::Error::new(io::ErrorKind::InvalidInput,"missing proxy port"))
                    }
                }
            };
            log::info!("using proxy {}:{}", phost, pport);
            match scheme {
                Some("http") => {
                    connect_via_http_prx(host, port,
                        phost, pport).await
                },
                Some(socks5) if socks5=="socks5"||socks5=="socks5h" => {
                    connect_via_socks_prx(host, port,
                        phost, pport, 
                        socks5=="socks5h").await
                },
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput,"unsupported proxy scheme"))
            }
        }
    }    
}

impl Stream {
    pub async fn connect(host: &str, port: u16, tls: bool) -> io::Result<Stream> {
        #[cfg(feature = "proxies")]
        let tcp = connect_w_proxy(host, port, tls).await?;
        #[cfg(not(feature = "proxies"))]
        let tcp = TcpStream::connect((host, port)).await?;
        log::trace!("connected to {}:{}", host, port);
        
        if tls {
            #[cfg(feature = "rustls")]
            {
                let domain = DNSNameRef::try_from_ascii_str(host)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput,e))?;
                let mut config = ClientConfig::default();

                #[cfg(feature = "use_hyper")]
                config.alpn_protocols.push(b"h2".to_vec());
                config.alpn_protocols.push(b"http/1.1".to_vec());

                config.root_store.add_server_trust_anchors(&TLS_SERVER_ROOTS);
                let tls = TlsConnector::from(Arc::new(config))
                    .connect(domain, tcp)
                    .await;
                return match tls {
                    Ok(stream) => {
                        log::trace!("wrapped TLS");
                        Ok(Stream {
                            state: State::Tls(stream)
                        })
                    },
                    Err(e) => {
                        log::error!("TLS Handshake: {}", e);
                        Err(e)
                    },
                };
            }
            #[cfg(not(any(feature = "rustls")))]
            return Err(io::Error::new(io::ErrorKind::InvalidInput,"no TLS backend available"));
        }else{
            return Ok(Stream {
                state: State::Plain(tcp)
            });
        }
    }
}

#[cfg(feature = "use_hyper")]
impl Connection for Stream {
    fn connected(&self) -> hyper::client::connect::Connected {
        let mut c = hyper::client::connect::Connected::new();

        match self.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref t) => {
                let (_, s) = t.get_ref();
                if Some(&b"h2"[..]) == s.get_alpn_protocol() {
                    c = c.negotiated_h2();
                }
            },
            _ => {},
        }
        c
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
                #[cfg(feature = "rustls")]
                State::Tls(ref mut t) => Pin::new(t).poll_write(cx, buf),
                State::Plain(ref mut t) => Pin::new(t).poll_write(cx, buf),
            }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref mut t) => Pin::new(t).poll_flush(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_flush(cx),
        }
    }

    #[cfg(feature = "use_async_h1")]
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref mut t) => Pin::new(t).poll_close(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_close(cx),
        }
    }

    #[cfg(feature = "use_hyper")]
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), std::io::Error>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref mut t) => Pin::new(t).poll_shutdown(cx),
            State::Plain(ref mut t) => Pin::new(t).poll_shutdown(cx),
        }
    }
}
#[cfg(feature = "use_async_h1")]
impl Read for Stream {
    fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref mut t) => Pin::new(t).poll_read(cx, buf),
            State::Plain(ref mut t) => Pin::new(t).poll_read(cx, buf),
        }
    }
}
#[cfg(feature = "use_hyper")]
impl AsyncRead for Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let pin = self.get_mut();
        match pin.state {
            #[cfg(feature = "rustls")]
            State::Tls(ref mut t) => Pin::new(t).poll_read(cx, buf),
            State::Plain(ref mut t) => Pin::new(t).poll_read(cx, buf),
        }
    }
}