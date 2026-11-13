# Moon Road: sources

What the toy follows and what it draws. Moon Road loads no data: the lake, the moon and the sky are made
by code.

## The rules it follows

- **The glitter path.** A low moon (or sun) over rough water is seen as a road of light because each little
  facet of the surface sends its light to your eye only when it is tilted so that its normal halves the angle
  between the moon and your eye; the road is where such tilts are common, so it always runs from the moon
  to the viewer, and everyone on a shore sees their own. Its width grows with the spread of the slopes: the
  rougher the water, the wider the road. M. Minnaert, *Light and Colour in the Outdoors* (Springer, 1993),
  the chapter on reflections on water; the slope statistics after C. Cox and W. Munk, "Measurement of the
  roughness of the sea surface from photographs of the sun's glitter", *Journal of the Optical Society of
  America* 44 (1954) 838–850. `lake.rs` tests each facet's tilt against the one that sends the moon to the
  eye.
- **The slopes.** A calm night's rms slope of about 0.06 on each axis, a little less than Cox and Munk's
  light-wind figures for the open sea (a lake's short fetch); a gust raises it locally, as a cat's paw does.
- **The place.** Lake Superior from the beach at Marquette, Michigan, looking east-north-east over open water,
  where the moon rises out of the lake.

## Drawn

The template's cells (about a facet's worth of water each, fixed to the water and foreshortened as the
surface is), the paints and their numbers, Paint by Moonlight's plum night and cream paper, the moon drawn
larger than its half a degree, the haze, the point of land, the swell's period and length, the sound.
