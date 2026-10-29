# The night crossing: sources

What is drawn in this pack, and where each part comes from. Nothing on the screen says any of it;
the chip's Open shows the panel (`notes.js`), which says it to the reader.

## The story

- **Arrian, *Anabasis of Alexander* 5.8–19** (E. J. Chinnock's translation, 1884, Project Gutenberg
  46976). Everything that happens: Porus on the far bank with his elephants (5.9); the clamours and
  fires along the bank night after night until Porus stops answering them (5.10); the wooded headland
  at a bend, the uninhabited island full of trees before it, 150 stades from the great camp, and
  Craterus left there with his orders (5.11); the picked horse and foot marching far from the bank, the
  hides stuffed with hay, the galleys cut in pieces and put together again in the wood, the storm of
  rain whose thunder covered the noise, the wind and rain calming at daybreak (5.12); the landing on
  what proved to be an island, the second channel swollen so the foot waded to the chest and only the
  horses' heads showed (5.13); 6,000 foot and 5,000 horse, Porus' son's chariots (sixty by
  Aristobulus; 2,000 horse and 120 chariots by Ptolemy, whom Arrian follows) (5.14); the clay, Porus
  marching with 4,000 horse, 300 chariots, 200 elephants and 30,000 foot, the elephants a plethron
  apart in front (5.15); Alexander against the left with the horse-archers ahead, Coenus round the
  right to the rear (5.16); the Indian horse driven onto the elephants, the elephants cooped in,
  trampling their own, backing "like ships backing water" (5.17); Craterus crossing, the losses (5.18).
- **Curtius 8.13**: the man dressed as the king left in the camp (the decoy): not in Arrian.
- **Diodorus 17.87, Plutarch *Alexander* 62**: the other counts of Porus' army, in the panel.

## The ground

- **The river's course**: OpenStreetMap (© OpenStreetMap contributors, ODbL), the Jhelum between
  Jhelum city and Jalalpur Sharif: the water as mapped (the river's multipolygons and ways), its
  centreline, the torrents off the Salt Range (the Kahan among them), the riverine woods and scrub.
  Read by `tools/hydaspes/build.py` from the API's map call and written to `tools/hydaspes/terrain.js`
  (what the art sheet draws from) and `data.js` (the ways). The river in 326 BCE ran elsewhere;
  this is a drawing after today's river.
- **The site**: Aurel Stein, "The site of Alexander's passage of the Hydaspes and the battle with
  Poros", *The Geographical Journal* 80 (1932): the great camp at Haranpur, the headland and island at
  Jalalpur. The other reading (near Jhelum, where the Uttarapatha crossed; the river's old beds far from
  Haranpur and Jalalpur): Livius.org, "Hydaspes (326 BCE)". The panel gives both.
- **Authored, not read** (marked AUTHORED in `build.py`): the camps a kilometre back from the water
  on each side, the headland's grove and the island placed on today's bend by Jalalpur, the second
  channel, the march route inland behind the hills' skirt, the level field, Porus' line and the sweeps
  of the horse, the hills' skirt itself (`art.js`, the Salt Range's foot north-west of the river), and
  the hours of the night (dusk to noon).

## The pictures

All drawn from code by `tools/hydaspes/art.js` through `tools/packart/render.cjs` (with
`--with tools/hydaspes/terrain.js`): the valley by night and by day as pictures on the globe; the camps
in a high three-quarter view; the column, the galleys and hide-rafts, the soldiers wading, the horse,
the phalanx, the chariots, the elephants and Porus' line; the rain and the lightning; the doorway. No
photograph, no scan, no hand-made picture. The armies are pictures, not counts, and far larger than
the ground they stood on.
