#!/bin/sh

if [ $(id -u) -ne 0 ]; then
	echo "[!] Run as root"
	exit 1
fi

SETCAP_ARGS="cap_net_admin,cap_net_raw+ep"
if [ "$1" = "-r" ]; then
	SETCAP_ARGS="-r"
fi

binaries="./target/debug/wipi-web-api ./target/release/wipi-web-api"
for binary in $binaries; do
	test -f "$binary" && setcap "$SETCAP_ARGS" "$binary"
done

echo "[*] Updated network capabilities for the project binaries"
