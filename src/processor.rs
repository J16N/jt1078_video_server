use crate::rtp::RtpPacket;
use crate::tcp_client::TcpClient;
use crate::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tokio::net::TcpSocket;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::Receiver;

pub(crate) struct RtpProcessor {
    address: SocketAddr,
    client: Option<TcpClient>,
    dir_init: bool,
    ffmpeg_process: Option<Child>,
    imei: String,
}

impl RtpProcessor {
    pub fn new() -> Self {
        let address = "127.0.0.1:0".parse().expect("Failed to parse address");

        Self {
            address,
            client: None,
            dir_init: false,
            ffmpeg_process: None,
            imei: String::new(),
        }
    }

    pub async fn listen(&mut self, mut channel: Receiver<RtpPacket>) {
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

        if let Some(mut process) = self.ffmpeg_process.take() {
            if let Err(e) = process.kill().await {
                eprintln!("Failed to kill ffmpeg process ({}): {e}", self.imei);
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
                        eprintln!(
                            "Failed to connect to client ({} - {}): {e}",
                            client.address, imei
                        );
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
            -hls_time 5 -hls_segment_filename {}/streams/%Y-%m-%d_%H-%M-%S.ts \
            -hls_list_size 5 -hls_flags delete_segments -f hls {}/playlist.m3u8",
            self.address, self.imei, self.imei
        );

        println!("ffmpeg {arguments}");

        let arguments: Vec<&str> = arguments.split(' ').collect();
        let child = Command::new("ffmpeg").args(&arguments).spawn()?;
        self.ffmpeg_process = Some(child);

        Ok(())
    }

    async fn clean_up(&mut self) -> Result<()> {
        let root = PathBuf::from(&self.imei);
        fs::remove_dir_all(root).await?;
        Ok(())
    }
}
