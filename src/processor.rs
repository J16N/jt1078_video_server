use crate::rtp::RtpPacket;
use crate::Result;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc::Receiver;
use tokio::time::timeout;

pub(crate) struct RtpProcessor {
    child_stdin: Option<ChildStdin>,
    dir_init: bool,
    ffmpeg_process: Option<Child>,
    imei: String,
}

impl RtpProcessor {
    pub fn new() -> Self {
        Self {
            child_stdin: None,
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

        if let Some(mut stdin) = self.child_stdin.take() {
            if let Err(e) = stdin.shutdown().await {
                eprintln!("Failed to shutdown ffmpeg stdin ({}): {e}", self.imei);
            }
        }

        if let Some(mut process) = self.ffmpeg_process.take() {
            // if let Err(e) = process.kill().await {
            //     eprintln!("Failed to kill ffmpeg process ({}): {e}", self.imei);
            // }
            if let Err(e) = timeout(Duration::from_secs(10), process.wait()).await {
                eprintln!("Failed to wait for ffmpeg process ({}): {e}", self.imei);
                let _ = process.kill().await;
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
            self.init_ffmpeg_process().await?;
        }

        if let Some(stdin) = &mut self.child_stdin {
            stdin.write_all(&packet.payload).await?;
            stdin.flush().await?;
        }

        Ok(())
    }

    async fn init_dir(&mut self, name: &str) -> Result<()> {
        let streams_dir = PathBuf::from(format!("{name}/streams"));
        fs::create_dir_all(&streams_dir).await?;
        self.dir_init = true;
        Ok(())
    }

    async fn init_ffmpeg_process(&mut self) -> Result<()> {
        let arguments = format!(
            "-hide_banner -loglevel error -re -f h264 -i pipe: \
            -c copy -preset:v fast -strftime 1 -hls_init_time 1 \
            -hls_time 6 -hls_segment_filename {}/streams/%Y-%m-%d_%H-%M-%S.ts \
            -hls_list_size 10 -hls_flags delete_segments -f hls {}/playlist.m3u8",
            self.imei, self.imei
        );

        let arguments: Vec<&str> = arguments.split(' ').collect();
        let mut child = Command::new("ffmpeg")
            .stdin(Stdio::piped())
            .args(&arguments)
            .spawn()?;
        let stdin = child.stdin.take().ok_or(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "Failed to open stdin.",
        ))?;
        self.ffmpeg_process = Some(child);
        self.child_stdin = Some(stdin);

        Ok(())
    }

    async fn clean_up(&mut self) -> Result<()> {
        fs::remove_dir_all(&self.imei).await?;
        Ok(())
    }
}
