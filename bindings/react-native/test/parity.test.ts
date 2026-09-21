import assert from 'node:assert/strict';
import { test } from 'node:test';

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
const reference = OutputData;

const shape = (d: { blindedMessage: { amount: unknown; B_: string; id: string } }) => ({
  amount: String(d.blindedMessage.amount),
  B_: d.blindedMessage.B_,
  id: d.blindedMessage.id,
});

test('deterministic outputs match cashu-ts byte for byte', () => {
  for (const amount of [1, 5, 31, 1000]) {
    for (const counter of [0, 7, 4242]) {
      const mine = subject.createDeterministicData(amount, SEED, counter, keyset);
      const theirs = reference.createDeterministicData(amount, SEED, counter, keyset);
      assert.deepEqual(mine.map(shape), theirs.map(shape), `amount=${amount} counter=${counter}`);
    }
  }
});

test('deterministic blinding factors and secrets match cashu-ts', () => {
  const mine = subject.createDeterministicData(64, SEED, 12, keyset);
  const theirs = reference.createDeterministicData(64, SEED, 12, keyset);
  assert.deepEqual(
    mine.map((d) => d.blindingFactor.toString(16)),
    theirs.map((d) => d.blindingFactor.toString(16)),
  );
  assert.deepEqual(
    mine.map((d) => Buffer.from(d.secret).toString()),
    theirs.map((d) => Buffer.from(d.secret).toString()),
  );
});

test('a single deterministic output matches the batch at the same counter', () => {
  const batch = subject.createDeterministicData(8, SEED, 3, keyset);
  const single = subject.createSingleDeterministicData(8, SEED, 3, KEYSET_ID);
  assert.deepEqual(shape(single), shape(batch[0]));
});

test('random outputs use the same denominations as cashu-ts', () => {
  const mine = subject.createRandomData(1000, keyset);
  const theirs = reference.createRandomData(1000, keyset);
  assert.deepEqual(
    mine.map((d) => String(d.blindedMessage.amount)),
    theirs.map((d) => String(d.blindedMessage.amount)),
  );
  assert.equal(new Set(mine.map((d) => d.blindedMessage.B_)).size, mine.length);
});

test('a restore batch matches one deterministic call over blank denominations', () => {
  const count = 25;
  const viaRestore = native.createRestoreOutputs(SEED, KEYSET_ID, 100, count);
  const viaDeterministic = subject.createDeterministicData(
    0,
    SEED,
    100,
    keyset,
    Array<number>(count).fill(0),
  );
  assert.deepEqual(
    viaRestore.map((o) => o.blindedSecret),
    viaDeterministic.map((d) => d.blindedMessage.B_),
  );
});
