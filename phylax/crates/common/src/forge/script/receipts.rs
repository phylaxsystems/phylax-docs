use super::sequence::ScriptSequence;
use alloy_primitives::TxHash;
use ethers::{prelude::PendingTransaction, providers::Middleware, types::TransactionReceipt};
use ethers_core::types::Chain;
use eyre::Result;
use foundry_common::{units::format_units, RetryProvider};
use foundry_utils::types::{ToAlloy, ToEthers};
use futures::StreamExt;
use phylax_tracing::tracing::{debug, error, trace, warn};
use std::{ops::Mul, sync::Arc};

/// Convenience enum for internal signalling of transaction status
enum TxStatus {
    Dropped,
    Success(TransactionReceipt),
    Revert(TransactionReceipt),
}

impl From<TransactionReceipt> for TxStatus {
    fn from(receipt: TransactionReceipt) -> Self {
        let status = receipt.status.expect("receipt is from an ancient, pre-EIP658 block");
        if status.is_zero() {
            TxStatus::Revert(receipt)
        } else {
            TxStatus::Success(receipt)
        }
    }
}

/// Gets the receipts of previously pending transactions, or removes them from
/// the deploy sequence's pending vector
pub async fn wait_for_pending(
    provider: Arc<RetryProvider>,
    deployment_sequence: &mut ScriptSequence,
) -> Result<()> {
    if deployment_sequence.pending.is_empty() {
        return Ok(());
    }
    println!("##\nChecking previously pending transactions.");
    clear_pendings(provider, deployment_sequence, None).await
}

/// Traverses a set of pendings and either finds receipts, or clears them from
/// the deployment sequnce.
///
/// If no `tx_hashes` are provided, then `deployment_sequence.pending` will be
/// used. For each `tx_hash`, we check if it has confirmed. If it has
/// confirmed, we push the receipt (if successful) or push an error (if
/// revert). If the transaction has not confirmed, but can be found in the
/// node's mempool, we wait for its receipt to be available. If the transaction
/// has not confirmed, and cannot be found in the mempool, we remove it from
/// the `deploy_sequence.pending` vector so that it will be rebroadcast in
/// later steps.
pub async fn clear_pendings(
    provider: Arc<RetryProvider>,
    deployment_sequence: &mut ScriptSequence,
    tx_hashes: Option<Vec<TxHash>>,
) -> Result<()> {
    let to_query = tx_hashes.unwrap_or_else(|| deployment_sequence.pending.clone());

    let count = deployment_sequence.pending.len();

    trace!("Checking status of {count} pending transactions");

    let futs = to_query.iter().copied().map(|tx| check_tx_status(&provider, tx));
    let mut tasks = futures::stream::iter(futs).buffer_unordered(10);

    let mut errors: Vec<String> = vec![];
    let mut receipts = Vec::<TransactionReceipt>::with_capacity(count);

    while let Some((tx_hash, result)) = tasks.next().await {
        match result {
            Err(err) => {
                errors.push(format!("Failure on receiving a receipt for {tx_hash:?}:\n{err}"))
            }
            Ok(TxStatus::Dropped) => {
                // We want to remove it from pending so it will be re-broadcast.
                deployment_sequence.remove_pending(tx_hash);
                errors.push(format!("Transaction dropped from the mempool: {tx_hash:?}"));
            }
            Ok(TxStatus::Success(receipt)) => {
                trace!(tx_hash = ?tx_hash, "received tx receipt");
                deployment_sequence.remove_pending(receipt.transaction_hash.to_alloy());
                receipts.push(receipt);
            }
            Ok(TxStatus::Revert(receipt)) => {
                // consider:
                // if this is not removed from pending, then the script becomes
                // un-resumable. Is this desirable on reverts?
                warn!(tx_hash = ?tx_hash, "Transaction Failure");
                deployment_sequence.remove_pending(receipt.transaction_hash.to_alloy());
                errors.push(format!("Transaction Failure: {:?}", receipt.transaction_hash));
            }
        }
    }

    // sort receipts by blocks asc and index
    receipts.sort_unstable();

    // print all receipts
    for receipt in receipts {
        log_receipt(deployment_sequence.chain.try_into().unwrap(), &receipt);
        deployment_sequence.add_receipt(receipt);
    }

    // print any erros
    if !errors.is_empty() {
        let mut error_msg = errors.join("\n");
        if !deployment_sequence.pending.is_empty() {
            error_msg += "\n\n Add `--resume` to your command to try and continue broadcasting
    the transactions."
        }
        eyre::bail!(error_msg);
    }
    Ok(())
}

/// Prints the transaction receipt details using tracing::debug
///
/// # Arguments
///
/// * `chain` - A Chain instance representing the blockchain
/// * `receipt` - A TransactionReceipt instance representing the transaction receipt
pub fn log_receipt(chain: Chain, receipt: &TransactionReceipt) {
    let gas_used = receipt.gas_used.unwrap_or_default();
    let gas_price = receipt.effective_gas_price.unwrap_or_default();
    let status = if receipt.status.map_or(true, |s| s.is_zero()) { "Failed" } else { "Success" };
    let tx_hash = receipt.transaction_hash;
    let contract_address =
        receipt.contract_address.as_ref().map(|addr| addr.to_alloy().to_checksum(None));
    let block_number = receipt.block_number.unwrap_or_default();
    let gas = if gas_price.is_zero() {
        format!("Gas Used: {gas_used}")
    } else {
        let paid =
            format_units(gas_used.mul(gas_price).to_alloy(), 18).unwrap_or_else(|_| "N/A".into());
        let gas_price = format_units(gas_price.to_alloy(), 9).unwrap_or_else(|_| "N/A".into());
        format!(
            "Paid: {} ETH ({gas_used} gas * {} gwei)",
            paid.trim_end_matches('0'),
            gas_price.trim_end_matches('0').trim_end_matches('.')
        )
    };
    if status == "Failed" {
        error!(
            chain = ?chain,
            status = status,
            tx_hash = ?tx_hash,
            contract_address = ?contract_address,
            block_number = %block_number,
            gas = gas,
            "Transaction Receipt"
        );
    } else {
        debug!(
            chain = ?chain,
            status = status,
            tx_hash = ?tx_hash,
            contract_address = ?contract_address,
            block_number = %block_number,
            gas = gas,
            "Transaction Receipt"
        );
    }
}

/// Checks the status of a txhash by first polling for a receipt, then for
/// mempool inclusion. Returns the tx hash, and a status
async fn check_tx_status(
    provider: &RetryProvider,
    hash: TxHash,
) -> (TxHash, Result<TxStatus, eyre::Report>) {
    // We use the inner future so that we can use ? operator in the future, but
    // still neatly return the tuple
    let result = async move {
        // First check if there's a receipt
        let receipt_opt = provider.get_transaction_receipt(hash.to_ethers()).await?;
        if let Some(receipt) = receipt_opt {
            return Ok(receipt.into());
        }

        // If the tx is present in the mempool, run the pending tx future, and
        // assume the next drop is really really real
        let pending_res = PendingTransaction::new(hash.to_ethers(), provider).await?;
        match pending_res {
            Some(receipt) => Ok(receipt.into()),
            None => Ok(TxStatus::Dropped),
        }
    }
    .await;

    (hash, result)
}
