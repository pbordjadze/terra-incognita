// Ink orders: the plans a finger draws (CSS px on the 1376x1032 field), and the film's clock.
'use strict';
const PLANS = {
  // the crescent is left to give ground under the Roman press; the Africans hook in; Hasdrubal rides
  // up the river, through the Roman horse and round the rear
  good: [
    [[262, 616], [258, 540], [252, 460], [258, 400], [276, 320], [312, 252], [390, 206], [520, 190], [640, 200], [706, 250], [724, 322]],
    [[334, 664], [332, 600], [332, 540], [344, 500], [374, 476], [410, 466]],
    [[706, 664], [708, 600], [708, 540], [696, 500], [666, 476], [630, 466]],
  ],
  // everything straight at the Roman centre: the crescent charges, the Africans crowd in behind it
  bad: [
    [[520, 548], [520, 470], [520, 380]],
    [[334, 664], [380, 640], [440, 610], [480, 590]],
    [[706, 664], [660, 640], [600, 610], [560, 590]],
  ],
  none: [],
};
const FRAMES = 300;

// the film as actions: a still beat, each stroke drawn with the battle running gently under it,
// then the battle on its own with the clock quickening
function schedule(plan) {
  const a = []; let f = 0; const frame = n => a.push({ t: 'frame', f: f++, n });
  for (let k = 0; k < 8; k++) frame(0);
  for (const st of plan) {
    const pts = [];
    for (let k = 1; k < st.length; k++) { const p = st[k - 1], q = st[k], n = Math.ceil(Math.hypot(q[0] - p[0], q[1] - p[1]) / 6); for (let t = 0; t < n; t++) pts.push([p[0] + (q[0] - p[0]) * t / n, p[1] + (q[1] - p[1]) * t / n]); }
    pts.push(st[st.length - 1]);
    a.push({ t: 'down', x: pts[0][0], y: pts[0][1] });
    const per = Math.max(1, Math.ceil(pts.length / 16));
    for (let k = 1; k < pts.length; k++) { a.push({ t: 'move', x: pts[k][0], y: pts[k][1] }); if (k % per === 0) frame(2); }
    a.push({ t: 'up' }); frame(2); frame(2);
  }
  if (!plan.length) { a.push({ t: 'start' }); }
  while (f < FRAMES) frame(f < 120 ? 6 : f < 200 ? 9 : 12);
  return a;
}
module.exports = { PLANS, FRAMES, schedule };
