use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

pub struct Req {
    opts: RequestInit,
    uri: String,

}
pub struct Resp {
    resp: Response
}

impl Req {
    pub fn get(uri: &str) -> Req {
        Self::new("GET", uri)
    }
    pub fn post(uri: &str) -> Req {
        Self::new("POST", uri)
    }
    pub fn put(uri: &str) -> Req {
        Self::new("PUT", uri)
    }
    pub fn delete(uri: &str) -> Req {
        Self::new("DELETE", uri)
    }
    pub fn head(uri: &str) -> Req {
        Self::new("HEAD", uri)
    }
    pub fn options(uri: &str) -> Req {
        Self::new("OPTIONS", uri)
    }
    pub fn new(meth: &str, uri: &str) -> Result<Req, Error> {
        let mut opts = RequestInit::new();
        opts.method(meth);
        opts.mode(RequestMode::Cors);
    
        Request {
            opts,
            uri: uri.to_string()
        }
    }
    pub async fn send_request(self) -> Result<Resp, Error> {
        let req = Request::new_with_str_and_init(&self.uri, &self.opts)?;
        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&req)).await?;
        let resp: Response = resp_value.dyn_into().unwrap();
        Ok(Resp{resp})
    }
    pub fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
        todo!();
    }
    pub fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
        todo!();
    }
    pub fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
        todo!();
    }
    pub fn body<B: Into<Body>>(&mut self, body: B) -> Result<(), Error> {
        self.opts.body(Some(body));
    }
    pub fn set_header(&mut self, name: HeaderName, value: HeaderValue) -> Result<(), Error> {
        self.req
        .headers()
        .set(name, value)?;
        Ok(())
    }
}
use hyper::body::{to_bytes, aggregate};
use serde::de::DeserializeOwned;
use hyper::body::Buf;
impl Resp {
    pub fn status(&self) -> u16 {
        self.resp.status()
    }
    pub fn status_str(&self) -> &'static str {
        self.resp.status_text()
    }
    pub async fn json(&mut self) -> Result<impl DeserializeOwned, Error> {
        let json = JsFuture::from(self.resp.json()?).await?;
        Ok(json.into_serde()?)
    }
    pub async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        let abuf = JsFuture::from(self.resp.arrayBuffer()?).await?;
        //blob() -> raw
        let array = js_sys::Uint8Array::new(&abuf);
        Ok(array.to_vec())
    }
    pub async fn string(&mut self) -> Result<String, Error> {
        let text = JsFuture::from(self.resp.text()?).await?;
        match text.as_string() {
            Some(s) => Ok(s),
            None => Err(Error::NoString)
        }
    }
    pub fn get_header(&self, name: HeaderName) -> Option<&HeaderValue> {
        self.resp.headers().get(name).ok().flatten()
    }
    pub fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        todo!();
    }
}

#[derive(Debug)]
pub enum Error {
    NoString,
    Error(JsValue)
}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
impl From<JsValue> for Error {
    fn from(e: JsValue) -> Self {
        Self::Error(e)
    }
}