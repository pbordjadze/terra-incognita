//! check.rs's tests, kept beside it so the check itself stays short.
use super::*;

fn plan(json: &str) -> Plan {
    serde_json::from_str(json).unwrap()
}

/// A plan of points schedules exactly the pointers it always did.
#[test]
fn schedule_matches_pointers() {
    let p = plan(
        r#"{"events": [
            {"t": 1.0, "tap": [200, 400]},
            {"t": 2.0, "drag": [[100, 500], [300, 450]], "for": 0.6},
            {"t": 3.0, "pinch": [[150, 400], [250.3, 401]], "to": 2.7, "for": 0.5},
            {"t": 4.0, "hold": [200, 300], "for": 1.5},
            {"t": 5.0, "down": [10, 10], "id": 7}, {"t": 5.2, "move": [40, 10], "id": 7}, {"t": 5.3, "up": [40, 10], "id": 7},
            {"t": 6.0, "shake": true}]}"#,
    );
    let old = pointers(&p.events);
    let new = schedule(&p.events);
    assert_eq!(old.len(), new.len());
    let mut resolved = vec![[[0.0; 2]; 9]; p.events.len()];
    for (i, e) in p.events.iter().enumerate() {
        for (s, place) in e.places.slots().iter().enumerate() {
            if let Some(pl) = place {
                resolved[i][s] = resolve(pl, Vec::new, |_| None).unwrap();
            }
        }
    }
    for (o, n) in old.iter().zip(&new) {
        assert_eq!(o.0.to_bits(), n.t.to_bits());
        match (o.1, n.what) {
            (Some(pe), What::Pointer { id, phase, pos }) => {
                let [x, y] = pos.at(&resolved[n.ev]);
                assert_eq!(
                    (
                        pe.id,
                        pe.phase,
                        pe.x.to_bits(),
                        pe.y.to_bits(),
                        pe.t.to_bits()
                    ),
                    (id, phase, x.to_bits(), y.to_bits(), n.t.to_bits())
                );
            }
            (None, What::Shake) => {}
            _ => panic!("different kinds"),
        }
    }
}

#[test]
fn places_by_name_and_lonlat() {
    let p = plan(
        r#"{"events": [{"t": 1, "tap": "whale"}, {"t": 2, "drag": [[1, 2], {"lonlat": [10, 20]}]}, {"t": 3, "hold": "nowhere", "inert": true}]}"#,
    );
    let anchors = || vec![("whale".to_string(), [50.0, 60.0])];
    let e = &p.events;
    assert_eq!(e[0].tap, None);
    assert_eq!(
        resolve(e[0].places.tap.as_ref().unwrap(), anchors, |_| None).unwrap(),
        [50.0, 60.0]
    );
    let d = e[1].places.drag.as_ref().unwrap();
    assert_eq!(resolve(&d[0], anchors, |_| None).unwrap(), [1.0, 2.0]);
    assert_eq!(
        resolve(&d[1], anchors, |ll| Some([ll[0] * 2.0, ll[1]])).unwrap(),
        [20.0, 20.0]
    );
    assert!(resolve(&d[1], anchors, |_| None).is_err());
    assert!(resolve(e[2].places.hold.as_ref().unwrap(), anchors, |_| None).is_err());
    assert!(e[2].inert);
}

#[test]
fn touches_and_their_moments() {
    let p = plan(
        r#"{"events": [{"t": 1, "tap": [1, 1]}, {"t": 2, "hold": [1, 1], "for": 1.5}, {"t": 4, "tap": [1, 1], "inert": true},
        {"t": 5, "down": [1, 1], "id": 3}, {"t": 5.5, "move": [2, 1], "id": 3}, {"t": 6, "up": [2, 1], "id": 3}, {"t": 7, "shake": true}]}"#,
    );
    let t = touches(&p.events, 30.0, 300);
    let kinds: Vec<&str> = t.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, ["tap", "hold", "down"]);
    assert_eq!(t[0].frames, [45]); // the first frame at or after 1.08 + 0.4 s
    assert_eq!(t[1].frames, [72, 117]); // 2.4 s and 3.9 s
    assert_eq!(t[2].omit, [3, 4, 5]);
    assert_eq!(t[2].frames, [162, 192]);
}

#[test]
fn loudness_of_a_tone() {
    // a full-scale 1 kHz sine is -3.0 LUFS in one channel (BS.1770's reference), 0 in both
    let s: Vec<f32> = (0..48_000 * 3)
        .flat_map(|i| {
            let v = (i as f64 * 1000.0 * std::f64::consts::TAU / 48_000.0).sin() as f32;
            [v, v]
        })
        .collect();
    let (lufs, rms, peak) = loudness(&s, 48_000.0);
    assert!(lufs.abs() < 0.2, "lufs {lufs}");
    assert!((rms + 3.01).abs() < 0.05, "rms {rms}");
    assert!(peak.abs() < 0.01);
}
