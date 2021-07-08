
use crate::{Request, Error, HeaderName, HeaderValue};
use std::convert::TryInto;
use std::collections::HashMap;

/// A helper to perform multiple associated requests.
/// (Session-)Cookies will be handled
pub struct Session {
    headers: HashMap<HeaderName, HeaderValue>,
    #[cfg(feature = "cookies")]
    cookies: CookieStore
}
impl Session {
    #[cfg(feature = "cookies")]
    pub fn cookies(&self) {
        /*CookieStore::parse(
            &mut self,
            cookie_str: &str,
            request_url: &Url*/
    }
    pub fn new() -> Session {
        Session{
            headers: HashMap::new()
        }
    }
    /// Add a single header to all request done with this session
    pub fn set_header(mut self,
        name: impl TryInto<HeaderName, Error = Error>,
        value: impl TryInto<HeaderValue, Error = Error>) -> Result<Self, Error> {

        let val :HeaderValue = value.try_into()?;
        let name :HeaderName = name.try_into()?;
        self.headers.insert(name, val);
        
        Ok(self)
    }
    fn add_session_data(&self, req: Request) -> Request {
        for (n, v) in self.headers.iter() {
            req.set_header(n.clone(), v.clone());
        }
        #[cfg(feature = "cookies")]
        {
            let c = self.cookies.get_request_values(url)
            .map(|(n,v)| format!("{}={}", n, v))
            .collect::<Vec<_>>()
            .join("; ");
            if c.len() > 0 {
                req.set_header("Cookie", c);
            }
        }
        req
    }

    pub fn get(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::get(uri))
    }
    pub fn post(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::post(uri))
    }
    pub fn put(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::put(uri))
    }
    pub fn delete(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::delete(uri))
    }
    pub fn head(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::head(uri))
    }
    pub fn options(&mut self, uri: &str) -> Request {
        self.add_session_data(Request::options(uri))
    }
    pub fn request(&mut self, meth: &str, uri: &str) -> Result<Request, Error> {
        Request::new(meth, uri).map(|r|self.add_session_data(r))
    }
}