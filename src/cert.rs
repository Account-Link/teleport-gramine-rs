use std::path::Path;

use acme_lib::create_rsa_key;
use openssl::{
    hash::MessageDigest,
    pkey::{self, PKey},
    stack::Stack,
    x509::{extension::SubjectAlternativeName, X509Req, X509ReqBuilder},
};

pub async fn load_or_create_private_key(private_key_path: &Path) -> PKey<openssl::pkey::Private> {
    if private_key_path.exists() {
        let pk_bytes = tokio::fs::read(private_key_path).await.expect("Failed to read pk file");
        PKey::private_key_from_pem(pk_bytes.as_slice()).unwrap()
    } else {
        let pk = create_rsa_key(2048);
        let pk_bytes = pk.private_key_to_pem_pkcs8().unwrap();
        tokio::fs::write(private_key_path, pk_bytes).await.expect("Failed to write pk to file");
        pk
    }
}

pub async fn create_and_save_csr(
    csr_path: &Path,
    tee_url: &str,
    private_key: &PKey<openssl::pkey::Private>,
) -> X509Req {
    let csr = create_csr(tee_url, private_key).unwrap();
    let csr_pem_bytes = csr.to_pem().unwrap();
    tokio::fs::write(csr_path, csr_pem_bytes).await.expect("Failed to write csr to file");
    csr
}

pub fn create_csr(domain: &str, pkey: &PKey<pkey::Private>) -> eyre::Result<X509Req> {
    //
    // the csr builder
    let mut req_bld = X509ReqBuilder::new().expect("X509ReqBuilder");

    let mut x509_name = openssl::x509::X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "US")?;
    x509_name.append_entry_by_text("ST", "IL")?;
    x509_name.append_entry_by_text("O", "n/a")?;
    x509_name.append_entry_by_text("CN", domain)?;
    let x509_name = x509_name.build();

    req_bld.set_subject_name(&x509_name)?;

    // set private/public key in builder
    req_bld.set_pubkey(pkey).expect("set_pubkey");

    // set all domains as alt names
    let mut stack = Stack::new().expect("Stack::new");
    let ctx = req_bld.x509v3_context(None);
    let mut an = SubjectAlternativeName::new();
    an.dns(domain);

    let ext = an.build(&ctx).expect("SubjectAlternativeName::build");
    stack.push(ext).expect("Stack::push");
    req_bld.add_extensions(&stack).expect("add_extensions");

    // sign it
    req_bld.sign(pkey, MessageDigest::sha256()).expect("csr_sign");

    // the csr
    Ok(req_bld.build())
}
