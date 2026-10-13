"""Integrity of public inputs, NOT calibration/industrial model validation."""
import csv
import hashlib
from pathlib import Path

root = Path(__file__).resolve().parents[1] / "data/reference"
checks = {
    "geomet/comminution.csv": "1a33df8ba77f5d49c281ba3fff16b20b",
    "geomet/flotation.csv": "f2e90da6bfa81de1261177ee85a91570",
}
for name, expected in checks.items():
    raw = (root / name).read_bytes()
    assert hashlib.md5(raw).hexdigest() == expected, f"Upstream checksum: {name}"
    with (root / name).open(newline="") as source:
        rows = list(csv.DictReader(source))
    assert rows, f"Empty dataset: {name}"
    if "comminution" in name:
        assert all(float(row["F80"]) >= float(row["P80"]) > 0 for row in rows)
    else:
        measured = [row for row in rows if row["LCT"].strip()]
        assert len(measured) >= 3
        assert all(0 <= float(row["LCT"]) <= 1 for row in measured)
        print(f"LCT: {len(measured)} available, {len(rows)-len(measured)} missing; no imputation")
    print(f"{name}: {len(rows)} records; original checksum verified")
for name, expected in {
    "phreeqc/phreeqc.dat": "5b80d45c989cd1db7aab485e198321ba550d5902be8097c8be26d85ba03da278",
    "phreeqc/pitzer.dat": "3640e62aee63a118f800b115b46a2760576e63e05e1792022315a28f75dbe9bb",
}.items():
    assert hashlib.sha256((root / name).read_bytes()).hexdigest() == expected, name
print("Reference inputs verified; no assertion of experimental model equivalence")
