# Magnet Marsh: sources

What the toy follows and what it draws. It loads no data: the drop and its bacteria are made by code
(src/drop.rs; plotted by src/plot.rs, filmed by src/sc4020.rs). The field is the one the phone feels.

## The place

- **Woods Hole, Massachusetts** (about 41.5° N, 70.7° W), where Richard Blakemore, a graduate student,
  looked at mud from the salt marshes nearby under the microscope in 1975 and saw that some bacteria all swam
  to the same side of the drop, and turned when a magnet was brought near. (R. P. Blakemore, "Magnetotactic
  bacteria", *Science* 190, 377–379, 1975.)

## The rules it follows

- **Each cell carries a compass.** Magnetotactic bacteria make a chain of magnetite (Fe₃O₄) crystals, the
  magnetosomes, fixed along the cell, so the field turns the whole cell as it would a needle (Frankel and
  Blakemore, "Navigational compass in magnetic bacteria", *J. Magn. Magn. Mater.* 15–18, 1980). The chain's
  moment is around 10⁻¹⁵ A m², so in the Earth's field its magnetic energy is several times the water's
  thermal energy: most cells line up, not all, and the more field the better they line up. `drop.rs` turns
  each cell toward the field at a rate set by that ratio and lets the water turn it at random as well. The
  first switch takes the compass away: nothing turns them, and they scatter every way.
- **They swim along it.** The compass only turns them; it is their swimming that carries them along the field
  lines, at tens to a couple of hundred micrometres a second (the cocci are the fast ones). In a drop they
  swim until they reach its edge and gather there, in a band on the side the field points to: Blakemore's
  first sighting. The second switch stops them swimming: they still turn to north, but go nowhere.
- **North-seeking in the north.** Northern cells swim with the field (north, and down along its dip, into the
  mud); in the Southern Hemisphere they mostly swim the other way (Blakemore, Frankel and Kalmijn,
  "South-seeking magnetotactic bacteria in the Southern Hemisphere", *Nature* 286, 384–385, 1980). A few
  per cent of a northern sample seek south; so do 3 % here.
- **The field is real.** On the phone, the field across the drop is the phone's own calibrated magnetometer
  (Core Motion's `magneticField`, in µT): the Earth's at your place, as it crosses the screen, plus any
  magnet near the phone. A fridge magnet held to the phone's edge is hundreds of µT there, so the cells pour
  toward it. A finger is a magnet's south pole held over the drop. Without a compass (a desktop browser) the
  field is the Earth's horizontal 20 µT pointing up the screen.

## What is the toy's own

- The drop is drawn about 1.5 mm across; the cells are drawn far bigger than their 1–3 µm, as the bright
  points a dark-field microscope shows, with a quarter second's streak behind each.
- The water's turning of a cell (rotational diffusion, 0.18 rad² a second) and the chain's moment (kT/m at
  3 µT) are chosen within the published range so the alignment reads, not measured for one species.
- The finger's magnet is a pole of 900 µT at 30 points, a strong fridge magnet's.

## The look

Drawn in the style of Frank W. Sinden's *Force, Mass and Motion* (Bell Labs, Murray Hill, New Jersey, 1965),
a film of bodies moving under force laws, made on the laboratory's Stromberg-Carlson SC 4020 microfilm
recorder.

- **The film:** "Computer-animated film 'Force, Mass and Motion'", Frank W. Sinden, 1965 (ETHW, *Timeline of
  Early Digital-Graphics Innovations at Bell Telephone Laboratories*). It "demonstrates how gravity-like
  forces and inertia affect bodies with various initial velocities", and was "produced using the standard
  SC4020 camera without pin registration" (content-animation.acorn-web-design-wantage.co.uk, *early
  examples*). "Most of the films show the jitter on the image due to the lack of pin registration in the
  Bell Labs camera" (the same site, *early films*).
- **The machine** (*S-C 4020 Computer Recorder Information Manual*, August 1964, bitsavers.org): a point
  "can be positioned to any of 1024 positions vertically or 1024 positions horizontally"; "the vector
  generator makes it possible to connect any two addressable points with a straight line made up of line
  segments… The maximum vector length is 1/16th of maximum deflection"; a 35 mm or 16 mm camera
  "positioned above the tube face". So here every mark is a straight vector or a point: the drop's rim a
  96-sided polygon, a spirillum a chain of six, a protist a 20-sided outline (`plot.rs`); the frame is read
  at 1024 positions across and each frame sits off in the gate by a little, differently every frame
  (`sc4020.wgsl`; none with Reduce Motion).

**Drawn, not sourced:** white lines on black (no source found gives the screening print's polarity); the
beam's width, the halation and the grain; the second, fainter ring inside the rim; a cell out of focus
plotted dimmer; one cell in three plotted, each with its places of the last six eighths of a second left as
points behind it; the finger's pole as a circle and a cross; 24 frames a second.
