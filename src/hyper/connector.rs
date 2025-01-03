use crate::tcp::Stream;
#[cfg(feature = "http2")]
use hyper::client::conn::http2;
use hyper::{
    client::conn::http1,
    header::{HeaderValue, HOST},
    http::uri::{Scheme, Uri},
};

async fn connect_to_uri(dst: &Uri) -> Result<Stream, super::Error> {
    let tls = match dst.scheme_str() {
        Some("https") => true,
        Some("http") => false,
        _ => return Err(super::Error::Scheme),
    };
    let host = match dst.host() {
        Some(s) => s,
        None => {
            return Err(hyper::http::uri::Authority::try_from("]")
                .unwrap_err()
                .into());
        }
    };
    let port = match dst.port() {
        Some(port) => port.as_u16(),
        None => {
            if dst.scheme() == Some(&Scheme::HTTPS) {
                443
            } else {
                80
            }
        }
    };
    Stream::connect(host, port, tls).await.map_err(|e| e.into())
}

#[derive(Debug, Default)]
pub enum HyperClient {
    #[default]
    New,
    H1(http1::SendRequest<super::Body>),
    #[cfg(feature = "http2")]
    H2(http2::SendRequest<super::Body>),
}

fn origin_form(uri: &mut Uri) {
    let path = match uri.path_and_query() {
        Some(path) if path.as_str() != "/" => {
            let mut parts = hyper::http::uri::Parts::default();
            parts.path_and_query = Some(path.clone());
            Uri::from_parts(parts).expect("path is valid uri")
        }
        _none_or_just_slash => {
            debug_assert!(Uri::default() == "/");
            Uri::default()
        }
    };
    *uri = path
}

#[cfg(feature = "http2")]
#[derive(Clone)]
struct TokioExecutor;

#[cfg(feature = "http2")]
impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, future: F) {
        tokio::spawn(future);
    }
}

impl HyperClient {
    pub async fn request(
        &mut self,
        mut req: super::Request<super::Body>,
    ) -> Result<super::Response<super::Incoming>, super::Error> {
        match self {
            HyperClient::New => {
                let io = connect_to_uri(req.uri()).await?;
                match io.get_proto() {
                    #[cfg(feature = "http2")]
                    hyper::Version::HTTP_2 => {
                        let (sender, conn) =
                            hyper::client::conn::http2::handshake(TokioExecutor, io).await?;
                        tokio::task::spawn(async move {
                            if let Err(err) = conn.await {
                                println!("Connection failed: {:?}", err);
                            }
                        });
                        let _ = std::mem::replace(self, HyperClient::H2(sender));
                    }
                    hyper::Version::HTTP_11 => {
                        let (sender, conn) = hyper::client::conn::http1::handshake(io).await?;
                        tokio::task::spawn(async move {
                            if let Err(err) = conn.await {
                                println!("Connection failed: {:?}", err);
                            }
                        });
                        let _ = std::mem::replace(self, HyperClient::H1(sender));
                    }
                    _ => unreachable!(),
                };
            }
            HyperClient::H1(_) => {}
            #[cfg(feature = "http2")]
            HyperClient::H2(_) => {}
        }

        match self {
            HyperClient::New => unreachable!(),
            HyperClient::H1(sender) => {
                let uri = req
                    .uri()
                    .authority()
                    .cloned()
                    .expect("authority implies host");
                req.headers_mut().entry(HOST).or_insert_with(|| {
                    let hostname = uri.host();
                    if let Some(port) = uri.port() {
                        let s = format!("{}:{}", hostname, port);
                        HeaderValue::from_str(&s)
                    } else {
                        HeaderValue::from_str(hostname)
                    }
                    .expect("uri host is valid header value")
                });

                origin_form(req.uri_mut());

                sender.send_request(req).await.map_err(|e| e.into())
            }
            #[cfg(feature = "http2")]
            HyperClient::H2(sender) => sender.send_request(req).await.map_err(|e| e.into()),
        }
    }
}
