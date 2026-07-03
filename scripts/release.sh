#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# release.sh - Create and push version tags for the Aircast QGroundControl fork
# =============================================================================
# The aircast-release workflow (.github/workflows/aircast-release.yml) builds
# and publishes on these tag patterns, so the types below map to those triggers.
# Tags are prefixed `aircast-v` because bare vX.Y.Z is upstream QGC's tag
# namespace and collides on fetch.
#
#   patch    - Bump patch version   (aircast-v5.0.8 -> aircast-v5.0.9)   [production]
#   minor    - Bump minor version   (aircast-v5.0.8 -> aircast-v5.1.0)   [production]
#   major    - Bump major version   (aircast-v5.0.8 -> aircast-v6.0.0)   [production]
#   dev      - Development release  (aircast-v5.0.8 -> aircast-v5.0.9-dev.1)
#   staging  - Staging release      (aircast-v5.0.8 -> aircast-v5.0.8-staging.1)
#
# Usage:
#   ./scripts/release.sh <patch|minor|major|dev|staging>
# =============================================================================

TYPE="${1:?Usage: $0 <patch|minor|major|dev|staging>}"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}▶${NC} $1"; }
log_warn() { echo -e "${YELLOW}▶${NC} $1" >&2; }

get_current_version() {
    git tag -l 'aircast-v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | grep -E '^aircast-v[0-9]+\.[0-9]+\.[0-9]+$' | head -1 || echo "aircast-v0.0.0"
}

tag_exists() {
    git rev-parse "$1" >/dev/null 2>&1
}

find_next_version() {
    local version="$1"
    while tag_exists "$version"; do
        log_warn "Tag $version already exists, trying next..."
        case "$version" in
            *-dev.*)
                num="${version##*-dev.}"
                base="${version%-dev.*}"
                version="${base}-dev.$((num + 1))"
                ;;
            *-staging.*)
                num="${version##*-staging.}"
                base="${version%-staging.*}"
                version="${base}-staging.$((num + 1))"
                ;;
            *)
                IFS='.' read -r major minor patch <<< "${version#aircast-v}"
                version="aircast-v${major}.${minor}.$((patch + 1))"
                ;;
        esac
    done
    echo "$version"
}

parse_version() {
    local version="${1#aircast-v}"
    version="${version%%-*}"
    echo "$version"
}

CURRENT=$(get_current_version)
BASE=$(parse_version "$CURRENT")
IFS='.' read -r MAJOR MINOR PATCH <<< "$BASE"

log_info "Current version: $CURRENT"

case "$TYPE" in
    patch)
        NEW_VERSION="aircast-v${MAJOR}.${MINOR}.$((PATCH + 1))"
        NEW_VERSION=$(find_next_version "$NEW_VERSION")
        MESSAGE="Release $NEW_VERSION"
        ;;
    minor)
        NEW_VERSION="aircast-v${MAJOR}.$((MINOR + 1)).0"
        NEW_VERSION=$(find_next_version "$NEW_VERSION")
        MESSAGE="Release $NEW_VERSION"
        ;;
    major)
        NEW_VERSION="aircast-v$((MAJOR + 1)).0.0"
        NEW_VERSION=$(find_next_version "$NEW_VERSION")
        MESSAGE="Release $NEW_VERSION"
        ;;
    dev)
        NEW_VERSION="aircast-v${MAJOR}.${MINOR}.$((PATCH + 1))-dev.1"
        NEW_VERSION=$(find_next_version "$NEW_VERSION")
        MESSAGE="Development Release $NEW_VERSION"
        ;;
    staging)
        NEW_VERSION="aircast-v${MAJOR}.${MINOR}.${PATCH}-staging.1"
        NEW_VERSION=$(find_next_version "$NEW_VERSION")
        MESSAGE="Staging Release $NEW_VERSION"
        ;;
    *)
        echo "Unknown release type: $TYPE"
        echo "Valid types: patch, minor, major, dev, staging"
        exit 1
        ;;
esac

log_info "New version: $NEW_VERSION"
log_info "Creating tag..."

git tag -a "$NEW_VERSION" -m "$MESSAGE"
git push fork "$NEW_VERSION"

log_info "Released $NEW_VERSION"
