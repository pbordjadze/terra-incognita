# Meander: sources

What is measured, what is modelled, what is drawn (painting-company row 218).

## Measured (data)

- **The floodplain**: Copernicus Sentinel-2 L2A true colour (TCI, 10 m), tiles 19LFF and 19LGF of
  15 August 2024 (cloud under 0.01 %), read from the AWS open-data bucket `sentinel-cogs` (Element 84's
  earth-search). Licence: Copernicus open data (free, full and open); credit: contains modified Copernicus
  Sentinel data 2024. `tools/toys/meander/build.py` cuts a 22 km square of the Rio Beni's meander belt
  centred near 67.03 W, 12.97 S (Beni department, Bolivia, between Rurrenabaque and Riberalta), turns
  it 29 degrees so the belt runs up the screen (the river flows north: up), and writes
  `toys/meander/data/plain.jpg`. The dark crescents (oxbow lakes), the faint arcs (old scroll bars), the
  forest of the belt and the pastures on the terraces either side are the photograph.
- **Today's river is taken out**: the channel and its bare sand bars (bright, warm pixels joined to the
  long channel) are filled from the forest round them (their low tones blurred in from outside, the
  detail copied from real forest nearby). A faint trace of where it ran is still visible; nothing else in
  the picture is changed beyond a mild contrast lift in the shader.

## Modelled (the science)

- **Bank migration**: A. D. Howard and T. R. Knutson, "Sufficient conditions for river meandering: a
  simulation approach", Water Resources Research 20 (1984), as written in Z. Sylvester, P. Durkin and
  J. A. Covault, "High curvatures drive river meandering", Geology 47 (2019) and their `meanderpy`
  (Apache 2.0; read, not copied): each point of the centreline moves sideways at
  `R1 = omega R0 + gamma * (R0 upstream, weighted by exp(-alpha xi))`, `R0 = kl W C`, with
  meanderpy's omega = -1, gamma = 2.5, kl = 60 m a year, Cf = 0.022. Loops grow, skew and travel
  downstream because of that rule alone. Where two parts of the river far apart along it come within
  1.1 widths of each other, the neck is cut and the loop becomes a lake (the neck cutoff of Camporeale,
  Perona and Ridolfi and every such model).
- **The Beni**: a width of 260 m (its channel is 200-400 m wide in this reach); lateral migration of
  tens of metres a year in its bends, as measured from Landsat (E. Gautier, D. Brunstein, P. Vauchel,
  M. Roulet, O. Fuertes, J. L. Guyot, J. Darozzes and L. Bourrel, "Temporal relations between meander
  deformation, water discharge and sediment fluxes in the floodplain of the Rio Beni (Bolivian Amazonia)",
  Earth Surface Processes and Landforms 32, 2007); the model's rates come out in that range.

## Drawn (not measured)

- The pace: a second is seven years. The depth in the model (16 m, setting alpha and so the loops'
  size) was chosen so the bends are the size of the photograph's old loops at this width; the beginning
  wobble a drawn course is given (a few waves of 2-4 km), so the first oxbow comes within about ten
  seconds; a whisper of random noise on the banks; light smoothing of the line every step; the belt's
  edges (a bank outside the forest belt erodes a quarter as fast, as older cohesive ground does).
- The colours: the water's café-au-lait (a white-water river carrying the Andes' sediment, as the
  photograph shows it), the wet-sand bank, the bare sand of a new point bar greening to cane and back to
  forest over a few hundred years, its scroll-bar ridges (one every seven years or so), a cut lake
  clearing to black water in a few decades, its ends silting shut, then narrowing to marsh and fading.
- The flecks of light riding the current, the ring where a neck breaks, the struck notes and the haptic
  tap at a cutoff: a way to see and feel the event, not measured things.
