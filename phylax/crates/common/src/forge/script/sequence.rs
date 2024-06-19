use super::NestedValue;
use crate::{
    forge::script::transaction::{wrapper, TransactionWithMetadata},
    git::get_commit_hash,
};
use alloy_primitives::TxHash;
use ethers::{prelude::TransactionReceipt, types::transaction::eip2718::TypedTransaction};
use eyre::{ContextCompat, Result, WrapErr};
use foundry_cli::utils::now;
use foundry_common::{fs, SELECTOR_LEN};
use foundry_compilers::{artifacts::Libraries, ArtifactId};
use foundry_config::Config;

use phylax_tracing::tracing::trace;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

pub const DRY_RUN_DIR: &str = "dry-run";

/// Helper that saves the transactions sequence and its state on which transactions have been
/// broadcasted
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct ScriptSequence {
    pub transactions: VecDeque<TransactionWithMetadata>,
    #[serde(serialize_with = "wrapper::serialize_receipts")]
    pub receipts: Vec<TransactionReceipt>,
    pub libraries: Vec<String>,
    pub pending: Vec<TxHash>,
    #[serde(skip)]
    pub path: PathBuf,
    #[serde(skip)]
    pub sensitive_path: PathBuf,
    pub returns: HashMap<String, NestedValue>,
    pub timestamp: u64,
    pub chain: u64,
    /// If `True`, the sequence belongs to a `MultiChainSequence` and won't save to disk as usual.
    pub multi: bool,
    pub commit: Option<String>,
}

/// Sensitive values from the transactions in a script sequence
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SensitiveTransactionMetadata {
    pub rpc: Option<String>,
}

/// Sensitive info from the script sequence which is saved into the cache folder
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct SensitiveScriptSequence {
    pub transactions: VecDeque<SensitiveTransactionMetadata>,
}

impl From<&mut ScriptSequence> for SensitiveScriptSequence {
    fn from(sequence: &mut ScriptSequence) -> Self {
        SensitiveScriptSequence {
            transactions: sequence
                .transactions
                .iter()
                .map(|tx| SensitiveTransactionMetadata { rpc: tx.rpc.clone() })
                .collect(),
        }
    }
}

impl ScriptSequence {
    pub fn new(
        transactions: VecDeque<TransactionWithMetadata>,
        returns: HashMap<String, NestedValue>,
        sig: &str,
        target: &ArtifactId,
        config: &Config,
        broadcasted: bool,
        is_multi: bool,
    ) -> Result<Self> {
        let chain = config.chain_id.unwrap_or_default().id();

        let (path, sensitive_path) = ScriptSequence::get_paths(
            &config.broadcast,
            &config.cache_path,
            sig,
            target,
            chain,
            broadcasted && !is_multi,
        )?;

        let commit = get_commit_hash(&config.__root.0);

        Ok(ScriptSequence {
            transactions,
            returns,
            receipts: vec![],
            pending: vec![],
            path,
            sensitive_path,
            timestamp: now().as_secs(),
            libraries: vec![],
            chain,
            multi: is_multi,
            commit,
        })
    }

    /// Loads The sequence for the corresponding json file
    pub fn load(
        config: &Config,
        sig: &str,
        target: &ArtifactId,
        chain_id: u64,
        broadcasted: bool,
    ) -> Result<Self> {
        let (path, sensitive_path) = ScriptSequence::get_paths(
            &config.broadcast,
            &config.cache_path,
            sig,
            target,
            chain_id,
            broadcasted,
        )?;

        let mut script_sequence: Self = foundry_compilers::utils::read_json_file(&path)
            .wrap_err(format!("Deployment not found for chain `{chain_id}`."))?;

        let sensitive_script_sequence: SensitiveScriptSequence =
            foundry_compilers::utils::read_json_file(&sensitive_path).wrap_err(format!(
                "Deployment's sensitive details not found for chain `{chain_id}`."
            ))?;

        script_sequence
            .transactions
            .iter_mut()
            .enumerate()
            .for_each(|(i, tx)| tx.rpc = sensitive_script_sequence.transactions[i].rpc.clone());

        script_sequence.path = path;
        script_sequence.sensitive_path = sensitive_path;

        Ok(script_sequence)
    }

    /// Saves the transactions as file if it's a standalone deployment.
    pub fn save(&mut self) -> Result<()> {
        if self.multi || self.transactions.is_empty() {
            return Ok(());
        }

        self.timestamp = now().as_secs();
        let ts_name = format!("run-{}.json", self.timestamp);

        let sensitive_script_sequence: SensitiveScriptSequence = self.into();

        // broadcast folder writes
        //../run-latest.json
        let mut writer = BufWriter::new(fs::create_file(&self.path)?);
        serde_json::to_writer_pretty(&mut writer, &self)?;
        writer.flush()?;
        //../run-[timestamp].json
        fs::copy(&self.path, self.path.with_file_name(&ts_name))?;

        // cache folder writes
        //../run-latest.json
        let mut writer = BufWriter::new(fs::create_file(&self.sensitive_path)?);
        serde_json::to_writer_pretty(&mut writer, &sensitive_script_sequence)?;
        writer.flush()?;
        //../run-[timestamp].json
        fs::copy(&self.sensitive_path, self.sensitive_path.with_file_name(&ts_name))?;

        trace!(target: "forge::script", sensitive_path=%self.sensitive_path.display(), path=%self.path.display(), "transcations saved in filesystem");
        Ok(())
    }

    pub fn add_receipt(&mut self, receipt: TransactionReceipt) {
        self.receipts.push(receipt);
    }

    /// Sorts all receipts with ascending transaction index
    pub fn sort_receipts(&mut self) {
        self.receipts.sort_unstable()
    }

    pub fn add_pending(&mut self, index: usize, tx_hash: TxHash) {
        if !self.pending.contains(&tx_hash) {
            self.transactions[index].hash = Some(tx_hash);
            self.pending.push(tx_hash);
        }
    }

    pub fn remove_pending(&mut self, tx_hash: TxHash) {
        self.pending.retain(|element| element != &tx_hash);
    }

    pub fn add_libraries(&mut self, libraries: Libraries) {
        self.libraries = libraries
            .libs
            .iter()
            .flat_map(|(file, libs)| {
                libs.iter()
                    .map(|(name, address)| format!("{}:{name}:{address}", file.to_string_lossy()))
            })
            .collect();
    }

    /// Gets paths in the formats
    /// ./broadcast/[contract_filename]/[chain_id]/[sig]-[timestamp].json and
    /// ./cache/[contract_filename]/[chain_id]/[sig]-[timestamp].json
    pub fn get_paths(
        broadcast: &Path,
        cache: &Path,
        sig: &str,
        target: &ArtifactId,
        chain_id: u64,
        broadcasted: bool,
    ) -> Result<(PathBuf, PathBuf)> {
        let mut broadcast = broadcast.to_path_buf();
        let mut cache = cache.to_path_buf();
        let mut common = PathBuf::new();

        let target_fname = target.source.file_name().wrap_err("No filename.")?;
        common.push(target_fname);
        common.push(chain_id.to_string());
        if !broadcasted {
            common.push(DRY_RUN_DIR);
        }

        broadcast.push(common.clone());
        cache.push(common);

        fs::create_dir_all(&broadcast)?;
        fs::create_dir_all(&cache)?;

        // TODO: ideally we want the name of the function here if sig is calldata
        let filename = sig_to_file_name(sig);

        broadcast.push(format!("{filename}-latest.json"));
        cache.push(format!("{filename}-latest.json"));

        Ok((broadcast, cache))
    }

    /// Returns the list of the transactions without the metadata.
    pub fn typed_transactions(&self) -> Vec<(String, &TypedTransaction)> {
        self.transactions
            .iter()
            .map(|tx| {
                (tx.rpc.clone().expect("to have been filled with a proper rpc"), tx.typed_tx())
            })
            .collect()
    }
}

impl Drop for ScriptSequence {
    fn drop(&mut self) {
        self.sort_receipts();
        self.save().expect("not able to save deployment sequence");
    }
}

/// Converts the `sig` argument into the corresponding file path.
///
/// This accepts either the signature of the function or the raw calldata

fn sig_to_file_name(sig: &str) -> String {
    if let Some((name, _)) = sig.split_once('(') {
        // strip until call argument parenthesis
        return name.to_string();
    }
    // assume calldata if `sig` is hex
    if let Ok(calldata) = hex::decode(sig) {
        // in which case we return the function signature
        return hex::encode(&calldata[..SELECTOR_LEN]);
    }

    // return sig as is
    sig.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_convert_sig() {
        assert_eq!(sig_to_file_name("run()").as_str(), "run");
        assert_eq!(
            sig_to_file_name(
                "522bb704000000000000000000000000f39fd6e51aad88f6f4ce6ab8827279cfFFb92266"
            )
            .as_str(),
            "522bb704"
        );
    }
}
