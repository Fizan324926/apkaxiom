// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! P1.14 corpus archive — upload finding inputs to a MinIO /
//! S3-compatible object store via hand-rolled S3 v4 signing.
//!
//! ## Why hand-rolled?
//!
//! AWS SDK v3 + Tokio + hyper would balloon the Reindeer surface
//! by ~30 transitive crates. The S3 v4 signature algorithm is
//! ~80 lines of HMAC-SHA256 over a canonical string (RFC: see
//! AWS Signing v4 docs); for our request shape (single-object
//! PUT/GET against a known endpoint) the spec collapses to:
//!
//! ```text
//!   StringToSign = "AWS4-HMAC-SHA256\n" + ts + "\n" + scope + "\n" + sha256(canonical-request)
//!   SigningKey   = HMAC(HMAC(HMAC(HMAC("AWS4"+SK, date), region), service), "aws4_request")
//!   Signature    = HMAC(SigningKey, StringToSign)
//! ```
//!
//! We curl out the actual HTTP transport — keeps the crate
//! `#![forbid(unsafe_code)]` clean and aligns with the project
//! convention of preferring CLI shell-outs to vendoring large
//! HTTP stacks.
//!
//! ## Bucket layout
//!
//! Objects are stored as `<bucket>/<aa>/<bb>/<sha>.bin` where
//! `<aa><bb>` are the first 4 hex chars of the input's SHA-256
//! (matches the harness's `inputs/` hash-shard). The object key
//! is content-addressed; PUT is idempotent.
//!
//! ## Auth modes
//!
//! - **Auth via env**:
//!     `S3_ENDPOINT=http://127.0.0.1:9000`,
//!     `S3_ACCESS_KEY=admin`,
//!     `S3_SECRET_KEY=<minio root password>`,
//!     `S3_REGION=us-east-1` (MinIO accepts any region),
//!     `S3_BUCKET=corpus`.
//! - **Anonymous**: skip the access keys; for read-only public
//!   buckets only.

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(clippy::doc_markdown, clippy::too_long_first_doc_paragraph)]

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// One object-store endpoint. Construct from env via
/// [`Endpoint::from_env`].
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// `http://127.0.0.1:9000` etc.
    pub url: String,
    /// `admin`.
    pub access_key: String,
    /// MinIO root password.
    pub secret_key: String,
    /// `us-east-1` for MinIO.
    pub region: String,
    /// `corpus`.
    pub bucket: String,
}

impl Endpoint {
    /// Read from `S3_*` environment variables.
    pub fn from_env() -> std::io::Result<Self> {
        fn req(name: &str) -> std::io::Result<String> {
            std::env::var(name).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("env var {name} not set"),
                )
            })
        }
        Ok(Self {
            url: req("S3_ENDPOINT")?,
            access_key: req("S3_ACCESS_KEY")?,
            secret_key: req("S3_SECRET_KEY")?,
            region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
            bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "corpus".into()),
        })
    }

    /// Sharded object key for an input's SHA-256.
    #[must_use]
    pub fn key(&self, sha256_hex: &str) -> String {
        let aa = &sha256_hex[0..2];
        let bb = &sha256_hex[2..4];
        format!("{aa}/{bb}/{sha256_hex}.bin")
    }
}

fn hmac(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}

/// Compute the S3 v4 signature over a canonical request and
/// return the value of the `Authorization` header.
fn sign_v4(
    method: &str,
    host: &str,
    path: &str,
    payload_hash: &str,
    ts: &str,           // YYYYMMDDTHHMMSSZ
    date: &str,         // YYYYMMDD
    ep: &Endpoint,
) -> String {
    let canonical_headers = format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{ts}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = format!(
        "{method}\n{path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let scope = format!("{date}/{}/s3/aws4_request", ep.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{ts}\n{scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );
    let k_date = hmac(format!("AWS4{}", ep.secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, ep.region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let sig = hex(&hmac(&k_signing, string_to_sign.as_bytes()));
    format!(
        "AWS4-HMAC-SHA256 Credential={}/{},SignedHeaders={},Signature={}",
        ep.access_key, scope, signed_headers, sig
    )
}

/// Current UTC timestamp in the AWS-required formats
/// (`YYYYMMDDTHHMMSSZ`, `YYYYMMDD`).
fn now_aws() -> (String, String) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Shell out to `date -u` — keeps the crate dependency-free.
    let ts = std::process::Command::new("date")
        .args(["-u", "+%Y%m%dT%H%M%SZ", &format!("--date=@{now}")])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "19700101T000000Z".into());
    let date = ts.split('T').next().unwrap_or("19700101").to_string();
    (ts, date)
}

/// PUT a single object. Returns the HTTP status as parsed from
/// the curl exit status.
pub fn put_object(ep: &Endpoint, key: &str, body: &[u8]) -> std::io::Result<u16> {
    let payload_hash = sha256_hex(body);
    let (ts, date) = now_aws();
    let url = format!("{}/{}/{}", ep.url.trim_end_matches('/'), ep.bucket, key);
    let host = url
        .split("://")
        .nth(1)
        .and_then(|h| h.split('/').next())
        .unwrap_or("localhost");
    let path = format!("/{}/{}", ep.bucket, key);
    let auth = sign_v4("PUT", host, &path, &payload_hash, &ts, &date, ep);

    // Write the body to a temp file so curl can stream it without
    // depending on stdin chunking.
    let tmp = std::env::temp_dir().join(format!(
        "p114-corpus-put-{}-{}.bin",
        std::process::id(),
        ts
    ));
    std::fs::write(&tmp, body)?;

    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-X",
            "PUT",
            "--data-binary",
            &format!("@{}", tmp.display()),
            "-H",
            &format!("Host: {host}"),
            "-H",
            &format!("Authorization: {auth}"),
            "-H",
            &format!("x-amz-date: {ts}"),
            "-H",
            &format!("x-amz-content-sha256: {payload_hash}"),
            &url,
        ])
        .output()?;
    let _ = std::fs::remove_file(&tmp);
    let code = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u16>()
        .unwrap_or(0);
    Ok(code)
}

/// GET a single object's bytes (anonymous or signed).
pub fn get_object(ep: &Endpoint, key: &str) -> std::io::Result<Vec<u8>> {
    let payload_hash = sha256_hex(&[]);
    let (ts, date) = now_aws();
    let url = format!("{}/{}/{}", ep.url.trim_end_matches('/'), ep.bucket, key);
    let host = url
        .split("://")
        .nth(1)
        .and_then(|h| h.split('/').next())
        .unwrap_or("localhost");
    let path = format!("/{}/{}", ep.bucket, key);
    let auth = sign_v4("GET", host, &path, &payload_hash, &ts, &date, ep);
    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "-X",
            "GET",
            "-H",
            &format!("Host: {host}"),
            "-H",
            &format!("Authorization: {auth}"),
            "-H",
            &format!("x-amz-date: {ts}"),
            "-H",
            &format!("x-amz-content-sha256: {payload_hash}"),
            &url,
        ])
        .output()?;
    Ok(out.stdout)
}

/// Issue an `mc`-equivalent bucket-create call. MinIO accepts a
/// PUT against `/<bucket>/` as bucket-create.
pub fn create_bucket(ep: &Endpoint) -> std::io::Result<u16> {
    // Anonymous bucket-create works on the root credentials.
    let payload_hash = sha256_hex(&[]);
    let (ts, date) = now_aws();
    let url = format!("{}/{}", ep.url.trim_end_matches('/'), ep.bucket);
    let host = url
        .split("://")
        .nth(1)
        .and_then(|h| h.split('/').next())
        .unwrap_or("localhost");
    let path = format!("/{}", ep.bucket);
    let auth = sign_v4("PUT", host, &path, &payload_hash, &ts, &date, ep);
    let out = std::process::Command::new("curl")
        .args([
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "-X",
            "PUT",
            "-H",
            &format!("Host: {host}"),
            "-H",
            &format!("Authorization: {auth}"),
            "-H",
            &format!("x-amz-date: {ts}"),
            "-H",
            &format!("x-amz-content-sha256: {payload_hash}"),
            &url,
        ])
        .output()?;
    let code = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u16>()
        .unwrap_or(0);
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_layout() {
        let ep = Endpoint {
            url: "http://x".into(),
            access_key: "".into(),
            secret_key: "".into(),
            region: "".into(),
            bucket: "corpus".into(),
        };
        let k = ep.key("0011223344556677889900112233445566778899001122334455667788990011");
        assert_eq!(k, "00/11/0011223344556677889900112233445566778899001122334455667788990011.bin");
    }

    #[test]
    fn sign_v4_smoke() {
        let ep = Endpoint {
            url: "http://x".into(),
            access_key: "AK".into(),
            secret_key: "SK".into(),
            region: "us-east-1".into(),
            bucket: "b".into(),
        };
        let s = sign_v4("PUT", "x", "/b/k", &sha256_hex(&[]), "20240101T000000Z", "20240101", &ep);
        assert!(s.starts_with("AWS4-HMAC-SHA256 "));
        assert!(s.contains("Credential=AK/20240101/us-east-1/s3/aws4_request"));
        assert!(s.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"));
    }
}
