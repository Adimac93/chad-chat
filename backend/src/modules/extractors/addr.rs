use axum::extract::{connect_info::Connected};
use hyper::server::conn::AddrStream;
use sqlx::types::ipnetwork::IpNetwork;

#[derive(Clone)]
pub struct ClientAddr(IpNetwork);

impl ClientAddr {
    pub fn network(&self) -> IpNetwork {
        self.0
    }
}

impl Connected<&AddrStream> for ClientAddr {
    fn connect_info(target: &AddrStream) -> Self {
        Self(IpNetwork::from(target.remote_addr().ip()))
    }
}
