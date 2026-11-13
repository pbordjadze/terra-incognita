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
            use std::ffi::{c_char, c_void, CStr};
            type S = $crate::shell::Shell<$toy>;

            /// Where to keep the log (call before `toy_new`).
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_log(path: *const c_char) {
                $crate::set_log(CStr::from_ptr(path).to_string_lossy().into_owned().into());
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut S {
                let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
                match S::new(layer, width, height, scale, &dir) {
                    Some(s) => Box::into_raw(Box::new(s)),
                    None => std::ptr::null_mut(),
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_resize(s: *mut S, width: u32, height: u32, scale: f64) {
                if let Some(s) = s.as_mut() { s.resize(width, height, scale) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_frame(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.frame(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pointer(s: *mut S, id: u64, phase: u32, x: f64, y: f64, t: f64) {
                if let Some(s) = s.as_mut() { s.pointer(id, phase, x, y, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_shaken(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.toy.shaken(&mut s.host.ctx) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_take_haptic(s: *mut S) -> bool {
                s.as_mut().map(|s| s.host.take_haptic()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_night(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_night(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_reduce_motion(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_reduce_motion(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_store(s: *mut S, path: *const c_char) {
                if let Some(s) = s.as_mut() { s.store(&CStr::from_ptr(path).to_string_lossy()) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pause(s: *mut S) {
                if let Some(s) = s.as_mut() { s.pause() }
            }
            /// The toy's sound, for the audio thread (null: silent). Freed by `toy_synth_free`.
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth(s: *mut S) -> *mut $crate::SharedSynth {
                match s.as_ref().and_then(|s| s.synth()) {
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
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.info_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_line(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.line()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_open(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.host.open_info(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.rules_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_text(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.rules()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_open(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.open_rules() }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_origins_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.origins_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_origins_text(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.origins()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_origins_open(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.open_origins() }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_switches_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.switches_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_switches_text(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.switches_text()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_switches_open(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.open_switches() }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_switch_set(s: *mut S, i: u32, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_switch(i as usize, on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_card_at_top(s: *mut S) -> bool {
                s.as_ref().map(|s| s.host.card_at_top()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_buttons(s: *mut S) -> *const c_char {
                s.as_mut().map(|s| s.host.buttons_text().as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_button_press(s: *mut S, i: u32, t: f64) {
                if let Some(s) = s.as_mut() { s.host.press(i as usize, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_plays_music(s: *mut S) -> bool {
                s.as_ref().map(|s| s.host.toy.plays_music()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_wants_file(s: *mut S) -> bool {
                s.as_mut().map(|s| s.host.wants_file()).unwrap_or(false)
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
                s.host.opened($crate::AudioFile { name, rate, left: l, right: r });
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_title(s: *mut S) -> *const c_char {
                s.as_mut().and_then(|s| s.host.title_text()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
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
