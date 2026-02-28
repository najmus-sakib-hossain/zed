import fs from 'node:fs';
import path from 'node:path';

import { normalizeWhatsAppTarget as normalizeDx } from '../src/bridge/whatsapp/normalize.js';
import { normalizeWhatsAppTarget as normalizeOpenClaw } from '../../../docs/agent/openclaw_whatsapp_normalize_reference.mjs';

const ITERATIONS = Number.parseInt(process.env.DX_WA_NORMALIZE_BENCH_ITERS ?? '200000', 10);
const OUTPUT_JSON = process.env.DX_WA_NORMALIZE_BENCH_OUTPUT_JSON ?? 'docs/agent/openclaw-vs-dx-normalize-benchmark.json';
const OUTPUT_MD = process.env.DX_WA_NORMALIZE_BENCH_OUTPUT_MD ?? 'docs/agent/openclaw-vs-dx-normalize-benchmark.md';

const corpus = [
  '120363401234567890@g.us',
  '123456789-987654321@g.us',
  'whatsapp:120363401234567890@g.us',
  '41796666864:0@s.whatsapp.net',
  '1234567890:123@s.whatsapp.net',
  '41796666864@s.whatsapp.net',
  '123456789@lid',
  '123456789@LID',
  '1555123@s.whatsapp.net',
  '+15551234567',
  'whatsapp:+15551234567',
  'whatsapp:whatsapp:+1555',
  'group:123456789-987654321@g.us',
  'whatsapp:group:120363401234567890@g.us',
  'abc@s.whatsapp.net',
  'wat',
  'whatsapp:',
  '@g.us',
  ' ',
];

function bench(name, fn) {
  const start = process.hrtime.bigint();
  let sink = 0;
  let mismatches = 0;

  for (let i = 0; i < ITERATIONS; i += 1) {
    const sample = corpus[i % corpus.length];
    const out = fn(sample);
    if (out === null) {
      sink += 1;
    } else {
      sink += out.length;
    }
  }

  const elapsedNs = Number(process.hrtime.bigint() - start);
  const avgNs = elapsedNs / ITERATIONS;

  return {
    name,
    iterations: ITERATIONS,
    totalNs: elapsedNs,
    avgNs,
    sink,
    mismatches,
  };
}

function verifyParity() {
  const mismatches = [];
  for (const sample of corpus) {
    const dx = normalizeDx(sample);
    const oc = normalizeOpenClaw(sample);
    if (dx !== oc) {
      mismatches.push({ sample, dx, openclaw: oc });
    }
  }
  return mismatches;
}

const parityMismatches = verifyParity();
const dxResult = bench('dx', normalizeDx);
const openclawResult = bench('openclaw', normalizeOpenClaw);

const speedup = openclawResult.avgNs > 0 ? openclawResult.avgNs / dxResult.avgNs : null;
const faster = speedup && speedup > 1 ? 'dx' : 'openclaw';

const report = {
  timestamp: new Date().toISOString(),
  corpusSize: corpus.length,
  parityMismatches,
  results: {
    dx: dxResult,
    openclaw: openclawResult,
  },
  summary: {
    faster,
    speedup,
  },
};

fs.mkdirSync(path.dirname(OUTPUT_JSON), { recursive: true });
fs.writeFileSync(OUTPUT_JSON, JSON.stringify(report, null, 2));

const md = [
  '# OpenClaw vs DX WhatsApp Normalize Benchmark',
  '',
  `- Timestamp: ${report.timestamp}`,
  `- Iterations per implementation: ${ITERATIONS}`,
  `- Corpus size: ${corpus.length}`,
  `- Parity mismatches: ${parityMismatches.length}`,
  '',
  '## Results',
  '',
  '| Impl | Avg ns/op | Total ms |',
  '|---|---:|---:|',
  `| DX | ${dxResult.avgNs.toFixed(2)} | ${(dxResult.totalNs / 1_000_000).toFixed(2)} |`,
  `| OpenClaw reference | ${openclawResult.avgNs.toFixed(2)} | ${(openclawResult.totalNs / 1_000_000).toFixed(2)} |`,
  '',
  speedup
    ? `- Winner: **${faster}** (${Math.max(speedup, 1 / speedup).toFixed(2)}x faster)`
    : '- Winner: n/a',
  '',
  '## Notes',
  '',
  '- OpenClaw reference implementation is adapted from source files in integrations/openclaw.',
  '- This benchmark measures target normalization path only.',
  '- End-to-end channel throughput should be measured separately.',
  '',
];

fs.mkdirSync(path.dirname(OUTPUT_MD), { recursive: true });
fs.writeFileSync(OUTPUT_MD, md.join('\n'));

console.log(JSON.stringify({
  outputJson: OUTPUT_JSON,
  outputMd: OUTPUT_MD,
  parityMismatches: parityMismatches.length,
  faster,
  speedup,
}, null, 2));
