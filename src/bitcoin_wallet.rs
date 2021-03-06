use crate::bitcoind;
use crate::bitcoind::WalletInfoResponse;
use ::bitcoin::hash_types::PubkeyHash;
use ::bitcoin::hashes::Hash;
use ::bitcoin::secp256k1;
use ::bitcoin::secp256k1::constants::SECRET_KEY_SIZE;
use ::bitcoin::Address;
use ::bitcoin::Network;
use bitcoin::{Amount, PrivateKey};
use rand::prelude::*;
use reqwest::Url;

// TODO: Go in its own module
#[derive(Debug, Clone)]
struct Seed([u8; SECRET_KEY_SIZE]);

impl Seed {
    pub fn new() -> Self {
        let mut bytes = [0u8; SECRET_KEY_SIZE];

        rand::thread_rng().fill_bytes(&mut bytes);
        Seed(bytes)
    }

    pub fn secret_key(&self) -> anyhow::Result<secp256k1::SecretKey> {
        Ok(secp256k1::SecretKey::from_slice(&self.0)?)
    }
}

struct Wallet {
    /// The wallet is named `nectar_x` with `x` being the first 4 byte of the public key hash
    name: String,
    bitcoind_client: bitcoind::Client,
    private_key: ::bitcoin::PrivateKey,
}

impl Wallet {
    pub fn new(seed: Seed, url: Url, network: Network) -> anyhow::Result<Wallet> {
        let private_key = ::bitcoin::PrivateKey {
            compressed: true,
            network,
            key: seed.secret_key()?,
        };

        let bitcoind_client = bitcoind::Client::new(url);

        let name = Wallet::gen_name(private_key);

        Ok(Wallet {
            name,
            bitcoind_client,
            private_key,
        })
    }

    pub async fn init(&self) -> anyhow::Result<()> {
        let info = self.info().await;

        // We assume the wallet present with the same name has the
        // same seed, which is fair but could be safer.
        if info.is_err() {
            // TODO: Probably need to protect the wallet with a passphrase
            self.bitcoind_client
                .create_wallet(&self.name, None, Some(true), "".into(), None)
                .await?;

            let wif = self.private_key.to_wif();

            self.bitcoind_client
                .set_hd_seed(&self.name, Some(true), Some(wif))
                .await?;
        }

        Ok(())
    }

    pub async fn info(&self) -> anyhow::Result<WalletInfoResponse> {
        self.bitcoind_client.get_wallet_info(&self.name).await
    }

    pub async fn new_address(&self) -> anyhow::Result<Address> {
        self.bitcoind_client
            .get_new_address(&self.name, None, Some("bech32".into()))
            .await
    }

    pub async fn balance(&self) -> anyhow::Result<Amount> {
        self.bitcoind_client
            .get_balance(&self.name, None, None, None)
            .await
    }

    fn gen_name(private_key: PrivateKey) -> String {
        let mut hash_engine = PubkeyHash::engine();
        private_key
            .public_key(&crate::SECP)
            .write_into(&mut hash_engine);

        let public_key_hash = PubkeyHash::from_engine(hash_engine);

        format!(
            "nectar_{:x}{:x}{:x}{:x}",
            public_key_hash[0], public_key_hash[1], public_key_hash[2], public_key_hash[3]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_random_seed() {
        let _seed = Seed::new();
    }
}

#[cfg(all(test, feature = "test-docker"))]
mod docker_tests {
    use super::*;
    use crate::test_harness::BitcoinBlockchain;
    use testcontainers::clients;

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
        wallet.init().await.unwrap();

        let _address = wallet.new_address().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_from_seed_and_get_balance() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
        wallet.init().await.unwrap();

        let _balance = wallet.balance().await.unwrap();
    }

    #[tokio::test]
    async fn create_bitcoin_wallet_when_already_existing_and_get_address() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        blockchain.init().await.unwrap();

        let seed = Seed::new();
        {
            let wallet =
                Wallet::new(seed.clone(), blockchain.node_url.clone(), Network::Regtest).unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }

        {
            let wallet = Wallet::new(seed, blockchain.node_url.clone(), Network::Regtest).unwrap();
            wallet.init().await.unwrap();

            let _address = wallet.new_address().await.unwrap();
        }
    }
}
