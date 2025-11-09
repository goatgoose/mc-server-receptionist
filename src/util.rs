use std::io::Error;
use tokio::net::TcpStream;

pub trait AsyncPeek {
    async fn peek(&self, buf: &mut [u8]) -> Result<usize, Error>;
}

impl AsyncPeek for TcpStream {
    async fn peek(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.peek(buf).await
    }
}
