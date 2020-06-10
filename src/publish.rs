use std::cmp::min;

pub trait LockedFunds {
    fn locked_funds(&self) -> u64;
}

pub trait Balance {
    // TODO: Is it the nominal balance? (balance in BTC or Satoshi)
    fn balance(&self) -> u64;
}

pub trait Fees {
    fn fees(&self) -> u64;
}

// TODO: I would represent this with floats in here and calculate everything using BTC and DAI (because the rate is represented like that too, it will be way easier to understand)
/// order amounts in smallest units (i.e. satoshi/wei)
struct Order {
    // TODO: What are these amounts? Is each amount supposed to be in the respective asset's smallest unit?

    pub sell_amount: u64,
    pub buy_amount: u64,
}

/// Contains a positive percentage value expressed in ratio: 1 is 100%
/// To avoid human errors, the max value is 1.
struct Spread(f64);

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

/// The maker creates an order that defines how much he wants to buy for the amount he is selling.
/// order's buy amount = what the maker wants from a taker
/// order's sell amount = what the maker is offering to a taker
///
/// The function expects the mid_market_rate according to the minimal unit of the sell asset.
///
/// mid_market_rate is set as 1 sell => x buy, where x is the mid_market_rate
///
/// BTC-DAI: When selling 1 BTC we should buy 9000 DAI, mid_market_rate is 1:9000
/// Given BTC:DAI and the rate of 1:9000
///     selling 1.0 BTC with spread_pc of 3% => buy 9270 DAI
///     selling 0.5 BTC with spread_pc of 3% => buy 4635 DAI
/// Given DAI:BTC and a rate of 1:0.0001
///     selling 10000 DAI with spread_pc of 3% => buy 1.03 BTC
///     selling 1000 DAI with spread_pc of 3% => buy 0.103 DAI
///
#[allow(clippy::cast_precision_loss)] // It's ok because it just means we are applying slightly more than the given spread
fn new_order<W, B>(
    sell_wallet: W,
    book: B,
    max_sell_amount: u64,
    mid_market_rate: f64,
    spread: Spread,
) -> Order
where
    W: Balance + Fees,
    B: LockedFunds,
{
    let sell_amount =
        min(sell_wallet.balance() - book.locked_funds(), max_sell_amount) - sell_wallet.fees();

    let rate = mid_market_rate * (1.0 + spread.as_f64());
    // TODO: The ceil() only makes sense if we really use the smallest units and also represent the mid_market_rate relative to both smallest asset units.
    //  Note that we should verify what the orderbook wants from us at some point; but either way we can decide how to represent it in here independent from the orderbook's representation.
    let buy_amount = (sell_amount as f64 * rate).ceil() as u64;

    Order {
        sell_amount,
        buy_amount,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Copy, Clone)]
    struct Book {
        locked_funds: u64,
    }

    #[derive(Copy, Clone)]
    struct Wallet {
        balance: u64,
        fees: u64,
    }

    impl Wallet {
        fn new(balance: u64, fees: u64) -> Wallet {
            Wallet { balance, fees }
        }
    }

    impl Balance for Wallet {
        fn balance(&self) -> u64 {
            self.balance
        }
    }

    impl Fees for Wallet {
        fn fees(&self) -> u64 {
            self.fees
        }
    }

    impl Book {
        fn new(locked_funds: u64) -> Book {
            Book { locked_funds }
        }
    }

    impl LockedFunds for Book {
        fn locked_funds(&self) -> u64 {
            self.locked_funds
        }
    }

    #[test]
    fn given_a_balance_return_order_selling_full_balance() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(0);

        let order = new_order(wallet, book, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 10);
    }

    #[test]
    fn given_a_balance_and_locked_funds_return_order_selling_available_balance() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 100, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 8);
    }

    #[test]
    fn given_an_available_balance_and_a_max_amount_sell_min_of_either() {
        let wallet = Wallet::new(10, 0);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 2);
    }

    #[test]
    fn given_an_available_balance_and_fees_sell_balance_minus_fees() {
        let wallet = Wallet::new(10, 1);

        let book = Book::new(2);

        let order = new_order(wallet, book, 2, 1.0, Spread::new(0.0).unwrap());

        assert_eq!(order.sell_amount, 1);
    }

    #[test]
    fn given_a_rate_return_order_with_both_amounts() {

        let wallet = Wallet::new(1051, 1);
        let book = Book::new(50);

        let order = new_order(wallet, book, 9999, 0.1, Spread::new(0.0).unwrap());

        // 1 Sell => 0.1 Buy
        // 1000 Sell => 100 Buy
        assert_eq!(order.sell_amount, 1000);
        assert_eq!(order.buy_amount, 100);

        let order = new_order(wallet, book, 9999, 10.0, Spread::new(0.0).unwrap());

        // 1 Sell => 10 Buy
        // 1000 Sell => 10000 Buy
        assert_eq!(order.sell_amount, 1000);
        assert_eq!(order.buy_amount, 10000);
    }

    #[test]
    fn given_a_rate_and_spread_return_order_with_both_amounts() {
        let wallet = Wallet::new(1051, 1);

        let book = Book::new(50);

        let order = new_order(wallet, book, 9999, 0.1, Spread::new(0.03).unwrap());

        // 1 Sell => 0.1 Buy
        // 3% spread
        // 1000 Sell => 103 Buy
        assert_eq!(order.sell_amount, 1000);
        // TODO: Why would it lose precision here? - I would not expect it to be 103 here (all results are within natural numbers)
        assert_eq!(order.buy_amount, 104); // Rounding up taking in account precision loss
    }

    #[test]
    fn given_a_rate_and_very_low_spread_return_order_that_is_still_profitable() {
        let wallet = Wallet::new(1051, 1);

        let book = Book::new(50);

        let order = new_order(wallet, book, 9999, 0.1, Spread::new(0.0003).unwrap());

        // 1 Sell => 0.1 Buy
        // 0.003% spread
        // 1000 Sell => 101 Buy
        assert_eq!(order.sell_amount, 1000);
        assert_eq!(order.buy_amount, 101); // Rounding up taking in account precision loss
    }

}
