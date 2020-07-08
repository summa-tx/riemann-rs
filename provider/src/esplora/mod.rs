mod types;
mod utils;

use types::*;
use utils::*;

use async_trait::async_trait;
use futures::lock::Mutex;
use lru::LruCache;
use std::time::Duration;
use thiserror::Error;

use riemann_core::prelude::*;
use rmn_btc::prelude::*;

use crate::{BTCProvider, PollingBTCProvider};

#[cfg(feature = "mainnet")]
static BLOCKSTREAM: &str = "https://blockstream.info/api";

#[cfg(feature = "testnet")]
static BLOCKSTREAM: &str = "https://blockstream.info/testnet/api";

/// An updater that uses the Esplora API and caches responses
#[derive(Debug)]
pub struct EsploraProvider {
    interval: usize,
    api_root: String,
    cache: Mutex<LruCache<TXID, BitcoinTx>>,
}

impl Default for EsploraProvider {
    fn default() -> Self {
        Self::with_api_root(BLOCKSTREAM)
    }
}

impl EsploraProvider {
    /// Instantiate the API pointing at a specific URL
    pub fn with_api_root(api_root: &str) -> Self {
        Self {
            interval: 300,
            api_root: api_root.to_owned(),
            cache: Mutex::new(LruCache::new(100)),
        }
    }

    /// Set the polling interval
    pub fn set_interval(&mut self, interval: usize) {
        self.interval = interval;
    }

    /// Return true if the cache has the tx in it
    pub async fn has_tx(&self, txid: TXID) -> bool {
        self.cache.lock().await.contains(&txid)
    }

    /// Return a reference to the TX, if it's in the cache.
    pub async fn peek_tx(&self, txid: TXID) -> Option<BitcoinTx> {
        self.cache.lock().await.peek(&txid).cloned()
    }
}

/// Enum of errors that can be produced by this updater
#[derive(Debug, Error)]
pub enum EsploraError {
    /// Bubbled up from the Tx Deserializer.
    #[error(transparent)]
    TxError(#[from] rmn_btc::types::transactions::TxError),

    /// Error in networking
    #[error(transparent)]
    FetchError(#[from] utils::FetchError),

    /// Bubbled up from riemann
    #[error(transparent)]
    EncoderError(#[from] rmn_btc::enc::bases::EncodingError),

    /// Bubbled up from Riemann
    #[error(transparent)]
    RmnSerError(#[from] riemann_core::ser::SerError),
}

#[async_trait]
impl BTCProvider for EsploraProvider {
    type Error = EsploraError;

    // async fn tip_hash(&self) -> Result<Hash256Digest, Self::Error> {
    //     let url = format!("{}/blocks/tip/hash", self.api_root);
    //     let response = ez_fetch_string(&url).await?;
    //     let mut digest = Hash256Digest::deserialize_hex(&response)?;
    //     digest.reverse();
    //     Ok(digest)
    // }
    //
    // async fn tip_height(&self) -> Result<usize, Self::Error> {
    //     let url = format!("{}/blocks/tip/height", self.api_root);
    //     let response = ez_fetch_string(&url).await?;
    //     Ok(response.parse().unwrap())
    // }
    //
    // async fn in_best_chain(&self, digest: Hash256Digest) -> Result<BlockStatus, Self::Error> {
    //     let status =
    // }

    async fn get_confs(&self, _txid: TXID) -> Result<Option<usize>, Self::Error> {
        unimplemented!()
    }

    async fn get_tx(&self, txid: TXID) -> Result<Option<BitcoinTx>, Self::Error> {
        if !self.has_tx(txid).await {
            let tx_hex = fetch_tx_hex_by_id(&self.api_root, txid).await?;
            if let Ok(tx) = BitcoinTx::deserialize_hex(&tx_hex) {
                self.cache.lock().await.put(txid, tx);
            }
        }
        Ok(self.cache.lock().await.get(&txid).cloned())
    }

    async fn get_outspend(&self, outpoint: BitcoinOutpoint) -> Result<Option<TXID>, Self::Error> {
        let outspend_opt = Outspend::fetch_by_outpoint(&self.api_root, &outpoint).await?;

        match outspend_opt {
            Some(outspend) => {
                if outspend.spent {
                    let txid = Default::default();
                    Ok(Some(txid))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    async fn get_utxos_by_address(&self, address: &Address) -> Result<Vec<UTXO>, Self::Error> {
        let res: Result<Vec<_>, EsploraError> =
            EsploraUTXO::fetch_by_address(&self.api_root, address)
                .await?
                .into_iter()
                .map(|e| e.into_utxo(address))
                .collect();
        Ok(res?)
    }

    async fn broadcast(&self, tx: BitcoinTx) -> Result<TXID, Self::Error> {
        let url = format!("{}/tx", self.api_root);
        let response = utils::post_hex(&url, tx.serialize_hex()?).await?;
        Ok(TXID::deserialize_hex(&response)?)
    }
}

#[async_trait]
impl PollingBTCProvider for EsploraProvider {
    fn interval(&self) -> Duration {
        Duration::from_secs(self.interval as u64)
    }

    fn set_interval(&mut self, interval: usize) {
        self.interval = interval;
    }
}