#!/usr/bin/env bash
# Signs the shell's release APKs and puts them where builds are given out.
#
#   VESSEL_APP_LABEL=0.0.0.0.1 \
#   VESSEL_ANDROID_KEYSTORE=<keystore.p12> \
#   VESSEL_ANDROID_KEYSTORE_PASS_FILE=<file holding its password> \
#   release-apk.sh <dist-root>
#
# Run after `cargo tauri android build --apk --split-per-abi` with the same
# label. Each unsigned APK it made is aligned, signed with the one key the
# app is ever signed with (a phone refuses an update signed by another),
# verified, and written as `vessel-<label>-android-shell-<abi>.apk` into
# <dist-root>/<label>-android/, with `SHA256SUMS` beside them. The set is
# made in a hidden directory beside it (a name starting with a dot is not
# served) and renamed into place when complete; an existing set of the
# same label is refused, not replaced.
#
# The keystore and its password live outside the repository and are
# passed by path; nothing here prints them. `apksigner` and `zipalign` come
# from the SDK's newest build-tools.
set -euo pipefail

: "${VESSEL_APP_LABEL:?the build label, e.g. 0.0.0.0.1}"
: "${VESSEL_ANDROID_KEYSTORE:?the signing keystore (PKCS12)}"
: "${VESSEL_ANDROID_KEYSTORE_PASS_FILE:?the file holding its password}"
: "${ANDROID_HOME:?the Android SDK}"
dist_root=${1:?the directory builds are given out from}

here=$(cd "$(dirname "$0")" && pwd)
outputs="$here/../app/gen/android/app/build/outputs/apk"
tools=$(ls -d "$ANDROID_HOME"/build-tools/* | sort -V | tail -1)
out="$dist_root/$VESSEL_APP_LABEL-android"
[ ! -e "$out" ] || { echo "$out exists; a label is given out once" >&2; exit 1; }
staging="$dist_root/.$VESSEL_APP_LABEL-android.partial"
rm -rf "$staging"
mkdir -p "$staging"

found=0
while IFS= read -r unsigned; do
  # app-<abi>-release-unsigned.apk, the ABI as Tauri names it.
  abi=$(basename "$unsigned" | sed -E 's/^app-(.*)-release-unsigned\.apk$/\1/')
  case "$abi" in
    arm64) name=arm64 ;;
    x86_64) name=x86_64 ;;
    universal) name=universal ;;
    *) name=$abi ;;
  esac
  target="$staging/vessel-$VESSEL_APP_LABEL-android-shell-$name.apk"
  aligned=$(mktemp --suffix=.apk)
  "$tools/zipalign" -f -p 4 "$unsigned" "$aligned"
  "$tools/apksigner" sign --ks "$VESSEL_ANDROID_KEYSTORE" --ks-type PKCS12 \
    --ks-pass "file:$VESSEL_ANDROID_KEYSTORE_PASS_FILE" --out "$target" "$aligned"
  rm -f "$aligned" "$target.idsig"
  "$tools/apksigner" verify "$target"
  echo "signed $(basename "$target")"
  found=$((found + 1))
done < <(find "$outputs" -name '*-release-unsigned.apk')

[ "$found" -gt 0 ] || { echo "no unsigned release APK under $outputs" >&2; exit 1; }
(cd "$staging" && sha256sum vessel-*.apk > SHA256SUMS && cat SHA256SUMS)
mv -T "$staging" "$out"
echo "given out in $out"
