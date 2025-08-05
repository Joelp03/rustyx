use std::{fs, net::SocketAddr, str::FromStr};

use serde::Deserialize;


#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    #[serde(rename = "server")]
    pub servers: Vec<Server>,
}

// server config
#[derive(Debug, Deserialize, Clone)]
pub struct Server{
    pub listen: Vec<SocketAddr>,
    pub name: String,
    pub location: Vec<Location>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Location {
    pub path: String,
    pub proxy_pass: String,
}



pub fn load_config() -> Result<ProxyConfig, Box<dyn std::error::Error>> {
    let filename = "rustyx.toml";

    let contents = fs::read_to_string(filename)?;

    let config: ProxyConfig = toml::from_str(&contents)?;
    

    Ok(config)
}