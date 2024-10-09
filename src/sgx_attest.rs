use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use openssl::{pkey::PKey, x509::X509Req};
use sha2::Digest;

pub fn sgx_attest(input: Vec<u8>) -> eyre::Result<Vec<u8>> {
    if !Path::new("/dev/attestation/quote").exists() {
        eyre::bail!("sgx quote not found");
    }

    let mut f = match File::create("/dev/attestation/user_report_data") {
        Ok(f) => f,
        Err(error) => {
            panic!("sgx open failed {:?}", error);
        }
    };

    let hash = sha2::Sha256::digest(input).to_vec();

    match f.write_all(&hash) {
        Ok(()) => (),
        Err(error) => {
            eyre::bail!("sgx write failed {:?}", error);
        }
    };

    let quote = match fs::read("/dev/attestation/quote") {
        Ok(quote) => quote,
        Err(error) => {
            eyre::bail!("sgx read failed {:?}", error);
        }
    };

    Ok(quote)
}

pub async fn handle_sgx_attestation(
    quote_path: &Path,
    private_key: &PKey<openssl::pkey::Private>,
    csr: &X509Req,
) {
    let mut pk_bytes = private_key.public_key_to_pem().unwrap();
    let mut csr_pem_bytes = csr.to_pem().unwrap();
    pk_bytes.append(&mut csr_pem_bytes);
    if let Ok(quote) = sgx_attest(pk_bytes) {
        log::info!("Writing quote to file: {}", quote_path.display());
        tokio::fs::write(quote_path, quote).await.expect("Failed to write quote to file");
    }
}
