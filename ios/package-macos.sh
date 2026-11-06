#!/usr/bin/env bash
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

step() {
    echo "package-macos: $(date '+%H:%M:%S') $*"
}

is_macho() {
    [[ $(file -b "$1") == *Mach-O* ]]
}

macho_files() {
    local path
    while IFS= read -r -d '' path; do
        is_macho "$path" && printf '%s\0' "$path"
    done < <(find "$1" -type f -print0)
    return 0
}

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

verify_bundle() {
    local app="$1" bad=0 count=0 binary dependency path list
    list="$(mktemp)"
    macho_files "$app" >"$list"
    while IFS= read -r -d '' binary; do
        count=$((count + 1))
        [ $((count % 250)) -ne 0 ] || step "checked $count files, now ${binary#"$app"/}"
        while IFS= read -r dependency; do
            [ -n "$dependency" ] || continue
            if ! allowed_reference "$dependency"; then
                echo "::error::${binary#"$app"/} loads $dependency" >&2
                bad=1
            fi
        done <<<"$(load_commands "$binary")"
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            if ! allowed_reference "$path"; then
                echo "::error::${binary#"$app"/} searches rpath $path" >&2
                bad=1
            fi
        done <<<"$(rpaths "$binary")"
    done <"$list"
    rm -f "$list"
    [ "$count" -gt 0 ] || die "no Mach-O files found in $app"
    [ "$bad" -eq 0 ] || die "$app loads code from outside the bundle and the OS"
    echo "package-macos: $count Mach-O files in $app load only from the bundle and the OS"
}

relative_path() {
    python3 -c 'import os, sys; print(os.path.relpath(sys.argv[2], sys.argv[1]))' "$1" "$2"
}

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

    rm -rf "${version:?}/bin" "$version/Headers" "$version/Commands" "$version/lib/pkgconfig" \
        "$version/lib/gst-validate-launcher" \
        "$version/share/aclocal" "$version/share/bash-completion" "$version/share/gdb" \
        "$version/share/gst-android" "$version/share/gtk-doc" "$version/share/installed-tests" \
        "$version/share/locale" "$version/share/man"
    find "$version" -depth -type d \( -name include -o -name Headers \) -exec rm -rf {} +
    find "$version" \( -name '*.a' -o -name '*.la' -o -name '*gstpython*' \) -exec rm -rf {} +

    helper="$version/libexec/gstreamer-1.0/gst-plugin-scanner"
    [ -x "$helper" ] || die "gst-plugin-scanner is missing from $gst_source"
    find "$version/libexec" -mindepth 1 -maxdepth 1 ! -name gstreamer-1.0 -exec rm -rf {} +
    find "$version/libexec/gstreamer-1.0" -mindepth 1 ! -name gst-plugin-scanner -exec rm -rf {} +

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

thin_to_arm64() {
    local app="$1" binary archs list
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

relink() {
    local app="$1" binary dependency dependencies path searched target replacement frameworks
    frameworks="$app/Contents/Frameworks"
    local system_roots=("/Library/Frameworks/GStreamer.framework/Versions/1.0" "/Library/Frameworks/GStreamer.framework/Versions/Current" "/Library/Frameworks/GStreamer.framework")
    local root list
    list="$(mktemp)"
    macho_files "$app" >"$list"
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
            while grep -qxF "$path" <<<"$(rpaths "$binary")"; do
                install_name_tool -delete_rpath "$path" "$binary"
            done
        done <<<"$searched"
    done <"$list"
    rm -f "$list"
}

embed_external_dylibs() {
    local app="$1" frameworks binary dependency source name target changed list searched path root
    frameworks="$app/Contents/Frameworks"
    mkdir -p "$frameworks"
    list="$(mktemp)"
    changed=1
    while [ "$changed" -eq 1 ]; do
        changed=0
        macho_files "$app" >"$list"
        while IFS= read -r -d '' binary; do
            searched="$(rpaths "$binary" | grep -E '^(/opt/homebrew|/usr/local)/' || true)"
            while IFS= read -r dependency; do
                [ -n "$dependency" ] || continue
                source=""
                case $dependency in
                    /opt/homebrew/* | /usr/local/*) source="$dependency" ;;
                    @rpath/*)
                        while IFS= read -r root; do
                            [ -n "$root" ] || continue
                            if [ -f "$root/${dependency#@rpath/}" ]; then
                                source="$root/${dependency#@rpath/}"
                                break
                            fi
                        done <<<"$searched"
                        ;;
                esac
                [ -n "$source" ] || continue
                [ -f "$source" ] || die "${binary#"$app"/} loads $source, which this machine does not have"
                name="$(basename "$source")"
                target="$frameworks/$name"
                if [ ! -f "$target" ]; then
                    step "embedding $source"
                    cp -L "$source" "$target"
                    chmod u+w "$target"
                    install_name_tool -id "@rpath/$name" "$target"
                    changed=1
                fi
                install_name_tool -change "$dependency" \
                    "@loader_path/$(relative_path "$(dirname "$binary")" "$target")" "$binary"
            done <<<"$(load_commands "$binary")"
            while IFS= read -r path; do
                [ -n "$path" ] || continue
                while grep -qxF "$path" <<<"$(rpaths "$binary")"; do
                    install_name_tool -delete_rpath "$path" "$binary"
                done
            done <<<"$searched"
        done <"$list"
    done
    rm -f "$list"
}

sign_bundle() {
    local app="$1" identity="$2" framework binary list
    local flags=(--force --options runtime)
    if [ "$identity" = "-" ]; then
        flags=(--force)
    else
        flags+=(--timestamp)
    fi
    list="$(mktemp)"
    macho_files "$app" >"$list"
    while IFS= read -r -d '' binary; do
        case $binary in
            "$app/Contents/MacOS/"*) ;;
            *) retry codesign "${flags[@]}" -s "$identity" "$binary" ;;
        esac
    done <"$list"
    rm -f "$list"
    for framework in "$app"/Contents/Frameworks/*.framework; do
        [ -d "$framework" ] || continue
        retry codesign "${flags[@]}" -s "$identity" "$framework"
    done
    retry codesign "${flags[@]}" --entitlements "$entitlements" -s "$identity" "$app"
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
embed_external_dylibs "$app"
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
