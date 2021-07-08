#[cfg(feature = "use_async_h1")]
use async_std::{net::TcpStream,
    io::{prelude::{ReadExt, WriteExt}}
};
#[cfg(feature = "use_hyper")]
use tokio::{
    net::{TcpStream},
    io::{AsyncReadExt, AsyncWriteExt}
};
use std::io;

pub async fn connect_via_http_prx(
    host: &str, port: u16,
    phost: &str, pport: u16) -> io::Result<TcpStream> {

    let mut socket = TcpStream::connect((phost, pport)).await?;
    let buf = format!(
        "CONNECT {0}:{1} HTTP/1.1\r\n\
         Host: {0}:{1}\r\n\
         {2}\
         \r\n",
        host,
        port,
        ""  //TODO Auth
    ).into_bytes();
    socket.write(&buf).await?;
    let mut buffer = [0; 40];
    let r = socket.read(&mut buffer).await?;

    let mut read = &buffer[..r];
    if r > 12{
        if read.starts_with(b"HTTP/1.1 200") || read.starts_with(b"HTTP/1.0 200") {
            loop {
                if read.ends_with(b"\r\n\r\n") {
                    return Ok(socket);
                }
                // else read more
                let r = socket.read(&mut buffer).await?;
                if r==0 {
                    break;
                }
                read = &buffer[..r];
            }
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, format!("{}",host)))
}