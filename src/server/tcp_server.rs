use crate::processor::RtpProcessor;
use crate::rtp::RtpPacket;
use std::net::SocketAddr;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpSocket, TcpStream};
use tokio::sync::mpsc;

pub struct TcpServer {
    address: SocketAddr,
    listener: Option<TcpListener>,
}

impl TcpServer {
    pub fn new(host: &str, port: u16) -> Self {
        let port: u16 = std::env::var("PORT")
            .unwrap_or_else(|_| port.to_string())
            .parse()
            .expect("Failed to parse port");

        let address = format!("{}:{}", host, port)
            .parse()
            .expect("Failed to parse server address");

        let socket = Self::prepare_socket(address);
        let listener = socket.listen(1024).expect("Failed to listen on socket");
        let address = listener.local_addr().expect("Failed to get local address");

        Self {
            address,
            listener: Some(listener),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    fn prepare_socket(address: SocketAddr) -> TcpSocket {
        let socket = TcpSocket::new_v4().expect("Failed to create socket");

        socket
            .set_reuseaddr(true)
            .expect("Failed to set reuse address");

        socket.bind(address).expect("Failed to bind to address");

        socket
    }

    pub async fn run(&mut self) {
        println!("TCP Server listening on {}", self.address);

        let listener = self.listener.take().expect("Listener not found");
        self.listen(listener).await;
    }

    async fn listen(&self, listener: TcpListener) {
        while let Ok((stream, peer)) = listener.accept().await {
            println!("Incoming connection from: {peer}");
            let (tx, rx) = mpsc::channel::<RtpPacket>(100);
            let mut processor = RtpProcessor::new();
            tokio::spawn(async move {
                processor.listen(rx).await;
            });
            tokio::spawn(Self::handle_connection(stream, tx));
        }
    }

    async fn handle_connection(mut stream: TcpStream, tx: mpsc::Sender<RtpPacket>) {
        let (reader, _) = stream.split();
        let mut buf_reader = BufReader::new(reader);

        loop {
            let packet = match RtpPacket::parse(&mut buf_reader).await {
                Ok(packet) => packet,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        println!("Connection closed by client");
                        return;
                    }
                    eprintln!("Failed to parse packet: {e}");
                    continue;
                }
            };

            if let Err(e) = tx.send(packet).await {
                eprintln!("Failed to send packet: {e}");
                break;
            }
        }
    }
}
