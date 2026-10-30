# The vine, round by round

The vine remade on the globe's dated geography (runtime 6), looked at as a reader would after each
round (the preview's `check.cjs`, `moments.cjs` plans, CI's screenshots) and torn apart by a fresh
critic given only the pictures. Each round: what was built, what the critic found that mattered, what
changed. The plan the moments were taken from is in each round so the shots can be taken again:

    node tools/packart/moments.cjs vine build/vine-moments '<plan>'

## Round 0 · the vine as it was (October 29)

Brown lines regrown by the script on every `time` event (`terra.geo.set` thirty times a second), leaf
stickers at fixed places, a tendril sprite re-set at each tip, clusters to pick. On the iPad at the
anchor the vine was a thin brown thread; the lines never thickened; nothing to touch mattered; the
tool it needed (lines that grow with the date) did not exist, so the script faked it.

## Round 1 · regrown on dated geography (October 30)

Built: dated geography in the runtime (lines drawn to the date with a tip, points shown once the date
passes, `thickens`, `bloom`; Swift, the preview, the validator, the contract, the preview's own check);
the data rebuilt (29 trunks following coasts, valleys and roads; 42 twigs to the world's wine regions
with the record's dates; land and sea cut by the globe's own coast into stems and voyages; leaves on
land only, each hemisphere its own season; tendrils as sprites on dated ways); the art redrawn (a
five-lobed toothed leaf, a writhing tendril with a young leaf, shaded clusters, the qvevri cut away);
touch: light runs from the qvevri along the vine to whatever is touched, and the foot names the place
and its year.

Plan: `[{"at":[42,39,2.4]},{"wait":22},{"shot":"01"},{"at":[30,36,2.0]},{"wait":40},{"shot":"02"},{"at":[25,35,1.15]},{"wait":40},{"shot":"03"},{"date":0.78},{"wait":3},{"shot":"04"},{"date":0.999},{"wait":4},{"shot":"05"},{"at":[-70,-25,1.8]},{"wait":2},{"shot":"06"},{"at":[5,46,5]},{"wait":2},{"shot":"07"},{"touch":[4.85,47.0]},{"wait":1.5},{"shot":"08"},{"at":[44.75,41.45,6]},{"wait":2},{"shot":"09"}]`

Seen myself before the critic: the qvevri's ground was a dark box (an ellipse now, soft at its ends);
the trace's light was too pale on the cream ground (amber under white beads now); "Burgundy · 100"
read as a count ("100 CE" now).

Critic (fresh, given the pictures and the bar only): not the best thing on Terra yet; "a brown polyline
with emoji stuck to it". What mattered, in its order: (1) leaves and clusters a fixed screen size, so the
hemisphere is buried in confetti and the close view sparse; (2) the stem a constant-width polyline with
hard kinks, the thickening with age invisible, leaves floating off it; (3) the first ten seconds don't
happen: it opens on a 30-point jar on a hemisphere; (4) the qvevri a side-view amphora with a drop shadow
on a top-down map; (5) the touch's light nearly invisible on cream land; (6) voyage lines indistinguishable
from the map's own dashes; (7) leaves along every kilometre (deserts, Brazil's coast read as vineyards),
clusters where vines never took (Cuba, Panama), green clusters in the southern spring; (8) "Burgundy ·
100"; (10) a children's-sticker palette. Fair, all of it.

## Round 2 · scale, the opening, the light (October 30)

Changed: the stems in three zoom bands (a slender thread from afar, a thick old stem close in), the
leaves and clusters sized to each band and thinned from afar (no clusters at the whole Earth); the
trunks curved through their places (Catmull-Rom) rather than kinked; leaves only within reach of a named
place, so a desert crossing is a bare stem; Cuba and Panama unnamed; clusters only in a hemisphere's
summer (green) and autumn (ripe), leaves alone in spring, dry sparse leaves in winter; the qvevri redrawn
from above (a dark mouth in the earth, a clay rim, the lid half off); the palette pulled toward the
map's (umber stem, ochre and rust leaves, dusty vine-black grapes); the voyage line in the vine's own
brown; the touch's light a gold line with white beads and a spark that runs from the qvevri to the finger;
"100 CE"; the first shoots staggered (one shoot, then the next); the opening close on the qvevri's mouth,
a beat, then the years play and the camera pulls back over half a minute to fifty-five degrees across the
screen (a phone no farther than twelve points a degree). The preview learned to redraw dated geography
when the date moves (its cached layer had not).

Plan (iPad): `[{"wait":3.2},{"shot":"01"},{"wait":4},{"shot":"02"},{"wait":10},{"shot":"03"},{"wait":16},{"shot":"04"},{"wait":30},{"shot":"05"},{"at":[25,35,1.0]},{"date":0.999},{"wait":3},{"shot":"06"},{"at":[8,45,3.2]},{"wait":2},{"shot":"07"},{"touch":[4.85,47.0]},{"wait":1.0},{"shot":"08"},{"wait":2.5},{"shot":"09"},{"at":[-70,-25,2.0]},{"wait":2},{"shot":"10"},{"at":[44.75,41.45,6.5]},{"wait":2},{"shot":"11"},{"at":[2,44,6]},{"wait":2},{"shot":"12"}]`

Critic 2 (fresh): "a pipeline map with emoji on it". Ranked: (1) the stem still a route: elbows, the taper
invisible (everything over 1,800 years old was full width, so Rome's branches matched the first trunk);
(2) the opening still a star of shoots; the qvevri "a coin at country zoom"; (3) the art stamp-like
(maple-ish leaves, snail tendrils, a bullseye qvevri with a ball-bearing lid); (4) the touch response
too faint at the iPad's scale; (5) the foot's two dates ("2025" over "Burgundy · 100 CE") read as a
contradiction; (6) the whole-Earth payoff without the Americas in frame; the iPhone's globe a small ball
at zoom 1. Kept, with thanks: the palette, the voyage line, the strip, the pace.

Overruled, and why: the whole-Earth framing is the reader's camera, not the pack's; the phone's globe at
zoom 1 is Terra's globe; Sicily joined to Calabria by a land stem is the strait (3 km) and true; the
China branch "to the Amur" is the Korea–Japan trunk foreshortened at the limb; blue land in the Andes
shot is the preview stand-in's polygon bug at that camera, not the app's (noted, not fixed here).

## Round 3 · a plant, not a route (October 30)

Changed: the trunks curved through their places with a point every 40 km (the elbows are gone); a
stem is full-grown after five thousand years, not 1,800, so the first trunks are the thick ones, Rome's
half their width, the colonies' shoots thin; the first shoots leave the valley one at a time (the first
vine alone for nearly four seconds at the slower opening pace, then the Kura, the Zagros, Anatolia);
the qvevri is a picture lying on the globe (`terra.images`), so it grows as the reader comes close and
the first shoot visibly leaves its mouth, with cracked earth round it and a flagstone lid a third off;
every tap is the pack's and a tap on the mouth begins again; the leaf rounder (a vine's, not a
maple's); the tendril thin and the stem's brown-green; the touch's light wider and brighter, white
beads, a bigger spark, and a slow pulse of light where the finger was while the way stays lit; the
foot says "since 100 CE"; a lake crossed is a stem, not a voyage (runs under four points join).

Plan (iPad): `[{"wait":3.4},{"shot":"01"},{"wait":4},{"shot":"02"},{"wait":8},{"shot":"03"},{"wait":18},{"shot":"04"},{"wait":30},{"shot":"05"},{"at":[25,35,1.0]},{"date":0.999},{"wait":3},{"shot":"06"},{"at":[-30,20,1.0]},{"wait":2},{"shot":"06b"},{"at":[8,45,3.2]},{"wait":2},{"shot":"07"},{"touch":[4.85,47.0]},{"wait":1.0},{"shot":"08"},{"wait":2.5},{"shot":"09"},{"at":[-70,-25,2.0]},{"wait":2},{"shot":"10"},{"at":[44.75,41.45,6.5]},{"wait":2},{"shot":"11"},{"at":[-3,40,5.5]},{"wait":2},{"shot":"12"},{"at":[44.75,41.45,14]},{"wait":2},{"shot":"13"}]`

Critic 3 (the hater, as the owner asked): worst first, (1) the first minute dead air: the camera pulled
back before the growth could be watched, so the first thirty seconds were a year counter over an empty
map; (2) stamps at equal intervals, clusters hanging over the sea at island regions, a qvevri that was a
marble with an octagon; (3) the lit way a motorway (a flat yellow band with white dashes), the caption
stale after the camera left; (4) corners at joints, trunks crossing the mouth like spokes, finished stems
cut flat; (5) modern borders and names in 5300 BCE; (6) the whole Earth a thin road network; (7) the
strip's dots unlabelled, a dark rectangle round the scrubber. Kept: the one-line caption, the thick old
trunks, the honest spine.

## Round 4 · the growth is the show (October 30)

Changed: the camera holds close for ten seconds while the first shoot leaves the mouth, curls and leafs,
then pulls back over twenty-five as the vine fills the frame; one stem leaves the mouth, east along the
Kura into Kakheti, and the others fork from it at nodes (Colchis, the Zagros, Anatolia from Shulaveri),
so the mouth is a mouth and not a roundabout; today's borders and names are hidden while the vine runs
(`historical`, as the herds do); a soft leaf-tinted band under the thread from afar; leaves at uneven
gaps with their kinds mixed, and a leaf at every finished trunk's end; the lit way a warm band the stem's
width with soft gold light running along it, the caption cleared with it after eleven seconds; the
qvevri's earth mottled, its rim worn and wine-stained with the potter's coil lines, the mouth truly dark,
the flagstone grained; the voyages dashed in the vine's own brown.

Overruled, and why: sprites scaling continuously with the camera is a runtime tool that doesn't exist
(a symbol's size in degrees); the three zoom bands stand in for it here, and it is noted below as the
next tool the vine would use. Clusters at Ibiza, Corsica, Carthage and Algiers hang at coasts and
islands because wine was made there; the voyage line reaches them. The strip's dots and the rectangle
round the scrubber are the host's and the preview's.

Plan (iPad): `[{"wait":5},{"shot":"01"},{"wait":4},{"shot":"02"},{"wait":5},{"shot":"03"},{"wait":10},{"shot":"04"},{"wait":16},{"shot":"05"},{"wait":25},{"shot":"06"},{"at":[-30,20,1.0]},{"date":0.999},{"wait":3},{"shot":"07"},{"at":[25,35,1.0]},{"wait":2},{"shot":"08"},{"at":[8,45,3.2]},{"wait":2},{"shot":"09"},{"touch":[4.85,47.0]},{"wait":1.0},{"shot":"10"},{"wait":2.5},{"shot":"11"},{"at":[44.75,41.45,14]},{"wait":2},{"shot":"12"},{"at":[-3,40,5.5]},{"wait":2},{"shot":"13"}]`

Critic 4 (the hater): worst first, (1) the spark sat in Greece for the seconds a player looks (the
sprite's path began where the camera happened to be, and the pulse was a glint, off between showings)
and the lit way was still a tape; (2) the whole Earth a smudge; (3) stamps beside the stem, not on it;
(4) corners and blunt ends; (5) the opening's two shoots leaving symmetrically, a bolt-head lid, cracks
like a broken screen; (6) the preview's focus rectangle round the strip (the preview's, not the app's);
(7) nothing moves but a line: no ship on the sea legs. Kept: the thickening, the seasons' split, the
dark mouth, the foot out of the way.

## Round 5 · the touch answers at once; ships on the sea (October 30)

Changed: the spark is under the finger within a breath (half a second to 1.3), the pulse is a continuous
breathing glow (six frames), the lit way a soft wide glow under a warm band with quiet light running
along it; a sail on every long voyage, a sprite on the voyage's dated way, so the sea legs are crossed
by ships as the years cross them (31 ships beside the 29 tendrils, within the sprite budget); corners
cut at the unnamed waypoints before the curve; a twig leaves its trunk along the trunk's own direction
and bends to its region; a branch is attached to the trunk point it grows from; leaves stand on the
stem (their stalk at the foot of the picture); the far view's leaf tint is a band under the thread (it
had been set before the stems existed and drew nothing); the second shoot leaves later and from along
the first, not from the mouth; the mouth smaller; the lid an uneven stone, the cracks fewer and fainter.

Critic 5 (the hater): (1) still a route map at the hemisphere and in Iberia (straight strokes, one width
west of Anatolia); (2) the opening a thumbnail: the mouth a fifteenth of the screen; (3) leaves and
clusters confetti, an "eternal October"; (4) the lit way a highway with ticks; (5) no ship on the
Hispaniola line at 1500 (it had arrived: the date was the arrival's); (6) a vinyl-record mouth and a
manhole lid; (7) only the vine answers a finger. Kept: the restrained words, the true way lit.

## Round 6 · bark, the mouth a quarter of the screen (October 30)

Changed: a darker edge under every stem band (the bark), so the stem has a body; the way's corners cut
twice before the curve and the sway wider; the opening's close zoom set from the screen so the mouth is a
quarter of its width (the globe's zoom limit raised to 32 for it), and a closest band of big leaves,
clusters and tendrils where the mouth fills the screen; the light's beads fewer and softer; the mouth's
inner wall in shadow instead of a highlight arc; the lid mottled stone with one hairline crack, no ruled
lines.

Overruled, and why: the reader's own season on the leaves is the design (the panel says so), not an
error; the sea, the empty land and the leaves answering a finger would be another experience's rules
(the vine answers where the vine is); a wobbling, noise-textured stem would need a tool the runtime
doesn't have (a textured stroke), noted under the tools the vine would use next.

Critic 6 (the hater): the worst complaint unchanged in kind: the ways are chords between the record's
places (a Z across Iberia, a ruler run across Central Asia), the leaves one stamp at even gaps, the
opening a flat icon on cream, the Atlantic a ferry route, nothing but the lit way answering a finger.
Kept: the lit way and the spark, the mouth as the opening image, the clean strip and caption.

## Round 7 · the wander, the leaves' sizes; and where the rounds stop (October 30)

Changed: the stem wanders on two wavelengths, wider, so it meanders rather than sines between places;
Iberia follows the Guadalquivir and the Meseta to the Ebro instead of a Z; each leaf kind has its own
size and turn, so a stem's leaves are three leaves and not one stamp.

Where the rounds stop, and why. The hater's worst complaint is now the premise's drawing: a way drawn
between the places the record names is a chord, and a stroke drawn by the globe is a stroke. Making the
vine "a plant" in the sense asked for needs two things the runtime does not have and this session did
not build: ways routed along valleys and coasts from a terrain or river dataset (the water study has
rivers for South America only), and a textured, noise-varied stroke (a symbol's size in degrees would
let leaves scale with the camera too). Both are listed under the tools the vine would use next. What
is here is set once and drawn by the globe at every zoom, grows as the years play, is true to the record
underneath, answers a touch at once with light running back to the qvevri, and opens on the mouth with
one shoot leaving it. Six fresh critics in a row found the first-minute problems of each round and
those are fixed; what remains is a tool's reach, not a round's.

## Tools the vine would use next

- A symbol's size in degrees (a leaf the size of a valley, scaling with the camera) instead of zoom bands.
- A line revealed over seconds (the lit way running from the root to the finger as a line, not only as a
  spark); dated geography reveals by the stage's date, not by the clock.
- A textured stroke for `border` (a noise-varied width, a bark or water texture along it), so a vine, a
  river or a road drawn by the globe is not a flat line close in.
- Ways routed over terrain: a builder that lays a way between two places along valleys, coasts or rivers
  from the globe's own terrain (as the beacons' sight lines read it), so a vine's or a road's way is the
  land's and not a chord.

## Round 8 · a plant after all: strands, grain, the ground round the mouth (October 30)

The Interviewer, rightly: the worst complaint was half-answered, not overruled. So: two thinner stems
twine round every trunk's land runs (built from the way, offset either side by a sine of the distance
along it, crossing the trunk twice a wave, dated as the trunk is), so close in the vine is a braid of
stems and not one line; `grain` on dated borders in the runtime (`thickens: { to, over, grain }`): the
ribbon swells and narrows place by place along the way like bark, steadily under the camera and the
date (Swift, the preview, the contract, the catalog); the qvevri's picture widened to the valley's
ground round it (trodden earth, stones, tufts of grass) with the rim a fifth of the screen at the
opening; the picture drawn at two pixels a point (it lies on the globe). Validated in the preview's own
check (96 of 96).

Critic 7 (the hater): the hemisphere still a wire diagram with nothing alive on it; the strands
"parallel copies that never cross"; three arms radiating over the rim and the lid at 14 s; the ship a
fingertip; the touch generic. Kept: the qvevri as a drawn object, the one line at the foot.

## Round 9 · the far view alive, the strands twining, the rim clear (October 30)

Changed: from afar a few leaves (one in four, small) and the trunks' own clusters stay, so the whole
Earth sees a plant with fruit on it and not a cable; the strands are built on a run resampled four
times finer, so they twine as curves and cross the trunk rather than running beside it; the first
branches leave the mouth's rim clear (Colchis north of it, the Zagros east of it). The far view's
leaf is one of the three drawn leaves by name, so the validator sees it.

The hater's remaining asks (a stem drawn as an organic stroke, ways that follow valleys, a ship that
grows with the sea it crosses) are the tools listed at the end; the round's changes answer what a
pack can answer. The rounds stop here: eight critics, the worst complaint now the premise's drawing.

## QA pass · the vine on an iPhone simulator (October 30)

An exploratory QA run (`qa-explore.yml`, agent mode, debug build, `-prototype vine`, 8 minutes) played
it on the iPhone: the opening, play from the end (it begins again), taps on clusters and stems. The
light ran to Shiraz ("Shiraz · since 2501 BCE") and the foot named it; nothing crashed. The app's log
had one error of the vine's: the Danube's tendril was refused because its way's dates fell for three
places (the trunk's start had been moved to 90 BCE in round 5 and its next place left at 100 BCE), so
that growing tip had no tendril on the device; fixed in the data (`build.py`) and rebuilt. The agent
ran out of its clock before writing a report (each screenshot took it two and a half minutes), so the
findings are mine from its seven pictures and the log. Seen in them and left: the playground's chip
wraps the pack's name and "Open" on the phone ("The vi… / Op en"), which is the stage's chrome shared by
every prototype and not the vine's to change; noted for the stage.
