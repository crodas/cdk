import React, { useEffect, useState } from 'react';
import { SafeAreaView, ScrollView, StyleSheet, Text, View } from 'react-native';

import { Wallet, OutputData } from '@cashu/cashu-ts';
import { uniffiInitAsync } from '@cashu/cashu-native';
import { NativeOutputDataCreator } from '@cashu/cashu-native/creator';

const KEYSET_ID = '009a1f293253e41e';

/** A 64 byte BIP39 seed. A real wallet reads this from secure storage. */
const SEED = Uint8Array.from(
  ('5e1b9d0a'.repeat(16).match(/../g) as string[]).map((b) => parseInt(b, 16)),
);

/** A keyset is `amount -> mint public key`; the demo only needs the amounts. */
const keyset = {
  id: KEYSET_ID,
  keys: Object.fromEntries(
    Array.from({ length: 32 }, (_, i) => [String(2 ** i), '02'.padEnd(66, 'a')]),
  ),
};

type Row = { label: string; value: string; ok?: boolean };

export default function App() {
  const [rows, setRows] = useState<Row[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        await uniffiInitAsync();
        const result = run();
        result.forEach((r) => console.log(`[cashu-native] ${r.label}: ${r.value}`));
        setRows(result);
      } catch (e) {
        const message = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
        console.log(`[cashu-native] FAILED: ${message}`);
        setError(message);
      }
    })();
  }, []);

  return (
    <SafeAreaView style={styles.screen}>
      <ScrollView contentContainerStyle={styles.body}>
        <Text style={styles.title}>cashu-native</Text>
        <Text style={styles.subtitle}>
          cashu-ts output construction, running in Rust over the JSI bridge.
        </Text>
        {error !== null && <Text style={styles.error}>{error}</Text>}
        {rows.map((row) => (
          <View key={row.label} style={styles.row}>
            <Text style={styles.label}>{row.label}</Text>
            <Text style={[styles.value, row.ok === false && styles.bad]}>{row.value}</Text>
          </View>
        ))}
      </ScrollView>
    </SafeAreaView>
  );
}

function run(): Row[] {
  const rows: Row[] = [];

  // 1. This is the whole integration: hand cashu-ts a native OutputDataCreator.
  //    Everything else about the wallet is unchanged.
  const creator = new NativeOutputDataCreator();
  const wallet = new Wallet('https://mint.example', {
    outputDataCreator: creator,
    bip39seed: SEED,
  });
  rows.push({ label: 'Wallet', value: `wired to ${wallet.mint.mintUrl}` });

  // 2. The native path must agree with cashu-ts byte for byte, or a restored
  //    wallet would derive different secrets than the ones it spent.
  const native = creator.createDeterministicData(1000, SEED, 0, keyset);
  const pure = OutputData.createDeterministicData(1000, SEED, 0, keyset);
  const same =
    native.length === pure.length &&
    native.every((d, i) => d.blindedMessage.B_ === pure[i].blindedMessage.B_);
  rows.push({
    label: 'Parity with cashu-ts',
    value: same ? `${native.length} outputs identical` : 'MISMATCH',
    ok: same,
  });
  rows.push({ label: 'First B_', value: native[0].blindedMessage.B_.slice(0, 32) + '…' });

  // 3. The reason the module exists: a NUT-09 restore batch.
  const zeros = Array<number>(500).fill(0);
  const jsMs = time(() => OutputData.createDeterministicData(0, SEED, 0, keyset, zeros));
  const rsMs = time(() => creator.createDeterministicData(0, SEED, 0, keyset, zeros));
  rows.push({ label: 'Restore 500, cashu-ts', value: `${jsMs.toFixed(0)} ms` });
  rows.push({ label: 'Restore 500, native', value: `${rsMs.toFixed(0)} ms` });
  rows.push({ label: 'Speedup', value: `${(jsMs / rsMs).toFixed(1)}x`, ok: rsMs < jsMs });

  return rows;
}

function time(fn: () => unknown): number {
  const start = Date.now();
  fn();
  return Date.now() - start;
}

const styles = StyleSheet.create({
  screen: { flex: 1, backgroundColor: '#12100e' },
  body: { padding: 24, gap: 4 },
  title: { color: '#f5f0e8', fontSize: 28, fontWeight: '700' },
  subtitle: { color: '#8e857a', fontSize: 14, marginBottom: 20, lineHeight: 20 },
  row: { borderTopWidth: StyleSheet.hairlineWidth, borderTopColor: '#332f2a', paddingVertical: 12 },
  label: { color: '#8e857a', fontSize: 12, textTransform: 'uppercase', letterSpacing: 1 },
  value: { color: '#f5f0e8', fontSize: 17, marginTop: 4, fontVariant: ['tabular-nums'] },
  bad: { color: '#e5484d' },
  error: { color: '#e5484d', fontSize: 14, marginBottom: 16 },
});
