use async_trait::async_trait;
use crate::connection::{Connection, TransferHandler, LoginStart, Transfer};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

struct InstanceInitializer {}

#[async_trait]
impl TransferHandler for InstanceInitializer {
    async fn on_join(&self, login_start: &LoginStart) -> Option<Transfer> {
        println!("{} joined!", login_start.username);
        // Some(Transfer {
        //     hostname: "classic.goatgoose.com".to_string(),
        //     port: 25565,
        // })
        None
    }

    async fn on_transfer_ready(&self) -> Option<Transfer> {
        Some(Transfer {
            hostname: "classic.goatgoose.com".to_string(),
            port: 25565,
        })
    }
}

pub struct Receptionist {}

impl Receptionist {
    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on: {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("Accepted connection from: {}", &addr);

            let initializer = InstanceInitializer {};

            tokio::spawn(async move {
                let mut connection = Connection::new(stream, initializer);
                match connection.process().await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("error: {e}");
                        return;
                    }
                }
                println!("process complete for {}", &addr);
            });
        }
    }
}
