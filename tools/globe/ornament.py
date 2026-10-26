"""Draw the globe's Persian ornaments (Globe/) with offline Python geometry.

    python3 tools/globe/ornament.py
    python3 tools/globe/ornament.py --sheet /tmp/ornament-sheet-r5.png

The default run writes all five assets; --sheet also regenerates the review sheet.
Only numpy, Pillow, scipy and the Python standard library are used, entirely offline.
The ornament source is the illuminated opening and blue-ink margin of LACMA's Khamsa of
Nizami, M.85.237.16. The palette and revised proportions below follow supplied scan
measurements; the lighter scroll, cusps, rulings and headpiece composition retain the
round 3 manuscript study. No fonts, lettering or source-image pixels are used.

The only paint colours are these sRGB scan samples, white-balanced so the manuscript's
lead white reads neutral: LAPIS 5F6CA7, GOLD A08C60 (all ornament and ruling gold),
CARTOUCHE 9D8A57 (cartouche and pendant grounds), SALMON C47C66, PINK D1A38F (pink paint
and outlines), BLACK 251F14, WHITE F5F5F5, LIGHT_BLUE A2B7D0, GREEN 79A18B, INK 6D6793
(margin ink), PAPER F2E4C8. Blossoms use only WHITE, PINK, SALMON and LIGHT_BLUE, with
GOLD hearts. Antialiasing and resampling blend these paints; no colour noise is added.

  band.png     1152 x 256, opaque RGBA, seamless in x; y runs from inner to outer edge.
               The salmon cell is centred on x = 0, nominal width 307 px (1.2 H), split
               across the seam. The lapis panel's cusp anchors are x = 154 and 998
               (3.3 H apart); its three end lobes extend 18/28/18 px into the salmon.
               The black quatrefoil retains four radius-0.2 H lobes, a 5 px WHITE
               outline, 2 px GOLD inner rim and 4 px GOLD star. Corner half-leaves keep
               their sizes and distance from the cell edges. The panel has a 6 px WHITE
               outline, 3 px lapis gap and 2 px GOLD inner ruling. Two 7 px GOLD sines,
               amplitude 0.24 H, form three compartments; their two crossings alternate
               over/under with 2 px lapis gaps. The round 3 scroll scales horizontally
               to this panel: 1.5-turn volutes taper 6 to 3 px, with 26 px blossoms;
               compartment khatai are 40 px; outward curls end in 14 px pink/white buds.
               Blossom paint positions retain round 3's order, mapped to this palette.
               Rounded 28 x 16 px split palmettes occur only at crests and crossings.
  braid.png    1024 x 256, opaque RGBA, seamless in x. The manuscript frame's braid,
               between its light-blue ruling and green band: GOLD ground, two BLACK
               triangle waves in antiphase, four periods per tile, 30 px strokes.
               Their extreme centre-lines are 15 px from the edges (half a stroke).
               At all eight mid-height crossings the top strand alternates, with a
               42 px GOLD knockout behind its 30 px stroke: 6 px clear on either side.
               The interstitial triangles remain GOLD; no other paint is used.
  fleuron.png  256 x 256, transparent RGBA. The margin's unfilled INK lily, about
               100 x 200 px, base at (128, 240): a short straight basal stem, a tapered
               stalk and narrow pointed 90 px lanceolate central prong, flanked by two
               outward/downward curling prongs with small hooks. Strokes start at 7 px
               and taper to their tips. Alpha stays inside (56, 16)..(200, 244).
  rosette.png  96 x 96, transparent RGBA. An 80 px PAPER roundel holds an INK rosette
               with eight semicircular lobes, a 3 px line and a 12 px centre dot. Lobes
               divide the roundel equally; a line-width inset keeps the ink on paper.
  headpiece.png 2048 x 256, opaque RGBA. The round 3 lapis field and WHITE/GOLD rulings,
               mirrored lighter scrolls and rounded-leaf black corner spandrels remain.
               The centred cusped cartouche is 52% W x 72% H; outward-facing lobed
               pendants are 12% W x 46% H. All three are flat CARTOUCHE, with a 4 px
               PINK outline and a 2 px WHITE line outside. Pendants carry PINK blossoms
               and short GOLD stems join them to the cartouche. Scrolls stay at least
               12 px from the cartouche. The plain interior's text box, fractions
               (x0, y0, x1, y1) = (0.31, 0.31, 0.69, 0.69), stays 12 px inside the fill.

Geometry is rasterised at 4x and box-filtered in premultiplied working colour, then saved
with straight alpha and sRGB metadata. Coverage is measured before filtering: visible
GOLD and blossom area inside the panel's WHITE outline, and in the headpiece's exposed
lapis field (excluding cartouche, pendants and black spandrels). Totals with and without
the inner GOLD ruling are printed; both fields retain round 3's 26-32% coverage target.
Both repeating tiles are checked for identical seam columns and opaque alpha.

The optional 2112 x 2848 PAPER sheet contains the bilinearly sampled concentric wraps:
braid at inner radius 504 px, thickness 0.0216 x 504 px, 74 repeats; the main band at
inner radius 1.065 x 504 px, thickness 0.05 x 504 px, 30 repeats, with the round 3 3 px
GOLD rules and 48 outward 25.2 px fleurons. Below are the whole band, braid and headpiece
at 1:1, a 360 x 160 px crop of the finished rings enlarged exactly 3x with nearest-neighbour,
fleurons at 256 and 25 px, rosettes at 96 and 24 px, and the headpiece at 360 x 45 px.
The sheet has no text and needs no globe data. The full PNG stays at the supplied path;
tools/globe/ornament-sheet.jpg is a copy at most 1600 px wide, JPEG quality 85.
"""
import argparse
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, PngImagePlugin
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Globe'
SCALE = 4
LAPIS = '#5F6CA7'
GOLD = '#A08C60'
CARTOUCHE = '#9D8A57'
SALMON = '#C47C66'
PINK = '#D1A38F'
BLACK = '#251F14'
WHITE = '#F5F5F5'
LIGHT_BLUE = '#A2B7D0'
GREEN = '#79A18B'
INK = '#6D6793'
PAPER = '#F2E4C8'
TEXT_BOX = (0.31, 0.31, 0.69, 0.69)


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

    def stroke(self, points, width, colour, end_width=None, taper=1):
        points = np.asarray(points, float)
        tangent = np.gradient(points, axis=0)
        normal = np.column_stack((-tangent[:, 1], tangent[:, 0]))
        normal /= np.maximum(np.linalg.norm(normal, axis=1, keepdims=True), 1e-9)
        progress = np.linspace(0, 1, len(points)) ** taper
        widths = width + ((width if end_width is None else end_width) - width) * progress
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
    """A smooth 28 x 16 px split palmette: two round lobes and a round notch."""
    shape = bezier((0, 0), ((0, -4), (8, -8), (17, -8)),
                   ((23, -8), (28, -7), (28, -4)),
                   ((28, -1), (20, -3), (20, 0)),
                   ((20, 3), (28, 1), (28, 4)),
                   ((28, 7), (23, 8), (17, 8)),
                   ((8, 8), (0, 4), (0, 0)))
    shape[:, 1] *= 16 / np.ptp(shape[:, 1])
    return shape


def blossom(drawing, centre, colour, petals=5, diameter=40):
    """Lobed khatai in manuscript flower paint, always with a GOLD heart."""
    radius = diameter / 2
    for angle in np.linspace(-np.pi / 2, 3 * np.pi / 2, petals, endpoint=False):
        petal = np.asarray(centre) + radius * 0.57 * np.array([np.cos(angle), np.sin(angle)])
        drawing.dot(petal, radius * 0.43, colour)
    drawing.dot(centre, radius * 0.27, GOLD)


def bud(drawing, centre, angle, colour):
    """A 14 px bud including its solid gold calyx; no hairline veins."""
    shape = bezier((-7, 0), ((-2, -7), (7, -7), (7, 0)),
                   ((7, 7), (-2, 7), (-7, 0)))
    drawing.polygon(transform(shape, centre, angle), colour)
    calyx = [(-7, 0), (-2, -5), (-3, 0), (-2, 5)]
    drawing.polygon(transform(calyx, centre, angle), GOLD)


def leafy_curve(drawing, points, width=5, spacing=23, sides=(-1, 1)):
    """A shoulder sprig with smooth, rounded split palmettes."""
    drawing.stroke(points, width, GOLD)
    arc = np.r_[0, np.cumsum(np.linalg.norm(np.diff(points, axis=0), axis=1))]
    for number, distance in enumerate(np.arange(16, arc[-1] - 10, spacing)):
        index = min(np.searchsorted(arc, distance), len(points) - 2)
        angle = np.arctan2(*(points[index + 1] - points[index - 1])[::-1])
        side = sides[number % len(sides)]
        leaf = transform(split_leaf(), points[index], angle + side * 0.9)
        if leaf[:, 1].min() < 16 or leaf[:, 1].max() > 240:
            leaf = transform(split_leaf(), points[index], angle + side * 0.9 + np.pi)
        if leaf[:, 1].min() >= 16 and leaf[:, 1].max() <= 240:
            drawing.polygon(leaf, GOLD)


def arabesque(drawing, start_x, end_x, middle_y=128, amplitude=61.44, compartments=3,
              flowers=True, outer_volutes=False):
    """A 7 px interlace with one 6-to-3 px, 1.5-turn volute at every crest."""
    span = end_x - start_x
    step = span / compartments
    position = np.linspace(0, 1, 2401)
    phase = compartments * np.pi * position
    stems = [np.column_stack((start_x + span * position,
                             middle_y + sign * amplitude * np.sin(phase)))
             for sign in (-1, 1)]
    for stem in stems:
        drawing.stroke(stem, 7, GOLD)

    for bay in range(compartments):
        centre_x = start_x + (bay + 0.5) * step
        for outward in (-1, 1):
            root = (centre_x, middle_y + outward * amplitude)
            # An elliptic Archimedean spiral makes exactly 1.5 tightening turns.
            # Rotate the lower band volute; reflect headpiece volutes into its two
            # narrow blue margins above and below the pendants.
            progress = np.linspace(0, 1, 721)
            initial_angle = 2.47 if outer_volutes else -0.8
            angle = initial_angle + 3 * np.pi * progress
            radius = 1 - 0.90 * progress
            edge_volute = ((bay == 0 and outward == -1)
                            or (bay == compartments - 1 and outward == 1))
            radii = np.array((204 * step / 296.5, 30) if outer_volutes else
                             ((112.5 if edge_volute else 125) * step / 296.5, 98))
            centre = np.array((0, -amplitude)) - radii * (np.cos(initial_angle), np.sin(initial_angle))
            spiral = centre + radii * radius[:, None] * np.column_stack((np.cos(angle), np.sin(angle)))
            spiral *= (1 if outer_volutes else -outward, -outward)
            spiral += (centre_x, middle_y)
            drawing.stroke(spiral, 6, GOLD, end_width=3, taper=4)
            blossom(drawing, spiral[-1],
                    (PINK, WHITE, SALMON, LIGHT_BLUE, PINK)[(2 * bay + (outward == 1)) % 5],
                    diameter=26)
            # A broad curl returns into the outer margin and ends in a small bud.
            curve = bezier((0, -amplitude),
                           ((90, -100), (230, -118), (230, -60)),
                           ((230, 0), (96, 0), (96, -50)),
                           ((96, -80), (155, -90), (165, -62)))
            curve[:, 0] *= (0.52 if bay == compartments - 1 else 1) * step / 296.5
            curve[:, 1] *= -outward
            curve += (centre_x, middle_y)
            drawing.stroke(curve, 7, GOLD)
            for angle in (outward * 0.4, np.pi - outward * 0.4):
                drawing.polygon(transform(split_leaf(), root, angle), GOLD)
            bud(drawing, curve[-1], 0, PINK if bay % 2 else WHITE)
        if flowers:
            blossom(drawing, (centre_x, middle_y),
                    WHITE if bay % 2 == 0 else (PINK, SALMON, LIGHT_BLUE)[(bay // 2) % 3],
                    petals=6 if bay % 2 == 0 else 5)
    # One pair of rounded leaves at each crossing, outside the open underpass.
    for crossing in range(1, compartments):
        centre = start_x + crossing * step
        for side in (-1, 1):
            root = (centre + side * 12, middle_y + side * amplitude * np.sin(12 * np.pi / step))
            drawing.polygon(transform(split_leaf(), root, np.pi / 2 if side == 1 else -np.pi / 2), GOLD)
    # Preserve the alternating strands and exactly 2 px of lapis beside the top one.
    for crossing in range(1, compartments):
        centre = start_x + crossing * step
        top = stems[crossing % 2]
        local = top[np.abs(top[:, 0] - centre) <= 15]
        drawing.stroke(local, 11, LAPIS)
        drawing.stroke(local, 7, GOLD)


def colour_mask(drawing, colours):
    pixels = np.asarray(drawing.image)
    mask = np.zeros(pixels.shape[:2], bool)
    for colour in colours:
        mask |= np.all(pixels[..., :3] == rgb(colour), axis=2)
    return mask


def band():
    width, height = 1152, 256
    half_cell = 307 / 2
    drawing = Drawing((width, height), SALMON)
    row, column = np.ogrid[:height * SCALE, :width * SCALE]
    horizontal = ((column + 0.5) / SCALE + width / 2) % width - width / 2
    vertical = (row + 0.5) / SCALE - height / 2
    distance = np.full(drawing.coverage.size[::-1], np.inf, np.float32)
    for offset_x, offset_y in ((0.17, 0), (-0.17, 0), (0, 0.17), (0, -0.17)):
        distance = np.minimum(distance, np.hypot(horizontal - offset_x * height,
                                                  vertical - offset_y * height) - 0.2 * height)
    for limit, colour in ((2.5, WHITE), (-2.5, GOLD), (-4.5, BLACK)):
        drawing.image.paste((*rgb(colour), 255),
                            mask=Image.fromarray((distance <= limit).astype('uint8') * 255))
    star = np.array([(0, -60), (12, -12), (60, 0), (12, 12), (0, 60),
                     (-12, 12), (-60, 0), (-12, -12), (0, -60)])
    drawing.stroke(star + (0, height / 2), 4, GOLD)
    for angle in np.arange(4) * np.pi / 2:
        tip = np.array([(49, -3), (60, 0), (49, 3)])
        drawing.polygon(transform(tip, (0, height / 2), angle), GOLD)
    drawing.polygon(np.array([(0, -10), (6, 0), (0, 10), (-6, 0)]) + (0, height / 2), GOLD)
    for centre_y in (18, 238):
        drawing.polygon(transform(teardrop(), (half_cell - 31, centre_y),
                                  (1 if centre_y == 18 else -1) * np.pi / 6), GOLD)
    cell = drawing.image.crop((0, 0, int(half_cell * SCALE), height * SCALE))
    drawing.image.paste(cell.transpose(Image.Transpose.FLIP_LEFT_RIGHT),
                        (int((width - half_cell) * SCALE), 0))

    edge_y = np.linspace(0, height, height * SCALE + 1)
    edge_x = np.full_like(edge_y, 154)
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
    # Three clear pixels after the white edge, then a two-pixel inner gold ruling.
    ruling = (edge_distance > 9) & (edge_distance <= 11)
    drawing.image.paste((*rgb(GOLD), 255), mask=Image.fromarray(ruling.astype('uint8') * 255))
    ornaments = Drawing((width, height))
    arabesque(ornaments, 185, width - 185, compartments=3)
    visible = np.asarray(ornaments.image)[..., 3] > 0
    clearance = float(edge_distance[visible].min() - 11)
    drawing.image.alpha_composite(ornaments.image)
    decoration = colour_mask(ornaments, (GOLD, WHITE, PINK, SALMON, LIGHT_BLUE)) & visible
    ornament_coverage = np.count_nonzero(decoration & interior) / np.count_nonzero(interior)
    coverage = np.count_nonzero((decoration | ruling) & interior) / np.count_nonzero(interior)
    if not (0.26 <= coverage <= 0.32) or clearance < 3:
        raise ValueError(f'panel decoration: {coverage:.2%} coverage, '
                         f'{ornament_coverage:.2%} without ruling, {clearance:.2f} px clearance')
    image = box_rgba(drawing.image)
    pixels = np.asarray(image)
    if not np.array_equal(pixels[:, 0], pixels[:, -1]) or not np.all(pixels[..., 3] == 255):
        raise ValueError('band does not join seamlessly')
    print(f'band: gold + blossoms {coverage:.2%} of panel inside white outline; '
          f'{ornament_coverage:.2%} without ruling; {clearance:.2f} px clearance from gold ruling')
    print('band: opposite edge columns identical; alpha 255 throughout')
    return image


def braid():
    """Two periodic 30 px BLACK zigzags, alternating across 6 px GOLD underpasses."""
    width, height = 1024, 256
    period, stroke, gap = width / 4, 30, 6
    amplitude = (height - stroke) / 2
    drawing = Drawing((width, height), GOLD)

    def strand_y(horizontal, sign):
        triangle = 4 * np.abs(np.asarray(horizontal) / period % 1 - 0.5) - 1
        return height / 2 + sign * amplitude * triangle

    def distance_squared(x, y, start, end):
        vector = end - start
        progress = np.clip(((x - start[0]) * vector[0] + (y - start[1]) * vector[1])
                           / np.dot(vector, vector), 0, 1)
        return ((x - start[0] - progress * vector[0]) ** 2
                + (y - start[1] - progress * vector[1]) ** 2)

    # Distances at subpixel centres avoid polygon rasterisation's asymmetric edge
    # rounding. Include neighbouring segments for round joins across the tile seam.
    row, column = np.ogrid[:height * SCALE, :width * SCALE]
    x, y = (column + 0.5) / SCALE, (row + 0.5) / SCALE
    nodes = np.arange(-period / 2, 2 * period, period / 2)
    black = np.zeros(drawing.image.size[::-1], bool)
    for sign in (-1, 1):
        points = np.column_stack((nodes, strand_y(nodes, sign)))
        for start, end in zip(points[:-1], points[1:]):
            black |= distance_squared(x % period, y, start, end) <= (stroke / 2) ** 2
    for crossing, centre in enumerate(np.arange(period / 4, width, period / 2)):
        sign = (-1, 1)[crossing % 2]
        ends = np.array([centre - period / 8, centre + period / 8])
        top = np.column_stack((ends, strand_y(ends, sign)))
        vector = top[1] - top[0]
        distance = ((x - top[0, 0]) * vector[1] - (y - top[0, 1]) * vector[0]) ** 2
        distance /= np.dot(vector, vector)
        local = np.abs(x - centre) <= period / 8
        # A bounded strip has no round cap to cut the continuing top strand.
        black[local & (distance <= (stroke / 2 + gap) ** 2)] = False
        black[local & (distance <= (stroke / 2) ** 2)] = True
    drawing.image.paste((*rgb(BLACK), 255), mask=Image.fromarray(black.astype('uint8') * 255))
    image = box_rgba(drawing.image)
    pixels = np.asarray(image)
    if not np.array_equal(pixels[:, 0], pixels[:, -1]) or not np.all(pixels[..., 3] == 255):
        raise ValueError('braid does not join seamlessly')
    print('braid: four periods, eight alternating crossings, 30 px strands, 6 px GOLD gaps')
    print('braid: opposite edge columns identical; alpha 255 throughout')
    return image


def fleuron():
    drawing = Drawing((256, 256))
    drawing.stroke([(128, 240), (128, 204)], 7, INK)
    stalk = bezier((128, 204), ((126, 182), (120, 154), (128, 130)))
    drawing.stroke(stalk, 7, INK, end_width=5)
    leaf = bezier((128, 130), ((114, 111), (118, 73), (128, 40)))
    prong = bezier((128, 204), ((109, 184), (112, 140), (94, 120)),
                   ((80, 104), (75, 121), (79, 143)),
                   ((81, 158), (91, 172), (95, 162)),
                   ((98, 156), (90, 150), (88, 156)))
    for points in (leaf, prong):
        drawing.stroke(points, 7, INK, end_width=0.75, taper=2)
        drawing.stroke(points * (-1, 1) + (256, 0), 7, INK, end_width=0.75, taper=2)
    image = box_rgba(drawing.image)
    bounds = image.getchannel('A').getbbox()
    if not (bounds and bounds[0] >= 56 and bounds[1] >= 16 and bounds[2] <= 200 and bounds[3] <= 244):
        raise ValueError(f'fleuron alpha bbox outside its frame: {bounds}')
    print(f'fleuron: alpha bbox {bounds}, {bounds[2] - bounds[0]} x {bounds[3] - bounds[1]} px; '
          '90 px central prong, unfilled INK strokes')
    return image


def rosette():
    drawing = Drawing((96, 96))
    row, column = np.ogrid[:96 * SCALE, :96 * SCALE]
    distance = np.hypot((column + 0.5) / SCALE - 48, (row + 0.5) / SCALE - 48)
    drawing.image.paste((*rgb(PAPER), 255),
                        mask=Image.fromarray((distance <= 40).astype('uint8') * 255))
    # Eight outward semicircles on a regular octagon give eight rounded lobes.
    # Inset by the line width so the entire stroke stays on the paper roundel.
    apothem = (40 - 3) / (1 + np.tan(np.pi / 8))
    radius = apothem * np.tan(np.pi / 8)
    angle = np.linspace(-np.pi / 2, np.pi / 2, 97)
    lobe = np.column_stack((apothem + radius * np.cos(angle), radius * np.sin(angle)))
    outline = np.concatenate([transform(lobe, (48, 48), turn)
                              for turn in np.linspace(0, 2 * np.pi, 8, endpoint=False)])
    drawing.stroke(np.concatenate((outline, outline[:1])), 3, INK)
    drawing.image.paste((*rgb(INK), 255),
                        mask=Image.fromarray((distance <= 6).astype('uint8') * 255))
    print('rosette: 80 px PAPER roundel, eight INK lobes, 3 px line, 12 px centre dot')
    return box_rgba(drawing.image)


def cartouche_shape(centre, width, height):
    """A long cusped oval; its four mirrored shoulders have three broad lobes."""
    quadrant = bezier((1, 0), ((0.92, -0.08), (0.99, -0.35), (0.85, -0.35)),
                      ((0.93, -0.72), (0.72, -0.88), (0.66, -0.72)),
                      ((0.63, -1.02), (0.39, -1.04), (0.32, -0.88)),
                      ((0.22, -1.03), (0.08, -1), (0, -1)))
    top = np.concatenate((quadrant, (quadrant * (-1, 1))[::-1]))
    shape = np.concatenate((top, (top * (1, -1))[::-1]))
    # Normalise to the exact requested outer bounds, including the slight cusp overshoot.
    shape /= np.max(np.abs(shape), axis=0)
    return shape * (width / 2, height / 2) + centre


def pendant_shape(centre, width, height, side):
    """A lobed teardrop, with its single sharp point facing away from the cartouche."""
    top = bezier((1, 0), ((0.55, -0.10), (0.85, -0.48), (0.47, -0.47)),
                 ((0.57, -0.85), (0.05, -1.05), (-0.12, -0.82)),
                 ((-0.40, -1.12), (-0.95, -0.86), (-0.82, -0.52)),
                 ((-1.08, -0.54), (-1.02, -0.13), (-1, 0)))
    shape = np.concatenate((top, (top * (1, -1))[::-1]))
    low, high = shape.min(axis=0), shape.max(axis=0)
    shape = 2 * (shape - low) / (high - low) - 1
    return shape * (side * width / 2, height / 2) + centre


def polygon_mask(size, points):
    mask = Image.new('L', size)
    ImageDraw.Draw(mask).polygon([(float(x) * SCALE - 0.5, float(y) * SCALE - 0.5)
                                  for x, y in points], fill=255)
    return np.asarray(mask) > 0


def gilded_shape(drawing, points):
    """Flat CARTOUCHE paint, with a 4 px PINK outline inside a 2 px WHITE line."""
    mask = polygon_mask(drawing.image.size, points)
    distance = ndimage.distance_transform_edt(mask) / SCALE
    for limit, colour in ((0, WHITE), (2, PINK), (6, CARTOUCHE)):
        drawing.image.paste((*rgb(colour), 255),
                            mask=Image.fromarray((distance > limit).astype('uint8') * 255))
    return distance


def headpiece():
    width, height = 2048, 256
    drawing = Drawing((width, height), LAPIS)
    cartouche = cartouche_shape((width / 2, height / 2), width * 0.52, height * 0.72)
    central_mask = polygon_mask(drawing.image.size, cartouche)
    reserved = central_mask.copy()
    ornaments = Drawing((width, height))
    # Draw whole side scrolls rather than clipping scroll fragments around the cartouche.
    for upper in (True, False):
        shoulder = bezier((452, 49), ((505, 23), (557, 17), (610, 29)))
        if not upper:
            shoulder[:, 1] = height - shoulder[:, 1]
        leafy_curve(ornaments, shoulder, width=5, spacing=35, sides=(-1 if upper else 1,))
        bud(ornaments, shoulder[-1], 0, PINK)
    # Keep the volute blossoms above the shoulder foliage where the two meet.
    arabesque(ornaments, 28, 466, compartments=2, flowers=False, outer_volutes=True)
    blossom(ornaments, (90, 128), WHITE, petals=6)
    blossom(ornaments, (451, 128), LIGHT_BLUE)
    ornaments.image.alpha_composite(ornaments.image.transpose(Image.Transpose.FLIP_LEFT_RIGHT))
    distance_outside = ndimage.distance_transform_edt(~central_mask) / SCALE
    ornament_alpha = np.asarray(ornaments.image)[..., 3] > 0
    ornament_clearance = float(distance_outside[ornament_alpha].min())
    if ornament_clearance < 12:
        raise ValueError(f'headpiece scroll clearance {ornament_clearance:.2f} px')
    drawing.image.alpha_composite(ornaments.image)
    for side in (-1, 1):
        centre_x = width / 2 + side * width * 0.365
        inner_x = centre_x - side * width * 0.06
        edge_x = width / 2 + side * width * 0.26
        drawing.stroke([(inner_x, height / 2), (edge_x, height / 2)], 8, GOLD)
        pendant = pendant_shape((centre_x, height / 2), width * 0.12, height * 0.46, side)
        reserved |= polygon_mask(drawing.image.size, pendant)
        gilded_shape(drawing, pendant)
        blossom(drawing, (centre_x, height / 2), PINK)
    for flip_x in (False, True):
        for flip_y in (False, True):
            corner = bezier((0, 0), ((50, 0), (104, 0), (144, 0)),
                            ((133, 23), (104, 29), (98, 19)),
                            ((103, 47), (66, 54), (56, 38)),
                            ((57, 63), (18, 58), (0, 56)))
            corner = np.concatenate((corner, [(0, 0)]))
            def reflect(points):
                return np.asarray(points) * (-1 if flip_x else 1, -1 if flip_y else 1) + (
                    width if flip_x else 0, height if flip_y else 0)
            reserved |= polygon_mask(drawing.image.size, reflect(corner))
            drawing.polygon(reflect(corner), BLACK)
            sprig = bezier((15, 13), ((43, 18), (69, 18), (90, 12)))
            drawing.stroke(reflect(sprig), 3, GOLD)
            for x, y, angle in ((32, 16, 0.75), (58, 17, -0.5)):
                drawing.polygon(reflect(transform(split_leaf() * 0.6, (x, y), angle)), GOLD)
            bud(drawing, reflect([(89, 12)])[0], np.pi if flip_x else 0, PINK)
    # Coverage refers only to exposed blue ground; reserve the cartouche, pendants and
    # black corners. Report totals both with and without the inner gold field ruling.
    cartouche_distance = gilded_shape(drawing, cartouche)
    row, column = np.ogrid[:height * SCALE, :width * SCALE]
    edge = np.minimum(np.minimum((row + 0.5) / SCALE, height - (row + 0.5) / SCALE),
                      np.minimum((column + 0.5) / SCALE, width - (column + 0.5) / SCALE))
    drawing.image.paste((*rgb(WHITE), 255), mask=Image.fromarray((edge < 6).astype('uint8') * 255))
    drawing.image.paste((*rgb(GOLD), 255), mask=Image.fromarray(((edge >= 9) & (edge < 11)).astype('uint8') * 255))
    lapis_region = (edge >= 6) & ~reserved
    decoration = colour_mask(drawing, (GOLD, WHITE, PINK, SALMON, LIGHT_BLUE))
    coverage = np.count_nonzero(decoration & lapis_region) / np.count_nonzero(lapis_region)
    decoration &= ~((edge >= 9) & (edge < 11))
    ornament_coverage = np.count_nonzero(decoration & lapis_region) / np.count_nonzero(lapis_region)
    if not 0.26 <= coverage <= 0.32:
        raise ValueError(f'headpiece field decoration: {coverage:.2%} coverage, '
                         f'{ornament_coverage:.2%} without ruling')
    x0, y0, x1, y1 = TEXT_BOX
    text_clearance = cartouche_distance[int(y0 * height * SCALE):int(y1 * height * SCALE),
                                       int(x0 * width * SCALE):int(x1 * width * SCALE)].min() - 6
    if text_clearance < 12:
        raise ValueError(f'cartouche text box clearance {text_clearance:.2f} px')
    print(f'headpiece: gold + blossoms {coverage:.2%} of exposed lapis; '
          f'{ornament_coverage:.2%} without ruling; {ornament_clearance:.2f} px scroll clearance; '
          f'text box {TEXT_BOX}, {text_clearance:.2f} px inside gold fill')
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


def review_sheet(tile, woven, finial, marker, heading):
    sheet = Image.new('RGBA', (2112, 2848), (*rgb(PAPER), 255))
    centre_x, centre_y, reference_radius = 1056, 604, 504
    inner, outer = reference_radius * 1.065, reference_radius * (1.065 + 0.05)
    braid_inner, braid_outer = reference_radius, reference_radius * (1 + 0.0216)
    inner_ring, outer_ring = inner - 2.5, outer + 2.5
    texture = np.asarray(tile, np.float32)[..., :3]
    braid_texture = np.asarray(woven, np.float32)[..., :3]
    extent = int(np.ceil(outer_ring + 1.5))
    left, top, size = centre_x - extent, centre_y - extent, 2 * extent
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
        colour[annulus] = sample(texture, (angle[annulus] / (2 * np.pi) * 30) % 1,
                                 (radius[annulus] - inner) / (outer - inner))
        braid_annulus = (radius >= braid_inner) & (radius < braid_outer)
        colour[braid_annulus] = sample(
            braid_texture, (angle[braid_annulus] / (2 * np.pi) * 74) % 1,
            (radius[braid_annulus] - braid_inner) / (braid_outer - braid_inner))
        rings = (np.abs(radius - inner_ring) <= 1.5) | (np.abs(radius - outer_ring) <= 1.5)
        colour[rings] = rgb(GOLD)
        filtered = colour.reshape(rows, SCALE, size, SCALE, 3).mean((1, 3))
        sheet.paste(Image.fromarray(np.rint(filtered).astype(np.uint8)), (left, top + first_row))
    for angle in np.linspace(0, 2 * np.pi, 48, endpoint=False):
        base = (centre_x + outer_ring * np.sin(angle), centre_y - outer_ring * np.cos(angle))
        place_fleuron(sheet, finial, base, angle, reference_radius * 0.05)
    # This is a crop of the finished app-scale ring, enlarged without resampling its pixels.
    crop = sheet.crop((1010, 0, 1370, 160)).resize((1080, 480), Image.Resampling.NEAREST)
    sheet.alpha_composite(crop, (516, 2096))
    sheet.alpha_composite(tile, (480, 1220))
    sheet.alpha_composite(woven, (544, 1508))
    sheet.alpha_composite(heading, (32, 1796))
    sheet.alpha_composite(finial, (160, 2592))
    place_fleuron(sheet, finial, (520, 2758), 0, 25)
    sheet.alpha_composite(marker, (744, 2682))
    sheet.alpha_composite(marker.resize((24, 24), Image.Resampling.BOX), (1020, 2715))
    sheet.alpha_composite(heading.resize((360, 45), Image.Resampling.BOX), (1450, 2702))
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
    tile, woven, finial, marker, heading = band(), braid(), fleuron(), rosette(), headpiece()
    for image, name in ((tile, 'band.png'), (woven, 'braid.png'), (finial, 'fleuron.png'),
                        (marker, 'rosette.png'), (heading, 'headpiece.png')):
        save(image, OUT / name)
    if args.sheet:
        sheet = review_sheet(tile, woven, finial, marker, heading)
        save(sheet, args.sheet)
        preview = sheet.convert('RGB')
        preview.thumbnail((1600, sheet.height), Image.Resampling.LANCZOS)
        preview_path = ROOT / 'tools/globe/ornament-sheet.jpg'
        preview.save(preview_path, quality=85, subsampling=0)
        print(f'wrote {preview_path.relative_to(ROOT)} '
              f'({preview.width} x {preview.height}, JPEG quality 85)')


if __name__ == '__main__':
    main()
