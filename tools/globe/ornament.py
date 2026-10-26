"""Draw the globe's Persian ornaments (Globe/) with offline Python geometry.

    python3 tools/globe/ornament.py
    python3 tools/globe/ornament.py --sheet /tmp/ornament-sheet-r3.png

The default run writes all four assets; --sheet also regenerates the review sheet.
Only numpy, Pillow, scipy and the Python standard library are used. No source images,
fonts, lettering or network access are needed.

  band.png     1536 x 256, opaque RGBA, one of sixteen repeats around the globe. x wraps;
               y runs from the band's inner edge to its outer edge. The original 288 px
               salmon seam cell and three-lobed lapis panel retain their layout. The black
               quatrefoil has four circular lobes of radius 0.2 H, a 5 px white outline,
               an immediately inner 2 px gold rim, and a 4 px gold star with light-gold tips.
               Inside the panel's 6 px white outline, a 2 px gold rule follows after a
               3 px lapis gap. Two 7 px gold sines in antiphase, amplitude 0.24 H, enclose
               four oval compartments. At the three internal centre-line crossings, the
               top strand alternates; an 11 px lapis knockout leaves 2 px beside it.
               Each bay has a nominal 40 px khatai: alternating six white petals/rose heart
               and five rose or orange petals/light-gold heart. Each crest carries one
               tightening 1.5-turn volute, tapering from 6 to 3 px, ending in a 26 px
               five-petal blossom with a light-gold heart. Its colours cycle through
               rose, white, orange, pale blue and pink. Outward crest curls each carry
               one 14 px pink/white bud with a gold calyx. Rounded 28 x 16 px split
               palmettes occur in pairs only at volute roots and stem crossings. All
               ornamental strokes are at least 3 px; the requested rules/gaps are 2 px.
  fleuron.png  256 x 256, transparent RGBA, pointing up. A washed trefoil crowns a blue-ink
               stem and two hooked tendrils (minimum stroke 3 px). Alpha stays inside
               (56, 16)..(200, 244); the sheet places the actual bottom alpha edge on the ring.
  rosette.png  96 x 96, transparent RGBA: eight pointed almond petals, C9AE6A light gold
               with 3 px 8C6D3A outlines. An 18 px 4E5B9B lapis centre has a 2 px gold ring.
  headpiece.png 2048 x 256, opaque RGBA, lapis with the same white/gold field ruling.
               The centred cusped gold oval is 52% W x 72% H; outward-pointing lobed
               teardrop pendants are 12% W x 46% H and carry small rose blossoms. Short
               gold stems join them to the cartouche. The three gold grounds use C9AE6A,
               deterministic +/-3% speckle in 3 px grains, a 4 px E3A08D pink outline,
               and a 2 px WHITE outer line. Black quarter-lobed corner spandrels reach
               about 22% H and contain gold/pink sprigs with rounded leaves. Mirrored
               interlaced scrolls, blossoms and shoulder sprigs remain at least 12 px
               from the cartouche; only the two joining stems enter that margin.
               The gold interior is plain.
               Cartouche inner text box, fractions (x0, y0, x1, y1):
                   (0.31, 0.31, 0.69, 0.69)
               Every pixel of this box is at least 12 px inside the gold fill.

The original sRGB palette studies LACMA's Khamsa of Nizami (M.85.237.16); only the newly
specified light gold, rosette outline and cartouche pink replace or extend those colours.
Geometry is rasterised at 4x and box-filtered in premultiplied working colour, then saved
with straight alpha and sRGB metadata. Coverage is measured before filtering, as visible
area of gold and blossom colours divided by area inside the white outline. Both total
coverage (including the inner gold rule) and ornament-only coverage are printed. The band
requires total coverage to lie in 26-32%; its seam columns and opaque alpha are checked.
Headpiece coverage excludes the cartouche, pendants and black spandrels, counting only exposed lapis.
Its total coverage must also lie in 26-32%; the value without its field ruling is printed too.

The optional 2112 x 2544 paper sheet contains, top to bottom: a bilinearly sampled app-scale
polar wrap (inner radius 504 px, thickness 0.0685 radius, sixteen repeats), gold rings and
48 outward fleurons; the whole band at 1:1; the whole headpiece at 1:1; a 360 x 160 px crop
of that finished ring enlarged exactly 3x with nearest-neighbour; then fleurons at 256 and
25 px, rosettes at 96 and 30 px, and the headpiece at its 360 x 45 px app size. The sheet
contains no text and needs no globe data. With --sheet, the full PNG stays at the supplied
path; tools/globe/ornament-sheet.jpg is a copy at most 1600 px wide, JPEG quality 85.
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
GOLD_LIGHT = '#C9AE6A'
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
ROSETTE_OUTLINE = '#8C6D3A'
CARTOUCHE_PINK = '#E3A08D'
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
    """Lobed khatai; the white six-petal form has a rose heart."""
    radius = diameter / 2
    for angle in np.linspace(-np.pi / 2, 3 * np.pi / 2, petals, endpoint=False):
        petal = np.asarray(centre) + radius * 0.57 * np.array([np.cos(angle), np.sin(angle)])
        drawing.dot(petal, radius * 0.43, colour)
    drawing.dot(centre, radius * 0.27, ROSE if petals == 6 else GOLD_LIGHT)


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


def arabesque(drawing, start_x, end_x, middle_y=128, amplitude=61.44, compartments=4,
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
                    (ROSE, WHITE, ORANGE, PALE_BLUE, PINK)[(2 * bay + (outward == 1)) % 5],
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
                    WHITE if bay % 2 == 0 else (ROSE, ORANGE, PALE_BLUE)[(bay // 2) % 3],
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
    width, height = 1536, 256
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
        drawing.polygon(transform(tip, (0, height / 2), angle), GOLD_LIGHT)
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
    # Three clear pixels after the white edge, then a two-pixel inner gold ruling.
    ruling = (edge_distance > 9) & (edge_distance <= 11)
    drawing.image.paste((*rgb(GOLD), 255), mask=Image.fromarray(ruling.astype('uint8') * 255))
    ornaments = Drawing((width, height))
    arabesque(ornaments, 175, width - 175)
    visible = np.asarray(ornaments.image)[..., 3] > 0
    clearance = float(edge_distance[visible].min() - 11)
    drawing.image.alpha_composite(ornaments.image)
    decoration = colour_mask(ornaments, (GOLD, GOLD_LIGHT, WHITE, ROSE, ORANGE, PALE_BLUE, PINK)) & visible
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
    drawing.stroke(tendril, 3.5, INK, end_width=3)
    drawing.stroke(tendril * (-1, 1) + (256, 0), 3.5, INK, end_width=3)
    image = box_rgba(drawing.image)
    bounds = image.getchannel('A').getbbox()
    if not (bounds and bounds[0] >= 56 and bounds[1] >= 16 and bounds[2] <= 200 and bounds[3] <= 244):
        raise ValueError(f'fleuron alpha bbox outside its frame: {bounds}')
    print(f'fleuron: alpha bbox {bounds}, within (56, 16, 200, 244)')
    return image


def rosette():
    drawing = Drawing((96, 96))
    petal = bezier((7, 0), ((18, -10), (32, -11), (40, 0)),
                   ((32, 11), (18, 10), (7, 0)))
    for angle in np.linspace(0, 2 * np.pi, 8, endpoint=False):
        points = transform(petal, (48, 48), angle)
        drawing.polygon(points, GOLD_LIGHT)
        drawing.stroke(points, 3, ROSETTE_OUTLINE)
    drawing.dot((48, 48), 11, GOLD_LIGHT)
    drawing.dot((48, 48), 9, LAPIS)
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


def gilded_shape(drawing, points, seed):
    """Opaque gold, with three-pixel grain at most +/-3%; 4 px pink, 2 px white."""
    mask = polygon_mask(drawing.image.size, points)
    distance = ndimage.distance_transform_edt(mask) / SCALE
    for limit, colour in ((0, WHITE), (2, CARTOUCHE_PINK)):
        drawing.image.paste((*rgb(colour), 255),
                            mask=Image.fromarray((distance > limit).astype('uint8') * 255))
    rng = np.random.default_rng(seed)
    height, width = mask.shape
    grain = rng.uniform(-0.03, 0.03, ((height + 11) // 12, (width + 11) // 12, 1))
    grain = np.repeat(np.repeat(grain, 12, axis=0), 12, axis=1)[:height, :width]
    gold = np.rint(np.asarray(rgb(GOLD_LIGHT)) * (1 + grain)).astype('uint8')
    drawing.image.paste(Image.fromarray(gold),
                        mask=Image.fromarray((distance > 6).astype('uint8') * 255))
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
    blossom(ornaments, (451, 128), PALE_BLUE)
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
        gilded_shape(drawing, pendant, 8)
        blossom(drawing, (centre_x, height / 2), ROSE)
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
    cartouche_distance = gilded_shape(drawing, cartouche, 23)
    row, column = np.ogrid[:height * SCALE, :width * SCALE]
    edge = np.minimum(np.minimum((row + 0.5) / SCALE, height - (row + 0.5) / SCALE),
                      np.minimum((column + 0.5) / SCALE, width - (column + 0.5) / SCALE))
    drawing.image.paste((*rgb(WHITE), 255), mask=Image.fromarray((edge < 6).astype('uint8') * 255))
    drawing.image.paste((*rgb(GOLD), 255), mask=Image.fromarray(((edge >= 9) & (edge < 11)).astype('uint8') * 255))
    lapis_region = (edge >= 6) & ~reserved
    decoration = colour_mask(drawing, (GOLD, GOLD_LIGHT, WHITE, ROSE, ORANGE, PALE_BLUE, PINK))
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


def review_sheet(tile, finial, marker, heading):
    sheet = Image.new('RGBA', (2112, 2544), (*rgb(PAPER), 255))
    centre_x, centre_y, inner = 1056, 580, 504
    outer = inner * (1 + 0.0685)
    inner_ring, outer_ring = inner - 2.5, outer + 2.5
    texture = np.asarray(tile, np.float32)[..., :3]
    left, top, size = 506, 30, 1100
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
    # This is a crop of the finished app-scale ring, enlarged without resampling its pixels.
    crop = sheet.crop((1010, 0, 1370, 160)).resize((1080, 480), Image.Resampling.NEAREST)
    sheet.alpha_composite(crop, (516, 1760))
    sheet.alpha_composite(tile, (288, 1170))
    sheet.alpha_composite(finial, (160, 2240))
    place_fleuron(sheet, finial, (520, 2406), 0, 25)
    sheet.alpha_composite(marker, (744, 2330))
    sheet.alpha_composite(marker.resize((30, 30), Image.Resampling.BOX), (1017, 2363))
    sheet.alpha_composite(heading, (32, 1460))
    sheet.alpha_composite(heading.resize((360, 45), Image.Resampling.BOX), (1450, 2350))
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
    tile, finial, marker, heading = band(), fleuron(), rosette(), headpiece()
    for image, name in ((tile, 'band.png'), (finial, 'fleuron.png'), (marker, 'rosette.png'),
                        (heading, 'headpiece.png')):
        save(image, OUT / name)
    if args.sheet:
        sheet = review_sheet(tile, finial, marker, heading)
        save(sheet, args.sheet)
        preview = sheet.convert('RGB')
        preview.thumbnail((1600, sheet.height), Image.Resampling.LANCZOS)
        preview_path = ROOT / 'tools/globe/ornament-sheet.jpg'
        preview.save(preview_path, quality=85, subsampling=0)
        print(f'wrote {preview_path.relative_to(ROOT)} '
              f'({preview.width} x {preview.height}, JPEG quality 85)')


if __name__ == '__main__':
    main()
