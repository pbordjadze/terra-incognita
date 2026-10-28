/* The delivery check: which copy of this pack is running (bundled with the app, or a revision that
   was downloaded), said at the foot of the screen, and a quiet ring where the sea is tapped. CI's
   remote-pack test serves a copy whose words differ (ci/remote_packs.sh changes them). */
(function () {
  'use strict';
  var WORDS = 'First words.';
  var pack = terra.pack;
  var where = pack.origin === 'bundled' ? 'Bundled with the app' :
              pack.origin === 'downloaded' ? 'Revision ' + pack.revision + ', downloaded' :
              'Previewed on a desktop';

  terra.caption.set({ title: 'Delivery check', text: WORDS + ' ' + where + '.' });

  terra.on('open', function () {
    terra.panel.show({
      title: 'Delivery check',
      text: WORDS + '\n\n' + where + '.\n\nThese words came from a pack rather than from the app build, so its words can change without a new version of the app.'
    });
  });

  // a ring where the sea is tapped, gone after a moment (one at a time)
  var timer = 0;
  terra.on('tap', function (t) {
    if (t.lon == null) return;
    terra.geo.set('ring', [{ type: 'Point', coordinates: [t.lon, t.lat] }], { style: 'ring', color: '#3B2C16' });
    clearTimeout(timer);
    timer = setTimeout(function () { terra.geo.remove('ring'); }, terra.settings.reduceMotion ? 1200 : 1600);
  });
})();
