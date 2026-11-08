#!/usr/bin/env bash
# Package AloLite as a Red Hat package (.rpm)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite-cli") | .version')}"

echo "=== Packaging .rpm for AloLite v${VERSION} ==="
if [ ! -f target/release/alolite ] || [ ! -f target/release/alolite-desktop ]; then
  (cd apps/desktop && bun install --frozen-lockfile && bun run build)
  cargo build --release --locked --bin alolite --bin alolite-desktop
  strip target/release/alolite target/release/alolite-desktop || true
fi

mkdir -p dist rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cat > rpmbuild/SPECS/alolite.spec << EOF
Name:           alolite
Version:        ${VERSION}
Release:        1
Summary:        Lightweight native messaging client in Rust + Tauri
License:        MIT OR Apache-2.0
URL:            https://github.com/ngxccc/alolite
BuildArch:      x86_64
AutoReqProv:    no
Requires:       gtk3, webkit2gtk4.1, libsoup3
Recommends:     gstreamer1-plugins-base, gstreamer1-plugins-good, gstreamer1-plugins-bad-free, gstreamer1-libav
%description
Lightweight native desktop messaging client written in Rust + Tauri.

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps
install -m 755 %{_topdir}/../target/release/alolite %{buildroot}/usr/bin/alolite
install -m 755 %{_topdir}/../target/release/alolite-desktop %{buildroot}/usr/bin/alolite-desktop
install -m 644 %{_topdir}/../packaging/linux/alolite.desktop %{buildroot}/usr/share/applications/alolite.desktop
install -m 644 %{_topdir}/../crates/alolite-core/assets/app_icon.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/alolite.png

%files
/usr/bin/alolite
/usr/bin/alolite-desktop
/usr/share/applications/alolite.desktop
/usr/share/icons/hicolor/256x256/apps/alolite.png
EOF

rpmbuild --define "_topdir $(pwd)/rpmbuild" -bb rpmbuild/SPECS/alolite.spec
cp rpmbuild/RPMS/x86_64/*.rpm "dist/alolite-${VERSION}-1.x86_64.rpm"
rm -rf rpmbuild
echo "Created: dist/alolite-${VERSION}-1.x86_64.rpm"
