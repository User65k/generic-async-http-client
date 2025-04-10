use std::{cell::RefCell, collections::HashMap, fmt::Debug, iter};

use crate::{
    imp::{Error as ErrorImp, HeaderName as HNameImp, HeaderValue as HValImp},
    HeaderName, HeaderValue,
};

thread_local! {
    static VALIDATOR: RefCell<Mock> = RefCell::new(Mock{v: HashMap::new()});
}
/// Mock Responses and validate Requests.
/// All responses of a thread will be checked against the Mock if **at least one Endpoint is Mocked**.
/// Otherwise a normal web request is done
///
/// ```
/// # use futures::executor::block_on;
/// use generic_async_http_client::{Request, Error, Mock};
/// # block_on(async {
///      Mock::update("GET", "http://example.com/", |r|{
///          r.set_response(200, "mock");
///          r.add_response_header("test", "jo").unwrap();
///      });
///
///      let mut resp = Request::get("http://example.com/").exec().await?;
///      assert_eq!(resp.header("test").unwrap(), "jo");
///      assert_eq!(resp.text().await?, "mock");
/// #     Result::<(),Error>::Ok(())
/// # }).unwrap();
/// ```
///
pub struct Mock {
    v: HashMap<Endpoint, MockedEndpoint>,
}
/// A mocked HTTP Endpoint.
///
/// It asserts the request body and headers
/// and returns a response
pub struct MockedEndpoint {
    req_body: Option<BodyMock>,
    res_body: BodyMock,
    req_header: Option<HashMap<MockHeaderName, Vec<MockHeaderValue>>>,
    res_header: HashMap<HNameImp, Vec<HValImp>>,
    res_code: u16,
}
impl MockedEndpoint {
    pub fn new(res_code: u16) -> Self {
        Self {
            req_body: None,
            res_body: BodyMock(Vec::new()),
            req_header: None,
            res_header: HashMap::new(),
            res_code,
        }
    }
    pub fn assert_body<B: Into<BodyMock>>(&mut self, body: B) {
        self.req_body = Some(body.into());
    }
    pub fn set_response<B: Into<BodyMock>>(&mut self, code: u16, body: B) {
        self.res_body = body.into();
        self.res_code = code;
    }
    pub fn set_response_status(&mut self, code: u16) {
        self.res_code = code;
    }
    /// Add a single header to the response
    /// If the map did have this key present, the new value is pushed to the end of the list of values
    pub fn add_response_header<N, V, E1, E2>(&mut self, name: N, value: V) -> Result<(), ErrorImp>
    where
        N: TryInto<HeaderName, Error = E1>,
        V: TryInto<HeaderValue, Error = E2>,
        ErrorImp: From<E1>,
        ErrorImp: From<E2>,
    {
        let value: HeaderValue = value.try_into()?;
        let name: HeaderName = name.try_into()?;
        let value: HValImp = value.into();
        let name: HNameImp = name.into();
        self.res_header.entry(name).or_default().push(value);

        Ok(())
    }
    /// Check a single header of the request
    /// If the map did have this key present, the new value is pushed to the end of the list of values
    pub fn add_header_assertion<N, V, E1, E2>(&mut self, name: N, value: V) -> Result<(), ErrorImp>
    where
        N: TryInto<HeaderName, Error = E1>,
        V: TryInto<HeaderValue, Error = E2>,
        ErrorImp: From<E1>,
        ErrorImp: From<E2>,
    {
        let value: HeaderValue = value.try_into()?;
        let name: HeaderName = name.try_into()?;
        let value: HValImp = value.into();
        let name: HNameImp = name.into();
        self.req_header
            .get_or_insert_default()
            .entry(name.into())
            .or_default()
            .push(value.into());

        Ok(())
    }
}
/// Uppercase Method and Full URI (scheme, authority, path, query)
pub type Endpoint = (String, String);
impl Mock {
    /// Add a Mocked endpoint
    ///
    /// `meth` and `uri` must be an exact match
    pub fn add(meth: &str, uri: &str, mep: MockedEndpoint) {
        VALIDATOR.with_borrow_mut(|v| {
            v.v.insert((meth.to_uppercase().to_string(), uri.to_string()), mep);
        });
    }
    /// Add or update a Mocked endpoint
    ///
    /// `meth` and `uri` must be an exact match
    pub fn update<F>(meth: &str, uri: &str, f: F)
    where
        F: FnOnce(&mut MockedEndpoint),
    {
        VALIDATOR.with_borrow_mut(|v| {
            let e =
                v.v.entry((meth.to_uppercase().to_string(), uri.to_string()))
                    .or_insert(MockedEndpoint {
                        req_body: None,
                        res_body: BodyMock(Vec::new()),
                        req_header: None,
                        res_header: HashMap::new(),
                        res_code: 503,
                    });
            f(e);
        });
    }
    pub(crate) fn check<R>(mut req: impl MockedRequest) -> Result<Resp<R>, MockErr> {
        VALIDATOR.with_borrow(|v| match v.v.get(&req.endpoint()) {
            None => Err(MockErr::NoResponseProvided),
            Some(v) => {
                if let Some(b) = v.req_body.as_ref() {
                    if let Err(e) = req.assert_body_bytes(&b.0) {
                        return Err(MockErr::BodyAssertionFailed(e));
                    }
                }
                if let Some(check_header) = &v.req_header {
                    for (h, v) in check_header {
                        match req.get_headers(&h.0) {
                            Some(hv) if &hv == v => {}
                            Some(wrong) => {
                                return Err(MockErr::HeaderAssertionFailed(format!(
                                    "{} is {:?} not {:?}",
                                    h.0, wrong, v
                                )))
                            }
                            None => {
                                return Err(MockErr::HeaderAssertionFailed(format!(
                                    "{} is None not {:?}",
                                    h.0, v
                                )))
                            }
                        }
                    }
                }
                Ok(Resp::Fake(MockResp {
                    code: v.res_code,
                    body: v.res_body.clone(),
                    header: v.res_header.clone(),
                }))
            }
        })
    }
    pub fn uses_mock() -> bool {
        !VALIDATOR.with_borrow(|v| v.v.is_empty())
    }
}
#[derive(Debug)]
pub enum MockErr {
    NoResponseProvided,
    BodyAssertionFailed(Vec<u8>),
    HeaderAssertionFailed(String),
}
impl std::fmt::Display for MockErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
impl std::error::Error for MockErr {}

pub trait MockedRequest {
    /// on error, return full body
    fn assert_body_bytes(&mut self, should_be: &[u8]) -> Result<(), Vec<u8>>;
    fn get_headers(&self, name: &str) -> Option<Vec<MockHeaderValue>>;
    fn endpoint(&self) -> Endpoint;
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct MockHeaderName(String);
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct MockHeaderValue(Vec<u8>);

impl std::fmt::Debug for MockHeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut input = self.0.as_ref();
        loop {
            match std::str::from_utf8(input) {
                Ok(valid) => {
                    f.write_str(valid)?;
                    break;
                }
                Err(error) => {
                    let (valid, after_valid) = input.split_at(error.valid_up_to());
                    f.write_str(unsafe { std::str::from_utf8_unchecked(valid) })?;
                    let inv = if let Some(invalid_sequence_length) = error.error_len() {
                        &after_valid[..invalid_sequence_length]
                    } else {
                        after_valid
                    };
                    for c in inv {
                        f.write_fmt(format_args!("\\x{:02x}", c))?;
                    }
                    if let Some(invalid_sequence_length) = error.error_len() {
                        input = &after_valid[invalid_sequence_length..]
                    } else {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

impl<'a> TryFrom<&'a str> for MockHeaderValue {
    type Error = std::convert::Infallible;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Ok(MockHeaderValue(t.as_bytes().to_vec()))
    }
}
#[derive(Debug, Clone)]
pub struct BodyMock(pub(crate) Vec<u8>);
impl From<String> for BodyMock {
    #[inline]
    fn from(t: String) -> Self {
        BodyMock(t.into_bytes())
    }
}
impl From<Vec<u8>> for BodyMock {
    #[inline]
    fn from(t: Vec<u8>) -> Self {
        BodyMock(t)
    }
}
impl From<&'static [u8]> for BodyMock {
    #[inline]
    fn from(t: &'static [u8]) -> Self {
        BodyMock(t.to_vec())
    }
}
impl From<&'static str> for BodyMock {
    #[inline]
    fn from(t: &'static str) -> Self {
        BodyMock(t.as_bytes().to_vec())
    }
}

impl From<HNameImp> for MockHeaderName {
    fn from(value: HNameImp) -> Self {
        let v: &crate::HeaderName = (&value).into();
        let s: &str = v.as_ref();
        Self(s.to_string())
    }
}
impl From<HValImp> for MockHeaderValue {
    fn from(value: HValImp) -> Self {
        let v: crate::HeaderValue = (value).into();
        Self(v.as_ref().to_vec())
    }
}

#[allow(private_interfaces)]
pub enum Resp<R> {
    Real(R),
    Fake(crate::mock::MockResp),
}
struct MockResp {
    code: u16,
    body: BodyMock,
    header: HashMap<HNameImp, Vec<HValImp>>,
}
impl<R> crate::response::Responses for Resp<R>
where
    R: crate::response::Responses,
{
    fn status(&self) -> u16 {
        match self {
            Resp::Real(resp) => resp.status(),
            Resp::Fake(resp) => resp.code,
        }
    }
    fn status_str(&self) -> &'static str {
        match self {
            Resp::Real(resp) => resp.status_str(),
            Resp::Fake(_) => "",
        }
    }
    async fn json<D: serde::de::DeserializeOwned>(&mut self) -> Result<D, ErrorImp> {
        match self {
            Resp::Real(resp) => resp.json().await,
            Resp::Fake(resp) => Ok(serde_json::from_slice(&resp.body.0)?),
        }
    }
    async fn bytes(&mut self) -> Result<Vec<u8>, ErrorImp> {
        match self {
            Resp::Real(resp) => resp.bytes().await,
            Resp::Fake(resp) => Ok(resp.body.0.clone()),
        }
    }
    async fn string(&mut self) -> Result<String, ErrorImp> {
        match self {
            Resp::Real(resp) => resp.string().await,
            Resp::Fake(resp) => Ok(String::from_utf8_lossy(&resp.body.0).to_string()),
        }
    }
    fn get_header(&self, name: HNameImp) -> Option<&HValImp> {
        match self {
            Resp::Real(resp) => resp.get_header(name),
            Resp::Fake(resp) => resp.header.get(&name)?.first(),
        }
    }
    fn get_headers(&self, name: HNameImp) -> impl Iterator<Item = &HValImp> {
        match self {
            Resp::Real(resp) => Vec::from_iter(resp.get_headers(name)).into_iter(),
            Resp::Fake(resp) => {
                if let Some(vals) = resp.header.get(&name) {
                    Vec::from_iter(vals.iter()).into_iter()
                } else {
                    vec![].into_iter()
                }
            }
        }
    }
    fn header_iter(&self) -> impl Iterator<Item = (&HNameImp, &HValImp)> {
        match self {
            Resp::Real(resp) => {
                Vec::from_iter(resp.header_iter())
            }
            Resp::Fake(resp) => Vec::from_iter(
                resp.header
                    .iter()
                    .flat_map(|(n, v)| iter::repeat(n).zip(v.iter())),
            ),
        }
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use futures::executor::block_on;
    #[test]
    fn mock_it() {
        block_on(async {
            Mock::update("GET", "http://example.com/", |r| {
                r.set_response(200, "mock");
                r.add_response_header("test", "jo").unwrap();
            });

            let mut resp = crate::Request::get("http://example.com/").exec().await?;
            assert_eq!(resp.header("test").unwrap(), "jo");
            assert_eq!(resp.header("test").map(|s| s.as_ref()), Some(&b"jo"[..]));
            assert_eq!(
                std::convert::TryInto::<String>::try_into(resp.header("test").unwrap()).unwrap(),
                "jo".to_string()
            );
            assert_eq!(resp.text().await?, "mock");
            Result::<(), Error>::Ok(())
        })
        .unwrap();
    }
    #[test]
    fn mock_it_different() {
        block_on(async {
            Mock::update("GET", "http://example.com/", |r| {
                r.set_response(201, "different");
            });

            let mut resp = crate::Request::get("http://example.com/").exec().await?;
            assert_eq!(resp.text().await?, "different");
            Result::<(), Error>::Ok(())
        })
        .unwrap();
    }
    #[test]
    fn error_on_miss() {
        block_on(async {
            Mock::update("GET", "anything", |_r| {});

            let err = crate::Request::get("http://example.com/")
                .exec()
                .await
                .expect_err("should fail");

            assert!(matches!(err, Error::Mock(MockErr::NoResponseProvided)));
            Result::<(), Error>::Ok(())
        })
        .unwrap();
    }
    #[test]
    fn assert_header() {
        block_on(async {
            Mock::update("GET", "http://example.com/", |r|{
                r.add_header_assertion("test", "jo").unwrap();
                r.set_response(200, "mock");
            });
            let err = crate::Request::get("http://example.com/").set_header("test", "123")?.exec().await.expect_err("should fail");
            assert!(matches!(err, Error::Mock(MockErr::HeaderAssertionFailed(m)) if m == "test is [123] not [jo]"));
            Result::<(),Error>::Ok(())
        }).unwrap();
    }
    #[test]
    fn multiple_headers() {
        block_on(async {
            Mock::update("GET", "http://example.com/", |r| {
                r.add_header_assertion("test", "jo").unwrap();
                r.add_header_assertion("test", "1").unwrap();

                r.set_response(200, "mock");
                r.add_response_header("ret", "jo").unwrap();
                r.add_response_header("ret", "1").unwrap();
            });
            let resp = crate::Request::get("http://example.com/")
                .set_header("test", "jo")?
                .add_header("test", "1")?
                .exec()
                .await?;
            let mut hv = resp.all_header("ret")?;
            assert_eq!(hv.next().unwrap(), "jo");
            assert_eq!(hv.next().unwrap(), "1");

            let mut allh = resp.headers();
            let h = allh.next().unwrap();
            assert_eq!(h.0, "ret");
            assert_eq!(h.1, "jo");
            let h = allh.next().unwrap();
            assert_eq!(h.0, "ret");
            assert_eq!(h.1, "1");

            Result::<(), Error>::Ok(())
        })
        .unwrap();
    }
}
