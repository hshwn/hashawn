import os, sys, numpy as np, torch
from PIL import Image
sys.argv = [sys.argv[0], "x"]
import texture2 as t2
T = t2.teacher("teacher/cheng2020-anchor-1-dad2ebff.pth.tar"); S = t2.load_student("student_q1map5.pt", 128).eval()
x = torch.from_numpy(np.asarray(Image.open(os.path.expanduser("~/ncz-img/kodak/kodak_kodim13.ppm")).convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(t2.dev)
with torch.no_grad():
    y = torch.round(T.g_a(x)); m = t2.detail_map(x); g = torch.Generator(device="cpu").manual_seed(1)
    z = torch.randn(1, 9, y.shape[-2] * 16, y.shape[-1] * 16, generator=g).to(t2.dev); xh = S(y, m, z).clamp(0, 1)
np.savez("texparity.npz", y=y.cpu().numpy(), m=m.cpu().numpy(), z=z.cpu().numpy(), x=xh.cpu().numpy(), o=x.cpu().numpy())
print("saved", y.shape, m.shape)
