# AXIOM-IR v0.1 — Cap'n Proto wire schema (declarative).
#
# This schema mirrors the Rust reference IR in `crates/axiom-ir/`. It is
# *not* a runtime dependency — APKAXIOM Phase 1 emits canonical bytes
# defined by `crates/axiom-ir/src/canonical.rs`, not Cap'n Proto frames.
# The schema text is committed for two reasons:
#
#   1. Inter-process IR transmission (Phase 4+) will plug Cap'n Proto in
#      as the wire format. Pinning the schema text now lets that work
#      arrive without a fresh design round.
#
#   2. Drift detection. The CI gate `p14-ir-drift` includes a SHA-256
#      hash of THIS file (`docs/phase-1/P1.4/ir-data/schema-capnp-hash.txt`).
#      Any drift between the Rust IR and this schema flips the hash and
#      flunks the gate.
#
# File ID generated via `capnp id` once and pinned. Do not regenerate.

@0xa9c1d4b1f7e23d51;

# ---------------------------------------------------------------------------
# Core kernel
# ---------------------------------------------------------------------------

struct Module {
  producer       @0 :Text;
  dialectTag     @1 :Text;
  attributes     @2 :List(Attribute);
  region         @3 :Region;
  nextValueId    @4 :UInt32;
}

struct Region {
  blocks @0 :List(Block);
}

struct Block {
  label @0 :Text;
  ops   @1 :List(Operation);
}

struct Operation {
  name        @0 :Text;
  operands    @1 :List(UInt32);
  results     @2 :List(Value);
  attributes  @3 :List(Attribute);
  regions     @4 :List(Region);
}

struct Value {
  id @0 :UInt32;
  ty @1 :Type;
}

struct Type {
  union {
    tribool       @0 :Void;
    u32           @1 :Void;
    i32           @2 :Void;
    string        @3 :Void;
    bytes         @4 :Void;
    resourceRef   @5 :Void;
    permissionRef @6 :Void;
    componentName @7 :Void;
    apiLevel      @8 :Void;
    list          @9 :Type;
    option        @10 :Type;
  }
}

struct Attribute {
  key @0 :Text;
  value :union {
    boolValue      @1 :Bool;
    triboolValue   @2 :TriboolValue;
    u32Value       @3 :UInt32;
    i32Value       @4 :Int32;
    stringValue    @5 :Text;
    bytesValue     @6 :Data;
    apiLevelValue  @7 :UInt8;
  }
}

enum TriboolValue {
  trueValue    @0;
  falseValue   @1;
  defaultValue @2;
}

# ---------------------------------------------------------------------------
# Manifest dialect
# ---------------------------------------------------------------------------

struct ManifestModule {
  package           @0 :Text;
  targetSdk         @1 :UInt8;
  minSdk            @2 :UInt8;
  applicationLabel  @3 :Text;       # empty if absent
  hasApplicationLabel @4 :Bool;     # `true` iff `applicationLabel` is set
  components        @5 :List(Component);
  permissions       @6 :List(Permission);
  usesPermissions   @7 :List(Text);
}

struct Component {
  kind          @0 :ComponentKind;
  name          @1 :Text;
  exported      @2 :TriboolValue;
  enabled       @3 :TriboolValue;
  permission    @4 :Text;
  hasPermission @5 :Bool;
  intentFilters @6 :List(IntentFilter);
  authorities   @7 :List(DataAuthority);
}

enum ComponentKind {
  activity @0;
  service  @1;
  receiver @2;
  provider @3;
}

struct IntentFilter {
  actions    @0 :List(Text);
  categories @1 :List(Text);
  data       @2 :List(DataFilter);
  priority   @3 :Int32;
}

struct DataFilter {
  scheme       @0 :Text;
  host         @1 :Text;
  port         @2 :Text;
  path         @3 :Text;
  pathPrefix   @4 :Text;
  pathPattern  @5 :Text;
  mimeType     @6 :Text;
  hasScheme       @7 :Bool;
  hasHost         @8 :Bool;
  hasPort         @9 :Bool;
  hasPath         @10 :Bool;
  hasPathPrefix   @11 :Bool;
  hasPathPattern  @12 :Bool;
  hasMimeType     @13 :Bool;
}

struct DataAuthority {
  host    @0 :Text;
  port    @1 :Text;
  hasPort @2 :Bool;
}

struct Permission {
  name        @0 :Text;
  protection  @1 :ProtectionLevel;
  group       @2 :Text;
  hasGroup    @3 :Bool;
}

enum ProtectionLevel {
  normal             @0;
  dangerous          @1;
  signatureLevel     @2;
  signatureOrSystem  @3;
  internalLevel      @4;
}

# ---------------------------------------------------------------------------
# Resource dialect
# ---------------------------------------------------------------------------

struct ResourceTable {
  package         @0 :Text;
  stringPool      @1 :StringPool;
  configurations  @2 :List(Configuration);
  entries         @3 :List(ResourceEntry);
}

struct StringPool {
  strings @0 :List(Text);
}

struct Configuration {
  qualifier      @0 :Text;
  densityDpi     @1 :UInt32;
  locale         @2 :Text;
  hasLocale      @3 :Bool;
  orientation    @4 :Text;
  hasOrientation @5 :Bool;
  minSdk         @6 :UInt8;
}

struct ResourceEntry {
  ref   @0 :ResourceRef;
  value @1 :ResourceValue;
}

struct ResourceRef {
  type @0 :ResourceType;
  id   @1 :UInt32;
  name @2 :Text;
}

enum ResourceType {
  stringRes   @0;
  drawableRes @1;
  layoutRes   @2;
  colorRes    @3;
  dimenRes    @4;
  styleRes    @5;
  boolRes     @6;
  integerRes  @7;
  rawRes      @8;
}

struct ResourceValue {
  union {
    stringValue @0 :Text;
    intValue    @1 :Int32;
    boolValue   @2 :Bool;
    refValue    @3 :ResourceRef;
  }
}
