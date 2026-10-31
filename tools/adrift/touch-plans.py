#!/usr/bin/env python3
"""Writes the touch module's plans, adrift/crates/adrift-core/src/touch/plans/*.json: one played
session a gesture and an ambiguous case of VOCABULARY.md, each with the expectations that prove it,
in the form `adrift-check -- play` takes (fingers by id at a place `at` or screen points `xy`,
absolute times, `expect`s it fails on). They run on the real globe: land in them is real land.

    python3 tools/adrift/touch-plans.py                                   # rewrites the plans
    cd adrift && for p in crates/adrift-core/src/touch/plans/*.json; do   # plays them all
        cargo run -q -p adrift-check -- play $p build/out/motion/$(basename $p .json); done

Sizes are points on a phone (393 x 852, the check's default at three pixels a point). Floats are
numbered from 1 in the order listed; a put float takes the next number.
"""
import json, os

OUT = os.path.join(os.path.dirname(__file__), "../../adrift/crates/adrift-core/src/touch/plans")
W, H = 393.0, 852.0
SEA = [-30.0, 20.0]          # open sea in the Atlantic view
SEA2 = [-20.0, 15.0]
LAND = [-100.0, 40.0]        # Kansas
OFF = [6.0, 6.0]             # a corner: off the disc
PLANS = {}

def px(fx, fy):
    return [round(fx * W, 1), round(fy * H, 1)]

def date_xy(dx=0.0, w=W, h=H):
    # the date's centre at zoom 1 (Look::date_bounds and touch::date_band agree): the baseline
    # cy + r + 40 reading points, the glyphs 10 above it
    r = (min(w, h) / 2 - 14.0)
    reading = 1.0 + 0.2 * min(max((min(w, h) - 430.0) / (1032.0 - 430.0), 0.0), 1.0)
    return [w / 2 + dx, min(h / 2 + r + 40.0 * reading, h - 72.0) - 10.0]

def plan(name, title, steps, **kw):
    p = {"title": title, "size": [W, H], "camera": {"lon": -40.0, "lat": 22.0, "zoom": 1.0}, "steps": steps}
    p.update(kw)
    PLANS[name] = p

def ev(t, kind, finger=0, at=None, xy=None):
    s = {"t": t, kind: finger}
    if at is not None: s["at"] = at
    if xy is not None: s["xy"] = xy
    return s

def expect(t, **e):
    if "not_" in e: e["not"] = e.pop("not_")
    return {"t": t, "expect": e}

def shot(t, name):
    return {"t": t, "shot": name}

def drag(t0, finger, points, dt=0.05, up=True):
    """Moves through screen fractions, then lifts."""
    steps = []
    for i, (fx, fy) in enumerate(points):
        steps.append(ev(t0 + dt * (i + 1), "move", finger, xy=px(fx, fy)))
    if up:
        steps.append(ev(t0 + dt * (len(points) + 1), "up", finger, xy=px(*points[-1])))
    return steps

F1 = {"lon": SEA[0], "lat": SEA[1]}
F2 = {"lon": SEA2[0], "lat": SEA2[1]}

# --- taps and the beat ---
plan("tap-rings", "A tap on the sea rings and changes nothing", [
    ev(0.0, "down", at=SEA), shot(0.05, "ring"), ev(0.12, "up", at=SEA),
    expect(0.5, floats=0, state="idle", saw=["Ring"], not_=["Put"], camera_moved=False),
])
plan("put", "Rest a beat on the sea: the ring thickens into a float; lift and the sea has it", [
    ev(0.0, "down", at=SEA), shot(0.2, "thickening"), shot(0.39, "almost"),
    expect(0.45, floats=1, state="holding", saw=["Put"], held=1),
    shot(0.6, "held"), ev(1.0, "up", at=SEA),
    expect(1.1, floats=1, state="idle", saw=["Release"], held=None, float={"id": 1, **F1}, camera_moved=False),
    shot(1.5, "afloat"),
])
plan("almost", "A lift just short of the beat is a tap: no float", [
    ev(0.0, "down", at=SEA), ev(0.35, "up", at=SEA),
    expect(0.6, floats=0, saw=["Ring"], not_=["Put"]),
])
plan("land-silent", "Land and off the disc are silent: no ring, no float; a drag is still the globe's", [
    ev(0.0, "down", at=LAND), ev(0.1, "up", at=LAND),
    expect(0.2, floats=0, not_=["Ring"]),
    ev(0.5, "down", at=LAND), ev(1.5, "up", at=LAND),
    expect(1.6, floats=0, not_=["Ring", "Put", "Hold"], camera_moved=False),
    ev(2.0, "down", at=LAND), *drag(2.0, 0, [(0.4, 0.55), (0.5, 0.57)]),
    expect(2.4, floats=0, camera_moved=True, not_=["Ring", "Put"]),
    ev(3.0, "down", xy=OFF), ev(4.0, "up", xy=OFF),
    expect(4.1, floats=0, not_=["Ring", "Put"]),
])

# --- the drag that begins on a float ---
plan("orbit-from-float", "A drag that begins on a float before the beat is the globe's: the float stays", [
    ev(0.0, "down", at=SEA), *drag(0.0, 0, [(0.55, 0.5), (0.6, 0.52), (0.65, 0.54)]),
    expect(0.3, floats=1, state="idle", camera_moved=True, not_=["Hold", "Carry", "Release", "Ride"], float={"id": 1, **F1}),
    shot(0.4, "turned"),
], floats=[F1])
plan("orbit-then-rest", "Once a drag, always a drag: a finger that moves and then rests puts nothing", [
    ev(0.0, "down", at=SEA), *drag(0.0, 0, [(0.55, 0.5), (0.6, 0.52)], up=False), ev(1.5, "up", xy=px(0.6, 0.52)),
    expect(1.6, floats=0, not_=["Put"], camera_moved=True),
])

# --- hold and carry ---
plan("hold-maybes", "Rest a beat on a float: you hold it (its maybes); lift and nothing changed", [
    ev(0.0, "down", at=SEA), expect(0.5, state="holding", saw=["Hold"], held=1, not_=["Ring"]),
    shot(0.8, "maybes"), ev(1.2, "up", at=SEA),
    expect(1.3, floats=1, state="idle", saw=["Release"], held=None, float={"id": 1, **F1}, camera_moved=False),
], floats=[F1])
plan("carry", "Rest, then drag: carry the float to other water; the globe does not move; undo takes it back", [
    ev(0.0, "down", at=SEA), expect(0.5, state="holding", saw=["Hold"]),
    ev(0.6, "move", at=[-25.0, 18.0]), ev(0.7, "move", at=SEA2), shot(0.75, "carrying"),
    expect(0.75, state="carrying", saw=["Carry"], held=1, camera_moved=False),
    ev(0.8, "up", at=SEA2),
    expect(0.9, floats=1, state="idle", saw=["Release"], held=None, float={"id": 1, **F2}, camera_moved=False),
    shot(1.0, "set-down"),
    {"t": 1.5, "key": "undo"}, expect(1.6, floats=1, saw=["Undo"], float={"id": 1, **F1}),
], floats=[F1])
plan("carry-fresh", "The float just put is already in hand: drag it before lifting", [
    ev(0.0, "down", at=SEA), expect(0.5, floats=1, state="holding", saw=["Put"]),
    ev(0.6, "move", at=SEA2), ev(0.8, "up", at=SEA2),
    expect(0.9, floats=1, saw=["Carry", "Release"], float={"id": 1, **F2}, camera_moved=False),
])
plan("carry-to-land", "A carry released over land goes back where it was lifted", [
    ev(0.0, "down", at=SEA), expect(0.5, saw=["Hold"]),
    ev(0.6, "move", at=[-50.0, 20.0]), ev(0.7, "move", at=LAND), expect(0.75, saw=["Carry"], state="carrying"),
    ev(0.8, "up", at=LAND),
    expect(0.9, floats=1, saw=["Release"], float={"id": 1, **F1}, held=None),
], floats=[F1])
plan("take-out", "Carry a float off the rim and it is taken out; undo brings it back", [
    ev(0.0, "down", at=SEA), expect(0.5, saw=["Hold"]),
    ev(0.6, "move", at=[-60.0, 40.0]), ev(0.7, "move", xy=OFF), shot(0.75, "over-the-rim"),
    expect(0.75, state="carrying", floats=1),
    ev(0.9, "move", at=[-60.0, 40.0]), expect(0.95, saw=["Carry"]),          # back over the water: in hand again
    ev(1.0, "move", xy=OFF), ev(1.1, "up", xy=OFF),
    expect(1.2, floats=0, saw=["TakeOut"], held=None),
    shot(1.3, "gone"),
    {"t": 1.5, "shake": True}, expect(1.6, floats=1, saw=["Undo"], float={"id": 1, **F1}),   # back as before the touch
], floats=[F1])
plan("cancel", "The system takes the touch: a press lets go, a hold puts the float back, a drag stops", [
    ev(0.0, "down", at=SEA2), ev(0.2, "cancel", at=SEA2), expect(0.3, floats=1, saw=["Ring"], not_=["Put"]),
    ev(0.5, "down", at=SEA), expect(1.0, saw=["Hold"]), ev(1.1, "move", at=[-25.0, 25.0]), ev(1.2, "cancel", at=[-25.0, 25.0]),
    expect(1.3, floats=1, saw=["Release"], float={"id": 1, **F1}, held=None, camera_moved=False),
    ev(1.5, "down", at=[-45.0, 0.0]), ev(1.55, "move", xy=px(0.5, 0.6)), ev(1.6, "move", xy=px(0.58, 0.62)), ev(1.62, "cancel", xy=px(0.58, 0.62)),
    expect(1.7, camera_moved=True), expect(2.5, camera_same_as_last=True),   # no fling
], floats=[F1])

# --- the ride ---
plan("ride", "Tap a float and the camera drifts with it; any drag leaves it", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.1, saw=["Ride"], riding=1, not_=["Put"]),
    shot(1.0, "catching-up"),
    expect(3.0, riding=1, camera_near_float=[1, 0.5], camera_moved=True), shot(3.0, "riding"),
    ev(3.1, "down", at=[-45.0, 0.0]), *drag(3.1, 0, [(0.5, 0.6), (0.58, 0.62)]),
    expect(3.4, riding=None),
    expect(6.0, riding=None),
], floats=[F1])
plan("ride-and-hold-another", "Riding one float, hold another: the ride goes on; carry it and the ride is left", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.1, riding=1),
    ev(0.5, "down", at=SEA2), expect(1.0, saw=["Hold"], riding=1, held=2),
    ev(1.1, "move", at=[-18.0, 12.0]), expect(1.2, saw=["Carry"]),
    ev(1.3, "up", at=[-18.0, 12.0]), expect(1.4, saw=["Release"], floats=2),
], floats=[F1, F2])
plan("tap-near-two", "A tap near two floats takes the nearest", [
    ev(0.0, "down", at=[-30.4, 20.0]), ev(0.08, "up", at=[-30.4, 20.0]), expect(0.1, riding=2),
], floats=[{"lon": -30.0, "lat": 20.0}, {"lon": -30.6, "lat": 20.0}])

# --- the globe's own camera ---
plan("fling", "Let go moving and the globe flings, then rests; a touch stops it at once", [
    ev(0.0, "down", at=[-45.0, 0.0]), *drag(0.0, 0, [(0.5, 0.6), (0.56, 0.61), (0.62, 0.62)]),
    expect(0.3, camera_moved=True), expect(0.6, camera_same_as_last=False),           # still going
    ev(0.7, "down", xy=px(0.62, 0.42)), ev(0.75, "up", xy=px(0.62, 0.42)),             # a tap (on the Atlantic, wherever the fling went): it stops, and the sea rings
    expect(0.8, saw=["Ring"]), expect(1.5, camera_same_as_last=True),
])
plan("pinch", "Two fingers zoom and turn; a second finger before the beat puts no float", [
    ev(0.0, "down", 0, xy=px(0.42, 0.5)), ev(0.05, "down", 1, xy=px(0.58, 0.5)),
    expect(0.1, state="pinch"),
    ev(0.15, "move", 0, xy=px(0.38, 0.5)), ev(0.2, "move", 1, xy=px(0.64, 0.5)), ev(0.3, "move", 0, xy=px(0.34, 0.5)),
    expect(0.35, zoom_gt=1.3, floats=0),
    ev(0.4, "up", 1, xy=px(0.64, 0.5)), expect(0.45, state="orbit"),
    ev(0.5, "move", 0, xy=px(0.4, 0.56)), ev(0.6, "up", 0, xy=px(0.4, 0.56)),
    expect(1.2, floats=0, not_=["Put"]),
])
plan("double-tap", "A double tap flies in about the point; from deep in, out to the whole globe", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), ev(0.2, "down", at=SEA), ev(0.28, "up", at=SEA),
    expect(0.5, zoom_gt=1.05, floats=0), shot(0.6, "flying"),
    expect(1.1, camera={"lon": SEA[0], "lat": SEA[1], "zoom": 2.0}),
    ev(2.0, "down", at=SEA), ev(2.08, "up", at=SEA), ev(2.2, "down", at=SEA), ev(2.28, "up", at=SEA),
    expect(3.1, camera={"lon": SEA[0], "lat": SEA[1], "zoom": 4.0}),
])
plan("double-tap-out", "Past half the deepest, a double tap flies out to the whole globe about the centre", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), ev(0.2, "down", at=SEA), ev(0.28, "up", at=SEA),
    expect(1.1, camera={"lon": -32.0, "lat": 20.0, "zoom": 1.0}),
], camera={"lon": -32.0, "lat": 20.0, "zoom": 7.0})
plan("idle-turn", "Left alone with nothing on the sea the globe turns; with a float, it holds still", [
    expect(7.9, camera_moved=False), expect(9.0, camera_moved=True),
    ev(9.1, "down", at=SEA), ev(9.6, "up", at=SEA), expect(9.7, floats=1),
    expect(19.0, camera_same_as_last=True),
])
plan("reduce-motion", "Reduce Motion: no fling, flights are cuts; the beat is unchanged", [
    ev(0.0, "down", at=[-45.0, 0.0]), *drag(0.0, 0, [(0.5, 0.6), (0.56, 0.61)]),
    expect(0.2, camera_moved=True), expect(1.0, camera_same_as_last=True),
    ev(1.1, "down", at=SEA), ev(1.18, "up", at=SEA), ev(1.3, "down", at=SEA), ev(1.38, "up", at=SEA),
    expect(1.4, camera={"lon": SEA[0], "lat": SEA[1], "zoom": 2.0}),
    ev(2.0, "down", at=SEA), expect(2.5, floats=1, saw=["Put"]), ev(2.6, "up", at=SEA),
    expect(11.0, camera_same_as_last=True),
], reduce_motion=True)

# --- the date and undo ---
plan("date-scrub", "Drag the date: the clock pauses and the day follows the finger; it stays paused", [
    ev(0.0, "down", xy=date_xy()), ev(0.1, "move", xy=date_xy(50)), ev(0.2, "move", xy=date_xy(100)),
    shot(0.22, "scrubbed"),
    expect(0.25, saw=["Pause", "Scrub"], state="scrubbing", playing=False, day=9900.0),
    ev(0.3, "up", xy=date_xy(100)), expect(0.4, playing=False, state="idle", camera_moved=False, floats=0),
], day=9800.0)
plan("date-tap", "Tap the date to pause, tap again to play", [
    ev(0.0, "down", xy=date_xy()), ev(0.1, "up", xy=date_xy()), shot(0.15, "paused"), expect(0.2, saw=["Pause"], playing=False),
    ev(0.5, "down", xy=date_xy()), ev(0.6, "up", xy=date_xy()), expect(0.7, saw=["Play"], playing=True),
], day=9800.0)
three = lambda t, fx: [ev(t, "down" if fx is None else "move", i, xy=px(0.3 + 0.15 * i + (fx or 0), 0.6)) for i in range(3)]
plan("undo-gestures", "Undo by three-finger swipe, by shake, by the keyboard: one touch at a time", [
    ev(0.0, "down", at=SEA), ev(0.5, "up", at=SEA), expect(0.6, floats=1),
    *[ev(1.0 + 0.02 * i, "down", i, xy=px(0.3 + 0.15 * i, 0.6)) for i in range(3)],
    *[ev(1.1 + 0.02 * i, "move", i, xy=px(0.3 + 0.15 * i - 0.08, 0.6)) for i in range(3)],
    *[ev(1.2 + 0.02 * i, "move", i, xy=px(0.3 + 0.15 * i - 0.16, 0.6)) for i in range(3)],
    expect(1.3, saw=["Undo"], floats=0),
    *[ev(1.4 + 0.02 * i, "up", i, xy=px(0.3 + 0.15 * i - 0.16, 0.6)) for i in range(3)],
    expect(1.5, state="idle", not_=["Put", "Ring"]),
    ev(2.0, "down", at=SEA), ev(2.5, "up", at=SEA), expect(2.6, floats=1),
    {"t": 3.0, "shake": True}, expect(3.1, saw=["Undo"], floats=0),
    ev(3.5, "down", at=SEA2), ev(4.0, "up", at=SEA2), expect(4.1, floats=1),
    {"t": 4.5, "key": "undo"}, expect(4.6, saw=["Undo"], floats=0),
])

# --- R3: the hater's list on the merged build ---
IPAD = (1032.0, 1376.0)
plan("date-ipad", "The date on the 13-inch iPad: a tap pauses, a drag scrubs, against the drawn date", [
    ev(0.0, "down", xy=date_xy(0, *IPAD)), ev(0.1, "up", xy=date_xy(0, *IPAD)), shot(0.15, "paused"),
    expect(0.2, saw=["Pause"], playing=False, floats=0),
    ev(0.5, "down", xy=date_xy(0, *IPAD)), ev(0.6, "move", xy=date_xy(60, *IPAD)), ev(0.7, "move", xy=date_xy(120, *IPAD)),
    shot(0.72, "scrubbed"), expect(0.75, saw=["Scrub"], state="scrubbing", day=9920.0),
    ev(0.8, "up", xy=date_xy(120, *IPAD)), expect(0.9, playing=False, state="idle", camera_moved=False, floats=0),
], size=list(IPAD), day=9800.0)
GULF = {"lon": -74.0, "lat": 36.0}     # the Gulf Stream off Cape Hatteras
plan("ride-fast", "Ride a Gulf Stream float close in: the camera keeps it centred, not trailing", [
    ev(0.0, "down", at=[GULF["lon"], GULF["lat"]]), ev(0.08, "up", at=[GULF["lon"], GULF["lat"]]), expect(0.1, saw=["Ride"], riding=1),
    shot(1.0, "catching-up"), expect(2.0, riding=1, camera_near_float=[1, 0.3]),
    shot(4.0, "riding"), expect(4.0, riding=1, camera_near_float=[1, 0.3]),
    expect(6.0, riding=1, camera_near_float=[1, 0.3]), shot(6.0, "still-riding"),
], floats=[GULF], camera={"lon": GULF["lon"], "lat": GULF["lat"], "zoom": 4.0})
plan("reduce-motion-ride", "Reduce Motion: a ride follows without a glide, a double tap cuts, nothing turns by itself", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.15, riding=1, camera_near_float=[1, 0.05]),
    ev(0.5, "down", at=SEA), ev(0.58, "up", at=SEA), ev(0.7, "down", at=SEA), ev(0.78, "up", at=SEA),
    expect(0.8, zoom_gt=1.99, riding=1), shot(0.9, "cut-in"),
    ev(1.0, "down", at=[-25.0, 10.0]), *drag(1.0, 0, [(0.5, 0.6), (0.58, 0.62)]),
    expect(1.3, riding=None), expect(10.0, camera_same_as_last=True),
], floats=[F1], reduce_motion=True)

def credit_xy(w=W, h=H):
    # the credit line along the foot: its glyphs 12 reading points above the bottom edge
    reading = 1.0 + 0.2 * min(max((min(w, h) - 430.0) / (1032.0 - 430.0), 0.0), 1.0)
    return [w / 2, h - 12.0 * reading - 5.0 * reading]
plan("credits", "A tap on the credit line opens the credits; any tap closes them; the date is unchanged", [
    ev(0.0, "down", xy=credit_xy()), ev(0.1, "up", xy=credit_xy()), shot(0.15, "page"),
    expect(0.2, saw=["ToggleCredits"], not_=["Ring", "Press"], floats=0),
    ev(0.5, "down", at=SEA), ev(1.2, "up", at=SEA),                       # a rest on the sea while the page is open
    expect(1.3, not_=["Put", "Ring"], floats=0, saw=["ToggleCredits"]),   # closes it, puts nothing
    shot(1.35, "closed"),
    ev(1.5, "down", at=SEA), ev(1.6, "up", at=SEA), expect(1.7, saw=["Ring"], not_=["ToggleCredits"]),
    ev(2.0, "down", xy=date_xy()), ev(2.1, "up", xy=date_xy()), expect(2.2, saw=["Pause"], playing=False, not_=["ToggleCredits"]),
    ev(2.5, "down", xy=credit_xy()), ev(2.6, "up", xy=credit_xy()), expect(2.7, saw=["ToggleCredits"]),
    ev(3.0, "down", xy=date_xy()), ev(3.1, "move", xy=date_xy(50)), ev(3.2, "move", xy=date_xy(100)), ev(3.3, "up", xy=date_xy(100)),
    expect(3.4, saw=["Scrub"], not_=["ToggleCredits"]),                   # the date still scrubs under the page
    ev(3.6, "down", at=[-25.0, 10.0]), *drag(3.6, 0, [(0.5, 0.6), (0.58, 0.62)]),
    expect(3.9, camera_moved=True, not_=["ToggleCredits"]),               # a drag is the globe's, page or no page
    ev(4.2, "down", at=SEA), ev(4.3, "up", at=SEA), expect(4.4, saw=["ToggleCredits"], not_=["Ring"]),
], day=9800.0)

os.makedirs(OUT, exist_ok=True)
for f in os.listdir(OUT):
    if f.endswith(".json"):
        os.remove(os.path.join(OUT, f))
for name, p in PLANS.items():
    with open(os.path.join(OUT, name + ".json"), "w") as f:
        json.dump(p, f, indent=1)
        f.write("\n")
print(len(PLANS), "plans")
