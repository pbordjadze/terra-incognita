/* {{id}}: a runtime-3 pack (docs/runtime3.md is the contract).
   This script runs in JavaScriptCore inside the app, with no web page: it tells the globe what to
   show and the globe draws it, in step with the camera. It reacts to events and timers; it never
   draws and never runs every frame. There is no network: data comes with the pack (a data.js listed
   before this file in "scripts", or terra.readJSON('data.json')). Everything it asks for goes when
   the prototype ends. */
(function () {
  'use strict';
  var PLACE = [10, 35];   // [lon, lat]: where this happens (the doorway and the anchor say the same)

  // The words at the foot: what the reader is looking at first, then why it matters.
  terra.caption.set({
    title: "{{title}}",
    text: 'What you are looking at, first. Then why it is interesting. Touch the mark for more.'
  });

  // A thing that lives on the globe. A look is chosen by zoom; touchable things need a label,
  // which VoiceOver speaks. Under Reduce Motion the globe holds sprites still by itself.
  terra.sprites.set('place', {
    at: PLACE,
    looks: [{ image: 'marker.png', size: [44, 44] }],
    label: "{{title}}",
    touchable: true
  });

  // Longer words and their sources, in a sheet over the globe, when the reader asks for them.
  terra.on('touch', function (t) {
    if (t.phase !== 'tap' || !t.target || t.target.sprite !== 'place') return;
    terra.panel.show({
      title: "{{title}}",
      text: 'Longer words: what this is, then why it matters.\n\nA second paragraph after a blank line.',
      sources: [
        // { title: 'Who published it, and what', url: '<its address>', note: 'what this pack takes from it' }
      ]
    });
  });
})();
