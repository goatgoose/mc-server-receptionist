use crate::connection::{Connection, JoinCallback, LoginStart};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

struct InstanceInitializer {}

impl JoinCallback for InstanceInitializer {
    fn on_join(&self, login_start: &LoginStart) {
        println!("{} joined!", login_start.username);
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
