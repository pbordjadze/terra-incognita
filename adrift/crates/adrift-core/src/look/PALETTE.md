# Visual: the palette, with its contrast measured

The colours are in `palette.json` (one spec: the look's Rust reads it, `tools/adrift/study.py`
mirrors it, `tools/adrift/palette.py` holds it to the rule). The sea's own tones are Terra's
(`Globe.metal`'s `G_` and `N_` palettes), which the ground pass keeps; everything here is laid on
them. The rule (CLAUDE.md › Design rules): an ink 4.5:1 against its ground (the date, the credits:
Libertinus Serif on a halo of the open sea's tone), a mark that must be read at a glance 3:1
(WCAG 1.4.11: the float's rim against the sea, the lamp against the rim, a bead's edge, the dark
line under the silver, a speck's grain). A wash or a glow (the glass, the lamp's light on the
water, what stays lit) owes nothing on its own: it is read through the rim or the bead that
carries it.

Why these: the float is the one warm thing on a cool sea, so the amber is the only warm hue
anywhere (the lamp, the beads, the maybes' pale amber, what stays lit), and everything else is
the sea's family (the rim dark sea glass, the glass a wash of aqua, the silver cool, the inks a
blue-black). On the pale day sea an amber disc alone is 2:1 and vanishes at the whole Earth: the
dark rim is what makes the float readable at a glance, and the dark edge is what makes a bead a
bead rather than a smudge. By night the sea is dark and the rim goes pale: the lamp is its own
light, the beads need no edge, and the maybes drop to half strength or they shout.

`python3 tools/adrift/palette.py` prints the table below and fails below the rule (CI should run it).

| style | colour | hex | against | ground | owes | has | |
| --- | --- | --- | --- | --- | --- | --- | --- |
| day | glass_rim | `#1E4A58` | shelf | `#D3F0EE` | 3.0:1 | 8.0:1 | ok |
| day | glass_rim | `#1E4A58` | open | `#C2E4F4` | 3.0:1 | 7.2:1 | ok |
| day | glass_rim | `#1E4A58` | deep | `#A9D2EC` | 3.0:1 | 6.0:1 | ok |
| day | glass_rim | `#1E4A58` | ice | `#F1F5F8` | 3.0:1 | 8.8:1 | ok |
| day | lamp_core | `#FFE9B8` | glass_rim | `#1E4A58` | 3.0:1 | 8.1:1 | ok |
| day | lamp | `#F4A52A` | glass_rim | `#1E4A58` | 3.0:1 | 4.7:1 | ok |
| day | bead_edge | `#8C4F10` | shelf | `#D3F0EE` | 3.0:1 | 5.4:1 | ok |
| day | bead_edge | `#8C4F10` | open | `#C2E4F4` | 3.0:1 | 4.8:1 | ok |
| day | bead_edge | `#8C4F10` | deep | `#A9D2EC` | 3.0:1 | 4.0:1 | ok |
| day | bead_edge | `#8C4F10` | ice | `#F1F5F8` | 3.0:1 | 5.9:1 | ok |
| day | maybe_core | `#8E5712` | shelf | `#D3F0EE` | 3.0:1 | 5.0:1 | ok |
| day | maybe_core | `#8E5712` | open | `#C2E4F4` | 3.0:1 | 4.5:1 | ok |
| day | maybe_core | `#8E5712` | deep | `#A9D2EC` | 3.0:1 | 3.7:1 | ok |
| day | silver_edge | `#4A5663` | shelf | `#D3F0EE` | 3.0:1 | 6.2:1 | ok |
| day | silver_edge | `#4A5663` | open | `#C2E4F4` | 3.0:1 | 5.6:1 | ok |
| day | silver_edge | `#4A5663` | deep | `#A9D2EC` | 3.0:1 | 4.7:1 | ok |
| day | silver_edge | `#4A5663` | ice | `#F1F5F8` | 3.0:1 | 6.8:1 | ok |
| day | date_ink | `#26303A` | open | `#C2E4F4` | 4.5:1 | 10.0:1 | ok |
| day | date_ink | `#26303A` | shelf | `#D3F0EE` | 4.5:1 | 11.2:1 | ok |
| day | date_ink | `#26303A` | ice | `#F1F5F8` | 4.5:1 | 12.2:1 | ok |
| day | credit_ink | `#3E4C5A` | open | `#C2E4F4` | 4.5:1 | 6.6:1 | ok |
| day | credit_ink | `#3E4C5A` | shelf | `#D3F0EE` | 4.5:1 | 7.3:1 | ok |
| day | credit_ink | `#3E4C5A` | ice | `#F1F5F8` | 4.5:1 | 8.0:1 | ok |
| night | glass_rim | `#B9DDE3` | sea | `#101828` | 3.0:1 | 12.3:1 | ok |
| night | glass_rim | `#B9DDE3` | deep | `#09101E` | 3.0:1 | 13.1:1 | ok |
| night | glass_rim | `#B9DDE3` | ice | `#444E60` | 3.0:1 | 5.8:1 | ok |
| night | lamp_core | `#FFF0C8` | sea | `#101828` | 3.0:1 | 15.7:1 | ok |
| night | lamp_core | `#FFF0C8` | deep | `#09101E` | 3.0:1 | 16.8:1 | ok |
| night | lamp | `#F6AE3A` | sea | `#101828` | 3.0:1 | 9.3:1 | ok |
| night | lamp | `#F6AE3A` | deep | `#09101E` | 3.0:1 | 10.0:1 | ok |
| night | bead | `#F0A040` | sea | `#101828` | 3.0:1 | 8.3:1 | ok |
| night | bead | `#F0A040` | deep | `#09101E` | 3.0:1 | 8.9:1 | ok |
| night | maybe | `#E9D3AE` | sea | `#101828` | 3.0:1 | 12.2:1 | ok |
| night | maybe | `#E9D3AE` | deep | `#09101E` | 3.0:1 | 13.0:1 | ok |
| night | silver | `#DCE3EA` | sea | `#101828` | 3.0:1 | 13.7:1 | ok |
| night | silver | `#DCE3EA` | deep | `#09101E` | 3.0:1 | 14.7:1 | ok |
| night | date_ink | `#E4ECF4` | sea | `#101828` | 4.5:1 | 14.9:1 | ok |
| night | date_ink | `#E4ECF4` | deep | `#09101E` | 4.5:1 | 15.9:1 | ok |
| night | credit_ink | `#B8C6D4` | sea | `#101828` | 4.5:1 | 10.2:1 | ok |
| night | credit_ink | `#B8C6D4` | deep | `#09101E` | 4.5:1 | 10.9:1 | ok |
