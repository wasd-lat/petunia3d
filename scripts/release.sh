#!/bin/sh
set -eu

VERSION="${VERSION:-0.5.0}"
OUTPUT="${OUTPUT:-dist}"

mkdir -p "$OUTPUT"

targets="linux/amd64 linux/arm64 darwin/amd64 darwin/arm64 windows/amd64 windows/arm64"

for target in $targets; do
  os="${target%%/*}"
  arch="${target##*/}"
  name="prumo-${os}-${arch}"
  if [ "$os" = "windows" ]; then
    name="${name}.exe"
  fi
  echo "building $name"
  GOOS="$os" GOARCH="$arch" CGO_ENABLED=0 go build -trimpath -ldflags "-s -w" -o "$OUTPUT/$name" ./cmd/prumo
done

(cd "$OUTPUT" && sha256sum prumo-* > checksums.txt)

cat > "$OUTPUT/release.json" <<EOF
{
  "version": "$VERSION",
  "artifacts": [
    "prumo-linux-amd64",
    "prumo-linux-arm64",
    "prumo-darwin-amd64",
    "prumo-darwin-arm64",
    "prumo-windows-amd64.exe",
    "prumo-windows-arm64.exe"
  ]
}
EOF

echo "release artifacts written to $OUTPUT"
