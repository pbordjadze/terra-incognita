"""Draws the globe's Persian ornaments (Globe/) entirely from procedural geometry.

    python3 tools/globe/ornament.py
    python3 tools/globe/ornament.py --sheet tools/globe/sheets/ornament-sheet.png

  band.png     1536 x 256, opaque RGBA, one of sixteen repeats around the globe. x wraps;
               y runs from the band's inner edge to its outer edge. A 288 px salmon cell
               straddles x = 0, with a white-edged black quatrefoil and a gold line star.
               The remaining lapis panel has three-lobed ends and a 6 px white ruling.
               A 7 px sine stem makes four alternating bays, with tapering spiral tendrils,
               split leaves and rose, white, orange and pale-blue five-petalled blossoms.
  fleuron.png  256 x 256, transparent RGBA, pointing up. A washed trefoil crowns a blue-ink
               stem and two hooked tendrils. Its alpha stays inside (56, 16)..(200, 244);
               the review places its bottom-centre alpha edge, not its padding, on the ring.
  rosette.png  96 x 96, transparent RGBA, an 80 px eight-petalled capital marker: white
               petals, 3 px gold outlines and a 16 px gold centre.

The sRGB palette studies the illuminated opening of LACMA's Khamsa of Nizami (M.85.237.16).
No source pictures, fonts or lettering are used. Geometry is rasterised at 4x and box-filtered
in premultiplied working colour, then stored with straight alpha and the specified sRGB bytes.
The optional 1600 x 1728 paper sheet fits the complete 1536 px tile at 1:1 below a bilinearly
sampled polar wrap (inner radius 504, thickness 0.0685 radius, sixteen repeats), gold rings and
48 outward fleurons. Below the tile, left to right, are the fleuron at 256 and 25 px and the
rosette at 96 and 30 px. The sheet contains no text and needs no globe data or network access.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, PngImagePlugin
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Globe'
SCALE = 4
LAPIS = '#4E5B9B'
GOLD = '#A8894F'
GOLD_LIGHT = '#C7A964'
SALMON = '#C7735A'
BLACK = '#2A2119'
WHITE = '#EFE7DA'
ROSE = '#D78E8A'
ORANGE = '#D9834E'
PINK = '#E3B2A4'
PALE_BLUE = '#A9BCE0'
INK = '#4F5D9A'
WASH = '#A9B5D8'
PAPER = '#F3E9D2'


def rgb(colour):
    return tuple(int(colour[index:index + 2], 16) for index in (1, 3, 5))


def box_rgba(image):
    """A 4x box filter with explicit unpremultiplication, including translucent edge pixels."""
    values = np.asarray(image, np.float32)
    height, width = values.shape[:2]
    alpha = values[..., 3:4] / 255
    premultiplied = values[..., :3] * alpha
    alpha = alpha.reshape(height // SCALE, SCALE, width // SCALE, SCALE, 1).mean((1, 3))
    colour = premultiplied.reshape(height // SCALE, SCALE, width // SCALE, SCALE, 3).mean((1, 3))
    colour = np.divide(colour, alpha, out=np.zeros_like(colour), where=alpha > 0)
    return Image.fromarray(np.rint(np.concatenate((colour, alpha * 255), axis=2)).astype(np.uint8))


class Drawing:
    def __init__(self, size, ground=None):
        self.image = Image.new('RGBA', tuple(dimension * SCALE for dimension in size),
                               (*rgb(ground), 255) if ground else (0, 0, 0, 0))
        self.draw = ImageDraw.Draw(self.image)
        self.coverage = Image.new('L', self.image.size)
        self.mask_draw = ImageDraw.Draw(self.coverage)

    def polygon(self, points, colour):
        vertices = [(float(point[0]) * SCALE - 0.5, float(point[1]) * SCALE - 0.5)
                    for point in points]
        self.draw.polygon(vertices, fill=(*rgb(colour), 255))
        self.mask_draw.polygon(vertices, fill=255)

    def dot(self, centre, radius, colour):
        bounds = tuple(value * SCALE - 0.5 for value in
                       (centre[0] - radius, centre[1] - radius,
                        centre[0] + radius, centre[1] + radius))
        self.draw.ellipse(bounds, fill=(*rgb(colour), 255))
        self.mask_draw.ellipse(bounds, fill=255)

    def outlined_polygon(self, points, fill, outline, width):
        mask = Image.new('L', self.image.size)
        vertices = [(float(point[0]) * SCALE - 0.5, float(point[1]) * SCALE - 0.5)
                    for point in points]
        ImageDraw.Draw(mask).polygon(vertices, fill=255)
        inside = np.asarray(mask) > 0
        outer = ndimage.distance_transform_edt(~inside) <= width * SCALE / 2
        inner = ndimage.distance_transform_edt(inside) > width * SCALE / 2
        outer_mask = Image.fromarray(outer.astype('uint8') * 255)
        self.image.paste((*rgb(outline), 255), mask=outer_mask)
        self.image.paste((*rgb(fill), 255), mask=Image.fromarray(inner.astype('uint8') * 255))
        self.coverage.paste(255, mask=outer_mask)

    def stroke(self, points, width, colour, end_width=None):
        points = np.asarray(points, float)
        tangent = np.gradient(points, axis=0)
        normal = np.column_stack((-tangent[:, 1], tangent[:, 0]))
        normal /= np.maximum(np.linalg.norm(normal, axis=1, keepdims=True), 1e-9)
        widths = np.linspace(width, width if end_width is None else end_width, len(points))
        offset = normal * widths[:, None] / 2
        self.polygon(np.concatenate((points + offset, (points - offset)[::-1])), colour)
        segments = np.diff(points, axis=0)
        lengths = np.linalg.norm(segments, axis=1)
        turns = np.sum(segments[:-1] * segments[1:], axis=1) / np.maximum(lengths[:-1] * lengths[1:], 1e-9)
        for point, local_width in zip(points[1:-1][turns < 0.5], widths[1:-1][turns < 0.5]):
            self.dot(point, local_width / 2, colour)
        self.dot(points[0], widths[0] / 2, colour)
        self.dot(points[-1], widths[-1] / 2, colour)


def bezier(start, *segments):
    points = [start]
    amount = np.linspace(0, 1, 49)[1:, None]
    for control_one, control_two, end in segments:
        curve = ((1 - amount) ** 3 * np.asarray(start)
                 + 3 * (1 - amount) ** 2 * amount * np.asarray(control_one)
                 + 3 * (1 - amount) * amount ** 2 * np.asarray(control_two)
                 + amount ** 3 * np.asarray(end))
        points.extend(curve)
        start = end
    return np.asarray(points)


def transform(points, origin, angle):
    cosine, sine = np.cos(angle), np.sin(angle)
    return np.asarray(points) @ np.array([[cosine, sine], [-sine, cosine]]) + origin


def teardrop():
    return bezier((-9, 0), ((-2, -7), (9, -5), (9, 0)),
                  ((9, 5), (-2, 7), (-9, 0)))


def split_leaf():
    return bezier((0, 0), ((7, -6), (19, -7), (26, -5)),
                  ((22, -5), (22, -1), (17, 0)),
                  ((22, 0), (24, 3), (26, 5)),
                  ((17, 7), (6, 6), (0, 0)))


def blossom(drawing, centre, colour):
    for angle in np.linspace(-np.pi / 2, 3 * np.pi / 2, 5, endpoint=False):
        petal = np.asarray(centre) + 8.5 * np.array([np.cos(angle), np.sin(angle)])
        drawing.dot(petal, 6.5, colour)
    drawing.dot(centre, 3.5, GOLD_LIGHT)


def band():
    width, height = 1536, 256
    drawing = Drawing((width, height), SALMON)
    row, column = np.ogrid[:height * SCALE, :width * SCALE]
    horizontal = ((column + 0.5) / SCALE + width / 2) % width - width / 2
    vertical = (row + 0.5) / SCALE - height / 2
    distance = np.full(drawing.coverage.size[::-1], np.inf, np.float32)
    for offset_x, offset_y in ((0.17, 0), (-0.17, 0), (0, 0.17), (0, -0.17)):
        distance = np.minimum(distance, np.hypot(horizontal - offset_x * height,
                                                  vertical - offset_y * height) - 0.22 * height)
    drawing.image.paste((*rgb(WHITE), 255), mask=Image.fromarray((distance <= 2.5).astype('uint8') * 255))
    drawing.image.paste((*rgb(BLACK), 255), mask=Image.fromarray((distance <= -2.5).astype('uint8') * 255))
    star = np.array([(0, -62), (12, -12), (62, 0), (12, 12), (0, 62),
                     (-12, 12), (-62, 0), (-12, -12), (0, -62)])
    drawing.stroke(star + (0, height / 2), 3, GOLD)
    drawing.polygon(np.array([(0, -10), (6, 0), (0, 10), (-6, 0)]) + (0, height / 2), GOLD)
    for centre_y in (18, 238):
        drawing.polygon(transform(teardrop(), (113, centre_y),
                                  (1 if centre_y == 18 else -1) * np.pi / 6), GOLD)
    half_cell = drawing.image.crop((0, 0, 144 * SCALE, height * SCALE))
    drawing.image.paste(half_cell.transpose(Image.Transpose.FLIP_LEFT_RIGHT),
                        ((width - 144) * SCALE, 0))

    edge_y = np.linspace(0, height, height * SCALE + 1)
    edge_x = np.full_like(edge_y, 144)
    for low, high, depth in ((0, 84, 18), (84, 172, 28), (172, 256, 18)):
        inside = (edge_y >= low) & (edge_y <= high)
        local = (edge_y[inside] - (low + high) / 2) / ((high - low) / 2)
        edge_x[inside] -= depth * np.sqrt(np.maximum(0, 1 - local ** 2))
    panel = Image.new('L', drawing.image.size)
    perimeter = np.concatenate((np.column_stack((edge_x, edge_y)),
                                np.column_stack((width - edge_x, edge_y))[::-1]))
    ImageDraw.Draw(panel).polygon([(float(point[0]) * SCALE - 0.5,
                                    float(point[1]) * SCALE - 0.5) for point in perimeter], fill=255)
    edge_distance = ndimage.distance_transform_edt(np.pad(np.asarray(panel) > 0, 1))[1:-1, 1:-1] / SCALE
    interior = edge_distance > 6
    drawing.image.paste((*rgb(WHITE), 255), mask=panel)
    drawing.image.paste((*rgb(LAPIS), 255), mask=Image.fromarray(interior.astype('uint8') * 255))
    drawing.coverage.paste(0, (0, 0, *drawing.coverage.size))

    start_x, end_x = 194, width - 194
    position = np.linspace(0, 1, 1601)
    stem = np.column_stack((start_x + (end_x - start_x) * position,
                            height / 2 - 0.22 * height * np.sin(4 * np.pi * position)))
    drawing.stroke(stem, 7, GOLD)
    for bay, colour in enumerate((ROSE, WHITE, ORANGE, PALE_BLUE)):
        fraction = (bay + 0.5) / 4
        centre_x = start_x + (end_x - start_x) * fraction
        inward = 1 if bay % 2 == 0 else -1
        root_y = height / 2 - inward * 0.22 * height
        centre_y = root_y + inward * 59
        amount = np.linspace(0, 1, 501)
        tightening = amount - np.sin(2 * np.pi * amount) / (2 * np.pi)
        radius = 59 * np.exp(-1.7 * tightening)
        angle = -np.pi / 2 + amount * 2.5 * np.pi
        tendril = np.column_stack((centre_x + radius * np.cos(angle),
                                  centre_y + inward * radius * np.sin(angle)))
        drawing.stroke(tendril, 5, GOLD, end_width=3)
        for leaf_angle in (-np.pi / 6, -5 * np.pi / 6):
            leaf = transform(split_leaf(), (0, 0), leaf_angle)
            leaf[:, 1] *= inward
            drawing.polygon(leaf + (centre_x, root_y), GOLD)
        blossom(drawing, tendril[-1], colour)
        for offset_x, dot_y in ((-75, 39), (75, 217)):
            drawing.dot((centre_x + offset_x, dot_y), 2, GOLD)

    decoration = np.asarray(drawing.coverage) > 0
    coverage = np.count_nonzero(decoration) / np.count_nonzero(interior)
    clearance = float(edge_distance[decoration].min() - 6)
    if coverage >= 0.35 or clearance < 10:
        raise ValueError(f'panel decoration: {coverage:.2%} coverage, {clearance:.2f} px clearance')
    image = box_rgba(drawing.image)
    pixels = np.asarray(image)
    if not np.array_equal(pixels[:, 0], pixels[:, -1]) or not np.all(pixels[..., 3] == 255):
        raise ValueError('band does not join seamlessly')
    print(f'band: gold + blossoms {coverage:.2%} of lapis panel; outline clearance {clearance:.2f} px')
    print('band: opposite edge columns identical; alpha 255 throughout')
    return image


def fleuron():
    drawing = Drawing((256, 256))
    drawing.stroke([(128, 240), (128, 110)], 5, INK)
    bud = bezier((128, 110), ((106, 103), (95, 80), (97, 60)),
                 ((97, 49), (103, 44), (110, 50)),
                 ((113, 53), (114, 58), (114, 62)),
                 ((112, 48), (117, 35), (128, 34)),
                 ((139, 35), (144, 48), (142, 62)),
                 ((142, 58), (143, 53), (146, 50)),
                 ((153, 44), (159, 49), (159, 60)),
                 ((161, 80), (150, 103), (128, 110)))
    drawing.outlined_polygon(bud, WASH, INK, 7)
    tendril = bezier((128, 170), ((108, 151), (82, 150), (69, 165)),
                     ((58, 178), (60, 199), (74, 199)),
                     ((85, 199), (88, 187), (80, 184)),
                     ((76, 182), (72, 185), (74, 189)))
    drawing.stroke(tendril, 3.5, INK, end_width=2.5)
    drawing.stroke(tendril * (-1, 1) + (256, 0), 3.5, INK, end_width=2.5)
    image = box_rgba(drawing.image)
    bounds = image.getchannel('A').getbbox()
    if not (bounds and bounds[0] >= 56 and bounds[1] >= 16 and bounds[2] <= 200 and bounds[3] <= 244):
        raise ValueError(f'fleuron alpha bbox outside its frame: {bounds}')
    print(f'fleuron: alpha bbox {bounds}, within (56, 16, 200, 244)')
    return image


def rosette():
    drawing = Drawing((96, 96))
    petal = bezier((7, 0), ((14, -5), (21, -9), (29, -7)),
                   ((41.5, -5), (41.5, 5), (29, 7)),
                   ((21, 9), (14, 5), (7, 0)))
    for angle in np.linspace(0, 2 * np.pi, 8, endpoint=False):
        points = transform(petal, (48, 48), angle)
        drawing.polygon(points, WHITE)
        drawing.stroke(points, 3, GOLD)
    drawing.dot((48, 48), 8, GOLD)
    return box_rgba(drawing.image)


def sample(image, horizontal, vertical):
    """Bilinear texture lookup: periodic x, clamped y, texel centres at half-integers."""
    height, width = image.shape[:2]
    column, row = horizontal * width - 0.5, vertical * height - 0.5
    column_low, row_low = np.floor(column).astype(int), np.floor(row).astype(int)
    blend_x, blend_y = (column - column_low)[..., None], (row - row_low)[..., None]
    top = (image[np.clip(row_low, 0, height - 1), column_low % width] * (1 - blend_x)
           + image[np.clip(row_low, 0, height - 1), (column_low + 1) % width] * blend_x)
    bottom = (image[np.clip(row_low + 1, 0, height - 1), column_low % width] * (1 - blend_x)
              + image[np.clip(row_low + 1, 0, height - 1), (column_low + 1) % width] * blend_x)
    return top * (1 - blend_y) + bottom * blend_y


def place_fleuron(sheet, image, base, angle, height):
    """Warp at 4x about the actual alpha base so no transparent padding opens a ring gap."""
    values = np.asarray(image, np.float32) / 255
    values[..., :3] *= values[..., 3:4]
    bounds = image.getchannel('A').getbbox()
    factor = height / (bounds[3] - bounds[1])
    extent = int(np.ceil(height)) + 3
    left, top = int(np.floor(base[0])) - extent, int(np.floor(base[1])) - extent
    size = 2 * extent
    row, column = np.mgrid[:size * SCALE, :size * SCALE]
    horizontal = left + (column + 0.5) / SCALE - base[0]
    vertical = top + (row + 0.5) / SCALE - base[1]
    source_x = (horizontal * np.cos(angle) + vertical * np.sin(angle)) / factor + 128 - 0.5
    source_y = (-horizontal * np.sin(angle) + vertical * np.cos(angle)) / factor + bounds[3] - 0.5
    channels = [ndimage.map_coordinates(values[..., channel], [source_y, source_x],
                                        order=1, mode='constant', cval=0, prefilter=False)
                for channel in range(4)]
    filtered = np.stack(channels, axis=2).reshape(size, SCALE, size, SCALE, 4).mean((1, 3))
    alpha = filtered[..., 3:4]
    filtered[..., :3] = np.divide(filtered[..., :3], alpha,
                                 out=np.zeros_like(filtered[..., :3]), where=alpha > 0)
    stamp = Image.fromarray(np.rint(np.clip(filtered, 0, 1) * 255).astype(np.uint8))
    sheet.alpha_composite(stamp, (left, top))


def review_sheet(tile, finial, marker):
    sheet = Image.new('RGBA', (1600, 1728), (*rgb(PAPER), 255))
    centre_x, centre_y, inner = 800, 580, 504
    outer = inner * (1 + 0.0685)
    inner_ring, outer_ring = inner - 2.5, outer + 2.5
    texture = np.asarray(tile, np.float32)[..., :3]
    left, top, size = 250, 30, 1100
    column = (np.arange(size * SCALE, dtype=np.float32) + 0.5) / SCALE + left - centre_x
    for first_row in range(0, size, 20):
        rows = min(20, size - first_row)
        row = ((np.arange(rows * SCALE, dtype=np.float32) + 0.5) / SCALE
               + top + first_row - centre_y)[:, None]
        radius = np.hypot(column, row)
        angle = np.arctan2(column, -row)
        colour = np.empty((*radius.shape, 3), np.float32)
        colour[:] = rgb(PAPER)
        annulus = (radius >= inner) & (radius < outer)
        colour[annulus] = sample(texture, (angle[annulus] / (2 * np.pi) * 16) % 1,
                                 (radius[annulus] - inner) / (outer - inner))
        rings = (np.abs(radius - inner_ring) <= 1.5) | (np.abs(radius - outer_ring) <= 1.5)
        colour[rings] = rgb(GOLD)
        filtered = colour.reshape(rows, SCALE, size, SCALE, 3).mean((1, 3))
        sheet.paste(Image.fromarray(np.rint(filtered).astype(np.uint8)), (left, top + first_row))
    for angle in np.linspace(0, 2 * np.pi, 48, endpoint=False):
        base = (centre_x + outer_ring * np.sin(angle), centre_y - outer_ring * np.cos(angle))
        place_fleuron(sheet, finial, base, angle, inner * 0.05)
    sheet.alpha_composite(tile, (32, 1170))
    sheet.alpha_composite(finial, (160, 1450))
    place_fleuron(sheet, finial, (520, 1616), 0, 25)
    sheet.alpha_composite(marker, (744, 1530))
    sheet.alpha_composite(marker.resize((30, 30), Image.Resampling.BOX), (1017, 1563))
    return sheet


def save(image, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    metadata = PngImagePlugin.PngInfo()
    metadata.add(b'sRGB', b'\x00')
    image.save(path, pnginfo=metadata, optimize=True)
    print(f'wrote {path.relative_to(ROOT) if path.is_relative_to(ROOT) else path} '
          f'({image.width} x {image.height}, RGBA)')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sheet', type=Path, help='also write a paper review sheet to this path')
    args = parser.parse_args()
    tile, finial, marker = band(), fleuron(), rosette()
    for image, name in ((tile, 'band.png'), (finial, 'fleuron.png'), (marker, 'rosette.png')):
        save(image, OUT / name)
    if args.sheet:
        save(review_sheet(tile, finial, marker), args.sheet)


if __name__ == '__main__':
    main()
