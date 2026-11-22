//! Optional native rustls HTTPS transport for `satya-server` (`SATYA_TLS_CERT` / `SATYA_TLS_KEY`).

use anyhow::{Context, anyhow};
use axum::Router;
use axum::body::Body;
use hyper::Request;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as AutoServerBuilder;
use rustls::ServerConfig;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower::Service;
use tracing::{debug, warn};

/// Filesystem paths to the PEM certificate chain and private key used for native HTTPS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsPaths {
    /// Path to the PEM-encoded leaf certificate (and any intermediate chain).
    pub cert_path: PathBuf,
    /// Path to the PEM-encoded private key matching the leaf certificate.
    pub key_path: PathBuf,
}

/// Resolves TLS certificate and key paths from `SATYA_TLS_CERT` and `SATYA_TLS_KEY`.
///
/// Returns `Ok(None)` when neither variable is set (plain HTTP mode) and errors if
/// only one of the two variables is provided.
pub fn resolve_tls_paths_from_env() -> anyhow::Result<Option<TlsPaths>> {
    let cert = std::env::var("SATYA_TLS_CERT").ok();
    let key = std::env::var("SATYA_TLS_KEY").ok();
    resolve_tls_paths(cert.as_deref(), key.as_deref())
}

fn resolve_tls_paths(cert: Option<&str>, key: Option<&str>) -> anyhow::Result<Option<TlsPaths>> {
    let cert = cert.map(str::trim).filter(|s| !s.is_empty());
    let key = key.map(str::trim).filter(|s| !s.is_empty());

    match (cert, key) {
        (None, None) => Ok(None),
        (Some(cert_str), Some(key_str)) => Ok(Some(TlsPaths {
            cert_path: PathBuf::from(cert_str),
            key_path: PathBuf::from(key_str),
        })),
        _ => Err(anyhow!(
            "Both SATYA_TLS_CERT and SATYA_TLS_KEY must be set to enable HTTPS"
        )),
    }
}

/// Loads the PEM certificate chain and private key and constructs a [`TlsAcceptor`].
pub fn build_tls_acceptor(paths: &TlsPaths) -> anyhow::Result<TlsAcceptor> {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_file_iter(&paths.cert_path)
        .with_context(|| {
            format!(
                "Failed to open TLS certificate file {}",
                paths.cert_path.display()
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| {
            format!(
                "Failed to parse PEM certificates from {}",
                paths.cert_path.display()
            )
        })?;

    if certs.is_empty() {
        return Err(anyhow!(
            "No certificates found in TLS certificate file {}",
            paths.cert_path.display()
        ));
    }

    let key = PrivateKeyDer::from_pem_file(&paths.key_path).with_context(|| {
        format!(
            "Failed to load TLS private key from {}",
            paths.key_path.display()
        )
    })?;

    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Invalid TLS certificate chain or private key")?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// Accepts incoming TCP connections, terminates TLS with `acceptor`, and serves `app`
/// with HTTP/1.1, HTTP/2, and WebSocket upgrade support.
pub async fn serve_https(
    listener: TcpListener,
    app: Router,
    acceptor: TlsAcceptor,
) -> anyhow::Result<()> {
    let executor = TokioExecutor::new();

    loop {
        let (tcp_stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(err) => {
                warn!("TLS TCP accept error: {err}");
                continue;
            }
        };

        let acceptor = acceptor.clone();
        let app = app.clone();
        let executor = executor.clone();

        tokio::spawn(async move {
            let tls_stream = match acceptor.accept(tcp_stream).await {
                Ok(stream) => stream,
                Err(err) => {
                    debug!("TLS handshake failed from {peer_addr}: {err}");
                    return;
                }
            };

            let hyper_svc = service_fn(move |req: Request<Incoming>| {
                let mut svc = app.clone();
                async move { svc.call(req.map(Body::new)).await }
            });

            if let Err(err) = AutoServerBuilder::new(executor)
                .serve_connection_with_upgrades(TokioIo::new(tls_stream), hyper_svc)
                .await
            {
                debug!("HTTPS connection error from {peer_addr}: {err}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_tls_paths_handles_unset_empty_partial_and_both() {
        assert_eq!(resolve_tls_paths(None, None).unwrap(), None);
        assert_eq!(resolve_tls_paths(Some("   "), Some("")).unwrap(), None);

        assert!(resolve_tls_paths(Some("/tmp/cert.pem"), None).is_err());
        assert!(resolve_tls_paths(Some("/tmp/cert.pem"), Some("  ")).is_err());
        assert!(resolve_tls_paths(None, Some("/tmp/key.pem")).is_err());
        assert!(resolve_tls_paths(Some(""), Some("/tmp/key.pem")).is_err());

        let resolved = resolve_tls_paths(Some(" /tmp/cert.pem "), Some(" /tmp/key.pem "))
            .unwrap()
            .expect("expected Some(TlsPaths)");
        assert_eq!(resolved.cert_path, PathBuf::from("/tmp/cert.pem"));
        assert_eq!(resolved.key_path, PathBuf::from("/tmp/key.pem"));
    }

    #[test]
    fn build_tls_acceptor_rejects_missing_or_empty_cert_file() {
        let missing = TlsPaths {
            cert_path: std::env::temp_dir().join("satya_nonexistent_cert_12345.pem"),
            key_path: std::env::temp_dir().join("satya_nonexistent_key_12345.pem"),
        };
        assert!(build_tls_acceptor(&missing).is_err());

        let empty_cert_path = std::env::temp_dir().join(format!(
            "satya_empty_cert_{}_{}.pem",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::write(&empty_cert_path, b"").expect("write empty temp cert");
        let empty = TlsPaths {
            cert_path: empty_cert_path.clone(),
            key_path: missing.key_path,
        };
        let res = build_tls_acceptor(&empty);
        let _ = std::fs::remove_file(&empty_cert_path);
        assert!(res.is_err());
    }
}
