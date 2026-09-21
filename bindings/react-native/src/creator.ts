import {
  Amount,
  OutputData,
  splitAmount,
  type AmountLike,
  type HasKeysetKeys,
  type OutputDataCreator,
  type OutputDataLike,
  type P2PKOptions,
} from '@cashu/cashu-ts';

import type { BlindedOutput, P2pkOptions } from './generated/cashu_ffi';
import { SigFlag } from './generated/cashu_ffi';

/**
 * The native entry points this module needs.
 *
 * Declared structurally rather than importing the module, so the pure logic
 * here can be bound to either the JSI bindings or the N-API ones the tests
 * run against.
 */
export type CashuNativeApi = {
  createRandomOutputs(amounts: Array<bigint>, keysetId: string): Array<BlindedOutput>;
  createSingleRandomOutput(amount: bigint, keysetId: string): BlindedOutput;
  createDeterministicOutputs(
    amounts: Array<bigint>,
    seed: Uint8Array,
    counter: number,
    keysetId: string,
  ): Array<BlindedOutput>;
  createSingleDeterministicOutput(
    amount: bigint,
    seed: Uint8Array,
    counter: number,
    keysetId: string,
  ): BlindedOutput;
  createP2pkOutputs(
    p2pk: P2pkOptions,
    amounts: Array<bigint>,
    keysetId: string,
  ): Array<BlindedOutput>;
  createSingleP2pkOutput(p2pk: P2pkOptions, amount: bigint, keysetId: string): BlindedOutput;
};

/**
 * The subset of {@link OutputDataCreator} used when a lock cannot go native.
 */
export type P2PKFallback = Pick<OutputDataCreator, 'createP2PKData' | 'createSingleP2PKData'>;

const stockP2PKFallback: P2PKFallback = {
  createP2PKData: (p2pk, amount, keyset, customSplit) =>
    OutputData.createP2PKData(p2pk, amount, keyset, customSplit),
  createSingleP2PKData: (p2pk, amount, keysetId) =>
    OutputData.createSingleP2PKData(p2pk, amount, keysetId),
};

/**
 * A cashu-ts {@link OutputDataCreator} backed by a native binding.
 *
 * Only the construction of outputs moves native. `toProof` stays on the
 * canonical cashu-ts {@link OutputData}, so the amount binding, DLEQ and
 * NUT-28 checks that protect a wallet against a malicious mint are unchanged.
 *
 * Takes the native module as an argument rather than importing it, so the same
 * logic serves the JSI bindings and the N-API ones the tests run against.
 */
export class NativeOutputDataCreatorBase implements OutputDataCreator {
  private readonly native: CashuNativeApi;
  private readonly fallback: P2PKFallback;

  /**
   * @param fallback Handles the shapes the native surface does not express:
   *   HTLC locks, NUT-28 blinded keys and caller-supplied extra tags.
   */
  constructor(native: CashuNativeApi, fallback: P2PKFallback = stockP2PKFallback) {
    this.native = native;
    this.fallback = fallback;
  }

  createRandomData(
    amount: AmountLike,
    keyset: HasKeysetKeys,
    customSplit?: AmountLike[],
  ): OutputDataLike[] {
    const amounts = splitAmount(amount, keyset.keys, customSplit);
    return this.native.createRandomOutputs(amounts.map(toBigInt), keyset.id).map(toOutputData);
  }

  createSingleRandomData(amount: AmountLike, keysetId: string): OutputDataLike {
    return toOutputData(this.native.createSingleRandomOutput(toBigInt(amount), keysetId));
  }

  createDeterministicData(
    amount: AmountLike,
    seed: Uint8Array,
    counter: number,
    keyset: HasKeysetKeys,
    customSplit?: AmountLike[],
  ): OutputDataLike[] {
    const amounts = splitAmount(amount, keyset.keys, customSplit);
    return this.native
      .createDeterministicOutputs(amounts.map(toBigInt), seed, counter, keyset.id)
      .map(toOutputData);
  }

  createSingleDeterministicData(
    amount: AmountLike,
    seed: Uint8Array,
    counter: number,
    keysetId: string,
  ): OutputDataLike {
    return toOutputData(
      this.native.createSingleDeterministicOutput(toBigInt(amount), seed, counter, keysetId),
    );
  }

  createP2PKData(
    p2pk: P2PKOptions,
    amount: AmountLike,
    keyset: HasKeysetKeys,
    customSplit?: AmountLike[],
  ): OutputDataLike[] {
    const options = toNativeP2PK(p2pk);
    if (options === undefined) {
      return this.fallback.createP2PKData(p2pk, amount, keyset, customSplit);
    }
    const amounts = splitAmount(amount, keyset.keys, customSplit);
    return this.native
      .createP2pkOutputs(options, amounts.map(toBigInt), keyset.id)
      .map(toOutputData);
  }

  createSingleP2PKData(p2pk: P2PKOptions, amount: AmountLike, keysetId: string): OutputDataLike {
    const options = toNativeP2PK(p2pk);
    if (options === undefined) {
      return this.fallback.createSingleP2PKData(p2pk, amount, keysetId);
    }
    return toOutputData(this.native.createSingleP2pkOutput(options, toBigInt(amount), keysetId));
  }
}

const encoder = new TextEncoder();

function toOutputData(output: BlindedOutput): OutputDataLike {
  return new OutputData(
    { amount: Amount.from(output.amount), B_: output.blindedSecret, id: output.keysetId },
    BigInt('0x' + output.blindingFactor),
    encoder.encode(output.secret),
  );
}

function toBigInt(amount: AmountLike): bigint {
  return Amount.from(amount).toBigInt();
}

/**
 * Map a cashu-ts lock onto the native one, or `undefined` when the native
 * surface cannot express it and the caller should fall back.
 */
function toNativeP2PK(p2pk: P2PKOptions): P2pkOptions | undefined {
  if (p2pk.kind !== 'P2PK') return undefined;
  if (p2pk.blindKeys === true) return undefined;
  if (p2pk.additionalTags !== undefined && p2pk.additionalTags.length > 0) return undefined;

  return {
    pubkey: p2pk.data,
    additionalPubkeys: p2pk.pubkeys,
    numSigs: p2pk.requiredSignatures === undefined ? undefined : BigInt(p2pk.requiredSignatures),
    locktime: p2pk.locktime === undefined ? undefined : BigInt(p2pk.locktime),
    refundPubkeys: p2pk.refundKeys,
    numSigsRefund:
      p2pk.requiredRefundSignatures === undefined
        ? undefined
        : BigInt(p2pk.requiredRefundSignatures),
    sigFlag: p2pk.sigFlag === 'SIG_ALL' ? SigFlag.SigAll : SigFlag.SigInputs,
  };
}

export type { HasKeysetKeys, OutputDataCreator, OutputDataLike };
