mod codec;
mod elastic_ip_manager;
mod protocol;
mod receptionist;

use receptionist::Receptionist;
use tokio::io;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let receptionist = Receptionist {};
    receptionist.listen("localhost:25565").await?;

    Ok(())
}
