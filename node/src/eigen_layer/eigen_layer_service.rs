use alloy_primitives::{Address, Bytes, FixedBytes, U256};
use alloy_signer_local::PrivateKeySigner;
use anyhow::Result;
use eigensdk::{
    client_avsregistry::writer::AvsRegistryChainWriter,
    client_elcontracts::{
        reader::ELChainReader,
        writer::{ELChainWriter, Operator},
    },
    crypto_bls::BlsKeyPair,
    logging::{get_logger, get_test_logger},
    testing_utils::m2_holesky_constants::{
        AVS_DIRECTORY_ADDRESS, DELEGATION_MANAGER_ADDRESS, OPERATOR_STATE_RETRIEVER,
        REGISTRY_COORDINATOR, REWARDS_COORDINATOR, SLASHER_ADDRESS, STRATEGY_MANAGER_ADDRESS,
    },
};
use std::{
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct EigenLayerServiceConfig {
    pub bls_secret_key: String,
    pub private_key: String,
    pub provider: String,
}

pub struct EigenLayerService {
    bls_secret_key: String,
    private_key: String,
    provider: String,
}

impl EigenLayerService {
    pub fn new(config: EigenLayerServiceConfig) -> Self {
        EigenLayerService {
            bls_secret_key: config.bls_secret_key,
            private_key: config.private_key,
            provider: config.provider,
        }
    }

    pub fn bls_key_pair(&self) -> Result<BlsKeyPair> {
        Ok(BlsKeyPair::new(self.bls_secret_key.clone())?)
    }

    pub fn private_key_signer(&self) -> Result<PrivateKeySigner> {
        Ok(PrivateKeySigner::from_str(&self.private_key)?)
    }

    pub async fn start(&self) -> Result<()> {
        let el_chain_reader = ELChainReader::new(
            get_test_logger(),
            SLASHER_ADDRESS,
            DELEGATION_MANAGER_ADDRESS,
            AVS_DIRECTORY_ADDRESS,
            self.provider.clone(),
        );

        let signer = self.private_key_signer()?;
        if let Ok(is_registered) = el_chain_reader
            .is_operator_registered(signer.address())
            .await
        {
            if is_registered {
                tracing::info!("operator {:?} already registered!", signer.address());
                return Ok(());
            }
        } else {
            tracing::error!(
                "failed to check operator registration for {:?}!",
                signer.address()
            );
            return Err(anyhow::anyhow!("failed to check operator registration"));
        }

        let el_chain_writer = ELChainWriter::new(
            STRATEGY_MANAGER_ADDRESS,
            REWARDS_COORDINATOR,
            el_chain_reader,
            self.provider.clone(),
            self.private_key.clone(),
        );

        el_chain_writer
            .register_as_operator(Operator {
                address: signer.address(),
                earnings_receiver_address: signer.address(),
                delegation_approver_address: Address::ZERO,
                staker_opt_out_window_blocks: 0u32,
                metadata_url: None,
            })
            .await?;
        // NOTE: this operation fails on holesky as the contract fx interfaces are not the same on mainnet.
        // i also wrote on discord (https://discord.com/channels/1089434273720832071/1187153894564966480/1337107784080162947)

        // Register the operator in registry coordinator
        let avs_registry_writer = AvsRegistryChainWriter::build_avs_registry_chain_writer(
            get_test_logger(),
            self.provider.clone(),
            self.private_key.clone(),
            REGISTRY_COORDINATOR,
            OPERATOR_STATE_RETRIEVER,
        )
        .await?;

        let digest_hash: FixedBytes<32> = FixedBytes::from([0x02; 32]);

        let now = SystemTime::now();
        let mut sig_expiry: U256 = U256::from(0);
        if let Ok(duration_since_epoch) = now.duration_since(UNIX_EPOCH) {
            // Convert the duration to seconds
            let seconds = duration_since_epoch.as_secs(); // Returns a u64
            sig_expiry = U256::from(seconds) + U256::from(86400); // 1 day
        } else {
            tracing::info!("System time seems to be before the UNIX epoch.");
        }
        let quorum_nums = Bytes::from([0x01]);

        tracing::info!(
            "Registering the operator {:?} in the registry coordinator ...",
            signer.address()
        );
        avs_registry_writer
            .register_operator_in_quorum_with_avs_registry_coordinator(
                self.bls_key_pair()?,
                digest_hash,
                sig_expiry,
                quorum_nums,
                "65.109.158.181:33078;31078".to_string(), // socket
            )
            .await?;

        Ok(())
    }
}
