#[macro_use]
extern crate log;

extern crate simplelog;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use reqwest::header::HeaderMap;
use simplelog::{CombinedLogger, TermLogger, WriteLogger};
use tokio::net::{TcpListener, TcpStream};

use config::Config;

use cached::proc_macro::cached;
use cached::TimedCache;

#[derive(Hash, PartialEq, Eq, Clone)]
struct AbuseIPDBConfig {
    enabled: bool,
    url: String,
    key: String,
    categories: Vec<String>,
    comment: AbuseIPDBCommentConfig,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct AbuseIPDBCommentConfig {
    hostname: String,
    display_port: String,
    message: String,
}

#[derive(Clone)]
struct Configuration {
    bind: Ipv4Addr,
    port: u16,
    abuseipdb: AbuseIPDBConfig,
}

#[tokio::main]
async fn main() -> io::Result<()> {
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

    let config = load_config().expect("Failed to load config");

    let addr = SocketAddr::from((config.bind, config.port));
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on {}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        handle_connection(stream, &config).await;
    }
}

fn load_config() -> Result<Configuration, config::ConfigError> {
    let config_builder = Config::builder()
        .set_default("port", 8080)?
        .set_default("bind", "0.0.0.0")?
        .set_default("abuseipdb.enabled", false)?
        .set_default("abuseipdb.url", "")?
        .set_default("abuseipdb.categories", Vec::<String>::new())?
        .set_default("abuseipdb.key", "")?
        .set_default("abuseipdb.comment.hostname", "")?
        .set_default("abuseipdb.comment.display_port", "")?
        .set_default("abuseipdb.comment.message", "")?;

    let loaded_config = config_builder
        .add_source(config::File::with_name("config.toml"))
        .build()?;

    let bind: Ipv4Addr = loaded_config
        .get_string("bind")?
        .parse()
        .expect("Failed to parse bind ip");
    let port: u16 = loaded_config
        .get_int("port")?
        .try_into()
        .expect("Failed to parse port");
    let categories: Vec<String> = loaded_config
        .get_array("abuseipdb.categories")?
        .iter()
        .map(|x| {
            x.clone()
                .into_string()
                .expect("Failed to get AbuseIPDB categories")
        })
        .collect();

    let config = Configuration {
        bind,
        port,
        abuseipdb: AbuseIPDBConfig {
            enabled: loaded_config.get_bool("abuseipdb.enabled")?,
            url: loaded_config.get_string("abuseipdb.url")?,
            key: loaded_config.get_string("abuseipdb.key")?,
            categories,
            comment: AbuseIPDBCommentConfig {
                hostname: loaded_config.get_string("abuseipdb.comment.hostname")?,
                display_port: loaded_config.get_string("abuseipdb.comment.display_port")?,
                message: loaded_config.get_string("abuseipdb.comment.message")?,
            },
        },
    };
    Ok(config)
}

async fn handle_connection(stream: TcpStream, config: &Configuration) {
    let remote_addr = match stream.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Failed to get peer address: {}", err);
            return;
        }
    };
    let local_addr = match stream.local_addr() {
        Ok(addr) => addr,
        Err(err) => {
            error!("Failed to get peer address: {}", err);
            return;
        }
    };

    let mut no_report = false;
    if let IpAddr::V4(ip) = remote_addr.ip() {
        if ip.is_private() {
            no_report = true;
        }
    }

    info!(
        "Connection attempt from IP={} SRC_PORT={} DEST_PORT={}",
        remote_addr.ip(),
        remote_addr.port(),
        local_addr.port(),
    );
    if !no_report {
        tokio::spawn(process_ip(remote_addr.ip(), config.clone()));
    }
}

async fn process_ip(ip: IpAddr, config: Configuration) -> () {
    if config.abuseipdb.enabled {
        to_abuseipdb(ip, config.abuseipdb.clone()).await;
    }
}

#[cached(
    type = "TimedCache<(IpAddr, AbuseIPDBConfig), bool>",
    create = "{ TimedCache::with_lifespan_and_capacity(900, 50) }"
)]
async fn to_abuseipdb(ip: IpAddr, config: AbuseIPDBConfig) -> bool {
    let mut headers = HeaderMap::new();
    headers.insert("Key", config.key.parse().unwrap());
    headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());

    let mut body: HashMap<&str, String> = HashMap::new();
    body.insert("ip", ip.to_string());
    body.insert("categories", config.categories.join(","));

    let mut comment = format!("Connection attemp from {}", ip.to_string());
    if config.comment.display_port != "" {
        comment += format!(" to port {}", config.comment.display_port).as_str();
    }
    if config.comment.hostname != "" {
        comment += format!(" ({})", config.comment.hostname).as_str();
    }
    if config.comment.message != "" {
        comment += format!(": {}", config.comment.message).as_str();
    }
    body.insert("comment", comment);

    let clinet = reqwest::Client::new();
    let res = match clinet
        .post(config.url)
        .headers(headers)
        .json(&body)
        .send()
        .await
    {
        Ok(res) => res,
        Err(err) => {
            error!("AbuseIPDB: Could not make request: {}", err);
            return false;
        }
    };
    if res.status().as_u16() == 429 {
        error!("AbuseIPDB: IP={} is ratelimited", ip);
        return false;
    } else if res.status().as_u16() != 200 {
        error!("AbuseIPDB: IP={} failed to report", ip);
        return false;
    }

    info!("AbuseIPDB: Reported IP={}", ip);
    return true;
}

// async fn to_discord_webhook(ip: IpAddr, config: DiscordConfig) -> bool {
//     // 1. send discord weebhoo
//     return true;
// }
