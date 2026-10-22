#!/usr/bin/env python3
"""Predictor checkpoint (pred.py) -> NCZP file for ncz::wonder::Predictor.   python pred_export.py <pred.pt> <out.pred>"""
import os, struct, sys, torch
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sd = torch.load(sys.argv[1], map_location="cpu"); order = sd["order"].tolist(); C = len(order)
groups = {128: [4, 4, 8, 8, 16, 24, 32, 32], 192: [4, 4, 8, 8, 16, 24, 32, 48, 48], 320: [8, 8, 16, 16, 32, 48, 64, 64, 64]}[C]
cb = "p1.0.red.weight" in sd          # two-pass checkerboard predictor
wd = sd["p1.0.red.weight" if cb else "nets.1.red.weight"].shape[0]
blob = bytearray(b"NCZC" if cb else b"NCZP") + struct.pack("<HHH", C, len(groups), wd) + struct.pack(f"<{len(groups)}H", *groups) + struct.pack(f"<{C}H", *order)
def put(t): blob.extend(t.detach().float().contiguous().numpy().astype("<f4").tobytes())
if cb:
    for pfx in ("p1", "p2"):
        for g in range(len(groups)):
            for k in ("red", "a", "b", "out"): put(sd[f"{pfx}.{g}.{k}.weight"]); put(sd[f"{pfx}.{g}.{k}.bias"])
    open(sys.argv[2], "wb").write(blob); print("pred (checkerboard)", len(blob), "bytes"); sys.exit()
for g in range(len(groups)):
    if g == 0: put(sd["nets.0.bias"].flatten())
    else:
        for k in ("red", "a", "b", "out"): put(sd[f"nets.{g}.{k}.weight"]); put(sd[f"nets.{g}.{k}.bias"])
open(sys.argv[2], "wb").write(blob); print("pred", len(blob), "bytes")
