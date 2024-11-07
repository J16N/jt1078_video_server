pub(crate) mod helper;
pub(crate) mod processor;
pub(crate) mod rtp;
pub mod server;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

use server::TcpServer;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

pub struct TcpServerTask {
    tcp_server_task: Option<JoinHandle<()>>,
    tx: Option<broadcast::Sender<()>>,
}

impl TcpServerTask {
    pub async fn end(mut self) {
        let tx = self.tx.take().expect("Failed to take transmitter.");
        let tcp_server_task = self
            .tcp_server_task
            .take()
            .expect("Failed to take tcp server task.");

        match tx.send(()) {
            Ok(_) => {
                if let Err(e) = tcp_server_task.await {
                    eprintln!("Failed to wait for tcp server task: {e}");
                }
            }
            Err(_) => {
                eprintln!("Failed to send signal to tcp server task.");
                tcp_server_task.abort();
            }
        }
    }
}

pub fn run_tcp_server(address: &str, port: u16) -> TcpServerTask {
    let mut tcp_server = TcpServer::new(address, port);

    let (tx, mut rx) = broadcast::channel(1);

    let tcp_sever_task = tokio::spawn(async move {
        tokio::select! {
            _ = tcp_server.run() => (),
            _ = rx.recv() => tcp_server.close().await,
        }
    });

    TcpServerTask {
        tcp_server_task: Some(tcp_sever_task),
        tx: Some(tx),
    }
}
