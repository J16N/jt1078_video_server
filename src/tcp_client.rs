use crate::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;

pub(crate) struct TcpClient {
    pub(crate) address: SocketAddr,
    stream: Option<BufWriter<TcpStream>>,
}

impl TcpClient {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Self {
            address,
            stream: None,
        }
    }

    pub(crate) async fn connect(&mut self) -> Result<()> {
        if self.stream.is_none() {
            let stream = TcpStream::connect(self.address).await?;
            self.stream = Some(BufWriter::new(stream));
        }
        Ok(())
    }

    pub(crate) async fn send(&mut self, data: &[u8]) -> Result<()> {
        if let Some(writer) = &mut self.stream {
            writer.write_all(data).await?;
        }
        Ok(())
    }

    pub(crate) async fn close(&mut self) -> Result<()> {
        if let Some(mut writer) = self.stream.take() {
            writer.shutdown().await?
        }
        Ok(())
    }
}
