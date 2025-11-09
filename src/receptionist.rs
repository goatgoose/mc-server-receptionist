use crate::connection::Connection;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

pub struct Receptionist {}

impl Receptionist {
    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on: {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("Accepted connection from: {}", &addr);

            tokio::spawn(async move {
                let mut connection = Connection::new(stream);
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
