use std::cmp::min;
use crate::markets::{TradingPair, Position};

pub trait LockedFunds {
    fn locked_funds(&self) -> u64;
}

pub trait Balance {
    fn balance(&self) -> u64;
}

pub trait Fees {
    fn fees(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BtcDaiOrder {
    pub position: Position,
    pub btc: u64,
    pub dai: u64,
}

/// Contains a positive percentage value expressed in ratio: 1 is 100%
/// To avoid human errors, the max value is 1.
#[derive(Debug, Copy, Clone)]
pub struct Spread(f64);

impl Spread {
    pub fn new(spread: f64) -> Result<Spread, ()> {
        if spread.is_sign_positive() && spread <= 1.0 {
            Ok(Spread(spread))
        } else {
            Err(())
        }
    }

    pub fn as_f64(&self) -> f64 {
        self.0
    }
}

/// mid_market_rate is buy/sell: 1 Buy => mid_market_rate Sell: = sell/buy
/// spread_pc: percent value to be added to the buy amount
#[allow(clippy::cast_precision_loss)] // It's ok because it just means we are applying slightly more than the given spread
#[allow(clippy::cast_possible_truncation)] // We probably want to use custom amounts down the line
#[allow(clippy::cast_sign_loss)] // It's ok because all values should be positive
pub fn new_btc_dai_sell_order(
    btc_balance: u64,
    btc_fee: u64,
    btc_locked_funds: u64,
    btc_max_sell_amount: u64,
    mid_market_rate: f64,
    spread: Spread,
) -> BtcDaiOrder
{
    let sell_amount =
        min(btc_balance - btc_locked_funds, btc_max_sell_amount) - btc_fee;

    let rate = mid_market_rate / (1.0 + spread.as_f64());

    let buy_amount = (sell_amount as f64 / rate).ceil() as u64;

    BtcDaiOrder {
        position: Position::Sell,
        btc: sell_amount,
        dai: buy_amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let order = new_btc_dai_sell_order(10, 0, 0, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.btc, 10);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let order = new_btc_dai_sell_order(10, 0, 2, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.btc, 8);
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let order = new_btc_dai_sell_order(10, 0, 2, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.btc, 2);
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let order = new_btc_dai_sell_order(10, 1, 2, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.btc, 1);
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {
        let order = new_btc_dai_sell_order(1051, 1, 50, 9999, 10.0, Spread::new(0.0).unwrap());
        // 1 Buy => 10 Sell
        // ? Buy => 1000 sell
        // 100 Buy => 1000 Sell

        assert_eq!(order.btc, 1000);
        assert_eq!(order.dai, 100)
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let order = new_btc_dai_sell_order(1051, 1, 50, 9999, 10.0, Spread::new(0.03).unwrap());
        // 1 Buy => 10 Sell
        // ? Buy => 1000 sell
        // 100 Buy => 1000 Sell
        // 3% spread
        // 103 Buy => 1000 Sell

        assert_eq!(order.btc, 1000);
        assert_eq!(order.dai, 104); // Rounding up taking in account precision loss
    }
}
