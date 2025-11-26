#!/bin/bash
# Build script for cosmic-monitor-control-applet RPM

set -e

NAME="cosmic-monitor-control-applet"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Read version from Cargo.toml
VERSION=$(grep -m1 '^version' "${PROJECT_ROOT}/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')
if [ -z "$VERSION" ]; then
    echo "Error: Could not read version from Cargo.toml"
    exit 1
fi

SPEC_FILE="${SCRIPT_DIR}/cosmic-monitor-control-applet-simple.spec"

echo "==> Building ${NAME} RPM v${VERSION}"
echo "==> Using spec file: ${SPEC_FILE}"

# Check if required tools are installed
command -v rpmbuild >/dev/null 2>&1 || {
    echo "Error: rpmbuild not found. Install it with:"
    echo "  sudo dnf install rpm-build rpmdevtools"
    exit 1
}

command -v just >/dev/null 2>&1 || {
    echo "Error: just not found. Install it with:"
    echo "  cargo install just"
    exit 1
}

# Setup RPM build directories
echo "==> Setting up RPM build directories"
rpmdev-setuptree

# Create source tarball
echo "==> Creating source tarball"
TARBALL="${NAME}-${VERSION}.tar.gz"
TEMP_DIR=$(mktemp -d)
SRC_DIR="${TEMP_DIR}/${NAME}-${VERSION}"

# Copy source files
mkdir -p "${SRC_DIR}"
cd "${PROJECT_ROOT}"
cp -r \
    src/ \
    res/ \
    i18n/ \
    data/ \
    Cargo.toml \
    Cargo.lock \
    justfile \
    LICENSE \
    README.md \
    flatpak_schema.json \
    i18n.toml \
    "${SRC_DIR}/"

# Create tarball
cd "${TEMP_DIR}"
tar czf "${TARBALL}" "${NAME}-${VERSION}"
mv "${TARBALL}" ~/rpmbuild/SOURCES/

# Cleanup temp directory
rm -rf "${TEMP_DIR}"

# Copy spec file
echo "==> Copying spec file"
cp "${SPEC_FILE}" ~/rpmbuild/SPECS/

# Build RPM
echo "==> Building RPM"
cd ~/rpmbuild/SPECS
rpmbuild -ba "${SPEC_FILE}"

echo ""
echo "==> Build complete!"
echo "RPMs available in:"
echo "  Source RPM: ~/rpmbuild/SRPMS/${NAME}-${VERSION}-1.*.src.rpm"
echo "  Binary RPM: ~/rpmbuild/RPMS/$(uname -m)/${NAME}-${VERSION}-1.*.rpm"
echo ""
echo "To install:"
echo "  sudo dnf install ~/rpmbuild/RPMS/$(uname -m)/${NAME}-${VERSION}-1.*.rpm"
