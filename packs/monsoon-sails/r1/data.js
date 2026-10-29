/* The monsoon's sails: the ports, the ways between them (waypoints keep a way at sea), and where the
   wind is drawn. Degrees [lon, lat]. The ports and ways are the Indian Ocean's old monsoon trade in
   outline (SOURCES.md): which ports and which ways is a choice, the waypoints are drawn. */
window.MONSOON = {
  ports: {
    kilwa: { at: [39.52, -8.96], name: 'Kilwa' },
    zanzibar: { at: [39.19, -6.16], name: 'Zanzibar' },
    mombasa: { at: [39.67, -4.05], name: 'Mombasa' },
    mogadishu: { at: [45.34, 2.05], name: 'Mogadishu' },
    aden: { at: [45.03, 12.78], name: 'Aden' },
    muscat: { at: [58.59, 23.61], name: 'Muscat' },
    sur: { at: [59.53, 22.57], name: 'Sur' },
    siraf: { at: [52.34, 27.67], name: 'Siraf' },
    mandvi: { at: [69.35, 22.83], name: 'Mandvi' },
    calicut: { at: [75.78, 11.25], name: 'Calicut' }
  },
  // each way from its south-western end to its north-eastern end: the summer wind (from the south-west)
  // carries ships up it, the winter wind (from the north-east) back down
  ways: [
    { from: 'zanzibar', to: 'muscat', via: [[43.0, -1.0], [50.5, 5.0], [55.5, 14.0], [58.3, 22.0]] },
    { from: 'kilwa', to: 'aden', via: [[42.5, -3.0], [48.5, 4.0], [51.5, 11.2], [47.5, 12.4]] },
    { from: 'mombasa', to: 'mandvi', via: [[46.0, 0.0], [56.0, 10.0], [66.0, 19.0]] },
    { from: 'mogadishu', to: 'calicut', via: [[55.0, 5.0], [68.0, 9.0]] },
    { from: 'aden', to: 'calicut', via: [[50.5, 12.0], [58.0, 12.0], [70.0, 10.5]] },
    { from: 'muscat', to: 'calicut', via: [[61.5, 22.0], [70.0, 15.0]] },
    { from: 'siraf', to: 'mandvi', via: [[56.6, 26.4], [60.0, 24.0], [66.0, 22.5]] },
    { from: 'zanzibar', to: 'sur', via: [[44.0, -1.0], [52.0, 8.0], [58.0, 16.0]] }
  ],
  // where the wind is drawn: over the open sea, the Arabian Sea and the western ocean mostly
  wind: [[50, 10], [56, 8], [62, 12], [66, 18], [58, 17], [52, 16], [47, 4], [44, -3], [49, -8], [55, -2], [60, 3],
         [66, 6], [70, 2], [72, 14], [60, -8], [66, -13], [75, -6], [82, 10], [86, 14], [88, 6], [80, -2], [90, -6]]
};
