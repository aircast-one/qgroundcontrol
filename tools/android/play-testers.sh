#!/bin/bash
set -euo pipefail

package=${PLAY_PACKAGE:-one.aircast.app}
track=${PLAY_TESTERS_TRACK:-alpha}
group=${PLAY_TESTERS_GROUP:-aircast-android-testers@googlegroups.com}
service_account=${PLAY_SERVICE_ACCOUNT:-play-publisher@aircast-439915.iam.gserviceaccount.com}
api="https://androidpublisher.googleapis.com/androidpublisher/v3/applications/$package"
token=${PLAY_ACCESS_TOKEN:-$(gcloud auth print-access-token --impersonate-service-account="$service_account" --scopes=https://www.googleapis.com/auth/androidpublisher)}

call() { curl -fsS -H "Authorization: Bearer $token" -H "Content-Type: application/json" "$@"; }

edit=$(call -X POST "$api/edits" | jq -r .id)
tracks=$(call "$api/edits/$edit/tracks" | jq -r '.tracks[].track')
if ! grep -qx "$track" <<<"$tracks"; then
    call -X DELETE "$api/edits/$edit" >/dev/null
    echo "REFUSED: $package has no track \"$track\". Its tracks: $(tr '\n' ' ' <<<"$tracks")" >&2
    echo "         Set PLAY_TESTERS_TRACK to the closed testing track." >&2
    exit 1
fi
groups=$(call "$api/edits/$edit/testers/$track" | jq -c --arg group "$group" '(.googleGroups // []) + [$group] | unique')
call -X PUT -d "{\"googleGroups\":$groups}" "$api/edits/$edit/testers/$track" >/dev/null
call -X POST "$api/edits/$edit:commit" >/dev/null
echo "Google Groups on $package $track: $groups"
