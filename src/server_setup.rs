use std::{net::SocketAddr, path::Path};

use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use tokio::{fs, time::Duration};

use crate::{cert, sgx_attest};

pub async fn setup_production_server(
    app: Router,
    private_key_path: &Path,
    csr_path: &Path,
    quote_path: &Path,
    tee_url: &str,
    certificate_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let private_key = cert::load_or_create_private_key(private_key_path).await;
    let csr = cert::create_and_save_csr(csr_path, &tee_url, &private_key).await;
    sgx_attest::handle_sgx_attestation(quote_path, &private_key, &csr).await;

    log::info!("Waiting for certificate... Use `scripts/fetch-certs.py` to fetch the certificate");
    while !certificate_path.exists() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    log::info!("Certificate found");

    let cert = fs::read(certificate_path).await?;
    let rustls_config =
        RustlsConfig::from_pem(cert, private_key.private_key_to_pem_pkcs8()?).await?;
    let addr = SocketAddr::from(([0, 0, 0, 0], 8001));

    log::info!("Production server starting on https://{}", addr);
    axum_server::bind_rustls(addr, rustls_config).serve(app.into_make_service()).await?;

    Ok(())
}

pub async fn setup_development_server(app: Router) -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    log::info!("Development server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
