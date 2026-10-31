//! The app answers the gestures' questions from its state and the frame as drawn.

use super::gestures::{date_band, Frame, Hit, Probe, Rect};
use super::motion::Motion;
use crate::app::App;
use crate::model::{Hand, Hit as Under, LonLat};

impl Probe for App {
    fn hit(&self, x: f64, y: f64) -> Hit {
        let frame = self.frame();
        // the date's rect: the look's, as drawn last frame; the stand-in band until it has drawn
        let date = match self.date_bounds() {
            Some([bx, by, bw, bh]) => Rect { x: bx as f64, y: by as f64, w: bw as f64, h: bh as f64 },
            None => date_band(&frame),
        };
        let credits = self.credit_bounds().map(|[bx, by, bw, bh]| Rect { x: bx as f64, y: by as f64, w: bw as f64, h: bh as f64 });
        match App::hit(self, x, y) {
            // the float in hand lies under the finger: what matters is the water under it
            Under::Float(id, p) if self.floats.iter().any(|f| f.id == id && f.hand == Hand::Carried) => {
                if self.rules.coast.is_land(p) {
                    Hit::Land(p)
                } else {
                    Hit::Sea(p)
                }
            }
            Under::Float(id, p) => Hit::Float(id, p),
            _ if credits.is_some_and(|r| r.contains(x, y)) => Hit::Credits,
            _ if date.contains(x, y) => Hit::Date,
            Under::Sea(p) => Hit::Sea(p),
            Under::Land(p) => Hit::Land(p),
            Under::Off => Hit::Off,
        }
    }
    fn float_at(&self, id: u32) -> Option<LonLat> {
        self.floats.iter().find(|f| f.id == id).map(|f| f.at)
    }
    fn riding(&self) -> Option<u32> {
        self.camera.riding
    }
    fn running(&self) -> bool {
        self.clock.running
    }
    fn day(&self) -> f64 {
        self.clock.day
    }
    fn frame(&self) -> Frame {
        let v = self.view();
        Frame { width: v.width as f64 / v.ppp, height: v.height as f64 / v.ppp, reading: v.reading, disc: v.disc, zoom: v.camera.zoom }
    }
    fn motion(&self) -> Motion {
        Motion { reduce: self.reduce_motion }
    }
    fn credits_open(&self) -> bool {
        self.credits_open
    }
}
