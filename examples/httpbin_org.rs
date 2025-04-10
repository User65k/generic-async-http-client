use generic_async_http_client::{Error, Request};
use serde::Deserialize;
use std::collections::HashMap;

#[cfg(feature = "use_async_h1")]
pub(crate) fn block_on(fut: impl futures::Future<Output = Result<(), Error>>) -> Result<(), Error> {
    async_std::task::block_on(fut)
}
#[cfg(feature = "use_hyper")]
pub(crate) fn block_on(fut: impl futures::Future<Output = Result<(), Error>>) -> Result<(), Error> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt")
        .block_on(fut)
}
#[cfg(not(any(feature = "use_hyper",feature = "use_async_h1")))]
pub(crate) fn block_on(fut: impl futures::Future<Output = Result<(), Error>>) -> Result<(), Error> {
    futures::executor::block_on(fut)
}

#[derive(Deserialize)]
pub struct HttpbinOrgHeaders {
    pub headers: HashMap<String, String>,
}

fn main() -> Result<(), Error> {
    block_on(async {
        let req = Request::get("https://httpbin.org/headers").set_header("Test", "yeha")?;
        let mut resp = req.exec().await?;
        //println!("{}", resp.text().await?);
        assert_eq!(resp.status_code(), 200);
        let headers: HttpbinOrgHeaders = resp.json().await?;
        assert_eq!(headers.headers.get("Test").map(|s|s.as_str()), Some("yeha"));
        Ok(())
    })
}
