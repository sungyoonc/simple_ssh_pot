use log::{error, info};
use std::net::{SocketAddr, TcpListener, TcpStream};

fn main() {
    env_logger::init();
    let addr = SocketAddr::from(([127, 0, 0, 1], 7878));
    let listener = TcpListener::bind(&addr).unwrap();
    info!("Listening on {}", addr);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        handle_connection(stream);
    }
}

fn handle_connection(stream: TcpStream) {
    match stream.peer_addr() {
        Ok(remote_addr) => match stream.local_addr() {
            Ok(local_addr) => {
                info!(
                    "Connection attempt from IP={} SRC_PORT={} DEST_PORT={}",
                    remote_addr.ip(),
                    remote_addr.port(),
                    local_addr.port(),
                );
            }
            Err(err) => {
                error!("Failed to get local address: {}", err);
            }
        },
        Err(err) => {
            error!("Failed to get peer address: {}", err);
        }
    }
}
