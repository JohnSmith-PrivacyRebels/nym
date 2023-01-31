// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::mixnet_handling::receiver::connection_handler::ConnectionHandler;
use crate::node::storage::Storage;
use tracing::*;
use std::net::SocketAddr;
use std::process;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
}

// TODO: this file is nearly identical to the one in mixnode
impl Listener {
    pub(crate) fn new(address: SocketAddr) -> Self {
        Listener { address }
    }
    //SW#[instrument(level="info", skip_all, name="Mixnet Listener")]
    pub(crate) async fn run<St>(&mut self, connection_handler: ConnectionHandler<St>)
    where
        St: Storage + Clone + 'static,
    {
        info!("Starting mixnet listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        loop {
            match tcp_listener.accept().await {
                Ok((socket, remote_addr)) => {
                    let handler = connection_handler.clone();
                    tokio::spawn(handler.handle_connection(socket, remote_addr));
                }
                Err(err) => warn!("failed to get client: {err}"),
            }
        }
    }

    pub(crate) fn start<St>(mut self, connection_handler: ConnectionHandler<St>) -> JoinHandle<()>
    where
        St: Storage + Clone + 'static,
    {
        info!("Running mix listener on {:?}", self.address.to_string());

        tokio::spawn(async move { self.run(connection_handler).await })
    }
}
