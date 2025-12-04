#!/usr/bin/env bash
set -euo pipefail

# usage:
# ./remaster-tc.sh --iso-url URL --bins-dir path/to/bins --out-iso name.iso

ISO_URL=""
BINS_DIR=""
OUT_ISO="tinycore-custom.iso"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --iso-url) ISO_URL="$2"; shift 2 ;;
    --bins-dir) BINS_DIR="$2"; shift 2 ;;
    --out-iso) OUT_ISO="$2"; shift 2 ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [[ -z "$ISO_URL" || -z "$BINS_DIR" ]]; then
  echo "Usage: $0 --iso-url URL --bins-dir path --out-iso name.iso"
  exit 1
fi

WORKDIR=$(mktemp -d)
cleanup() { rm -rf "$WORKDIR"; }
trap cleanup EXIT

cd "$WORKDIR"

echo "Downloading base ISO..."
curl -L -o base.iso "$ISO_URL"

echo "Mounting ISO (read‑only)..."
sudo mkdir mnt
sudo mount -o loop base.iso mnt
rsync -a mnt/ iso_tree/
sudo umount mnt

BOOT_DIR="iso_tree/boot"
# find initramfs inside boot
for c in core.gz corepure64.gz rootfs64.gz rootfs.gz tinycore.gz; do
  if [[ -f "$BOOT_DIR/$c" ]]; then
    CORE_GZ="$BOOT_DIR/$c"
    break
  fi
done

if [[ -z "${CORE_GZ:-}" ]]; then
  echo "Error: could not find initramfs in $BOOT_DIR" >&2
  exit 1
fi
echo "Found initramfs: $CORE_GZ"

# Prepare a directory for unpacking
mkdir initramfs_unpack

echo "Unpacking initramfs under fakeroot..."
fakeroot sh -c " \
  cd initramfs_unpack && \
  zcat \"$WORKDIR/$BOOT_DIR/$(basename $CORE_GZ)\" | cpio -id --no-absolute-filenames \
"

echo "Copying user binaries ..."
# Copy all executables from BINS_DIR into initramfs_unpack/usr/bin/
mkdir -p initramfs_unpack/usr/bin
find "$BINS_DIR" -type f -perm /a+x | while read -r bin; do
  cp "$bin" initramfs_unpack/usr/bin/
done

echo "Repacking initramfs under fakeroot..."
fakeroot sh -c " \
  cd initramfs_unpack && \
  find . | cpio -o -H newc --owner root:root | gzip -9 > \"$WORKDIR/new_core.gz\" \
"

echo "Replacing old initramfs with new one..."
cp "$WORKDIR/new_core.gz" "$BOOT_DIR/$(basename $CORE_GZ)"

echo "Building new ISO (BIOS + UEFI hybrid)..."
# Note: installing xorriso, isolinux etc must be done before calling this script
ISOHPFX=$(find /usr -name isohdpfx.bin -print -quit || true)
ISOLINUX_BIN=$(find /usr -name isolinux.bin -print -quit || true)

XORRISO=(xorriso -as mkisofs)
if [[ -n "$ISOHPFX" ]]; then
  XORRISO+=( -isohybrid-mbr "$ISOHPFX" )
fi
if [[ -n "$ISOLINUX_BIN" ]]; then
  XORRISO+=( -b isolinux/isolinux.bin -c isolinux/boot.cat -no-emul-boot -boot-load-size 4 -boot-info-table )
fi

XORRISO+=( -o "$OUT_ISO" iso_tree )

"${XORRISO[@]}"

echo "Done: created ISO $OUT_ISO"
