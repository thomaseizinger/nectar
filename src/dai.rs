use crate::publish::WorthIn;
use num::{BigInt, BigRational, BigUint};

pub const ATTOS_IN_DAI: u64 = 1_000_000_000_000_000_000;

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq)]
pub struct Amount(BigUint);

impl Amount {
    /// Note: the dai will be rounded to attodai.
    pub fn from_rounded_dai(dai: BigRational) -> anyhow::Result<Self> {
        let int = (dai * BigInt::from(ATTOS_IN_DAI)).round().to_integer();
        match int.to_biguint() {
            Some(uint) => Ok(Self(uint)),
            None => Err(anyhow::anyhow!("dai is negative.")),
        }
    }

    pub fn from_atto(atto: BigUint) -> Self {
        Amount(atto)
    }

    pub fn as_atto(&self) -> BigUint {
        self.0.clone()
    }
}

impl std::fmt::Debug for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for Amount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl WorthIn<crate::bitcoin::Amount> for Amount {
    fn worth_in(&self, _rhs: BigRational) -> anyhow::Result<crate::bitcoin::Amount> {
        todo!()
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Self) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}
