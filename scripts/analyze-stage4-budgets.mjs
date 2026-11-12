// Decode the allocator's documented [live, peak, blocks] tuple from an existing
// raw trace. Never rerun/change samples to turn a failed budget into a pass.
import fs from 'node:fs/promises';
import assert from 'node:assert/strict';
const file = process.argv[2];
if (!file) throw Error('Pass a budget JSON report');
const report = JSON.parse(await fs.readFile(file));
for (const row of [report.memoryBaseline, ...report.lifecycle]) {
  if (Array.isArray(row.allocations)) {
    const raw = row.allocations;
    assert(raw.length === 3 && raw[0] > 0);
    row.allocations = { raw, live_bytes: raw[0], peak_bytes: raw[1], live_blocks: raw[2] };
  }
}
report.memoryFinal = report.lifecycle.at(-1);
report.liveGrowthPercent =
  (report.memoryFinal.allocations.live_bytes / report.memoryBaseline.allocations.live_bytes - 1) *
  100;
report.gates.lifecycleAllocation = report.liveGrowthPercent <= 10;
report.result = Object.values(report.gates).every((v) => v !== false)
  ? 'passed-local-measured-gates'
  : 'failed-local-gates';
report.analysisCorrection =
  'Allocator tuple decoded into named fields; frame/input/resource samples unchanged. Original report retained in history/stage4/pre-final-replay.';
await fs.writeFile(file, JSON.stringify(report, null, 2) + '\n');
console.log(
  JSON.stringify(
    {
      frameP95Ms: report.frameP95Ms,
      inputP95Ms: report.inputP95Ms,
      liveGrowthPercent: report.liveGrowthPercent,
      gates: report.gates,
      result: report.result,
    },
    null,
    2,
  ),
);
