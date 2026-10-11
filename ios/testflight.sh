#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

: "${ASC_KEY_ID:?App Store Connect API key ID}"
: "${ASC_ISSUER_ID:?App Store Connect API issuer ID}"
: "${ASC_KEY_PATH:?path to the AuthKey_XXXX.p8 file}"

AUTH=(-allowProvisioningUpdates -authenticationKeyPath "$ASC_KEY_PATH" -authenticationKeyID "$ASC_KEY_ID" -authenticationKeyIssuerID "$ASC_ISSUER_ID")
OUT=../build-ios/testflight
ARCHIVE=$OUT/Aircast.xcarchive

./fetch-deps.sh
xcodegen generate >/dev/null
rm -rf "$OUT"
mkdir -p "$OUT"
xcodebuild -project Aircast.xcodeproj -scheme Aircast -configuration Release -destination 'generic/platform=iOS' \
  -archivePath "$ARCHIVE" "${AUTH[@]}" archive > "$OUT/archive.log" 2>&1 || { grep -E "error:" "$OUT/archive.log"; echo "archive failed, see build-ios/testflight/archive.log"; exit 1; }
env PATH=/usr/bin:/bin:/usr/sbin:/sbin xcodebuild -exportArchive -archivePath "$ARCHIVE" -exportOptionsPlist ExportOptions.plist -exportPath "$OUT/export" "${AUTH[@]}" \
  > "$OUT/export.log" 2>&1 || { grep -E "error:|Error" "$OUT/export.log"; echo "upload failed, see build-ios/testflight/export.log"; exit 1; }
echo "Uploaded. App Store Connect processes it in 5-15 minutes, then it appears in TestFlight."
