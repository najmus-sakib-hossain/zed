//! Sign Command
//!
//! Ed25519 signing for .drv binary rules.

use crate::security::{Ed25519Signer, KeyPair};
use crate::{DrivenError, Result};
use console::style;
use std::path::Path;

/// Sign command for cryptographic rule signing
#[derive(Debug)]
pub struct SignCommand;

impl SignCommand {
    /// Execute sign command
    pub fn execute(path: &Path, _key_path: Option<&Path>) -> Result<()> {
        println!("{} Signing rules in {}", style("🔐").bold(), path.display());

        // Generate new keypair
        println!("  Generating Ed25519 keypair...");
        let key_pair = KeyPair::generate()?;
        let signer = Ed25519Signer::with_key_pair(key_pair.clone());

        // Get public key for display
        let public_key = key_pair.public_key();
        println!("  Public key: {:02x?}...", &public_key.0[..8]);

        // Find and sign all .drv files
        let mut signed_count = 0;
        let mut error_count = 0;

        if path.is_file() {
            match Self::sign_file(&signer, path) {
                Ok(()) => signed_count += 1,
                Err(e) => {
                    eprintln!("  {} Failed to sign {}: {}", style("✗").red(), path.display(), e);
                    error_count += 1;
                }
            }
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let file_path = entry.path();
                if file_path.extension().is_some_and(|ext| ext == "drv") {
                    match Self::sign_file(&signer, file_path) {
                        Ok(()) => signed_count += 1,
                        Err(e) => {
                            eprintln!(
                                "  {} Failed to sign {}: {}",
                                style("✗").red(),
                                file_path.display(),
                                e
                            );
                            error_count += 1;
                        }
                    }
                }
            }
        }

        println!();
        if error_count > 0 {
            println!(
                "{} Signed {} files, {} errors",
                style("⚠").yellow().bold(),
                signed_count,
                error_count
            );
        } else if signed_count > 0 {
            println!("{} Successfully signed {} files", style("✓").green().bold(), signed_count);
        } else {
            println!("{} No .drv files found to sign", style("ℹ").blue());
        }

        Ok(())
    }

    /// Sign a single file
    fn sign_file(signer: &Ed25519Signer, path: &Path) -> Result<()> {
        let data = std::fs::read(path)?;
        let signature = signer.sign(&data)?;

        // Write signature file (.drv.sig)
        let sig_path = path.with_extension("drv.sig");
        std::fs::write(&sig_path, signature.0)?;

        println!("  {} {}", style("✓").green(), path.display());
        Ok(())
    }

    /// Verify signed files
    pub fn verify(path: &Path, public_key_path: &Path) -> Result<()> {
        println!("{} Verifying signatures in {}", style("🔍").bold(), path.display());

        let public_key_bytes = std::fs::read(public_key_path)?;
        if public_key_bytes.len() != 32 {
            return Err(DrivenError::Security("Invalid public key size".into()));
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&public_key_bytes);
        let public_key = crate::security::PublicKey(pk_array);

        let signer = Ed25519Signer::new();
        let mut verified = 0;
        let mut failed = 0;

        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let file_path = entry.path();
            if file_path.extension().is_some_and(|ext| ext == "drv") {
                let sig_path = file_path.with_extension("drv.sig");
                if sig_path.exists() {
                    let data = std::fs::read(file_path)?;
                    let sig_bytes = std::fs::read(&sig_path)?;

                    if sig_bytes.len() == 64 {
                        let mut sig_array = [0u8; 64];
                        sig_array.copy_from_slice(&sig_bytes);
                        let signature = crate::security::Signature(sig_array);

                        match signer.verify_with_key(&data, &signature, &public_key) {
                            Ok(true) => {
                                println!("  {} {}", style("✓").green(), file_path.display());
                                verified += 1;
                            }
                            _ => {
                                println!(
                                    "  {} {} (INVALID)",
                                    style("✗").red(),
                                    file_path.display()
                                );
                                failed += 1;
                            }
                        }
                    }
                }
            }
        }

        println!();
        if failed > 0 {
            println!("{} Verified: {}, Failed: {}", style("⚠").yellow().bold(), verified, failed);
            Err(DrivenError::Security(format!("{} signatures failed verification", failed)))
        } else {
            println!("{} All {} signatures verified", style("✓").green().bold(), verified);
            Ok(())
        }
    }
}

/// Generate a new keypair
pub fn generate_keypair(output_dir: &Path) -> Result<()> {
    println!("{} Generating Ed25519 keypair...", style("🔑").bold());

    let key_pair = KeyPair::generate()?;
    let public = key_pair.public_key();

    let public_path = output_dir.join("driven.pub");

    std::fs::create_dir_all(output_dir)?;
    std::fs::write(&public_path, public.0)?;

    println!("  Public key: {}", public_path.display());
    println!();
    println!("{} Keypair generated!", style("✓").green().bold());

    Ok(())
}
