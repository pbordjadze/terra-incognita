# Hoh: sources

What is measured, what is drawn, and from where. No data file is bundled: the stand is grown and drawn
from code (src/), by the rules below.

## The place (measured)

- **The Hoh Rain Forest**, in Olympic National Park on Washington's Olympic Peninsula, below Mount Olympus;
  its trailhead and visitor center are at the end of the Upper Hoh Road (National Park Service, *Visiting
  the Hoh*). The toy's `PLACE` is Wikipedia's coordinates for the Hoh Rainforest, 47.8614° N, 123.9247° W.
- **Its rain**: on average 140 inches (3.55 m) of precipitation a year (NPS, *Visiting the Hoh*); 140 to
  167 inches in Olympic's rain forests (NPS, *Temperate Rain Forests*).
- **Its trees**: the dominant species are Sitka spruce and western hemlock; many are hundreds of years old
  and reach 250 feet (NPS, *Temperate Rain Forests*). Here a tree tops out between 53 and 74 m, each at
  its own height, and grows at its own vigour.
- **Moss**: in the Hall of Mosses, maples are draped with club moss (NPS, *Visiting the Hoh*). Here the
  beards hang from the conifers' branch stubs, and the floor is a carpet of moss with sword ferns.

## The rules (measured, simplified)

- **Nurse logs**: "Because of the densely covered ground, many seedlings instead germinate on fallen,
  decaying trees" (NPS, *Temperate Rain Forests*). A log offers a seedling water, freedom from the moss's
  thickness and leaf litter, nutrients, light, and some protection from the soil's pathogens (Wikipedia,
  *Nurse log*). Here seed lands on the whole floor every year; on the moss a seedling is gone within a year
  or two (`Moss smothers`, the first switch), on a log it lives.
- **Colonnades**: "As they grow, their roots reach to the ground. When the log eventually rots away, a
  colonnade, or row of trees on stilt-like roots, remains" (NPS, *Temperate Rain Forests*). Here a tree's
  feet stay where the log's top was when it rooted, and its roots are drawn down from there to the floor.
- **Decay**: fallen giants "can take centuries to slowly decay back to the soil" (NPS, *Temperate Rain
  Forests*). Here a log keeps its shape for fifty years, then sinks over the next two centuries (260 years
  in all), and its root wad goes in its first 160.
- **Light and shade** (drawn, a common rule of forest gaps rather than a measured one): a crown over 18 m
  shades the floor round it; young trees grow fast in the gap a fallen giant leaves and hardly at all
  under closed canopy, and a seedling overtopped by a taller neighbour close by for some years dies
  (`Shade thins`, the second switch), so a log's crowded line thins to a few.
- **Falls** (hurried and thinned): a giant falls by itself about once in six hundred years here, one at a
  time and only in view, so that the stand renews while one watches; a tap fells one. Its log lies where
  its trunk fell, 78% of its height long.

## Time

The years run at one a second at rest; a drag runs them faster, up to sixty a second, so the centuries a
colonnade takes pass in a few swipes. The opening stand is grown, not drawn: old growth planted 480 years
back, with three giants felled at chosen years (330, 110 and 14 years before), so the first look holds an
old row on stilts, a log of a century with its young trees, and a fresh one with its first line of
seedlings (src/history.rs; `cargo run -p hoh --release --example census` and
`tools/toys/hoh/map.py` show it from above).

## The sound (drawn)

The forest's hush (filtered noise) and drips; a bird's long whistle on one pitch now and then, the next on
another, after the varied thrush of these forests; a giant's crack as it goes and the thud of its landing.
None of it recorded.

## Pages

- National Park Service, *Visiting the Hoh*: https://www.nps.gov/olym/planyourvisit/visiting-the-hoh.htm
- National Park Service, *Temperate Rain Forests*: https://www.nps.gov/olym/learn/nature/temperate-rain-forests.htm
- Wikipedia, *Hoh Rainforest*: https://en.wikipedia.org/wiki/Hoh_Rainforest
- Wikipedia, *Nurse log*: https://en.wikipedia.org/wiki/Nurse_log
