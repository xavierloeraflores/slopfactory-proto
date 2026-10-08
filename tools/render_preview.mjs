// Render a save as a deterministic pixel-art scene without a GPU or Bevy.
// Usage: node tools/render_preview.mjs world.json preview.png [centerX centerY]
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';

const [, , source, destination, centerX = '32', centerY = '32'] = process.argv;
if (!source || !destination) {
  console.error('Usage: node tools/render_preview.mjs world.json preview.png [centerX centerY]');
  process.exit(2);
}
const world = JSON.parse(fs.readFileSync(source, 'utf8'));
const spriteDir = path.join(import.meta.dirname, '..', 'assets', 'sprites');
const cache = new Map();
const tilesWide = 32;
const tilesHigh = 22;
const scale = 3;
const width = tilesWide * 16 * scale;
const height = tilesHigh * 16 * scale;
const pixels = Buffer.alloc(width * height * 4);
const minX = Math.round(Number(centerX)) - Math.floor(tilesWide / 2);
const minY = Math.round(Number(centerY)) - Math.floor(tilesHigh / 2);

const terrainName = { Grass: 'grass', Dirt: 'dirt', Sand: 'sand', Water: 'water', Rock: 'rock' };
const resourceName = { Iron: 'iron_deposit', Copper: 'copper_deposit', Stone: 'stone_deposit', Tree: 'tree' };
const structureName = { Miner: 'miner', Conveyor: 'conveyor', Crafter: 'crafter' };
const itemName = { IronOre: 'iron_ore', CopperOre: 'copper_ore', Stone: 'stone', Wood: 'wood',
  IronPlate: 'iron_plate', CopperPlate: 'copper_plate', MechanicalPart: 'mechanical_part' };
const snakeToPascal = value => value.replace(/(^|_)([a-z])/g, (_, _prefix, letter) => letter.toUpperCase());
const resources = new Map(Object.values(world.resources).map(node => [node.id, node]));
const structures = new Map(Object.values(world.structures).map(machine => [machine.id, machine]));

function tileAt(x, y) {
  if (x < 0 || y < 0 || x >= 64 || y >= 64) return null;
  const chunk = world.chunks[Math.floor(y / 16) * 4 + Math.floor(x / 16)];
  return chunk.tiles[(y % 16) * 16 + (x % 16)];
}

function sprite(name) {
  if (cache.has(name)) return cache.get(name);
  const data = fs.readFileSync(path.join(spriteDir, `${name}.png`));
  let at = 8;
  const idats = [];
  while (at < data.length) {
    const length = data.readUInt32BE(at);
    const type = data.toString('ascii', at + 4, at + 8);
    if (type === 'IDAT') idats.push(data.subarray(at + 8, at + 8 + length));
    at += length + 12;
  }
  const inflated = zlib.inflateSync(Buffer.concat(idats));
  const rgba = Buffer.alloc(16 * 16 * 4);
  for (let row = 0; row < 16; row++) {
    if (inflated[row * 65] !== 0) throw new Error(`Unexpected PNG filter in ${name}`);
    inflated.copy(rgba, row * 64, row * 65 + 1, row * 65 + 65);
  }
  cache.set(name, rgba);
  return rgba;
}

function draw(name, worldX, worldY, orientation = 'north', size = 16) {
  const image = sprite(name);
  const tileX = (worldX - minX) * 16 * scale;
  const tileY = (minY + tilesHigh - 1 - worldY) * 16 * scale;
  const pad = Math.floor((16 - size) / 2) * scale;
  for (let y = 0; y < size * scale; y++) {
    for (let x = 0; x < size * scale; x++) {
      let sx = Math.floor(x / scale * 16 / size);
      let sy = Math.floor(y / scale * 16 / size);
      [sx, sy] = orientation === 'east' ? [sy, 15 - sx]
        : orientation === 'south' ? [15 - sx, 15 - sy]
          : orientation === 'west' ? [15 - sy, sx] : [sx, sy];
      const from = (sy * 16 + sx) * 4;
      const alpha = image[from + 3] / 255;
      if (!alpha) continue;
      const px = tileX + pad + x;
      const py = tileY + pad + y;
      if (px < 0 || py < 0 || px >= width || py >= height) continue;
      const to = (py * width + px) * 4;
      for (let channel = 0; channel < 3; channel++) {
        pixels[to + channel] = Math.round(image[from + channel] * alpha + pixels[to + channel] * (1 - alpha));
      }
      pixels[to + 3] = 255;
    }
  }
}

for (let y = minY; y < minY + tilesHigh; y++) {
  for (let x = minX; x < minX + tilesWide; x++) {
    const tile = tileAt(x, y);
    if (!tile) continue;
    draw(terrainName[snakeToPascal(tile.terrain)], x, y);
    if (tile.resource_id) {
      const node = resources.get(tile.resource_id);
      if (node?.quantity > 0) draw(resourceName[snakeToPascal(node.kind)], x, y);
    }
    if (tile.structure_id) {
      const machine = structures.get(tile.structure_id);
      if (machine) {
        const kind = Object.keys(machine.state)[0];
        draw(structureName[snakeToPascal(kind)], x, y, machine.direction);
        if (kind === 'Conveyor' && machine.state.Conveyor.item) {
          draw(itemName[snakeToPascal(machine.state.Conveyor.item)], x, y, 'north', 10);
        }
      }
    }
  }
}
draw('player', world.player.position.x, world.player.position.y);

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, body) {
  const label = Buffer.from(type);
  const length = Buffer.alloc(4); length.writeUInt32BE(body.length);
  const crc = Buffer.alloc(4); crc.writeUInt32BE(crc32(Buffer.concat([label, body])));
  return Buffer.concat([length, label, body, crc]);
}
const rows = Buffer.alloc(height * (1 + width * 4));
for (let row = 0; row < height; row++) pixels.copy(rows, row * (1 + width * 4) + 1, row * width * 4, (row + 1) * width * 4);
const header = Buffer.alloc(13); header.writeUInt32BE(width, 0); header.writeUInt32BE(height, 4); header[8] = 8; header[9] = 6;
fs.mkdirSync(path.dirname(destination), { recursive: true });
fs.writeFileSync(destination, Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
  chunk('IHDR', header), chunk('IDAT', zlib.deflateSync(rows)), chunk('IEND', Buffer.alloc(0))]));
console.log(`Rendered ${destination} (${width}×${height})`);
