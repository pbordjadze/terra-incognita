//! A stand-in for `adrift-core/src/model.rs`: the types Motion proposes (VOCABULARY.md).
#![allow(dead_code)]

pub type FloatId = u32;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    pub lon: f64,
    pub lat: f64,
    pub zoom: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PointerEvent {
    pub id: u32,
    pub phase: Phase,
    pub x: f64,
    pub y: f64,
    pub t: f64,
}

/// Where a carried float's hand is.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub enum Hand {
    Sea(LonLat),
    Land(LonLat),
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub enum Intent {
    Ring { at: LonLat },
    Press { at: LonLat, since: f64 },
    Unpress,
    Put { id: FloatId, at: LonLat },
    Hold { id: FloatId },
    Carry { id: FloatId, to: Hand },
    Release { id: FloatId, at: Option<LonLat> },
    TakeOut { id: FloatId },
    Ride { id: FloatId },
    Leave,
    Camera(Camera),
    Scrub { day: f64 },
    Play(bool),
    Undo,
    Redo,
}
