use axum::Router;
use std::net::SocketAddr;
use tokio::time::Duration;

#[cfg(feature = "production")]
use {
    axum_server::tls_rustls::RustlsConfig,
    openssl::pkey::{PKey, Private},
    std::path::PathBuf,
    tokio::fs,
};

pub async fn setup_server(
    app: Router,
    #[cfg(feature = "production")] private_key: PKey<Private>,
    #[cfg(feature = "production")] certificate_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    
    #[cfg(feature = "production")]
    {
        log::info!("Waiting for certificate...");
        while !certificate_path.exists() {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        log::info!("Certificate found");

        let cert = fs::read(&certificate_path).await?;
        let config = RustlsConfig::from_pem(cert, private_key.private_key_to_pem_pkcs8()?).await?;
        let addr = SocketAddr::from(([0, 0, 0, 0], 8001));

        log::info!("Production server starting on https://{}", addr);
        axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await?;
    }

    #[cfg(not(feature = "production"))]
    {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
        log::info!("Development server running on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
    }

    Ok(())
}