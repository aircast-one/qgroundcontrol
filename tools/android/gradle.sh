#!/bin/bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
properties=$root/android/qgc.properties

if [[ ! -f "$properties" ]]; then
    echo "$properties is missing; run tools/android/studio.py first" >&2
    exit 1
fi

JAVA_HOME=$(sed -n 's/^javaHome=//p' "$properties")
if [[ -z "$JAVA_HOME" ]]; then
    echo "$properties declares no javaHome; regenerate it with tools/android/studio.py" >&2
    exit 1
fi
export JAVA_HOME

cd "$root/android"
exec ./gradlew "$@"
