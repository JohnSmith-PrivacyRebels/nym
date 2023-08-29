// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::active_clients::ActiveClientsStore;
use crate::node::client_handling::websocket::connection_handler::coconut::CoconutVerifier;
use crate::node::client_handling::websocket::connection_handler::FreshHandler;
use crate::node::storage::Storage;
use log::*;
use nym_crypto::asymmetric::{encryption, identity};
use nym_mixnet_client::forwarder::MixForwardingSender;
use nym_noise::upgrade_noise_responder;
use nym_validator_client::NymApiClient;
use rand::rngs::OsRng;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use tokio::task::JoinHandle;

pub(crate) struct Listener {
    address: SocketAddr,
    local_identity: Arc<identity::KeyPair>,
    local_sphinx: Arc<encryption::KeyPair>,
    only_coconut_credentials: bool,
    pub(crate) coconut_verifier: Arc<CoconutVerifier>,
    nym_api_client: NymApiClient,
}

impl Listener {
    pub(crate) fn new(
        address: SocketAddr,
        local_identity: Arc<identity::KeyPair>,
        local_sphinx: Arc<encryption::KeyPair>,
        only_coconut_credentials: bool,
        coconut_verifier: Arc<CoconutVerifier>,
        nym_api_client: NymApiClient,
    ) -> Self {
        Listener {
            address,
            local_identity,
            local_sphinx,
            only_coconut_credentials,
            coconut_verifier,
            nym_api_client,
        }
    }

    // TODO: change the signature to pub(crate) async fn run(&self, handler: Handler)

    pub(crate) async fn run<St>(
        &mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: St,
        active_clients_store: ActiveClientsStore,
        mut shutdown: nym_task::TaskClient,
    ) where
        St: Storage + Clone + 'static,
    {
        info!("Starting websocket listener at {}", self.address);
        let tcp_listener = match tokio::net::TcpListener::bind(self.address).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Failed to bind the websocket to {} - {err}. Are you sure nothing else is running on the specified port and your user has sufficient permission to bind to the requested address?", self.address);
                process::exit(1);
            }
        };

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("client_handling::Listener: received shutdown");
                }
                connection = tcp_listener.accept() => {
                    match connection {
                        Ok((socket, remote_addr)) => {
                            trace!("received a socket connection from {remote_addr}");
                            // TODO: I think we *REALLY* need a mechanism for having a maximum number of connected
                            // clients or spawned tokio tasks -> perhaps a worker system?

                            let current_epoch_id = match self.nym_api_client.get_current_epoch_id().await {
                                Ok(epoch_id) => epoch_id,
                                Err(err) => {
                                    error!("Failed to retrieve epoch Id for Noise handshake - {err}");
                                    continue;
                                }
                            };
                            let noise_stream = match upgrade_noise_responder(
                                socket,
                                Default::default(),
                                &self.local_sphinx.public_key().to_bytes(),
                                &self.local_sphinx.private_key().to_bytes(),
                                None, //connection from client, no remote pub key
                                current_epoch_id,
                            )
                            .await
                            {
                                Ok(noise_stream) => noise_stream,
                                Err(err) => {
                                    error!("Failed to perform Noise handshake with {remote_addr} - {err}");
                                    continue;
                                }
                            };
                            let handle = FreshHandler::new(
                                OsRng,
                                noise_stream,
                                self.only_coconut_credentials,
                                outbound_mix_sender.clone(),
                                Arc::clone(&self.local_identity),
                                storage.clone(),
                                active_clients_store.clone(),
                                Arc::clone(&self.coconut_verifier),
                            );
                            let shutdown = shutdown.clone();
                            tokio::spawn(async move { handle.start_handling(shutdown).await });
                        }
                        Err(err) => warn!("failed to get client: {err}"),
                    }
                }

            }
        }
    }

    pub(crate) fn start<St>(
        mut self,
        outbound_mix_sender: MixForwardingSender,
        storage: St,
        active_clients_store: ActiveClientsStore,
        shutdown: nym_task::TaskClient,
    ) -> JoinHandle<()>
    where
        St: Storage + Clone + 'static,
    {
        tokio::spawn(async move {
            self.run(outbound_mix_sender, storage, active_clients_store, shutdown)
                .await
        })
    }
}
