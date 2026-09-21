const path = require('path');
const { getDefaultConfig, mergeConfig } = require('@react-native/metro-config');

const root = path.resolve(__dirname, '..');

/**
 * The app consumes the package from source in the parent directory, so Metro
 * has to watch it and resolve React and React Native to this app's copies.
 * Two copies of either breaks hooks and the JSI installer.
 */
const config = {
  watchFolders: [root],
  resolver: {
    // The package ships its entrypoints through `exports`, with a
    // `react-native` condition pointing at TypeScript source.
    unstable_enablePackageExports: true,
    unstable_conditionNames: ['react-native', 'require'],
    nodeModulesPaths: [
      path.resolve(__dirname, 'node_modules'),
      path.resolve(root, 'node_modules'),
    ],
    extraNodeModules: {
      '@cashu/cashu-native': root,
      react: path.resolve(__dirname, 'node_modules/react'),
      'react-native': path.resolve(__dirname, 'node_modules/react-native'),
    },
  },
};

module.exports = mergeConfig(getDefaultConfig(__dirname), config);
