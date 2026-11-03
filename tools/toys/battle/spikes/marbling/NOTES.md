# Marbling (spike)

**The idea.** Cannae as a tray of marbling size (ebru). Your first touch flicks the brush: a pale spatter of ground, then each army falls in as drops of floating pigment in its order of battle, ringing outward (the Roman maniples packed into "stones", Hannibal's bowed crescent of Gauls and Spaniards alternating, Africans held back on the wings, the cavalry by the river and on the far flank). Your finger is a stylus: dragging it through the size pulls the pigment along, so a battle plan turns into a marbled pattern. Holding still lays a sheet down and lifts the print, and the page is left as an artwork. A touch on the print starts a new tray.

**What's new.** No units, arrows or icons. The plan exists only as the deformation of the pigment. Where armies meet, the colours fold into feathered chevrons (the cavalry's sweep behind the legions, the centre pushed back into the crescent), and the encirclement winds the Roman red into a spiral bouquet. Each run is a unique marbled sheet.

**Underneath (real).** Terrarium elevation round 41.30 N 16.13 E (`build_ground.py`): the Ofanto valley floor, the river at its lowest line and the Cannae rise, all faint under the size. The positions, depths and wings follow the accounts.

**Faked or chosen.** The ground is turned so the account reads on screen (the river on the Roman right, Romans above facing down), so north is not up. The river's meanders are drawn and the colours are an ebru palette, not uniforms (one exception: the Spaniards' white linen bordered with purple). The scripted plan is mine. Drops use Jaffer's exact ink-preserving drop map. The stylus is a smooth near-area-preserving falloff map (λ/(d+λ)·e^(−d/2.5λ)), not true fluid flow.

**The look.** Pigment is drawn as exact polygons (adaptively refined, about 190k vertices at the end), then one WebGL pass adds rims where colours meet, flocculated grain, slight bleed, a thin dispersant line round each drop, size texture, the tray wall, and for the print, paper fibre and uneven soak.

**Run.** `node render.cjs` (about 9 min in SwiftShader: the 2x moments, then the 1x film). Open `index.html` over http to touch it.
