#[cfg(feature = "use_async_h1")]
use async_std::{
    io::prelude::{ReadExt, WriteExt},
    net::TcpStream,
};
use std::io;
#[cfg(feature = "use_hyper")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub async fn connect_via_http_prx(
    host: &str,
    port: u16,
    phost: &str,
    pport: u16,
) -> io::Result<TcpStream> {
    let mut socket = TcpStream::connect((phost, pport)).await?;
    let buf = format!(
        "CONNECT {0}:{1} HTTP/1.1\r\n\
         Host: {0}:{1}\r\n\
         {2}\
         \r\n",
        host,
        port,
        "" //TODO Auth
    )
    .into_bytes();
    socket.write(&buf).await?;
    let mut buffer = [0; 40];
    let r = socket.read(&mut buffer).await?;

    let mut read = &buffer[..r];
    if r > 12 && (read.starts_with(b"HTTP/1.1 200") || read.starts_with(b"HTTP/1.0 200")) {
        loop {
            if read.ends_with(b"\r\n\r\n") {
                return Ok(socket);
            }
            // else read more
            let r = socket.read(&mut buffer).await?;
            if r == 0 {
                break;
            }
            read = &buffer[..r];
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        host.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{assert_stream, TcpListener, spawn, block_on, listen_somewhere};
    #[test]
    fn http_proxy() {
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;

            assert_stream(
                &mut stream,
                b"CONNECT host:1234 HTTP/1.1\r\nHost: host:1234\r\n\r\n",
            )
            .await?;
            stream.write_all(b"HTTP/1.1 200 Connected\r\n\r\n").await?;
            assert_stream(
                &mut stream,
                b"n0ice",
            )
            .await?;

            Ok(true)
        }
        block_on(async {
            let (listener, pport, phost) = listen_somewhere().await?;
            let t = spawn(server(listener));

            let mut stream = connect_via_http_prx(
                "host",
                1234,
                &phost,
                pport,
            ).await?;
            stream.write_all(b"n0ice").await?;

            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
}