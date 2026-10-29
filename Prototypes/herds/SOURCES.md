# The herds: sources and preparation

A runtime-5 pack (`docs/runtime3.md`): the camel family's history as small herds that move across the
globe with its date. Each herd is one lineage of the family (a population over many generations,
drawn as a few animals), each animal a sprite on a dated way, so the globe's date moves it. Its
script (`main.js`) runs in JavaScriptCore and draws nothing itself. Everything is in the pack: no
network, no storage, no record of the reader's actions.

Everything here except `main.js`, `notes.js`, `prototype.json` and this file is generated; rebuild,
never hand-edit:

- `data.js` and `beringia.png`: `python3 tools/herds/build.py`, from the lineages in
  `tools/herds/lineages.py` (each turn's source is beside it there) and the globe's own maps
  (`Globe/geo.png`'s coast and `Globe/terrain.png`'s depth, built by `tools/globe/build.py` from
  Natural Earth and GEBCO; `Globe/ground/07.jpg`, NASA's Blue Marble, for Beringia's colour).
- `art/` and `art.js`: `node tools/herds/draw.cjs`, which draws every animal from parameters (no
  source picture is used).
- `doorway-glint.png`: `python3 tools/playground/glint_png.py`, the mark every doorway uses.

## What is known, and what is drawn

- **The turns** of each herd's way are fossil sites, or places within the regions and spans the
  sources give for a lineage; the ways between them are drawn, meandering a little, and kept on the
  globe's land (an animal that would stand in the sea is drawn in towards its way). The Arctic herd
  crosses today's channels between the Arctic islands: how its ancestors reached Ellesmere Island is
  not known.
- **The dates** are the sources' estimates, rounded. Several are argued over: the parting of the
  camels' and llamas' lines (by about 17 million years ago in the fossils; about 16 in genetic
  estimates, with wide margins), where the true camels arose, and when the dromedary and the
  Bactrian parted (estimates from 0.6 to 8 million years ago; the herds here part as they go their
  separate ways, about 2.9 million years ago, which is no claim about the species).
- **The animals** are illustrations. The extinct animals' coats and colours are not known, nor whether
  Paracamelus or Camelops had humps; the High Arctic camel's shaggy coat and small hump are guesses.
  Sizes are evened out a little so the smallest can be seen; the giraffe-camel and the Arctic giant
  are drawn large. The herded camels wear saddle blankets and the llamas yarn tassels in their ears,
  as herders' animals do.
- **The map** is today's: the globe's coasts, with today's borders and names hidden before 1500 CE.
  **Beringia** is drawn as the sea floor shallower than about 90 metres round the Bering Strait,
  within a coast-like edge, while the strait was land: before it first opened about 5.4 million years
  ago, and in the last ice age from about 35,700 years ago until it flooded 13,000 to 11,000 years
  ago. It shows that there was land, not where its coast was.
- **The date's pace** is uneven: deep time passes fast and the present slowly, and the date slows
  further as the reader comes close, so that a herd's journey can be followed.

## Sources

The numbers are those in `tools/herds/lineages.py` and the panel (`notes.js`).

1. Prothero, D. R. (2009). Evolutionary transitions in the fossil record of terrestrial hoofed
   mammals. *Evolution: Education and Outreach* 2, 289–302. <https://doi.org/10.1007/s12052-009-0136-1>
   (Poebrodon of the middle Eocene; Poebrotherium; Eurasia about 7, South America about 3 million
   years ago.)
2. National Park Service, *The Camels*. <https://www.nps.gov/articles/000/the-camels.htm> (the family
   in North America from about 44 million years ago; north 7–6, Hemiauchenia south 3–2 million years
   ago; Hemiauchenia from about 10 million to about 10,000 years ago.)
3. Rybczynski, N., et al. (2013). Mid-Pliocene warm-period deposits in the High Arctic yield insight
   into camel evolution. *Nature Communications* 4, 1550. <https://pmc.ncbi.nlm.nih.gov/articles/PMC3615376/>
   (the Fyles Leaf Bed, 78.55° N, older than 3.4 million years; the lines parted by about 17 million
   years ago; Paracamelus in Eurasia.)
4. The Paleobiology Database, occurrences of *Poebrodon* and *Poebrotherium*. <https://paleobiodb.org/>
   (the Uinta Formation, Utah; the White River Group.)
5. National Park Service, Agate Fossil Beds: fossil mammals.
   <https://www.nps.gov/agfo/learn/nature/mammalfossils.htm> (Stenomylus, about 21–22 million years.)
6. Rincon, A. F., et al. (2012). New early Miocene camelids from Panama. *Journal of Vertebrate
   Paleontology* 32. <https://doi.org/10.1080/02724634.2012.635736>; Florida Museum,
   <https://www.floridamuseum.ufl.edu/science/new-ancient-camels-from-panama-canal-excavation-named/>
   (Aguascalientia, Las Cascadas Formation, about 21–20 million years ago; Panama the tip of North
   America then.)
7. Florida Museum, *Aepycamelus major*.
   <https://www.floridamuseum.ufl.edu/florida-vertebrate-fossils/species/aepycamelus-major/> (at least
   13 feet at the shoulder; late in the Miocene only in Florida.)
8. Gladenkov, A. Y., et al. (2002). The earliest opening of the Bering Strait. *Palaeogeography,
   Palaeoclimatology, Palaeoecology* 183, 321–328. <https://doi.org/10.1016/S0031-0182(02)00249-3>
9. Farmer, J. R., et al. (2023). The Bering Strait was flooded 10,000 years before the Last Glacial
   Maximum. *PNAS* 120, e2206742119. <https://doi.org/10.1073/pnas.2206742119>
10. Mohandesan, E., et al. (2017). Mitogenome sequencing in the genus Camelus. *Scientific Reports* 7.
    <https://pmc.ncbi.nlm.nih.gov/articles/PMC5577142/> (divergence estimates; the wild Bactrian's
    parting about 1.1 million years ago.)
11. Heintzman, P. D., et al. (2015). Genomic data from extinct North American Camelops revise camel
    evolutionary history. *Molecular Biology and Evolution* 32. <https://doi.org/10.1093/molbev/msv128>;
    Waters, M. R., et al. (2015). Late Pleistocene horse and camel hunting at the southern margin of
    the ice-free corridor. *PNAS* 112. <https://pmc.ncbi.nlm.nih.gov/articles/PMC4394292/>
12. Gibert, L., et al. (2013). *Geology* 41. <https://doi.org/10.1130/G34164.1> (Venta del Moro, 6.23
    million years); van der Made, J., et al. (2003). Camelidae from the Sinap Formation.
    <https://www.mncn.csic.es/sites/default/files/inline-files/2003%20Van%20der%20Made%20ea%20-%20Sinap-Camelidae%20-%20Sinap%20Monograph.pdf>
13. Geraads, D., et al. (2019). The first true camels. *Journal of Mammalian Evolution*.
    <https://doi.org/10.1007/s10914-019-09489-2> (open copy <https://hal.science/hal-02457813/document>):
    Camelus in eastern Africa about 3 million years ago; Paracamelus in Chad, Tunisia and Egypt; the
    Siwaliks.
14. Burger, P. A. (2016). The history of Old World camelids in the light of molecular genetics.
    *Tropical Animal Health and Production* 48. <https://pmc.ncbi.nlm.nih.gov/articles/PMC4884201/>
    (the wild Bactrian camel, Camelus ferus, a species of its own; fewer than 2,000 left.)
15. Berthon, R., et al. (2020), on the Bactrian camel's domestication.
    <https://books.openedition.org/momeditions/8527> (4,000–6,000 years ago, place uncertain; Anau;
    Shahr-i Sokhta.)
16. Almathen, F., et al. (2016). Ancient and modern DNA reveal dynamics of domestication and
    cross-continental dispersal of the dromedary. *PNAS* 113. <https://pmc.ncbi.nlm.nih.gov/articles/PMC4914195/>
17. O'Dea, A., et al. (2016). Formation of the Isthmus of Panama. *Science Advances* 2.
    <https://pmc.ncbi.nlm.nih.gov/articles/PMC4988774/>
18. Gasparini, G. M., et al. (2017). *Geobios* 50. <https://doi.org/10.1016/j.geobios.2016.12.003>
    (Hemiauchenia in South America; Olavarría; Uquía.)
19. Animal Diversity Web, *Lama guanicoe*. <https://animaldiversity.org/accounts/Lama_guanicoe/>;
    González, B. A., et al. (2019), on the vicuña. <https://pmc.ncbi.nlm.nih.gov/articles/PMC6562099/>
20. Kadwell, M., et al. (2001). Genetic analysis reveals the wild ancestors of the llama and the
    alpaca. *Proceedings of the Royal Society B* 268. <https://doi.org/10.1098/rspb.2001.1774>
21. Burke and Wills Web, Introducing camels into Australia.
    <https://burkeandwills.net.au/Camels/Introducing_Camels_Into_Australia.htm>; PestSmart, Feral
    camels. <https://pestsmart.org.au/toolkits/feral-camels/>
22. Handbook of Texas, Camels. <https://www.tshaonline.org/handbook/entries/camels>; National Park
    Service, El Morro: the Army Camel Corps.
    <https://www.nps.gov/elmo/learn/historyculture/the-army-camel-corps.htm>
