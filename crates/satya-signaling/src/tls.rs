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
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;
use tokio_rustls::TlsAcceptor;
use tower::Service;
use tracing::{debug, warn};

const DEFAULT_MAX_TLS_CONNECTIONS: usize = 1024;
const TLS_HANDSHAKE_TIMEOUT_SECS: u64 = 10;
const HTTP_HEADER_READ_TIMEOUT_SECS: u64 = 15;

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
/// with HTTP/1.1, HTTP/2, and WebSocket upgrade support under bounded concurrency and handshake timeouts.
pub async fn serve_https(
    listener: TcpListener,
    app: Router,
    acceptor: TlsAcceptor,
) -> anyhow::Result<()> {
    let executor = TokioExecutor::new();
    let max_conns = std::env::var("SATYA_MAX_CONNECTIONS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_MAX_TLS_CONNECTIONS);
    let conn_semaphore = Arc::new(Semaphore::new(max_conns));

    loop {
        let permit = match conn_semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => break,
        };
        let (tcp_stream, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(err) => {
                drop(permit);
                warn!("TLS TCP accept error: {err}");
                continue;
            }
        };

        let acceptor = acceptor.clone();
        let app = app.clone();
        let executor = executor.clone();

        tokio::spawn(async move {
            let _permit = permit;
            let tls_stream = match tokio::time::timeout(
                Duration::from_secs(TLS_HANDSHAKE_TIMEOUT_SECS),
                acceptor.accept(tcp_stream),
            )
            .await
            {
                Ok(Ok(stream)) => stream,
                Ok(Err(err)) => {
                    debug!("TLS handshake failed from {peer_addr}: {err}");
                    return;
                }
                Err(_) => {
                    debug!("TLS handshake timed out from {peer_addr}");
                    return;
                }
            };

            let hyper_svc = service_fn(move |mut req: Request<Incoming>| {
                req.extensions_mut()
                    .insert(axum::extract::ConnectInfo(peer_addr));
                let mut svc = app.clone();
                async move { svc.call(req.map(Body::new)).await }
            });

            let mut builder = AutoServerBuilder::new(executor);
            builder
                .http1()
                .header_read_timeout(Duration::from_secs(HTTP_HEADER_READ_TIMEOUT_SECS));

            if let Err(err) = builder
                .serve_connection_with_upgrades(TokioIo::new(tls_stream), hyper_svc)
                .await
            {
                debug!("HTTPS connection error from {peer_addr}: {err}");
            }
        });
    }
    Ok(())
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
