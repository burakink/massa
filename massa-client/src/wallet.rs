use crate::ReplData;
use crate::ReplError;
use crypto::hash::Hash;
use crypto::signature::{derive_public_key, PrivateKey};
use models::Address;
use models::Operation;
use models::OperationContent;
use models::OperationType;
use models::SerializeCompact;
use models::Slot;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// contains the private keys created in the wallet.
#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet {
    keys: Vec<PrivateKey>,
    wallet_path: String,
}

impl Wallet {
    /// Generates a new wallet initialized with the provided json file content
    pub fn new(json_file: &str) -> Result<Wallet, ReplError> {
        let path = std::path::Path::new(json_file);
        let keys = if path.exists() {
            serde_json::from_str::<Vec<PrivateKey>>(&std::fs::read_to_string(path)?)?
        } else {
            Vec::new()
        };
        Ok(Wallet {
            keys,
            wallet_path: json_file.to_string(),
        })
    }

    /// Adds a new private key to wallet, if it was missing
    pub fn add_private_key(&mut self, key: PrivateKey) -> Result<(), ReplError> {
        if self
            .keys
            .iter()
            .find(|file_key| file_key == &&key)
            .is_none()
        {
            self.keys.push(key);
            self.save()?;
        }
        Ok(())
    }

    /// Finds the private key associated with given address
    pub fn find_associated_private_key(&self, address: Address) -> Option<&PrivateKey> {
        self.keys.iter().find(|priv_key| {
            let pub_key = crypto::derive_public_key(&priv_key);
            Address::from_public_key(&pub_key)
                .map(|addr| if addr == address { true } else { false })
                .unwrap_or(false)
        })
    }

    pub fn get_wallet_address_list(&self) -> HashSet<Address> {
        self.keys
            .iter()
            .map(|key| {
                let public_key = derive_public_key(&key);
                Address::from_public_key(&public_key).unwrap() //private key has been tested: should never panic
            })
            .collect()
    }

    //save the wallet in json format in a file
    fn save(&self) -> Result<(), ReplError> {
        std::fs::write(&self.wallet_path, self.to_json_string()?)?;
        Ok(())
    }

    /// Export keys to json string
    pub fn to_json_string(&self) -> Result<String, ReplError> {
        serde_json::to_string_pretty(&self.keys).map_err(|err| err.into())
    }

    pub fn create_operation(
        &self,
        operation_type: OperationType,
        from_address: Address,
        fee: u64,
        data: &ReplData,
    ) -> Result<Operation, ReplError> {
        //get node serialisation context
        let url = format!("http://{}/api/v1/node_config", data.node_ip);
        let resp = reqwest::blocking::get(&url)?;
        if resp.status() != StatusCode::OK {
            return Err(ReplError::GeneralError(format!(
                "Error during node connection. Server response code: {}",
                resp.status()
            )));
        }
        let context = resp.json::<models::SerializationContext>()?;

        // Set the context for the client process.
        models::init_serialization_context(context);

        //get pool config
        /*        let url = format!("http://{}/api/v1/pool_config", data.node_ip);
        let resp = reqwest::blocking::get(&url)?;
        if resp.status() != StatusCode::OK {
            return Err(ReplError::GeneralError(format!(
                "Error during node connection. Server answer code :{}",
                resp.status()
            )));
        }
        let pool_cfg = resp.json::<pool::PoolConfig>()?;*/
        //get consensus config
        let url = format!("http://{}/api/v1/consensus_config", data.node_ip);
        let resp = reqwest::blocking::get(&url)?;
        if resp.status() != StatusCode::OK {
            return Err(ReplError::GeneralError(format!(
                "Error during node connection. Server response code: {}",
                resp.status()
            )));
        }
        let consensus_cfg = resp.json::<crate::data::ConsensusConfig>()?;

        //get from address private key
        let private_key =
            self.find_associated_private_key(from_address)
                .ok_or(ReplError::GeneralError(format!(
                    "No private key found in the wallet for the specified FROM address: {}",
                    from_address.to_string()
                )))?;
        let public_key = derive_public_key(&private_key);

        let slot = consensus::get_current_latest_block_slot(
            consensus_cfg.thread_count,
            consensus_cfg.t0,
            consensus_cfg.genesis_timestamp,
            0,
        )
        .map_err(|err| {
            ReplError::GeneralError(format!(
                "Error during current time slot computation: {}",
                err
            ))
        })?
        .unwrap_or(Slot::new(0, 0));

        let mut expire_period = slot.period + consensus_cfg.operation_validity_periods;
        if slot.thread >= from_address.get_thread(consensus_cfg.thread_count) {
            expire_period += 1;
        }

        let operation_content = OperationContent {
            fee,
            expire_period,
            sender_public_key: public_key,
            op: operation_type,
        };

        let hash = Hash::hash(&operation_content.to_bytes_compact().unwrap());
        let signature = crypto::sign(&hash, &private_key).unwrap();

        Ok(Operation {
            content: operation_content,
            signature,
        })
    }
}

impl std::fmt::Display for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Wallet private key list:")?;
        for key in &self.keys {
            let public_key = derive_public_key(&key);
            let addr = Address::from_public_key(&public_key).map_err(|_| std::fmt::Error)?;
            writeln!(f, "key:{} public:{} addr:{}", key, public_key, addr)?;
        }
        Ok(())
    }
}