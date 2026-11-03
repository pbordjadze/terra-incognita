#!/usr/bin/env bash
# Proves toy-kit works as a git dependency from outside this repository's workspace: a throwaway crate
# in a temporary directory depends on it by git URL and ref, and is built (natively, and with
# `--wasm` for wasm32 with the web feature). Run from anywhere:
#
#     bash tools/kit/consumer_check.sh [<git-url>] [--branch b | --tag t | --rev r] [--features "a b"]
#                                      [--globe] [--wasm] [--keep]
#
# Defaults: this repository's own checkout (file://, its committed HEAD's branch), `default-features =
# false` (no globe), no features. `--globe` keeps the default features; `--wasm` builds for
# wasm32-unknown-unknown with the `web` feature (the consumer then calls `toy_kit::web::run`). The
# consumer touches the kit's public API (Toy, Host, Ctx, paint, backdrop, layer, sound, check::main_with;
# Globe with --globe), so a missing export fails here. Its directory and target are deleted afterwards
# unless --keep. Example, the tag busquets pins:
#
#     bash tools/kit/consumer_check.sh https://github.com/pbordjadze/terra-incognita --tag toy-kit-v0.2.0
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
url="file://$root"; ref_kind=branch; ref="$(git -C "$root" rev-parse --abbrev-ref HEAD)"
features=""; globe=0; wasm=0; keep=0
while [ $# -gt 0 ]; do
  case "$1" in
    --branch|--tag|--rev) ref_kind="${1#--}"; ref="$2"; shift 2 ;;
    --features) features="$2"; shift 2 ;;
    --globe) globe=1; shift ;;
    --wasm) wasm=1; shift ;;
    --keep) keep=1; shift ;;
    -*) echo "unknown flag $1"; exit 2 ;;
    *) url="$1"; shift ;;
  esac
done
[ "$wasm" -eq 0 ] || features="$features web"
feat_list=$(for f in $features; do printf '"%s", ' "$f"; done)
default_line=$([ "$globe" -eq 1 ] && echo "" || echo "default-features = false, ")

dir="$(mktemp -d "${TMPDIR:-/tmp}/toy-kit-consumer.XXXXXX")"
[ "$keep" -eq 1 ] || trap 'rm -rf "$dir"' EXIT
mkdir -p "$dir/src"
# A local checkout that is shallow (as an agent's clone is) can't be fetched from by cargo: the ref's
# toys/ (the workspace and the kit, as committed) is snapshot into a one-commit repository instead, the
# same layout cargo would find in a full clone.
if [[ "$url" == file://* ]] && [ "$(git -C "${url#file://}" rev-parse --is-shallow-repository)" = true ]; then
  src="${url#file://}"; snap="$dir/snapshot"
  sha="$(git -C "$src" rev-parse "$ref^{commit}")"
  mkdir -p "$snap" && git -C "$src" archive "$sha" toys | tar -x -C "$snap"
  git -C "$snap" init -q -b snapshot && git -C "$snap" add -A
  git -C "$snap" -c user.name=check -c user.email=check@localhost commit -q -m "toys/ at $sha"
  echo "(shallow checkout: toys/ at $ref, $sha, snapshot in $snap)"
  url="file://$snap"; ref_kind=branch; ref=snapshot
fi
cat > "$dir/Cargo.toml" <<EOF
[package]
name = "toy-kit-consumer"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
toy-kit = { git = "$url", $ref_kind = "$ref", ${default_line}features = [${feat_list}] }
wgpu = { version = "26", default-features = false, features = ["wgsl"] }
anyhow = "1"

[workspace]
EOF
globe_use=$([ "$globe" -eq 1 ] && echo "#[allow(unused_imports)] use toy_kit::Globe;" || echo "")
cat > "$dir/src/lib.rs" <<EOF
//! A throwaway consumer of toy-kit (tools/kit/consumer_check.sh).
use toy_kit::{backdrop::Backdrop, layer::Layer, paint::Painter, sound::Bells, Ctx, PointerEvent, Synth, Toy};
$globe_use

pub struct Dot {
    painter: Painter,
    backdrop: Backdrop,
    layer: Layer,
    bells: Bells,
}

impl Toy for Dot {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let backdrop = Backdrop::new(&ctx.gpu, "bg", "fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.1, 0.1, 0.1, 1.0); }");
        let layer = Layer::new(&ctx.gpu, [ctx.width, ctx.height]);
        Ok(Dot { painter: Painter::new(&ctx.gpu), backdrop, layer, bells: Bells::new() })
    }
    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        self.bells.strike(toy_kit::sound::Note { freq: toy_kit::sound::pentatonic(220.0, 2), amp: 0.3, decay: 1.0, pan: 0.0, bright: 1.0 });
        let _ = e;
    }
    fn update(&mut self, _ctx: &mut Ctx, _t: f64, _dt: f64) {}
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let size = [ctx.width, ctx.height];
        self.backdrop.render(&ctx.gpu, &mut enc, target, size, &Default::default());
        self.painter.disc([size[0] as f32 / 2.0, size[1] as f32 / 2.0], 20.0, 1.0, [1.0, 0.8, 0.3, 1.0]);
        self.painter.render(&ctx.gpu, &mut enc, &self.layer.view, size, None);
        self.layer.composite(&mut enc, target);
        ctx.gpu.queue.submit([enc.finish()]);
    }
    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn check(args: &[String]) {
    toy_kit::check::main_with::<Dot>("dot", args)
}

#[cfg(target_arch = "wasm32")]
pub async fn start() {
    let _ = toy_kit::web::run::<Dot>("canvas", "dot").await;
}
EOF
cd "$dir"
# git's own fetch, not cargo's libgit2, which can't read a local checkout's objects through a
# worktree or alternates ("object not found")
export CARGO_NET_GIT_FETCH_WITH_CLI=true
echo "consumer of toy-kit at $url ($ref_kind $ref), ${default_line:-default features, }features [${feat_list%, }]"
if [ "$wasm" -eq 1 ]; then
  cargo build --target wasm32-unknown-unknown
else
  cargo build
fi
echo "consumer builds: $(grep -A2 'name = "toy-kit"' Cargo.lock | grep -E 'version|source' | tr '\n' ' ')"
