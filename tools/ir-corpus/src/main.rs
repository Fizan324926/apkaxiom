// Copyright (c) APKAXIOM Authors. Apache-2.0 OR MIT.

//! AXIOM-IR v0.1 corpus generator.
//!
//! Produces the deterministic test corpus that exercises every variant
//! of every dialect and emits drift-stable JSON summaries to
//! `docs/phase-1/P1.4/ir-data/`. The corpus is byte-deterministic — two
//! runs on any host produce bit-identical output.
//!
//! Outputs:
//!   * `ir-data/identity.json`       — schema version, producer tag, counts
//!   * `ir-data/manifest-corpus.json`— summary of 100 manifests (hashes only)
//!   * `ir-data/resource-corpus.json`— summary of 50 resource tables
//!   * `ir-data/lowering-corpus.json`— summary of 30 lowering pairs
//!   * `ir-data/schema-hash.txt`     — SHA-256 of the canonical-bytes
//!                                     concatenation across the entire corpus
//!   * `ir-data/summary.json`        — aggregated counts + corpus root hash
//!   * `ir-data/type-table.json`     — flat enumeration of every IR type
//!                                     and attribute variant + canonical tag
//!   * `corpus/manifest/<n>.json`    — per-sample inspection JSON (manifest)
//!   * `corpus/resource/<n>.json`    — per-sample inspection JSON (resource)
//!   * `corpus/lowering/<n>.json`    — per-sample inspection JSON (lowering)
//!
//! Hand-rolled JSON emission, no `serde_json`. Deterministic byte order.
//!
//! `usage: ir-corpus <ir-data-dir> <corpus-dir>`

#![forbid(unsafe_code)]
#![warn(missing_docs, unreachable_pub)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::too_many_lines,
    clippy::module_name_repetitions
)]

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::ExitCode,
};

use axiom_ir::{
    canonical, hash, json as ir_json, lowering,
    manifest::{
        launcher_activity, Component, ComponentKind, DataAuthority, DataFilter, IntentFilter,
        ManifestModule, Permission, ProtectionLevel,
    },
    resource::{
        Configuration, ResourceEntry, ResourceId, ResourceRef, ResourceTable, ResourceType,
        ResourceValue, StringPool,
    },
    Attribute, Tribool, Type, PRODUCER_TAG, SCHEMA_VERSION,
};

const MANIFEST_COUNT: usize = 100;
const RESOURCE_COUNT: usize = 50;
const LOWERING_COUNT: usize = 30;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let [_, ir, corpus] = args.as_slice() else {
        eprintln!("usage: ir-corpus <ir-data-dir> <corpus-dir>");
        return ExitCode::from(2);
    };
    let ir_data_dir = PathBuf::from(ir);
    let corpus_dir = PathBuf::from(corpus);

    if let Err(e) = run(&ir_data_dir, &corpus_dir) {
        eprintln!("ir-corpus: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn run(ir_data: &Path, corpus: &Path) -> std::io::Result<()> {
    fs::create_dir_all(ir_data.join("."))?;
    fs::create_dir_all(corpus.join("manifest"))?;
    fs::create_dir_all(corpus.join("resource"))?;
    fs::create_dir_all(corpus.join("lowering"))?;

    // Identity. Independent of any sample so it is the first artefact written.
    write_atomic(&ir_data.join("identity.json"), &identity_json())?;

    // Manifests.
    let manifests = build_manifests();
    assert_eq!(manifests.len(), MANIFEST_COUNT);
    let manifest_summary = corpus_summary("manifest", &manifests, |i, m| {
        let bytes = canonical::encode_manifest(m);
        let json = ir_json::encode_manifest(m);
        let path = corpus.join(format!("manifest/{i:03}.json"));
        write_atomic(&path, &json).map(|()| bytes)
    })?;
    write_atomic(
        &ir_data.join("manifest-corpus.json"),
        &manifest_summary.json,
    )?;

    // Resource tables.
    let resources = build_resources();
    assert_eq!(resources.len(), RESOURCE_COUNT);
    let resource_summary = corpus_summary("resource", &resources, |i, r| {
        let bytes = canonical::encode_resource(r);
        let json = ir_json::encode_resource(r);
        let path = corpus.join(format!("resource/{i:03}.json"));
        write_atomic(&path, &json).map(|()| bytes)
    })?;
    write_atomic(
        &ir_data.join("resource-corpus.json"),
        &resource_summary.json,
    )?;

    // Lowering pairs.
    let pairs = build_lowering_pairs();
    assert_eq!(pairs.len(), LOWERING_COUNT);
    let mut lowering_bytes_concat = Vec::with_capacity(8 * 1024);
    let mut lowering_buf = String::with_capacity(2 * 1024);
    lowering_buf.push('{');
    push_kv_str(&mut lowering_buf, "schema", "apkaxiom.p14-ir/v1");
    lowering_buf.push(',');
    push_kv_str(&mut lowering_buf, "kind", "lowering");
    lowering_buf.push(',');
    push_kv_u32(&mut lowering_buf, "count", LOWERING_COUNT as u32);
    lowering_buf.push(',');
    lowering_buf.push_str("\"items\":[");
    for (i, (manifest, resources)) in pairs.iter().enumerate() {
        let res = lowering::resolve(manifest, resources);
        let resolved_bytes = canonical::encode_manifest(&res.manifest);
        let pre_bytes = canonical::encode_manifest(manifest);
        let resources_bytes = canonical::encode_resource(resources);
        lowering_bytes_concat.extend_from_slice(&pre_bytes);
        lowering_bytes_concat.extend_from_slice(&resources_bytes);
        lowering_bytes_concat.extend_from_slice(&resolved_bytes);

        let inspection = lowering_inspection_json(i, manifest, resources, &res);
        let path = corpus.join(format!("lowering/{i:03}.json"));
        write_atomic(&path, &inspection)?;

        if i > 0 {
            lowering_buf.push(',');
        }
        lowering_buf.push('{');
        push_kv_u32(&mut lowering_buf, "index", i as u32);
        lowering_buf.push(',');
        push_kv_str(
            &mut lowering_buf,
            "manifest_pre_hash",
            &hash::hex(&hash::sha256(&pre_bytes)),
        );
        lowering_buf.push(',');
        push_kv_str(
            &mut lowering_buf,
            "resource_hash",
            &hash::hex(&hash::sha256(&resources_bytes)),
        );
        lowering_buf.push(',');
        push_kv_str(
            &mut lowering_buf,
            "manifest_post_hash",
            &hash::hex(&hash::sha256(&resolved_bytes)),
        );
        lowering_buf.push(',');
        push_kv_u32(
            &mut lowering_buf,
            "diagnostic_count",
            res.diagnostics.len() as u32,
        );
        lowering_buf.push('}');
    }
    lowering_buf.push(']');
    lowering_buf.push(',');
    push_kv_str(
        &mut lowering_buf,
        "concat_hash",
        &hash::hex(&hash::sha256(&lowering_bytes_concat)),
    );
    lowering_buf.push('}');
    lowering_buf.push('\n');
    write_atomic(&ir_data.join("lowering-corpus.json"), &lowering_buf)?;

    // Type / attribute table.
    write_atomic(&ir_data.join("type-table.json"), &type_table_json())?;

    // JSON Schema for the stable JSON output produced by
    // `ir_json::encode_manifest` / `encode_resource`. Hand-rolled
    // (Draft 2020-12), drift-stable, deterministic.
    write_atomic(&ir_data.join("axiom-ir.schema.json"), &json_schema())?;

    // Schema-freeze hash. Concat order: manifests then resources then
    // lowering. Equal to running `sha256sum` over the same concat externally.
    let mut full_concat =
        Vec::with_capacity(manifest_summary.bytes_len + resource_summary.bytes_len);
    full_concat.extend_from_slice(&manifest_summary.bytes_concat);
    full_concat.extend_from_slice(&resource_summary.bytes_concat);
    full_concat.extend_from_slice(&lowering_bytes_concat);
    let schema_hash = hash::hex(&hash::sha256(&full_concat));
    write_atomic(
        &ir_data.join("schema-hash.txt"),
        &format!("{schema_hash}\n"),
    )?;

    // Aggregated summary.
    let summary = build_summary(
        &manifest_summary,
        &resource_summary,
        LOWERING_COUNT,
        &schema_hash,
        &hash::hex(&hash::sha256(&lowering_bytes_concat)),
    );
    write_atomic(&ir_data.join("summary.json"), &summary)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

fn identity_json() -> String {
    let mut s = String::with_capacity(256);
    s.push('{');
    push_kv_str(&mut s, "schema", "apkaxiom.p14-ir/v1");
    s.push(',');
    push_kv_str(&mut s, "axiom_ir_version", SCHEMA_VERSION);
    s.push(',');
    push_kv_str(&mut s, "producer", PRODUCER_TAG);
    s.push(',');
    push_kv_u32(&mut s, "manifest_count", MANIFEST_COUNT as u32);
    s.push(',');
    push_kv_u32(&mut s, "resource_count", RESOURCE_COUNT as u32);
    s.push(',');
    push_kv_u32(&mut s, "lowering_count", LOWERING_COUNT as u32);
    s.push('}');
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// Manifest corpus — 100 hand-built modules
// ---------------------------------------------------------------------------

fn build_manifests() -> Vec<ManifestModule> {
    let mut out = Vec::with_capacity(MANIFEST_COUNT);

    // 0..10: minimal manifests at each major SDK boundary L (21) → V (35).
    for (i, sdk) in [21u8, 23, 26, 28, 29, 30, 31, 33, 34, 35]
        .iter()
        .enumerate()
    {
        out.push(
            ManifestModule::new(format!("com.apkaxiom.minimal.sdk{sdk}"))
                .with_target_sdk(*sdk)
                .with_min_sdk(21)
                .with_application_label(format!("Minimal{i}")),
        );
    }

    // 10..20: launcher activities.
    for i in 0..10 {
        out.push(
            ManifestModule::new(format!("com.apkaxiom.launcher{i}"))
                .with_target_sdk(34)
                .with_min_sdk(24)
                .with_application_label("@string/app_name")
                .with_component(launcher_activity(format!(".Activity{i}"))),
        );
    }

    // 20..30: services with permission gating.
    for i in 0..10 {
        let svc = Component {
            kind: ComponentKind::Service,
            name: format!(".Service{i}"),
            exported: if i % 2 == 0 {
                Tribool::True
            } else {
                Tribool::False
            },
            enabled: Tribool::Default,
            permission: Some(format!("com.apkaxiom.permission.SVC{i}")),
            intent_filters: Vec::new(),
            authorities: Vec::new(),
        };
        out.push(
            ManifestModule::new(format!("com.apkaxiom.service{i}"))
                .with_target_sdk(33)
                .with_min_sdk(21)
                .with_component(svc),
        );
    }

    // 30..40: receivers with intent filters.
    for i in 0..10 {
        let filter = IntentFilter::new()
            .with_action(format!("com.apkaxiom.action.EVENT_{i}"))
            .with_category("android.intent.category.DEFAULT");
        let recv = Component {
            kind: ComponentKind::Receiver,
            name: format!(".Receiver{i}"),
            exported: Tribool::Default,
            enabled: Tribool::True,
            permission: None,
            intent_filters: vec![filter],
            authorities: Vec::new(),
        };
        out.push(
            ManifestModule::new(format!("com.apkaxiom.receiver{i}"))
                .with_target_sdk(31)
                .with_min_sdk(21)
                .with_component(recv),
        );
    }

    // 40..50: providers (always not-exported by default — covers the
    // ComponentKind::Provider tribool-resolution edge case).
    for i in 0..10 {
        let provider = Component {
            kind: ComponentKind::Provider,
            name: format!(".Provider{i}"),
            exported: Tribool::Default,
            enabled: Tribool::Default,
            permission: Some("android.permission.READ_EXTERNAL_STORAGE".into()),
            intent_filters: Vec::new(),
            authorities: vec![DataAuthority {
                host: format!("com.apkaxiom.provider{i}"),
                port: None,
            }],
        };
        out.push(
            ManifestModule::new(format!("com.apkaxiom.provider{i}"))
                .with_target_sdk(34)
                .with_min_sdk(21)
                .with_component(provider),
        );
    }

    // 50..60: deep-link activities.
    for i in 0..10 {
        let scheme = if i % 2 == 0 { "https" } else { "http" };
        let f = IntentFilter::new()
            .with_action("android.intent.action.VIEW")
            .with_category("android.intent.category.DEFAULT")
            .with_category("android.intent.category.BROWSABLE")
            .with_data(
                DataFilter::new()
                    .with_scheme(scheme)
                    .with_host(format!("link{i}.apkaxiom.com")),
            );
        let act = Component {
            kind: ComponentKind::Activity,
            name: format!(".Deep{i}"),
            exported: Tribool::True,
            enabled: Tribool::Default,
            permission: None,
            intent_filters: vec![f],
            authorities: Vec::new(),
        };
        out.push(
            ManifestModule::new(format!("com.apkaxiom.deep{i}"))
                .with_target_sdk(33)
                .with_min_sdk(24)
                .with_component(act),
        );
    }

    // 60..70: declared permissions of every protection level (cycles).
    for (i, level) in [
        ProtectionLevel::Normal,
        ProtectionLevel::Dangerous,
        ProtectionLevel::Signature,
        ProtectionLevel::SignatureOrSystem,
        ProtectionLevel::Internal,
        ProtectionLevel::Normal,
        ProtectionLevel::Dangerous,
        ProtectionLevel::Signature,
        ProtectionLevel::SignatureOrSystem,
        ProtectionLevel::Internal,
    ]
    .iter()
    .enumerate()
    {
        out.push(
            ManifestModule::new(format!("com.apkaxiom.perm{i}"))
                .with_target_sdk(34)
                .with_min_sdk(21)
                .with_permission(Permission {
                    name: format!("com.apkaxiom.permission.PERM{i}"),
                    protection: *level,
                    group: if i % 3 == 0 {
                        Some("group.apkaxiom".into())
                    } else {
                        None
                    },
                }),
        );
    }

    // 70..80: uses-permissions stress.
    for i in 0..10 {
        let mut m = ManifestModule::new(format!("com.apkaxiom.uses{i}"))
            .with_target_sdk(34)
            .with_min_sdk(21);
        for j in 0..=i {
            m = m.with_uses_permission(format!("com.apkaxiom.permission.USES_{i}_{j}"));
        }
        out.push(m);
    }

    // 80..90: empty intent filters (legitimate Android shape — caller may
    // not specify any actions/categories on a non-default-exported
    // component for fingerprinting purposes).
    for i in 0..10 {
        let act = Component {
            kind: ComponentKind::Activity,
            name: format!(".EmptyFilter{i}"),
            exported: Tribool::True,
            enabled: Tribool::Default,
            permission: None,
            intent_filters: vec![IntentFilter::default()],
            authorities: Vec::new(),
        };
        out.push(
            ManifestModule::new(format!("com.apkaxiom.empty{i}"))
                .with_target_sdk(34)
                .with_min_sdk(21)
                .with_component(act),
        );
    }

    // 90..100: kitchen sink — every kind, multiple components, mixed
    // permissions, uses-permissions, deep-links.
    for i in 0..10 {
        let mut m = ManifestModule::new(format!("com.apkaxiom.kitchen{i}"))
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_application_label("@string/app_name")
            .with_uses_permission("android.permission.INTERNET")
            .with_uses_permission("android.permission.CAMERA")
            .with_permission(Permission {
                name: format!("com.apkaxiom.permission.KITCHEN{i}"),
                protection: ProtectionLevel::Signature,
                group: None,
            });

        m = m
            .with_component(launcher_activity(".Main"))
            .with_component(Component {
                kind: ComponentKind::Service,
                name: ".Sync".into(),
                exported: Tribool::False,
                enabled: Tribool::True,
                permission: None,
                intent_filters: Vec::new(),
                authorities: Vec::new(),
            })
            .with_component(Component {
                kind: ComponentKind::Receiver,
                name: ".Boot".into(),
                exported: Tribool::True,
                enabled: Tribool::True,
                permission: Some("android.permission.RECEIVE_BOOT_COMPLETED".into()),
                intent_filters: vec![
                    IntentFilter::new().with_action("android.intent.action.BOOT_COMPLETED")
                ],
                authorities: Vec::new(),
            })
            .with_component(Component {
                kind: ComponentKind::Provider,
                name: format!(".FilesProvider{i}"),
                exported: Tribool::Default,
                enabled: Tribool::Default,
                permission: None,
                intent_filters: Vec::new(),
                authorities: vec![DataAuthority {
                    host: format!("com.apkaxiom.kitchen{i}.files"),
                    port: None,
                }],
            });
        out.push(m);
    }

    out
}

// ---------------------------------------------------------------------------
// Resource corpus — 50 hand-built tables
// ---------------------------------------------------------------------------

fn build_resources() -> Vec<ResourceTable> {
    let mut out = Vec::with_capacity(RESOURCE_COUNT);

    // 0..10: simple string-only tables.
    for i in 0..10 {
        let mut pool = StringPool::new();
        let _ = pool.intern(format!("App {i}"));
        let _ = pool.intern("Settings");
        let _ = pool.intern("About");
        out.push(ResourceTable {
            package: format!("com.apkaxiom.r{i}"),
            string_pool: pool,
            configurations: vec![Configuration::default_for_sdk(21)],
            entries: vec![
                ResourceEntry {
                    ref_: ResourceRef {
                        r#type: ResourceType::String,
                        id: ResourceId(0x7f00_0001),
                        name: "app_name".into(),
                    },
                    value: ResourceValue::String(format!("App {i}")),
                },
                ResourceEntry {
                    ref_: ResourceRef {
                        r#type: ResourceType::String,
                        id: ResourceId(0x7f00_0002),
                        name: "settings".into(),
                    },
                    value: ResourceValue::String("Settings".into()),
                },
            ],
        });
    }

    // 10..20: every value-kind once.
    for i in 0i32..10 {
        let entries = vec![
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "label".into(),
                },
                value: ResourceValue::String(format!("Label {i}")),
            },
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::Integer,
                    id: ResourceId(0x7f00_0002),
                    name: "max_count".into(),
                },
                value: ResourceValue::Int(i * 100),
            },
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::Bool,
                    id: ResourceId(0x7f00_0003),
                    name: "is_premium".into(),
                },
                value: ResourceValue::Bool(i % 2 == 0),
            },
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::Color,
                    id: ResourceId(0x7f00_0004),
                    name: "primary".into(),
                },
                value: ResourceValue::Int(0x0633_0000 ^ i),
            },
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::Drawable,
                    id: ResourceId(0x7f00_0005),
                    name: "icon".into(),
                },
                value: ResourceValue::Ref(ResourceRef {
                    r#type: ResourceType::Drawable,
                    id: ResourceId(0x7f00_0006),
                    name: "ic_launcher".into(),
                }),
            },
        ];
        out.push(ResourceTable {
            package: format!("com.apkaxiom.kinds{i}"),
            string_pool: StringPool::default(),
            configurations: vec![Configuration::default_for_sdk(24)],
            entries,
        });
    }

    // 20..30: density-config matrix.
    let dpi_buckets = [120u32, 160, 240, 320, 480, 640];
    for i in 0..10 {
        let mut configs = Vec::new();
        for (j, dpi) in dpi_buckets.iter().enumerate() {
            configs.push(
                Configuration::default_for_sdk(21)
                    .with_qualifier(format!("dpi-{dpi}-c{j}"))
                    .with_density(*dpi),
            );
        }
        out.push(ResourceTable {
            package: format!("com.apkaxiom.dpi{i}"),
            string_pool: StringPool::default(),
            configurations: configs,
            entries: vec![ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::Drawable,
                    id: ResourceId(0x7f00_0010),
                    name: "logo".into(),
                },
                value: ResourceValue::String(format!("res/drawable/logo{i}.png")),
            }],
        });
    }

    // 30..40: locale-config matrix.
    let locales = [
        "en-US", "en-GB", "fr-FR", "de-DE", "ja-JP", "ko-KR", "zh-CN", "es-ES", "ar-SA", "hi-IN",
    ];
    for i in 0..10 {
        let mut configs = Vec::new();
        for l in &locales {
            configs.push(
                Configuration::default_for_sdk(24)
                    .with_qualifier(format!("locale-{l}"))
                    .with_locale(*l),
            );
        }
        out.push(ResourceTable {
            package: format!("com.apkaxiom.locales{i}"),
            string_pool: StringPool::default(),
            configurations: configs,
            entries: vec![ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "app_name".into(),
                },
                value: ResourceValue::String(format!("App-i18n-{i}")),
            }],
        });
    }

    // 40..50: chained references — entry whose value is a Ref to another
    // entry. Verifies canonical bytes correctly nest the recursive
    // ResourceRef shape.
    for i in 0..10 {
        let entries = vec![
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "leaf".into(),
                },
                value: ResourceValue::String(format!("leaf-{i}")),
            },
            ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0002),
                    name: "alias".into(),
                },
                value: ResourceValue::Ref(ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "leaf".into(),
                }),
            },
        ];
        out.push(ResourceTable {
            package: format!("com.apkaxiom.chain{i}"),
            string_pool: StringPool::default(),
            configurations: vec![Configuration::default_for_sdk(21)],
            entries,
        });
    }

    out
}

// ---------------------------------------------------------------------------
// Lowering pairs — 30 (manifest, resources, expected diagnostics)
// ---------------------------------------------------------------------------

fn build_lowering_pairs() -> Vec<(ManifestModule, ResourceTable)> {
    let mut out = Vec::with_capacity(LOWERING_COUNT);

    // 0..10: clean substitution — manifest references a known string in
    // the resource table.
    for i in 0..10 {
        let app_name = format!("Resolved App {i}");
        let resources =
            ResourceTable::new(format!("com.apkaxiom.lower{i}")).with_entry(ResourceEntry {
                ref_: ResourceRef {
                    r#type: ResourceType::String,
                    id: ResourceId(0x7f00_0001),
                    name: "app_name".into(),
                },
                value: ResourceValue::String(app_name.clone()),
            });
        let manifest = ManifestModule::new(format!("com.apkaxiom.lower{i}"))
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_application_label("@string/app_name")
            .with_component(launcher_activity(".Main"));
        out.push((manifest, resources));
    }

    // 10..20: missing reference — emits a warning diagnostic.
    for i in 0..10 {
        let resources = ResourceTable::new(format!("com.apkaxiom.miss{i}"));
        let manifest = ManifestModule::new(format!("com.apkaxiom.miss{i}"))
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_application_label("@string/missing_label")
            .with_component(launcher_activity(".Main"));
        out.push((manifest, resources));
    }

    // 20..30: literal-passthrough — no `@string/` prefix, lowering must
    // be a no-op.
    for i in 0..10 {
        let resources = ResourceTable::new(format!("com.apkaxiom.lit{i}"));
        let manifest = ManifestModule::new(format!("com.apkaxiom.lit{i}"))
            .with_target_sdk(34)
            .with_min_sdk(21)
            .with_application_label(format!("Literal Label {i}"))
            .with_component(launcher_activity(".Main"));
        out.push((manifest, resources));
    }

    out
}

// ---------------------------------------------------------------------------
// Summaries
// ---------------------------------------------------------------------------

struct CorpusSummary {
    json: String,
    bytes_concat: Vec<u8>,
    bytes_len: usize,
}

fn corpus_summary<T, F>(
    kind: &'static str,
    items: &[T],
    mut emit: F,
) -> std::io::Result<CorpusSummary>
where
    F: FnMut(usize, &T) -> std::io::Result<Vec<u8>>,
{
    let mut bytes_concat = Vec::with_capacity(items.len() * 256);
    let mut s = String::with_capacity(items.len() * 200);
    s.push('{');
    push_kv_str(&mut s, "schema", "apkaxiom.p14-ir/v1");
    s.push(',');
    push_kv_str(&mut s, "kind", kind);
    s.push(',');
    push_kv_u32(&mut s, "count", items.len() as u32);
    s.push(',');
    s.push_str("\"items\":[");
    for (i, item) in items.iter().enumerate() {
        let bytes = emit(i, item)?;
        bytes_concat.extend_from_slice(&bytes);
        let h = hash::hex(&hash::sha256(&bytes));
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        push_kv_u32(&mut s, "index", i as u32);
        s.push(',');
        push_kv_u32(&mut s, "bytes", bytes.len() as u32);
        s.push(',');
        push_kv_str(&mut s, "sha256", &h);
        s.push('}');
    }
    s.push(']');
    s.push(',');
    let concat_hash = hash::hex(&hash::sha256(&bytes_concat));
    push_kv_str(&mut s, "concat_hash", &concat_hash);
    s.push('}');
    s.push('\n');
    let bytes_len = bytes_concat.len();
    Ok(CorpusSummary {
        json: s,
        bytes_concat,
        bytes_len,
    })
}

fn build_summary(
    manifests: &CorpusSummary,
    resources: &CorpusSummary,
    lowering_count: usize,
    schema_hash: &str,
    lowering_hash: &str,
) -> String {
    let mut s = String::with_capacity(512);
    s.push('{');
    push_kv_str(&mut s, "schema", "apkaxiom.p14-ir/v1");
    s.push(',');
    push_kv_str(&mut s, "axiom_ir_version", SCHEMA_VERSION);
    s.push(',');
    push_kv_str(&mut s, "producer", PRODUCER_TAG);
    s.push(',');
    push_kv_u32(&mut s, "manifest_count", MANIFEST_COUNT as u32);
    s.push(',');
    push_kv_u32(&mut s, "resource_count", RESOURCE_COUNT as u32);
    s.push(',');
    push_kv_u32(&mut s, "lowering_count", lowering_count as u32);
    s.push(',');
    push_kv_str(
        &mut s,
        "manifest_concat_hash",
        &corpus_concat_hash(manifests),
    );
    s.push(',');
    push_kv_str(
        &mut s,
        "resource_concat_hash",
        &corpus_concat_hash(resources),
    );
    s.push(',');
    push_kv_str(&mut s, "lowering_concat_hash", lowering_hash);
    s.push(',');
    push_kv_str(&mut s, "corpus_root_hash", schema_hash);
    s.push('}');
    s.push('\n');
    s
}

fn corpus_concat_hash(c: &CorpusSummary) -> String {
    hash::hex(&hash::sha256(&c.bytes_concat))
}

// ---------------------------------------------------------------------------
// Lowering inspection JSON
// ---------------------------------------------------------------------------

fn lowering_inspection_json(
    index: usize,
    pre: &ManifestModule,
    resources: &ResourceTable,
    res: &lowering::ResolveResult,
) -> String {
    let mut s = String::with_capacity(1024);
    s.push('{');
    push_kv_u32(&mut s, "index", index as u32);
    s.push(',');
    s.push_str("\"manifest_pre\":");
    s.push_str(&ir_json::encode_manifest(pre));
    s.push(',');
    s.push_str("\"resources\":");
    s.push_str(&ir_json::encode_resource(resources));
    s.push(',');
    s.push_str("\"manifest_post\":");
    s.push_str(&ir_json::encode_manifest(&res.manifest));
    s.push(',');
    s.push_str("\"diagnostics\":[");
    for (i, d) in res.diagnostics.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        push_kv_str(
            &mut s,
            "severity",
            match d.severity {
                axiom_ir::core::Severity::Error => "error",
                axiom_ir::core::Severity::Warning => "warning",
                axiom_ir::core::Severity::Info => "info",
            },
        );
        s.push(',');
        s.push_str("\"message\":");
        push_string(&mut s, &d.message);
        s.push('}');
    }
    s.push(']');
    s.push('}');
    s
}

// ---------------------------------------------------------------------------
// Type table
// ---------------------------------------------------------------------------

fn type_table_json() -> String {
    let mut s = String::with_capacity(1024);
    s.push('{');
    push_kv_str(&mut s, "schema", "apkaxiom.p14-ir/v1");
    s.push(',');
    s.push_str("\"types\":[");
    let scalars: &[Type] = &[
        Type::Tribool,
        Type::U32,
        Type::I32,
        Type::String,
        Type::Bytes,
        Type::ResourceRef,
        Type::PermissionRef,
        Type::ComponentName,
        Type::ApiLevel,
    ];
    let constructors = [
        Type::List(Box::new(Type::U32)),
        Type::Option(Box::new(Type::String)),
    ];
    let mut first = true;
    for t in scalars.iter().chain(constructors.iter()) {
        if !first {
            s.push(',');
        }
        first = false;
        s.push('{');
        push_kv_str(&mut s, "name", &ir_json::type_str(t));
        s.push(',');
        push_kv_u32(&mut s, "tag", u32::from(t.tag()));
        s.push(',');
        s.push_str("\"is_scalar\":");
        s.push_str(if t.is_scalar() { "true" } else { "false" });
        s.push('}');
    }
    s.push(']');
    s.push(',');
    s.push_str("\"attributes\":[");
    let attrs: [(&str, Attribute); 7] = [
        ("Bool", Attribute::Bool(false)),
        ("Tribool", Attribute::Tribool(Tribool::Default)),
        ("U32", Attribute::U32(0)),
        ("I32", Attribute::I32(0)),
        ("String", Attribute::String(String::new())),
        ("Bytes", Attribute::Bytes(Vec::new())),
        ("ApiLevel", Attribute::ApiLevel(0)),
    ];
    for (i, (name, a)) in attrs.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        push_kv_str(&mut s, "name", name);
        s.push(',');
        push_kv_u32(&mut s, "tag", u32::from(a.tag()));
        s.push('}');
    }
    s.push(']');
    s.push('}');
    s.push('\n');
    s
}

// ---------------------------------------------------------------------------
// JSON Schema (Draft 2020-12) for the stable JSON output.
//
// Hand-rolled, deterministic. Describes the shape of the JSON produced
// by `ir_json::encode_manifest` and `ir_json::encode_resource`.
// Downstream SDKs (P4 py / go / ts) can consume this schema directly
// rather than re-deriving the shape from source.
// ---------------------------------------------------------------------------

fn json_schema() -> String {
    let s = r##"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://apkaxiom.dev/schema/axiom-ir-v0.1.json",
  "title": "AXIOM-IR v0.1 stable JSON shape",
  "description": "Schema for ir_json::encode_manifest and ir_json::encode_resource output. Pinned by docs/phase-1/P1.4/CHECKLIST.md §F.",
  "type": "object",
  "$defs": {
    "tribool": { "enum": ["true", "false", "default"] },
    "componentKind": { "enum": ["activity", "service", "receiver", "provider"] },
    "protectionLevel": { "enum": ["normal", "dangerous", "signature", "signatureOrSystem", "internal"] },
    "resourceType": { "enum": ["string", "drawable", "layout", "color", "dimen", "style", "bool", "integer", "raw"] },
    "intentFilter": {
      "type": "object",
      "required": ["actions", "categories", "data", "priority"],
      "properties": {
        "actions": { "type": "array", "items": { "type": "string" } },
        "categories": { "type": "array", "items": { "type": "string" } },
        "data": { "type": "array", "items": { "$ref": "#/$defs/dataFilter" } },
        "priority": { "type": "integer" }
      }
    },
    "dataFilter": {
      "type": "object",
      "properties": {
        "scheme":       { "type": ["string", "null"] },
        "host":         { "type": ["string", "null"] },
        "port":         { "type": ["string", "null"] },
        "path":         { "type": ["string", "null"] },
        "path_prefix":  { "type": ["string", "null"] },
        "path_pattern": { "type": ["string", "null"] },
        "mime_type":    { "type": ["string", "null"] }
      }
    },
    "authority": {
      "type": "object",
      "required": ["host", "port"],
      "properties": {
        "host": { "type": "string" },
        "port": { "type": ["string", "null"] }
      }
    },
    "permission": {
      "type": "object",
      "required": ["name", "protection", "group"],
      "properties": {
        "name":       { "type": "string" },
        "protection": { "$ref": "#/$defs/protectionLevel" },
        "group":      { "type": ["string", "null"] }
      }
    },
    "component": {
      "type": "object",
      "required": ["kind", "name", "exported", "enabled", "is_exported", "permission", "intent_filters", "authorities"],
      "properties": {
        "kind":            { "$ref": "#/$defs/componentKind" },
        "name":            { "type": "string" },
        "exported":        { "$ref": "#/$defs/tribool" },
        "enabled":         { "$ref": "#/$defs/tribool" },
        "is_exported":     { "type": "boolean" },
        "permission":      { "type": ["string", "null"] },
        "intent_filters":  { "type": "array", "items": { "$ref": "#/$defs/intentFilter" } },
        "authorities":     { "type": "array", "items": { "$ref": "#/$defs/authority" } }
      }
    },
    "manifest": {
      "type": "object",
      "required": ["package", "target_sdk", "min_sdk", "application_label", "components", "permissions", "uses_permissions"],
      "properties": {
        "package":           { "type": "string" },
        "target_sdk":        { "type": "integer", "minimum": 0, "maximum": 255 },
        "min_sdk":           { "type": "integer", "minimum": 0, "maximum": 255 },
        "application_label": { "type": ["string", "null"] },
        "components":        { "type": "array", "items": { "$ref": "#/$defs/component" } },
        "permissions":       { "type": "array", "items": { "$ref": "#/$defs/permission" } },
        "uses_permissions":  { "type": "array", "items": { "type": "string" } }
      }
    },
    "resourceRef": {
      "type": "object",
      "required": ["type", "id", "name"],
      "properties": {
        "type": { "$ref": "#/$defs/resourceType" },
        "id":   { "type": "integer", "minimum": 0, "maximum": 4294967295 },
        "name": { "type": "string" }
      }
    },
    "resourceValue": {
      "oneOf": [
        { "type": "object", "required": ["kind", "value"], "properties": { "kind": { "const": "string" }, "value": { "type": "string" } } },
        { "type": "object", "required": ["kind", "value"], "properties": { "kind": { "const": "int" },    "value": { "type": "integer" } } },
        { "type": "object", "required": ["kind", "value"], "properties": { "kind": { "const": "bool" },   "value": { "type": "boolean" } } },
        { "type": "object", "required": ["kind", "value"], "properties": { "kind": { "const": "ref" },    "value": { "$ref": "#/$defs/resourceRef" } } }
      ]
    },
    "resourceEntry": {
      "type": "object",
      "required": ["ref", "value"],
      "properties": {
        "ref":   { "$ref": "#/$defs/resourceRef" },
        "value": { "$ref": "#/$defs/resourceValue" }
      }
    },
    "configuration": {
      "type": "object",
      "required": ["qualifier", "density_dpi", "locale", "orientation", "min_sdk"],
      "properties": {
        "qualifier":   { "type": "string" },
        "density_dpi": { "type": "integer", "minimum": 0, "maximum": 4294967295 },
        "locale":      { "type": ["string", "null"] },
        "orientation": { "type": ["string", "null"] },
        "min_sdk":     { "type": "integer", "minimum": 0, "maximum": 255 }
      }
    },
    "resourceTable": {
      "type": "object",
      "required": ["package", "string_pool_size", "configurations", "entries"],
      "properties": {
        "package":          { "type": "string" },
        "string_pool_size": { "type": "integer", "minimum": 0, "maximum": 4294967295 },
        "configurations":   { "type": "array", "items": { "$ref": "#/$defs/configuration" } },
        "entries":          { "type": "array", "items": { "$ref": "#/$defs/resourceEntry" } }
      }
    }
  },
  "oneOf": [
    { "$ref": "#/$defs/manifest" },
    { "$ref": "#/$defs/resourceTable" }
  ]
}
"##;
    s.to_string()
}

// ---------------------------------------------------------------------------
// IO helpers
// ---------------------------------------------------------------------------

fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Hand-rolled JSON helpers (mirror crates/axiom-ir/src/json.rs shape)
// ---------------------------------------------------------------------------

fn push_kv_str(out: &mut String, key: &str, value: &str) {
    push_string(out, key);
    out.push(':');
    push_string(out, value);
}

fn push_kv_u32(out: &mut String, key: &str, value: u32) {
    push_string(out, key);
    out.push(':');
    out.push_str(&value.to_string());
}

fn push_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
