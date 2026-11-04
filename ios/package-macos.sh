#!/usr/bin/env bash
# Turns the AircastMac build (Aircast.app, from xcodebuild) into a self-contained .dmg.
#
#   package-macos.sh [--sign IDENTITY] APP DMG    stage a copy of APP, embed GStreamer, sign, (notarize), make DMG
#   package-macos.sh --verify APP                 only check that APP loads nothing from outside itself
#
# The build links GStreamer.framework from /Library/Frameworks (ios/fetch-deps-macos.sh installs it) and libusb and
# libsodium statically from Homebrew. This copies the framework into Contents/Frameworks, keeping only the plugins
# the app uses (build-config.json, as the Qt desktop release did) and the plugin scanner; CoreHost.swift points
# GStreamer at them on start. Everything then has to load from the bundle or the OS, which --verify checks.
#
# With --sign, every Mach-O is signed with IDENTITY for the hardened runtime and the app with
# MacApp/Aircast.entitlements. Without it the bundle is signed ad hoc, which Apple silicon needs to run it at all.
# With --sign and NOTARY_USER, NOTARY_TEAM and NOTARY_PASSWORD in the environment, the app is notarized and stapled
# before it goes into the DMG.
#
# GSTREAMER_FRAMEWORK overrides the framework to embed.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
config="$here/../.github/build-config.json"
entitlements="$here/MacApp/Aircast.entitlements"
gst_source="${GSTREAMER_FRAMEWORK:-/Library/Frameworks/GStreamer.framework}"
required_plugins="coreelements app typefindfunctions playback videoconvertscale udp tcp rtp rtpmanager rtsp"
required_plugins="$required_plugins mpegtsdemux videoparsersbad libav applemedia opengl isomp4 matroska"

die() { echo "package-macos: $*" >&2; exit 1; }

usage() {
    echo "usage: $0 [--sign IDENTITY] APP DMG" >&2
    echo "       $0 --verify APP" >&2
    exit 2
}

retry() {
    local attempt
    for attempt in 1 2 3 4 5 6; do
        "$@" && return 0
        echo "failed (attempt $attempt of 6), retrying in 20 s: $*"
        sleep 20
    done
    return 1
}

# One line per phase with the time, so a stalled step shows where it stopped.
step() {
    echo "package-macos: $(date '+%H:%M:%S') $*"
}

is_macho() {
    [[ $(file -b "$1") == *Mach-O* ]]
}

# Every Mach-O file in the bundle, NUL-separated.
macho_files() {
    local path
    while IFS= read -r -d '' path; do
        is_macho "$path" && printf '%s\0' "$path"
    done < <(find "$1" -type f -print0)
    return 0
}

# Libraries a Mach-O loads (not its own install name), one per line.
load_commands() {
    otool -l "$1" | awk '
        $1 == "cmd" { load = ($2 ~ /^LC_(LOAD_DYLIB|LOAD_WEAK_DYLIB|REEXPORT_DYLIB|LAZY_LOAD_DYLIB|LOAD_UPWARD_DYLIB)$/) }
        load && $1 == "name" { print $2; load = 0 }
    ' | sort -u
}

rpaths() {
    otool -l "$1" | awk '
        $1 == "cmd" { rpath = ($2 == "LC_RPATH") }
        rpath && $1 == "path" { print $2; rpath = 0 }
    ' | sort -u
}

allowed_reference() {
    case $1 in
        @rpath/* | @loader_path/* | @executable_path/* | /System/* | /usr/lib/*) return 0 ;;
        *) return 1 ;;
    esac
}

# Fails when any Mach-O in the bundle loads a library, or searches an rpath, that is not inside the bundle or the OS
# (Homebrew, /usr/local, /Library/Frameworks, a build machine's directories).
verify_bundle() {
    local app="$1" bad=0 count=0 binary dependency path
    while IFS= read -r -d '' binary; do
        count=$((count + 1))
        while IFS= read -r dependency; do
            [ -n "$dependency" ] || continue
            if ! allowed_reference "$dependency"; then
                echo "::error::${binary#"$app"/} loads $dependency" >&2
                bad=1
            fi
        done < <(load_commands "$binary")
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            if ! allowed_reference "$path"; then
                echo "::error::${binary#"$app"/} searches rpath $path" >&2
                bad=1
            fi
        done < <(rpaths "$binary")
    done < <(macho_files "$app")
    [ "$count" -gt 0 ] || die "no Mach-O files found in $app"
    [ "$bad" -eq 0 ] || die "$app loads code from outside the bundle and the OS"
    echo "package-macos: $count Mach-O files in $app load only from the bundle and the OS"
}

# The relative path from directory $1 to path $2, for @loader_path references.
relative_path() {
    python3 -c 'import os, sys; print(os.path.relpath(sys.argv[2], sys.argv[1]))' "$1" "$2"
}

# Copies the framework's 1.0 version into the app, leaving out headers, static and development files, and every
# plugin and helper the app does not use.
embed_gstreamer() {
    local app="$1" source_version destination version plugin name keep wanted missing helper
    source_version="$(cd "$gst_source/Versions/1.0" 2>/dev/null && pwd -P)" ||
        die "no GStreamer framework at $gst_source (run ios/fetch-deps-macos.sh)"
    destination="$app/Contents/Frameworks/GStreamer.framework"
    version="$destination/Versions/1.0"
    case $destination in
        *.app/Contents/Frameworks/GStreamer.framework) ;;
        *) die "refusing to replace unexpected path $destination" ;;
    esac
    rm -rf "$destination"
    mkdir -p "$destination/Versions"
    ditto "$source_version" "$version"
    [ -f "$version/GStreamer" ] || die "$version/GStreamer is missing; the framework layout has changed"

    # The same exclusions as the Qt release's framework install (cmake/GStreamer/Install.cmake).
    rm -rf "${version:?}/bin" "$version/Headers" "$version/Commands" "$version/lib/pkgconfig" \
        "$version/lib/gst-validate-launcher" \
        "$version/share/aclocal" "$version/share/bash-completion" "$version/share/gdb" \
        "$version/share/gst-android" "$version/share/gtk-doc" "$version/share/installed-tests" \
        "$version/share/locale" "$version/share/man"
    find "$version" -depth -type d \( -name include -o -name Headers \) -exec rm -rf {} +
    find "$version" \( -name '*.a' -o -name '*.la' -o -name '*gstpython*' \) -exec rm -rf {} +

    # Only the plugin scanner from libexec.
    helper="$version/libexec/gstreamer-1.0/gst-plugin-scanner"
    [ -x "$helper" ] || die "gst-plugin-scanner is missing from $gst_source"
    find "$version/libexec" -mindepth 1 -maxdepth 1 ! -name gstreamer-1.0 -exec rm -rf {} +
    find "$version/libexec/gstreamer-1.0" -mindepth 1 ! -name gst-plugin-scanner -exec rm -rf {} +

    # Only the plugins build-config.json lists for every platform and for Apple. As in the Qt release, a listed
    # plugin the SDK does not ship is a warning; the ones every video source needs are required.
    wanted="$(python3 -c '
import json, sys
plugins = json.load(open(sys.argv[1]))["gstreamer"]["plugins"]
print("\n".join(plugins["common"] + plugins["apple"]))' "$config")"
    for plugin in "$version"/lib/gstreamer-1.0/*; do
        name="$(basename "$plugin")"
        keep=0
        if [[ $name =~ ^libgst([a-z0-9]+)\.(dylib|so)$ ]] && grep -qx "${BASH_REMATCH[1]}" <<<"$wanted"; then
            keep=1
        fi
        [ "$keep" -eq 1 ] || rm -rf "$plugin"
    done
    missing=""
    while IFS= read -r plugin; do
        case $plugin in
            # 1.22 fused these into videoconvertscale.
            videoconvert | videoscale) continue ;;
        esac
        if [ ! -f "$version/lib/gstreamer-1.0/libgst$plugin.dylib" ] && [ ! -f "$version/lib/gstreamer-1.0/libgst$plugin.so" ]; then
            case " $required_plugins " in
                *" $plugin "*) missing="$missing $plugin" ;;
                *) echo "::warning::GStreamer plugin $plugin is not in $gst_source" ;;
            esac
        fi
    done <<<"$wanted"
    [ -z "$missing" ] || die "plugins missing from $gst_source:$missing"

    ln -sfn 1.0 "$destination/Versions/Current"
    ln -sfn Versions/Current/GStreamer "$destination/GStreamer"
    if [ -d "$version/Resources" ]; then
        ln -sfn Versions/Current/Resources "$destination/Resources"
    fi
}

# The app is arm64 only; the framework is universal. Drop the other slices.
thin_to_arm64() {
    local app="$1" binary archs list
    # Listed up front: the loop writes .thin files beside the binaries, which a scan still running would pick up.
    list="$(mktemp)"
    macho_files "$app" >"$list"
    while IFS= read -r -d '' binary; do
        archs="$(lipo -archs "$binary" 2>/dev/null || true)"
        case " $archs " in
            *" arm64 "*) [ "$archs" = arm64 ] || lipo "$binary" -thin arm64 -output "$binary.thin" ;;
            *) die "${binary#"$app"/} has no arm64 slice (archs: $archs)" ;;
        esac
        if [ -f "$binary.thin" ]; then
            mv "$binary.thin" "$binary"
        fi
    done <"$list"
    rm -f "$list"
}

# Points anything that still names the system framework at the embedded copy, and drops rpaths into anywhere
# outside the bundle. The framework is relocatable (@rpath), so this normally finds nothing to do.
relink() {
    local app="$1" binary dependency dependencies path searched target replacement frameworks
    frameworks="$app/Contents/Frameworks"
    local system_roots=("/Library/Frameworks/GStreamer.framework/Versions/1.0" "/Library/Frameworks/GStreamer.framework/Versions/Current" "/Library/Frameworks/GStreamer.framework")
    local root
    while IFS= read -r -d '' binary; do
        dependencies="$(load_commands "$binary")"
        searched="$(rpaths "$binary")"
        while IFS= read -r dependency; do
            [ -n "$dependency" ] || continue
            for root in "${system_roots[@]}"; do
                case $dependency in
                    "$root"/*)
                        target="$frameworks/GStreamer.framework/Versions/1.0/${dependency#"$root"/}"
                        [ -e "$target" ] || die "${binary#"$app"/} loads $dependency, which is not embedded"
                        install_name_tool -change "$dependency" \
                            "@loader_path/$(relative_path "$(dirname "$binary")" "$target")" "$binary"
                        break
                        ;;
                esac
            done
        done <<<"$dependencies"
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            allowed_reference "$path" && continue
            target=""
            for root in "${system_roots[@]}"; do
                case $path in
                    "$root" | "$root"/*)
                        target="$frameworks/GStreamer.framework/Versions/1.0${path#"$root"}"
                        break
                        ;;
                esac
            done
            if [ -n "$target" ] && [ -e "$target" ]; then
                replacement="@loader_path/$(relative_path "$(dirname "$binary")" "$target")"
                grep -qxF "$replacement" <<<"$(rpaths "$binary")" ||
                    install_name_tool -add_rpath "$replacement" "$binary"
            fi
            # install_name_tool removes one copy at a time.
            while grep -qxF "$path" <<<"$(rpaths "$binary")"; do
                install_name_tool -delete_rpath "$path" "$binary"
            done
        done <<<"$searched"
    done < <(macho_files "$app")
}

# Inside out: loose Mach-O files, then each framework, then the app.
sign_bundle() {
    local app="$1" identity="$2" framework binary
    local flags=(--force --options runtime)
    if [ "$identity" = "-" ]; then
        flags=(--force)
    else
        flags+=(--timestamp)
    fi
    while IFS= read -r -d '' binary; do
        case $binary in
            "$app/Contents/MacOS/"*) ;;
            *) codesign "${flags[@]}" -s "$identity" "$binary" ;;
        esac
    done < <(macho_files "$app")
    for framework in "$app"/Contents/Frameworks/*.framework; do
        [ -d "$framework" ] || continue
        codesign "${flags[@]}" -s "$identity" "$framework"
    done
    codesign "${flags[@]}" --entitlements "$entitlements" -s "$identity" "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
}

notarize() {
    local app="$1" zip submission
    zip="$(dirname "$app")/notarization-upload.zip"
    ditto -c -k --keepParent "$app" "$zip"
    submission="$(xcrun notarytool submit "$zip" \
        --apple-id "$NOTARY_USER" --team-id "$NOTARY_TEAM" --password "$NOTARY_PASSWORD" \
        --wait --output-format json)"
    echo "$submission"
    local id status
    id="$(python3 -c 'import json, sys; print(json.loads(sys.argv[1])["id"])' "$submission")"
    status="$(python3 -c 'import json, sys; print(json.loads(sys.argv[1]).get("status", ""))' "$submission")"
    xcrun notarytool log "$id" \
        --apple-id "$NOTARY_USER" --team-id "$NOTARY_TEAM" --password "$NOTARY_PASSWORD" || true
    [ "$status" = Accepted ] || die "notarization finished with status '$status'"
    rm -f "$zip"
    retry xcrun stapler staple "$app"
    retry xcrun stapler validate "$app"
}

make_dmg() {
    local app="$1" dmg="$2" identity="$3" contents
    contents="$(dirname "$app")/dmg"
    rm -rf "$contents"
    mkdir -p "$contents" "$(dirname "$dmg")"
    ditto "$app" "$contents/$(basename "$app")"
    rm -f "$dmg"
    # The image is made from the app alone, then mounted to add the Applications link: hdiutil -srcfolder can follow
    # a symlink to /Applications and copy everything installed there. hdiutil now and then fails with "Resource
    # busy" on CI machines, hence the retries.
    local writable mount
    writable="$(dirname "$app")/aircast-rw.dmg"
    rm -f "$writable"
    retry hdiutil create -volname Aircast -srcfolder "$contents" -fs HFS+ -format UDRW -ov "$writable"
    mount="$(mktemp -d)"
    retry hdiutil attach "$writable" -nobrowse -noautoopen -mountpoint "$mount"
    ln -s /Applications "$mount/Applications"
    retry hdiutil detach "$mount"
    rmdir "$mount" 2>/dev/null || true
    retry hdiutil convert "$writable" -format UDZO -ov -o "$dmg"
    rm -f "$writable"
    rm -rf "$contents"
    if [ "$identity" != "-" ]; then
        codesign --force --timestamp -s "$identity" "$dmg"
    fi
}

identity="-"
verify_only=0
while [ $# -gt 0 ]; do
    case $1 in
        --sign)
            [ $# -ge 2 ] && [ -n "$2" ] || usage
            identity="$2"
            shift 2
            ;;
        --verify)
            verify_only=1
            shift
            ;;
        -*) usage ;;
        *) break ;;
    esac
done

if [ "$verify_only" -eq 1 ]; then
    [ $# -eq 1 ] || usage
    [ -d "$1/Contents/MacOS" ] || die "$1 is not an app bundle"
    verify_bundle "${1%/}"
    exit 0
fi

[ $# -eq 2 ] || usage
source_app="${1%/}"
dmg="$2"
[ -d "$source_app/Contents/MacOS" ] || die "$source_app is not an app bundle"
case $dmg in
    /*) ;;
    *) dmg="$PWD/$dmg" ;;
esac

staging="$(mktemp -d "${TMPDIR:-/tmp}/aircast-package.XXXXXX")"
trap 'rm -rf "$staging"' EXIT
app="$staging/$(basename "$source_app")"
step "copying $(basename "$source_app")"
ditto "$source_app" "$app"

step "embedding GStreamer"
embed_gstreamer "$app"
step "thinning to arm64"
thin_to_arm64 "$app"
step "relinking"
relink "$app"
step "verifying"
verify_bundle "$app"
step "signing ($identity)"
sign_bundle "$app" "$identity"

if [ "$identity" != "-" ] && [ -n "${NOTARY_USER:-}" ] && [ -n "${NOTARY_TEAM:-}" ] && [ -n "${NOTARY_PASSWORD:-}" ]; then
    notarize "$app"
elif [ "$identity" != "-" ]; then
    echo "package-macos: NOTARY_USER, NOTARY_TEAM or NOTARY_PASSWORD is not set, so the app is not notarized"
fi

step "making the disk image"
make_dmg "$app" "$dmg" "$identity"
step "wrote $dmg"
