#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
while read -r expected path; do
    path=${path%$'\r'}
    [[ -n "$expected" && -f "$path" ]] || { printf 'Missing license input: %s\n' "$path" >&2; exit 1; }
    actual=$(sed 's/\r$//' "$path" | sha256sum)
    [[ ${actual%% *} == "$expected" ]] || { printf 'Stale notices: %s. Run tools/update-licenses.ps1.\n' "$path" >&2; exit 1; }
done < licenses/inputs.sha256
sed 's/\r$//' assets/fonts/SHA256SUMS | sha256sum --check --strict
printf 'Verified third-party notices and all generation inputs.\n'
