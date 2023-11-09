// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::authenticated::RequestHandlingError;
use chrono::Utc;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use nym_api_requests::coconut::VerifyCredentialBody;
use nym_compact_ecash::scheme::EcashCredential;
use nym_compact_ecash::setup::Parameters;
use nym_compact_ecash::{PayInfo, VerificationKeyAuth};
use nym_validator_client::coconut::all_ecash_api_clients;
use nym_validator_client::{
    nyxd::contract_traits::DkgQueryClient, CoconutApiClient, DirectSigningHttpRpcNyxdClient,
};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::time::{interval, Duration};

const TIME_RANGE_SEC: i64 = 30;

pub(crate) struct EcashVerifier {
    nyxd_client: DirectSigningHttpRpcNyxdClient,
    ecash_parameters: Parameters,
    pk_bytes: [u8; 32], //bytes represenation of a pub key representing the verifier
    pay_infos: Arc<Mutex<Vec<PayInfo>>>,
    cred_sender: UnboundedSender<CredentialToBeSent>,
}

impl EcashVerifier {
    pub fn new(
        nyxd_client: DirectSigningHttpRpcNyxdClient,
        ecash_parameters: Parameters,
        pk_bytes: [u8; 32],
        shutdown: nym_task::TaskClient,
    ) -> Self {
        let (cred_sender, cred_receiver) = mpsc::unbounded();
        let cs = CredentialSender::new(cred_receiver);
        cs.start(shutdown);

        EcashVerifier {
            nyxd_client,
            ecash_parameters,
            pk_bytes,
            pay_infos: Arc::new(Mutex::new(Vec::new())),
            cred_sender,
        }
    }

    pub async fn all_current_ecash_api_clients(
        &self,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        let epoch_id = self.nyxd_client.get_current_epoch().await?.epoch_id;
        self.all_ecash_api_clients(epoch_id).await
    }

    pub async fn all_ecash_api_clients(
        &self,
        epoch_id: u64,
    ) -> Result<Vec<CoconutApiClient>, RequestHandlingError> {
        Ok(all_ecash_api_clients(&self.nyxd_client, epoch_id).await?)
    }

    //Check for duplicate pay_info, then check the payment, then insert pay_info if everything succeeded
    pub async fn check_payment(
        &self,
        credential: &EcashCredential,
        aggregated_verification_key: &VerificationKeyAuth,
    ) -> Result<(), RequestHandlingError> {
        let insert_index = self.verify_pay_info(credential.pay_info().clone()).await?;

        credential
            .payment()
            .spend_verify(
                &self.ecash_parameters,
                aggregated_verification_key,
                credential.pay_info(),
            )
            .map_err(|_| {
                RequestHandlingError::InvalidBandwidthCredential(String::from(
                    "credential failed to verify on gateway",
                ))
            })?;

        self.insert_pay_info(credential.pay_info().clone(), insert_index)
            .await
    }

    pub async fn verify_pay_info(&self, pay_info: PayInfo) -> Result<usize, RequestHandlingError> {
        //Public key check
        if pay_info.pk() != self.pk_bytes {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway - invalid provider pk"),
            ));
        }

        //Timestamp range check
        let timestamp = Utc::now().timestamp();
        let tmin = timestamp - TIME_RANGE_SEC;
        let tmax = timestamp + TIME_RANGE_SEC;
        if pay_info.timestamp() > tmax || pay_info.timestamp() < tmin {
            return Err(RequestHandlingError::InvalidBandwidthCredential(
                String::from("credential failed to verify on gateway - invalid timestamp"),
            ));
        }

        let mut inner = self
            .pay_infos
            .lock()
            .map_err(|_| RequestHandlingError::InternalError)?;

        //Cleanup inner
        let low = inner.partition_point(|x| x.timestamp() < tmin);
        let high = inner.partition_point(|x| x.timestamp() < tmax);
        inner.truncate(high);
        drop(inner.drain(..low));

        //Duplicate check
        match inner.binary_search_by(|info| info.timestamp().cmp(&pay_info.timestamp())) {
            Result::Err(index) => Ok(index),
            Result::Ok(index) => {
                if inner[index] == pay_info {
                    return Err(RequestHandlingError::InvalidBandwidthCredential(
                        String::from("credential failed to verify on gateway - duplicate payInfo"),
                    ));
                }
                //tbh, I don't expect ending up here if all parties are honest
                //binary search returns an arbitrary match, so we have to check for potential multiple matches
                let mut i = index as i64;
                while i >= 0 && inner[i as usize].timestamp() == pay_info.timestamp() {
                    if inner[i as usize] == pay_info {
                        return Err(RequestHandlingError::InvalidBandwidthCredential(
                            String::from(
                                "credential failed to verify on gateway - duplicate payInfo",
                            ),
                        ));
                    }
                    i -= 1;
                }

                let mut i = index + 1;
                while i < inner.len() && inner[i].timestamp() == pay_info.timestamp() {
                    if inner[i] == pay_info {
                        return Err(RequestHandlingError::InvalidBandwidthCredential(
                            String::from(
                                "credential failed to verify on gateway - duplicate payInfo",
                            ),
                        ));
                    }
                    i += 1;
                }
                Ok(index)
            }
        }
    }

    pub async fn insert_pay_info(
        &self,
        pay_info: PayInfo,
        index: usize,
    ) -> Result<(), RequestHandlingError> {
        let mut inner = self
            .pay_infos
            .lock()
            .map_err(|_| RequestHandlingError::InternalError)?;
        if index > inner.len() {
            inner.push(pay_info);
            return Ok(());
        }
        inner.insert(index, pay_info);
        Ok(())
    }

    pub fn post_credential(
        &self,
        api_clients: Vec<CoconutApiClient>,
        credential: EcashCredential,
    ) -> Result<(), RequestHandlingError> {
        let req = nym_api_requests::coconut::VerifyCredentialBody::new(
            credential.clone(),
            proposal_id,
            self.nyxd_client.address(),
        );

        for client in api_clients {
            self.cred_sender
                .unbounded_send(CredentialToBeSent(req.clone(), client))
                .map_err(|_| RequestHandlingError::InternalError)?
        }
        Ok(())
    }
}

#[derive(Clone)]
struct CredentialToBeSent(VerifyCredentialBody, CoconutApiClient);
struct CredentialSender {
    pending: VecDeque<CredentialToBeSent>,
    cred_receiver: UnboundedReceiver<CredentialToBeSent>,
}

impl CredentialSender {
    fn new(cred_receiver: UnboundedReceiver<CredentialToBeSent>) -> Self {
        CredentialSender {
            pending: VecDeque::new(),
            cred_receiver,
        }
    }

    async fn handle_credential(&mut self, credential: CredentialToBeSent) {
        self.pending.push_back(credential);
        self.try_empty_pending().await;
    }
    async fn try_empty_pending(&mut self) {
        let mut new_pending = VecDeque::new();
        for credential in &self.pending {
            match credential
                .1
                .api_client
                .verify_bandwidth_credential(&credential.0)
                .await
            {
                Ok(res) => {
                    if !res.verification_result {
                        log::debug!(
                            "Validator {} didn't accept the credential.",
                            credential.1.api_client.nym_api.current_url()
                        );
                    }
                }
                Err(e) => {
                    log::warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", credential
                    .1.api_client.nym_api.current_url(), e);
                    new_pending.push_back(credential.clone());
                }
            }
        }
        self.pending = new_pending;
    }

    fn start(self, shutdown: nym_task::TaskClient) {
        tokio::spawn(async move { self.run(shutdown).await });

        //spawn a new thread that tries to send all pending Credentials.
        //if it cannot send something, back of the queue and retry in 5min
        //if everything has been sent, stop running
    }

    async fn run(mut self, mut shutdown: nym_task::TaskClient) {
        log::debug!("Starting Ecash CredentialSender");
        let mut interval = interval(Duration::from_secs(300));

        while !shutdown.is_shutdown() {
            tokio::select! {
                biased;
                _ = shutdown.recv() => {
                    log::trace!("client_handling::credentialSender : received shutdown");
                },
                Some(credential) = self.cred_receiver.next() => self.handle_credential(credential).await,
                _ = interval.tick(), if !self.pending.is_empty() => self.try_empty_pending().await,

            }
        }
    }
}

//let ret = client.api_client.verify_bandwidth_credential(&req).await;

// match ret {
//     Ok(res) => {
//         if !res.verification_result {
//             debug!(
//                 "Validator {} didn't accept the credential.",
//                 client.api_client.nym_api.current_url()
//             );
//         }
//     }
//     Err(e) => {
//         warn!("Validator {} could not be reached. There might be a problem with the coconut endpoint - {:?}", client.api_client.nym_api.current_url(), e);
//     }
// }
