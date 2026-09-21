import * as native from '../turbo/index';
import { NativeOutputDataCreatorBase, type P2PKFallback } from './creator';

export type { P2PKFallback, CashuNativeApi } from './creator';

/**
 * A cashu-ts `OutputDataCreator` that builds blinded outputs in Rust.
 *
 * Imports through the package entrypoint rather than the generated bindings
 * directly, because that is what installs the Rust crate into Hermes. Lives
 * behind the `/creator` subpath because it is the one part of the package that
 * needs cashu-ts at runtime; the root entrypoint stays importable without it.
 *
 * @example
 *
 *     import { NativeOutputDataCreator } from '@cashu/cashu-native/creator';
 *
 *     const wallet = new Wallet(mint, {
 *       outputDataCreator: new NativeOutputDataCreator(),
 *     });
 */
export class NativeOutputDataCreator extends NativeOutputDataCreatorBase {
  constructor(fallback?: P2PKFallback) {
    super(native, fallback);
  }
}
