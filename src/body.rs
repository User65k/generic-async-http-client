use crate::imp;

/*use std::task::{Poll, Context};
use std::pin::Pin;

use futures_core::Stream;*/
/*
#[cfg(feature = "use_async_h1")]
use async_std::io::prelude::{AsyncRead, AsyncBufRead};
*/

/// A Body for the Request. You will most likely use [`Request::body`](./struct.Request.html#method.body) directly.
pub struct Body(imp::Body);
impl From<String> for Body {
    #[inline]
    fn from(t: String) -> Self {
        Body(imp::Body::from(t))
    }
}
impl From<Vec<u8>> for Body {
    #[inline]
    fn from(t: Vec<u8>) -> Self {
        Body(imp::Body::from(t))
    }
}
impl From<&'static [u8]> for Body {
    #[inline]
    fn from(t: &'static [u8]) -> Self {
        Body(imp::Body::from(t))
    }
}
impl From<&'static str> for Body {
    #[inline]
    fn from(t: &'static str) -> Self {
        Body(imp::Body::from(t))
    }
}

impl From<Body> for imp::Body {
    #[inline]
    fn from(t: Body) -> Self {
        t.0
    }
}
impl From<imp::Body> for Body {
    #[inline]
    fn from(t: imp::Body) -> Self {
        Body(t)
    }
}

/*/TODO Stream to server -> AsyncWrite for Body
impl Stream for Body {
    type Item = Result<&'a [u8], imp::Error>;

    #[cfg(feature = "use_hyper")]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Ok(self.0.poll_next(cx).map(|b|&b)?)
    }
    #[cfg(feature = "use_async_h1")]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Ok(self.0.poll_fill_buf(cx)?)
    }
}*/
/*
#[cfg(feature = "use_async_h1")]
impl AsyncRead for Body {
    #[allow(missing_doc_code_examples)]
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut buf = match self.length {
            None => buf,
            Some(length) if length == self.bytes_read => return Poll::Ready(Ok(0)),
            Some(length) => {
                let max_len = (length - self.bytes_read).min(buf.len());
                &mut buf[0..max_len]
            }
        };

        let bytes = ready!(Pin::new(&mut self.reader).poll_read(cx, &mut buf))?;
        self.bytes_read += bytes;
        Poll::Ready(Ok(bytes))
    }
}
#[cfg(feature = "use_async_h1")]
impl AsyncBufRead for Body {
    #[allow(missing_doc_code_examples)]
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&'_ [u8]>> {
        self.project().reader.poll_fill_buf(cx)
    }

    fn consume(mut self: Pin<&mut Self>, amt: usize) {
        Pin::new(&mut self.reader).consume(amt)
    }
}*/
