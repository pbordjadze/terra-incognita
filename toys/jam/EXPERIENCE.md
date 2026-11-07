# The cars toy, round three: the experience (row 255, Nov 7, 2026)

Written before reading the tools, as the row asks. Working title: **Shortcut**.

## What Paul should find

A small town at night seen from above, filling a phone. Cars come in at the bottom edge, cross the
town and leave at the top: a steady stream, never a loop. Between the two ends lies a lake. Two ways
round it: on the left a short narrow street and then a long wide road; on the right a long wide road
and then a short narrow street. The narrow streets are quick when empty and slow as they fill; the
wide roads take the same time however busy. Half the drivers go each way, and it flows.

Each car is a small light. It leaves the bottom white and warms toward red the longer its trip runs:
a quick trip arrives gold, a slow one arrives red. The town at the top glows in the colour its
arrivals bring. So without a word, white means quick and red means slow, and the whole picture's
colour is everyone's trip.

The obvious thing to want is a bridge across the lake: a shortcut joining the two narrow streets.
Paul draws it with his finger. Every driver, choosing for themselves the way that looks fastest, takes
it: up the left street, over the bridge, down the right street. Both streets fill, the wide roads go
empty, and within half a minute the whole town is redder: everyone's trip got slower, and nobody
chose anything but the quickest way. He slashes across the bridge to close it, and the colour
cools. That is Braess's paradox, and it is not scripted: it falls out of each driver's rule.

Then it is a toy: draw any road between two roads (a second bridge, a road up the middle, a
detour), slash any road closed, and watch what the drivers do with it. Some roads help, some hurt,
and the only way to know is to try. Shake to put the town back.

## The rules underneath (the truth, credited in SOURCES.md, never printed)

- A road's time grows with the cars on it: each car on a road adds a fixed delay to everyone on it
  (Braess 1968's linear link cost; Pigou's and Wardrop's selfish routing; the traffic engineers'
  volume-delay curve is the same idea). A narrow street adds much per car, a wide road little.
- Drivers are selfish and local: at every junction each takes the road whose time, as it looks now
  plus the quickest way on from its end, is least, seen with a little personal error. No one
  coordinates. Wardrop's equilibrium emerges; so does the paradox.
- Cars keep a gap and queue where a road is full.
- Cars enter at a steady rate. The colour is the car's trip clock against the town's old trip.

## The gesture

One finger: draw from a road to a road to build one; slash across a road to close it. The "i" shows
a bridge drawn across the lake; the "?" gives the rules above in three plain lines and "shake to put
the roads back".

## Taste checks

- First five seconds: a lit town with a living stream of cars, beautiful before anything is touched.
- No text, no chrome, no score. The colour is the score.
- Would he discover that his road made it worse? The test: a newcomer who draws the obvious bridge
  sees the town redden within ~20 s, and sees it cool when the bridge is cut.
- No closed loop. The fun of trying to fix a jam: there should always be something a little slow
  to tempt a fix.
