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
    tokio::time::sleep(tokio::time::Duration::from_secs(u64::MAX)).await;
    Ok::<(), anyhow::Error>(())
}
