// Original 16×16 pixel sprites for Tiny Factory. Run with `node tools/generate_sprites.mjs`.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const out = path.join(import.meta.dirname, '..', 'assets', 'sprites');
fs.mkdirSync(out, { recursive: true });
const P = {
  ink: '#25303a', shadow: '#344450', steel: '#708490', light: '#adc2c2',
  grass: '#659d61', grassDark: '#4f8053', grassLight: '#8bbc70',
  dirt: '#92704f', dirtDark: '#75573e', dirtLight: '#b08a5d',
  sand: '#d4b77b', sandDark: '#b59963', sandLight: '#ead29b',
  water: '#3d7894', waterDark: '#2d617d', waterLight: '#78b6bd',
  rock: '#68727b', rockDark: '#4e5864', rockLight: '#9ca8a9',
  iron: '#899daa', ironLight: '#d0e0dc', copper: '#be7148', copperLight: '#f2ab67',
  wood: '#865c43', woodLight: '#b38255', leaf: '#3e8a61', leafLight: '#71b86e',
  orange: '#df9256', yellow: '#efc873', blue: '#6bc1d0', purple: '#826fa0',
};

function sprite(name, draw) {
  const data = Buffer.alloc(16 * 16 * 4);
  const color = (hex, alpha = 255) => {
    const n = Number.parseInt(hex.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255, alpha];
  };
  const pixel = (x, y, hex, alpha = 255) => {
    if (x < 0 || x > 15 || y < 0 || y > 15) return;
    const at = (y * 16 + x) * 4;
    data.set(color(hex, alpha), at);
  };
  const rect = (x, y, w, h, hex, alpha = 255) => {
    for (let yy = y; yy < y + h; yy++) for (let xx = x; xx < x + w; xx++) pixel(xx, yy, hex, alpha);
  };
  const points = (coords, hex) => coords.forEach(([x, y]) => pixel(x, y, hex));
  draw({ pixel, rect, points });
  const rows = Buffer.alloc(16 * (1 + 16 * 4));
  for (let y = 0; y < 16; y++) data.copy(rows, y * 65 + 1, y * 64, (y + 1) * 64);
  const chunk = (type, body) => {
    const label = Buffer.from(type);
    const length = Buffer.alloc(4); length.writeUInt32BE(body.length);
    const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(Buffer.concat([label, body])));
    return Buffer.concat([length, label, body, crc]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(16, 0); header.writeUInt32BE(16, 4);
  header[8] = 8; header[9] = 6;
  const png = Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', header), chunk('IDAT', zlib.deflateSync(rows)), chunk('IEND', Buffer.alloc(0)),
  ]);
  fs.writeFileSync(path.join(out, `${name}.png`), png);
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

sprite('grass', ({ rect, points }) => {
  rect(0, 0, 16, 16, P.grass);
  points([[2, 3], [7, 2], [13, 4], [4, 11], [10, 12], [14, 14]], P.grassLight);
  points([[1, 8], [5, 5], [9, 8], [12, 10], [6, 14]], P.grassDark);
  rect(3, 6, 1, 2, P.grassDark); rect(11, 2, 1, 2, P.grassDark);
});
sprite('dirt', ({ rect, points }) => {
  rect(0, 0, 16, 16, P.dirt);
  points([[2, 2], [6, 4], [12, 3], [3, 9], [9, 8], [13, 12], [5, 14]], P.dirtLight);
  rect(8, 2, 2, 1, P.dirtDark); rect(1, 12, 3, 1, P.dirtDark); rect(10, 13, 2, 1, P.dirtDark);
});
sprite('sand', ({ rect, points }) => {
  rect(0, 0, 16, 16, P.sand);
  rect(1, 4, 5, 1, P.sandLight); rect(8, 9, 6, 1, P.sandLight);
  rect(4, 6, 3, 1, P.sandDark); rect(11, 13, 3, 1, P.sandDark);
  points([[2, 11], [7, 2], [14, 4]], P.sandDark);
});
sprite('water', ({ rect }) => {
  rect(0, 0, 16, 16, P.water);
  rect(0, 4, 5, 1, P.waterLight); rect(9, 3, 5, 1, P.waterLight);
  rect(4, 10, 6, 1, P.waterLight); rect(11, 12, 5, 1, P.waterDark);
  rect(2, 13, 4, 1, P.waterDark); rect(7, 6, 5, 1, P.waterDark);
});
sprite('rock', ({ rect, points }) => {
  rect(0, 0, 16, 16, P.rock);
  rect(0, 0, 7, 7, P.rockDark); rect(9, 9, 7, 7, P.rockDark);
  rect(2, 2, 3, 1, P.rockLight); rect(10, 10, 4, 1, P.rockLight);
  points([[7, 5], [8, 6], [6, 11], [5, 12], [12, 3]], P.ink);
});

sprite('iron_deposit', ({ rect, points }) => {
  rect(2, 8, 12, 5, P.ink); rect(3, 6, 5, 6, P.iron); rect(8, 4, 5, 8, P.steel);
  rect(4, 5, 2, 2, P.ironLight); rect(9, 5, 3, 2, P.ironLight);
  points([[3, 11], [7, 9], [12, 10]], P.blue);
});
sprite('copper_deposit', ({ rect, points }) => {
  rect(2, 9, 12, 4, P.ink); rect(3, 6, 5, 6, P.copper); rect(8, 4, 5, 8, P.copper);
  rect(4, 6, 2, 2, P.copperLight); rect(9, 5, 3, 2, P.copperLight);
  points([[5, 10], [11, 9]], P.yellow);
});
sprite('stone_deposit', ({ rect }) => {
  rect(2, 9, 12, 4, P.ink); rect(3, 7, 6, 5, P.rock); rect(8, 5, 5, 7, P.rockLight);
  rect(4, 8, 2, 1, P.light); rect(9, 6, 2, 1, P.light);
  rect(11, 10, 2, 1, P.rockDark);
});
sprite('tree', ({ rect, points }) => {
  rect(7, 9, 3, 6, P.wood); rect(8, 10, 1, 4, P.woodLight);
  rect(4, 3, 9, 9, P.ink); rect(5, 2, 7, 9, P.leaf);
  rect(3, 6, 3, 4, P.leaf); rect(11, 5, 3, 4, P.leaf);
  points([[6, 4], [9, 3], [11, 7], [5, 8]], P.leafLight);
});
sprite('miner', ({ rect, points }) => {
  rect(2, 4, 12, 11, P.ink); rect(3, 5, 10, 8, P.steel);
  rect(4, 7, 8, 4, P.shadow); rect(5, 8, 6, 2, P.blue);
  rect(3, 12, 3, 2, P.iron); rect(10, 12, 3, 2, P.iron);
  rect(6, 2, 4, 4, P.orange); rect(7, 0, 2, 2, P.yellow);
  points([[4, 5], [11, 5]], P.light);
});
sprite('conveyor', ({ rect, points }) => {
  rect(1, 1, 14, 14, P.ink); rect(2, 2, 12, 12, P.dirtDark);
  rect(4, 2, 8, 12, P.steel); rect(5, 4, 6, 2, P.shadow); rect(5, 10, 6, 2, P.shadow);
  rect(7, 4, 2, 8, P.yellow); rect(5, 6, 6, 2, P.yellow);
  points([[7, 2], [8, 2]], P.light);
});
sprite('crafter', ({ rect, points }) => {
  rect(1, 2, 14, 13, P.ink); rect(2, 3, 12, 10, P.purple);
  rect(4, 5, 8, 7, P.shadow); rect(5, 6, 6, 5, P.orange);
  rect(6, 7, 4, 3, P.yellow); rect(3, 2, 3, 2, P.light);
  rect(10, 2, 3, 2, P.light); rect(3, 12, 3, 2, P.steel); rect(10, 12, 3, 2, P.steel);
  points([[4, 7], [11, 7]], P.ink);
});
sprite('player', ({ rect, points }) => {
  rect(5, 3, 7, 10, P.ink); rect(6, 4, 5, 5, P.yellow);
  rect(7, 6, 3, 2, P.woodLight); rect(6, 9, 5, 4, P.blue);
  rect(4, 10, 2, 4, P.steel); rect(11, 10, 2, 4, P.steel);
  rect(6, 13, 2, 2, P.shadow); rect(9, 13, 2, 2, P.shadow);
  points([[8, 6]], P.ink);
});
sprite('iron_ore', ({ rect }) => {
  rect(3, 7, 10, 6, P.ink); rect(4, 6, 5, 6, P.iron); rect(9, 5, 3, 6, P.ironLight);
  rect(5, 7, 2, 1, P.blue);
});
sprite('copper_ore', ({ rect }) => {
  rect(3, 7, 10, 6, P.ink); rect(4, 6, 5, 6, P.copper); rect(9, 5, 3, 6, P.copperLight);
  rect(5, 7, 2, 1, P.yellow);
});
sprite('stone', ({ rect }) => {
  rect(3, 8, 10, 5, P.ink); rect(4, 6, 6, 6, P.rock); rect(9, 7, 3, 5, P.rockLight);
});
sprite('wood', ({ rect }) => {
  rect(4, 4, 8, 10, P.ink); rect(5, 5, 6, 8, P.wood); rect(7, 5, 2, 8, P.woodLight);
});
sprite('iron_plate', ({ rect }) => {
  rect(3, 4, 10, 9, P.ink); rect(4, 5, 8, 7, P.iron); rect(5, 6, 6, 2, P.ironLight);
  rect(5, 10, 6, 1, P.shadow);
});
sprite('copper_plate', ({ rect }) => {
  rect(3, 4, 10, 9, P.ink); rect(4, 5, 8, 7, P.copper); rect(5, 6, 6, 2, P.copperLight);
  rect(5, 10, 6, 1, P.wood);
});
sprite('mechanical_part', ({ rect, points }) => {
  rect(5, 3, 6, 10, P.ink); rect(3, 5, 10, 6, P.ink);
  rect(5, 5, 6, 6, P.yellow); rect(7, 7, 2, 2, P.ink);
  points([[7, 2], [8, 2], [7, 13], [8, 13], [2, 7], [13, 7]], P.orange);
});

console.log(`Wrote ${fs.readdirSync(out).length} original sprites to ${out}`);
