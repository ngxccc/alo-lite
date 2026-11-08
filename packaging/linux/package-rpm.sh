#!/usr/bin/env bash
# Package Alolite as a Red Hat package (.rpm)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

VERSION="${1:-$(cargo metadata --locked --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "alolite") | .version')}"

echo "=== Packaging .rpm for Alolite v${VERSION} ==="
if [ ! -f target/release/alolite ]; then
  echo "Building release binary..."
  cargo build --release --locked --bin alolite
  strip target/release/alolite || true
fi

mkdir -p dist rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

cat > rpmbuild/SPECS/alolite.spec << EOF
Name:           alolite
Version:        ${VERSION}
Release:        1
Summary:        Lightweight native messaging client in Rust
License:        MIT OR Apache-2.0
URL:            https://github.com/ngxccc/alolite
BuildArch:      x86_64
AutoReqProv:    no
Requires:       libX11, libxcb, libxkbcommon

%description
Lightweight native desktop messaging client written in Rust (egui GUI).

%install
mkdir -p %{buildroot}/usr/bin
mkdir -p %{buildroot}/usr/share/applications
mkdir -p %{buildroot}/usr/share/icons/hicolor/256x256/apps
install -m 755 %{_topdir}/../target/release/alolite %{buildroot}/usr/bin/alolite
install -m 644 %{_topdir}/../packaging/linux/alolite.desktop %{buildroot}/usr/share/applications/alolite.desktop
install -m 644 %{_topdir}/../src/assets/app_icon.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/alolite.png

%files
/usr/bin/alolite
/usr/share/applications/alolite.desktop
/usr/share/icons/hicolor/256x256/apps/alolite.png
EOF

rpmbuild --define "_topdir $(pwd)/rpmbuild" -bb rpmbuild/SPECS/alolite.spec
cp rpmbuild/RPMS/x86_64/*.rpm "dist/alolite-${VERSION}-1.x86_64.rpm"
rm -rf rpmbuild
echo "✓ Created: dist/alolite-${VERSION}-1.x86_64.rpm"
