// Fixed small feasibility scene; not the full Stage 2 conformance matrix.
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
import sharp from 'sharp';
const evidence = new URL('../docs/evidence/', import.meta.url);
const a = await sharp(new URL('stage2-probe/probe-cpu.png', evidence).pathname)
  .raw()
  .toBuffer({ resolveWithObject: true });
const b = await sharp(new URL('stage2-probe/probe-gpu.png', evidence).pathname)
  .raw()
  .toBuffer({ resolveWithObject: true });
assert.deepEqual(a.info, b.info);
let sum = 0,
  max = 0,
  bad = 0;
for (let i = 0; i < a.data.length; i += 4) {
  let p = 0;
  for (let k = 0; k < 3; k++) {
    const d = Math.abs(a.data[i + k] - b.data[i + k]);
    sum += d;
    max = Math.max(max, d);
    p = Math.max(p, d);
  }
  if (p > 32) bad++;
}
const pixels = a.info.width * a.info.height;
const report = {
  scene:
    'stage2_probe: Persian/English/emoji fallback, gradient, translated clip, shadow, group/image alpha and custom drawing path/text',
  viewport: [a.info.width, a.info.height],
  dpr: 1,
  thresholds: { meanAbsoluteRGB: 2, percentPixelsOver32: 2 },
  meanAbsoluteRGB: sum / (pixels * 3),
  maximumChannelDifference: max,
  percentPixelsOver32: (100 * bad) / pixels,
  limitations:
    'Matched native CPU/Metal 900x720 DPR1 conformance fixture. Different text rasterizers; Windows/browser parity and performance certification remain unverified.',
};
report.result =
  report.meanAbsoluteRGB <= 2 && report.percentPixelsOver32 <= 2 ? 'passed' : 'failed';
await fs.writeFile(
  new URL('stage2-renderer-comparison.json', evidence),
  JSON.stringify(report, null, 2) + '\n',
);
console.log(JSON.stringify(report, null, 2));
assert.equal(report.result, 'passed');
