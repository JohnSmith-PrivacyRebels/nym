// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::hash::Hasher;
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;

#[cfg(feature = "wireguard-verify")]
use nym_crypto::asymmetric::encryption::{PrivateKey, PublicKey};

#[cfg(feature = "wireguard-verify")]
use hmac::{Hmac, Mac};
#[cfg(feature = "wireguard-verify")]
use sha2::Sha256;

#[cfg(feature = "wireguard-verify")]
type HmacSha256 = Hmac<Sha256>;

pub type Nonce = u64;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ClientMessage {
    Initial(InitMessage),
    Final(Client),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InitMessage {
    /// Base64 encoded x25519 public key
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
    pub pub_key: ClientPublicKey,
}

impl InitMessage {
    pub fn pub_key(&self) -> ClientPublicKey {
        self.pub_key
    }

    pub fn new(pub_key: ClientPublicKey) -> Self {
        InitMessage { pub_key }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ClientRegistrationResponse {
    PendingRegistration { nonce: u64 },
    Registered { success: bool },
}

/// Client that wants to register sends its PublicKey and SocketAddr bytes mac digest encrypted with a DH shared secret.
/// Gateway/Nym node can then verify pub_key payload using the same process
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Client {
    /// Base64 encoded x25519 public key
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
    pub pub_key: ClientPublicKey,

    /// Client's socket address
    #[cfg_attr(feature = "openapi", schema(example = "1.2.3.4:51820", value_type = String))]
    pub socket: SocketAddr,

    /// Sha256 hmac on the data (alongside the prior nonce)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
    pub mac: ClientMac,
}

impl Client {
    #[cfg(feature = "wireguard-verify")]
    pub fn new(
        local_secret: &PrivateKey,
        remote_public: PublicKey,
        socket_address: SocketAddr,
        nonce: u64,
    ) -> Self {
        // convert from 1.0 x25519-dalek private key into 2.0 x25519-dalek
        #[allow(clippy::expect_used)]
        let static_secret = x25519_dalek::StaticSecret::try_from(local_secret.to_bytes())
            .expect("conversion between x25519 private keys is infallible");
        let local_public: x25519_dalek::PublicKey = (&static_secret).into();

        let remote_public = x25519_dalek::PublicKey::from(remote_public.to_bytes());

        let dh = static_secret.diffie_hellman(&remote_public);

        // TODO: change that to use our nym_crypto::hmac module instead
        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(local_public.as_bytes());
        mac.update(socket_address.ip().to_string().as_bytes());
        mac.update(socket_address.port().to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        Client {
            pub_key: ClientPublicKey(local_public),
            socket: socket_address,
            mac: ClientMac(mac.finalize().into_bytes().to_vec()),
        }
    }

    // Reusable secret should be gateways Wireguard PK
    // Client should perform this step when generating its payload, using its own WG PK
    #[cfg(feature = "wireguard-verify")]
    pub fn verify(&self, gateway_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        // convert from 1.0 x25519-dalek private key into 2.0 x25519-dalek
        #[allow(clippy::expect_used)]
        let static_secret = x25519_dalek::StaticSecret::try_from(gateway_key.to_bytes())
            .expect("conversion between x25519 private keys is infallible");

        let dh = static_secret.diffie_hellman(&self.pub_key);

        // TODO: change that to use our nym_crypto::hmac module instead
        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(self.pub_key.as_bytes());
        mac.update(self.socket.ip().to_string().as_bytes());
        mac.update(self.socket.port().to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        mac.verify_slice(&self.mac)
            .map_err(|source| Error::FailedClientMacVerification {
                client: self.pub_key.to_string(),
                source,
            })
    }

    pub fn pub_key(&self) -> ClientPublicKey {
        self.pub_key
    }

    pub fn socket(&self) -> SocketAddr {
        self.socket
    }
}

// This should go into nym-wireguard crate
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ClientPublicKey(x25519_dalek::PublicKey);

// TODO: change the inner type into generic array of size HmacSha256::OutputSize
#[derive(Debug, Clone)]
pub struct ClientMac(Vec<u8>);

impl fmt::Display for ClientMac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(&self.0))
    }
}

impl ClientMac {
    #[allow(dead_code)]
    pub fn new(mac: Vec<u8>) -> Self {
        ClientMac(mac)
    }
}

impl Deref for ClientMac {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for ClientPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.as_bytes()))
    }
}

impl Hash for ClientPublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state)
    }
}

impl FromStr for ClientMac {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mac_bytes: Vec<u8> =
            general_purpose::STANDARD
                .decode(s)
                .map_err(|source| Error::MalformedClientMac {
                    mac: s.to_string(),
                    source,
                })?;

        Ok(ClientMac(mac_bytes))
    }
}

impl ClientPublicKey {
    #[allow(dead_code)]
    pub fn new(key: x25519_dalek::PublicKey) -> Self {
        ClientPublicKey(key)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Deref for ClientPublicKey {
    type Target = x25519_dalek::PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ClientPublicKey {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_bytes: Vec<u8> = general_purpose::STANDARD.decode(s).map_err(|source| {
            Error::MalformedClientPublicKeyEncoding {
                pub_key: s.to_string(),
                source,
            }
        })?;

        let decoded_length = key_bytes.len();
        let Ok(key_arr): Result<[u8; 32], _> = key_bytes.try_into() else {
            return Err(Error::InvalidClientPublicKeyLength {
                pub_key: s.to_string(),
                decoded_length,
            })?;
        };

        Ok(ClientPublicKey(x25519_dalek::PublicKey::from(key_arr)))
    }
}

impl Serialize for ClientMac {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.clone());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> Deserialize<'de> for ClientMac {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        ClientMac::from_str(&encoded_key).map_err(serde::de::Error::custom)
    }
}

impl Serialize for ClientPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.as_bytes());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> Deserialize<'de> for ClientPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        Ok(ClientPublicKey::from_str(&encoded_key).map_err(serde::de::Error::custom))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::encryption;

    #[test]
    #[cfg(feature = "wireguard-verify")]
    fn client_request_roundtrip() {
        let mut rng = rand::thread_rng();

        let gateway_key_pair = encryption::KeyPair::new(&mut rng);
        let client_key_pair = encryption::KeyPair::new(&mut rng);

        let socket: SocketAddr = "1.2.3.4:5678".parse().unwrap();
        let nonce = 1234567890;

        let client = Client::new(
            client_key_pair.private_key(),
            *gateway_key_pair.public_key(),
            socket,
            nonce,
        );
        assert!(client.verify(gateway_key_pair.private_key(), nonce).is_ok())
    }
}
