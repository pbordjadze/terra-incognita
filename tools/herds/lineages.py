"""The camel family's history as lineages, each a way across the map with a date at every turn.

Dates are years of the common era (negative before it; 1 BCE is 0), so deep time is a large negative
number: -6.23e6 is about 6.23 million years ago. Places are [lon, lat]. Every turn is traced to a
source in Prototypes/herds/SOURCES.md (the numbers in brackets); where the sources give a region or a
span rather than a place and a date, the turn is placed within it and the way between turns is drawn,
not known. A lineage is a population over many generations, not one animal's journey.

Fields: id; name (the panel's heading); kinds [[from date, kind, what it is]] (the drawing, from
tools/herds/draw.cjs, and the words a reader gets on touching it); parent (the lineage it parts from:
its way starts where the parent is at its first date); take (how many of the parent's animals come
over); leader_from_parent (the parent's leader, the animal seen close, comes over); members (the herd's
size, filled up with animals born at the parting); grow [[date, n]]; way [[lon, lat, date], ...]; ends
(the date its last animals go; none: still here); spread (km round the way); sea_ok (its way crosses
water the globe's map shows, see its note).
"""

START = -44e6          # the oldest camelid, Poebrodon [1, 2, 4]
TODAY = 2026

# when Beringia was land between Siberia and Alaska: before the strait first opened, about 5.4 million
# years ago [8]; and in the last ice age, from about 35,700 years ago until it flooded 13,000 to 11,000
# years ago [9]
BERINGIA_LAND = [[START, -5.4e6], [-33700, -10000]]

# the date's pace: [date, seconds of playback]. Deep time passes fast and the present slowly, so the
# journeys move at a pace a reader can follow
PACE = [
    [START, 0],
    [-24e6, 16],        # the first camelids, on the plains
    [-17e6, 28],        # Panama, the giraffe-camels, the parting of camels and llamas
    [-7.6e6, 44],       # north to Alaska
    [-6.0e6, 74],       # across Beringia and Eurasia to Spain and Egypt
    [-3.3e6, 98],       # Africa, China, the High Arctic
    [-1.9e6, 122],      # the isthmus, the Andes; true camels out of Africa
    [-0.4e6, 130],
    [-14000, 138],      # the last ice age: the American camels' end
    [-5000, 148],       # the first herders
    [0, 154],
    [1800, 160],
    [TODAY, 168],
]

# the strip's ticks: where the evidence is (SOURCES.md)
TICKS = [START, -33e6, -21e6, -17e6, -6.23e6, -5.5e6, -3.7e6, -2.8e6, -2.6e6, -11350, -4500, -1000, 1856, 1866]

LINEAGES = [
    dict(id='first', name='The first camelids',
         kinds=[[START, 'early', 'Poebrodon, one of the first camelids, the size of a goat']],
         members=3, grow=[[-36e6, 1], [-27e6, 2], [-21e6, 2]], spread=90,
         # the Uinta Basin, Utah [1, 4]; the White River Badlands [1, 4]; Agate, Nebraska [5]
         way=[[-109.9, 40.2, START], [-108.4, 42.3, -40e6], [-102.0, 43.8, -33e6], [-101.2, 42.6, -29e6],
              [-103.7, 42.4, -24e6], [-100.9, 40.6, -20e6], [-100.6, 42.0, -17e6]],
         ends=-17e6),
    dict(id='panama', name='The camels of Panama', parent='first', take=1, members=3, spread=60,
         kinds=[[-24e6, 'early', 'Aguascalientia, a small camel of Panama, then the tip of North America']],
         # Las Cascadas, the Panama Canal, about 21–20 million years ago [6]
         way=[[0, 0, -24e6], [-98.2, 30.0, -23.2e6], [-98.6, 21.0, -22.2e6], [-91.0, 15.3, -21.4e6],
              [-83.6, 10.4, -21.1e6], [-79.7, 9.04, -20.8e6], [-79.9, 8.7, -20.0e6]],
         ends=-19.6e6),
    dict(id='giraffes', name='The giraffe-camels', parent='first', take=1, members=3, spread=120,
         kinds=[[-18.5e6, 'giraffe', 'Aepycamelus, a giraffe-camel, at least 13 feet at the shoulder']],
         # widespread 16–12.5 million years ago (Valentine, Nebraska); late, only in Florida [7]
         way=[[0, 0, -18.5e6], [-100.55, 42.87, -16e6], [-101.0, 38.4, -13.5e6], [-97.6, 31.6, -11.5e6],
              [-90.5, 31.2, -10.5e6], [-82.45, 29.39, -9.5e6], [-82.0, 28.6, -7.0e6]],
         ends=-6.0e6),
    dict(id='camels', name='The camels’ line, in North America', parent='first', take=3, leader_from_parent=True, members=5,
         kinds=[[-17e6, 'paracamelus', 'An early camel of the American plains']],
         # the camels' and llamas' lines had parted by about 17 million years ago [3, 10]
         way=[[0, 0, -17e6], [-104.5, 43.6, -14e6], [-110.0, 47.4, -11.5e6], [-113.9, 52.8, -9.5e6]],
         ends=-9.5e6),
    dict(id='american-camels', name='The camels that stayed: Camelops', parent='camels', take=2, members=3, spread=130,
         kinds=[[-9.5e6, 'paracamelus', 'A camel of the American West'],
                [-4e6, 'camelops', 'Camelops, the camel of the American West: closer to today’s camels than to llamas']],
         # Camelops gone about 13,000 years ago; a camel butchered at Wally's Beach, Alberta, 13,300 years ago [11]
         way=[[0, 0, -9.5e6], [-116.8, 39.4, -6e6], [-119.6, 36.2, -3e6], [-112.0, 33.6, -1e6], [-117.0, 37.0, -0.3e6],
              [-113.0, 44.0, -40000], [-113.2, 49.3, -11350]],
         ends=-11350),
    dict(id='north', name='North, to Beringia', parent='camels', take=3, leader_from_parent=True, members=5,
         kinds=[[-9.5e6, 'paracamelus', 'Paracamelus, a camel of the north, on its way to Asia']],
         # across Beringia, land then, about 7 million years ago [2, 3, 8]
         way=[[0, 0, -9.5e6], [-124.0, 58.6, -8.6e6], [-136.0, 63.5, -8.0e6], [-152.0, 65.0, -7.4e6],
              [-163.0, 65.6, -7.15e6], [-169.0, 65.9, -7.0e6], [-176.0, 66.0, -6.88e6]],
         ends=-6.88e6),
    dict(id='arctic', name='The giant camels of the High Arctic', parent='north', take=2, members=3, spread=90, sea_ok=True,
         kinds=[[-7.6e6, 'paracamelus', 'A camel heading north, into the Arctic forests'],
                [-5.5e6, 'arctic', 'The giant camel of the High Arctic: a third larger than a camel today, in a boreal forest']],
         # the Fyles Leaf Bed, Ellesmere Island, older than 3.4 million years [3]. Its way across today's
         # channels of the Arctic islands is drawn, not known
         way=[[0, 0, -7.6e6], [-134.0, 68.4, -6.6e6], [-123.0, 71.6, -5.6e6], [-110.5, 75.2, -4.8e6],
              [-95.0, 76.6, -4.2e6], [-82.4, 78.55, -3.7e6], [-80.5, 79.3, -3.4e6]],
         ends=-3.2e6),
    dict(id='europe', name='Paracamelus in Europe', parent='north', take=1, leader_from_parent=True, members=4,
         kinds=[[-6.88e6, 'paracamelus', 'Paracamelus, the first camels of Eurasia']],
         # Odessa and Venta del Moro (6.23 million years ago, the earliest camel in Europe), Librilla [12]
         way=[[0, 0, -6.88e6], [150.0, 64.0, -6.8e6], [128.0, 61.5, -6.7e6], [104.0, 53.0, -6.6e6],
              [72.0, 49.0, -6.5e6], [45.0, 47.0, -6.44e6], [30.75, 46.52, -6.4e6], [22.0, 42.6, -6.32e6],
              [14.5, 46.2, -6.29e6], [5.5, 45.0, -6.265e6], [1.5, 42.0, -6.245e6], [-1.36, 39.48, -6.23e6], [-1.36, 37.89, -5.9e6], [-3.5, 40.0, -4.5e6],
              [2.5, 44.6, -4.0e6], [16.0, 45.0, -3.6e6]],
         ends=-3.0e6),
    dict(id='africa', name='Into Africa: Paracamelus, then the true camels', parent='europe', take=0, leader_from_parent=True,
         members=3,
         kinds=[[-6.4e6, 'paracamelus', 'Paracamelus, on its way into Africa'],
                [-3.0e6, 'camelus', 'Camelus, the first true camels, in eastern Africa about 3 million years ago']],
         # Wadi Natrun, Egypt; Kossom Bougoudi, Chad; Camelus arose in eastern Africa about 3 million
         # years ago, at Mille-Logya, Afar [12, 13]
         way=[[0, 0, -6.4e6], [37.5, 47.5, -6.37e6], [44.0, 42.5, -6.33e6], [40.0, 38.0, -6.27e6], [36.4, 33.5, -6.2e6], [34.9, 30.8, -6.12e6], [33.2, 30.0, -6.06e6], [30.3, 30.4, -6.0e6],
              [24.0, 22.0, -5.6e6], [18.0, 16.0, -5.2e6], [30.0, 13.0, -4.0e6], [40.7, 11.5, -2.9e6]],
         ends=-2.9e6),
    dict(id='asia', name='The true camels, out of Africa to Asia', parent='africa', take=1, members=3,
         kinds=[[-2.9e6, 'camelus', 'Camelus, the true camels, on their way to Asia'],
                [-1.6e6, 'bactrian', 'A wild two-humped camel of Central Asia']],
         # the Siwaliks, at least 2.6 million years ago [13]
         way=[[0, 0, -2.9e6], [36.5, 19.0, -2.86e6], [32.0, 23.0, -2.83e6], [31.5, 28.0, -2.8e6], [34.0, 29.8, -2.78e6], [39.5, 30.5, -2.76e6],
              [50.0, 31.0, -2.7e6], [64.0, 30.5, -2.64e6], [76.9, 30.8, -2.6e6], [69.0, 38.5, -2.2e6],
              [68.0, 44.0, -1.6e6], [72.0, 44.5, -1.1e6]],
         ends=-1.1e6),
    dict(id='wild-bactrian', name='The wild Bactrian camel', parent='asia', take=1, members=3,
         kinds=[[-1.1e6, 'bactrian', 'The wild Bactrian camel: a species of its own, fewer than 2,000 left']],
         # parted from the domestic Bactrians' line about 1.1 million years ago; Lop Nur and the Great Gobi [10, 14]
         way=[[0, 0, -1.1e6], [82.0, 42.5, -0.8e6], [90.3, 40.3, -0.3e6], [95.0, 42.0, -20000], [97.0, 43.5, 1900],
              [96.0, 43.2, TODAY]]),
    dict(id='bactrian', name='The Bactrian camel, wild and then herded', parent='asia', take=2, leader_from_parent=True, members=3,
         kinds=[[-1.1e6, 'bactrian', 'A two-humped camel of Central Asia'],
                [-4000, 'bactrian-domestic', 'A Bactrian camel, herded for the last 4,000 to 6,000 years']],
         # domesticated 4,000–6,000 years ago, where is uncertain; Anau, Turkmenistan; Shahr-i Sokhta, Iran [15]
         way=[[0, 0, -1.1e6], [66.0, 45.5, -0.5e6], [63.0, 42.0, -20000], [60.0, 39.5, -6000], [58.5, 37.9, -3500],
              [61.33, 30.6, -2800], [63.0, 36.0, -1500], [80.0, 41.0, 0], [95.0, 44.0, 1000], [103.0, 46.0, TODAY]]),
    dict(id='dromedaries', name='The dromedary', parent='africa', take=2, leader_from_parent=True, members=4,
         kinds=[[-2.9e6, 'camelus', 'Camelus grattardi, a true camel of eastern Africa'],
                [-1.0e6, 'dromedary', 'A wild dromedary of Arabia'],
                [-1000, 'dromedary-domestic', 'A dromedary, first herded in southeast Arabia about 3,000 years ago']],
         # Omo, 2.2 million years ago; wild dromedaries in southeast Arabia; domesticated late in the 2nd
         # millennium BCE; Tell Abraq [13, 16]
         way=[[0, 0, -2.9e6], [36.0, 5.0, -2.2e6], [33.0, 15.0, -1.6e6], [33.0, 26.0, -1.2e6], [37.0, 28.0, -1.1e6],
              [45.0, 24.0, -0.6e6], [52.0, 23.0, -50000], [55.24, 25.1, -5000], [55.64, 25.5, -1000],
              [54.5, 24.0, -500], [50.0, 24.0, 0], [47.0, 25.5, TODAY]]),
    dict(id='sahara', name='The dromedary across North Africa', parent='dromedaries', take=1, members=3,
         kinds=[[-900, 'dromedary-domestic', 'A dromedary, carrying trade across North Africa']],
         # into northeast Africa by Sinai from early in the 1st millennium BCE; large herds in northwest
         # Africa by the 4th–7th centuries CE [16]
         way=[[0, 0, -900], [54.6, 23.4, -870], [50.0, 24.5, -820], [38.0, 28.5, -760], [33.8, 29.5, -700], [31.2, 30.0, -300], [20.0, 30.5, 200], [10.0, 28.0, 450],
              [2.0, 27.0, 600], [-6.0, 30.5, 800], [-9.0, 24.0, 1500], [-6.0, 27.0, TODAY]]),
    dict(id='llamas', name='The llamas’ line', parent='first', take=3, members=4,
         kinds=[[-17e6, 'hemiauchenia', 'An early member of the llamas’ line, on the American plains']],
         # Hemiauchenia in North America from about 10 million years ago, into South America about 3 to 2
         # million years ago [2]; the isthmus complete about 2.8 million years ago [17]; Uquía [18]
         way=[[0, 0, -17e6], [-99.5, 36.0, -13e6], [-98.8, 30.8, -10e6], [-101.0, 23.0, -6e6], [-96.5, 17.5, -4e6],
              [-90.5, 15.0, -3.2e6], [-85.0, 12.0, -3.0e6], [-83.9, 9.9, -2.9e6], [-80.4, 8.5, -2.83e6], [-77.8, 8.2, -2.78e6],
              [-75.5, 6.5, -2.74e6], [-74.0, 5.0, -2.7e6],
              [-78.3, -1.5, -2.6e6], [-76.0, -10.0, -2.48e6], [-69.0, -17.5, -2.38e6], [-65.35, -23.3, -2.3e6]],
         ends=-2.3e6),
    dict(id='llamas-north', name='The llamas that stayed: Hemiauchenia', parent='llamas', take=1, members=3, spread=130,
         kinds=[[-6e6, 'hemiauchenia', 'Hemiauchenia, a long-legged llama of North America']],
         # gone from North America about 10,000 years ago [2]
         way=[[0, 0, -6e6], [-98.0, 29.5, -4e6], [-90.0, 31.5, -2e6], [-86.0, 31.3, -1.6e6], [-82.4, 29.6, -1e6], [-97.0, 33.0, -0.4e6],
              [-112.0, 34.0, -60000], [-104.0, 35.0, -8000]],
         ends=-8000),
    dict(id='pampas', name='Hemiauchenia on the pampas', parent='llamas', take=1, members=3, spread=130,
         kinds=[[-2.3e6, 'hemiauchenia', 'Hemiauchenia, the first of the family in South America']],
         # Olavarría, Buenos Aires [18]; gone about 10,000 years ago [2]
         way=[[0, 0, -2.3e6], [-62.0, -30.0, -2.0e6], [-60.32, -36.89, -1.6e6], [-62.0, -33.5, -0.5e6], [-63.0, -35.0, -8000]],
         ends=-8000),
    dict(id='guanacos', name='The guanaco', parent='llamas', take=1, leader_from_parent=True, members=4,
         kinds=[[-2.3e6, 'hemiauchenia', 'An early llama, moving south along the Andes'],
                [-2.0e6, 'guanaco', 'A guanaco, wild, from northern Peru to Tierra del Fuego']],
         # guanacos range from northern Peru to Tierra del Fuego [19]
         way=[[0, 0, -2.3e6], [-68.8, -33.0, -2.0e6], [-69.5, -40.0, -1.4e6], [-69.5, -46.0, -0.8e6],
              [-68.65, -52.28, -0.3e6], [-69.5, -49.0, -20000], [-70.0, -45.5, TODAY]]),
    dict(id='andean-guanacos', name='The guanacos of the Andes, and the llama', parent='guanacos', take=1, members=3,
         kinds=[[-2.0e6, 'guanaco', 'A guanaco of the high Andes']],
         way=[[0, 0, -2.0e6], [-67.5, -27.0, -1.4e6], [-69.0, -19.0, -0.8e6], [-74.0, -13.0, -0.3e6],
              [-75.9, -11.0, -5000], [-75.5, -10.0, TODAY]]),
    dict(id='llama', name='The llama, herded', parent='andean-guanacos', take=1, members=3,
         kinds=[[-4500, 'llama', 'A llama: herded from guanacos in the Andes, some 6,000 or 7,000 years ago']],
         # Kadwell and others: llama from guanaco, alpaca from vicuña, about 6,000–7,000 years ago [20]
         way=[[0, 0, -4500], [-74.0, -12.6, -2000], [-72.0, -13.5, 1000], [-69.5, -16.5, 1500], [-67.0, -19.0, TODAY]]),
    dict(id='vicunas', name='The vicuña, and the alpaca', parent='llamas', take=1, members=3,
         kinds=[[-2.3e6, 'hemiauchenia', 'An early llama of the high Andes'],
                [-2.0e6, 'vicuna', 'A vicuña, wild, in the high puna, 3,000 to 5,000 metres up']],
         # the puna, 9°30′ S to 29° S [19]; Telarmachay, the puna of Junín [20]
         way=[[0, 0, -2.3e6], [-67.0, -21.0, -2.0e6], [-69.0, -17.0, -1.0e6], [-73.0, -14.0, -0.3e6],
              [-75.85, -11.2, -6000], [-72.5, -14.5, 1000], [-69.8, -16.5, TODAY]]),
    dict(id='alpaca', name='The alpaca, herded', parent='vicunas', take=1, members=3,
         kinds=[[-4500, 'alpaca', 'An alpaca: herded from vicuñas for their wool, some 6,000 or 7,000 years ago']],
         way=[[0, 0, -4500], [-75.85, -11.2, -4400], [-73.5, -13.8, -1500], [-70.5, -15.3, 1000], [-70.2, -15.0, TODAY]]),
    dict(id='australia', name='Dromedaries in Australia', members=4, spread=150,
         kinds=[[1866, 'dromedary-domestic', 'A dromedary brought to Australia to carry loads; its herds now wild'],
                [1925, 'dromedary', 'A feral dromedary of the Australian deserts, one of some 300,000']],
         # 124 camels to Port Augusta in 1866, then Beltana; some 10,000–20,000 brought 1840–1907; about
         # 300,000 wild today [21]
         way=[[137.77, -32.49, 1866], [138.4, -30.81, 1867], [136.3, -27.6, 1885], [134.5, -25.0, 1910],
              [131.0, -24.0, 1940], [126.0, -24.5, 1980], [129.0, -25.5, TODAY]]),
    dict(id='texas', name='The US Army’s camels', members=3, spread=60,
         kinds=[[1856, 'dromedary-domestic', 'A camel of the US Army, landed in Texas in 1856']],
         # Indianola, April 1856; Camp Verde; Fort Defiance to Fort Tejon with Beale, 1857; sold off in the
         # 1860s; wild ones seen into the early 1900s [22]
         way=[[-96.62, 28.53, 1856.3], [-99.12, 29.89, 1856.6], [-104.5, 32.5, 1857.2], [-109.08, 35.74, 1857.45],
              [-114.6, 35.0, 1857.8], [-118.88, 34.87, 1857.9], [-118.24, 34.05, 1858], [-113.0, 33.5, 1870],
              [-112.0, 33.0, 1905]],
         ends=1906),
]
