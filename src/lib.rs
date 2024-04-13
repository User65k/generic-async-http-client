/*! A generic async HTTP request create.

It is meant to be a thin wrapper around various HTTP clients
and handles TLS, serialisation and parsing.

The main goal is to allow binaries (that pull in some libraries that make use of a HTTP client)
to specify what implementation should be used.

And if there is a Proxy. If not specified auto detection is performed by looking at HTTP_PROXY.

You need to specify via features what crates are used to the actual work.

- `use_hyper` (and tokio)
- `use_async_h1` (and async-std)

Without anything specified you will end up with *No HTTP backend was selected*.

If performing more than one HTTP Request you should favor the use of [`Session`] over [`Request`].
*/
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
#[path = "dummy/mod.rs"]
mod imp;

#[cfg(any(feature = "use_hyper", feature = "use_async_h1"))]
mod tcp;
#[cfg(all(
    any(feature = "use_hyper", feature = "use_async_h1"),
    feature = "proxies"
))]
#[cfg_attr(docsrs, doc(cfg(feature = "proxies")))]
pub use tcp::proxy;

#[cfg(feature = "use_async_h1")]
#[path = "a_h1/mod.rs"]
mod imp;

#[cfg(feature = "use_hyper")]
#[path = "hyper/mod.rs"]
mod imp;

mod body;
mod header;
//mod session;
mod request;
mod response;

pub use request::Request;
pub use response::Response;
//pub use session::Session;
pub use body::Body;
pub use header::{HeaderName, HeaderValue};
pub use imp::Error;

#[cfg(test)]
mod tests {
    #[cfg(feature = "use_async_h1")]
    pub(crate) use async_std::{
        io::prelude::{ReadExt, WriteExt},
        net::{TcpListener, TcpStream},
        task::spawn,
    };
    #[cfg(feature = "use_async_h1")]
    pub(crate) fn block_on(
        fut: impl futures::Future<Output = Result<(), Box<dyn std::error::Error>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        async_std::task::block_on(fut)
    }
    //use futures::{AsyncWriteExt};
    #[cfg(feature = "use_hyper")]
    pub(crate) use tokio::{
        io::{AsyncReadExt as ReadExt, AsyncWriteExt as WriteExt},
        net::{TcpListener, TcpStream},
        runtime::Builder,
    };
    #[cfg(feature = "use_hyper")]
    pub(crate) fn block_on(
        fut: impl futures::Future<Output = Result<(), Box<dyn std::error::Error>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt")
            .block_on(fut)
    }
    #[cfg(feature = "use_hyper")]
    pub(crate) fn spawn<T>(fut: T) -> impl futures::Future<Output = T::Output>
    where
        T: futures::Future + Send + 'static,
        T::Output: Send + 'static,
    {
        let jh = tokio::task::spawn(fut);
        async { jh.await.expect("spawn failed") }
    }

    pub(crate) async fn assert_stream(
        stream: &mut TcpStream,
        should_be: &[u8],
    ) -> std::io::Result<()> {
        let l = should_be.len();
        let mut req: Vec<u8> = vec![0; l];
        let _r = stream.read(req.as_mut_slice()).await?;
        assert_eq!(req, should_be);
        Ok(())
    }
    pub(crate) async fn listen_somewhere() -> Result<(TcpListener, u16, String), std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        Ok((listener, addr.port(), addr.ip().to_string()))
    }

    use super::*;
    #[test]
    fn get() {
        async fn server(listener: TcpListener, host: String, port: u16) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;
            let mut output = Vec::with_capacity(1);
            assert_stream(
                &mut stream,
                format!(
                    "GET / HTTP/1.1\r\nhost: {}:{}\r\ncontent-length: 0\r\n\r\n",
                    host, port
                )
                .as_bytes(),
            )
            .await?;

            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\nabc")
                .await?;
            let _ = stream.read(&mut output).await?;
            Ok(true)
        }
        block_on(async {
            let (listener, port, host) = listen_somewhere().await?;
            let uri = format!("http://{}:{}", host, port);
            let t = spawn(server(listener, host, port));
            let r = Request::get(&uri);
            let mut aw = r.exec().await?;

            assert_eq!(aw.status_code(), 200, "wrong status");
            assert_eq!(aw.text().await?, "abc", "wrong text");
            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn header() {
        async fn server(listener: TcpListener, host: String, port: u16) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;
            //let mut output = Vec::with_capacity(2);

            #[cfg(feature = "use_async_h1")]
            assert_stream(
                &mut stream,
                format!(
                    "PUT / HTTP/1.1\r\nhost: {}:{}\r\ncontent-length: 0\r\ncookies: jo\r\n\r\n",
                    host, port
                )
                .as_bytes(),
            )
            .await?;
            #[cfg(feature = "use_hyper")]
            assert_stream(
                &mut stream,
                format!(
                    "PUT / HTTP/1.1\r\ncookies: jo\r\nhost: {}:{}\r\ncontent-length: 0\r\n\r\n",
                    host, port
                )
                .as_bytes(),
            )
            .await?;

            stream
                .write_all(b"HTTP/1.1 200 OK\r\ntest: a\r\ntest: 1\r\n\r\n")
                .await?;
            //stream.read(&mut output).await?;
            stream.flush().await?;
            Ok(true)
        }
        block_on(async {
            let (listener, port, host) = listen_somewhere().await?;
            let uri = format!("http://{}:{}", host, port);
            let server = spawn(server(listener, host, port));
            let r = Request::new("PUT", &uri)?;
            let r = r.set_header("Cookies", "jo")?;
            let resp = r.exec().await;
            if resp.is_err() {
                server.await.expect("sent data wrong");
                resp.expect("request failed");
                return Ok(());
            }
            let resp = resp.expect("request failed");

            assert_eq!(
                resp.header("test").expect("no test header"),
                "a",
                "wrong first header"
            );

            let mut h = resp.headers().filter(|(n, _v)| *n != "date"); //async h1 adds date
            let (n, v) = h.next().expect("two header missing");
            assert_eq!(
                <header::HeaderName as AsRef<[u8]>>::as_ref(n),
                &b"test"[..],
                "wrong 1st header"
            );
            assert_eq!(v, "a", "wrong 1st header");
            let (n, v) = h.next().expect("one header missing");
            assert_eq!(
                <header::HeaderName as AsRef<str>>::as_ref(n),
                "test",
                "wrong 2nd header"
            );
            assert_eq!(v, "1", "wrong 2nd header");

            let fin = h.next();
            assert!(fin.is_none(), "to much headers {:?}", fin);

            assert!(server.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
}
