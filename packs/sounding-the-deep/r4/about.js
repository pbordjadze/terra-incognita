/* Sounding the deep: what the voyage on the globe is, how to read it and where it comes from, in the
   panel the chip's Open shows (it was this pack's own page before runtime 3). */
(function () {
  'use strict';
  terra.on('open', function () {
    terra.panel.show({
      title: 'Sounding the deep',
      wikipedia: 'Challenger expedition',
      text: 'HMS Challenger was a British ship sent out from 1872 to 1876 to explore the deep ocean. At its stations the crew stopped, let a weighted line down to measure the depth, and brought up samples of the sea floor.',
      sections: [
        { title: 'Reading the globe',
          text: 'Dots are the stations in this record: filled once the date has reached them, hollow still ahead.\n\n' +
                'The dashed line between two close stations is an illustration of the passage, not the ship\'s logged course; it is drawn only when they are two weeks apart or less and the straight way between them stays off land.\n\n' +
                'Across a longer gap the record has no position, so the ship is not shown.\n\n' +
                'The hanging line goes down to the depth recorded at that station, drawn to a fixed scale on the screen.\n\n' +
                'Today\'s borders and names are hidden; the coasts are today\'s, as a reference.' },
        { title: 'What this record is', wikipedia: 'John Murray (oceanographer)',
          text: '94 dated stations, from 2 February 1873 to 6 May 1876, with their positions and recorded depths: the stations whose seabed deposits were examined for manganese by Murray and Renard. It is a selection, not the expedition\'s complete log, and its positions are nineteenth-century measurements, not modern ones.' },
        { title: 'How a sounding worked', wikipedia: 'Depth sounding',
          text: 'A hemp line, marked every 25 fathoms, was paid out with iron weights on the end until it stopped running; sinking to 3,000 fathoms took about three-quarters of an hour from the 500-fathom mark. A depth is the length of line that ran out, read to the nearest mark: a fathom is six feet, 1.8288 m.' }
      ],
      sources: [
        { title: 'Murray, John; Renard, A. F. (1891). Annotated record of the detailed examination of Mn deposits from the HMS Challenger stations. PANGAEA.',
          url: 'https://doi.org/10.1594/PANGAEA.849072', note: 'Licence CC BY 3.0. The stations and their dates (reduced to days). Depths recomputed from the recorded fathoms (Murray & Renard 1891); PANGAEA 849072\'s elevation converts them at 1.6421 m a fathom.' },
        { title: 'Murray, John; Renard, A. F. (1891). Report on the deep-sea deposits. The station table, as transcribed for PANGAEA.',
          url: 'https://epic.awi.de/38636/1/NGDC_MURRAY1891_V01.txt', note: 'The recorded depths in fathoms, and six positions as printed.' },
        { title: 'Natural Earth, 1:110m land (public domain)', url: 'https://www.naturalearthdata.com/', note: 'Only to keep illustrated passages off land.' },
        { title: 'The ship is an original drawing, not a picture of Challenger\'s rigging.', note: '' }
      ]
    });
  });
})();
