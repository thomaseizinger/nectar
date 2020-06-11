use crate::dai;
use crate::dai::ATTOS_IN_DAI_EXP;
use crate::publish::WorthIn;
use num::pow::Pow;
use num::BigUint;

pub const SATS_IN_BITCOIN_EXP: u16 = 8;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub struct Amount(::bitcoin::Amount);

impl Amount {
    pub fn from_btc(btc: f64) -> anyhow::Result<Amount> {
        Ok(Amount(::bitcoin::Amount::from_btc(btc)?))
    }

    pub fn from_sat(sat: u64) -> Self {
        Amount(::bitcoin::Amount::from_sat(sat))
    }

    pub fn as_sat(self) -> u64 {
        self.0.as_sat()
    }

    pub fn as_btc(self) -> f64 {
        self.0.as_btc()
    }
}

impl WorthIn<dai::Amount> for Amount {
    const MAX_PRECISION_EXP: u16 = 9;

    fn worth_in(&self, btc_to_dai_rate: f64) -> anyhow::Result<dai::Amount> {
        // Conversion rate must be positive
        // Ensure there is no cast sign loss
        if btc_to_dai_rate <= 0.0 {
            anyhow::bail!("Rate is negative or null.");
        }

        if btc_to_dai_rate.is_infinite() {
            anyhow::bail!("Rate is infinite.");
        }

        // Avoiding float calculations, make conversion rate an integer
        let rate = btc_to_dai_rate * (10.0f64.powi(Self::MAX_PRECISION_EXP as i32));

        // If the fraction is not null then
        // It means the rate passed had a precision higher than expected.
        // Bailing here instead of purposefully loosing precision.
        // This ensures there is no possible truncation
        if rate.fract() > 0.0 {
            anyhow::bail!("Rate's precision is too high, truncation would ensue.");
        }

        // We can now truncate to integer without loosing precision.
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        let rate = BigUint::from(rate.trunc() as u64);

        // Apply the rate
        let worth = rate * self.0.as_sat();

        // The rate input is for bitcoin to dai but we applied to satoshis so we need to:
        // - divide to get bitcoins
        // - divide to adjust for max_precision
        // - multiple to get attodai
        let adjustment_exp =
            BigUint::from(ATTOS_IN_DAI_EXP - Self::MAX_PRECISION_EXP - SATS_IN_BITCOIN_EXP);

        let adjustment = BigUint::from(10u64).pow(adjustment_exp);

        let atto_dai = worth * adjustment;

        Ok(dai::Amount::from_atto(atto_dai))
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn using_too_precise_rate_returns_error() {
        let btc = Amount::from_btc(1.0).unwrap();

        let res: anyhow::Result<dai::Amount> = btc.worth_in(1000.1234567891);

        assert!(res.is_err())
    }

    #[test]
    fn using_rate_returns_correct_result() {
        let btc = Amount::from_btc(1.0).unwrap();

        let res: dai::Amount = btc.worth_in(1000.123456789).unwrap();

        assert_eq!(res, dai::Amount::from_rounded_dai(1000.123456789));
    }
}
