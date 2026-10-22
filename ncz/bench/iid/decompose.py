#!/usr/bin/env python3
"""Intrinsic split of each photo with Marigold-IID Lighting v1.1 (albedo, diffuse shading, non-diffuse residual).

    python decompose.py <out_dir> <a.ppm> [...]          # on the rig's 3090; writes <name>.npz (float32, HxWx3 each)

Encoder-side only: the decoder never runs this model, it just gets the layers (or what is derived from them).
"""
import os, sys, json, time
import numpy as np, torch
from PIL import Image
from diffusers import MarigoldIntrinsicsPipeline

out = sys.argv[1]; os.makedirs(out, exist_ok=True)
pipe = MarigoldIntrinsicsPipeline.from_pretrained("prs-eth/marigold-iid-lighting-v1-1", variant="fp16", torch_dtype=torch.float16).to("cuda")
print("targets", pipe.target_properties)
for p in sys.argv[2:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = Image.open(p).convert("RGB"); t = time.time()
    r = pipe(img, ensemble_size=5, processing_resolution=0, generator=torch.Generator("cuda").manual_seed(0))
    pred = np.asarray(r.prediction, dtype=np.float32)            # (targets, H, W, 3)
    np.savez_compressed(f"{out}/{name}.npz", **{k: pred[i] for i, k in enumerate(pipe.target_properties["target_names"])})
    print(name, pred.shape, [f"{k} {pred[i].min():.3f}..{pred[i].max():.3f}" for i, k in enumerate(pipe.target_properties["target_names"])], f"{time.time() - t:.1f}s", flush=True)
json.dump({k: v for k, v in pipe.target_properties.items() if isinstance(v, (str, list, dict, int, float))}, open(f"{out}/targets.json", "w"), indent=1, default=str)
