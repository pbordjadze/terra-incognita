# Lamplight

A runtime-4 globe pack (`docs/runtime3.md`): evening at the Himalaya's edge. It asks the app for the
globe's own night (`terra.globe.set({ night: true })`, the Earth at night of painting-company row
174) and lays over the mountains and the plains below them the same night drawn closer: the relief
and the snow by moonlight, the plains' lights as the satellite saw them, the setting sun's last light
on the high snow, and lamps where people live in the hills, which the satellite barely sees. An app
without the night gets a pool of evening the pack draws itself (`pool.png`). Every picture is built
by `tools/lamplight/build.py` (its docstring is their layout); never hand-edit them. The question it
answers is painting-company row 173's: are there villages high in those mountains, or are they all
in the foothills?

## What is measured, and what is drawn

- **The plains' lights are measured.** NASA's Black Marble 2016, the 500 m colour and grey tiles C1
  and D1 (NASA Earth Observatory, *Earth at Night 2016*, records 144898 and 144897: images by Joshua
  Stevens, using Suomi NPP VIIRS data from Miguel Román, NASA Goddard Space Flight Center;
  https://earthobservatory.nasa.gov/features/NightLights). NASA imagery is in the public domain;
  credit as given. Drawn as the globe's night draws its own lights (Globe.metal's `N_` palette).
- **The hills' lamps are drawn, from a measured map of where people live.** GHSL GHS-POP R2023A,
  epoch 2020, 3 arcseconds (European Commission, Joint Research Centre; Schiavina, M., Freire, S.,
  Carioli, A., MacManus, K., 2023, *GHS-POP R2023A, GHS population grid multitemporal (1975-2030)*,
  https://doi.org/10.2905/2FF68A52-5B5B-4A22-8F40-C41DA8332CFE), CC BY 4.0. A lamp for every 150
  people, placed by chance where they live (seeded: the same build draws the same lamps); a quarter
  of them from further off (`lamps-1..3.png`, by height: below 1,500 m, to 3,000 m, above), all of
  them close in (`detail.jpg`). Left out on purpose: the plains (the satellite's lights are there:
  a square needs 350 m of height and 200 m of relief within 3 km, or 2,000 m of height), anything
  above 4,900 m, and anything above 3,700 m on a slope steeper than 18 degrees. GHSL spreads people
  onto glaciers and bare rock where its built-up layer misreads them (it puts thousands beside
  Everest at 6,000 m); the year-round villages stop at about 4,600 m.
- **The relief, the snow and the alpenglow are drawn from the elevation model.** Copernicus DEM
  GLO-90 (https://registry.opendata.aws/copernicus-dem/; produced using Copernicus WorldDEM-90,
  DLR e.V. 2010-2014 and Airbus Defence and Space GmbH 2014-2018, provided under COPERNICUS by the
  European Union and ESA), under the Copernicus DEM licence. The moon from the south-west, the snow
  above about 5,100 m, the setting sun from the west-south-west 6 degrees up; the highest peaks
  (above 7,000 m) keep its light longest. A moment drawn, not a measurement.
- **The highest villages' lamps** (`villages.js`): villages lived in through the winter, each where
  OpenStreetMap puts it (Nominatim, ODbL), its height measured on the same elevation model and
  rounded to 10 m. Their lines, shown only when a lamp is touched:
  - Karzok: 1,291 people in the 2011 census, a permanent village (Census of India 2011, via
    https://en.wikipedia.org/wiki/Karzok).
  - Komic: 114 people in 12 households, there through the winter (Unnati Sharma, *Village Square*,
    12 July 2024, https://villagesquare.in/worlds-highest-hamlet-what-its-like-to-live-at-4500-metres/).
  - Hikkim: its post office closes about six months a year
    (https://thebetterindia.com/117789/hikkim-spiti-srinagar-antarctica-unusual-post-office-india).
  - Kibber and Spiti through the winter: J. P. Kapoor and R. K. Bhagat, "Resource potentials of
    Spiti", *Annals of Arid Zone* 29(4), 1990, p. 244.
  - Chushul (949) and Hanle (1,879): Census of India 2011, via Wikipedia.
  - Dolpo's winter descent with yaks: *Kantipur Koseli*, 6 December 2025.
  - Dho Tarap, "considered the highest inhabited area in Nepal": *Onlinekhabar*, 14 June 2017
    (https://english.onlinekhabar.com/meanwhile-in-nepals-highest-settlement-time-stands-still.html).
  - Lo Manthang, "Almost all except aged people have left the village with the beginning of winter":
    *The Himalayan Times*, 10 November 2018.
  - Dingboche, Pangboche, Khumjung: S. F. Stevens, *Claiming the High Ground* (University of
    California Press, 1993), pp. 41-46.
- **The panel's numbers** (half of Nepal on the plain, a third between 1,000 and 2,500 m, one in 700
  above 3,000 m) are GHSL 2020 measured against the same elevation model; C. L. Chidi, "Human
  Settlements in High Altitude Region Nepal", *Geographical Journal of Nepal* 7 (2009) 1-6, counted
  51,486 people above 3,000 m from Nepal's survey maps.
- **Pictures**: `lamp-1..6.png` (a lamp's flicker) are drawn from code by `build.py`;
  `doorway-glint.png` is the shared glint (`tools/playground/glint_png.py`).
