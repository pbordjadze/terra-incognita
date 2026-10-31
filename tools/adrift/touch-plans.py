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
    ev(0.0, "down", at=SEA), shot(0.2, "rising"), expect(0.5, state="holding", saw=["PressFloat", "Hold"], held=1, not_=["Ring", "Unpress"]),
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
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.1, saw=["PressFloat", "Unpress", "Ride"], riding=1, not_=["Put", "Hold"]),
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
    ev(0.7, "down", at=SEA), ev(0.75, "up", at=SEA),                                   # a tap on the Atlantic, wherever the fling took it: it stops, and the sea rings
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
# (R6: under the speed limit the date reaches a fast finger a moment later, so the finger rests a
# tenth before the check; let go still, the clock is given back as it was, running)
plan("date-scrub", "Drag the date: the clock pauses and the day follows the finger; let go still, the clock is given back as it was (running) (R6: under the speed limit the date reaches the finger a moment after a fast drag)", [
    ev(0.0, "down", xy=date_xy()), ev(0.1, "move", xy=date_xy(50)), ev(0.2, "move", xy=date_xy(100)),
    shot(0.32, "scrubbed"),
    expect(0.35, saw=["Pause", "Scrub"], state="scrubbing", playing=False, day=9900.0),
    ev(0.4, "up", xy=date_xy(100)), expect(0.5, playing=True, state="idle", camera_moved=False, floats=0),
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
plan("date-ipad", "The date on the 13-inch iPad: a tap pauses, a drag scrubs, against the drawn date (R6: under the speed limit the date reaches the finger a moment after a fast drag)", [
    ev(0.0, "down", xy=date_xy(0, *IPAD)), ev(0.1, "up", xy=date_xy(0, *IPAD)), shot(0.15, "paused"),
    expect(0.2, saw=["Pause"], playing=False, floats=0),
    ev(0.5, "down", xy=date_xy(0, *IPAD)), ev(0.6, "move", xy=date_xy(60, *IPAD)), ev(0.7, "move", xy=date_xy(120, *IPAD)),
    shot(0.84, "scrubbed"), expect(0.87, saw=["Scrub"], state="scrubbing", day=9920.0),
    ev(0.92, "up", xy=date_xy(120, *IPAD)), expect(1.02, playing=False, state="idle", camera_moved=False, floats=0),
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
plan("credits", "A rest on the date opens the credits; any tap closes them; the date's tap, drag and flick are unchanged", [
    ev(0.0, "down", xy=date_xy()), shot(0.3, "resting"), expect(0.5, saw=["ToggleCredits"], not_=["Pause", "Scrub"], state="idle"),
    ev(0.6, "up", xy=date_xy()), shot(0.65, "page"), expect(0.7, not_=["Pause", "Play"]),
    ev(1.0, "down", at=SEA), ev(1.7, "up", at=SEA),                       # a rest on the sea while the page is open
    expect(1.8, not_=["Put", "Ring"], floats=0, saw=["ToggleCredits"]),   # closes it, puts nothing
    shot(1.85, "closed"),
    ev(2.0, "down", at=SEA), ev(2.1, "up", at=SEA), expect(2.2, saw=["Ring"], not_=["ToggleCredits"]),
    ev(2.5, "down", xy=date_xy()), ev(2.6, "up", xy=date_xy()), expect(2.7, saw=["Pause"], playing=False, not_=["ToggleCredits"]),
    ev(3.0, "down", xy=date_xy()), ev(3.1, "move", xy=date_xy(50)), ev(3.2, "move", xy=date_xy(100)), ev(3.3, "up", xy=date_xy(100)),
    expect(3.4, saw=["Scrub"], not_=["ToggleCredits"]),                   # a drag scrubs, never opens
    ev(4.0, "down", xy=date_xy()), expect(4.5, saw=["ToggleCredits"]), ev(4.6, "up", xy=date_xy()),
    ev(5.0, "down", at=[-25.0, 10.0]), *drag(5.0, 0, [(0.5, 0.6), (0.58, 0.62)]),
    expect(5.3, camera_moved=True, not_=["ToggleCredits"]),               # a drag is the globe's, page or no page
    ev(5.6, "down", xy=date_xy()), ev(5.7, "up", xy=date_xy()), expect(5.8, saw=["ToggleCredits"], not_=["Pause", "Play"]),  # a tap on the date closes it too
], day=9800.0)

# --- R4: clear the sea, bring the globe home, the date's flick ---
HOME = {"lon": -30.0, "lat": 25.0, "zoom": 1.0}
def three(t, fy, dy=0.0):
    return [ev(t + 0.02 * i, "down", i, xy=px(0.3 + 0.15 * i, fy)) for i in range(3)], \
           [ev(t + 0.1 + 0.02 * i, "move", i, xy=px(0.3 + 0.15 * i, fy + dy)) for i in range(3)], \
           [ev(t + 0.2 + 0.02 * i, "up", i, xy=px(0.3 + 0.15 * i, fy + dy)) for i in range(3)]
plan("home", "Pinch out past the whole globe and let go, or swipe up with three fingers: the camera flies home", [
    ev(0.0, "down", 0, xy=px(0.3, 0.5)), ev(0.02, "down", 1, xy=px(0.7, 0.5)),
    ev(0.1, "move", 0, xy=px(0.4, 0.5)), ev(0.15, "move", 1, xy=px(0.6, 0.5)),
    ev(0.2, "move", 0, xy=px(0.46, 0.5)), ev(0.25, "move", 1, xy=px(0.54, 0.5)),
    expect(0.3, state="pinch", zoom_lt=0.85),
    ev(0.35, "up", 0, xy=px(0.46, 0.5)), ev(0.37, "up", 1, xy=px(0.54, 0.5)),
    expect(0.4, saw=["Home"]), shot(0.7, "flying-home"), expect(1.3, camera=HOME, state="idle"),
    ev(1.5, "down", at=[-25.0, 10.0]), *drag(1.5, 0, [(0.5, 0.6), (0.6, 0.65)]),
    expect(2.5, camera_moved=True),
    *[e for g in three(3.0, 0.6, -0.12) for e in g],
    expect(3.3, saw=["Home"], not_=["Undo", "Redo", "ClearAll"]), expect(4.2, camera=HOME),
], camera={"lon": -60.0, "lat": 10.0, "zoom": 2.0})
plan("clear", "Three fingers down the screen, or a second shake, clears the sea as one touch; undo brings every float back", [
    expect(0.1, floats=3),
    *[e for g in three(0.5, 0.5, 0.12) for e in g],
    expect(0.8, saw=["ClearAll"], floats=0, not_=["Undo", "Redo", "Home"]),
    {"t": 1.0, "key": "undo"}, expect(1.1, floats=3),
    {"t": 1.3, "key": "redo"}, expect(1.4, floats=0),
    {"t": 2.0, "shake": True}, expect(2.1, saw=["Undo"], not_=["ClearAll"], floats=3),     # one shake: undo
    {"t": 3.0, "shake": True}, expect(3.1, saw=["ClearAll"], floats=0),                   # a second within two seconds: clear
    {"t": 6.0, "shake": True}, expect(6.1, saw=["Undo"], not_=["ClearAll"], floats=3),     # long after: undo again
], floats=[F1, F2, {"lon": -40.0, "lat": 0.0}], camera={"lon": -50.0, "lat": 20.0, "zoom": 1.0})
plan("date-flick", "Flick the date and it runs on, slowing; a touch stops it", [
    ev(0.0, "down", xy=date_xy()), ev(0.05, "move", xy=date_xy(30)), ev(0.1, "move", xy=date_xy(70)), ev(0.15, "move", xy=date_xy(120)),
    ev(0.17, "up", xy=date_xy(120)),
    expect(0.2, saw=["Pause", "Scrub"], playing=False, state="idle"),
    expect(0.6, saw=["Scrub"]), shot(0.6, "running-on"),                 # still scrubbing with no finger down
    ev(0.7, "down", at=SEA), ev(0.8, "up", at=SEA),                       # a touch stops it (and rings), and the clock runs again
    expect(0.9, saw=["Ring", "Play"], playing=True), expect(1.5, not_=["Scrub"]),
    expect(3.0, not_=["Scrub"]),
], day=9800.0)

# --- R5: the clock as the finger found it; a choice that outlives the camera ---
plan("date-keeps-running", "A scrub or a flick leaves the clock as it found it: running stays running, resting stays resting", [
    ev(0.0, "down", xy=date_xy()), ev(0.1, "move", xy=date_xy(40)), ev(0.2, "move", xy=date_xy(80)),
    expect(0.25, saw=["Pause", "Scrub"], playing=False),
    ev(0.28, "move", xy=date_xy(80)), ev(0.3, "up", xy=date_xy(80)), expect(0.4, saw=["Play"], playing=True),   # it was running: it runs again
    ev(1.0, "down", xy=date_xy()), ev(1.1, "up", xy=date_xy()), expect(1.2, saw=["Pause"], playing=False),
    ev(1.5, "down", xy=date_xy()), ev(1.6, "move", xy=date_xy(40)), ev(1.7, "move", xy=date_xy(80)), ev(1.78, "move", xy=date_xy(80)), ev(1.8, "up", xy=date_xy(80)),
    expect(1.9, saw=["Scrub"], not_=["Play"], playing=False),                                 # it was resting: it rests still
    ev(2.2, "down", xy=date_xy()), ev(2.3, "up", xy=date_xy()), expect(2.4, saw=["Play"], playing=True),
    ev(2.6, "down", xy=date_xy()), ev(2.65, "move", xy=date_xy(30)), ev(2.7, "move", xy=date_xy(70)), ev(2.75, "move", xy=date_xy(120)), ev(2.77, "up", xy=date_xy(120)),
    expect(2.8, saw=["Pause", "Scrub"], playing=False), expect(3.3, saw=["Scrub"], playing=False, not_=["Play"]),   # a flick: resting while it runs on
    expect(6.0, saw=["Play"], playing=True),                                                  # at rest, running again
    ev(6.5, "down", xy=date_xy()), ev(6.55, "move", xy=date_xy(30)), ev(6.6, "move", xy=date_xy(70)), ev(6.65, "move", xy=date_xy(120)), ev(6.67, "up", xy=date_xy(120)),
    expect(6.8, playing=False), ev(7.0, "down", at=SEA), ev(7.1, "up", at=SEA),              # a touch stops the flick and gives the clock back at once
    expect(7.2, saw=["Play"], playing=True),
], day=9800.0)
plan("chosen", "Tap a float and it is chosen and ridden; pinch away and it stays chosen; tap it to let the ride go; tap the sea to unchoose", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.2, chosen=1, riding=1),
    ev(0.5, "down", 0, xy=px(0.42, 0.5)), ev(0.52, "down", 1, xy=px(0.58, 0.5)),
    ev(0.6, "move", 0, xy=px(0.38, 0.5)), ev(0.65, "move", 1, xy=px(0.62, 0.5)), ev(0.7, "up", 0, xy=px(0.38, 0.5)), ev(0.72, "up", 1, xy=px(0.62, 0.5)),
    expect(0.8, chosen=1, riding=None, zoom_gt=1.1),                                        # the pinch ended the ride, not the choice
    ev(1.0, "down", at=SEA), ev(1.08, "up", at=SEA), expect(1.2, chosen=1, riding=1),       # tap it again: ridden again
    ev(2.5, "down", at=SEA), ev(2.58, "up", at=SEA), expect(2.7, chosen=1, riding=None),    # tap the ridden one: the ride lets go, the choice stays
    ev(3.0, "down", at=SEA2), ev(3.08, "up", at=SEA2), expect(3.2, chosen=2, riding=2),     # another: chosen instead
    ev(3.5, "down", at=[-45.0, 5.0]), ev(3.58, "up", at=[-45.0, 5.0]), expect(3.7, saw=["Ring"], chosen=None, riding=None),  # the open sea: none
], floats=[F1, F2])


# --- R6: several clouds, the panel, the scrub's speed limit, the rim (painting-company row 196) ---
# The panel's rows until Visual's land: a stand-in column at the left edge, a row 36 points tall per
# lamp (the plan's `panel`, D11: the check holds it as `App::panel_bounds()`).
def row(i):
    return [8.0, 96.0 + 40.0 * i, 120.0, 36.0]
def row_xy(i):
    r = row(i)
    return [r[0] + r[2] / 2, r[1] + r[3] / 2]
NEAR = {"lon": -29.0, "lat": 20.5}      # dropped beside F1: two clouds from almost one place
# (the clock runs between scrubs, two days a second: a scrub lands 300 days from the day it began,
# which is what the `day`s below say)
def scrub(t0, finger, dxs, dt=0.02, rest=0.0):
    """A drag on the date through `dxs` points from its centre, `dt` apart, resting `rest` before the lift."""
    steps = [ev(t0, "down", finger, xy=date_xy())]
    for i, dx in enumerate(dxs):
        steps.append(ev(t0 + dt * (i + 1), "move", finger, xy=date_xy(dx)))
    t = t0 + dt * len(dxs) + rest
    if rest:
        steps.append(ev(t, "move", finger, xy=date_xy(dxs[-1])))
    steps.append(ev(t + 0.02, "up", finger, xy=date_xy(dxs[-1])))
    return steps
plan("panel-two", "The panel: tap two lamps' rows and both clouds are out; scrub them forward and back together; tap one row again and only the other stays; nothing on the panel rings or puts", [
    ev(0.0, "down", 0, xy=row_xy(0)), ev(0.08, "up", 0, xy=row_xy(0)), expect(0.2, chosen=[1], riding=None, not_=["Ring", "Press", "Put"]),
    ev(0.5, "down", 0, xy=row_xy(1)), ev(0.58, "up", 0, xy=row_xy(1)), expect(0.7, chosen=[1, 2], floats=2, not_=["Ring", "Put", "Unchoose"]),
    shot(1.2, "two-chosen"),
    *scrub(1.5, 0, [60, 120, 180, 240, 300], rest=0.6), shot(2.8, "two-forward"), expect(2.8, chosen=[1, 2], day=10104.2),
    *scrub(3.0, 0, [-60, -120, -180, -240, -300], rest=0.6), shot(4.3, "two-back"), expect(4.3, chosen=[1, 2], day=9805.7),
    ev(4.5, "down", 0, xy=row_xy(0)), ev(4.58, "up", 0, xy=row_xy(0)), expect(4.7, chosen=[2], floats=2),
    shot(5.0, "one-let-go"),
    # a rest on a row rides it, never puts a float
    ev(5.5, "down", 0, xy=row_xy(0)), expect(6.0, riding=1, chosen=[2, 1], floats=2, not_=["Put", "Ring", "Hold"]), ev(6.1, "up", 0, xy=row_xy(0)),
    # a float carried over the panel and let go there goes back where it was lifted
    ev(7.0, "down", 1, at=SEA2), expect(7.5, held=2), ev(7.6, "move", 1, xy=row_xy(2)), ev(7.7, "up", 1, xy=row_xy(2)),
    expect(7.9, floats=2, not_=["Put", "Ring"]), shot(8.0, "carried-back"),
], floats=[NEAR, F2], day=9800.0, panel=[[1] + row(0), [2] + row(1)], frames=True)
plan("choose-two-on-globe", "On the globe: tap two lamps and both are chosen, the last ridden; a pinch keeps both; scrub, both stay; the open sea lets both go", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.2, chosen=[1], riding=1),
    ev(0.6, "down", at=[NEAR["lon"], NEAR["lat"]]), ev(0.68, "up", at=[NEAR["lon"], NEAR["lat"]]), expect(0.8, chosen=[1, 2], riding=2),
    shot(2.0, "two-out"),
    *scrub(2.2, 0, [40, 80, 120], rest=0.4), expect(3.0, chosen=[1, 2]), shot(3.0, "two-scrubbed"),
    ev(3.5, "down", at=[-45.0, 5.0]), ev(3.58, "up", at=[-45.0, 5.0]), expect(3.7, saw=["Ring"], chosen=None, riding=None),
], floats=[F1, NEAR], day=9800.0, camera={"lon": -30.0, "lat": 20.0, "zoom": 1.0})
plan("scrub-limit", "The speed limit: two clouds out, the date flung 300 days in a tenth of a second goes after the finger at ten days a frame and lands exactly; flung back, the same clouds", [
    {"t": 0.0, "intent": {"Choose": {"float": 1}}}, {"t": 0.0, "intent": {"ChooseAlso": {"float": 2}}},
    shot(1.0, "before"),
    *scrub(1.1, 0, [100, 200, 300], dt=0.033, rest=0.8),
    shot(1.25, "limit-0.15s"), shot(1.4, "limit-0.3s"), shot(1.7, "limit-0.6s"), expect(1.95, day=10102.2, chosen=[1, 2]),
    *scrub(2.2, 0, [-100, -200, -300], dt=0.033, rest=0.8), expect(3.1, day=9802.6, chosen=[1, 2]), shot(3.1, "back"),
], floats=[F1, NEAR], day=9800.0, camera={"lon": -30.0, "lat": 20.0, "zoom": 1.0}, frames=True)
plan("panel-rim", "The rim: a lamp out at the globe's edge chosen from the panel is turned inward, its cloud on the face; one already facing leaves the globe still", [
    shot(0.1, "at-the-rim"),
    ev(0.2, "down", 0, xy=row_xy(0)), ev(0.28, "up", 0, xy=row_xy(0)), expect(0.3, chosen=[1]),
    expect(1.2, camera_moved=True), shot(1.2, "turned-in"),
    ev(1.5, "down", 0, xy=row_xy(1)), ev(1.58, "up", 0, xy=row_xy(1)), expect(2.5, chosen=[1, 2], camera_same_as_last=True),
], floats=[{"lon": 8.0, "lat": -33.0, "days_ago": 20.0}, {"lon": -25.0, "lat": -15.0, "days_ago": 20.0}], day=9800.0, camera={"lon": -40.0, "lat": 22.0, "zoom": 1.0},
   panel=[[1] + row(0), [2] + row(1)], frames=True)

# The poles (R6): the orb has no north to stop at. A finger takes a point above the centre and
# drags it down the screen, carrying the north pole through the middle and on: the point stays
# under the finger the whole way (no lock, no jump), then Home turns it back upright.
POLE_STEPS = [ev(0.0, "down", xy=[W / 2, H / 2 - 150.0])]
for i in range(1, 31):
    POLE_STEPS.append(ev(0.0 + 0.04 * i, "move", xy=[W / 2, H / 2 - 150.0 + 9.0 * i]))
POLE_STEPS.append(ev(1.3, "up", xy=[W / 2, H / 2 + 120.0]))
plan("over-the-pole", "Drag the orb over the north pole: the point taken stays under the finger the whole way, no lock and no jump; Home brings it back upright", [
    shot(0.0, "before"), *POLE_STEPS[:16], shot(0.62, "pole-in-the-middle"), *POLE_STEPS[16:],
    shot(1.25, "over"), expect(1.25, state="orbit", camera_moved=True),
    expect(3.0, state="idle"), shot(3.0, "after"),
    {"t": 3.2, "key": "home"}, expect(4.2, camera={"lon": -30.0, "lat": 25.0}), shot(4.2, "home"),
], camera={"lon": -40.0, "lat": 55.0, "zoom": 1.0})

for name in ("panel-two", "choose-two-on-globe", "scrub-limit", "panel-rim", "over-the-pole"):
    PLANS[name]["steps"].sort(key=lambda st: st["t"])    # played in order of time (a stable sort)

os.makedirs(OUT, exist_ok=True)
for f in os.listdir(OUT):
    if f.endswith(".json"):
        os.remove(os.path.join(OUT, f))
for name, p in PLANS.items():
    with open(os.path.join(OUT, name + ".json"), "w") as f:
        json.dump(p, f, indent=1)
        f.write("\n")
print(len(PLANS), "plans")
