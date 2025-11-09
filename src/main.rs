mod receptionist;
mod elastic_ip_manager;
mod protocol;
mod codec;

use tokio::io;
use receptionist::Receptionist;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let receptionist = Receptionist {};
    receptionist.listen("localhost:25565").await?;

    Ok(())
}
