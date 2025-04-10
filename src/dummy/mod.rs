use serde::de::DeserializeOwned;
use serde::Serialize;
use std::convert::TryFrom;

static ERR_MSG: &str = "No HTTP backend was selected";

pub struct Body(Vec<u8>);
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct HeaderName(pub(crate) String);
#[derive(Debug, Clone)]
pub struct HeaderValue(pub(crate) Vec<u8>);

impl crate::response::Responses for () {
    fn status(&self) -> u16 {
        500
    }
    fn status_str(&self) -> &'static str {
        "not implemented"
    }
    async fn json<D: DeserializeOwned>(&mut self) -> Result<D, Error> {
        Err(Error {})
    }
    async fn bytes(&mut self) -> Result<Vec<u8>, Error> {
        Err(Error {})
    }
    async fn string(&mut self) -> Result<String, Error> {
        Err(Error {})
    }
    fn get_header(&self, _name: HeaderName) -> Option<&HeaderValue> {
        None
    }
    fn header_iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        vec![].into_iter()
    }
    fn get_headers(&self, _name: self::HeaderName) -> impl Iterator<Item = &self::HeaderValue> {
        vec![].into_iter()
    }
}

/// Backend specific error
#[derive(Debug)]
pub struct Error {}
impl std::error::Error for Error {}
use std::fmt;
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(ERR_MSG)
    }
}
impl From<Error> for crate::Error {
    fn from(e: Error) -> Self {
        Self::Other(e)
    }
}

pub use maybemock::{Req, Resp};
#[cfg(not(all(feature = "mock_tests", test)))]
mod maybemock {
    use super::*;
    #[derive(Debug)]
    pub struct Req {}
    impl crate::request::Requests for Req {
        fn get(uri: &str) -> Req {
            Req::new("GET", uri).unwrap()
        }
        fn post(uri: &str) -> Req {
            Req::new("POST", uri).unwrap()
        }
        fn put(uri: &str) -> Req {
            Req::new("PUT", uri).unwrap()
        }
        fn delete(uri: &str) -> Req {
            Req::new("DELETE", uri).unwrap()
        }
        fn head(uri: &str) -> Req {
            Req::new("HEAD", uri).unwrap()
        }
        fn options(uri: &str) -> Req {
            Req::new("OPTIONS", uri).unwrap()
        }
        fn new(meth: &str, uri: &str) -> Result<Req, Error> {
            log::debug!("{} {}", meth, uri);
            Ok(Req {})
        }
        async fn send_request(self) -> Result<crate::Response, Error> {
            eprintln!("{}", ERR_MSG);
            println!("{}", ERR_MSG);
            Err(Error {})
        }
        fn json<T: Serialize + ?Sized>(&mut self, _json: &T) -> Result<(), Error> {
            Ok(())
        }
        fn form<T: Serialize + ?Sized>(&mut self, _data: &T) -> Result<(), Error> {
            Ok(())
        }
        fn query<T: Serialize + ?Sized>(&mut self, _query: &T) -> Result<(), Error> {
            Ok(())
        }
        fn body<B: Into<Body>>(&mut self, _body: B) -> Result<(), Error> {
            Ok(())
        }
        fn set_header(&mut self, _name: HeaderName, _values: HeaderValue) -> Result<(), Error> {
            Ok(())
        }
        fn add_header(&mut self, _name: HeaderName, _values: HeaderValue) -> Result<(), Error> {
            Ok(())
        }
    }
    pub type Resp = ();
}
#[cfg(all(feature = "mock_tests", test))]
mod maybemock {
    use super::*;
    impl From<serde_json::Error> for Error {
        fn from(e: serde_json::Error) -> Self {
            panic!("test with invalid json {}", e);
        }
    }
    impl crate::mock::MockedRequest for Req {
        /// on error, return full body
        fn assert_body_bytes(&mut self, should_be: &[u8]) -> Result<(), Vec<u8>> {
            let is = &self.body;
            if is != should_be {
                Err(is.clone())
            } else {
                Ok(())
            }
        }
        fn get_headers(&self, name: &str) -> Option<Vec<crate::mock::MockHeaderValue>> {
            let name = HeaderName(name.to_string());
            let hm = &self.header;
            Some(hm.get(&name)?.iter().cloned().map(|v| v.into()).collect())
        }
        fn endpoint(&self) -> crate::mock::Endpoint {
            (self.meth.to_string(), self.uri.to_string())
        }
    }
    #[derive(Debug)]
    pub struct Req {
        meth: String,
        uri: String,
        body: Vec<u8>,
        header: std::collections::HashMap<HeaderName, Vec<HeaderValue>>,
    }
    pub type Resp = crate::mock::Resp<()>;
    impl crate::request::Requests for Req {
        fn get(uri: &str) -> Req {
            Req::new("GET", uri).unwrap()
        }
        fn post(uri: &str) -> Req {
            Req::new("POST", uri).unwrap()
        }
        fn put(uri: &str) -> Req {
            Req::new("PUT", uri).unwrap()
        }
        fn delete(uri: &str) -> Req {
            Req::new("DELETE", uri).unwrap()
        }
        fn head(uri: &str) -> Req {
            Req::new("HEAD", uri).unwrap()
        }
        fn options(uri: &str) -> Req {
            Req::new("OPTIONS", uri).unwrap()
        }
        fn new(meth: &str, uri: &str) -> Result<Req, Error> {
            log::debug!("{} {}", meth, uri);
            Ok(Req {
                meth: meth.to_ascii_uppercase(),
                uri: uri.to_string(),
                body: Default::default(),
                header: Default::default(),
            })
        }
        async fn send_request(self) -> Result<crate::Response, Error> {
            eprintln!("{}", ERR_MSG);
            println!("{}", ERR_MSG);
            Err(Error {})
        }
        fn json<T: Serialize + ?Sized>(&mut self, json: &T) -> Result<(), Error> {
            let b = serde_json::to_string(json).unwrap();
            self.set_header(
                HeaderName("CONTENT_TYPE".to_string()),
                HeaderValue(b"application/json".to_vec()),
            )?;
            self.body(b)
        }
        fn form<T: Serialize + ?Sized>(&mut self, data: &T) -> Result<(), Error> {
            let b = serde_urlencoded::to_string(data).unwrap();
            self.set_header(
                HeaderName("CONTENT_TYPE".to_string()),
                HeaderValue(b"application/x-www-form-urlencoded".to_vec()),
            )?;
            self.body(b)
        }
        fn query<T: Serialize + ?Sized>(&mut self, query: &T) -> Result<(), Error> {
            let q = serde_qs::to_string(&query).unwrap();
            todo!();
            Ok(())
        }
        fn body<B: Into<Body>>(&mut self, b: B) -> Result<(), Error> {
            let b: Body = b.into();
            self.body = b.0;
            Ok(())
        }
        fn set_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
            self.header.insert(name, vec![values]);
            Ok(())
        }
        fn add_header(&mut self, name: HeaderName, values: HeaderValue) -> Result<(), Error> {
            match self.header.entry(name) {
                std::collections::hash_map::Entry::Occupied(mut o) => o.get_mut().push(values),
                std::collections::hash_map::Entry::Vacant(v) => {
                    v.insert(vec![values]);
                }
            }
            Ok(())
        }
    }
}

impl From<String> for Body {
    #[inline]
    fn from(t: String) -> Self {
        Body(t.as_bytes().to_vec())
    }
}
impl From<Vec<u8>> for Body {
    #[inline]
    fn from(t: Vec<u8>) -> Self {
        Body(t)
    }
}
impl From<&'static [u8]> for Body {
    #[inline]
    fn from(t: &'static [u8]) -> Self {
        Body(t.to_vec())
    }
}
impl From<&'static str> for Body {
    #[inline]
    fn from(t: &'static str) -> Self {
        Body(t.as_bytes().to_vec())
    }
}
impl<'a> TryFrom<&'a str> for HeaderValue {
    type Error = Error;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Ok(HeaderValue(t.as_bytes().to_vec()))
    }
}
