#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
destination=${1:?Usage: collect-license-sources.sh DESTINATION}
mkdir -p -- "$destination"
while read -r expected filename; do
    filename=${filename%$'\r'}
    [[ "$expected" =~ ^[0-9a-f]{64}$ && "$filename" =~ ^[a-zA-Z0-9._-]+\.crate$ ]] || exit 1
    cache=${CARGO_HOME:-$HOME/.cargo}/registry/cache
    source=$(find "$cache" -mindepth 2 -maxdepth 2 -type f -name "$filename" -print -quit 2>/dev/null || true)
    if [[ -n "$source" ]]; then
        cp -- "$source" "$destination/$filename"
    else
        crate=${filename%.crate}; name=${crate%-*}
        curl --fail --location --proto '=https' --tlsv1.2 \
            "https://static.crates.io/crates/$name/$filename" --output "$destination/$filename"
    fi
    actual=$(sha256sum "$destination/$filename")
    [[ ${actual%% *} == "$expected" ]] || { printf 'Source archive checksum mismatch: %s\n' "$filename" >&2; exit 1; }
done < "$root/licenses/copyleft-sources.sha256"
printf 'Collected and verified original MPL source archives.\n'
