mod connection;
mod receptionist;
mod util;
mod config;

use std::error::Error;
use std::fs;
use receptionist::Receptionist;
use tokio::io;
use crate::config::ReceptionistConfig;
use toml;

const CONFIG_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml");

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("{}", CONFIG_PATH);
    let config: ReceptionistConfig = toml::from_str(fs::read_to_string(CONFIG_PATH)?.as_str())?;

    let receptionist = Receptionist::new(
        config.target_instance_name,
        config.mc_target_port,
        config.mc_target_motd,
    ).await;
    receptionist.listen("0.0.0.0:25565").await?;

    Ok(())
}
