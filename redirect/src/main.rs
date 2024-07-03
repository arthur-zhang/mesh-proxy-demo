use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::os::fd::AsFd;
use std::str::FromStr;

use nix::sys::socket;
use nix::unistd::{Gid, setgid, setuid, Uid};
use tokio::net::{TcpSocket, TcpStream};

const PORT: u16 = 15006;
const LISTENER_BACKLOG: u32 = 65535;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setgid(Gid::from_raw(1337))?;
    setuid(Uid::from_raw(1337))?;
    let listen_addr = format!("0.0.0.0:{}", PORT).parse().unwrap();
    println!("Listening on: {}", listen_addr);
    let socket = TcpSocket::new_v4()?;

    socket.bind(listen_addr)?;
    let listener = socket.listen(LISTENER_BACKLOG)?;

    while let Ok((mut downstream_conn, _)) = listener.accept().await {
        println!("accept new connection, peer[{:?}]->local[{:?}]", downstream_conn.peer_addr()?, downstream_conn.local_addr()?);

        tokio::spawn(async move {
            let result = handle_connection(downstream_conn).await;
            match result {
                Ok(_) => {
                    println!("connection closed");
                }
                Err(err) => {
                    println!("connection closed with error: {:?}", err);
                }
            }
        });
    }

    Ok(())
}

async fn handle_connection(mut downstream_conn: TcpStream) -> anyhow::Result<()> {
    let sock_fd = downstream_conn.as_fd();

    let socket_info = socket::getsockopt(&sock_fd, socket::sockopt::OriginalDst)?;
    let origin_dst_port = <u16>::from_be(socket_info.sin_port);
    let origin_dst_addr = {
        let addr = <u32>::from_be(socket_info.sin_addr.s_addr);
        Ipv4Addr::from(addr)
    };

    println!("origin dst addr: {:?}:{:?}", origin_dst_addr, origin_dst_port);

    let upstream_addr = SocketAddr::new(IpAddr::V4(origin_dst_addr), origin_dst_port);

    // bind to localhost
    let socket = TcpSocket::new_v4()?;
    let bind_addr = SocketAddr::new(IpAddr::from_str("127.0.0.1").unwrap(), 0);
    match socket.bind(bind_addr) {
        Ok(_) => {
            println!("bind to: {} success", bind_addr);
        }
        Err(err) => {
            println!("bind to: {} failed, err: {:?}", bind_addr, err);
            return Err(err.into());
        }
    };
    let mut upstream_conn = socket.connect(upstream_addr).await?;
    println!("connected to upstream, local[{:?}]->peer[{:?}]", upstream_conn.local_addr()?, upstream_conn.peer_addr()?);
    tokio::io::copy_bidirectional(&mut downstream_conn, &mut upstream_conn).await?;
    Ok::<(), anyhow::Error>(())
}
