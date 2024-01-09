#[macro_use]
extern crate log;

extern crate simplelog;

use simplelog::{CombinedLogger, TermLogger, WriteLogger};
use std::fs::File;

use std::collections::HashMap;

use config::Config;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};

fn main() {
    CombinedLogger::init(vec![
        TermLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto,
        ),
        WriteLogger::new(
            simplelog::LevelFilter::Info,
            simplelog::Config::default(),
            File::create("simple_ssh_pot.log").unwrap(),
        ),
    ])
    .unwrap();

    let mut settings = Config::builder()
        .add_source(config::File::with_name("config.toml"))
        .build()
        .expect("Failed to load config")
        .try_deserialize::<HashMap<String, String>>()
        .unwrap();

    debug!("Loaded config: {:?}", settings);
    settings
        .entry(String::from("port"))
        .or_insert(String::from("7878"));

    let port: u16 = settings
        .get(&String::from("port"))
        .unwrap()
        .parse()
        .expect("Failed to parse port");
    let bind_ip: Ipv4Addr = settings
        .get(&String::from("bind"))
        .unwrap()
        .parse()
        .expect("Failed to parse bind ip");

    let addr = SocketAddr::from((bind_ip, port));
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
