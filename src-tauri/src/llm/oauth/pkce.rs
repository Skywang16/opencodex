use super::types::{OAuthResult, PkceCodes};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};

/// Generate PKCE code pair (S256 method)
pub fn generate_pkce() -> OAuthResult<PkceCodes> {
    // Generate 32-byte random code_verifier
    let mut verifier_bytes = [0u8; 32];
    getrandom::getrandom(&mut verifier_bytes).map_err(|e| {
        super::types::OAuthError::Other(format!("Failed to generate random bytes: {e}"))
    })?;

    let code_verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);

    // Generate code_challenge (SHA256 hash)
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let challenge_bytes = hasher.finalize();
    let code_challenge = URL_SAFE_NO_PAD.encode(challenge_bytes);

    Ok(PkceCodes {
        verifier: code_verifier,
        challenge: code_challenge,
    })
}

/// Generate random state parameter (CSRF protection)
pub fn generate_state() -> OAuthResult<String> {
    let mut state_bytes = [0u8; 32];
    getrandom::getrandom(&mut state_bytes).map_err(|e| {
        super::types::OAuthError::Other(format!("Failed to generate random bytes: {e}"))
    })?;

    Ok(URL_SAFE_NO_PAD.encode(state_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_pkce() {
        let pkce = generate_pkce().unwrap();
        assert!(!pkce.verifier.is_empty());
        assert!(!pkce.challenge.is_empty());
        assert_ne!(pkce.verifier, pkce.challenge);
    }

    #[test]
    fn test_generate_state() {
        let state = generate_state().unwrap();
        assert!(!state.is_empty());

        // Test that multiple generations produce different states
        let state2 = generate_state().unwrap();
        assert_ne!(state, state2);
    }
}
