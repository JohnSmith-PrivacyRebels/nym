use std::{collections::HashMap, fmt};

use log::{debug, error, info};
use nym_explorer_api_requests::{PrettyDetailedGatewayBond, PrettyDetailedMixNodeBond};
use nym_network_defaults::var_names::EXPLORER_API;
use nym_topology::{
    nym_topology_from_detailed,
    provider_trait::{async_trait, TopologyProvider},
    NymTopology,
};
use nym_validator_client::client::{IdentityKey, MixId};
use rand::{prelude::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use tap::TapOptional;
use url::Url;

use crate::config::GroupBy;

const MIN_NODES_PER_LAYER: usize = 1;

#[cfg(target_arch = "wasm32")]
fn reqwest_client() -> Option<reqwest::Client> {
    reqwest::Client::builder().build().ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn reqwest_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()
}

// TODO: create a explorer-api-client
async fn fetch_mixnodes_from_explorer_api() -> Option<Vec<PrettyDetailedMixNodeBond>> {
    let explorer_api_url = std::env::var(EXPLORER_API).ok()?;
    let explorer_api_url = Url::parse(&explorer_api_url)
        .ok()?
        .join("v1/mix-nodes")
        .ok()?;

    debug!("Fetching: {}", explorer_api_url);
    reqwest_client()?
        .get(explorer_api_url)
        .send()
        .await
        .ok()?
        .json::<Vec<PrettyDetailedMixNodeBond>>()
        .await
        .ok()
}

// TODO: create a explorer-api-client
async fn fetch_gateways_from_explorer_api() -> Option<Vec<PrettyDetailedGatewayBond>> {
    let explorer_api_url = std::env::var(EXPLORER_API).ok()?;
    let explorer_api_url = Url::parse(&explorer_api_url)
        .ok()?
        .join("v1/gateways")
        .ok()?;

    debug!("Fetching: {}", explorer_api_url);
    reqwest_client()?
        .get(explorer_api_url)
        .send()
        .await
        .ok()?
        .json::<Vec<PrettyDetailedGatewayBond>>()
        .await
        .ok()
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum CountryGroup {
    Europe,
    NorthAmerica,
    SouthAmerica,
    Oceania,
    Asia,
    Africa,
    Unknown,
}

impl CountryGroup {
    // We map contry codes into group, which initially are continent codes to a first approximation,
    // but we do it manually to reserve the right to tweak this distribution for our purposes.
    fn new(country_code: &str) -> Self {
        let country_code = country_code.to_uppercase();
        use CountryGroup::*;
        match country_code.as_ref() {
            // Europe
            "AT" => Europe,
            "BG" => Europe,
            "CH" => Europe,
            "CY" => Europe,
            "CZ" => Europe,
            "DE" => Europe,
            "DK" => Europe,
            "ES" => Europe,
            "FI" => Europe,
            "FR" => Europe,
            "GB" => Europe,
            "GR" => Europe,
            "IE" => Europe,
            "IT" => Europe,
            "LT" => Europe,
            "LU" => Europe,
            "LV" => Europe,
            "MD" => Europe,
            "MT" => Europe,
            "NL" => Europe,
            "NO" => Europe,
            "PL" => Europe,
            "RO" => Europe,
            "SE" => Europe,
            "SK" => Europe,
            "TR" => Europe,
            "UA" => Europe,

            // North America
            "CA" => NorthAmerica,
            "MX" => NorthAmerica,
            "US" => NorthAmerica,

            // South America
            "AR" => SouthAmerica,
            "BR" => SouthAmerica,
            "CL" => SouthAmerica,
            "CO" => SouthAmerica,
            "CR" => SouthAmerica,
            "GT" => SouthAmerica,

            // Oceania
            "AU" => Oceania,

            // Asia
            "AM" => Asia,
            "BH" => Asia,
            "CN" => Asia,
            "GE" => Asia,
            "HK" => Asia,
            "ID" => Asia,
            "IL" => Asia,
            "IN" => Asia,
            "JP" => Asia,
            "KH" => Asia,
            "KR" => Asia,
            "KZ" => Asia,
            "MY" => Asia,
            "RU" => Asia,
            "SG" => Asia,
            "TH" => Asia,
            "VN" => Asia,

            // Africa
            "SC" => Africa,
            "UG" => Africa,
            "ZA" => Africa,

            // And group level codes work too
            "EU" => Europe,
            "NA" => NorthAmerica,
            "SA" => SouthAmerica,
            "OC" => Oceania,
            "AS" => Asia,
            "AF" => Africa,

            // And some aliases
            "EUROPE" => Europe,
            "NORTHAMERICA" => NorthAmerica,
            "SOUTHAMERICA" => SouthAmerica,
            "OCEANIA" => Oceania,
            "ASIA" => Asia,
            "AFRICA" => Africa,

            _ => {
                info!("Unknown country code: {}", country_code);
                Unknown
            }
        }
    }
}

impl fmt::Display for CountryGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use CountryGroup::*;
        match self {
            Europe => write!(f, "EU"),
            NorthAmerica => write!(f, "NA"),
            SouthAmerica => write!(f, "SA"),
            Oceania => write!(f, "OC"),
            Asia => write!(f, "AS"),
            Africa => write!(f, "AF"),
            Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::str::FromStr for CountryGroup {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let group = CountryGroup::new(s);
        if group == CountryGroup::Unknown {
            Err(())
        } else {
            Ok(group)
        }
    }
}

impl CountryGroup {
    #[allow(unused)]
    fn known(self) -> Option<CountryGroup> {
        use CountryGroup::*;
        match self {
            Europe | NorthAmerica | SouthAmerica | Oceania | Asia | Africa => Some(self),
            Unknown => None,
        }
    }
}

fn group_mixnodes_by_country_code(
    mixnodes: Vec<PrettyDetailedMixNodeBond>,
) -> HashMap<CountryGroup, Vec<MixId>> {
    mixnodes
        .into_iter()
        .fold(HashMap::<CountryGroup, Vec<MixId>>::new(), |mut acc, m| {
            if let Some(ref location) = m.location {
                let country_code = location.two_letter_iso_country_code.clone();
                let group_code = CountryGroup::new(country_code.as_str());
                let mixnodes = acc.entry(group_code).or_insert_with(Vec::new);
                mixnodes.push(m.mix_id);
            }
            acc
        })
}

fn group_gateways_by_country_code(
    gateways: Vec<PrettyDetailedGatewayBond>,
) -> HashMap<CountryGroup, Vec<IdentityKey>> {
    gateways.into_iter().fold(
        HashMap::<CountryGroup, Vec<IdentityKey>>::new(),
        |mut acc, g| {
            if let Some(ref location) = g.location {
                let country_code = location.two_letter_iso_country_code.clone();
                let group_code = CountryGroup::new(country_code.as_str());
                let gateways = acc.entry(group_code).or_insert_with(Vec::new);
                gateways.push(g.gateway.identity_key)
            }
            acc
        },
    )
}

fn log_mixnode_distribution(mixnodes: &HashMap<CountryGroup, Vec<MixId>>) {
    let mixnode_distribution = mixnodes
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.len()))
        .collect::<Vec<_>>()
        .join(", ");
    debug!("Mixnode distribution - {}", mixnode_distribution);
}

fn log_gateway_distribution(gateways: &HashMap<CountryGroup, Vec<IdentityKey>>) {
    let gateway_distribution = gateways
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v.len()))
        .collect::<Vec<_>>()
        .join(", ");
    debug!("Gateway distribution - {}", gateway_distribution);
}

fn check_layer_integrity(topology: NymTopology) -> Result<(), ()> {
    let mixes = topology.mixes();
    if mixes.keys().len() < 3 {
        error!("Layer is missing in topology!");
        return Err(());
    }
    for (layer, mixnodes) in mixes {
        debug!("Layer {:?} has {} mixnodes", layer, mixnodes.len());
        if mixnodes.len() < MIN_NODES_PER_LAYER {
            error!(
                "There are only {} mixnodes in layer {:?}",
                mixnodes.len(),
                layer
            );
            return Err(());
        }
    }
    Ok(())
}

pub struct GeoAwareTopologyProvider {
    validator_client: nym_validator_client::client::NymApiClient,
    filter_on: GroupBy,
    client_version: String,
}

impl GeoAwareTopologyProvider {
    pub fn new(
        mut nym_api_urls: Vec<Url>,
        client_version: String,
        filter_on: GroupBy,
    ) -> GeoAwareTopologyProvider {
        log::info!(
            "Creating geo-aware topology provider with filter on {}",
            filter_on
        );
        nym_api_urls.shuffle(&mut thread_rng());

        GeoAwareTopologyProvider {
            validator_client: nym_validator_client::client::NymApiClient::new(
                nym_api_urls[0].clone(),
            ),
            filter_on,
            client_version,
        }
    }

    async fn get_topology(&self) -> Option<NymTopology> {
        let mixnodes = match self.validator_client.get_cached_active_mixnodes().await {
            Err(err) => {
                error!("failed to get network mixnodes - {err}");
                return None;
            }
            Ok(mixes) => mixes,
        };

        let gateways = match self.validator_client.get_cached_gateways().await {
            Err(err) => {
                error!("failed to get network gateways - {err}");
                return None;
            }
            Ok(gateways) => gateways,
        };

        // Also fetch mixnodes cached by explorer-api, with the purpose of getting their
        // geolocation.
        debug!("Fetching mixnodes from explorer-api...");
        let Some(mixnodes_from_explorer_api) = fetch_mixnodes_from_explorer_api().await else {
            error!("failed to get mixnodes from explorer-api");
            return None;
        };

        debug!("Fetching gateways from explorer-api...");
        let Some(gateways_from_explorer_api) = fetch_gateways_from_explorer_api().await else {
            error!("failed to get mixnodes from explorer-api");
            return None;
        };

        // Determine what we should filter around
        let filter_on = match self.filter_on {
            GroupBy::CountryGroup(group) => group,
            GroupBy::NymAddress(recipient) => {
                // Convert recipient into a country group by extracting out the gateway part and
                // using that as the country code.
                let gateway = recipient.gateway().to_base58_string();

                // Lookup the location of this gateway by using the location data from the
                // explorer-api
                let gateway_location = gateways_from_explorer_api
                    .iter()
                    .find(|g| g.gateway.identity_key == gateway)
                    .and_then(|g| g.location.clone())
                    .map(|location| location.two_letter_iso_country_code)
                    .tap_none(|| error!("No location found for the gateway: {}", gateway))?;
                debug!(
                    "Filtering on nym-address: {}, with location: {}",
                    recipient, gateway_location
                );

                CountryGroup::new(&gateway_location)
            }
        };
        debug!("Filter group: {}", filter_on);

        // Partition mixnodes_from_explorer_api according to the value of
        // two_letter_iso_country_code.
        // NOTE: we construct the full distribution here, but only use the one we're interested in.
        // The reason we this instead of a straight filter is that this opens up the possibility to
        // complement a small grouping with mixnodes from adjecent countries.
        let mixnode_distribution = group_mixnodes_by_country_code(mixnodes_from_explorer_api);
        log_mixnode_distribution(&mixnode_distribution);

        let gateway_distribution = group_gateways_by_country_code(gateways_from_explorer_api);
        log_gateway_distribution(&gateway_distribution);

        let Some(filtered_mixnode_ids) = mixnode_distribution.get(&filter_on) else {
            error!("no mixnodes found for: {}", filter_on);
            return None;
        };

        let Some(filtered_gateway_ids) = gateway_distribution.get(&filter_on) else {
            error!("no gateways found for: {}", filter_on);
            return None;
        };

        let mixnodes = mixnodes
            .into_iter()
            .filter(|m| filtered_mixnode_ids.contains(&m.mix_id()))
            .collect::<Vec<_>>();

        let gateways = gateways
            .into_iter()
            .filter(|g| filtered_gateway_ids.contains(g.identity()))
            .collect::<Vec<_>>();

        let topology = nym_topology_from_detailed(mixnodes, gateways)
            .filter_system_version(&self.client_version);

        // TODO: return real error type
        check_layer_integrity(topology.clone()).ok()?;

        Some(topology)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait(?Send)]
impl TopologyProvider for GeoAwareTopologyProvider {
    // this will be manually refreshed on a timer specified inside mixnet client config
    async fn get_new_topology(&mut self) -> Option<NymTopology> {
        self.get_topology().await
    }
}
