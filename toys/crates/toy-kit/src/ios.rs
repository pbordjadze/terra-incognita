//! The C ABI every toy's static library exports for the iOS shell (toys/ios/Sources/toy.h), as macros a
//! toy (or a host such as Terra) invokes; kept apart from lib.rs so the Host and the ABI each stay short.

/// The C ABI over a toy type: `toy_kit::ios_shell!(MyToy);` in the toy's `lib.rs`. Compiled only for
/// iOS (the static library the shell links).
///
/// A host links several toys into one library (Terra holds them all: `toys/terra`), so each toy's own
/// C ABI is left out when the kit's `hosted` feature is on (the host's crate turns it on; cargo then
/// builds the kit hosted for every toy in that build), and the host exports its own with
/// `ios_shell!(host MyHost)`. A toy's own app is built without it, as before.
#[macro_export]
macro_rules! ios_shell {
    (host $toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            $crate::ios_shell!(@body $toy);
        }
    };
    (@body $toy:ty) => {
            use super::*;
            use std::ffi::{c_char, c_void, CStr, CString};
            /// The shell, and the text it last handed the frame (valid until the next call of the same kind).
            pub struct S {
                shell: $crate::shell::Shell<$toy>,
                chrome: CString,
                events: CString,
            }

            /// Where to keep the log (call before `toy_new`).
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_log(path: *const c_char) {
                $crate::set_log(CStr::from_ptr(path).to_string_lossy().into_owned().into());
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut S {
                let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
                match $crate::shell::Shell::new(layer, width, height, scale, &dir) {
                    Some(shell) => Box::into_raw(Box::new(S { shell, chrome: CString::default(), events: CString::default() })),
                    None => std::ptr::null_mut(),
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_resize(s: *mut S, width: u32, height: u32, scale: f64) {
                if let Some(s) = s.as_mut() { s.shell.frame.resize(width, height, scale) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_frame(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.shell.frame.frame(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pointer(s: *mut S, id: u64, phase: u32, x: f64, y: f64, t: f64) {
                if let Some(s) = s.as_mut() { s.shell.frame.pointer(id, phase, x, y, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_shaken(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.shell.frame.host.shaken(t) }
            }
            /// The device's own acceleration (g, gravity out) at `t`: a few hard jolts are a shake.
            #[no_mangle]
            pub unsafe extern "C" fn toy_motion(s: *mut S, x: f64, y: f64, z: f64, t: f64) {
                if let Some(s) = s.as_mut() { s.shell.frame.motion([x, y, z], t) }
            }
            /// What the phone feels (toy-kit's `Sensors`): gravity in g and the calibrated magnetic field in
            /// µT, in the device's frame; a NaN in a vector is "no such sense". Heading NaN: none.
            #[no_mangle]
            pub unsafe extern "C" fn toy_sense(
                s: *mut S, gx: f64, gy: f64, gz: f64, bx: f64, by: f64, bz: f64, heading: f64,
            ) {
                if let Some(s) = s.as_mut() { s.shell.frame.sense([gx, gy, gz], [bx, by, bz], heading) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_night(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.shell.frame.host.set_night(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_reduce_motion(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.shell.frame.host.set_reduce_motion(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_store(s: *mut S, path: *const c_char) {
                if let Some(s) = s.as_mut() { s.shell.store(&CStr::from_ptr(path).to_string_lossy()) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pause(s: *mut S) {
                if let Some(s) = s.as_mut() { s.shell.pause() }
            }
            /// The chrome as JSON (toy-kit's chrome.rs), when it changed since the last call (NULL while the
            /// same); valid until the next call.
            #[no_mangle]
            pub unsafe extern "C" fn toy_chrome(s: *mut S) -> *const c_char {
                let Some(s) = s.as_mut() else { return std::ptr::null() };
                match s.shell.frame.chrome_json() {
                    Some(j) => { s.chrome = CString::new(j).unwrap_or_default(); s.chrome.as_ptr() }
                    None => std::ptr::null(),
                }
            }
            /// What the toy asks of the frame (a JSON list of chrome.rs's `Out`: haptics, a file), NULL when
            /// nothing; valid until the next call.
            #[no_mangle]
            pub unsafe extern "C" fn toy_events(s: *mut S) -> *const c_char {
                let Some(s) = s.as_mut() else { return std::ptr::null() };
                match s.shell.frame.events_json() {
                    Some(j) => { s.events = CString::new(j).unwrap_or_default(); s.events.as_ptr() }
                    None => std::ptr::null(),
                }
            }
            /// What the reader did in the frame's controls (JSON: chrome.rs's `In`).
            #[no_mangle]
            pub unsafe extern "C" fn toy_send(s: *mut S, json: *const c_char) {
                if let Some(s) = s.as_mut() { s.shell.frame.send_json(&CStr::from_ptr(json).to_string_lossy()) }
            }
            /// The toy's sound, for the audio thread (null: silent). Freed by `toy_synth_free`.
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth(s: *mut S) -> *mut $crate::SharedSynth {
                match s.as_ref().and_then(|s| s.shell.frame.synth()) {
                    Some(a) => Box::into_raw(Box::new(a)),
                    None => std::ptr::null_mut(),
                }
            }
            /// Fills `frames` interleaved frames of `channels` floats at `rate` Hz (the audio thread).
            #[no_mangle]
            pub unsafe extern "C" fn toy_audio(synth: *mut $crate::SharedSynth, out: *mut f32, frames: u32, channels: u32, rate: f64) {
                let out = std::slice::from_raw_parts_mut(out, (frames * channels) as usize);
                $crate::Host::<$toy>::audio(synth.as_ref(), out, channels as usize, rate);
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth_free(synth: *mut $crate::SharedSynth) {
                if !synth.is_null() { drop(Box::from_raw(synth)) }
            }
            /// A sound file the reader chose, decoded: `frames` samples a channel (`right` may be null
            /// for a mono file), at `rate` Hz; copied, so the shell frees its buffers after.
            #[no_mangle]
            pub unsafe extern "C" fn toy_opened_audio(s: *mut S, name: *const c_char, left: *const f32, right: *const f32, frames: u64, rate: f64) {
                let Some(s) = s.as_mut() else { return };
                if left.is_null() { return }
                let l = std::slice::from_raw_parts(left, frames as usize).to_vec();
                let r = if right.is_null() { l.clone() } else { std::slice::from_raw_parts(right, frames as usize).to_vec() };
                let name = CStr::from_ptr(name).to_string_lossy().into_owned();
                s.shell.frame.host.opened($crate::AudioFile { name, rate, left: l, right: r });
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_free(s: *mut S) {
                if !s.is_null() { drop(Box::from_raw(s)) }
            }
    };
    ($toy:ty) => {
        $crate::__own_shell!($toy);
    };
}

/// A toy's own C ABI (`ios_shell!(MyToy)`), unless the kit is built `hosted`.
#[cfg(not(feature = "hosted"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __own_shell {
    ($toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            $crate::ios_shell!(@body $toy);
        }
    };
}

/// Built `hosted` (a host such as Terra links several toys: their own C ABIs would collide), a toy's
/// `ios_shell!` exports nothing; the host's `ios_shell!(host MyHost)` does.
#[cfg(feature = "hosted")]
#[doc(hidden)]
#[macro_export]
macro_rules! __own_shell {
    ($toy:ty) => {};
}
