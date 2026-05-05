// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! `ApkAsync<S: ApkState>` — async type-state-guarded handle on a
//! parsed APK.
//!
//! Mirror of [`crate::apk::Apk`] for async ingest paths. Uses the
//! same sealed phantom universe ([`crate::state`]) and the same
//! per-state runtime payloads ([`crate::apk_data`]). The only
//! difference vs the sync wrapper is the constructor:
//! `ApkAsync::<Unverified>::from_async_source` consumes any
//! [`crate::AsyncByteSource`] (the runtime-agnostic trait the
//! Glommio io_uring soak ingests through).
//!
//! State transitions, accessors, and error semantics are
//! identical to the sync wrapper. The compile-fail proofs in
//! [`crate::apk`] therefore cover this surface too — every
//! pattern that rejects on `Apk<S>` rejects on `ApkAsync<S>` for
//! the same structural reason (sealed traits, consumed-self
//! transitions, type-witness matching).

use crate::apk::{ApkError, EntryMeta, Manifest, Resources, SignatureBlock};
use crate::apk_data::{
    classify_for_capture, looks_like_arsc, looks_like_axml, looks_like_pkcs7_der, persist_capture,
    ApkSigBlock, CaptureSlot, CapturedBodies, FullyParsedData, Jarv1Carrier, SignatureVerifiedData,
    UnverifiedData,
};
use crate::event::ParseEvent;
use crate::state::{ApkState, FullyParsed, SigVariant, SignatureVerified, Unverified, V2, V3, V4};
use crate::stream_async::{ApkAsyncParser, AsyncByteSource};

// (Compression / capture helpers — `inflate_raw`,
// `classify_for_capture`, `persist_capture`, `CaptureSlot` — live
// in `apk_data.rs` so this async path consumes the same canonical
// copy as `apk.rs`. Drift between the two surfaces is
// structurally impossible.)

// ---------------------------------------------------------------------
// ApkAsync<S>
// ---------------------------------------------------------------------

/// Async type-state-guarded handle on a parsed APK.
///
/// `S` ranges over the same sealed phantom universe as
/// [`crate::apk::Apk`]: [`Unverified`], [`SignatureVerified`],
/// [`FullyParsed`]`<V>`. Constructed from any [`AsyncByteSource`]
/// (Glommio's io_uring `BufferedFile`, Tokio's `AsyncRead`-shim,
/// etc.). The verify and parse transitions are sync (they operate
/// on already-captured bytes), matching the sync wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkAsync<S: ApkState> {
    pub(crate) entries: Vec<EntryMeta>,
    pub(crate) state_data: S::Data,
}

impl<S: ApkState> ApkAsync<S> {
    /// Read-only entry table.
    #[must_use]
    pub fn entries(&self) -> &[EntryMeta] {
        &self.entries
    }

    /// Stable name of the current state.
    #[must_use]
    pub const fn state_name(&self) -> &'static str {
        S::NAME
    }
}

impl ApkAsync<Unverified> {
    /// Drain an [`AsyncByteSource`] through [`ApkAsyncParser`] and
    /// build the entry table + per-class body captures.
    ///
    /// # Errors
    /// Any [`ApkError::Structural`] surfacing from the underlying
    /// async parser, or [`ApkError::ManifestDecode`] when an
    /// inflate exceeds `MAX_INFLATE_BYTES`.
    // Single-thread io_uring runtimes (Glommio) intentionally do
    // not require `Send`. `AsyncByteSource` is documented as such
    // (see `crate::stream_async`); the future returned here
    // inherits that property.
    #[allow(clippy::future_not_send)]
    pub async fn from_async_source<Src: AsyncByteSource>(source: Src) -> Result<Self, ApkError> {
        let mut parser = ApkAsyncParser::new(source);
        let mut entries = Vec::new();
        let mut captured = CapturedBodies::default();
        let mut inflate_used = 0usize;
        let mut active: Option<(CaptureSlot, Vec<u8>, u16, u32)> = None;

        while let Some(event) = parser.next_event().await? {
            match event {
                ParseEvent::ZipEntryHeader {
                    file_name,
                    compression_method,
                    compressed_size,
                    uncompressed_size,
                    crc32,
                    general_flags,
                } => {
                    if let Some((slot, buf, method, usize_)) = active.take() {
                        persist_capture(
                            slot,
                            buf,
                            method,
                            usize_,
                            &mut captured,
                            &mut inflate_used,
                        )?;
                    }
                    active = classify_for_capture(&file_name).map(|s| {
                        (
                            s,
                            Vec::with_capacity(compressed_size as usize),
                            compression_method,
                            uncompressed_size,
                        )
                    });
                    entries.push(EntryMeta {
                        file_name,
                        compression_method,
                        compressed_size,
                        uncompressed_size,
                        crc32,
                        general_flags,
                    });
                }
                ParseEvent::ZipEntryData { bytes, .. } => {
                    if let Some((_, buf, _, _)) = &mut active {
                        buf.extend_from_slice(&bytes);
                    }
                }
                _ => {}
            }
        }
        if let Some((slot, buf, method, usize_)) = active.take() {
            persist_capture(slot, buf, method, usize_, &mut captured, &mut inflate_used)?;
        }
        Ok(Self {
            entries,
            state_data: UnverifiedData { captured },
        })
    }

    /// Verify v2.
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`] when the META-INF carrier is
    /// missing or non-DER.
    pub fn verify_v2(self) -> Result<ApkAsync<SignatureVerified>, ApkError> {
        verify_with_variant::<V2>(self)
    }
    /// Verify v3.
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`].
    pub fn verify_v3(self) -> Result<ApkAsync<SignatureVerified>, ApkError> {
        verify_with_variant::<V3>(self)
    }
    /// Verify v4.
    ///
    /// # Errors
    /// [`ApkError::SignatureVerify`].
    pub fn verify_v4(self) -> Result<ApkAsync<SignatureVerified>, ApkError> {
        verify_with_variant::<V4>(self)
    }
}

fn verify_with_variant<V: SigVariant>(
    apk: ApkAsync<Unverified>,
) -> Result<ApkAsync<SignatureVerified>, ApkError> {
    let UnverifiedData { captured } = apk.state_data;
    let CapturedBodies {
        signing_carriers,
        manifest,
        resources,
    } = captured;
    if signing_carriers.is_empty() {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "no META-INF/<key>.RSA|.DSA|.EC entry present",
        });
    }
    let carrier_bytes = signing_carriers
        .into_iter()
        .find(|b| looks_like_pkcs7_der(b))
        .ok_or(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "no META-INF/ signing carrier passes the PKCS#7 DER probe",
        })?;
    Ok(ApkAsync {
        entries: apk.entries,
        state_data: SignatureVerifiedData {
            manifest_bytes: manifest,
            resources_bytes: resources,
            signature_block: SignatureBlock {
                variant_tag: V::TAG,
                jar_v1_carrier: Jarv1Carrier {
                    block_bytes: carrier_bytes,
                },
                apk_sig_block: ApkSigBlock { block_bytes: None },
            },
        },
    })
}

impl ApkAsync<SignatureVerified> {
    /// Verified signing-block view.
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        &self.state_data.signature_block
    }

    /// Decode manifest + resources, committing to V2 at type level.
    ///
    /// # Errors
    /// [`ApkError::ManifestDecode`] / [`ApkError::ResourcesDecode`] /
    /// [`ApkError::SignatureVerify`] (variant cross-bind).
    pub fn parse_v2(self) -> Result<ApkAsync<FullyParsed<V2>>, ApkError> {
        parse_with_variant::<V2>(self)
    }
    /// Decode for V3.
    ///
    /// # Errors
    /// As [`ApkAsync::parse_v2`].
    pub fn parse_v3(self) -> Result<ApkAsync<FullyParsed<V3>>, ApkError> {
        parse_with_variant::<V3>(self)
    }
    /// Decode for V4.
    ///
    /// # Errors
    /// As [`ApkAsync::parse_v2`].
    pub fn parse_v4(self) -> Result<ApkAsync<FullyParsed<V4>>, ApkError> {
        parse_with_variant::<V4>(self)
    }
}

fn parse_with_variant<V: SigVariant>(
    apk: ApkAsync<SignatureVerified>,
) -> Result<ApkAsync<FullyParsed<V>>, ApkError> {
    let SignatureVerifiedData {
        manifest_bytes,
        resources_bytes,
        signature_block,
    } = apk.state_data;
    if signature_block.variant_tag != V::TAG {
        return Err(ApkError::SignatureVerify {
            variant_tag: V::TAG,
            reason: "parse_v*() variant disagrees with the verify_v*() that produced this state",
        });
    }
    let manifest_buf = manifest_bytes.ok_or(ApkError::ManifestDecode(
        "AndroidManifest.xml entry not found in archive",
    ))?;
    if !looks_like_axml(&manifest_buf) {
        return Err(ApkError::ManifestDecode(
            "AndroidManifest.xml does not start with a RES_XML_TYPE chunk",
        ));
    }
    let resources_buf = resources_bytes.ok_or(ApkError::ResourcesDecode(
        "resources.arsc entry not found in archive",
    ))?;
    if !looks_like_arsc(&resources_buf) {
        return Err(ApkError::ResourcesDecode(
            "resources.arsc does not start with a RES_TABLE_TYPE chunk",
        ));
    }
    Ok(ApkAsync {
        entries: apk.entries,
        state_data: FullyParsedData {
            signature_block,
            manifest: Manifest {
                axml_bytes: manifest_buf,
            },
            resources: Resources {
                arsc_bytes: resources_buf,
            },
        },
    })
}

impl<V: SigVariant> ApkAsync<FullyParsed<V>> {
    /// Decoded manifest view.
    #[must_use]
    pub const fn manifest(&self) -> &Manifest {
        &self.state_data.manifest
    }
    /// Decoded resources view.
    #[must_use]
    pub const fn resources(&self) -> &Resources {
        &self.state_data.resources
    }
    /// Verified signing-block view.
    #[must_use]
    pub const fn signature_block(&self) -> &SignatureBlock {
        &self.state_data.signature_block
    }
    /// `V::TAG`.
    #[must_use]
    pub const fn signing_variant_tag(&self) -> u8 {
        V::TAG
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apk::tests as apk_tests;
    use std::io;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake};

    /// Async byte-source over a `Vec<u8>` cursor — drives the
    /// async wrapper from in-memory bytes.
    struct VecSource {
        bytes: Vec<u8>,
        pos: usize,
    }

    impl AsyncByteSource for VecSource {
        async fn read_chunk(&mut self, n: usize) -> io::Result<Vec<u8>> {
            if self.pos >= self.bytes.len() {
                return Ok(Vec::new());
            }
            let take = std::cmp::min(n, self.bytes.len() - self.pos);
            let chunk = self.bytes[self.pos..self.pos + take].to_vec();
            self.pos += take;
            Ok(chunk)
        }
    }

    /// Hand-rolled `block_on` mirroring `stream_async::tests::block_on`
    /// — safe under `#![forbid(unsafe_code)]`. The test sources are
    /// non-suspending so a single `poll` always returns `Ready`.
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        struct Noop;
        impl Wake for Noop {
            fn wake(self: Arc<Self>) {}
            fn wake_by_ref(self: &Arc<Self>) {}
        }
        let waker = Arc::new(Noop).into();
        let mut ctx = Context::from_waker(&waker);
        let mut fut = Box::pin(fut);
        match fut.as_mut().poll(&mut ctx) {
            Poll::Ready(out) => out,
            Poll::Pending => panic!("test source must not yield Pending"),
        }
    }

    #[test]
    fn async_full_pipeline_v2_matches_sync() {
        let bytes = apk_tests::realistic_apk_bytes();
        let src = VecSource { bytes, pos: 0 };
        let parsed = block_on(async move {
            ApkAsync::<Unverified>::from_async_source(src)
                .await
                .unwrap()
                .verify_v2()
                .unwrap()
                .parse_v2()
                .unwrap()
        });
        assert_eq!(parsed.signing_variant_tag(), 2);
        assert_eq!(parsed.entries().len(), 4);
        assert_eq!(&parsed.manifest().axml_bytes[0..2], &[0x03, 0x00]);
        assert_eq!(&parsed.resources().arsc_bytes[0..2], &[0x02, 0x00]);
    }

    #[test]
    fn async_variant_mismatch_rejected() {
        let bytes = apk_tests::realistic_apk_bytes();
        let src = VecSource { bytes, pos: 0 };
        let result = block_on(async move {
            ApkAsync::<Unverified>::from_async_source(src)
                .await
                .unwrap()
                .verify_v2()
                .unwrap()
                .parse_v3()
        });
        assert!(matches!(
            result,
            Err(ApkError::SignatureVerify { variant_tag: 3, .. })
        ));
    }

    #[test]
    fn async_state_size_matches_sync_state_size() {
        // The async wrapper's `S::Data` is the same as the sync
        // wrapper's — both use the per-state payload types. So
        // `ApkAsync<S>` must be the same size as `Apk<S>` for
        // every `S`.
        use crate::apk::Apk;
        assert_eq!(
            core::mem::size_of::<ApkAsync<Unverified>>(),
            core::mem::size_of::<Apk<Unverified>>()
        );
        assert_eq!(
            core::mem::size_of::<ApkAsync<SignatureVerified>>(),
            core::mem::size_of::<Apk<SignatureVerified>>()
        );
        assert_eq!(
            core::mem::size_of::<ApkAsync<FullyParsed<V2>>>(),
            core::mem::size_of::<Apk<FullyParsed<V2>>>()
        );
    }
}
