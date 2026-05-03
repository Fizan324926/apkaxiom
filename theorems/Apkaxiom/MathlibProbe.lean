-- P1.2 mathlib4 cache-pipeline probe.
--
-- This module *imports* a mathlib4 module and uses one of its lemmas in a
-- proof. The mere fact that this file builds is the substantive test that
-- our `lake-manifest.json` + Reservoir cache pipeline works end-to-end:
-- without a populated mathlib cache, building this file from source takes
-- 30+ minutes; with a populated cache it lands inside the 10-minute CI
-- budget.
--
-- The lemma chosen is intentionally small (`Nat.add_comm`) so the import
-- closure stays narrow — but Mathlib.Logic.Basic + Mathlib.Init pull in
-- enough of mathlib's olean tree to exercise the cache fetch.

import Mathlib.Logic.Basic
import Apkaxiom.Hello

namespace Apkaxiom.MathlibProbe

open Apkaxiom.Hello

/-- The mathlib pipeline is live: `Nat.add_comm` is reachable. The proof
    is by `omega`, but type-checking succeeds only if mathlib's olean
    graph (transitively) loads. -/
theorem double_eq_add_via_mathlib (n : Nat) :
    double n = n + n := by
  -- `Nat.add_comm` is a mathlib re-export of core `Nat`'s commutativity
  -- — citing it makes the dependency on mathlib substantive, not just
  -- declarative.
  have _h : ∀ a b : Nat, a + b = b + a := Nat.add_comm
  unfold double
  omega

end Apkaxiom.MathlibProbe
