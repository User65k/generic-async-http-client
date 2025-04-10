use crate::{imp, Body, Error, HeaderName, HeaderValue, Response};
use serde::Serialize;
use std::{convert::TryInto, fmt::Debug};

/// Builds a HTTP request, poll it to query
/// ```
/// # use generic_async_http_client::{Request, Response, Error};
/// # async fn get() -> Result<(), Error> {
///     let req = Request::get("http://example.com/");
///     let resp = req.exec().await?;
/// # Ok(())
/// # }
/// ```
///
/// Depending on the chosen implementation, `Request` implements `TryFrom<(TryInto<Method>, TryInto<Url>)>`.
pub struct Request(pub(crate) imp::Req);
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
        Ok(imp::Req::new(meth, uri).map(Request)?)
    }
    /// Add a JSON body to the request
    /// ```
    /// # use generic_async_http_client::{Request, Response, Error};
    /// # use serde::Serialize;
    /// #[derive(Serialize)]
    /// struct JoseBody {
    ///     protected: String,
    ///     payload: String,
    ///     signature: String,
    /// }
    /// async fn jose(jose: &JoseBody) -> Result<Response, Error> {
    ///    let req = Request::put("http://example.com/").json(jose)?;
    ///    req.exec().await
    /// }
    /// ```
    pub fn json<T: Serialize + ?Sized>(mut self, json: &T) -> Result<Self, Error> {
        self.0.json(json)?;
        Ok(self)
    }
    /// Add a form data body to the request
    /// ```
    /// # use generic_async_http_client::{Request, Response, Error};
    /// # use serde::Serialize;
    /// #[derive(Serialize)]
    /// struct ContactForm {
    ///     email: String,
    ///     text: String,
    /// }
    /// async fn post_form(form: &ContactForm) -> Result<Response, Error> {
    ///    let req = Request::post("http://example.com/").form(form)?;
    ///    req.exec().await
    /// }
    /// ```
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> Result<Self, Error> {
        self.0.form(form)?;
        Ok(self)
    }
    /// Add query parameter to the request
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> Result<Self, Error> {
        self.0.query(query)?;
        Ok(self)
    }
    /// Add a body to the request
    /// ```
    /// # use generic_async_http_client::{Request, Response, Error};
    /// # async fn body() -> Result<Response, Error> {
    ///     let req = Request::post("http://example.com/").body("some body")?;
    /// #   req.exec().await
    /// # }
    /// ```
    pub fn body(mut self, body: impl Into<Body>) -> Result<Self, Error> {
        self.0.body(body.into())?;
        Ok(self)
    }
    /// Add a single header to the request
    /// If the map did have this key present, the new value is associated with the key
    /// ```
    /// # use generic_async_http_client::{Request, Response, Error};
    /// # async fn ua() -> Result<Response, Error> {
    ///     let req = Request::get("http://example.com/").set_header("User-Agent", "generic_async_http_client v0.2")?;
    /// #   req.exec().await
    /// # }
    /// ```
    pub fn set_header<N, V, E1, E2>(mut self, name: N, value: V) -> Result<Self, Error>
    where
        N: TryInto<HeaderName, Error = E1>,
        V: TryInto<HeaderValue, Error = E2>,
        Error: From<E1>,
        Error: From<E2>,
    {
        let val: HeaderValue = value.try_into()?;
        let name: HeaderName = name.try_into()?;
        self.0.set_header(name.into(), val.into())?;

        Ok(self)
    }
    /// Add a single header to the request
    /// If the map did have this key present, the new value is pushed to the end of the list of values
    pub fn add_header<N, V, E1, E2>(mut self, name: N, value: V) -> Result<Self, Error>
    where
        N: TryInto<HeaderName, Error = E1>,
        V: TryInto<HeaderValue, Error = E2>,
        Error: From<E1>,
        Error: From<E2>,
    {
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
        #[cfg(all(feature = "mock_tests", test))]
        let r = if crate::Mock::uses_mock() {
            Response(crate::Mock::check(self.0)?)
        } else {
            self.0.send_request().await?
        };
        #[cfg(not(all(feature = "mock_tests", test)))]
        let r = self.0.send_request().await?;
        //https://crates.io/crates/hreq

        match r.status_code() {
            100..300 => Ok(r),
            300..400 => {
                if let Some(loc) = r.header("Location").and_then(|l| l.try_into().ok()) {
                    let _l: String = loc;
                    //TODO redirect
                }
                Ok(r)
            }
            s @ 400..500 => Err(Error::HTTPClientErr(s, r)),
            //s @ 500..600 => Err(Error::HTTPServerErr(s, r)),
            s => Err(Error::HTTPServerErr(s, r)),
        }
    }
}
impl Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

pub trait Requests: Debug {
    fn get(uri: &str) -> Self;
    fn post(uri: &str) -> Self;
    fn put(uri: &str) -> Self;
    fn delete(uri: &str) -> Self;
    fn head(uri: &str) -> Self;
    fn options(uri: &str) -> Self;
    fn new(meth: &str, uri: &str) -> Result<Self, imp::Error>
    where
        Self: std::marker::Sized;
    async fn send_request(self) -> Result<Response, imp::Error>
    where
        Self: std::marker::Sized;
    fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), imp::Error>;
    fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), imp::Error>;
    fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), imp::Error>;
    fn body<B: Into<imp::Body>>(&mut self, body: B) -> Result<(), imp::Error>;
    fn set_header(
        &mut self,
        name: imp::HeaderName,
        values: imp::HeaderValue,
    ) -> Result<(), imp::Error>;
    fn add_header(
        &mut self,
        name: imp::HeaderName,
        values: imp::HeaderValue,
    ) -> Result<(), imp::Error>;
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
