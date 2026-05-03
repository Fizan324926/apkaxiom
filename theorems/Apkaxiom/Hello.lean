-- P1.2 bring-up theorems. Trivial content; the point is to prove the
-- toolchain end-to-end before P1.5 invests in real ZIP-layer formalization.

namespace Apkaxiom.Hello

/-- A trivial proposition for bring-up. -/
theorem zero_is_zero : (0 : Nat) = 0 := rfl

/-- A trivial computable function. Extracted by `tools/lean-to-rust`.
    The `--! test` lines below are read by the extractor (deliberately
    inside Lean comments so Lean ignores them) to generate Rust unit
    tests over a fixed input set. -/
--! test double_zero(0) = 0
--! test double_seven(7) = 14
--! test double_billion(1_000_000_000) = 2_000_000_000
def double (n : Nat) : Nat := 2 * n

/-- Correctness lemma for `double`. Proven by `omega` (linear-arith
    decision procedure shipped with core Lean 4). -/
theorem double_correct : ∀ n, double n = n + n := by
  intro n
  unfold double
  omega

end Apkaxiom.Hello
