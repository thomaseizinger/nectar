mod kraken;
use crate::markets::kraken::AskBidRate;
use chrono::{DateTime, Utc};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, strum_macros::Display)]
pub enum TradingPair {
    BtcDai,
}

// TODO: Maybe we should re-think this model again
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Rate {
    trading_pair: TradingPair,
    position: Position,
    rate: f64,
    timestamp: DateTime<Utc>,
}

impl Rate {
    fn from_kraken(ask_bid: AskBidRate, position: Position) -> Rate {
        match position {
            // ask = sell
            Position::Sell => Rate {
                trading_pair: ask_bid.trading_pair,
                position,
                rate: ask_bid.ask,
                timestamp: Utc::now(),
            },
            // buy = bid
            Position::Buy => Rate {
                trading_pair: ask_bid.trading_pair,
                position,
                rate: 1f64 / ask_bid.bid,
                timestamp: Utc::now(),
            },
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, strum_macros::Display)]
pub enum Position {
    Buy,
    Sell,
}

// Only Kraken atm, can be extended to more markets later (and then choosing best rate or whatnot)
pub async fn get_rate(trading_pair: TradingPair, position: Position) -> anyhow::Result<Rate> {
    let ask_bid = kraken::get_ask_bid(trading_pair).await?;
    Ok(Rate::from_kraken(ask_bid, position))
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("no rate found for trading pair {trading_pair} on position {position}")]
pub struct NoRateFound {
    trading_pair: TradingPair,
    position: Position,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_kraken_ask_bid_and_position_sell_returns_sell_rate() {
        let ask_bid = AskBidRate {
            trading_pair: TradingPair::BtcDai,
            ask: 9.000,
            bid: 8.000,
        };

        let rate = Rate::from_kraken(ask_bid, Position::Sell);
        assert_eq!(rate.rate, 9.000)
    }

    #[test]
    fn given_kraken_ask_bid_and_position_buy_returns_buy_rate() {
        let ask_bid = AskBidRate {
            trading_pair: TradingPair::BtcDai,
            ask: 9.000,
            bid: 8.000,
        };

        let rate = Rate::from_kraken(ask_bid, Position::Buy);
        assert_eq!(rate.rate, 0.125)
    }
}
