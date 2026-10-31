#!/usr/bin/env python3
"""Writes the touch module's plans, adrift/crates/adrift-core/src/touch/plans/*.json: one played
session a gesture and an ambiguous case of VOCABULARY.md, each with the expectations that prove it.
touch-replay checks them now (README.md); `adrift-check -- play` photographs them once it runs.

    python3 tools/adrift/touch-standalone/plans.py        # rewrites the plans
"""
import json, os

OUT = os.path.join(os.path.dirname(__file__), "../../../adrift/crates/adrift-core/src/touch/plans")
SEA = [-30.0, 20.0]          # open sea in the Atlantic view
SEA2 = [-20.0, 15.0]
LAND = [-70.0, 20.0]         # the stand-in continent's centre (the real coast in adrift-check)
BEAT = 0.40
PLANS = {}

def plan(name, title, steps, **kw):
    p = {"title": title, "camera": {"lon": -40.0, "lat": 22.0, "zoom": 1.0}, "land": [[-70.0, 20.0, 14.0]], "steps": steps}
    p.update(kw)
    PLANS[name] = p

def ev(t, kind, finger=0, at=None, xy=None, **more):
    s = {"t": t, kind: finger}
    if at is not None: s["at"] = at
    if xy is not None: s["xy"] = xy
    s.update(more)
    return s

def expect(t, **e):
    return {"t": t, "expect": e}

def shot(t, name):
    return {"t": t, "shot": name}

# --- taps and the beat ---
plan("tap-rings", "A tap on the sea rings and changes nothing", [
    ev(0.0, "down", at=SEA), shot(0.05, "ring"), ev(0.12, "up", at=SEA),
    expect(0.5, floats=0, state="idle", saw=["Ring", "Unpress"], **{"not": ["Put"]}, camera_moved=False),
])
plan("put", "Rest a beat on the sea: the ring thickens into a float; lift and the sea has it", [
    ev(0.0, "down", at=SEA), shot(0.2, "thickening"), shot(0.39, "almost"),
    expect(0.45, floats=1, state="holding", saw=["Press", "Put"], held=0),
    shot(0.6, "held"), ev(1.0, "up", at=SEA),
    expect(1.1, floats=1, state="idle", saw=["ReleaseBack"], held=None, float={"id": 0, "lon": SEA[0], "lat": SEA[1]}, camera_moved=False),
    shot(1.5, "afloat"),
])
plan("almost", "A lift just short of the beat is a tap: no float", [
    ev(0.0, "down", at=SEA), ev(0.35, "up", at=SEA),
    expect(0.6, floats=0, saw=["Unpress"], **{"not": ["Put"]}),
])
plan("land-silent", "Land and off the disc are silent: no ring, no float; a drag is still the globe's", [
    ev(0.0, "down", at=LAND), ev(0.1, "up", at=LAND),
    expect(0.2, floats=0, **{"not": ["Ring", "Press"]}),
    ev(0.5, "down", at=LAND), ev(1.5, "up", at=LAND),
    expect(1.6, floats=0, **{"not": ["Ring", "Put", "Hold"]}, camera_moved=False),
    ev(2.0, "down", at=LAND), ev(2.1, "move", xy=[300, 500]), ev(2.2, "move", xy=[380, 520]), ev(2.3, "up", xy=[380, 520]),
    expect(2.4, floats=0, camera_moved=True, **{"not": ["Ring", "Put"]}),
    ev(3.0, "down", xy=[10, 10]), ev(4.0, "up", xy=[10, 10]),
    expect(4.1, floats=0, **{"not": ["Ring", "Put"]}),
])

# --- the drag that begins on a float ---
plan("orbit-from-float", "A drag that begins on a float before the beat is the globe's: the float stays", [
    ev(0.0, "down", at=SEA),
    ev(0.05, "move", xy=[420, 560]), ev(0.10, "move", xy=[470, 580]), ev(0.15, "move", xy=[520, 600]), ev(0.2, "up", xy=[520, 600]),
    expect(0.3, floats=1, state="idle", camera_moved=True, **{"not": ["Hold", "Carry", "Release", "Ride"]},
           float={"id": 0, "lon": SEA[0], "lat": SEA[1]}),
    shot(0.4, "turned"),
], floats=[{"lon": SEA[0], "lat": SEA[1]}])
plan("orbit-then-rest", "Once a drag, always a drag: a finger that moves and then rests puts nothing", [
    ev(0.0, "down", at=SEA), ev(0.1, "move", xy=[430, 560]), ev(0.2, "move", xy=[480, 570]),
    ev(1.5, "up", xy=[480, 570]),
    expect(1.6, floats=0, saw=["Unpress"], **{"not": ["Put"]}, camera_moved=True),
])

# --- hold and carry ---
plan("hold-maybes", "Rest a beat on a float: you hold it (its maybes); lift and nothing changed", [
    ev(0.0, "down", at=SEA), expect(0.5, state="holding", saw=["Hold"], held=0, **{"not": ["Ring", "Press"]}),
    shot(0.8, "maybes"), ev(1.2, "up", at=SEA),
    expect(1.3, floats=1, state="idle", saw=["ReleaseBack"], held=None, float={"id": 0, "lon": SEA[0], "lat": SEA[1]}, camera_moved=False),
], floats=[{"lon": SEA[0], "lat": SEA[1]}])
plan("carry", "Rest, then drag: carry the float to other water; the globe does not move; undo and redo", [
    ev(0.0, "down", at=SEA), expect(0.5, state="holding", saw=["Hold"]),
    ev(0.6, "move", at=[-25.0, 18.0]), ev(0.7, "move", at=SEA2), shot(0.75, "carrying"),
    expect(0.75, state="carrying", saw=["Carry"], held=0, camera_moved=False),
    ev(0.8, "up", at=SEA2),
    expect(0.9, floats=1, state="idle", saw=["Release"], held=None, float={"id": 0, "lon": SEA2[0], "lat": SEA2[1]}, camera_moved=False),
    shot(1.0, "set-down"),
    {"t": 1.5, "key": "undo"}, expect(1.6, floats=1, saw=["Undo"], float={"id": 0, "lon": SEA[0], "lat": SEA[1]}),
    {"t": 2.0, "key": "redo"}, expect(2.1, floats=1, saw=["Redo"], float={"id": 0, "lon": SEA2[0], "lat": SEA2[1]}),
], floats=[{"lon": SEA[0], "lat": SEA[1]}])
plan("carry-fresh", "The float just put is already in hand: drag it before lifting", [
    ev(0.0, "down", at=SEA), expect(0.5, floats=1, state="holding", saw=["Put"]),
    ev(0.6, "move", at=SEA2), ev(0.8, "up", at=SEA2),
    expect(0.9, floats=1, saw=["Carry", "Release"], float={"id": 0, "lon": SEA2[0], "lat": SEA2[1]}, camera_moved=False),
])
plan("carry-to-land", "A carry released over land goes back where it was lifted", [
    ev(0.0, "down", at=SEA), expect(0.5, saw=["Hold"]),
    ev(0.6, "move", at=[-50.0, 20.0]), ev(0.7, "move", at=LAND), expect(0.75, saw=["CarryLand"], state="carrying"),
    ev(0.8, "up", at=LAND),
    expect(0.9, floats=1, saw=["ReleaseBack"], float={"id": 0, "lon": SEA[0], "lat": SEA[1]}, held=None),
], floats=[{"lon": SEA[0], "lat": SEA[1]}])
plan("take-out", "Carry a float off the rim and it is taken out in plain sight; undo brings it back", [
    ev(0.0, "down", at=SEA), expect(0.5, saw=["Hold"]),
    ev(0.6, "move", at=[-60.0, 40.0]), ev(0.7, "move", xy=[30, 200]), shot(0.75, "over-the-rim"),
    expect(0.75, saw=["CarryOff"], state="carrying", floats=1),
    ev(0.9, "move", at=[-60.0, 40.0]), expect(0.95, saw=["Carry"]),          # back over the water: in hand again
    ev(1.0, "move", xy=[30, 200]), ev(1.1, "up", xy=[30, 200]),
    expect(1.2, floats=0, saw=["TakeOut"], held=None),
    shot(1.3, "gone"),
    {"t": 1.5, "shake": True}, expect(1.6, floats=1, saw=["Undo"], float={"id": 0, "lon": SEA[0], "lat": SEA[1]}),
], floats=[{"lon": SEA[0], "lat": SEA[1]}])
plan("cancel", "The system takes the touch: a press lets go, a hold puts the float back, a drag stops", [
    ev(0.0, "down", at=SEA2), ev(0.2, "cancel", at=SEA2), expect(0.3, floats=1, saw=["Unpress"], **{"not": ["Put"]}),
    ev(0.5, "down", at=SEA), expect(1.0, saw=["Hold"]), ev(1.1, "move", at=[-25.0, 25.0]), ev(1.2, "cancel", at=[-25.0, 25.0]),
    expect(1.3, floats=1, saw=["ReleaseBack"], float={"id": 0, "lon": SEA[0], "lat": SEA[1]}, held=None, camera_moved=False),
    ev(1.5, "down", at=[-45.0, 0.0]), ev(1.55, "move", xy=[400, 700]), ev(1.6, "move", xy=[460, 720]), ev(1.62, "cancel", xy=[460, 720]),
    expect(1.7, camera_moved=True), expect(2.5, camera_same_as_last=True),   # no fling
], floats=[{"lon": SEA[0], "lat": SEA[1]}])

# --- the ride ---
plan("ride", "Tap a float and the camera drifts with it; any drag leaves it", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.1, saw=["Ride"], riding=0, **{"not": ["Press"]}),
    shot(1.0, "catching-up"),
    expect(3.0, riding=0, camera_near_float=[0, 0.3], camera_moved=True), shot(3.0, "riding"),
    ev(3.1, "down", at=[-45.0, 0.0]), ev(3.15, "move", xy=[400, 700]), ev(3.2, "move", xy=[460, 720]), ev(3.3, "up", xy=[460, 720]),
    expect(3.4, saw=["Leave"], riding=None),
    expect(6.0, riding=None),
], floats=[{"lon": SEA[0], "lat": SEA[1]}], drift=[0.5, 0.0])
plan("ride-and-hold-another", "Riding one float, hold another: the ride goes on; carry it and the ride is left", [
    ev(0.0, "down", at=SEA), ev(0.08, "up", at=SEA), expect(0.1, riding=0),
    ev(0.5, "down", at=SEA2), expect(1.0, saw=["Hold"], riding=0, held=1),
    ev(1.1, "move", at=[-18.0, 12.0]), expect(1.2, saw=["Leave", "Carry"], riding=None),
    ev(1.3, "up", at=[-18.0, 12.0]), expect(1.4, saw=["Release"], floats=2),
], floats=[{"lon": SEA[0], "lat": SEA[1]}, {"lon": SEA2[0], "lat": SEA2[1]}])
plan("tap-near-two", "A tap near two floats takes the nearest", [
    ev(0.0, "down", at=[-30.4, 20.0]), ev(0.08, "up", at=[-30.4, 20.0]), expect(0.1, riding=1),
], floats=[{"lon": -30.0, "lat": 20.0}, {"lon": -30.6, "lat": 20.0}])

# --- the globe's own camera ---
plan("fling", "Let go moving and the globe flings, then rests; a touch stops it at once", [
    ev(0.0, "down", at=[-45.0, 0.0]), ev(0.05, "move", xy=[400, 700]), ev(0.10, "move", xy=[450, 710]), ev(0.15, "move", xy=[500, 720]),
    ev(0.2, "up", xy=[500, 720]),
    expect(0.3, camera_moved=True), expect(0.6, camera_same_as_last=False),           # still going
    ev(0.7, "down", at=[-45.0, 0.0]), ev(0.75, "up", at=[-45.0, 0.0]),               # a tap: it stops, and the sea rings
    expect(0.8, saw=["Ring"]), expect(1.5, camera_same_as_last=True),
])
plan("pinch", "Two fingers zoom and turn; a second finger before the beat puts no float", [
    ev(0.0, "down", 0, xy=[350, 500]), ev(0.05, "down", 1, xy=[470, 500]),
    expect(0.1, saw=["Unpress"], state="pinch"),
    ev(0.15, "move", 0, xy=[320, 500]), ev(0.2, "move", 1, xy=[520, 500]), ev(0.3, "move", 0, xy=[300, 500]),
    expect(0.35, zoom_gt=1.3, floats=0),
    ev(0.4, "up", 1, xy=[520, 500]), expect(0.45, state="orbit"),
    ev(0.5, "move", 0, xy=[340, 560]), ev(0.6, "up", 0, xy=[340, 560]),
    expect(1.2, floats=0, **{"not": ["Put"]}),
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
], camera={"lon": -32.0, "lat": 20.0, "zoom": 6.0})
plan("idle-turn", "Left alone with nothing on the sea the globe turns; with a float, it holds still", [
    expect(7.9, camera_moved=False), expect(9.0, camera_moved=True),
    ev(9.1, "down", at=SEA), ev(9.6, "up", at=SEA), expect(9.7, floats=1),
    expect(19.0, camera_same_as_last=True),
])
plan("reduce-motion", "Reduce Motion: no fling, flights are cuts; the beat is unchanged", [
    ev(0.0, "down", at=[-45.0, 0.0]), ev(0.05, "move", xy=[400, 700]), ev(0.10, "move", xy=[450, 710]), ev(0.15, "up", xy=[450, 710]),
    expect(0.2, camera_moved=True), expect(1.0, camera_same_as_last=True),
    ev(1.1, "down", at=SEA), ev(1.18, "up", at=SEA), ev(1.3, "down", at=SEA), ev(1.38, "up", at=SEA),
    expect(1.4, camera={"lon": SEA[0], "lat": SEA[1], "zoom": 2.0}),
    ev(2.0, "down", at=SEA), expect(2.5, floats=1, saw=["Put"]), ev(2.6, "up", at=SEA),
    expect(11.0, camera_same_as_last=True),
], reduce_motion=True)

# --- the date and undo ---
plan("date-scrub", "Drag the date: the clock pauses and the day follows the finger; it stays paused", [
    ev(0.0, "down", xy=[410, 1140]), ev(0.1, "move", xy=[460, 1140]), ev(0.2, "move", xy=[510, 1142]),
    expect(0.25, saw=["Pause", "Scrub"], state="scrubbing", playing=False, day=9900.0),
    ev(0.3, "up", xy=[510, 1142]), expect(0.4, playing=False, state="idle", camera_moved=False, floats=0),
], day=9800.0)
plan("date-tap", "Tap the date to pause, tap again to play", [
    ev(0.0, "down", xy=[410, 1140]), ev(0.1, "up", xy=[410, 1140]), expect(0.2, saw=["Pause"], playing=False, day=9800.0),
    ev(0.5, "down", xy=[410, 1140]), ev(0.6, "up", xy=[410, 1140]), expect(0.7, saw=["Play"], playing=True),
], day=9800.0)
plan("undo-gestures", "Undo by three-finger swipe, by shake, by the keyboard; redo its mirror", [
    ev(0.0, "down", at=SEA), ev(0.5, "up", at=SEA), expect(0.6, floats=1),
    ev(1.0, "down", 0, xy=[300, 600]), ev(1.02, "down", 1, xy=[400, 600]), ev(1.04, "down", 2, xy=[500, 600]),
    ev(1.1, "move", 0, xy=[270, 600]), ev(1.12, "move", 1, xy=[370, 600]), ev(1.14, "move", 2, xy=[470, 600]),
    ev(1.2, "move", 0, xy=[240, 600]), ev(1.22, "move", 1, xy=[340, 600]), ev(1.24, "move", 2, xy=[440, 600]),
    expect(1.3, saw=["Undo"], floats=0),
    ev(1.4, "up", 0, xy=[240, 600]), ev(1.42, "up", 1, xy=[340, 600]), ev(1.44, "up", 2, xy=[440, 600]),
    expect(1.5, state="idle", **{"not": ["Put", "Ring"]}),
    ev(2.0, "down", 0, xy=[300, 600]), ev(2.02, "down", 1, xy=[400, 600]), ev(2.04, "down", 2, xy=[500, 600]),
    ev(2.1, "move", 0, xy=[360, 600]), ev(2.12, "move", 1, xy=[460, 600]), ev(2.14, "move", 2, xy=[560, 600]),
    expect(2.2, saw=["Redo"], floats=1),
    ev(2.3, "up", 0, xy=[360, 600]), ev(2.32, "up", 1, xy=[460, 600]), ev(2.34, "up", 2, xy=[560, 600]),
    {"t": 3.0, "shake": True}, expect(3.1, saw=["Undo"], floats=0),
    {"t": 3.5, "key": "redo"}, expect(3.6, saw=["Redo"], floats=1),
    {"t": 4.0, "key": "undo"}, expect(4.1, saw=["Undo"], floats=0),
])

os.makedirs(OUT, exist_ok=True)
for name, p in PLANS.items():
    with open(os.path.join(OUT, name + ".json"), "w") as f:
        json.dump(p, f, indent=1)
        f.write("\n")
print(len(PLANS), "plans")
