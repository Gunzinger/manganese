#!/usr/bin/env bash
set -euo pipefail

WORKDIR=$(mktemp -d)
cd "$WORKDIR"

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

echo "Using binaries from: $BINS_DIR"

curl -L -o base.iso "$ISO_URL"

mkdir mnt iso_tree
sudo mount -o loop base.iso mnt
rsync -a mnt/ iso_tree/
sudo umount mnt

# shellcheck disable=SC2046
sudo chown -R $(id -u):$(id -g) iso_tree
BOOT_DIR=iso_tree/boot
CORE_GZ=""
for c in core.gz corepure64.gz rootfs64.gz; do
  if [[ -f "$BOOT_DIR/$c" ]]; then CORE_GZ="$BOOT_DIR/$c"; break; fi
done

if [[ -z "$CORE_GZ" ]]; then
  echo "Cannot find initramfs in $BOOT_DIR"; exit 1
fi

mkdir core_unpacked
pushd core_unpacked >/dev/null
zcat "../$CORE_GZ" | cpio -i --no-absolute-filenames >/dev/null 2>&1 || true
popd >/dev/null

# Copy all executable files from BINS_DIR into initramfs /usr/bin/
mkdir -p core_unpacked/usr/bin
find "$BINS_DIR" -type f ! -name "*.exe" ! -name "*.sha256" ! -name "*.zip" ! -name "*.tar.gz" -perm /a+x | while read -r bin; do
  cp "$bin" core_unpacked/usr/bin/
  chmod +x core_unpacked/usr/bin/"$(basename "$bin")"
done

pushd core_unpacked >/dev/null
find . | cpio -o -H newc --owner root:root | gzip -9 > ../new_core.gz
popd >/dev/null

cp new_core.gz "$BOOT_DIR/$(basename $CORE_GZ)"

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

echo "Built ISO: $OUT_ISO"
