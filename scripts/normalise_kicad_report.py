#!/usr/bin/env python3
"""Normalise a `kicad-cli` JSON report so reviewed baselines stay stable.

Only volatile metadata is removed: the report timestamp and KiCad's
per-load item UUIDs (KiCad assigns fresh item UUIDs at load time, so they
cannot identify a finding). Everything else — including the KiCad version,
every violation, every item, its coordinates, and the emitted UUID path — is
retained because it is exactly what acceptance must compare.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Keys that change on every KiCad load without changing a single finding.
VOLATILE_KEYS = frozenset({"date", "uuid"})

# Arrays whose order is not semantically meaningful. KiCad's report ordering
# can differ between platforms, so sort them by canonical JSON before
# comparison.
UNORDERED_KEYS = frozenset(
    {
        "violations",
        "unconnected_items",
        "schematic_parity",
        "items",
        "ignored_checks",
        "included_severities",
        "sheets",
    }
)


def normalise(value: object, key: str | None = None) -> object:
    if isinstance(value, dict):
        return {
            child_key: normalise(item, child_key)
            for child_key, item in value.items()
            if child_key not in VOLATILE_KEYS
        }
    if isinstance(value, list):
        items = [normalise(item) for item in value]
        if key in UNORDERED_KEYS:
            items.sort(key=lambda item: json.dumps(item, sort_keys=True))
        return items
    return value


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <input.json> <output.json>", file=sys.stderr)
        return 2
    source = Path(sys.argv[1])
    destination = Path(sys.argv[2])
    report = json.loads(source.read_text(encoding="utf-8"))
    destination.write_text(
        json.dumps(normalise(report), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
