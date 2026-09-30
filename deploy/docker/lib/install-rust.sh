# shellcheck shell=sh
# Rust toolchain for groundstation, which every variant builds as part of the application
# (groundstation/CMakeLists.txt stops configure when cargo is not on PATH).
#   install-rust.sh <channel> [target...]    install rustup + <channel> (+ targets)
#   install-rust.sh --targets <target...>     add targets to the installed channel
# <channel> comes from groundstation/rust-toolchain.toml so the image holds exactly the
# toolchain a cargo run in /project/source/groundstation selects; the mapped runtime UID
# could not install a missing one. rustup-init is fetched with python3 because curl
# is not in every base image, and RUSTUP_HOME/CARGO_HOME are world-writable so that
# UID can write cargo's registry and build cache.
set -eu

RUST_ROOT=/opt/rust
export RUSTUP_HOME="${RUST_ROOT}/rustup"
export CARGO_HOME="${RUST_ROOT}/cargo"
export PATH="${CARGO_HOME}/bin:${PATH}"

. /usr/local/lib/qgc/retry.sh

if [ "${1:-}" = "--targets" ]; then
    shift
    channel="$(rustup default | cut -d' ' -f1)"
else
    channel="${1:?install-rust.sh needs the toolchain channel from groundstation/rust-toolchain.toml}"
    shift
    host="$(uname -m)-unknown-linux-gnu"
    retry python3 -c "import sys, urllib.request; urllib.request.urlretrieve(sys.argv[1], sys.argv[2])" \
        "https://static.rust-lang.org/rustup/dist/${host}/rustup-init" /tmp/rustup-init
    chmod +x /tmp/rustup-init
    retry /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain "${channel}"
    rm -f /tmp/rustup-init
    {
        echo "export RUSTUP_HOME=${RUSTUP_HOME}"
        echo "export CARGO_HOME=${CARGO_HOME}"
        echo "export PATH=${CARGO_HOME}/bin:\$PATH"
    } > /etc/profile.d/rust.sh
fi

for target in "$@"; do
    retry rustup target add --toolchain "${channel}" "${target}"
done
rustup show active-toolchain
rustup target list --installed --toolchain "${channel}"
chmod -R a+rwX "${RUST_ROOT}"
