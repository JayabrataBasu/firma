#!/usr/bin/env bash
# Manual §25.6 — CI-enforced project lints, implemented as grep/metadata checks.
# Run from anywhere; operates on the repository root. Exit non-zero on violation.
#
# These complement, not replace, the crate-level attributes
# (`#![forbid(unsafe_code)]`, workspace `missing_docs = "deny"`) and
# `cargo clippy --all-targets -- -D warnings`.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
note() { printf '  \033[31mFAIL\033[0m %s\n' "$1"; fail=1; }
ok() { printf '  \033[32m ok \033[0m %s\n' "$1"; }

# Source trees whose code runs inside the simulation step — the
# "result-affecting path" of manual §19.6 and §21.1. firma-io / firma-config /
# firma-cli are orchestration: they may use hash maps as long as iteration order
# never feeds a simulation result.
SIM_PATH_SRC=(
  crates/firma-core/src
  crates/firma-rng/src
  crates/firma-domain/src
  crates/firma-viability/src
  crates/firma-kernel/src
  crates/firma-plugins/firma-plugin-testkit/src
  crates/firma-plugins/firma-plugin-constraint/src
  crates/firma-plugins/firma-plugin-action-market/src
  crates/firma-plugins/firma-plugin-action-shaping/src
  crates/firma-plugins/firma-plugin-decision/src
  crates/firma-plugins/firma-plugin-observation/src
  crates/firma-plugins/firma-plugin-shock/src
  crates/firma-plugins/firma-plugin-locality/src
  crates/firma-plugins/firma-plugin-resource/src
)

echo "[1/9] no-kernel-domain-deps + no-cross-plugin-deps  (§17 A1, §18.1)"
if out=$(cargo metadata --format-version 1 --no-deps 2>/dev/null | python3 scripts/check_deps.py); then
  ok "dependency graph clean"
else
  printf '%s\n' "$out" | sed 's/^/     /'
  note "workspace dependency rule violated"
fi

echo "[2/9] no-hashmap-iteration  (HashMap/HashSet banned in the sim path — §19.2)"
if grep -RInE '\b(HashMap|HashSet|hashbrown|FxHashMap|AHashMap|IndexMap)\b' "${SIM_PATH_SRC[@]}"; then
  note "HashMap/HashSet present in a result-affecting crate (use BTreeMap)"
else
  ok "clean"
fi

echo "[3/9] no-ambient-rng  (no thread_rng / OS entropy / wall clock — §21.1 D2/D4)"
if grep -RInE '\b(thread_rng|rand::random|OsRng|getrandom|SystemTime|Instant::now)\b|std::time::' "${SIM_PATH_SRC[@]}"; then
  note "ambient randomness or wall-clock in the sim path"
else
  ok "clean"
fi

echo "[4/9] no-unsafe  (#![forbid(unsafe_code)] in every crate — §24.5)"
missing=0
while IFS= read -r librs; do
  grep -q 'forbid(unsafe_code)' "$librs" || { note "missing #![forbid(unsafe_code)] in $librs"; missing=1; }
done < <(find crates tests -name lib.rs -path '*/src/*')
if grep -RInE '\bunsafe\b[[:space:]]*\{' crates tests --include='*.rs' 2>/dev/null; then
  note "an 'unsafe' block is present"
fi
[ "$missing" = 0 ] && ok "all crates forbid unsafe"

echo "[5/9] no-dynamic-loading  (no libloading / dlopen — ADR 0008)"
if grep -RInE '\b(libloading|dlopen|dlopen2|LoadLibraryA?)\b' crates --include='*.rs' 2>/dev/null; then
  note "dynamic-loading symbol present"
else
  ok "clean"
fi

echo "[6/9] assumption-nonempty  (§20.2, §25.6)"
if grep -RInE 'fn assumption\(&self\)[^{]*\{[[:space:]]*""[[:space:]]*\}' crates --include='*.rs' 2>/dev/null; then
  note "a plugin returns an empty assumption()"
else
  ok "no empty literal; runtime check in tests/tests/integration.rs"
fi

echo "[7/9] no-magic-numbers  (heuristic tripwire; binding check is review — §24.6)"
# Deliberately narrow: literals in non-test sim-path source, excluding named
# constants, attributes, doc/comments, hex, byte-width array/shift idioms
# (`[u8; 32]`, `<< 32`, `/ 64`), and the RNG's cited Philox/SHA constants.
hits=$(grep -RInE '[^A-Za-z0-9_."/](3[0-9]|[4-9][0-9]|[1-9][0-9]{2,})[^0-9A-Za-z_]' "${SIM_PATH_SRC[@]}" \
  | grep -vE '/tests\.rs:' \
  | grep -vE '(const |static |#\[|// |//!|///|assert|_be_bytes|0x[0-9A-Fa-f])' \
  | grep -vE '(<< 32|>> 32|/ 64|% 64|\* 64|; 32\]|; 64\]|div_ceil|1 << |9_007_199)' \
  || true)
if [ -n "$hits" ]; then
  printf '%s\n' "$hits" | sed 's/^/     /'
  printf '  \033[33mwarn\033[0m review the literals above for named-constant candidates\n'
else
  ok "no obvious bare literals in the sim path"
fi

echo "[8/9] docs-required  (workspace missing_docs = deny — §24.3)"
grep -q 'missing_docs = "deny"' Cargo.toml && ok "workspace lint set" || note "missing_docs deny not set"

echo "[9/9] no-tui-kernel-deps  (firma-tui MUST NOT link firma-kernel — §23.2, ADR 0045)"
if command -v cargo >/dev/null 2>&1 && [ -d crates/firma-tui ]; then
  if cargo tree -p firma-tui -e normal --prefix none 2>/dev/null | grep -q '^firma-kernel '; then
    note "firma-tui's resolved dependency tree includes firma-kernel"
  else
    ok "firma-tui carries no firma-kernel dependency, direct or transitive"
  fi
else
  note "firma-tui crate not found — cannot check"
fi

echo
if [ "$fail" -ne 0 ]; then echo "architecture lints: FAILED"; exit 1; fi
echo "architecture lints: passed"
