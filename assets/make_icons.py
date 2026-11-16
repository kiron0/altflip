import sys
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw, ImageFilter

out = Path(sys.argv[1])
out.mkdir(parents=True, exist_ok=True)


def glyph_boxes(k, ox, oy):
    def box(x, y, w, h):
        return [ox + x * k, oy + (18 - (y + h)) * k, ox + (x + w) * k, oy + (18 - y) * k]

    return {
        "body": box(1.5, -7, 15, 23.5),
        "radius": 7.5 * k,
        "eyes": [box(4.1, 7.4, 4.4, 5.6), box(9.5, 8.0, 4.4, 5.6)],
        "pupils": [box(6.4, 9.0, 1.8, 2.3), box(11.8, 9.6, 1.8, 2.3)],
    }


def menubar_glyph(px):
    S = px * 8
    g = glyph_boxes(S / 18.0, 0, 0)
    a = Image.new("L", (S, S), 0)
    d = ImageDraw.Draw(a)
    d.rounded_rectangle(g["body"], radius=g["radius"], fill=255)
    for e in g["eyes"]:
        d.ellipse(e, fill=0)
    for p in g["pupils"]:
        d.ellipse(p, fill=255)
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    img.putalpha(a)
    return img.resize((px, px), Image.LANCZOS)


def gradient(size, c1, c2, vertical=False):
    w, h = size
    base = Image.new("RGB", size, c1)
    top = Image.new("RGB", size, c2)
    mask = Image.new("L", size)
    md = ImageDraw.Draw(mask)
    steps = h if vertical else w + h
    for i in range(steps):
        v = int(255 * i / max(1, steps - 1))
        if vertical:
            md.line([(0, i), (w, i)], fill=v)
        else:
            md.line([(i, 0), (0, i)], fill=v, width=2)
    return Image.composite(top, base, mask)


def squircle(S, x0, y0, x1, y1, n=5.0, steps=720):
    import math

    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    rx, ry = (x1 - x0) / 2, (y1 - y0) / 2
    pts = []
    for i in range(steps):
        t = 2 * math.pi * i / steps
        c, s_ = math.cos(t), math.sin(t)
        pts.append((cx + rx * math.copysign(abs(c) ** (2 / n), c), cy + ry * math.copysign(abs(s_) ** (2 / n), s_)))
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).polygon(pts, fill=255)
    return m


def app_icon(size=1024):
    S = size * 2
    k0 = S / 1024
    x0, y0, x1, y1 = 100 * k0, 100 * k0, 924 * k0, 924 * k0
    shape = squircle(S, x0, y0, x1, y1)

    bg = gradient((S, S), (122, 130, 255), (110, 63, 216))
    glow = Image.new("L", (S, S), 0)
    ImageDraw.Draw(glow).ellipse([int(S * 0.02), int(S * -0.08), int(S * 0.62), int(S * 0.45)], fill=95)
    glow = glow.filter(ImageFilter.GaussianBlur(S * 0.09))
    bg = Image.composite(Image.new("RGB", (S, S), (196, 200, 255)), bg, glow)
    deep = Image.new("L", (S, S), 0)
    ImageDraw.Draw(deep).ellipse([int(S * 0.55), int(S * 0.6), int(S * 1.15), int(S * 1.2)], fill=110)
    deep = deep.filter(ImageFilter.GaussianBlur(S * 0.1))
    bg = Image.composite(Image.new("RGB", (S, S), (42, 22, 147)), bg, deep)

    k = (x1 - x0) / 25.0
    gw = 18 * k
    ox = (S - gw) / 2
    oy = y0 + (x1 - x0) * 0.2
    g = glyph_boxes(k, ox, oy)
    b = g["body"]

    shadow = Image.new("L", (S, S), 0)
    ImageDraw.Draw(shadow).rounded_rectangle([b[0], b[1] + 16 * k0, b[2], b[3] + 16 * k0], radius=g["radius"], fill=150)
    shadow = ImageChops.multiply(shadow.filter(ImageFilter.GaussianBlur(18 * k0)), shape)
    bg = Image.composite(Image.new("RGB", (S, S), (28, 15, 107)), bg, shadow)

    body_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(body_mask).rounded_rectangle(b, radius=g["radius"], fill=255)
    body_mask = ImageChops.multiply(body_mask, shape)
    body = gradient((S, S), (255, 255, 255), (228, 226, 255), vertical=True)
    img = Image.composite(body, bg, body_mask)

    d = ImageDraw.Draw(img)
    for e in g["eyes"]:
        d.ellipse(e, fill=(27, 24, 64))
    for p in g["pupils"]:
        d.ellipse(p, fill=(255, 255, 255))

    inner = squircle(S, x0 + 3 * k0, y0 + 3 * k0, x1 - 3 * k0, y1 - 3 * k0)
    rim = ImageChops.subtract(shape, inner)
    rim_grad = gradient((S, S), (255, 255, 255), (255, 255, 255), vertical=True)
    rim_alpha = Image.new("L", (S, S), 0)
    ImageDraw.Draw(rim_alpha).rectangle([0, 0, S, S], fill=0)
    for yy in range(S):
        f = yy / S
        a_ = 140 if f < 0.05 else int(140 - (f - 0.05) / 0.35 * 120) if f < 0.4 else int(20 + (f - 0.4) / 0.6 * 26)
        ImageDraw.Draw(rim_alpha).line([(0, yy), (S, yy)], fill=max(0, a_))
    img = Image.composite(rim_grad, img, ImageChops.multiply(rim, rim_alpha))

    plate_shadow = shape.filter(ImageFilter.GaussianBlur(20 * k0))
    plate_shadow = ImageChops.offset(plate_shadow, 0, int(14 * k0)).point(lambda v: int(v * 0.32))
    final = Image.new("RGBA", (S, S), (20, 12, 64, 0))
    final.putalpha(plate_shadow)
    plate = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    plate.paste(img, (0, 0), shape)
    final = Image.alpha_composite(final, plate)
    return final.resize((size, size), Image.LANCZOS)


def tray_color(px):
    S = px * 8
    pad = S * 0.04
    k = (S - 2 * pad) / 18.0
    g = glyph_boxes(k, pad, pad)
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    m = Image.new("L", (S, S), 0)
    ImageDraw.Draw(m).rounded_rectangle([g["body"][0], g["body"][1], g["body"][2], S - pad], radius=g["radius"], fill=255)
    fill = Image.new("RGBA", (S, S), (90, 99, 255, 255))
    img.paste(fill, (0, 0), m)
    d = ImageDraw.Draw(img)
    for e in g["eyes"]:
        d.ellipse(e, fill=(255, 255, 255, 255))
    for p in g["pupils"]:
        d.ellipse(p, fill=(27, 24, 64, 255))
    return img.resize((px, px), Image.LANCZOS)


menubar_glyph(36).save(out / "tray.png")
menubar_glyph(72).save(out / "tray@2x.png")
tray_color(32).save(out / "tray-32.png")
app_icon().save(out / "app-icon-1024.png")
print("ok")
