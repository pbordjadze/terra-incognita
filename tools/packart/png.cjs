// PNG files read and written in Node with nothing but zlib, for the one art drawer's sprite sheets
// (tools/packart/render.cjs): frames drawn one by one are laid into a sheet pixel for pixel, never
// through a canvas, whose premultiplied alpha would change the faint edge of a drawing.
//
//   const png = require('./png.cjs')
//   const { width, height, data } = png.read(buffer)   // data: RGBA, 8 bits a channel, rows top down
//   png.write({ width, height, data })                  // a Buffer: an RGBA PNG
//
// It reads 8-bit greyscale, grey with alpha, RGB, RGBA and palette pictures without interlacing (what
// Chromium and Pillow write); anything else is an error, not a guess.
'use strict';
const zlib = require('zlib');

const SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

const CRC = new Int32Array(256).map((_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c;
});
function crc32(buf) {
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = CRC[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}

function paeth(a, b, c) {
  const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

/** A PNG's pixels as RGBA. */
function read(buf) {
  if (!buf.subarray(0, 8).equals(SIGNATURE)) throw new Error('not a PNG');
  let pos = 8, width, height, depth, type, interlace, palette = null, alphas = null;
  const idat = [];
  while (pos < buf.length) {
    const len = buf.readUInt32BE(pos), kind = buf.toString('latin1', pos + 4, pos + 8), body = buf.subarray(pos + 8, pos + 8 + len);
    if (kind === 'IHDR') {
      width = body.readUInt32BE(0); height = body.readUInt32BE(4); depth = body[8]; type = body[9]; interlace = body[12];
    } else if (kind === 'PLTE') palette = body;
    else if (kind === 'tRNS') alphas = body;
    else if (kind === 'IDAT') idat.push(body);
    else if (kind === 'IEND') break;
    pos += 12 + len;
  }
  const channels = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[type];
  if (depth !== 8 || !channels || interlace) throw new Error(`a PNG of depth ${depth}, colour type ${type}, interlace ${interlace}: only 8-bit pictures without interlacing`);
  if (type === 3 && !palette) throw new Error('a palette PNG without its palette');
  const raw = zlib.inflateSync(Buffer.concat(idat)), stride = width * channels, rows = Buffer.alloc(stride * height);
  for (let y = 0, at = 0; y < height; y++) {
    const filter = raw[at++], row = y * stride, up = row - stride;
    for (let x = 0; x < stride; x++, at++) {
      const a = x >= channels ? rows[row + x - channels] : 0, b = y ? rows[up + x] : 0, c = y && x >= channels ? rows[up + x - channels] : 0;
      const v = raw[at];
      rows[row + x] = filter === 0 ? v : filter === 1 ? v + a : filter === 2 ? v + b : filter === 3 ? v + ((a + b) >> 1) : v + paeth(a, b, c);
    }
  }
  const data = Buffer.alloc(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    const s = i * channels, d = i * 4;
    if (type === 6) rows.copy(data, d, s, s + 4);
    else if (type === 2) { data[d] = rows[s]; data[d + 1] = rows[s + 1]; data[d + 2] = rows[s + 2]; data[d + 3] = 255; }
    else if (type === 0) { data[d] = data[d + 1] = data[d + 2] = rows[s]; data[d + 3] = 255; }
    else if (type === 4) { data[d] = data[d + 1] = data[d + 2] = rows[s]; data[d + 3] = rows[s + 1]; }
    else { const k = rows[s]; palette.copy(data, d, k * 3, k * 3 + 3); data[d + 3] = alphas && k < alphas.length ? alphas[k] : 255; }
  }
  return { width, height, data };
}

function chunk(kind, body) {
  const out = Buffer.alloc(12 + body.length);
  out.writeUInt32BE(body.length, 0);
  out.write(kind, 4, 'latin1');
  body.copy(out, 8);
  out.writeUInt32BE(crc32(out.subarray(4, 8 + body.length)), 8 + body.length);
  return out;
}

/** An RGBA picture as a PNG (each row filtered by the filter that compresses it best, as Pillow guesses: the least sum). */
function write({ width, height, data }) {
  const stride = width * 4, raw = Buffer.alloc((stride + 1) * height), line = Buffer.alloc(stride);
  for (let y = 0; y < height; y++) {
    const row = y * stride, up = row - stride;
    let best = null, bestSum = Infinity, bestFilter = 0;
    for (let f = 0; f < 5; f++) {
      let sum = 0;
      for (let x = 0; x < stride; x++) {
        const v = data[row + x], a = x >= 4 ? data[row + x - 4] : 0, b = y ? data[up + x] : 0, c = y && x >= 4 ? data[up + x - 4] : 0;
        const p = f === 0 ? v : f === 1 ? v - a : f === 2 ? v - b : f === 3 ? v - ((a + b) >> 1) : v - paeth(a, b, c);
        line[x] = p & 0xff;
        sum += line[x] < 128 ? line[x] : 256 - line[x];
      }
      if (sum < bestSum) { bestSum = sum; bestFilter = f; best = Buffer.from(line); }
    }
    raw[y * (stride + 1)] = bestFilter;
    best.copy(raw, y * (stride + 1) + 1);
  }
  const head = Buffer.alloc(13);
  head.writeUInt32BE(width, 0); head.writeUInt32BE(height, 4);
  head[8] = 8; head[9] = 6; head[10] = 0; head[11] = 0; head[12] = 0;
  return Buffer.concat([SIGNATURE, chunk('IHDR', head), chunk('IDAT', zlib.deflateSync(raw, { level: 9 })), chunk('IEND', Buffer.alloc(0))]);
}

module.exports = { read, write };
