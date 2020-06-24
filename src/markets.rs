mod kraken;
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, strum_macros::Display)]
pub enum Position {
    Buy,
    Sell,
}

// Only Kraken atm, can be extended to more markets later (and then choosing best rate or whatnot)
pub async fn get_rate(trading_pair: TradingPair, position: Position) -> anyhow::Result<Rate> {

    let ask_bid = kraken::get_ask_bid(trading_pair).await?;

    match position {
        // ask = sell
        Position::Sell => {
            Ok(Rate {
                trading_pair,
                position,
                rate: ask_bid.ask,
                timestamp: Utc::now()
            })
        }
        // buy = bid
        Position::Buy => {
            Ok(Rate {
                trading_pair,
                position,
                rate: 1f64 / ask_bid.bid,
                timestamp: Utc::now()
            })
        }
    }
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("no rate found for trading pair {trading_pair} on position {position}")]
pub struct NoRateFound {
    trading_pair: TradingPair,
    position: Position,
}
