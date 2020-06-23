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

use crate::publish::{BtcDaiOrder, new_btc_dai_sell_order, Spread};
use crate::markets::{Rate, Position, get_rate, TradingPair};
use bitcoin::blockdata::constants::genesis_block;

mod bitcoin_wallet;
mod bitcoind;
mod jsonrpc;
mod markets;
mod ongoing_swaps;
mod publish;

#[cfg(all(test, feature = "test-docker"))]
pub mod test_harness;

lazy_static::lazy_static! {
    pub static ref SECP: ::bitcoin::secp256k1::Secp256k1<::bitcoin::secp256k1::All> =
        ::bitcoin::secp256k1::Secp256k1::new();
}

enum Event {
    // Publish the inital orders
    InitialOrders,

    // When an order expires a new order is published.
    // New orders are only published after the old one has expired.
    OrderExpired(BtcDaiOrder),

    // Probably not needed, because
    // The new rate is just stored in the maker. Nothing else.
    // RateUpdated(Rate),

    // When the order is taken the funds have to be booked as locked.
    OrderTaken(BtcDaiOrder), // Emitted by orderbook => ExecuteOrder

    // TODO: Events needed for book-keeping...
    OrderWasFundedByOtherParty(BtcDaiOrder), // => Bookkeeping, lock up funds in book
    OrderExecutionFinished(BtcDaiOrder), // => Bookkeeping, remove locked up funds (are reflected on balance now)
}

// Given the current, tightly coupled model we don't need these, can re-introduce if we feel they are needed.
// #[derive(Debug, Clone, PartialEq)]
// enum Action {
//     PublishNewOrder(BtcDaiOrder),
//     ExecuteOrder(BtcDaiOrder),
// }

// holds all the state of the application
struct Maker {
    btc_balance: u64,
    dai_balance: u64,
    btc_fee: u64,
    dai_fee: u64,
    btc_locked_funds: u64,
    dai_locked_funds: u64,
    btc_max_sell_amount: u64,
    dai_max_sell_amount: u64,
    mid_market_rate: f64,
    spread: Spread,

    // TODO: Does the maker have to know about the orders as well or is it good enough if only the orderbook knows?
}

async fn publish_order(_order: BtcDaiOrder) {
    unimplemented!()
}

impl Maker {
    pub async fn handle_event(&mut self, event: Event) -> anyhow::Result<()> {
        match event {
            Event::InitialOrders => {
                let sell_rate = get_rate(TradingPair::BtcDai, Position::Sell).await?;
                let _buy_rate = get_rate(TradingPair::BtcDai, Position::Buy).await?;

                let new_order = new_btc_dai_sell_order(self.btc_balance, self.btc_fee, self.btc_locked_funds, self.btc_max_sell_amount, sell_rate.rate, self.spread);
                publish_order(new_order);

                // TODO: also publish buy side (we might want to re-think this concept)
            },
            Event::OrderExpired(order) => {
                let rate = get_rate(TradingPair::BtcDai, order.position).await?;

                // only one trading pair at the moment
                match order.position {
                    Position::Sell => {
                        let new_order = new_btc_dai_sell_order(self.btc_balance, self.btc_fee, self.btc_locked_funds, self.btc_max_sell_amount, rate.rate, self.spread);
                        publish_order(new_order);
                    },
                    Position::Buy => {
                        unimplemented!()
                    }
                }
            }
            Event::OrderTaken(order) => {
                // TODO: Spawn execution - how and when do we "adjust the book" with locked up funds for execution?
                // Adjust books here already? Or at later point (would require reacting to the execution state)

                unimplemented!()
            },
            _ => unimplemented!()
        }

        Ok(())
    }
}

fn main() {
    // Main loop that ties everything together
    async {
        loop {
            unimplemented!()
        }
    };

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_that_an_order_expired_then_publish_new_order() {
        let mut maker = Maker {
            btc_balance: 10,
            dai_balance: 10000,
            btc_fee: 0,
            dai_fee: 0,
            btc_locked_funds: 0,
            dai_locked_funds: 0,
            btc_max_sell_amount: 10,
            dai_max_sell_amount: 10000,
            mid_market_rate: 0.0,
            spread: Spread::new(0.0).unwrap()
        };

        let action = maker.handle_event(Event::InitialOrders);

        // TODO: Given the current model we would not have to check that the orders were added to the orderbook.
        //  Otherwise we have to re-introduce a model where we emit something to be "caught" by a listener (i.e. new order(s) that are caught by some OrderbookHandler)
        // let expected_order = Order {
        //     sell_amount: 10,
        //     buy_amount: 10
        // };
        //
        // assert_eq!(action, Action::PublishNewOrder(expected_order))
    }
}
