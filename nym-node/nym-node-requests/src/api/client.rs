// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::v1::gateway::models::WebSockets;
use crate::api::v1::node::models::SignedHostInformation;
use crate::routes;
use async_trait::async_trait;
use http_api_client::{ApiClient, HttpClientError};

pub use http_api_client::Client;

pub type NymNodeApiClientError = HttpClientError;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NymNodeApiClientExt: ApiClient {
    async fn get_host_information(&self) -> Result<SignedHostInformation, NymNodeApiClientError> {
        self.get_json_from(routes::api::v1::host_info_absolute())
            .await
    }

    // TODO: implement calls for other endpoints; for now I only care about the wss
    async fn get_mixnet_websockets(&self) -> Result<WebSockets, NymNodeApiClientError> {
        self.get_json_from(
            routes::api::v1::gateway::client_interfaces::mixnet_websockets_absolute(),
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl NymNodeApiClientExt for Client {}
