
mod rustyx;
mod handlers;
mod config;
mod http;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
   let master =  rustyx::Master::new();
   master.start().await
}
