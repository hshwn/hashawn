#!/bin/sh
# The Kodak Lossless True Color Image Suite: 24 photos, 768x512, PNG. Standard lossless benchmark set.
set -e
mkdir -p "$(dirname "$0")/kodak"
for i in $(seq -w 1 24); do
  f="$(dirname "$0")/kodak/kodim$i.png"
  [ -s "$f" ] || curl -sfL -o "$f" "https://r0k.us/graphics/kodak/kodak/kodim$i.png"
done
ls "$(dirname "$0")/kodak" | wc -l
