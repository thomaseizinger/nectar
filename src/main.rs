#![warn(
    unused_extern_crates,
    missing_debug_implementations,
    missing_copy_implementations,
    rust_2018_idioms,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::fallible_impl_from,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::dbg_macro
)]
#![allow(dead_code)] // To be removed further down the line
#![forbid(unsafe_code)]

use std::process;
use crate::config::Settings;
use crate::cli::Options;
use anyhow::Context;
use structopt::StructOpt;

mod bitcoin_wallet;
mod bitcoind;
mod config;
mod jsonrpc;
mod markets;
mod ongoing_swaps;
mod publish;

mod cli;
mod trace;

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

lazy_static::lazy_static! {
    pub static ref SECP: ::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All> =
        ::bitcoin::secp256k1::Secp256k1::new();
}

fn main() -> anyhow::Result<()> {

    // TODO introduce sub-commands here to properly depict "trade"

    let options = cli::Options::from_args();

    let settings = read_config(&options).and_then(Settings::from_config_file_and_defaults)?;

    if options.dump_config {
        dump_config(settings)?;
        process::exit(0);
    }

    crate::trace::init_tracing(settings.logging.level)?;
    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!(
            "thread panicked at {}: {}",
            panic_info.location().expect("location is always present"),
            panic_info
                .payload()
                .downcast_ref::<String>()
                .unwrap_or(&String::from("no panic message"))
        )
    }));

    // We have to read configuration that is:

    // CONFIGURATION
    // Spread
    // bitcoind
    // infura
    // (logging)

    // FEEDBACK
    // Upon startup the application print the information for Bitcoin, Ethereum for funding.

    // VALIDATION
    // We have to ensure the configured network is the provided one (?)
    // We have to ensure Bitcoin is "in sync" (bitcoind)
    // We have to ensure Etheruem is in sync (infura)

    // Keep in mind
    // - We are always in the role of Bob

    println!("Hello, world!");

    Ok(())
}

#[allow(clippy::print_stdout)] // We cannot use `log` before we have the config file
fn read_config(options: &Options) -> anyhow::Result<config::File> {
    // if the user specifies a config path, use it
    if let Some(path) = &options.config_file {
        eprintln!("Using config file {}", path.display());

        return config::File::read(&path)
            .with_context(|| format!("failed to read config file {}", path.display()));
    }

    // try to load default config
    let default_path = config::default_config_path()?;

    if !default_path.exists() {
        return Ok(config::File::default());
    }

    eprintln!(
        "Using config file at default path: {}",
        default_path.display()
    );

    config::File::read(&default_path)
        .with_context(|| format!("failed to read config file {}", default_path.display()))
}

#[allow(clippy::print_stdout)] // Don't use the logger so its easier to cut'n'paste
fn dump_config(settings: Settings) -> anyhow::Result<()> {
    let file = config::File::from(settings);
    let serialized = toml::to_string(&file)?;
    println!("{}", serialized);
    Ok(())
}
