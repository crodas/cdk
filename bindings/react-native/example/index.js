/**
 * @format
 */

// Hermes ships no TextEncoder/TextDecoder, and cashu-ts needs both to turn
// secrets into bytes. It constructs the decoder with `{ ignoreBOM, fatal }`,
// which rules out the smaller polyfills that ignore those options. Must come
// before anything that imports cashu-ts.
import { TextDecoder, TextEncoder } from '@zxing/text-encoding';

globalThis.TextDecoder ??= TextDecoder;
globalThis.TextEncoder ??= TextEncoder;

import { AppRegistry } from 'react-native';
import App from './App';
import { name as appName } from './app.json';

AppRegistry.registerComponent(appName, () => App);
