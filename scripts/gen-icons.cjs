// Generates src-tauri/icons/icon.png (256x256) and icon.ico (PNG-compressed
// 256x256 entry, valid on Windows Vista+).
// Pure Node, no dependencies: assembles the PNG format manually
// (signature + IHDR/IDAT/IEND with CRC32) and wraps the PNG in an ICO container.
"use strict";

const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

let crcTable;
function crc32(buf) {
  if (!crcTable) {
    crcTable = new Int32Array(256);
    for (let n = 0; n < 256; n++) {
      let c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      crcTable[n] = c;
    }
  }
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = crcTable[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([len, body, crc]);
}

function makePng(size, pixelAt) {
  // 8-bit RGBA, PNG filter byte 0 on every scanline.
  const stride = size * 4 + 1;
  const raw = Buffer.alloc(size * stride);
  for (let y = 0; y < size; y++) {
    raw[y * stride] = 0;
    for (let x = 0; x < size; x++) {
      const [r, g, b, a] = pixelAt(x, y, size);
      const o = y * stride + 1 + x * 4;
      raw[o] = r;
      raw[o + 1] = g;
      raw[o + 2] = b;
      raw[o + 3] = a;
    }
  }
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // color type RGBA
  const idat = zlib.deflateSync(raw, { level: 9 });
  return Buffer.concat([sig, chunk("IHDR", ihdr), chunk("IDAT", idat), chunk("IEND", Buffer.alloc(0))]);
}

// Logo: rounded dark square with an accent diamond.
function design(x, y, s) {
  const u = (x + 0.5) / s;
  const v = (y + 0.5) / s;
  const r = 0.2;
  const cx = Math.max(Math.abs(u - 0.5) - (0.5 - r), 0);
  const cy = Math.max(Math.abs(v - 0.5) - (0.5 - r), 0);
  if (Math.hypot(cx, cy) > r) return [0, 0, 0, 0]; // transparent corner
  let col = [23, 26, 38, 255]; // #171a26 background
  const d = Math.abs(u - 0.5) + Math.abs(v - 0.5); // diamond metric
  if (d < 0.24) col = [124, 92, 255, 255]; // #7c5cff accent
  if (d < 0.11) col = [196, 181, 253, 255]; // #c4b5fd inner
  return col;
}

const iconsDir = path.join(__dirname, "..", "src-tauri", "icons");
fs.mkdirSync(iconsDir, { recursive: true });

const png256 = makePng(256, design);
fs.writeFileSync(path.join(iconsDir, "icon.png"), png256);

// ICO container: 6-byte header + 16-byte directory entry + PNG bytes.
const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(1, 4); // count
const entry = Buffer.alloc(16);
entry[0] = 0; // width 256 (0 means 256)
entry[1] = 0; // height 256
entry.writeUInt16LE(1, 2); // color planes
entry.writeUInt16LE(32, 4); // bits per pixel
entry.writeUInt32LE(png256.length, 8); // image size
entry.writeUInt32LE(6 + 16, 12); // data offset
fs.writeFileSync(path.join(iconsDir, "icon.ico"), Buffer.concat([header, entry, png256]));

console.log("icons written to", iconsDir);
