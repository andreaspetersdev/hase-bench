use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonEndpoint {
    pub address: SocketAddr,
    pub module: String,
}
