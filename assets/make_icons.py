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


def app_icon(size=1024):
    S = size * 2
    pad = int(S * 0.098)
    r = int(S * 0.185)
    shape = Image.new("L", (S, S), 0)
    ImageDraw.Draw(shape).rounded_rectangle([pad, pad, S - pad, S - pad], r, fill=255)

    bg = gradient((S, S), (122, 128, 255), (84, 52, 214))
    glow = Image.new("L", (S, S), 0)
    ImageDraw.Draw(glow).ellipse([int(S * 0.05), int(S * -0.05), int(S * 0.75), int(S * 0.55)], fill=90)
    glow = glow.filter(ImageFilter.GaussianBlur(S * 0.08))
    bg = Image.composite(Image.new("RGB", (S, S), (190, 196, 255)), bg, glow)

    k = S * 0.032
    gw = 18 * k
    ox = (S - gw) / 2
    oy = S * 0.30 - 1.5 * k
    g = glyph_boxes(k, ox, oy)

    shadow = Image.new("L", (S, S), 0)
    b = g["body"]
    ImageDraw.Draw(shadow).rounded_rectangle([b[0], b[1] + S * 0.03, b[2], b[3] + S * 0.03], radius=g["radius"], fill=150)
    shadow = shadow.filter(ImageFilter.GaussianBlur(S * 0.035))
    bg = Image.composite(Image.new("RGB", (S, S), (30, 18, 90)), bg, shadow)

    body_mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(body_mask).rounded_rectangle(g["body"], radius=g["radius"], fill=255)
    body = gradient((S, S), (255, 255, 255), (226, 225, 242), vertical=True)
    hl = Image.new("L", (S, S), 0)
    ImageDraw.Draw(hl).ellipse([b[0] + S * 0.04, b[1] + S * 0.03, b[0] + S * 0.2, b[1] + S * 0.2], fill=110)
    hl = ImageChops.multiply(hl.filter(ImageFilter.GaussianBlur(S * 0.03)), body_mask)
    body = Image.composite(Image.new("RGB", (S, S), (255, 255, 255)), body, hl)
    img = Image.composite(body, bg, body_mask)

    d = ImageDraw.Draw(img)
    for e in g["eyes"]:
        d.ellipse(e, fill=(27, 24, 64))
    for p in g["pupils"]:
        d.ellipse(p, fill=(255, 255, 255))
    rim = Image.new("L", (S, S), 0)
    ImageDraw.Draw(rim).rounded_rectangle([pad, pad, S - pad, S - pad], r, outline=60, width=int(S * 0.004))
    img = Image.composite(Image.new("RGB", (S, S), (255, 255, 255)), img, rim)

    final = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    final.paste(img, (0, 0), shape)
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
