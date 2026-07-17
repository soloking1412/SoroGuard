use anyhow::{Context, Result};
use ed25519_dalek::{Signer as _, SigningKey};
use sha2::{Digest, Sha256};
use stellar_xdr::{
    AccountId, DecoratedSignature, Hash, Limits, MuxedAccount, PublicKey, Signature, SignatureHint,
    Transaction, TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction,
    Uint256, WriteXdr,
};

/// The keeper's key. It pays the fee and signs the root call. It never has any authority
/// over a user's account; that comes from the user's own policy, keyed to a rule id.
pub struct Signer {
    key: SigningKey,
    public: [u8; 32],
}

impl Signer {
    /// Build from a Stellar secret seed (`S...`), read from the environment by the caller.
    pub fn from_seed(seed: &str) -> Result<Self> {
        let parsed = stellar_strkey::ed25519::PrivateKey::from_string(seed)
            .map_err(|e| anyhow::anyhow!("keeper secret is not a valid S... seed: {e}"))?;
        let key = SigningKey::from_bytes(&parsed.0);
        let public = key.verifying_key().to_bytes();
        Ok(Self { key, public })
    }

    pub fn account_id(&self) -> AccountId {
        AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(self.public)))
    }

    pub fn muxed_account(&self) -> MuxedAccount {
        MuxedAccount::Ed25519(Uint256(self.public))
    }

    pub fn strkey(&self) -> String {
        format!("{}", stellar_strkey::ed25519::PublicKey(self.public))
    }

    /// Sign a transaction for `network_passphrase`, returning the decorated signature to
    /// attach to the envelope.
    pub fn sign(&self, network_passphrase: &str, tx: &Transaction) -> Result<DecoratedSignature> {
        let network_id = Sha256::digest(network_passphrase.as_bytes());
        let payload = TransactionSignaturePayload {
            network_id: Hash(network_id.into()),
            tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
        };

        let hash = Sha256::digest(
            payload
                .to_xdr(Limits::none())
                .context("serializing signature payload")?,
        );
        let signature = self.key.sign(&hash);

        let mut hint = [0u8; 4];
        hint.copy_from_slice(&self.public[28..]);

        Ok(DecoratedSignature {
            hint: SignatureHint(hint),
            signature: Signature(
                signature
                    .to_bytes()
                    .to_vec()
                    .try_into()
                    .expect("64-byte ed25519 signature"),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test seed from the Stellar SEP-0005 vectors. Never used for anything but tests.
    const SEED: &str = "SBGWSG6BTNCKCOB3DIFBGCVMUPQFYPA2G4O34RMTB343OYPXU5DJDVMN";
    const ADDR: &str = "GDRXE2BQUC3AZNPVFSCEZ76NJ3WWL25FYFK6RGZGIEKWE4SOOHSUJUJ6";

    #[test]
    fn derives_the_matching_public_key() {
        let signer = Signer::from_seed(SEED).unwrap();
        assert_eq!(signer.strkey(), ADDR);
    }

    #[test]
    fn rejects_a_key_that_is_not_a_seed() {
        assert!(Signer::from_seed("not-a-seed").is_err());
    }
}
