#!/usr/bin/env python3
"""ncz benchmark harness. Same corpus, same settings, every build.

usage:  python3 bench.py [label]        # builds nczcli (release), runs every file in bench/corpus/
Writes a table to stdout and appends it to bench/results.md under the label (default: git rev).
Reports bytes, ratio, bits/byte, encode+decode MB/s, verifies the round trip, and shows
gzip -9 and xz for the same bytes so every number has its control.
"""
import gzip, hashlib, lzma, os, subprocess, sys, time

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, "bench", "corpus")
RESULTS = os.path.join(HERE, "bench", "results.md")
CLI = os.path.join(HERE, "target", "release", "nczcli")


def sh(*a):
    return subprocess.run(a, check=True, capture_output=True, text=True).stdout.strip()


def main():
    label = sys.argv[1] if len(sys.argv) > 1 else sh("git", "-C", HERE, "rev-parse", "--short", "HEAD")
    subprocess.run(["cargo", "build", "--release", "--bin", "nczcli", "-q"], cwd=HERE, check=True)
    files = sorted(f for f in os.listdir(CORPUS) if not f.startswith("."))
    rows, tot_in, tot_ncz, tot_gz, tot_xz = [], 0, 0, 0, 0
    tmp = os.path.join(HERE, "target", "bench.tmp")
    for f in files:
        path = os.path.join(CORPUS, f)
        data = open(path, "rb").read()
        h = hashlib.sha256(data).hexdigest()[:8]
        t0 = time.perf_counter(); subprocess.run([CLI, "compress", path, tmp], check=True); t1 = time.perf_counter()
        comp = open(tmp, "rb").read()
        subprocess.run([CLI, "decompress", tmp, tmp + ".out"], check=True); t2 = time.perf_counter()
        ok = open(tmp + ".out", "rb").read() == data
        gz = len(gzip.compress(data, 9)); xz = len(lzma.compress(data, preset=9))
        n = len(data); enc = n / 1e6 / (t1 - t0); dec = n / 1e6 / (t2 - t1)
        rows.append((f, h, n, len(comp), gz, xz, enc, dec, ok))
        tot_in += n; tot_ncz += len(comp); tot_gz += gz; tot_xz += xz
    for p in (tmp, tmp + ".out"):
        try: os.remove(p)
        except OSError: pass

    def bpb(c, n): return 8.0 * c / n if n else 0
    lines = [f"### {label}  ({time.strftime('%Y-%m-%d %H:%M')})", "",
             "| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |",
             "|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|"]
    for f, h, n, c, gz, xz, enc, dec, ok in rows:
        lines.append(f"| {f} | {h} | {n} | {c} | {bpb(c, n):.3f} | {gz} | {xz} | {enc:.2f} | {dec:.2f} | {'✓' if ok else '✗ FAIL'} |")
    lines.append(f"| **total** | | {tot_in} | **{tot_ncz}** | **{bpb(tot_ncz, tot_in):.3f}** | {tot_gz} | {tot_xz} | | | |")
    lines.append(f"\nncz vs xz on the total: {100 * (1 - tot_ncz / tot_xz):+.1f}% smaller\n")
    out = "\n".join(lines)
    print(out)
    os.makedirs(os.path.dirname(RESULTS), exist_ok=True)
    with open(RESULTS, "a") as fh:
        fh.write(out + "\n")
    return 0 if all(r[-1] for r in rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
