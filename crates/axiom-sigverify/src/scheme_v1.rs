// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! Real verifier for APK Signature Scheme v1 (JAR signing).
//!
//! Verifies a JAR-signed APK end-to-end:
//!
//!   1. Walk the central directory; enumerate every entry's
//!      filename + offset + compressed/uncompressed sizes.
//!   2. Inflate each entry body (stored or deflate).
//!   3. Read `META-INF/MANIFEST.MF` text → parse into per-entry
//!      digest declarations.
//!   4. Read `META-INF/<KEY>.SF` → parse manifest-digest claim.
//!   5. Read `META-INF/<KEY>.{RSA,DSA,EC}` as PKCS#7 SignedData;
//!      extract cert + signature; verify under cert's public key
//!      over the verbatim .SF bytes.
//!   6. Recompute SHA over .SF; compare against .SF's manifest-digest.
//!   7. For every non-META-INF regular entry, recompute SHA over
//!      its inflated body; compare against MANIFEST.MF's declaration.
//!
//! The cryptographic primitives reuse the v2/v3 scheme's RSA-PKCS1
//! / RSA-PSS / ECDSA verifiers (algorithm picked from the PKCS#7
//! SignerInfo's `signatureAlgorithm` OID).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::doc_markdown,
    clippy::missing_errors_doc
)]

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::{RejectReason, Verdict};

/// One regular entry in a JAR/APK — name + uncompressed body.
#[derive(Debug, Clone)]
pub struct ApkEntry {
    /// Filename relative to archive root.
    pub name: Vec<u8>,
    /// Uncompressed (inflated) body bytes.
    pub body: Vec<u8>,
}

/// META-INF/MANIFEST.MF declares per-entry SHA digest. The format
/// is line-oriented:
///
/// ```text
/// Manifest-Version: 1.0
///
/// Name: <entry-name>
/// SHA-256-Digest: <base64 digest>
/// ...
/// ```
#[derive(Debug, Clone)]
pub struct ManifestEntry {
    /// Entry name.
    pub name: Vec<u8>,
    /// Per-algorithm declared digest (SHA-256 / SHA-1 base64).
    pub digests: BTreeMap<String, Vec<u8>>,
}

/// Parse MANIFEST.MF text into `(main_attributes, per_entry_attrs)`.
/// `main_attributes` are the lines before the first blank line;
/// `per_entry_attrs` is the list of subsequent sections, each
/// starting with `Name: ...`.
#[must_use]
pub fn parse_manifest(text: &[u8]) -> (BTreeMap<String, String>, Vec<ManifestEntry>) {
    let s = String::from_utf8_lossy(text);
    let mut sections: Vec<Vec<(String, String)>> = vec![Vec::new()];
    let mut current: Vec<(String, String)> = Vec::new();
    // Each "section" is separated by a blank line. Within a section,
    // long lines may be continued with a leading single space.
    let lines: Vec<&str> = s.split('\n').collect();
    let mut last_continued: Option<usize> = None;
    for raw_line in &lines {
        // Strip optional trailing \r (CRLF tolerance).
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            if !current.is_empty() {
                sections.push(current.clone());
                current.clear();
                last_continued = None;
            }
            continue;
        }
        if line.starts_with(' ') {
            // Continuation of the previous attribute's value.
            if let Some(idx) = last_continued {
                current[idx].1.push_str(&line[1..]);
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(": ") {
            current.push((k.to_string(), v.to_string()));
            last_continued = Some(current.len() - 1);
        }
    }
    if !current.is_empty() {
        sections.push(current);
    }
    sections.retain(|s| !s.is_empty());
    if sections.is_empty() {
        return (BTreeMap::new(), Vec::new());
    }
    let main: BTreeMap<String, String> = sections[0].iter().cloned().collect();
    let entries: Vec<ManifestEntry> = sections
        .iter()
        .skip(1)
        .filter_map(|s| {
            let attrs: BTreeMap<String, String> = s.iter().cloned().collect();
            let name = attrs.get("Name")?;
            let mut digests = BTreeMap::new();
            for (k, v) in &attrs {
                if let Some(alg) = k.strip_suffix("-Digest") {
                    if let Some(dec) = base64_decode(v) {
                        digests.insert(alg.to_string(), dec);
                    }
                }
            }
            Some(ManifestEntry {
                name: name.as_bytes().to_vec(),
                digests,
            })
        })
        .collect();
    (main, entries)
}

/// Hand-rolled base64 decoder (RFC 4648, with `+/` alphabet).
/// JAR digests use this canonical form.
#[must_use]
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let trimmed: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = Vec::with_capacity(trimmed.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for c in trimmed.chars() {
        let v: u32 = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            '=' => break,
            _ => return None,
        };
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xff) as u8);
        }
    }
    Some(out)
}

/// Walk an APK's central directory and return every regular
/// entry as `(name, inflated_body)`.
///
/// Supports stored (compression = 0) and deflate (8). Other
/// methods are returned with empty body (acceptable for our
/// purposes: META-INF entries are always stored/deflate).
pub fn walk_entries(apk: &[u8]) -> Result<Vec<ApkEntry>, String> {
    // Locate EOCD.
    let eocd = find_eocd(apk).ok_or("no EOCD")?;
    let cd_offset = u32::from_le_bytes(apk[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let cd_size = u32::from_le_bytes(apk[eocd + 12..eocd + 16].try_into().unwrap()) as usize;
    let total = u16::from_le_bytes(apk[eocd + 10..eocd + 12].try_into().unwrap()) as usize;
    let mut out = Vec::with_capacity(total);
    let mut cur = cd_offset;
    let cd_end = cd_offset + cd_size;
    while cur < cd_end {
        if cur + 46 > apk.len() {
            return Err("CD truncated".into());
        }
        // CDR signature = 0x02014b50
        let sig = u32::from_le_bytes(apk[cur..cur + 4].try_into().unwrap());
        if sig != 0x0201_4b50 {
            return Err("bad CDR signature".into());
        }
        let comp_method = u16::from_le_bytes(apk[cur + 10..cur + 12].try_into().unwrap());
        let comp_size = u32::from_le_bytes(apk[cur + 20..cur + 24].try_into().unwrap()) as usize;
        let uncomp_size = u32::from_le_bytes(apk[cur + 24..cur + 28].try_into().unwrap()) as usize;
        let name_len = u16::from_le_bytes(apk[cur + 28..cur + 30].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(apk[cur + 30..cur + 32].try_into().unwrap()) as usize;
        let comment_len = u16::from_le_bytes(apk[cur + 32..cur + 34].try_into().unwrap()) as usize;
        let lfh_offset = u32::from_le_bytes(apk[cur + 42..cur + 46].try_into().unwrap()) as usize;
        if cur + 46 + name_len > apk.len() {
            return Err("CDR name truncated".into());
        }
        let name = apk[cur + 46..cur + 46 + name_len].to_vec();
        cur += 46 + name_len + extra_len + comment_len;
        // Decode the body via the LFH.
        if lfh_offset + 30 > apk.len() {
            return Err("LFH offset out of bounds".into());
        }
        let lfh_sig = u32::from_le_bytes(apk[lfh_offset..lfh_offset + 4].try_into().unwrap());
        if lfh_sig != 0x0403_4b50 {
            return Err("bad LFH signature".into());
        }
        let lfh_name_len =
            u16::from_le_bytes(apk[lfh_offset + 26..lfh_offset + 28].try_into().unwrap()) as usize;
        let lfh_extra_len =
            u16::from_le_bytes(apk[lfh_offset + 28..lfh_offset + 30].try_into().unwrap()) as usize;
        let body_start = lfh_offset + 30 + lfh_name_len + lfh_extra_len;
        let body_end = body_start + comp_size;
        if body_end > apk.len() {
            return Err("body out of bounds".into());
        }
        let raw_body = &apk[body_start..body_end];
        let body = match comp_method {
            0 => raw_body.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec(raw_body)
                .map_err(|e| format!("inflate {name:?}: {e:?}"))?,
            other => {
                let _ = uncomp_size;
                return Err(format!("unsupported compression method {other}"));
            }
        };
        out.push(ApkEntry { name, body });
    }
    Ok(out)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 22 {
        return None;
    }
    let mut i = bytes.len() - 22;
    loop {
        if u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) == 0x0605_4b50 {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

/// Verify a v1 (JAR) signed APK.
pub fn verify(apk: &[u8]) -> Verdict {
    let entries = match walk_entries(apk) {
        Ok(e) => e,
        Err(e) => return Verdict::Malformed(format!("CD walk: {e}")),
    };
    let manifest_entry = entries.iter().find(|e| e.name == b"META-INF/MANIFEST.MF");
    let manifest = match manifest_entry {
        Some(e) => &e.body,
        None => return Verdict::Reject(RejectReason::NoCertificates),
    };
    // Find first .SF + first signature block.
    let sf_entry = entries
        .iter()
        .find(|e| has_meta_inf_prefix(&e.name) && e.name.ends_with(b".SF"));
    let sig_entry = entries.iter().find(|e| {
        has_meta_inf_prefix(&e.name)
            && (e.name.ends_with(b".RSA") || e.name.ends_with(b".DSA") || e.name.ends_with(b".EC"))
    });
    let sf = match sf_entry {
        Some(e) => &e.body,
        None => return Verdict::Reject(RejectReason::NoSignatures),
    };
    let sig_block = match sig_entry {
        Some(e) => &e.body,
        None => return Verdict::Reject(RejectReason::NoCertificates),
    };
    // Verify PKCS#7 SignedData over .SF bytes.
    let pkcs7_ok = match verify_pkcs7_over(sig_block, sf) {
        Ok(b) => b,
        Err(e) => return Verdict::Reject(RejectReason::AlgorithmError(e)),
    };
    if !pkcs7_ok {
        return Verdict::Reject(RejectReason::SignatureFailed { algorithm_id: 0 });
    }
    // .SF must declare a SHA-256 (or SHA-1) digest of MANIFEST.MF
    // matching the recomputed digest.
    let (sf_main, _) = parse_manifest(sf);
    let manifest_recomputed_sha256 = Sha256::digest(manifest);
    let mut sf_manifest_ok = false;
    for (k, v) in &sf_main {
        if k == "SHA-256-Digest-Manifest" {
            if let Some(declared) = base64_decode(v) {
                if declared == manifest_recomputed_sha256.as_slice() {
                    sf_manifest_ok = true;
                }
            }
        }
    }
    if !sf_manifest_ok {
        // SHA-1 fallback.
        for (k, v) in &sf_main {
            if k == "SHA1-Digest-Manifest" {
                let manifest_sha1 = sha1::Sha1::digest(manifest);
                if let Some(declared) = base64_decode(v) {
                    if declared == manifest_sha1.as_slice() {
                        sf_manifest_ok = true;
                    }
                }
            }
        }
    }
    if !sf_manifest_ok {
        return Verdict::Reject(RejectReason::DigestMismatch { algorithm_id: 0 });
    }
    // Walk every regular APK entry; verify per-entry digest.
    let (_, manifest_entries) = parse_manifest(manifest);
    let manifest_by_name: BTreeMap<&[u8], &ManifestEntry> = manifest_entries
        .iter()
        .map(|m| (m.name.as_slice(), m))
        .collect();
    for entry in &entries {
        if has_meta_inf_prefix(&entry.name) {
            continue;
        }
        // Skip directory entries (filename ending in '/').
        if entry.name.ends_with(b"/") {
            continue;
        }
        let m = match manifest_by_name.get(entry.name.as_slice()) {
            Some(m) => m,
            None => return Verdict::Reject(RejectReason::DigestMismatch { algorithm_id: 0 }),
        };
        // Try SHA-256 first, then SHA-1.
        let mut ok = false;
        if let Some(declared) = m.digests.get("SHA-256") {
            let recomputed = Sha256::digest(&entry.body);
            if declared == recomputed.as_slice() {
                ok = true;
            }
        }
        if !ok {
            if let Some(declared) = m.digests.get("SHA1") {
                let recomputed = sha1::Sha1::digest(&entry.body);
                if declared == recomputed.as_slice() {
                    ok = true;
                }
            }
        }
        if !ok {
            return Verdict::Reject(RejectReason::DigestMismatch { algorithm_id: 0 });
        }
    }
    Verdict::Accept
}

fn has_meta_inf_prefix(name: &[u8]) -> bool {
    name.starts_with(b"META-INF/")
}

/// Verify a PKCS#7 SignedData over `signed_data_bytes`.
/// Returns `true` iff at least one signer's signature verifies
/// under the cert it identifies.
fn verify_pkcs7_over(pkcs7_der: &[u8], signed_data_bytes: &[u8]) -> Result<bool, String> {
    use cms::content_info::ContentInfo;
    use cms::signed_data::SignedData;
    use der::{Decode, Encode};
    let ci = ContentInfo::from_der(pkcs7_der).map_err(|e| format!("PKCS#7 ContentInfo: {e}"))?;
    let sd_value = ci
        .content
        .to_der()
        .map_err(|e| format!("SignedData encode: {e}"))?;
    let sd = SignedData::from_der(&sd_value).map_err(|e| format!("PKCS#7 SignedData: {e}"))?;
    // Collect certificates (DER-encoded).
    let mut certs_der: Vec<Vec<u8>> = Vec::new();
    if let Some(cs) = &sd.certificates {
        for any in cs.0.iter() {
            // any is CertificateChoices — we only handle Certificate (the first variant).
            let cert_der = any.to_der().map_err(|e| format!("cert encode: {e}"))?;
            // Strip the outer ContextSpecific wrapper if any. The
            // re-encoded bytes from CertificateChoices are already
            // the Certificate DER for the standard variant.
            certs_der.push(cert_der);
        }
    }
    // For each SignerInfo, find its cert and verify.
    for si in sd.signer_infos.0.iter() {
        let sig_alg_oid = &si.signature_algorithm.oid;
        let digest_alg_oid = &si.digest_alg.oid;
        let signature = si.signature.as_bytes();
        // Determine what bytes the signature is over:
        //   - If `signed_attrs` is present: signature is over the
        //     DER-encoded SignedAttributes (which has an implicit
        //     tag [0]; the verifier strips that and uses SET DER).
        //     `signed_attrs` MUST include a `messageDigest`
        //     attribute equal to the hash of the encapContentInfo.
        //   - If `signed_attrs` is absent: signature is directly
        //     over the encapContentInfo.eContent.
        let to_verify: Vec<u8> = if let Some(sa) = &si.signed_attrs {
            // Verify messageDigest matches signed_data hash first.
            let md_oid_str = "1.2.840.113549.1.9.4";
            let mut md_attr_bytes: Option<Vec<u8>> = None;
            for attr in sa.iter() {
                if attr.oid.to_string() == md_oid_str {
                    if let Some(v) = attr.values.get(0) {
                        md_attr_bytes = Some(v.value().to_vec());
                    }
                }
            }
            let md_decl = match md_attr_bytes {
                Some(b) => b,
                None => return Ok(false),
            };
            // Hash the .SF bytes; compare.
            let recomputed = match sig_alg_oid.to_string().as_str() {
                "1.2.840.113549.1.1.5" => sha1::Sha1::digest(signed_data_bytes).to_vec(),
                _ => Sha256::digest(signed_data_bytes).to_vec(),
            };
            if md_decl != recomputed {
                return Ok(false);
            }
            // Re-encode signed_attrs as SET DER (RFC 5652 §5.4:
            // the implicit [0] tag is rewritten to SET = 0x31 for
            // the digest computation).
            let mut encoded = sa
                .to_der()
                .map_err(|e| format!("signed_attrs encode: {e}"))?;
            if !encoded.is_empty() {
                encoded[0] = 0x31; // SET tag
            }
            encoded
        } else {
            signed_data_bytes.to_vec()
        };
        let mut anyone_verified = false;
        for cert_der in &certs_der {
            let cert_der = strip_certificate_choices_wrapper(cert_der).unwrap_or(cert_der.clone());
            let spki_der = match crate::scheme_v2::extract_spki_der(&cert_der) {
                Some(s) => s,
                None => continue,
            };
            let res = verify_pkcs7_sig_dispatched(
                sig_alg_oid,
                digest_alg_oid,
                &spki_der,
                &to_verify,
                signature,
            );
            if matches!(res, Ok(true)) {
                anyone_verified = true;
                break;
            }
        }
        if anyone_verified {
            return Ok(true);
        }
    }
    Ok(false)
}

fn strip_certificate_choices_wrapper(der: &[u8]) -> Option<Vec<u8>> {
    // CertificateChoices DER starts with the chosen variant tag.
    // For Certificate (the most common), the inner DER is itself a
    // SEQUENCE (tag 0x30). If the outer byte is 0x30, the wrapper
    // is already absent.
    if der.first() == Some(&0x30) {
        return Some(der.to_vec());
    }
    None
}

fn verify_pkcs7_sig_dispatched(
    sig_alg_oid: &const_oid::ObjectIdentifier,
    digest_alg_oid: &const_oid::ObjectIdentifier,
    spki_der: &[u8],
    signed_data: &[u8],
    signature: &[u8],
) -> Result<bool, String> {
    use axiom_sigblock::scheme::SignatureAlgorithmId;
    // Common PKCS#7 algorithm OIDs:
    //   1.2.840.113549.1.1.1   rsaEncryption (generic; hash from digest_alg)
    //   1.2.840.113549.1.1.5   sha1WithRSAEncryption
    //   1.2.840.113549.1.1.11  sha256WithRSAEncryption
    //   1.2.840.113549.1.1.12  sha384WithRSAEncryption
    //   1.2.840.113549.1.1.13  sha512WithRSAEncryption
    //   1.2.840.10045.4.3.2    ecdsa-with-SHA256
    //   1.2.840.10045.4.3.4    ecdsa-with-SHA512
    // Hash OIDs (when sig_alg is generic rsaEncryption):
    //   1.3.14.3.2.26          sha1
    //   2.16.840.1.101.3.4.2.1 sha256
    //   2.16.840.1.101.3.4.2.3 sha512
    let sig = sig_alg_oid.to_string();
    let dig = digest_alg_oid.to_string();
    match sig.as_str() {
        "1.2.840.113549.1.1.11" => crate::scheme_v2::verify_signature(
            SignatureAlgorithmId::RsaPkcs1Sha256,
            spki_der,
            signed_data,
            signature,
        ),
        "1.2.840.113549.1.1.5" => verify_rsa_sha1(spki_der, signed_data, signature),
        "1.2.840.10045.4.3.2" => crate::scheme_v2::verify_signature(
            SignatureAlgorithmId::EcdsaSha256,
            spki_der,
            signed_data,
            signature,
        ),
        // Generic rsaEncryption — dispatch on digest_alg OID.
        "1.2.840.113549.1.1.1" => match dig.as_str() {
            "1.3.14.3.2.26" => verify_rsa_sha1(spki_der, signed_data, signature),
            "2.16.840.1.101.3.4.2.1" => crate::scheme_v2::verify_signature(
                SignatureAlgorithmId::RsaPkcs1Sha256,
                spki_der,
                signed_data,
                signature,
            ),
            other => Err(format!(
                "unsupported PKCS#7 digest OID {other} (with rsaEncryption)"
            )),
        },
        other => Err(format!("unsupported PKCS#7 signature OID {other}")),
    }
}

fn verify_rsa_sha1(spki_der: &[u8], signed_data: &[u8], signature: &[u8]) -> Result<bool, String> {
    use rsa::pkcs1v15::{Signature, VerifyingKey};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::signature::Verifier;
    let pk = rsa::RsaPublicKey::from_public_key_der(spki_der)
        .map_err(|e| format!("RSA-SHA1 public key: {e}"))?;
    let vk: VerifyingKey<sha1::Sha1> = VerifyingKey::new(pk);
    let sig = Signature::try_from(signature).map_err(|e| format!("RSA-SHA1 sig: {e}"))?;
    Ok(vk.verify(signed_data, &sig).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn read_fixture(rel: &str) -> Vec<u8> {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(rel);
        std::fs::read(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
    }

    #[test]
    fn v1_verify_real_fdroid_apk_accepts() {
        let apk = read_fixture("crates/axiom-l1-rs/tests/fixtures/wifiautoff.apk");
        let v = verify(&apk);
        assert!(matches!(v, Verdict::Accept), "v1 wifiautoff: {v:?}");
    }

    #[test]
    fn v1_verify_real_fdroid_apk_2_accepts() {
        let apk = read_fixture("crates/axiom-l1-rs/tests/fixtures/clipboard.apk");
        let v = verify(&apk);
        assert!(matches!(v, Verdict::Accept), "v1 clipboard: {v:?}");
    }

    #[test]
    fn v1_verify_resigned_apk_accepts() {
        let apk = read_fixture("corpus/signing/v1-only/wifiautoff-v1.apk");
        let v = verify(&apk);
        assert!(matches!(v, Verdict::Accept), "v1 resigned: {v:?}");
    }

    #[test]
    fn parse_manifest_extracts_entry_digest() {
        let text = b"Manifest-Version: 1.0\r\nCreated-By: test\r\n\r\nName: classes.dex\r\nSHA-256-Digest: dGVzdA==\r\n";
        let (main, entries) = parse_manifest(text);
        assert_eq!(
            main.get("Manifest-Version").map(String::as_str),
            Some("1.0")
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, b"classes.dex");
        assert_eq!(
            entries[0].digests.get("SHA-256").map(Vec::as_slice),
            Some(b"test".as_slice())
        );
    }

    #[test]
    fn base64_decode_round_trip_basic() {
        assert_eq!(base64_decode("dGVzdA==").as_deref(), Some(b"test".as_ref()));
        assert_eq!(
            base64_decode("aGVsbG8gd29ybGQ=").as_deref(),
            Some(b"hello world".as_ref())
        );
    }
}
