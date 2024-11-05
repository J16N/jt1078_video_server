pub(crate) mod helper;
pub(crate) mod processor;
pub(crate) mod rtp;
pub mod server;
pub(crate) mod tcp_client;

pub type Result<T> = std::result::Result<T, anyhow::Error>;
