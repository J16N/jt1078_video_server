use crate::rtp::RtpPacket;
use crate::tcp_client::TcpClient;
use crate::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tokio::net::TcpSocket;
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::sync::oneshot::{channel, Receiver, Sender};
use tokio::task::JoinHandle;

pub(crate) struct RtpProcessor {
    address: SocketAddr,
    client: Option<TcpClient>,
    dir_init: bool,
    ffmpeg_process: Option<JoinHandle<()>>,
    imei: String,
    rx: Option<Receiver<()>>,
    tx: Option<Sender<()>>,
}

impl RtpProcessor {
    pub fn new() -> Self {
        let (tx, rx) = channel::<()>();
        let address = "127.0.0.1:0".parse().expect("Failed to parse address");

        Self {
            address,
            client: None,
            dir_init: false,
            ffmpeg_process: None,
            imei: String::new(),
            rx: Some(rx),
            tx: Some(tx),
        }
    }

    pub async fn listen(&mut self, mut channel: mpsc::Receiver<RtpPacket>) {
        while let Some(packet) = channel.recv().await {
            if let Err(e) = self.process(packet).await {
                eprintln!("Failed to process packet ({}): {e}", self.imei);
                break;
            }
        }

        if let Some(mut client) = self.client.take() {
            if let Err(e) = client.close().await {
                eprintln!(
                    "Failed to close client ({} - {}): {e}",
                    client.address, self.imei
                );
            }
        }

        if let Some(handle) = self.ffmpeg_process.take() {
            let tx = self.tx.take().expect("Transmitter not found");

            match tx.send(()) {
                Ok(_) => {
                    if let Err(e) = handle.await {
                        eprintln!("Failed to wait for ffmpeg process ({}): {e}", self.imei);
                    }
                }

                Err(_) => {
                    handle.abort();
                }
            }
        }

        if let Err(e) = self.clean_up().await {
            eprintln!("Failed to clean up directories ({}): {e}", self.imei);
        }
    }

    async fn process(&mut self, packet: RtpPacket) -> Result<()> {
        if !self.dir_init {
            let imei = &packet.header.terminal_serial_number;
            self.imei.push_str(imei);
            self.init_dir(imei).await?;
            self.init_address().await?;
            self.init_client();
            self.init_ffmpeg_process().await?;

            if let Some(client) = &mut self.client {
                match client.connect().await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Client connection failed ({imei}): {}", client.address);
                        return Err(e);
                    }
                };
            }
        }

        if let Some(client) = &mut self.client {
            client.send(&packet.payload).await?;
        }

        Ok(())
    }

    async fn init_address(&mut self) -> Result<()> {
        let socket = TcpSocket::new_v4()?;
        socket.set_reuseaddr(true)?;
        socket.bind(self.address)?;
        let address = socket.local_addr()?;
        self.address = address;
        Ok(())
    }

    fn init_client(&mut self) {
        let client = TcpClient::new(self.address);
        self.client = Some(client);
    }

    async fn init_dir(&mut self, name: &str) -> Result<()> {
        let streams_dir = PathBuf::from(format!("{name}/streams"));
        fs::create_dir_all(&streams_dir).await?;
        self.dir_init = true;
        Ok(())
    }

    async fn init_ffmpeg_process(&mut self) -> Result<()> {
        let arguments = format!(
            "-hide_banner -loglevel error -re -f h264 -i tcp://{}\\?listen -c copy -strftime 1 \
            -hls_time 2 -hls_segment_filename {}/streams/%Y-%m-%d_%H-%M-%S.ts \
            -hls_list_size 5 -hls_flags delete_segments -f hls {}/playlist.m3u8",
            self.address, self.imei, self.imei
        );

        let arguments: Vec<&str> = arguments.split(' ').collect();
        let mut child = Command::new("ffmpeg").args(&arguments).spawn()?;
        let rx = self.rx.take().expect("Receiver not found");

        let handle = tokio::spawn(async move {
            tokio::select! {
                _ = child.wait() => (),
                _ = rx => {
                    if let Err(e) = child.kill().await {
                        eprintln!("Failed to kill ffmpeg process: {e}");
                    }
                }
            };
        });
        self.ffmpeg_process = Some(handle);
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        Ok(())
    }

    async fn clean_up(&mut self) -> Result<()> {
        let root = PathBuf::from(&self.imei);
        fs::remove_dir_all(root).await?;
        Ok(())
    }
}
