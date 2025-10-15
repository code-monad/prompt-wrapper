use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::{json, Value};

use crate::config::BitcoinConfig;

#[derive(Clone, Debug)]
pub struct BitcoinData {
    pub btcpay_height: u64,
    pub rpc_blocks: Option<u64>,
    pub rpc_difficulty: Option<f64>,
    pub raw_btcpay: Value,
    pub raw_rpc: Option<Value>,
}

#[derive(Clone)]
pub struct BitcoinService {
    client: Client,
    config: BitcoinConfig,
}

impl BitcoinService {
    pub fn new(config: BitcoinConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn fetch_data(&self) -> Result<BitcoinData> {
        let btcpay_value = self.fetch_btcpay_info().await?;
        let btcpay_height = extract_btcpay_height(&btcpay_value)?;

        let rpc_value = match self.fetch_rpc_blockchain_info().await {
            Ok(value) => Some(value),
            Err(err) => {
                tracing::warn!("Failed to fetch Bitcoin RPC data: {}", err);
                None
            }
        };

        let (rpc_blocks, rpc_difficulty) = rpc_value
            .as_ref()
            .map(|value| {
                (
                    value
                        .get("result")
                        .and_then(|result| result.get("blocks"))
                        .and_then(Value::as_u64),
                    value
                        .get("result")
                        .and_then(|result| result.get("difficulty"))
                        .and_then(Value::as_f64),
                )
            })
            .unwrap_or((None, None));

        Ok(BitcoinData {
            btcpay_height,
            rpc_blocks,
            rpc_difficulty,
            raw_btcpay: btcpay_value,
            raw_rpc: rpc_value,
        })
    }

    async fn fetch_btcpay_info(&self) -> Result<Value> {
        let response = self
            .client
            .get(&self.config.btcpay_api_url)
            .header(
                "Authorization",
                format!("token {}", self.config.btcpay_api_key),
            )
            .send()
            .await
            .context("Failed to request BTCPay Server info")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "BTCPay Server returned status {}",
                response.status()
            ));
        }

        response
            .json::<Value>()
            .await
            .context("Failed to parse BTCPay Server response")
    }

    async fn fetch_rpc_blockchain_info(&self) -> Result<Value> {
        let response = self
            .client
            .post(&self.config.rpc_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "id": "orange-pill",
                "method": "getblockchaininfo",
                "params": [],
            }))
            .send()
            .await
            .context("Failed to request Bitcoin RPC data")?;

        if !response.status().is_success() {
            return Err(anyhow!("Bitcoin RPC returned status {}", response.status()));
        }

        response
            .json::<Value>()
            .await
            .context("Failed to parse Bitcoin RPC response")
    }
}

fn extract_btcpay_height(value: &Value) -> Result<u64> {
    let sync_status = value
        .get("syncStatus")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("BTCPay response missing syncStatus array"))?;

    for entry in sync_status {
        let payment_method = entry
            .get("paymentMethodId")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if payment_method == "BTC-CHAIN" {
            if let Some(height) = entry.get("chainHeight").and_then(Value::as_u64) {
                return Ok(height);
            }
            if let Some(sync_height) = entry.get("syncHeight").and_then(Value::as_u64) {
                return Ok(sync_height);
            }
            if let Some(node_info) = entry.get("nodeInformation") {
                if let Some(headers) = node_info.get("headers").and_then(Value::as_u64) {
                    return Ok(headers);
                }
                if let Some(blocks) = node_info.get("blocks").and_then(Value::as_u64) {
                    return Ok(blocks);
                }
            }
        }
    }

    Err(anyhow!(
        "Unable to extract BTC chain height from BTCPay response"
    ))
}
