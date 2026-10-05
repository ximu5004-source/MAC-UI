"""Losslessly redact approved documentation screenshots without publishing originals.

Requires Pillow. Supply private source paths through --source NAME=PATH; the
script stores no input paths in its output and never overwrites an image.
"""

import argparse
import math
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw


def grid(xs, ys, half_width, bottom_offset):
    return [(x - half_width, y, x + half_width, y + bottom_offset)
            for y in ys for x in xs]


# Rectangles use coordinates of the inspected reference size, scaled outward
# when the source has a higher resolution. Include complete wrapped labels.
MASKS = {
    "desktop-overview": ((2048, 1150), [
        (0, 0, 25, 20), (1840, 0, 2048, 24),
        (1694, 89, 1765, 155), (1767, 89, 1841, 155),
        (1694, 210, 1765, 267), (1767, 210, 1841, 267),
        (1694, 278, 1765, 340), (1767, 278, 1841, 340),
        (1694, 346, 1765, 390),
        (1694, 431, 1765, 493), (1767, 431, 1841, 493),
        (1694, 499, 1765, 565), (1767, 499, 1841, 565),
        (1877, 88, 1944, 154), (1947, 88, 2019, 154),
        (1877, 154, 1944, 225), (1947, 154, 2019, 225),
        (1877, 231, 1944, 297), (1947, 231, 2019, 297),
        (1877, 301, 1944, 372), (1947, 301, 2019, 372),
        (1877, 373, 1944, 393), (1947, 373, 2019, 393),
        (1877, 431, 1944, 493), (1947, 431, 2019, 493),
        (1877, 499, 1944, 565),
        (839, 1118, 1211, 1150),
    ] + grid([1724, 1788, 1851, 1914, 1976],
             [620, 688, 757, 826, 895, 964], 32, 63)),
    "desktop-stacks": ((710, 347), [
        (55, 137, 178, 265), (179, 137, 316, 265),
        (416, 137, 530, 264), (531, 137, 664, 264),
        (428, 273, 514, 347), (550, 273, 638, 347),
    ]),
    "launchpad": ((1100, 850), [
        (0, 0, 1100, 11), (0, 11, 10, 850),
        (1090, 11, 1100, 850), (0, 840, 1100, 850),
        (38, 786, 171, 825),
    ] + grid([99, 228, 357, 486, 615, 744, 873, 1002],
             [160, 304, 447, 594], 61, 123)),
    "control-center": ((380, 448), [
        (0, 0, 11, 448), (370, 0, 380, 448), (0, 437, 380, 448),
    ]),
    "power-session": ((563, 532), [
        (22, 22, 74, 74), (79, 22, 209, 46),
        (0, 527, 563, 532), (559, 0, 563, 532),
    ]),
    "desktop-settings": ((1104, 1109), []),
    "dock-settings": ((1104, 1109), []),
}


def redact(name, source, destination):
    if name not in MASKS:
        raise ValueError(f"Unknown screenshot role: {name}")
    if destination.exists():
        raise FileExistsError(f"Refusing to overwrite {destination.name}")
    with Image.open(source) as input_image:
        # Screenshots are opaque. RGB conversion removes private metadata and
        # prevents hidden source pixels in a transparent alpha channel.
        original = input_image.convert("RGB")
    result = original.copy()
    mask = Image.new("L", original.size, 0)
    painter, mask_painter = ImageDraw.Draw(result), ImageDraw.Draw(mask)
    (ref_width, ref_height), regions = MASKS[name]
    sx, sy = original.width / ref_width, original.height / ref_height
    for left, top, right, bottom in regions:
        bounds = (max(0, math.floor(left * sx)),
                  max(0, math.floor(top * sy)),
                  min(original.width - 1, math.ceil(right * sx)),
                  min(original.height - 1, math.ceil(bottom * sy)))
        painter.rectangle(bounds, fill=(105, 105, 105))
        mask_painter.rectangle(bounds, fill=255)
    result.info.clear()
    result.save(destination, format="PNG")
    with Image.open(destination) as saved_image:
        saved = saved_image.convert("RGB")
        difference = ImageChops.difference(original, saved)
        outside_masks = Image.new("RGB", original.size, (0, 0, 0))
        outside_masks.paste(difference, mask=ImageChops.invert(mask))
        if outside_masks.getbbox() is not None:
            raise AssertionError("Pixels outside redaction masks changed")
        if saved.size != original.size or saved_image.info:
            raise AssertionError("Dimensions or metadata verification failed")
    print(f"{destination.name}: {original.width}x{original.height}; "
          f"{len(regions)} masks; unchanged outside masks; no metadata")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", action="append", required=True,
                        metavar="NAME=PATH")
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args()
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for specification in args.source:
        name, source_path = specification.split("=", 1)
        redact(name, Path(source_path), args.output_dir / f"{name}.png")


if __name__ == "__main__":
    main()
