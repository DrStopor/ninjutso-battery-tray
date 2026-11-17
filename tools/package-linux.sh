#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
out=${1:-target/linux-package}
mkdir -p -- "$out"
out=$(cd -- "$out" && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { printf 'Invalid source version\n' >&2; exit 1; }
arch=$(dpkg --print-architecture)
[[ "$arch" == amd64 ]] || { printf 'Only the amd64 package has been validated.\n' >&2; exit 1; }
bash tools/verify-licenses.sh
cargo build --release --locked
binary=${CARGO_TARGET_DIR:-target}/release/ninjutso-battery-tray
[[ "$binary" = /* ]] || binary="$root/$binary"
[[ $("$binary" --version) == "Ninjutso Battery Tray $version" ]]
desktop-file-validate packaging/linux/io.github.DrStopor.NinjutsoBatteryTray.desktop
stage=$(mktemp -d "$out/.ninjutso-stage.XXXXXXXX")
trap 'rm -rf -- "$stage"' EXIT
package="$stage/package"
install -Dm755 "$binary" "$package/usr/bin/ninjutso-battery-tray"
strip --strip-unneeded "$package/usr/bin/ninjutso-battery-tray"
install -Dm644 packaging/linux/70-ninjutso-battery-tray.rules "$package/usr/lib/udev/rules.d/70-ninjutso-battery-tray.rules"
install -Dm644 packaging/linux/io.github.DrStopor.NinjutsoBatteryTray.desktop "$package/usr/share/applications/io.github.DrStopor.NinjutsoBatteryTray.desktop"
install -Dm644 assets/ui/app.svg "$package/usr/share/icons/hicolor/scalable/apps/io.github.DrStopor.NinjutsoBatteryTray.svg"
for file in LICENSE THIRD_PARTY_NOTICES.txt CHANGELOG.md; do
    install -Dm644 "$file" "$package/usr/share/doc/ninjutso-battery-tray/$file"
done
install -Dm644 docs/linux.md "$package/usr/share/doc/ninjutso-battery-tray/linux.md"
install -Dm644 licenses/MPL-SOURCES.md "$package/usr/share/doc/ninjutso-battery-tray/licenses/MPL-SOURCES.md"
install -Dm644 licenses/copyleft-sources.sha256 "$package/usr/share/doc/ninjutso-battery-tray/licenses/copyleft-sources.sha256"
install -Dm644 vendor/xattr/UPSTREAM.md "$package/usr/share/doc/ninjutso-battery-tray/licenses/xattr-UPSTREAM.md"
for file in UPSTREAM.md SHA256SUMS source-sans-3/OFL.txt noto-sans-sc/OFL.txt noto-sans-sc/NOTICE.txt; do
    install -Dm644 "assets/fonts/$file" "$package/usr/share/doc/ninjutso-battery-tray/licenses/fonts/$file"
done
bash tools/collect-license-sources.sh "$package/usr/share/doc/ninjutso-battery-tray/licenses/sources"
mkdir -p "$package/DEBIAN" "$stage/debian"
install -m644 packaging/linux/source-control "$stage/debian/control"
dependencies=$(cd "$stage" && dpkg-shlibdeps -O -e "$package/usr/bin/ninjutso-battery-tray")
dependencies=${dependencies#shlibs:Depends=}
[[ -n "$dependencies" && "$dependencies" != *shlibs:* ]] || { printf 'Could not resolve ELF dependencies\n' >&2; exit 1; }
sed -e "s/@VERSION@/$version/g" -e "s/@ARCH@/$arch/g" -e "s/@DEPENDS@/$dependencies/g" \
    packaging/linux/control.in > "$package/DEBIAN/control"
install -m755 packaging/linux/postinst packaging/linux/postrm "$package/DEBIAN/"
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}
deb="ninjutso-battery-tray_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$package" "$stage/$deb"
# Keep ELF, desktop integration, permissions and license files together in the
# archive too. This archive does not install rules or alter the host by itself.
archive="ninjutso-battery-tray-${version}-linux-x86_64.tar.gz"
tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
    -C "$package" -czf "$stage/$archive" usr
install -m644 "$stage/$deb" "$stage/$archive" "$out/"
(cd "$out" && sha256sum "$deb" "$archive" > SHA256SUMS.txt)
printf 'Built local packages in %s (nothing published).\n' "$out"
