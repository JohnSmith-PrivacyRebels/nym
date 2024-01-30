// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::api::{FormattedResponse, OutputParams};
use axum::extract::Query;
use nym_node_requests::api::v1::node::models::NoiseInformation;

/// Returns host information of this node.
#[utoipa::path(
    get,
    path = "/noise",
    context_path = "/api/v1",
    tag = "Node",
    responses(
        (status = 200, content(
            ("application/json" = NoiseInformation),
            ("application/yaml" = NoiseInformation)
        ))
    ),
    params(OutputParams)
)]
pub(crate) async fn noise_information(
    host_information: NoiseInformation,
    Query(output): Query<OutputParams>,
) -> NoiseInformationResponse {
    let output = output.output.unwrap_or_default();
    output.to_response(host_information)
}

pub type NoiseInformationResponse = FormattedResponse<NoiseInformation>;
