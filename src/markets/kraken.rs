use crate::markets;
use crate::markets::TradingPair;
use serde::de::Error;
use serde::Deserialize;
use std::convert::TryFrom;

/// Fetch Ticker data
/// More info here: https://www.kraken.com/features/api
pub async fn get_ask_bid(trading_pair: TradingPair) -> anyhow::Result<AskBidRate> {
    let trading_pair_code = get_trading_pair_code(trading_pair);

    let request_url = format!(
        "https://api.kraken.com/0/public/Ticker?pair={trading_pair}",
        trading_pair = trading_pair_code,
    );

    let response = reqwest::get(&request_url)
        .await?
        .json::<TickerResponse>()
        .await?;

    let ticker_data = response.result.xbtdai;
    let ask_bid = AskBid::try_from(ticker_data)?;

    Ok(AskBidRate {
        trading_pair,
        ask: ask_bid.ask,
        bid: ask_bid.bid,
    })
}

#[derive(Deserialize)]
struct TickerResponse {
    result: Ticker,
}

#[derive(Deserialize)]
struct Ticker {
    #[serde(rename = "XBTDAI")]
    xbtdai: TickerData,
}

#[derive(Deserialize)]
struct TickerData {
    #[serde(rename = "a")]
    ask: Vec<String>,
    #[serde(rename = "b")]
    bid: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(try_from = "TickerData")]
struct AskBid {
    pub ask: f64,
    pub bid: f64,
}

pub struct AskBidRate {
    pub trading_pair: TradingPair,
    pub ask: f64,
    pub bid: f64,
}

impl TryFrom<TickerData> for AskBid {
    type Error = serde_json::Error;

    fn try_from(value: TickerData) -> Result<Self, Self::Error> {
        let ask_price = value
            .ask
            .first()
            .ok_or(serde_json::Error::custom("no ask price"))?;
        let bid_price = value
            .bid
            .first()
            .ok_or(serde_json::Error::custom("no ask price"))?;

        Ok(AskBid {
            ask: ask_price
                .parse::<f64>()
                .map_err(serde_json::Error::custom)?,
            bid: bid_price
                .parse::<f64>()
                .map_err(serde_json::Error::custom)?,
        })
    }
}

fn get_trading_pair_code(trading_pair: TradingPair) -> String {
    match trading_pair {
        TradingPair::BtcDai => "XBTDAI".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TICKER_EXAMPLE: &str = r#"{
    "error": [],
    "result": {
        "XBTDAI": {
            "a": [
                "9489.50000",
                "1",
                "1.000"
            ],
            "b": [
                "9462.70000",
                "1",
                "1.000"
            ],
            "c": [
                "9496.50000",
                "0.00220253"
            ],
            "v": [
                "0.19793959",
                "0.55769847"
            ],
            "p": [
                "9583.44469",
                "9593.15707"
            ],
            "t": [
                12,
                22
            ],
            "l": [
                "9496.50000",
                "9496.50000"
            ],
            "h": [
                "9594.90000",
                "9616.10000"
            ],
            "o": "9562.30000"
        }
    }
}"#;

    #[test]
    fn given_ticker_example_data_deserializes_correctly() {
        let xbt_dai = serde_json::from_str::<TickerResponse>(TICKER_EXAMPLE)
            .unwrap()
            .result
            .xbtdai;
        assert!(AskBid::try_from(xbt_dai).is_ok());
    }
}
