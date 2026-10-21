#!/usr/bin/env python3
"""Step 1: split a grainy photo into picture + grain, and check the split is honest.

    uv run --with numpy --with scipy --with scikit-image --with pillow python separate.py <png> [<png> ...]

A good grain/picture split leaves no picture in the grain: no edges, no texture, no outlines. The check:
grain energy next to strong edges vs in flat areas of the same brightness ("edge leak", ideal ~1.0; a denoiser
that smears or cuts edges pushes it up), plus how much noise is left in the clean picture (estimate_sigma,
ideal ~0) and how much grain was removed. Several denoisers are compared; their parameters are printed.
"""
import sys, time
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from skimage.restoration import denoise_nl_means, denoise_wavelet, estimate_sigma, denoise_bilateral

def load(p): return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64) / 255.0

def luma(a): return a @ np.array([0.299, 0.587, 0.114])

def edge_leak(img, grain):
    """Grain energy near strong edges / in flat areas, matched by brightness band."""
    clean = img - grain; y = luma(clean)
    gm = np.hypot(ndi.sobel(y, 0), ndi.sobel(y, 1))
    e = (grain ** 2).sum(-1)
    ratios = []
    for lo in np.arange(0.1, 0.9, 0.1):
        band = (y >= lo) & (y < lo + 0.1)
        if band.sum() < 2000: continue
        g = gm[band]; hi_t, lo_t = np.quantile(g, 0.9), np.quantile(g, 0.5)
        hi_e, flat_e = e[band][g >= hi_t].mean(), e[band][g <= lo_t].mean()
        ratios.append(hi_e / max(flat_e, 1e-12))
    return float(np.median(ratios))

# orthonormal opponent colour transform: brightness, red-blue, green-magenta
OPP = np.array([[1, 1, 1], [1, 0, -1], [1, -2, 1]], dtype=np.float64) / np.array([[np.sqrt(3)], [np.sqrt(2)], [np.sqrt(6)]])

def grain_psd(img, clean_pilot, size=32):
    """Grain power spectrum per opponent channel, from flat patches of (img - pilot), embedded at image size."""
    H, W, _ = img.shape
    res = (img - clean_pilot) @ OPP.T
    y = luma(clean_pilot); gm = np.hypot(ndi.sobel(y, 0), ndi.sobel(y, 1))
    patches = []
    for r in range(0, H - size, size):
        for c in range(0, W - size, size):
            patches.append((gm[r:r + size, c:c + size].mean(), r, c))
    patches.sort(); flat = patches[: max(8, len(patches) * 6 // 100)]    # flattest 6%: busy photos still have texture in their flattest quarter
    win = np.outer(np.hanning(size), np.hanning(size)); win /= np.sqrt((win ** 2).mean())
    stack = []
    for _, r, c in flat:
        blk = res[r:r + size, c:c + size] - res[r:r + size, c:c + size].mean((0, 1))
        stack.append(np.abs(np.fft.fft2(blk * win[..., None], axes=(0, 1))) ** 2 / (size * size))
    psd = np.median(np.stack(stack), axis=0)        # median: a patch with texture in it does not drag the estimate up
    # small PSD -> autocovariance kernel -> image-size PSD
    full = np.zeros((H, W, 3))
    for ch in range(3):
        k = np.fft.fftshift(np.real(np.fft.ifft2(psd[..., ch])))
        big = np.zeros((H, W)); big[:size, :size] = k; big = np.roll(big, (-(size // 2), -(size // 2)), (0, 1))
        full[..., ch] = np.maximum(np.real(np.fft.fft2(big)), 1e-10) * H * W     # bm3d wants PSD x pixel count
    return full, psd

def bm3d_grain(img, pilot):
    import bm3d
    full, _ = grain_psd(img, pilot)
    z = img @ OPP.T
    est = bm3d.bm3d(z, full)
    return est @ np.linalg.inv(OPP).T

def denoisers(img):
    sig = estimate_sigma(img, channel_axis=-1, average_sigmas=True)
    yield f"nl-means h=0.8*sigma", denoise_nl_means(img, h=0.8 * sig, sigma=sig, patch_size=5, patch_distance=6, fast_mode=True, channel_axis=-1)
    yield f"nl-means h=1.2*sigma", denoise_nl_means(img, h=1.2 * sig, sigma=sig, patch_size=5, patch_distance=6, fast_mode=True, channel_axis=-1)
    if False: yield "wavelet BayesShrink", denoise_wavelet(img, method="BayesShrink", mode="soft", rescale_sigma=True, channel_axis=-1, convert2ycbcr=True)
    pilot = denoise_nl_means(img, h=1.2 * sig, sigma=sig, patch_size=5, patch_distance=6, fast_mode=True, channel_axis=-1)
    yield "BM3D, grain spectrum", bm3d_grain(img, pilot)

if __name__ == "__main__":
    for p in sys.argv[1:]:
        img = load(p)
        s0 = estimate_sigma(img, channel_axis=-1, average_sigmas=True) * 255
        print(f"{p.split('/')[-1]}: estimated grain sigma {s0:.2f} levels (of 255)")
        for name, clean in denoisers(img):
            t = time.time()
            grain = img - clean
            left = estimate_sigma(clean, channel_axis=-1, average_sigmas=True) * 255
            print(f"   {name:24s} removed rms {grain.std() * 255:5.2f}  left in picture {left:4.2f}  edge leak {edge_leak(img, grain):4.2f}")
