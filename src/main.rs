
mod rustyx;
mod handlers;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    rustyx::start().await
}
