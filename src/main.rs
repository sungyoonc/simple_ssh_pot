#[macro_use]
extern crate log;

extern crate simplelog;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;
use simplelog::{CombinedLogger, TermLogger, WriteLogger};
use tokio::net::{TcpListener, TcpStream};

use config::Config;

use cached::proc_macro::cached;
use cached::TimedCache;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[derive(Hash, PartialEq, Eq, Clone)]
struct AbuseIPDBConfig {
    enabled: bool,
    url: String,
    key: String,
    categories: Vec<String>,
    comment: CommentConfig,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct DiscordConfig {
    enabled: bool,
    url: String,
    username: String,
    comment: CommentConfig,
}

#[derive(Hash, PartialEq, Eq, Clone)]
struct CommentConfig {
    hostname: String,
    display_port: String,
    message: String,
}

#[derive(Clone)]
struct Configuration {
    bind: Ipv4Addr,
    port: u16,
    abuseipdb: AbuseIPDBConfig,
    discord: DiscordConfig,
}

#[derive(Debug)]
struct DiscordRatelimit {
    limit: i64,
    remaining: i64,
    reset: Option<SystemTime>,
    reset_after: Option<Duration>,
}

impl Default for DiscordRatelimit {
    fn default() -> Self {
        Self {
            limit: i64::MAX,
            remaining: i64::MAX,
            reset: None,
            reset_after: None,
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    fs::create_dir_all("data").expect("Failed to create data directory");
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
            fs::File::create("data/simple_ssh_pot.log").unwrap(),
        ),
    ])
    .unwrap();

    let config = load_config().expect("Failed to load config");

    let discord_ratelimit = Arc::new(Mutex::new(DiscordRatelimit::default()));
    let cancel_token = CancellationToken::new();
    let cloned_token = cancel_token.clone();

    let addr = SocketAddr::from((config.bind, config.port));
    let task_handle = tokio::spawn(async move {
        let listener = TcpListener::bind(&addr).await.unwrap();
        info!("Listening on {}", addr);

        loop {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    handle_connection(stream, &config, &discord_ratelimit).await;
                },
                _ = cloned_token.cancelled() => break,
                else => break,
            }
        }
    });

    let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = signal::ctrl_c() => {},
    }

    info!("Shutting Down");
    cancel_token.cancel();
    task_handle.await.unwrap();
    return Ok(());
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
        .set_default("abuseipdb.comment.message", "")?
        .set_default("discord.enabled", false)?
        .set_default("discord.url", "")?
        .set_default("discord.username", "")?
        .set_default("discord.comment.hostname", "")?
        .set_default("discord.comment.display_port", "")?
        .set_default("discord.comment.message", "")?;

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
            comment: CommentConfig {
                hostname: loaded_config.get_string("abuseipdb.comment.hostname")?,
                display_port: loaded_config.get_string("abuseipdb.comment.display_port")?,
                message: loaded_config.get_string("abuseipdb.comment.message")?,
            },
        },
        discord: DiscordConfig {
            enabled: loaded_config.get_bool("discord.enabled")?,
            url: loaded_config.get_string("discord.url")?,
            username: loaded_config.get_string("discord.username")?,
            comment: CommentConfig {
                hostname: loaded_config.get_string("discord.comment.hostname")?,
                display_port: loaded_config.get_string("discord.comment.display_port")?,
                message: loaded_config.get_string("discord.comment.message")?,
            },
        },
    };
    Ok(config)
}

async fn handle_connection(
    stream: TcpStream,
    config: &Configuration,
    discord_ratelimit: &Arc<Mutex<DiscordRatelimit>>,
) {
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
        tokio::spawn(process_ip(
            remote_addr.ip(),
            config.clone(),
            Arc::clone(discord_ratelimit),
        ));
    }
}

async fn process_ip(
    ip: IpAddr,
    config: Configuration,
    discord_ratelimit: Arc<Mutex<DiscordRatelimit>>,
) -> () {
    if config.abuseipdb.enabled {
        to_abuseipdb(ip, config.abuseipdb.clone()).await;
    }
    if config.discord.enabled {
        let epoch_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        to_discord_webhook(
            ip,
            &config.discord,
            epoch_time.as_secs(),
            &discord_ratelimit,
        )
        .await;
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

async fn to_discord_webhook(
    ip: IpAddr,
    config: &DiscordConfig,
    epoch_time: u64,
    discord_ratelimit: &Arc<Mutex<DiscordRatelimit>>,
) -> bool {
    let mut headers = HeaderMap::new();
    headers.insert(reqwest::header::ACCEPT, "application/json".parse().unwrap());

    let mut comment = format!("Connection attemp from {}", ip.to_string());
    if config.comment.display_port != "" {
        comment += format!(" to port {}", config.comment.display_port).as_str();
    }
    comment += format!(" <t:{}:D><t:{}:T>", epoch_time, epoch_time).as_str();
    if config.comment.hostname != "" {
        comment += format!(" ({})", config.comment.hostname).as_str();
    }
    if config.comment.message != "" {
        comment += format!(": {}", config.comment.message).as_str();
    }

    let mut body: HashMap<&str, String> = HashMap::new();
    body.insert("username", config.username.clone());
    body.insert("content", comment);

    let clinet = reqwest::Client::new();
    let mut ratelimited = true;
    while ratelimited {
        // Wait for ratelimit
        let reset;
        let reset_after;
        let remaining;
        {
            let ratelimit = discord_ratelimit.lock().unwrap();
            reset = ratelimit.reset.unwrap_or(UNIX_EPOCH);
            reset_after = ratelimit.reset_after.unwrap_or(Duration::from_secs(0)); // unwrap_or makes sure that when reset_after is None, it doesn't sleep
            remaining = ratelimit.remaining;
        }
        if remaining == 0 && SystemTime::now().cmp(&reset) == Ordering::Less {
            tokio::time::sleep(reset_after.clone()).await;
        }

        // Send webhook
        let res = match clinet
            .post(&config.url)
            .headers(headers.clone())
            .json(&body)
            .send()
            .await
        {
            Ok(res) => res,
            Err(err) => {
                error!("Discord Webhook: Could not make request: {}", err);
                return false;
            }
        };
        if res.status().as_u16() == 429 {
            // Do nothing
        } else if res.status().is_success() {
            ratelimited = false;
        } else {
            println!("{}", res.status().is_success());
            error!("Discord Webhook: IP={} failed to make request", ip);
            return false;
        }

        // Save ratelimit information
        {
            let mut ratelimit = discord_ratelimit.lock().unwrap();
            if let Some(limit) = res.headers().get("X-RateLimit-Limit") {
                ratelimit.limit = limit.to_str().unwrap().parse().unwrap();
            }
            if let Some(remaining) = res.headers().get("X-RateLimit-Remaining") {
                ratelimit.remaining = remaining.to_str().unwrap().parse().unwrap();
            }
            if let Some(reset) = res.headers().get("X-RateLimit-Reset") {
                let time = Duration::from_secs(reset.to_str().unwrap().parse().unwrap());
                ratelimit.reset = Some(UNIX_EPOCH + time);
            }
            if let Some(reset_after) = res.headers().get("X-RateLimit-Reset-After") {
                let time = Duration::from_secs(reset_after.to_str().unwrap().parse().unwrap());
                ratelimit.reset_after = Some(time);
            }
        }
    }
    return true;
}
