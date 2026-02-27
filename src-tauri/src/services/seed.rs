/// Generate a new BIP39 seed phrase (internal version)
pub fn generate_seed_internal() -> Result<String, String> {
    use bip39::{Language, Mnemonic};

    let mut entropy = [0u8; 32];
    getrandom::getrandom(&mut entropy)
        .map_err(|e| format!("Failed to generate random bytes: {}", e))?;

    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    Ok(mnemonic.to_string())
}
