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

use conquer_once::Lazy;

pub mod bitcoin;
pub mod bitcoin_wallet;
pub mod bitcoind;
pub mod dai;
pub mod float_maths;
pub mod jsonrpc;
pub mod market;
pub mod ongoing_swaps;
pub mod publish;
pub mod rate;
pub mod swap;

pub static SECP: Lazy<::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All>> =
    Lazy::new(::bitcoin::secp256k1::Secp256k1::new);

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

#[cfg(test)]
mod tests {
    use crate::market::{get_rate, Position, TradingPair};
    use crate::rate::{Rate, Spread};
    use anyhow::Context;
    use std::convert::TryInto;

    #[tokio::test]
    #[ignore] // Ignoring because no need to spam Kraken
    async fn publish_btc_dai_sell_order() {
        let spread = Spread::new(500).unwrap(); // TODO: load from config
        println!("{:?}", &spread);
        let rate = get_rate(TradingPair::BtcDai, Position::Sell)
            .await
            .and_then(|rate| {
                println!("{:?}", &rate);
                let rate: anyhow::Result<Rate> = rate.try_into();
                rate
            })
            .and_then(|rate| spread.apply(rate))
            .context("Could not publish rate")
            .unwrap();
        println!("{:?}", &rate);
    }
}
