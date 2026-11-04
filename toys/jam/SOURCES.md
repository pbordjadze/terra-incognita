# Ghost Jam: sources

What is measured and what is drawn. The cars, the ground and the sound are drawn from code; nothing
is a photograph or a recording.

## The ring (measured)

- Y. Sugiyama, M. Fukui, M. Kikuchi, K. Hasebe, A. Nakayama, K. Nishinari, S. Tadaki and S. Yukawa,
  "Traffic jams without bottlenecks: experimental evidence for the physical mechanism of the formation
  of a jam", *New Journal of Physics* 10 (2008) 033001 (open access, CC BY-NC-SA). From the paper:
  - a circular single-lane road on flat ground, **230 m** round (here a radius of 36.6 m), with
    **22** cars; a 360-degree camera at the centre (drawn here as a tripod at the ring's centre);
  - the drivers were asked to cruise at about **30 km/h**, evenly spaced, and only to follow the car
    ahead safely;
  - after a while a jam formed with no cause: a cluster of cars (five, in the run shown) stopped
    completely while the cars outside it ran at about **40 km/h**; the front car pulls away as another
    joins the back, and the cluster travels **backward at about 20 km/h**, the speed measured for jams
    on real highways, keeping its size.
- The paper's group are from Nagoya University and Meijo University (Nagoya); the paper itself doesn't
  name the test ground, so the toy shows no map.

## The drivers (a model, tuned to the measurements)

- One rule each, the Intelligent Driver Model: M. Treiber, A. Hennecke and D. Helbing, "Congested traffic
  states in empirical observations and microscopic simulations", *Physical Review E* 62 (2000) 1805.
  It belongs to the same family as the Optimal Velocity model the paper compares with (M. Bando,
  K. Hasebe, A. Nakayama, A. Shibata and Y. Sugiyama, *Physical Review E* 51 (1995) 1035): a driver
  speeds toward the speed its gap allows. IDM was chosen because its cars never collide.
- The constants (desired speed 15 m/s, acceleration 2 m/s², comfortable braking 3 m/s², time gap 0.5 s,
  standstill gap 1.5 m, cars 4.4 m long) are chosen so the ring matches the paper: even flow at 31 km/h;
  once jammed, standing cars, about 40–45 km/h outside the jam, and the jam moving backward at about 20 km/h (the tests hold it between 15 and 26).
  `cargo test -p jam` holds them to that (`src/traffic.rs`).
- **One departure from the real ring.** In Nagoya the even flow was unstable, so the drivers' own small
  wobbles grew into the jam after a few minutes. Here the flow is held just on the stable side
  (metastable): the drivers wobble (a small random push on each car's speed) and the flow absorbs it,
  but one hard brake (6.5 m/s² for 1.2 s, your tap) grows into a jam that then keeps itself going as
  the real one did. So a jam starts only where you tap.
- Jams are emergent: the code has no notion of a jam, only of each car and the car ahead.

## Drawn

- The hour (dusk; the experiment ran by day), the paving, the ring's asphalt and painted edges, the cars'
  shapes and colours (drawn 1.25 times as long and 1.45 times as wide as life, the road wider with them,
  so they read on a phone), headlights, brake lights (lit while a car brakes harder than 1.8 m/s² or
  stands), a touch's ring, and, until the first touch, a soft halo breathing round one car.
- Inside the ring, the last 40 seconds: each car's position every 0.2 s drawn at a radius that shrinks
  with its age (a faint line), and a soft red glow, spread along the ring by a car's spacing, where its brake lights were on. It is the paper's own kind of figure (its figure 4,
  each car's position against time) wrapped round the ring: the cars' tracks wind forward, a jam's red
  arm winds backward.
- The sound: each car an engine buzz whose pitch follows its speed and grows louder as it pulls away,
  a car pulling out of a stop revving loudest (so the jam's front is heard as one car after another pulling away), tyres' hiss with the ring's speed, a short tyre chirp for each car braking harder than 1.8 m/s² (so a jam is heard as one chirp after another as cars arrive at its back), a soft thump for a tap; heard from the centre.

## Known

- Tapping many cars within a few seconds can leave no jam: when most of the ring stops together, the
  drivers in this model all pull away together too, and the even flow comes back. Real drivers wait for
  the car ahead to move off; a "slow to start" rule was tried (Nov 4) and made jams too big and too slow
  (about 15 km/h backward) for the paper's numbers, so it was left out. `cargo test -p jam probe_many
  -- --ignored --nocapture` shows it.
