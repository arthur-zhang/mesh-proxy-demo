use std::net::SocketAddr;

use nix::sys::socket;
use nix::sys::socket::sockopt;
use tokio::net::{TcpSocket, TcpStream};

const PORT: u16 = 15006;
const LISTENER_BACKLOG: u32 = 65535;


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listen_addr = format!("0.0.0.0:{}", PORT).parse().unwrap();
    println!("Listening on: {}", listen_addr);
    let socket = TcpSocket::new_v4()?;

    #[cfg(any(target_os = "linux"))]
    socket::setsockopt(&socket, sockopt::IpTransparent, &true)?;

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
    // client_real_ip: 172.100.36.0
    let client_real_ip = downstream_conn.peer_addr()?.ip();
    // upstream_addr: 172.100.1.2:8080
    let upstream_addr = downstream_conn.local_addr()?;
    println!("start connect to upstream: {}, from {}", upstream_addr, client_real_ip);
    let socket = TcpSocket::new_v4()?;

    #[cfg(any(target_os = "linux"))]
    socket::setsockopt(&socket, sockopt::IpTransparent, &true)?;

    #[cfg(any(target_os = "linux"))]
    socket::setsockopt(&socket, sockopt::Mark, &0x539)?;

    let bind_addr = SocketAddr::new(client_real_ip, 0);

    // bind src ip before connect
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
