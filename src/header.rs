use std::borrow::Borrow;
use std::convert::TryFrom;

use crate::imp;

/// A HTTP Header Name
///
/// It can be converted from `&[u8]` and `&str`.
/// You can obtain `&str` and `&[u8]` references and compare with str.
/// ```
/// # use std::convert::TryInto;
/// # use generic_async_http_client::HeaderName;
/// let hn: HeaderName = "test".try_into().unwrap();
/// assert!(hn=="test");
/// let s: &str = hn.as_ref();
/// assert!(s.is_ascii());
/// let b: &[u8] = hn.as_ref();
/// assert!(b.contains(&b's'));
/// ```
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct HeaderName(imp::HeaderName);
impl<'a> TryFrom<&'a str> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn try_from(t: &'a str) -> Result<HeaderName, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_string(t.to_string())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::try_from(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Ok(HeaderName(imp::HeaderName(t.to_string())));
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return t.to_vec().try_into();
    }
}
impl TryFrom<Vec<u8>> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn try_from(t: Vec<u8>) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(&t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Ok(HeaderName(imp::HeaderName(String::from_utf8(t).unwrap())));
    }
}

impl From<HeaderName> for imp::HeaderName {
    #[inline]
    fn from(t: HeaderName) -> Self {
        t.0
    }
}
impl<'a> From<&'a imp::HeaderName> for &'a HeaderName {
    #[inline]
    fn from(t: &'a imp::HeaderName) -> Self {
        //safe because repr(transparent)
        unsafe { std::mem::transmute(t) }
    }
}
impl AsRef<str> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &str {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str();
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0.as_str();
    }
}

impl AsRef<[u8]> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str().as_bytes();
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0.as_bytes();
    }
}

impl Borrow<str> for HeaderName {
    #[inline]
    fn borrow(&self) -> &str {
        #[cfg(feature = "use_hyper")]
        return self.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str();
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0.as_str();
    }
}
impl PartialEq<str> for HeaderName {
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0 == other;
    }
}
impl PartialEq<&str> for HeaderName {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

/// A HTTP Header Value
///
/// It can be converted from `&[u8]` and `&str`.
/// You can obtain a `&[u8]` reference and compare with str and `&[u8]`.
/// You can also convert it to `String` if it is valid utf-8.
/// ```
/// # use std::convert::TryInto;
/// # use generic_async_http_client::HeaderValue;
/// let hv: HeaderValue = b"test"[..].try_into().unwrap();
/// assert!(hv=="test");
/// assert!(hv.as_ref().contains(&b's'));
/// let val: String = hv.try_into().unwrap();
/// ```
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct HeaderValue(imp::HeaderValue);

impl HeaderValue {
    /// Try to parse the HeaderValue as some Type (implementing FromStr)
    /// ```
    /// # use std::convert::TryInto;
    /// # use generic_async_http_client::HeaderValue;
    /// let hv: HeaderValue = b"4"[..].try_into().unwrap();
    /// let four: u32 = hv.parse().unwrap();
    /// ```
    pub fn parse<T: std::str::FromStr>(&self) -> Option<T> {
        self.as_str().ok()?.parse::<T>().ok()
    }
    // Get a `&str` reference of this HeaderValue
    pub fn as_str(&self) -> Result<&str, std::str::Utf8Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(self.0.as_str());
        #[cfg(not(feature = "use_async_h1"))]
        std::str::from_utf8(self.as_ref())
    }
}
impl<'a> TryFrom<&'a str> for HeaderValue {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Ok(HeaderValue(imp::HeaderValue::try_from(t)?))
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderValue {
    type Error = imp::Error;
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Ok(HeaderValue(imp::HeaderValue(t.to_vec())));
    }
}

impl From<HeaderValue> for imp::HeaderValue {
    #[inline]
    fn from(t: HeaderValue) -> Self {
        t.0
    }
}
impl From<imp::HeaderValue> for HeaderValue {
    #[inline]
    fn from(t: imp::HeaderValue) -> Self {
        HeaderValue(t)
    }
}
impl<'a> From<&'a imp::HeaderValue> for &'a HeaderValue {
    #[inline]
    fn from(t: &'a imp::HeaderValue) -> Self {
        //safe because repr(transparent)
        unsafe { std::mem::transmute(t) }
    }
}
impl AsRef<[u8]> for HeaderValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str().as_bytes();
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0.as_ref();
    }
}
impl std::convert::TryInto<String> for HeaderValue {
    type Error = std::string::FromUtf8Error;
    #[cfg_attr(feature = "use_async_h1", allow(unreachable_code))]
    fn try_into(self) -> Result<String, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(self.0.as_str().to_string());
        String::from_utf8(self.as_ref().to_vec())
    }
}
impl std::convert::TryInto<String> for &HeaderValue {
    type Error = std::string::FromUtf8Error;
    #[cfg_attr(feature = "use_async_h1", allow(unreachable_code))]
    fn try_into(self) -> Result<String, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(self.0.as_str().to_string());
        String::from_utf8(self.as_ref().to_vec())
    }
}
impl PartialEq<str> for HeaderValue {
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other.as_bytes();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0 == other.as_bytes();
    }
}
impl PartialEq<&str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.eq(*other)
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[cfg_attr(
        not(any(feature = "use_hyper", feature = "use_async_h1")),
        allow(unused_variables)
    )]
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.as_ref() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return self.0 .0 == other;
    }
}
impl PartialEq<&[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &&[u8]) -> bool {
        self.eq(*other)
    }
}
