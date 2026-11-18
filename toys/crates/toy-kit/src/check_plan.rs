//! A plan's format: what the headless check reads (check.rs has the whole of it), and what the phone
//! feels as a plan's senses ease it there.

use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Plan {
    pub size: Option<[u32; 2]>,
    pub ppp: Option<f64>,
    pub night: bool,
    pub reduce_motion: bool,
    pub seconds: Option<f64>,
    pub fps: Option<f64>,
    pub events: Vec<Event>,
    pub shots: Vec<Shot>,
    pub film: Option<Film>,
    pub audio: bool,
    pub restore: Option<String>, // handed to the toy's `restore` before the first frame
    pub strict_taps: bool,       // a touch that does nothing fails the check (else a warning)
    pub first_info: bool,        // the demo opens by itself as on a first visit (info::First)
}

/// One event of a plan. The point fields hold the places given as points; `places` holds every
/// place as written (points, names, `lonlat`), which the player resolves.
#[derive(Deserialize, Default, Clone)]
#[serde(from = "RawEvent")]
pub struct Event {
    pub t: f64,
    pub tap: Option<[f64; 2]>,
    pub drag: Option<[[f64; 2]; 2]>,
    pub pinch: Option<[[f64; 2]; 2]>,
    pub to: Option<f64>,
    pub hold: Option<[f64; 2]>,
    pub dur: Option<f64>,
    pub down: Option<[f64; 2]>,
    pub mv: Option<[f64; 2]>,
    pub up: Option<[f64; 2]>,
    pub id: Option<u64>,
    pub shake: bool,
    /// What the phone feels from here (`Sensors`): gravity in g and the magnetic field in µT in the
    /// device's frame, the heading in degrees; with `for`, eased there from what it felt over that long.
    pub gravity: Option<[f64; 3]>,
    pub field: Option<[f64; 3]>,
    pub heading: Option<f64>,
    /// A sound file (WAV) handed to the toy as if the reader chose it from Files (`Toy::opened`).
    pub open: Option<String>,
    /// A rule on the switches panel turned on or off, as the reader would (`[i, on]`; switches.rs).
    pub switch: Option<(usize, bool)>,
    /// One of the sheet's buttons pressed: "demo" or "origins" (info.rs).
    pub show: Option<String>,
    /// The event is meant to change nothing on screen (the touch check leaves it alone).
    pub inert: bool,
    pub places: Places,
}

/// A place in a plan: a point (points from the top left), a name the toy gives (`Toy::anchors`), or a
/// longitude and latitude on the toy's globe (`Toy::project`).
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Place {
    At([f64; 2]),
    Named(String),
    LonLat { lonlat: [f64; 2] },
}

/// An event's places as the plan wrote them.
#[derive(Default, Clone, Debug)]
pub struct Places {
    pub tap: Option<Place>,
    pub hold: Option<Place>,
    pub drag: Option<[Place; 2]>,
    pub pinch: Option<[Place; 2]>,
    pub down: Option<Place>,
    pub mv: Option<Place>,
    pub up: Option<Place>,
}

impl Places {
    pub(super) fn slots(&self) -> [Option<&Place>; 9] {
        fn two(p: &Option<[Place; 2]>, i: usize) -> Option<&Place> {
            p.as_ref().map(|p| &p[i])
        }
        [
            self.tap.as_ref(),
            self.hold.as_ref(),
            two(&self.drag, 0),
            two(&self.drag, 1),
            two(&self.pinch, 0),
            two(&self.pinch, 1),
            self.down.as_ref(),
            self.mv.as_ref(),
            self.up.as_ref(),
        ]
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawEvent {
    t: f64,
    tap: Option<Place>,
    drag: Option<[Place; 2]>,
    pinch: Option<[Place; 2]>,
    to: Option<f64>,
    hold: Option<Place>,
    #[serde(rename = "for")]
    dur: Option<f64>,
    down: Option<Place>,
    #[serde(rename = "move")]
    mv: Option<Place>,
    up: Option<Place>,
    id: Option<u64>,
    shake: bool,
    gravity: Option<[f64; 3]>,
    field: Option<[f64; 3]>,
    heading: Option<f64>,
    open: Option<String>,
    switch: Option<(usize, bool)>,
    show: Option<String>,
    inert: bool,
}

impl From<RawEvent> for Event {
    fn from(r: RawEvent) -> Event {
        let pt = |p: &Option<Place>| match p {
            Some(Place::At(a)) => Some(*a),
            _ => None,
        };
        let two = |p: &Option<[Place; 2]>| match p {
            Some([Place::At(a), Place::At(b)]) => Some([*a, *b]),
            _ => None,
        };
        Event {
            t: r.t,
            tap: pt(&r.tap),
            drag: two(&r.drag),
            pinch: two(&r.pinch),
            to: r.to,
            hold: pt(&r.hold),
            dur: r.dur,
            down: pt(&r.down),
            mv: pt(&r.mv),
            up: pt(&r.up),
            id: r.id,
            shake: r.shake,
            gravity: r.gravity,
            field: r.field,
            heading: r.heading,
            open: r.open,
            switch: r.switch,
            show: r.show,
            inert: r.inert,
            places: Places {
                tap: r.tap,
                hold: r.hold,
                drag: r.drag,
                pinch: r.pinch,
                down: r.down,
                mv: r.mv,
                up: r.up,
            },
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Shot {
    At(f64),
    Named { t: f64, name: String },
}

#[derive(Deserialize, Clone)]
pub struct Film {
    #[serde(default)]
    pub from: f64,
    pub to: f64,
    pub every: f64,
    /// Draw a soft ring where each finger is down on the film's frames (not on shots), so a film of
    /// several fingers shows which is where.
    #[serde(default)]
    pub fingers: bool,
}

/// What the phone feels `k` of the way from `from` to what event `e` gives (gravity kept a unit vector,
/// the heading the short way round); a sense the event leaves out stays as it was.
pub(super) fn eased(from: crate::Sensors, e: &Event, k: f64) -> crate::Sensors {
    let lerp3 = |a: Option<[f64; 3]>, b: [f64; 3]| {
        let a = a.unwrap_or(b);
        [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * k)
    };
    let mut s = from;
    if let Some(g) = e.gravity {
        let v = lerp3(from.gravity, g);
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
        let m = (g[0] * g[0] + g[1] * g[1] + g[2] * g[2]).sqrt();
        s.gravity = Some(v.map(|c| c / n * m));
    }
    if let Some(b) = e.field {
        s.field = Some(lerp3(from.field, b));
    }
    if let Some(h) = e.heading {
        let a = from.heading.unwrap_or(h);
        let d = (h - a + 540.0).rem_euclid(360.0) - 180.0;
        s.heading = Some((a + d * k).rem_euclid(360.0));
    }
    s
}
