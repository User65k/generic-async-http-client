#[cfg(feature = "use_async_h1")]
use async_std::{
    io::prelude::{ReadExt, WriteExt},
    net::TcpStream,
};
use std::io;
use std::net::{IpAddr, ToSocketAddrs};
#[cfg(feature = "use_hyper")]
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub async fn connect_via_socks_prx(
    host: &str,
    port: u16,
    phost: &str,
    pport: u16,
    dns_via_prx: bool,
) -> io::Result<TcpStream> {
    let mut buf = Vec::with_capacity(22);
    buf.push(5 as u8);
    buf.push(1);
    buf.push(0);

    if dns_via_prx {
        match host.parse::<IpAddr>() {
            Ok(ip) => match ip {
                IpAddr::V4(ip) => {
                    buf.push(1);
                    buf.extend_from_slice(&ip.octets())
                }
                IpAddr::V6(ip) => {
                    buf.push(4);
                    buf.extend_from_slice(&ip.octets())
                }
            },
            Err(_) => {
                if host.len() > 255 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "host name too long",
                    ));
                }
                buf.push(3);
                buf.push(host.len() as u8);
                buf.extend_from_slice(host.as_bytes());
            }
        }
    } else {
        let a = (host, port)
            .to_socket_addrs()?
            .next()
            .ok_or(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not resolve the host",
            ))?;
        match a.ip() {
            IpAddr::V4(ip) => {
                buf.push(1);
                buf.extend_from_slice(&ip.octets())
            }
            IpAddr::V6(ip) => {
                buf.push(4);
                buf.extend_from_slice(&ip.octets())
            }
        }
    }
    buf.extend_from_slice(&port.to_be_bytes());
    let mut socket = TcpStream::connect((phost, pport)).await?;
    socket.write(&buf).await?;
    let n = socket.read(&mut buf[..5]).await?;
    if n > 4 && buf[0] == 5 && buf[1] == 0 && buf[2] == 0 {
        let m = match buf[3] {
            1 => 10,
            4 => 22,
            3 => 7 + buf[4] as usize,
            _ => 0, //leads to error below
        };
        match m.checked_sub(n) {
            Some(r) if r > 0 => {
                buf.resize(m, 0);
                socket.read_exact(&mut buf[n..m]).await?;
            }
            Some(_) => {} //0
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionRefused,
                    "socks error",
                ))
            } //already read to much
        }
        Ok(socket)
    } else {
        Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "socks error",
        ))
    }
}
