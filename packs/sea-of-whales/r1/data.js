/* Whales in the desert: where the drawn sea lies (the same shore tools/whales/build_sea.py paints; an
   illustration of the late Eocene coast, not a reconstruction), the fossil valley, and the whales'
   ways through the drowned desert. All positions are degrees [lon, lat]. */
window.WHALES = {
  sea: { bounds: [24, 26, 37, 34] },   // the pictures tethys-1.png to -4.png
  // the drawn shore west to east; the sea lies north of it
  shore: [[24.0, 27.6], [25.2, 27.7], [26.3, 27.9], [27.4, 28.25], [28.3, 28.65], [29.0, 28.85], [29.6, 28.9],
          [30.05, 29.0], [30.3, 29.12], [30.7, 29.02], [31.15, 29.18], [31.7, 29.05], [32.2, 29.25], [32.75, 29.55],
          [33.3, 29.85], [33.9, 29.9], [34.5, 30.1], [35.2, 30.4], [35.9, 30.95], [36.6, 31.3], [37.0, 31.45]],
  valley: [30.18333, 29.33333],       // Wadi Al-Hitan (UNESCO's reference point for the property)
  // the bones as they lie now: the valley's, and a few more along the old bay (drawn where the record
  // says whales lie: the Fayum's Eocene beds; their exact places are not claimed)
  bones: [
    { at: [30.18333, 29.33333], image: 'bones.png', size: [64, 22], label: 'A whale skeleton, Wadi Al-Hitan' },
    { at: [30.26, 29.41], image: 'bones-2.png', size: [44, 16], label: 'A smaller whale skeleton' },
    { at: [30.11, 29.29], image: 'bones-3.png', size: [30, 12], label: 'A young whale\'s skeleton' },
    { at: [30.31, 29.30], image: 'bones-3.png', size: [30, 12], label: 'A whale skeleton' }
  ],
  // the whales, each on a way round the drowned desert (closed loops; seconds for the whole way)
  whales: [
    { id: 'basilosaurus-1', kind: 'basilosaurus', label: 'Basilosaurus, eighteen metres of whale',
      path: [[30.2, 29.6], [31.0, 29.9], [31.6, 29.7], [31.4, 29.35], [30.6, 29.25], [30.0, 29.45]], seconds: 156 },
    { id: 'basilosaurus-2', kind: 'basilosaurus', label: 'Basilosaurus, out in the open sea',
      path: [[29.0, 29.6], [28.0, 30.4], [27.2, 31.4], [28.6, 31.9], [29.8, 31.0], [29.6, 30.1]], seconds: 252 },
    { id: 'dorudon-1', kind: 'dorudon', label: 'Dorudon, a smaller whale',
      path: [[30.35, 29.25], [30.75, 29.5], [30.55, 29.75], [30.1, 29.55]], seconds: 90 },
    { id: 'dorudon-2', kind: 'dorudon', label: 'Dorudon',
      path: [[30.45, 29.3], [30.85, 29.42], [30.7, 29.68], [30.25, 29.6]], seconds: 102 },
    { id: 'dorudon-3', kind: 'dorudon', label: 'Dorudon, along the old shore',
      path: [[31.9, 29.3], [32.6, 29.75], [33.3, 30.05], [32.9, 30.4], [32.1, 29.9]], seconds: 138 },
    { id: 'dorudon-4', kind: 'dorudon', label: 'Dorudon, far to the west',
      path: [[26.5, 28.3], [27.6, 28.9], [27.2, 29.6], [26.0, 29.0]], seconds: 156 }
  ]
};
