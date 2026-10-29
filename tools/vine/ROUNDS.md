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
