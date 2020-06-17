use crate::dai;
use crate::publish::WorthIn;
use num::{BigInt, BigRational, FromPrimitive, Signed, Zero};

pub const SATS_IN_BITCOIN: u64 = 100_000_000;

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

    pub fn as_btc(self) -> BigRational {
        let sats = BigRational::from_u64(self.0.as_sat())
            .expect("should be able to create BigRational from u64");
        sats / BigInt::from(SATS_IN_BITCOIN)
    }
}

impl WorthIn<dai::Amount> for Amount {
    fn worth_in(&self, btc_to_dai_rate: BigRational) -> anyhow::Result<dai::Amount> {
        if btc_to_dai_rate.is_negative() {
            anyhow::bail!("Rate is negative.");
        }

        if btc_to_dai_rate.is_zero() {
            anyhow::bail!("Rate is zero.");
        }

        let worth = btc_to_dai_rate * self.as_btc();

        Ok(dai::Amount::from_rounded_dai(worth)?)
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
    fn using_rate_returns_correct_result() {
        let btc = Amount::from_btc(1.0).unwrap();

        let res: dai::Amount = btc
            .worth_in(BigRational::from_f64(1000.123456789).unwrap())
            .unwrap();

        assert_eq!(
            res,
            dai::Amount::from_rounded_dai(BigRational::from_f64(1000.123456789).unwrap()).unwrap()
        );
    }
}
