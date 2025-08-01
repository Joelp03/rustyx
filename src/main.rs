
mod rustyx;
mod handlers;
mod config;
mod http;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustyx::start().await
}
