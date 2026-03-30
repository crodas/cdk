/**
 * CDK React Native Bindings - Wallet API Surface Tests
 *
 * These tests validate that the React Native binding package is correctly
 * structured and that the generated bindings expose the same wallet API
 * as the Swift and Dart bindings.
 *
 * Reference tests that exercise the actual FFI against testnut.cashudevkit.org:
 *   - Swift:  bindings/swift/Tests/CdkTests.swift
 *   - Dart:   bindings/dart/test/wallet_test.dart
 *
 * Since React Native tests cannot exercise the actual Rust FFI without a
 * device/simulator runtime, these tests validate:
 *   1. The native turbo module can be loaded (mocked)
 *   2. The package structure and exports are correct
 *   3. The generated TypeScript bindings expose all required wallet APIs
 *      with correct signatures (matching Swift/Dart test expectations)
 */

import * as fs from 'fs';
import * as path from 'path';

// Test the native module interface
describe('NativeCdkReactNative', () => {
  it('should load the turbo module', () => {
    const nativeModule =
      require('../src/NativeCdkReactNative').default;
    expect(nativeModule).toBeDefined();
    expect(nativeModule.installRustCrate).toBeDefined();
    expect(nativeModule.cleanupRustCrate).toBeDefined();
  });

  it('installRustCrate should return true', () => {
    const nativeModule =
      require('../src/NativeCdkReactNative').default;
    expect(nativeModule.installRustCrate()).toBe(true);
  });

  it('cleanupRustCrate should return true', () => {
    const nativeModule =
      require('../src/NativeCdkReactNative').default;
    expect(nativeModule.cleanupRustCrate()).toBe(true);
  });
});

// Test package structure
describe('Package structure', () => {
  it('should have a valid package.json', () => {
    const pkg = require('../package.json');
    expect(pkg.name).toBe('@cashudevkit/cdk-react-native');
    expect(pkg.dependencies['uniffi-bindgen-react-native']).toBe('0.30.0-1');
  });

  it('should have ubrn config', () => {
    const configPath = path.join(__dirname, '..', 'ubrn.config.yaml');
    expect(fs.existsSync(configPath)).toBe(true);
  });

  it('should have rust wrapper crate', () => {
    const cargoPath = path.join(__dirname, '..', 'rust', 'Cargo.toml');
    expect(fs.existsSync(cargoPath)).toBe(true);

    const content = fs.readFileSync(cargoPath, 'utf-8');
    expect(content).toContain('cdk-ffi-react-native');
    expect(content).toContain('cdk-ffi');
  });
});

/**
 * Wallet API surface tests
 *
 * These mirror the Dart test (bindings/dart/test/wallet_test.dart) and
 * Swift test (bindings/swift/Tests/CdkTests.swift) by verifying the
 * generated TypeScript bindings export all the types and functions needed
 * to perform the same wallet operations:
 *
 *   1. Create a Wallet with (mintUrl, unit, mnemonic, store, config)
 *   2. Call wallet.totalBalance() → Amount { value }
 *   3. Call wallet.mintQuote(paymentMethod, amount, description, extra) → MintQuote { id, request }
 *   4. Call wallet.mint(quoteId, amountSplitTarget, spendingConditions) → Proof[]
 *
 * Since the generated code requires a JSI native runtime to execute,
 * we validate by parsing the generated TypeScript source to confirm
 * all required exports exist with the correct shapes.
 */
describe('Wallet API surface (generated bindings)', () => {
  let generatedSource: string;

  beforeAll(() => {
    const generatedPath = path.join(
      __dirname, '..', 'src', 'generated', 'cdk_ffi.ts'
    );
    expect(fs.existsSync(generatedPath)).toBe(true);
    generatedSource = fs.readFileSync(generatedPath, 'utf-8');
  });

  // -- Wallet class --

  it('should export Wallet class with correct constructor', () => {
    // Dart: Wallet(mintUrl, unit, mnemonic, store, config)
    // Swift: Wallet(mintUrl, unit, mnemonic, store, config)
    expect(generatedSource).toMatch(
      /export class Wallet/
    );
    expect(generatedSource).toMatch(
      /constructor\(mintUrl:\s*string,\s*unit:\s*CurrencyUnit,\s*mnemonic:\s*string,\s*store:\s*WalletStore,\s*config:\s*WalletConfig\)/
    );
  });

  it('should have Wallet.totalBalance() returning Promise<Amount>', () => {
    // Dart: wallet.totalBalance() → Amount
    // Swift: wallet.totalBalance() → Amount
    expect(generatedSource).toMatch(
      /async\s+totalBalance\(.*\):\s*Promise<Amount>/
    );
  });

  it('should have Wallet.mintQuote() with correct parameters', () => {
    // Dart: wallet.mintQuote(paymentMethod, amount, description, extra)
    // Swift: wallet.mintQuote(paymentMethod, amount, description, extra)
    expect(generatedSource).toMatch(
      /async\s+mintQuote\(\s*paymentMethod:\s*PaymentMethod,\s*amount:\s*Amount\s*\|\s*undefined,\s*description:\s*string\s*\|\s*undefined,\s*extra:\s*string\s*\|\s*undefined/
    );
    expect(generatedSource).toMatch(
      /mintQuote\(.*\):\s*Promise<MintQuote>/
    );
  });

  it('should have Wallet.mint() with correct parameters', () => {
    // Dart: wallet.mint(quoteId, amountSplitTarget, spendingConditions)
    // Swift: wallet.mint(quoteId, amountSplitTarget, spendingConditions)
    expect(generatedSource).toMatch(
      /async\s+mint\(\s*quoteId:\s*string,\s*amountSplitTarget:\s*SplitTarget,\s*spendingConditions:\s*SpendingConditions\s*\|\s*undefined/
    );
    expect(generatedSource).toMatch(
      /mint\(.*\):\s*Promise<Array<Proof>>/
    );
  });

  // -- Helper functions --

  it('should export generateMnemonic()', () => {
    // Dart: generateMnemonic()
    // Swift: generateMnemonic()
    expect(generatedSource).toMatch(
      /export function generateMnemonic\(\):\s*string/
    );
  });

  it('should export sqliteWalletStore()', () => {
    // Dart: SqliteWalletStore(dbPath)
    // Swift: .sqlite(path: dbPath)
    expect(generatedSource).toMatch(
      /export function sqliteWalletStore\(path:\s*string\):\s*WalletStore/
    );
  });

  // -- Record types --

  it('should export Amount record with bigint value field', () => {
    // Dart: Amount(value: 100)
    // Swift: Amount(value: 100)
    expect(generatedSource).toMatch(
      /export type Amount\s*=\s*\{[^}]*value:\s*\/\*u64\*\/bigint/
    );
  });

  it('should export WalletConfig record with optional targetProofCount', () => {
    // Dart: WalletConfig(targetProofCount: null)
    // Swift: WalletConfig(targetProofCount: nil)
    expect(generatedSource).toMatch(
      /export type WalletConfig\s*=\s*\{[^}]*targetProofCount\??:\s*\/\*u32\*\/number(\s*\|\s*undefined)?/
    );
  });

  it('should export MintQuote record with id and request fields', () => {
    // Dart: quote.id, quote.request
    // Swift: quote.id, quote.request
    expect(generatedSource).toMatch(
      /export type MintQuote\s*=\s*\{[^}]*id:\s*string/
    );
    expect(generatedSource).toMatch(
      /export type MintQuote\s*=\s*\{[^}]*request:\s*string/s
    );
  });

  // -- Enum variants --

  it('should export CurrencyUnit.Sat variant', () => {
    // Dart: SatCurrencyUnit()
    // Swift: .sat
    expect(generatedSource).toMatch(/CurrencyUnit_Tags/);
    expect(generatedSource).toContain('Sat = "Sat"');
  });

  it('should export PaymentMethod.Bolt11 variant', () => {
    // Dart: Bolt11PaymentMethod()
    // Swift: .bolt11
    expect(generatedSource).toMatch(/PaymentMethod_Tags/);
    expect(generatedSource).toContain('Bolt11 = "Bolt11"');
  });

  it('should export SplitTarget.None variant', () => {
    // Dart: NoneSplitTarget()
    // Swift: .none
    expect(generatedSource).toMatch(/SplitTarget_Tags/);
    expect(generatedSource).toContain('None = "None"');
  });
});
