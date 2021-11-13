use std::convert::TryFrom;
use std::borrow::Borrow;

use crate::imp;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct HeaderName(imp::HeaderName);
impl<'a> TryFrom<&'a str> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn try_from(t: &'a str) -> Result<HeaderName, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_string(t.to_string())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::try_from(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Err(imp::Error{})
    }
}
impl<'a> TryFrom<&'a [u8]> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Err(imp::Error{})
    }
}
impl TryFrom<Vec<u8>> for HeaderName {
    type Error = imp::Error;
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn try_from(t: Vec<u8>) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(t)?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderName(imp::HeaderName::from_bytes(&t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Err(imp::Error{})
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
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return "";
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
        return &[];
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
        return "";
    }
}
impl PartialEq<str> for HeaderName {
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return false;
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
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn try_from(t: &'a [u8]) -> Result<Self, Self::Error> {
        #[cfg(feature = "use_async_h1")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t.to_vec())?));
        #[cfg(feature = "use_hyper")]
        return Ok(HeaderValue(imp::HeaderValue::from_bytes(t)?));
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return Err(imp::Error{})
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
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return &[];
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
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn eq(&self, other: &str) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other.as_bytes();
        #[cfg(feature = "use_async_h1")]
        return self.0.as_str() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return false;
    }
}

impl PartialEq<[u8]> for HeaderValue {
    #[cfg_attr(not(any(feature = "use_hyper", feature = "use_async_h1")), allow(unused_variables))]
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        #[cfg(feature = "use_hyper")]
        return self.0 == other;
        #[cfg(feature = "use_async_h1")]
        return self.as_ref() == other;
        #[cfg(not(any(feature = "use_hyper", feature = "use_async_h1")))]
        return false;
    }
}
