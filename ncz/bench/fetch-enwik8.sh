#!/bin/sh
# Fetches enwik8 (Hutter Prize / LTCB benchmark) and puts its first 10MB in the corpus. Not committed: 10MB.
set -e
cd "$(dirname "$0")"
curl -sSL -o /tmp/enwik8.zip https://mattmahoney.net/dc/enwik8.zip
unzip -q -o /tmp/enwik8.zip -d /tmp
head -c 10000000 /tmp/enwik8 > corpus/text-enwik8-10mb.txt
rm -f /tmp/enwik8.zip /tmp/enwik8
echo "corpus/text-enwik8-10mb.txt ready"
