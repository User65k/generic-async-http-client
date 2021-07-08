use std::{future::Future, pin::Pin, task::{Context, Poll}};
use hyper::{service::Service, http::uri::{Scheme, Uri}};

use std::io;


use crate::tcp::Stream;

#[derive(Clone)]
pub struct Connector;

impl Connector {
    pub fn new() -> Connector {
        Connector {
        
        }
    }
}

impl Service<Uri> for Connector {
    type Response = Stream;
    type Error = std::io::Error;
    // We can't "name" an `async` generated future.
    type Future = Pin<Box<
        dyn Future<Output = Result<Self::Response, Self::Error>> + Send
    >>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // This connector is always ready, but others might not be.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let fut = async move {
            let tls = match dst.scheme_str() {
                Some("https") => true,
                Some("http") => false,
                _ => return Err(io::Error::new(io::ErrorKind::InvalidInput, "scheme must be http or https"))
            };
            let host = match dst.host() {
                Some(s) => s,
                None => {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing host"));
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
            Stream::connect(host, port, tls).await
        };

        Box::pin(fut)
    }
}
