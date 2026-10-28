/* Borders on old maps: what the maps on the globe are, how to read them and where they come from, in
   the panel the chip's Open shows (it was this pack's own page before runtime 3). */
(function () {
  'use strict';
  terra.on('open', function () {
    terra.panel.show({
      title: 'Borders on old maps',
      text: 'Each year on the date strip shows one of eight maps of Europe\'s states, drawn for 1530, 1600, 1815, 1878, 1920, 1938, 1945 and 1994. Between them the borders do not move: a year with no map of its own shows the nearest one, and the caption says which, so no border is invented for the years between.',
      sections: [
        { title: 'Reading the globe',
          text: 'The filled country is the one you chose, as that map draws it.\n\n' +
                'The dashed line is the same country on the closest earlier map that has it.\n\n' +
                'The 1530 and 1600 maps here show only the Ottoman Empire. Later maps cover Europe and its edges only.\n\n' +
                'The 1945 map shows Germany in four zones of occupation, named here as the British, American, French and Soviet zones.\n\n' +
                'Today\'s borders and names are hidden; the coasts are today\'s, as a reference.' },
        { title: 'What these maps are',
          text: 'Generalised maps of states made for a world atlas of history, simplified further here (to about 0.025°). They are not precise records of a treaty or a day: 1945, for example, combines arrangements made after the war. A country missing from a map means only that this map has no shape by that name; it says nothing about the people who lived there.' }
      ],
      sources: [
        { title: 'André Ourednik and contributors, historical-basemaps', url: 'https://github.com/aourednik/historical-basemaps',
          note: 'Files world_<year>.geojson at commit da7a4b7, cut to Europe and simplified. GPL-3.0 (LICENSE-HISTORY.txt in the pack).' },
        { title: 'Natural Earth, 1:110m land (public domain)', url: 'https://www.naturalearthdata.com/', note: 'Only to tell land the maps don\'t cover from the sea.' }
      ]
    });
  });
})();
