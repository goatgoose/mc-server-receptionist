use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct ReceptionistConfig {
    pub target_instance_name: String,
    pub mc_target_port: u16,
    pub mc_target_motd: String,
}