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
    socket.write_all(b"\x05\x01\0").await?;//client auth methods: [no auth]
    let mut auth = [0, 0];
    socket.read_exact(&mut auth).await?;
    if auth[0]==5 && auth[1]==0 {
        //proxy wants no auth
    }else{
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "socks error",
        ));
    }
    socket.write_all(&buf).await?;
    let bytes_read = socket.read(&mut buf[..5]).await?;
    if bytes_read > 4 && buf[0] == 5 && buf[1] == 0 && buf[2] == 0 {
        let socks_header_len = match buf[3] {
            1 => 10,
            4 => 22,
            3 => 7 + buf[4] as usize,
            _ => 0, //leads to error below
        };
        match socks_header_len.checked_sub(bytes_read) {
            Some(missing_bytes) if missing_bytes > 0 => {
                buf.resize(socks_header_len, 0);
                socket.read_exact(&mut buf[bytes_read..socks_header_len]).await?;
            }
            Some(_) => {}   //0
            None => {       //already read to much
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "socks error",
                ))
            }
        }
        Ok(socket)
    } else {
        Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "socks error",
        ))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{assert_stream, TcpListener, spawn, block_on};
    #[test]
    fn socks5h() {
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;

            assert_stream(
                &mut stream,
                b"\x05\x01\0",//client auth methods: [no auth]
            )
            .await?;
            stream.write_all(b"\x05\0").await?;//proxy wants no auth
            assert_stream(
                &mut stream,
                b"\x05\x01\0\x03\x04host\x12\x34", //version connect reserved dns len host port
            )
            .await?;
            stream.write_all(b"\x05\0\0\x03\x04host\x12\x34").await?; //version ok reserved dns len host port
            assert_stream(
                &mut stream,
                b"n0ice",
            )
            .await?;

            Ok(true)
        }
        block_on(async {
            let listener = TcpListener::bind("127.0.0.1:61081").await?;
            let t = spawn(server(listener));

            let mut stream = connect_via_socks_prx(
                "host",
                0x1234,
                "127.0.0.1",
                61081,
                true,
            ).await?;
            stream.write_all(b"n0ice").await?;

            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn socks5_ip4() {
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;

            assert_stream(
                &mut stream,
                b"\x05\x01\0",//client auth methods: [no auth]
            )
            .await?;
            stream.write_all(b"\x05\0").await?;//proxy wants no auth
            assert_stream(
                &mut stream,
                b"\x05\x01\0\x01\x7f\0\0\x01\x12\x34", //version connect reserved dns len host port
            )
            .await?;
            stream.write_all(b"\x05\0\0\x01\x7f\0\0\x01\x12\x34").await?; //version ok reserved dns len host port
            assert_stream(
                &mut stream,
                b"n0ice",
            )
            .await?;

            Ok(true)
        }
        block_on(async {
            let listener = TcpListener::bind("127.0.0.1:61082").await?;
            let t = spawn(server(listener));

            let mut stream = connect_via_socks_prx(
                "127.0.0.1",
                0x1234,
                "127.0.0.1",
                61082,
                true,
            ).await?;
            stream.write_all(b"n0ice").await?;

            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
    #[test]
    fn socks5_ip6() {
        async fn server(listener: TcpListener) -> std::io::Result<bool> {
            let (mut stream, _) = listener.accept().await?;

            assert_stream(
                &mut stream,
                b"\x05\x01\0",//client auth methods: [no auth]
            )
            .await?;
            stream.write_all(b"\x05\0").await?;//proxy wants no auth
            assert_stream(
                &mut stream,
                b"\x05\x01\0\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\x12\x34", //version connect reserved dns len host port
            )
            .await?;
            stream.write_all(b"\x05\0\0\x04\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01\x12\x34").await?; //version ok reserved dns len host port
            assert_stream(
                &mut stream,
                b"n0ice",
            )
            .await?;

            Ok(true)
        }
        block_on(async {
            let listener = TcpListener::bind("127.0.0.1:61083").await?;
            let t = spawn(server(listener));

            let mut stream = connect_via_socks_prx(
                "::1",
                0x1234,
                "127.0.0.1",
                61083,
                true,
            ).await?;
            stream.write_all(b"n0ice").await?;

            assert!(t.await?, "not cool");
            Ok(())
        })
        .unwrap();
    }
}
