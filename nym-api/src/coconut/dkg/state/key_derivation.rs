// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::serde_helpers::recovered_keys;
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::types::{DealingIndex, EpochId};
use nym_dkg::{G2Projective, NodeIndex, RecoveredVerificationKeys, Threshold};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use thiserror::Error;

type ReceiverIndex = usize;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DerivationFailure {
    #[error("there were no valid dealings in epoch {epoch_id}")]
    NoValidDealings { epoch_id: EpochId },

    #[error("did not receive sufficient number of dealings for key derivation. got {available} per key fragment whislt the threshold is {threshold}")]
    InsufficientNumberOfDealings {
        available: usize,
        threshold: Threshold,
    },

    #[error("could not recover partial verification keys for index {dealing_index}: {err_msg}")]
    KeyRecoveryFailure {
        dealing_index: DealingIndex,
        err_msg: String,
    },

    #[error("could not decrypt share at index {dealing_index} generated by dealer at index {dealer_index}: {err_msg}")]
    ShareDecryptionFailure {
        dealing_index: DealingIndex,
        dealer_index: NodeIndex,
        err_msg: String,
    },

    #[error("the derived verification key does not match the expected partial elements")]
    MismatchedPartialKey,
}

#[derive(Debug, Clone, Error, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DealerRejectionReason {
    #[error("no dealings were provided")]
    NoDealingsProvided,

    #[error("insufficient number of dealings was provided. got {got} but expected {expected}")]
    InsufficientNumberOfDealingsProvided { got: usize, expected: usize },

    #[error("no [verified] verification key from the previous epoch was available")]
    MissingVerifiedLastEpochKey,

    #[error("the key size from the previous epoch does not match the resharing dealing requirements: {key_size} vs {expected}")]
    LastEpochKeyOfWrongSize { key_size: usize, expected: usize },

    #[error("the dealing at index {index} is malformed: {err_msg}")]
    MalformedDealing {
        index: DealingIndex,
        err_msg: String,
    },

    #[error("the dealing at index {index} is [cryptographically] valid: {err_msg}")]
    InvalidDealing {
        index: DealingIndex,
        err_msg: String,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct KeyDerivationState {
    pub(crate) expected_threshold: Option<Threshold>,

    #[serde(with = "recovered_keys")]
    pub(crate) derived_partials: BTreeMap<DealingIndex, RecoveredVerificationKeys>,

    pub(crate) rejected_dealers: HashMap<Addr, DealerRejectionReason>,

    pub(crate) proposal_id: Option<u64>,

    pub(crate) completed: Option<Result<(), DerivationFailure>>,
}

impl KeyDerivationState {
    pub fn derived_partials_for(&self, receiver_index: ReceiverIndex) -> Option<Vec<G2Projective>> {
        let mut recovered = Vec::new();
        for keys in self.derived_partials.values() {
            // SAFETY:
            // make sure the receiver index of this receiver/dealer is within the size of the derived keys
            if keys.recovered_partials.len() <= receiver_index {
                return None;
            };
            recovered.push(keys.recovered_partials[receiver_index])
        }
        Some(recovered)
    }

    pub fn completed_with_success(&self) -> bool {
        matches!(self.completed, Some(Ok(_)))
    }

    pub fn completion_failure(&self) -> Option<DerivationFailure> {
        self.completed.clone().and_then(Result::err)
    }
}
