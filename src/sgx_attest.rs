use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

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
