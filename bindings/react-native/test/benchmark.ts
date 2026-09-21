import { OutputData } from '@cashu/cashu-ts';

import { NativeOutputDataCreatorBase } from '../src/creator';
import * as native from './generated/index';

const KEYSET_ID = '009a1f293253e41e';
const SEED = new Uint8Array(Buffer.from('5e1b9d0a'.repeat(16), 'hex'));
const keyset = {
  id: KEYSET_ID,
  keys: Object.fromEntries(
    Array.from({ length: 32 }, (_, i) => [String(2 ** i), '02'.padEnd(66, 'a')]),
  ),
};
const subject = new NativeOutputDataCreatorBase(native);

function time(label: string, runs: number, fn: () => unknown) {
  fn();
  const start = performance.now();
  for (let i = 0; i < runs; i++) fn();
  const ms = (performance.now() - start) / runs;
  console.log(`${label.padEnd(44)} ${ms.toFixed(2)} ms`);
  return ms;
}

const zeros = Array<number>(500).fill(0);
const a = time('restore 500 counters (cashu-ts)', 5, () =>
  OutputData.createDeterministicData(0, SEED, 0, keyset, zeros),
);
const b = time('restore 500 counters (native)', 5, () =>
  subject.createDeterministicData(0, SEED, 0, keyset, zeros),
);
console.log(`${''.padEnd(44)} ${(a / b).toFixed(1)}x\n`);

const c = time('deterministic outputs for 1000 sat (cashu-ts)', 50, () =>
  OutputData.createDeterministicData(1000, SEED, 0, keyset),
);
const d = time('deterministic outputs for 1000 sat (native)', 50, () =>
  subject.createDeterministicData(1000, SEED, 0, keyset),
);
console.log(`${''.padEnd(44)} ${(c / d).toFixed(1)}x`);
