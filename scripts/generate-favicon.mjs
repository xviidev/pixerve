// Package the existing Pixerve icon as a multi-resolution ICO; no downloads.
import { writeFile } from 'node:fs/promises';
import sharp from 'sharp';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const sizes = [16, 32, 48, 256];
const frames = await Promise.all(sizes.map(size => sharp(fileURLToPath(new URL('assets/pixerve-icon.png', root)))
  .resize(size, size).png().toBuffer()));
const directory = Buffer.alloc(6 + sizes.length * 16);
directory.writeUInt16LE(1, 2);
directory.writeUInt16LE(sizes.length, 4);
let offset = directory.length;
for (let i = 0; i < sizes.length; i++) {
  const entry = 6 + i * 16;
  directory[entry] = sizes[i] === 256 ? 0 : sizes[i];
  directory[entry + 1] = directory[entry];
  directory.writeUInt16LE(1, entry + 4);
  directory.writeUInt16LE(32, entry + 6);
  directory.writeUInt32LE(frames[i].length, entry + 8);
  directory.writeUInt32LE(offset, entry + 12);
  offset += frames[i].length;
}
await writeFile(new URL('assets/favicon.ico', root), Buffer.concat([directory, ...frames]));
console.log('Generated assets/favicon.ico (16, 32, 48, 256 px).');
