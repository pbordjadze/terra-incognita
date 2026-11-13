# Kanto: sources

What is measured, what is modelled, what is drawn, and from where. The plate's data is built by
`tools/toys/kanto/build.py` (its docstring is the layout); the mold follows the model in `src/net.rs`.

## The experiment (the paper)

- A. Tero, S. Takagi, T. Saigusa, K. Ito, D. P. Bebber, M. D. Fricker, K. Yumiki, R. Kobayashi, T. Nakagaki,
  "Rules for biologically inspired adaptive network design", *Science* 327 (2010) 439–442. The slime mold
  *Physarum polycephalum* was put down at Tokyo's place on a plate with oat flakes at the region's towns;
  the mountains and other ground it should not cross were lit, as the mold shies from light. It spread,
  found the flakes, and thinned to a network of tubes the authors compared with the region's rail network:
  much like it in cost, efficiency and how well it survives a break.

## The plate (measured data)

- **The land**: Natural Earth 10 m land (public domain); the sea is off the plate.
- **The ground's height** (the lit mountains): AWS Terrain Tiles, Terrarium encoding, zoom 9 (made from SRTM
  and other open elevation data; Mapzen's attributions); lit from 150 m, fully at 700 m.
- **The towns** (the oat flakes): GeoNames (CC BY 4.0), Japan's cities (`JP.zip`, the seats of the
  prefectures and municipalities), the most populous first, none within 14 km of another, 36 of them as in
  the experiment; Tokyo first.
- **The railways**: Natural Earth 10 m railroads (public domain), the main lines on the plate.

## The mold (the paper's model)

- **The tubes' rule** (Tero et al. 2010): the plasmodium as a mesh of tubes with conductivities D; food
  sources drive a flow through it, the pressures by Kirchhoff's law and Poiseuille flow (Q = D/L · Δp);
  each tube's D grows with its flux and decays without it, dD/dt = f(|Q|) − D, f(Q) = (1+a)Q^γ/(1+aQ^γ).
- **Here**: each step one reached flake is the source of the flux and the others draw it off alike; a = 1,
  γ = 1.6 (γ > 1 thins the mesh, merging parallel tubes into one; nearer 1 keeps more loops); a lit tube's growth held
  back by up to 85%. These are chosen, not fitted to the paper's runs.

## Drawn (not measured)

- **The sheet's spread** from Tokyo before the tubes: a front over the agar, slower on lit ground and
  never onto the brightest; its speed is a pace for the toy (the real mold took about a day).
- **The mesh**: a jittered triangular lattice about 3 km apart; a tube's drawn path is eased toward its
  neighbours so a run reads as one tube.
- **The colours**: the mold's yolk yellow (Physarum's plasmodium is yellow), the dim amber agar, the cool
  light on the mountains, the flakes, the railways' thin red.
- **A finger's flake**: laid where the reader taps; up to eight, the oldest eaten after.
