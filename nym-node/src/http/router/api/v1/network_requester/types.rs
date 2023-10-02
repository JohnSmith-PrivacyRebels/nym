// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, Debug, Clone, ToSchema)]
pub struct NetworkRequester {
    /// Base58 encoded ed25519 EdDSA public key of the network requester.
    pub encoded_identity_key: String,

    /// Base58-encoded x25519 public key used for performing key exchange with remote clients.
    pub encoded_x25519_key: String,

    /// Nym address of this network requester.
    pub address: String,
}

// #[derive(Serialize, Debug, Clone, ToSchema)]
// pub struct ExitPolicy {
//     // pub allowed_ports:
//     // pub deny_list: DenyList,
// }
//
// #[derive(Serialize, Debug, Clone, ToSchema)]
// pub struct DenyListEntry {
//     // pub ports:
//     // pub ips:
//     pub description: String,
// }
