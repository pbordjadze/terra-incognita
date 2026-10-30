# The night crossing: the round log

The experience: `Prototypes/hydaspes/` (the Hydaspes, 326 BCE, Alexander's crossing against Porus,
as Arrian tells it in Anabasis 5.9–18). This is the log of the rounds it was built in: what was
pictured, what each round looked at, what the fresh eye said, what the next round took on.
Painting-company row 183 holds the brief; nothing here quotes it.

## The picture (before any tool was read)

A reader comes in over the Punjab. Coming closer, the day gives way: the valley of the Jhelum darkens
to the night of the crossing, the river swollen and brown, two camps of fires on its two banks, rain
drifting over the plain's edges, lightning now and then over the hills. Nothing is written but the
name at the foot. Close in, the camps are armies as a strategy game draws them, in a high three-quarter
view: tents, pickets, horse lines; on the far bank, a row of grey shapes at the water: the elephants.

A touch on the near camp and a column peels away from it. The reader drags along the river and the
night moves under the finger: the column goes through the dark, inland behind the hills' skirt, to the
wooded headland at the bend, while the fires it left keep burning and figures still walk among them,
so from across the water the camp looks full. The elephants keep watching it.

At the bend the reader pushes the boats and the hay-stuffed hides across to the island. The column
lands, forms up, marches, and the river opens in front of it again: it is on an island. Close, the
soldiers wade the second channel to the chest, shields over their heads; of the horses only the heads
show. Nothing explains it. Then dawn: the chariots come and bog in the clay; Porus' line comes up with
the elephants a plethron apart; the horsemen sweep wide round its flank; the elephants turn to meet
them. All of it at the speed of the drag, or played from the foot.

Open: the sources (Arrian, Curtius for the decoy tent), the two sites still argued over (Stein's
Jalalpur, with the camp at Haranpur; the road crossing by Jhelum, where the river's old courses point)
and how uncertain every number is.

## Rounds

### Round 1 — build
Built the ground from today's river (OpenStreetMap through the API's map call; the camps, headland,
island, field and the hours authored, marked so), the art sheet (the valley by night and by day,
the camps, column, boats, waders, horse, phalanx, chariots, elephants, rain, lightning, doorway), the
script (everything a sprite on a dated way; a drag on any of them scrubs the night along the chain of
ways). Raised the runtime's zoom limit to 2048 (a valley a few kilometres across), gave the art
drawer `--with` for built data, made the moments tool settle frames before a shot. Looked in the
preview at eleven moments on the iPad and three on the phone.

**The fresh eye** (a critic with only the shots and the pitch): the bones are right and the dusk camps
are close; the rest is halfway. The night is flat (one navy, one brown, the river a smudge with no
surface, no light from the fires on the water, no relief); the storm is a tile of strokes with
visible edges, no flash; the waders, the hero moment, are fourteen identical busts on a grid with
dinner-plate shields and grey arcs for water, and the ant-sized column stands beside them; the column
is indistinct dots at the march scale and a muddle with the boats at the bend; the battle is stiff
rows; the dawn is a desaturation; the tip never changes. Verdict: halfway to the bar.

### Round 2 — the night and the waders
The river got a surface (fine streaks running with the current in the moon's light, foam fretting at
the banks, a wet margin of mud instead of a blurred halo; the flood spreading round the island over a
long tapered stretch rather than a pool), the hills a wavering foot and brighter ridges, the camps' fires
a glow on the water nearest them. The rain became a round curtain with no edge of its own, the
lightning a wash over a bigger field. The waders were drawn by hand: no two alike, the near rank to
the chest, some to the shoulder, one or two with only a shield and the crown of the head above the
water, a horse's head thrown up, a man stumbling with his shield going over, a shield gone floating;
the current drags pale streaks downstream from every body. The column from afar is a thin line of pale
points, nearer a readable file with a horse at its head. Porus' line lies across the line of advance
with every figure upright and facing the landing, drawn in the high view's order; the phalanx closes
to a pike's length. The morning is warmer, with a wash of light from the east. The tip changes after
the touch and goes after the first drag. A near band of looks between the mid and the close zooms
keeps a camp from swallowing its bank. The ground pictures are held to 64 levels a channel by the
drawer's new `shrink` (`tools/packart/shrink.py`): 31 MB became 10. Pictures reordered so the night
thins the same inside the close picture as outside it.

Looked again at the camps at dusk, the march in the storm, the bend, the waders, the half-light and
the battle. Still mine to see: the dawn's half-light shows the close picture's soft rim as a lighter
halo while the night thins (narrowed; not gone); the battle is rows meeting rows.

### Round 3 — the second fresh eye
The second fresh eye (new shots, no memory of the first): the bones read without a word and the
firelight on the water between the camps is the atlas-child feeling; but the mistake can't be seen (the
island is a dark smudge in one brown ribbon, no second channel, no far bank), the waders break the
world's scale (seven large discs on sticks beside six-pixel marchers, no water over them), the night is
dark rather than night (fires are blurs, tents white icons in a grid, no moon on the current), the dawn
goes to khaki mud with hard-edged woods, the frontal elephants read as owls; a grey ring sat in every
shot from the storm on (the lightning's wash), a straight edge crossed the close picture (the hills'
foot), the elephants bridged the river from afar, the column stood on the water at the bend, the end of
the battle was a tangle, the tip went stale after a strip move. Verdict: 40 % of the way; the one change
that moves it most is the bend as two channels round a readable island with the wading staged in the
far channel as a chest-deep crowd at the column's own scale.

### Round 3 — the island, the crowd in the channel, the night's light
The bend is two channels round a pale island now: sand above the flood with a strand of foam and a
grove in its middle, water drawn either side; the column waits in the grove on land, not at the water.
The waders are a crowd at the column's own scale (thirty-odd, three staggered ranks, the far rank
deeper, four horses' heads), the river's own tone laid over each body's lower half so what shows is
what rises out of the flood, a shield adrift. Fires are hot points with a small halo; tents are dimmed to
the night and lit on the fire's side, staggered. The elephants stand on the bank in side view facing
the far camp, one raising its trunk now and then, and show from the valley zoom in. The rain is
curtains. The dawn is grey-blue. The lightning is a faint wash with a bolt, not a disc. The sweeps end
wide of the line. The tip goes when the strip moves the night on. Every mark on the ground is placed
in longitude and latitude from one seeded scatter, so the wide and the close picture are one scene at
two resolutions: the close one is a disc with a short rim, the wide one has the matching hole, and the
night is never laid twice over one place (the halo while the night thinned is gone; a faint seam
remains if you look for it). The wide picture is 3780 across (its rows just under the globe's 4096).
Pushed with the UI test (a drag on the column moves it; the strip at the channel shows the waders).

The third fresh eye (new shots): the night flat (a navy pool, a tan road for a river, fires glowing
in a vacuum), the wade the smallest thing on screen, the column a block in the plain, the dawn a wash,
the elephants loaves on sticks; the one change that moves it most: make the water an actor. So: the
river near-black with sparse silver threads and a dark edge to the land; the fires lie on the water
in long broken streaks that flicker; the island is sand with a dark water's edge and no rim; the
elephants have a domed back, an ear, a trunk, a tusk and an eye; the column is a file of two long
ranks with the horse at its head; the horse and the chariots raise dust; the waders' back rank stands
dry on the island's shore, so the depth is told by comparison; the globe comes close on its own, once,
when the night reaches the second channel (after a drag ends, or as play reaches it). The pebble mottle
is near-invisible. Seen in the preview: the camps at dusk are now a night with a river in it.

Two notes from the Interviewer, read late (they were queued): the critic is to be a hater (a famously
hard-to-impress reviewer who thinks most of the globe is slop, ranking complaints by how much a player
would notice; the loop ends when its worst complaint is a quibble; a complaint may be overruled here,
with the reason), and the globe's first sign of an experience should read as ordinary life slightly
off, not an attraction: no glow, badge or label. So the doorway is no longer a disc of night with
fires on it; it is an elephant standing at the water on the Jhelum's bank, stirring a little way along
it and back, which doesn't quite belong on that bank today.

### Round 5 — the hater, and the real app

**The hater** (a fresh eye told it is a famously hard-to-impress reviewer, ranking by what a player
would notice), worst first: the scale lies at every zoom (ten elephants spanning the river at the camp
zoom, camps the size of cities), the sprites are flat identical stamps with no three-quarter view and
the two camps told apart only by the elephants, the wade doesn't wade (discs on legs on a lavender
decal, two scales on one line), the column is a straight bar that ignores the river and the headland
is a grey outline with no trees, the night is navy with a tar river and dawn a palette swap, words
do the picture's job, the battle is a counter diagram, the opening screen is empty, the rain's bands
are stepped, the doorway gives the elephant away, the boats are too small to push. Worst complaint:
the stamps at a lying scale. A real failing.

Taken: the drawn pictures now show only from the close zoom (a kilometre at 48 points); between the
valley and that the camps are fires and the elephants grains at the water, as the pitch has it. Tents
are in a high three-quarter view, no two the same size, their faces lit on the fire's side, and Porus'
camp is told apart: ochre pavilions with pennants, chariots parked. The elephants vary in size and
pose. Close up the column is a thread of nine clusters a few minutes apart along the same way, so it
bends with the way and the headland as a file does, the horse at its head. The wade has a drawn
current the length of the channel, every figure cut by the water (the leftmost at the knee, wading
in from the island's shore), shields up with a highlight on the rim. The island is a warm grey, not
lavender; the grove's trees are big enough to be a mass. The rain's bands are strokes. The boats are
larger and leave wakes, and the globe comes close on its own for the crossing as it does for the
wade. The words at the foot are gone: the title alone; what is touched still says its one line.

Overruled: the doorway stays an elephant at the water (the owner's rule, read tonight: the first sign
of an experience is ordinary life slightly off, not an attraction; that it gives the elephant away is
the point); no moon (Arrian's night was a furious storm); the dark plain at the opening is the night,
and Play moves it; the camps' fires are not gone at dawn, they are off the screen when the shots look
at the field.

### Round 6 — the second hater

Worst first: the dawn a pink disc lying on the ground (the morning's light as a sprite: read as a
fault), the night dim rather than dark with a river carrying no light, the elephants identical stamps
and the phalanx walking into a line the size of a county, the rain tiles (fixed between its shots and
its report), the waders floating, the island a bandage and the headland four dots, the river on fire
with sparks, the finger finding nothing to say touch me, the battle a shrug, the dawn mud. Worst: the
night. A real failing.

Taken: the dawn sprite is gone; the morning is a warm wash from the east drawn into the day pictures,
over the wide box so both pictures agree. The night's river is a dark blue-grey sheen between black
banks (the sky's light on the flood), the threads sparse and bright, the camps' fires on it in fewer,
softer streaks; the camps' ground is feathered. The island is a mottled sandbar with scrub and a dark
water's edge, no rim; the grove is a mass of bigger trees with no outline. Porus' line is smaller, so
the phalanx and the horse have room to meet it. The waders' bodies stop at the water exactly, the
water over them the river's own morning tone.

Overruled: the column marches away from the river, inland (Arrian: the picked troops kept far from
the bank so as not to be seen; the drag still follows it, and the way is what the finger traces); a
trace under the finger (the column is the trace); sentries pacing to say touch me (the figures walk
among the fires already; the owner's rule is no sign that says tap here).

### Round 7 — the third hater

It gave the dusk camps and the file snaking inland the best frame on the globe, then: the day kills it
(a flat khaki wash, blurred hills, a sausage river, at the moment the story pays off), the battle a
sprite sheet (identical elephants stacked, pikes through their bodies, nothing turning), the island
mistake invisible (a ruler-straight block overrunning a parade-ground island), the waders peppered
across the whole river and onto the sandbar, the rain smeared bands, the boats a clump, the camp's
elephants a fence of ten, the night river lavender and the fires confetti, the chariots illegible from
the middle distance, the opening a void, the phone's chip over the pool. Worst: the day. Real.

Taken: the morning is a lighting change over the same ground, cold blue-grey with the river keeping
its brown water and its sheen and the banks their shadow, the east warm; the night's river is muddy
and dark with the sheen kept; the plain has faint fields near the river. The battle line has six
elephants with room between them, varied, and the phalanx stops a pike's length short. Across the
island the column is a thread of clusters too, so it halts at the water's edge; the second channel
has its own foam. The waders are a bunched band across the one channel, the way the ford runs, the
first few at the knee, the middle deepest, three horses nose-up. The boats are spaced, each with a
wake. Five elephants stand at the water, one turned to the viewer. The rain is threads with the
faintest bands. The chariots from the middle distance are chariots.

Overruled: the phone's chip over the pool (the app's chrome, every pack's); the unlabelled strip (the
atlas's date strip, every dated pack's).

### Round 8 — the fourth hater

It called the close camps the best frame on the globe and the boats and the wakes drawn by someone
who looked; then, worst first: the first sight is a map legend (tent icons at a counter's size where
the pitch promises two camps of tiny fires), the dawn and the battle a deployment diagram with no sun
and no shadow, the night hides nothing (the column as bright as the camp it left), the storm's blocks
and a dot screen on the eastern bank, the headland and island pasted on, orange dashes on both banks,
the wade too small and too dry, the chariots five dots from the middle distance, the elephants never
moving in a still, the phone's halo badges (a quibble). Worst: icons instead of fires from afar.

Taken: from afar a camp is only its fires, pinpricks of warm light breathing, no tents until the
close zoom; the night march is a dim grey thread, cloaks over everything, no gold, the column from
afar grey points; the fires' light lies narrower on the water, river side only; the island is mostly
trees with a thin strand; the fine mottle is only in the close picture; the morning's low sun lays a
soft shadow west of every formation. The camps' fires from the valley are a little larger.

Overruled: the elephants never move (they sway and one raises its trunk; a still can't show it); the
phone's halo badges; the chariots at the zoom the reader chose (the globe leans in only twice, for the
crossing and the wade, and the reader's zoom is theirs). Still mine to see: the battle is formations
on a field; its motion (the sweep, the turn) is in time, not in a still.

### Round 9 — the fifth hater

It gave the dusk camp the best still on the globe tonight and the phalanx and the elephant line real
counters; worst first: the river is not a river (a road on a road map by night, scratches by day) and
the island shows from the start, so the mistake has no surprise; the night march a void with pawns;
hard polygon seams across the ground; the day dead; cheap sprites where it matters (the reflections a
barcode across a diagonal river); the rain stamped patches; the first sight inviting nothing; the
column's leaving easy to miss. Worst: the river, and the island shown before the column finds it. Real.

Taken, and the best of the night's ideas came from it: the dark over the second channel. A small
picture in the ground's own night tone joins the island to the far bank, laid over the close picture
until the column reaches the island's far shore, then thinning over a quarter of an hour as the river
opens in front of them; it is only as dark as the night is. Before, the island reads as a headland of
the far bank; after, as an island in the river with the file wading the channel beyond. The river is
brown water again, dark by night with a faint sheen, mud-brown by day, the threads fewer; the night's
woods can just be seen; the hills' foot is a wide soft skirt wavering on every segment; the morning's
ground is the Punjab's grey-green; the fires' streaks lie along the river's own run at the camps; the
rain is six wider, fainter fields along the plain's edges; the opening is the two camps at the centre
with the river between them, the bend a drag away.

Overruled: a peel of torches leaving the camp (the march was dark on purpose; the thread of grey is the
reward, and the drag finds it); the elephants as masses until close (they are grains until the close
zoom already).

### Round 10 — the sixth hater

It gave the camps the one thing that earns the pitch; worst first: the daylight a placeholder (khaki
with blotches, the storm a grey half-plane with a dead-straight edge, the river a tan road), a
hard-edged dark wedge out of the landing, the night's river the brightest thing in the night with a
rapids icon of yellow hatching under no fire, the rain as spotlights, picture-book elephants, the first
sight giving the armies away, the mid-zoom elephants a dotted rule, the battle scale-less, the night
march too dim to be beautiful. Worst: the daylight ground where the battle plays. Real.

Taken: the grey half-plane was the hills' fill; the hills are ridges and a faint slope now, no plate,
by night and by day. The river is dark and wet below the land in both lights. The dark over the
channel reaches the far bank and no further (the wedge was its overshoot). The elephants are darker
masses without an eye; from the middle distance five masses of unequal size, not a rule. The fires'
light lies at each camp's own bank. The rain is sheets, its fall-off stretched along the fall. The
night march keeps its dimness but every helmet has a glint. The opening is a step wider, so the camps
are fires first.

Overruled: the battle's scale (counters); the island surprise judged from a still at the landed
moment, when the dark over the channel has just thinned (the surprise is in time: the column arrives
on what reads as a headland, and the water opens as it does).

### Round 11 — the seventh hater

It gave the dusk camps the best frame on the globe and the shield drifting off downstream the one
detail that shows somebody thought like a player; worst first: the island and the waders, the very
moment, drawn worse than the camps (a teal polygon with a hard ragged edge, coins for men), the dawn
mud, three projections in one army (three-quarter tents, plan boats, side elephants, top-down files),
the day ground flat with a black-outlined river, the rain dashes in an oval, the rafts beige blobs,
the pool's rim crisp in one shot and soft in another, the elephants five copies. Worst: the island and
the waders. Real.

Taken: the globe leans deeper on the iPad for the crossing and the wade (ninety points a kilometre;
the phone was at the globe's limit already), so the men in the channel are bodies, not coins; the
dawn thins more slowly, so the dark over the channel holds while the column is on the island, and
that dark is a dusk tone between the two grounds, so it sits in the half-light; the island's edge
and the day's banks are a little darker than the water, not black; the rafts are a row of stuffed
hides lashed to poles with two paddlers and a horse; two of the elephants carry mahouts; the rain's
smear under the threads is fainter; the pool's rim is one softness.

Overruled: one projection for every counter (the camps are the three-quarter view the pitch names;
the boats are seen from above because a boat on a map is; the elephants and horse in profile because
that is how they are told apart at a counter's size; a redraw of every animal to the three-quarter
view is the next round of art, not this night's).
