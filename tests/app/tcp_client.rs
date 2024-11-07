use jt1078_video_server::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpSocket, TcpStream};

pub(crate) struct TcpClient {
    address: SocketAddr,
    data_path: PathBuf,
    stream: Option<BufWriter<TcpStream>>,
}

impl TcpClient {
    pub(crate) fn new(address: SocketAddr, path: &str) -> Self {
        Self {
            address,
            data_path: PathBuf::from(path),
            stream: None,
        }
    }

    pub(crate) async fn connect(&mut self) -> Result<()> {
        if self.stream.is_none() {
            let socket = TcpSocket::new_v4()?;
            socket.set_reuseaddr(true)?;
            let stream = socket.connect(self.address).await?;
            self.stream = Some(BufWriter::new(stream));
        }
        Ok(())
    }

    pub(crate) async fn send(&mut self) -> Result<()> {
        let mut reader = self.get_reader().await?;
        let writer = self.stream.as_mut().expect("Stream not found");
        let mut buffer = [0; 512 * 1024];

        let mut count = 0;

        loop {
            let bytes_read = reader.read(&mut buffer).await?;
            if bytes_read == 0 || count >= 5 * 1024 * 1024 {
                break;
            }
            writer.write_all(&buffer[..bytes_read]).await?;
            count += bytes_read;
        }

        writer.flush().await?;
        Ok(())
    }

    async fn get_reader(&self) -> Result<BufReader<File>> {
        let file = File::open(&self.data_path).await?;
        Ok(BufReader::new(file))
    }

    pub(crate) async fn close(&mut self) -> Result<()> {
        if let Some(mut writer) = self.stream.take() {
            writer.shutdown().await?
        }
        Ok(())
    }
}
