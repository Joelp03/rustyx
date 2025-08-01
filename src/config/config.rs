use std::{fs, net::SocketAddr, str::FromStr};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listener: Listener,
    pub servers: Servers,
}

#[derive(Debug, Deserialize)]
pub struct Listener {
    pub listen: String,
}

#[derive(Debug, Deserialize)]
pub struct Location {
    path: String,
    proxy_pass: String,
}

#[derive(Debug, Deserialize)]
pub struct Servers {
    locations: Vec<Location>,
}


pub struct ProxyConfig {
    pub listener: SocketAddr,
    servers: Servers,
}


pub fn load_config() -> Result<ProxyConfig, Box<dyn std::error::Error>> {
    let filename = "rustyx.toml";

    let contents = fs::read_to_string(filename)?;

    let config: Config = toml::from_str(&contents)?;
    
        
    let listener = SocketAddr::from_str(&config.listener.listen)
        .map_err(|e| format!("Failed to parse listen address: {}", e))?;
    

    Ok(ProxyConfig { listener, servers: config.servers })
}