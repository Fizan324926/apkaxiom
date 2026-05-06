/-
P1.11 G17 — properties of `Apkaxiom.Signing.X509`.

Mechanical theorems over the X.509 SPKI extractor's error tags +
canonical wire-format constants.
-/

import Std
import Apkaxiom.Signing.X509

namespace Apkaxiom.Signing.X509.Properties

open Apkaxiom.Signing.X509

/-! ## Tag bytes -/

theorem sequence_tag_byte_value : sequenceTagByte = 0x30 := by native_decide
theorem context0_tag_byte_value : context0TagByte = 0xa0 := by native_decide
theorem integer_tag_byte_value : integerTagByte = 0x02 := by native_decide

theorem sequence_distinct_context0 : sequenceTagByte ≠ context0TagByte := by native_decide
theorem sequence_distinct_integer  : sequenceTagByte ≠ integerTagByte := by native_decide
theorem context0_distinct_integer  : context0TagByte ≠ integerTagByte := by native_decide

/-! ## X509Error tag injectivity -/

theorem x509_error_tag_missing_outer_distinct_tbs :
    X509Error.missingOuter.tag ≠ X509Error.missingTbs.tag := by native_decide
theorem x509_error_tag_missing_tbs_distinct_spki :
    X509Error.missingTbs.tag ≠ X509Error.missingSpki.tag := by native_decide
theorem x509_error_tag_missing_outer_distinct_spki :
    X509Error.missingOuter.tag ≠ X509Error.missingSpki.tag := by native_decide
theorem x509_error_tag_missing_outer_distinct_wrong :
    X509Error.missingOuter.tag ≠ (X509Error.wrongTag 0 0).tag := by native_decide

/-! ## Extractor on synthetic inputs -/

/-- Empty input rejects with missingOuter (or missingTbs, depending
    on parser path) — never returns success. -/
theorem extract_spki_empty_rejects :
    (extractSpkiDer ByteArray.empty).toOption = none := by native_decide

/-- A SEQUENCE { INTEGER 1 } is too short to be a cert; rejects. -/
theorem extract_spki_minimal_seq_rejects :
    (extractSpkiDer (ByteArray.mk #[0x30, 0x03, 0x02, 0x01, 0x01])).toOption = none := by
  native_decide

/-- A SEQUENCE that doesn't start with SEQUENCE tag rejects. -/
theorem extract_spki_non_sequence_outer_rejects :
    (extractSpkiDer (ByteArray.mk #[0x02, 0x03, 0x01, 0x02, 0x03])).toOption = none := by
  native_decide

end Apkaxiom.Signing.X509.Properties
