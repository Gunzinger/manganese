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

echo "Downloading base ISO from: $ISO_URL"
curl -L -o base.iso "$ISO_URL"

echo "Mounting ISO read-only..."
sudo mkdir mnt
sudo mount -o loop,ro base.iso mnt

echo "Copying ISO contents to writable tree..."
# Copy everything but do not preserve ownership/group (so we become owner)
rsync -a --no-owner --no-group mnt/ iso_tree/

sudo umount mnt
rm -rf mnt

# Ensure our copy is writable by us
chmod -R u+w iso_tree

BOOT_DIR="iso_tree/boot"
CORE_GZ=""
for c in core.gz corepure64.gz rootfs64.gz rootfs.gz tinycore.gz; do
  if [[ -f "$BOOT_DIR/$c" ]]; then
    CORE_GZ="$BOOT_DIR/$c"
    break
  fi
done

if [[ -z "$CORE_GZ" ]]; then
  echo "Error: Could not find initramfs (core.gz) inside ISO tree." >&2
  exit 1
fi

echo "Found initramfs: $CORE_GZ"

mkdir initrd_unpacked

echo "Unpacking initramfs under fakeroot..."
fakeroot sh -c " \
  cd \"$WORKDIR/initrd_unpacked\" && \
  zcat \"$WORKDIR/$CORE_GZ\" | cpio -i --no-absolute-filenames \
"

echo "Copying user binaries from '$BINS_DIR' into initrd..."
mkdir -p initrd_unpacked/usr/bin
find "$BINS_DIR" -type f -perm /a+x | while read -r bin; do
  cp "$bin" initrd_unpacked/usr/bin/
  # optional: preserve original filename & make executable
  chmod +x initrd_unpacked/usr/bin/"$(basename "$bin")"
done

echo "Repacking initramfs under fakeroot..."
fakeroot sh -c " \
  cd \"$WORKDIR/initrd_unpacked\" && \
  find . | cpio -o -H newc --owner root:root | gzip -9 > \"$WORKDIR/new_core.gz\" \
"

echo "Replacing old initramfs with new one..."
cp "$WORKDIR/new_core.gz" "$CORE_GZ"

echo "Building new ISO (${OUT_ISO})..."
# Build hybrid BIOS/UEFI ISO (assuming isolinux/xorriso layout)
# Ensure xorriso etc installed in calling environment

ISOHPFX=$(find /usr -name isohdpfx.bin -print -quit || true)

# Determine bootloader path
if [[ -f iso_tree/boot/isolinux/isolinux.bin ]]; then
  BOOT_BIN="boot/isolinux/isolinux.bin"
  BOOT_CAT="boot/isolinux/boot.cat"
elif [[ -f iso_tree/isolinux/isolinux.bin ]]; then
  BOOT_BIN="isolinux/isolinux.bin"
  BOOT_CAT="isolinux/boot.cat"
else
  echo "Error: isolinux.bin not found in expected paths" >&2
  exit 1
fi

XORRISO=(xorriso -as mkisofs)
if [[ -n "$ISOHPFX" ]]; then
  XORRISO+=( -isohybrid-mbr "$ISOHPFX" )
fi
XORRISO+=( -b "$BOOT_BIN" \
            -c "$BOOT_CAT" \
            -no-emul-boot \
            -boot-load-size 4 \
            -boot-info-table \
            -o "$OUT_ISO" iso_tree )

echo "Running: ${XORRISO[*]}"
"${XORRISO[@]}"

echo "Custom ISO created at: $WORKDIR/$OUT_ISO"
# Copy to cwd (in case WORKDIR is /tmp)
cp "$WORKDIR/$OUT_ISO" "$PWD/../" 2>/dev/null || true
