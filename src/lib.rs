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

*/
#[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
#[path = "dummy/mod.rs"]
mod imp;

#[cfg(any(feature = "use_hyper", feature = "use_async_h1"))]
mod tcp;

#[cfg(feature = "use_async_h1")]
#[path = "a_h1/mod.rs"]
mod imp;

#[cfg(feature = "use_hyper")]
#[path = "hyper/mod.rs"]
mod imp;

mod body;
mod header;
//mod session;

//pub use session::Session;
pub use body::Body;
pub use header::{HeaderName, HeaderValue};
pub use imp::Error;
use std::convert::TryInto;

use serde::de::DeserializeOwned;
use serde::Serialize;

//use futures::{Future, FutureExt};
//use std::task::Poll;
//use std::pin::Pin;

/// Builds a HTTP request, poll it to query
pub struct Request(imp::Req);
impl Request {
    //auth
    //proxy - should be set by bin
    //cookies
    //timeout
    //tls validation
    //tls client certa
    //session (ref + cookies)
    pub fn get(uri: &str) -> Request {
        Request(imp::Req::get(uri))
    }
    pub fn post(uri: &str) -> Request {
        Request(imp::Req::post(uri))
    }
    pub fn put(uri: &str) -> Request {
        Request(imp::Req::put(uri))
    }
    pub fn delete(uri: &str) -> Request {
        Request(imp::Req::delete(uri))
    }
    pub fn head(uri: &str) -> Request {
        Request(imp::Req::head(uri))
    }
    pub fn options(uri: &str) -> Request {
        Request(imp::Req::options(uri))
    }
    pub fn new(meth: &str, uri: &str) -> Result<Request, Error> {
        imp::Req::new(meth, uri).map(|r| Request(r))
    }
    /// Add a JSON boby to the request
    pub fn json<T: Serialize + ?Sized>(mut self, json: &T) -> Result<Self, Error> {
        self.0.json(json)?;
        Ok(self)
    }
    /// Add a form data boby to the request
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> Result<Self, Error> {
        self.0.form(form)?;
        Ok(self)
    }
    /// Add query parameter to the request
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Result<Self, Error> {
        self.0.query(query)?;
        Ok(self)
    }
    /// Add a boby to the request
    pub fn body(mut self, body: impl Into<Body>) -> Result<Self, Error> {
        self.0.body(body.into())?;
        Ok(self)
    }
    /// Add a single header to the request
    /// If the map did have this key present, the new value is associated with the key
    pub fn set_header(
        mut self,
        name: impl TryInto<HeaderName, Error = imp::Error>,
        value: impl TryInto<HeaderValue, Error = imp::Error>,
    ) -> Result<Self, Error> {
        let val: HeaderValue = value.try_into()?;
        let name: HeaderName = name.try_into()?;
        self.0.set_header(name.into(), val.into())?;

        Ok(self)
    }
    /// Add a single header to the request
    /// If the map did have this key present, the new value is pushed to the end of the list of values
    pub fn add_header(
        mut self,
        name: impl TryInto<HeaderName, Error = imp::Error>,
        value: impl TryInto<HeaderValue, Error = imp::Error>,
    ) -> Result<Self, Error> {
        let val: HeaderValue = value.try_into()?;
        let name: HeaderName = name.try_into()?;
        self.0.add_header(name.into(), val.into())?;

        Ok(self)
    }
    /*
    TODO stream body
    body(Body::from_reader)
    */
    //TODO multipart

    /// Send the request to the webserver
    pub async fn exec(self) -> Result<Response, Error> {
        let r = self.0.send_request().await.map(|r| Response(r))?;

        if r.status_code() > 299 && r.status_code() < 399 {
            if let Some(loc) = r.header("Location").and_then(|l| l.try_into().ok()) {
                let _l: String = loc;
                //TODO redirect
            }
        }
        Ok(r)
    }
}
impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/*
enum State{
    Build(imp::Req),
    Fetch(Pin<Box<dyn Future<Output=Result<imp::Resp, Error>>>>)
}
struct Request2{
    state: std::cell::Cell<State>
}
impl Future for Request2{
    type Output = Result<Response, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        println!("poll");
        let pin = self.get_mut();

        match pin.state.get_mut() {
            State::Build(req) => {
                let fut = req.send_request();
                pin.state.set(State::Fetch(fut.boxed()));
                Poll::Pending
            },
            State::Fetch(mut fut) => {
                match fut.poll_unpin(cx) {
                    Poll::Ready(Ok(resp)) => Poll::Ready(Ok(Response(resp))),
                    Poll::Pending => Poll::Pending,
                    Poll::Ready(Err(e)) => Poll::Ready(Err(e))
                }
            },
        }
    }
}*/

/// The response of a webserver.
/// Headers and Status are available from the start,
/// the body must be polled/awaited again
pub struct Response(imp::Resp);
impl Response {
    /// Return the status code
    pub fn status_code(&self) -> u16 {
        self.0.status()
    }
    /// Return the status as string
    pub fn status(&self) -> &str {
        self.0.status_str()
    }
    /// Return the Body as some type deserialized from JSON
    pub async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        self.0.json().await
    }
    /// Return the whole Body as Bytes
    pub async fn content(&mut self) -> Result<Vec<u8>, Error> {
        self.0.bytes().await
    }
    /// Return the whole Body as String
    pub async fn text(&mut self) -> Result<String, Error> {
        self.0.string().await
    }
    /// If there are multiple values associated with the key, then the first one is returned.
    pub fn header(
        &self,
        name: impl TryInto<HeaderName, Error = imp::Error>,
    ) -> Option<&HeaderValue> {
        match name.try_into() {
            Err(_) => None,
            Ok(name) => self.0.get_header(name.into()).map(|v| v.into()),
        }
    }
    /// Each key will be yielded once per associated value. So, if a key has 3 associated values, it will be yielded 3 times.
    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.0.header_iter().map(|(n, v)| (n.into(), v.into()))
    }
    /*
    TODO cookie
    TODO encoding
    TODO raw (impl Read and or Stream)
    */
}

impl std::fmt::Debug for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h: Vec<(&HeaderName, &HeaderValue)> = self.headers().collect();
        write!(f, "HTTP {} Header: {:?}", self.status_code(), h)
    }
}

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
        io::{AsyncReadExt, AsyncWriteExt},
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

    pub(crate) async fn assert_stream(stream: &mut TcpStream, should_be: &[u8]) -> std::io::Result<()> {
        let l = should_be.len();
        let mut req: Vec<u8> = vec![0; l];
        stream.read_exact(req.as_mut_slice()).await?;
        if req != should_be {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "req not as expected",
            ));
        }
        return Ok(());
    }

    use super::*;
    #[test]
    fn get() {
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;
            let mut output = Vec::with_capacity(1);

            #[cfg(feature = "use_hyper")]
            assert_stream(
                &mut stream,
                b"GET / HTTP/1.1\r\nhost: 127.0.0.1:4657\r\n\r\n",
            )
            .await?;
            #[cfg(feature = "use_async_h1")]
            assert_stream(
                &mut stream,
                b"GET / HTTP/1.1\r\nhost: 127.0.0.1:4657\r\ncontent-length: 0\r\n\r\n",
            )
            .await?;

            stream
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 3\r\n\r\nabc")
                .await?;
            stream.read(&mut output).await?;
            Ok(true)
        }
        block_on(async {
            let listener = TcpListener::bind("127.0.0.1:4657").await?;
            let t = spawn(server(listener));
            let r = Request::get("http://127.0.0.1:4657");
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
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;
            //let mut output = Vec::with_capacity(2);

            #[cfg(feature = "use_async_h1")]
            assert_stream(&mut stream, b"PUT / HTTP/1.1\r\nhost: 127.0.0.1:5657\r\ncontent-length: 0\r\ncookies: jo\r\n\r\n").await?;
            #[cfg(feature = "use_hyper")]
            assert_stream(
                &mut stream,
                b"PUT / HTTP/1.1\r\ncookies: jo\r\nhost: 127.0.0.1:5657\r\n\r\n",
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
            let listener = TcpListener::bind("127.0.0.1:5657").await?;
            let t = spawn(server(listener));
            let r = Request::new("PUT", "http://127.0.0.1:5657")?;
            let r = r.set_header("Cookies", "jo")?;
            let resp = r.exec().await;
            if resp.is_err() {
                t.await.expect("sent data wrong");
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

            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
}
