use std::convert::TryFrom;
use std::borrow::Borrow;

use crate::imp;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct HeaderName(imp::HeaderName);
impl<'a> TryFrom<&'a str> for HeaderName {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: &'a str) -> Result<HeaderName, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_string(t.to_string())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::try_from(t)?));
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderName {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
    }
}
impl TryFrom<Vec<u8>> for HeaderName {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: Vec<u8>) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(&t)?));
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
        unsafe{std::mem::transmute(t)}
    }
}
impl AsRef<str> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &str {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str();
    }
}

impl AsRef<[u8]> for HeaderName {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str().as_bytes();
    }
}

impl Borrow<str> for HeaderName {
    #[inline]
    fn borrow(&self) -> &str {
        #[cfg(feature = "use_hyper")]
        return self.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str();
    }
}
impl PartialEq<str> for HeaderName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct HeaderValue(imp::HeaderValue);
impl<'a> TryFrom<&'a str> for HeaderValue {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: &'a str) -> Result<Self, Self::Error> {
        Ok(HeaderValue(imp::HeaderValue::try_from(t)?))
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderValue {
    type Error = imp::Error;
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t)?));
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
        unsafe{std::mem::transmute(t)}
    }
}
impl AsRef<[u8]> for HeaderValue {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        #[cfg(feature = "use_hyper")]
        return self.0.as_ref();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str().as_bytes();
    }
}
impl std::convert::TryInto<String> for HeaderValue {
    type Error = std::string::FromUtf8Error;
    fn try_into(self) -> Result<String, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(self.0.as_str().to_string());
        String::from_utf8(self.as_ref().to_vec())
    }
}
impl std::convert::TryInto<String> for &HeaderValue {
    type Error = std::string::FromUtf8Error;
    fn try_into(self) -> Result<String, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(self.0.as_str().to_string());
        String::from_utf8(self.as_ref().to_vec())
    }
}
impl PartialEq<str> for HeaderValue {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other.as_bytes();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.as_ref() == other;
    }
}