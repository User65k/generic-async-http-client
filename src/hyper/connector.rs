use hyper::{
    http::uri::{Scheme, Uri},
    service::Service,
};
use std::{
    future::Future,
    pin::Pin,
};

use crate::tcp::Stream;

async fn connect_to_uri(dst: &Uri) -> Result<Stream, super::Error> {
    let tls = match dst.scheme_str() {
        Some("https") => true,
        Some("http") => false,
        _ => {
            return Err(super::Error::Scheme)
        }
    };
    let host = match dst.host() {
        Some(s) => s,
        None => {
            return Err(hyper::http::uri::Authority::try_from("]").unwrap_err().into());
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
    Stream::connect(host, port, tls).await.map_err(|e|e.into())
}

#[derive(Debug, Clone, Default)]
pub enum HyperClient {
    #[default]
    New,/*
    H1(hyper::client::conn::http1::SendRequest<super::Body>),
    TlsH1(),
    TlsH2(),*/
}
impl HyperClient {
    pub async fn request(&mut self, req: super::Request<super::Body>) -> Result<super::Response<super::Incoming>, super::Error> {
        let io = connect_to_uri(req.uri()).await?;
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::task::spawn(async move {
            if let Err(err) = conn.await {
                println!("Connection failed: {:?}", err);
            }
        });
        sender.send_request(req).await.map_err(|e|e.into())
    }
}
