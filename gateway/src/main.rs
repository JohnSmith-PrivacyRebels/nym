// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use clap::{crate_version, Parser};
use logging::setup_logging;
use network_defaults::setup_env;
use once_cell::sync::OnceCell;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, filter};
use tracing::*;
use opentelemetry::trace::{TraceError};
use tracing_flame::FlameLayer;
use std::time::Duration;
use std::thread::sleep;



mod commands;
mod config;
mod node;

static LONG_VERSION: OnceCell<String> = OnceCell::new();

// Helper for passing LONG_ABOUT to clap
fn long_version_static() -> &'static str {
    LONG_VERSION.get().expect("Failed to get long about text")
}

#[derive(Parser)]
#[clap(author = "Nymtech", version, about, long_version = long_version_static())]
struct Cli {
    /// Path pointing to an env file that configures the gateway.
    #[clap(short, long)]
    pub(crate) config_env_file: Option<std::path::PathBuf>,

    #[clap(subcommand)]
    command: commands::Commands,
}

#[tokio::main]
async fn main()  -> Result<(), TraceError>{
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_endpoint("143.42.21.138:6831")
        .with_service_name("nym_gateway")
        .install_simple()
        .expect("Failed to initialize tracer");

    let jaeger_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter_layer = filter::filter_fn(|metadata| {metadata.target().starts_with("nym_gateway")});

    let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(flame_layer)
        .with(jaeger_layer);


    tracing::subscriber::set_global_default(subscriber)
     .expect("Failed to set global subscriber");
    //tracing_subscriber::fmt::init();
    //setup_logging();
    println!("{}", banner());
    LONG_VERSION
        .set(long_version())
        .expect("Failed to set long about text");

    let args = Cli::parse();
    setup_env(args.config_env_file.clone());
    commands::execute(args).instrument( info_span!("Executing run command")).await;
    opentelemetry::global::shutdown_tracer_provider();
    println!("Exiting");
    Ok(())
}

fn banner() -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             (gateway - version {:})

    "#,
        crate_version!()
    )
}

fn long_version() -> String {
    format!(
        r#"
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
{:<20}{}
"#,
        "Build Timestamp:",
        env!("VERGEN_BUILD_TIMESTAMP"),
        "Build Version:",
        env!("VERGEN_BUILD_SEMVER"),
        "Commit SHA:",
        env!("VERGEN_GIT_SHA"),
        "Commit Date:",
        env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
        "Commit Branch:",
        env!("VERGEN_GIT_BRANCH"),
        "rustc Version:",
        env!("VERGEN_RUSTC_SEMVER"),
        "rustc Channel:",
        env!("VERGEN_RUSTC_CHANNEL"),
        "cargo Profile:",
        env!("VERGEN_CARGO_PROFILE")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_cli() {
        LONG_VERSION
            .set(long_version())
            .expect("Failed to set long about text");
        Cli::command().debug_assert();
    }
}
