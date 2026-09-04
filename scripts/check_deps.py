#!/usr/bin/env python3
"""Manual §25.6 lints `no-kernel-domain-deps` and `no-cross-plugin-deps`.

Reads `cargo metadata` on stdin and prints violations, one per line.
Exits 1 if any violation was printed.
"""
import json
import sys

# firma-kernel's ONLY permitted workspace deps (manual §17 A1, §18.1).
# `firma-domain` is deliberately excluded: it is the crate that defines "firm"
# (FirmState, ConstraintParams, ...), and ADR 0020 keeps it out of the kernel's
# reachable graph. This allowlist is that guard.
KERNEL_ALLOWED = {"firma-core", "firma-rng"}

meta = json.load(sys.stdin)
by_name = {p["name"]: p for p in meta["packages"]}
violations = []

# no-kernel-domain-deps: firma-kernel may depend only on firma-core / firma-rng
# among workspace crates (manual §17 A1, §18.1).
kernel = by_name.get("firma-kernel")
if kernel:
    for dep in kernel["dependencies"]:
        name = dep["name"]
        if name.startswith("firma-") and name not in KERNEL_ALLOWED:
            violations.append(f"no-kernel-domain-deps: firma-kernel -> {name}")

# no-cross-plugin-deps: a plugin crate may not depend on another plugin crate
# (manual §18.1 — composition is via the delta/reconciler mechanism only).
for pkg in meta["packages"]:
    if "firma-plugin" not in pkg["name"]:
        continue
    for dep in pkg["dependencies"]:
        if "firma-plugin" in dep["name"] and dep["name"] != pkg["name"]:
            violations.append(
                f"no-cross-plugin-deps: {pkg['name']} -> {dep['name']}"
            )

for v in violations:
    print(v)
sys.exit(1 if violations else 0)
