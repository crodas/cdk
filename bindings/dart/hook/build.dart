import 'dart:convert';
import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:crypto/crypto.dart';
import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';
import 'package:path/path.dart' as p;

/// Name of the manifest the release workflow writes into the published package.
const _manifestFileName = 'prebuilt_manifest.json';

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets) return;

    final codeConfig = input.config.code;
    final targetTriple = _targetTriple(codeConfig);
    final linkMode = _linkMode(codeConfig);
    final packageRoot = p.fromUri(input.packageRoot);
    final libFileName =
        codeConfig.targetOS.libraryFileName('cdk_ffi_dart', linkMode);
    final key = '$targetTriple/$libFileName';

    final manifestFile = File(p.join(packageRoot, _manifestFileName));
    if (manifestFile.existsSync()) {
      // Re-run the hook if the manifest changes.
      output.dependencies.add(manifestFile.uri);
    }

    final library = await _resolveLibrary(
      input: input,
      packageRoot: packageRoot,
      manifestFile: manifestFile,
      key: key,
    );

    if (library != null) {
      final outputPath = p.join(p.fromUri(input.outputDirectory), libFileName);
      await File(library).copy(outputPath);

      output.assets.code.add(
        CodeAsset(
          package: input.packageName,
          name: 'uniffi:cdk',
          linkMode: linkMode,
          file: Uri.file(outputPath),
        ),
      );
      return;
    }

    // Only the monorepo ships the Rust crate. In the published package its
    // absence is terminal: there is nothing to compile, so say exactly what
    // was missing rather than failing somewhere deeper.
    if (!File(p.join(packageRoot, 'rust', 'Cargo.toml')).existsSync()) {
      throw StateError(
        'No prebuilt library for $key, and this package ships no Rust sources '
        'to build one from. Expected an entry in $_manifestFileName, or a file '
        'at prebuilt/$key, or a prebuilt_dir user-define pointing at one.',
      );
    }

    await _buildFromSource(input: input, output: output);
  });
}

/// Returns a path to a usable library, or null if one has to be built.
///
/// A user-supplied directory wins over everything, then a library committed in
/// the package, then the shared cache, then a download.
Future<String?> _resolveLibrary({
  required BuildInput input,
  required String packageRoot,
  required File manifestFile,
  required String key,
}) async {
  final override = input.userDefines.path('prebuilt_dir');
  if (override != null) {
    final path = p.join(p.fromUri(override), key);
    if (File(path).existsSync()) {
      print('cdk: using prebuilt_dir override $path');
      return path;
    }
    print('cdk: prebuilt_dir is set but has no $key');
  }

  if (input.userDefines['force_build'] == true) {
    print('cdk: force_build is set, skipping prebuilt libraries');
    return null;
  }

  final local = p.join(packageRoot, 'prebuilt', key);
  if (File(local).existsSync()) {
    print('cdk: using committed prebuilt $local');
    return local;
  }

  if (!manifestFile.existsSync()) {
    return null;
  }

  final manifest =
      jsonDecode(await manifestFile.readAsString()) as Map<String, dynamic>;
  final assets = manifest['assets'] as Map<String, dynamic>?;
  final entry = assets?[key] as Map<String, dynamic>?;
  if (entry == null) {
    print('cdk: $_manifestFileName has no entry for $key');
    return null;
  }

  final expectedDigest = entry['sha256'] as String;
  final url = Uri.parse('${manifest['baseUrl']}${entry['asset']}');
  final cached = File(p.join(
    p.fromUri(input.outputDirectoryShared),
    'prebuilt',
    _shortTag(manifest['tag'] as String),
    key,
  ));

  if (cached.existsSync()) {
    if (await _digestOf(cached) == expectedDigest) {
      print('cdk: using cached ${cached.path}');
      return cached.path;
    }
    // A truncated or corrupted cache entry is recoverable; a bad download is
    // not. Drop it and fetch once more.
    print('cdk: cached ${cached.path} failed its checksum, refetching');
    await cached.delete();
  }

  return await _download(url: url, into: cached, expectedDigest: expectedDigest);
}

/// Streams [url] through gzip into [into], verifying [expectedDigest].
///
/// Returns null when the asset cannot be fetched, so the caller can decide
/// whether building from source is an option. A checksum failure is not in that
/// category and throws.
Future<String?> _download({
  required Uri url,
  required File into,
  required String expectedDigest,
}) async {
  // The asset is already gzip, so transport encoding would only be redundant.
  // Decoding it ourselves keeps the pipeline deterministic.
  final client = HttpClient()
    ..connectionTimeout = const Duration(seconds: 30)
    ..autoUncompress = false;
  final temp = File('${into.path}.$pid.tmp');
  await into.parent.create(recursive: true);

  try {
    print('cdk: downloading $url');
    final response = await client
        .getUrl(url)
        .then((request) => request.close())
        .timeout(const Duration(minutes: 4));

    if (response.statusCode != 200) {
      // A missing or renamed asset arrives as a normal response, not an error.
      print('cdk: $url returned HTTP ${response.statusCode}');
      return null;
    }

    // Streamed to disk first, then hashed from disk: a second pass over a
    // local file is cheaper than holding 30 MB in memory.
    final fileSink = temp.openWrite();
    try {
      await fileSink.addStream(response.transform(gzip.decoder));
    } finally {
      await fileSink.close();
    }

    final actual = await _digestOf(temp);
    if (actual != expectedDigest) {
      throw StateError(
        'Checksum mismatch for $url: expected $expectedDigest, got $actual. '
        'Refusing to use the downloaded library.',
      );
    }

    await temp.rename(into.path);
    print('cdk: cached ${into.path}');
    return into.path;
  } on SocketException catch (e) {
    print('cdk: could not reach $url: $e');
    return null;
  } on HandshakeException catch (e) {
    print('cdk: TLS failure for $url: $e');
    return null;
  } on HttpException catch (e) {
    print('cdk: HTTP failure for $url: $e');
    return null;
  } finally {
    client.close(force: false);
    if (temp.existsSync()) {
      await temp.delete();
    }
  }
}

Future<String> _digestOf(File file) async {
  final digest = await sha256.bind(file.openRead()).first;
  return digest.toString();
}

/// Keeps cache paths short. Windows path limits bite otherwise, since nightly
/// tags are long and the shared output directory is already deep.
String _shortTag(String tag) =>
    sha256.convert(utf8.encode(tag)).toString().substring(0, 12);

Future<void> _buildFromSource({
  required BuildInput input,
  required BuildOutputBuilder output,
}) async {
  // native_toolchain_rust replaces the process environment entirely when
  // spawning cargo (Dart's Process.run with an explicit environment map).
  // On Linux this results in an empty environment, so cargo can't find
  // system libraries like OpenSSL. Forward the environment the hook runner
  // allowed through so nix-provided paths reach cargo's build scripts.
  final env = Map<String, String>.from(Platform.environment);

  // Fallback: if OPENSSL_DIR/OPENSSL_INCLUDE_DIR/OPENSSL_LIB_DIR are not set
  // but we're in a nix shell, extract openssl paths from NIX_CFLAGS_COMPILE
  // and NIX_LDFLAGS (which nix always populates for packages in buildInputs).
  if (!env.containsKey('OPENSSL_DIR') &&
      !env.containsKey('OPENSSL_INCLUDE_DIR')) {
    final cflags = env['NIX_CFLAGS_COMPILE'] ?? '';
    final ldflags = env['NIX_LDFLAGS'] ?? '';

    final includeMatch =
        RegExp(r'-isystem\s+(\S*openssl[^/]*/include)').firstMatch(cflags);
    final libMatch = RegExp(r'-L(\S*openssl[^/]*/lib)').firstMatch(ldflags);

    if (includeMatch != null) {
      env['OPENSSL_INCLUDE_DIR'] = includeMatch.group(1)!;
    }
    if (libMatch != null) {
      env['OPENSSL_LIB_DIR'] = libMatch.group(1)!;
    }
  }

  final builder = RustBuilder(
    assetName: 'uniffi:cdk',
    extraCargoEnvironmentVariables: env,
  );
  await builder.run(input: input, output: output);
}

String _targetTriple(CodeConfig config) {
  return switch ((config.targetOS, config.targetArchitecture)) {
    (OS.android, Architecture.arm64) => 'aarch64-linux-android',
    (OS.android, Architecture.arm) => 'armv7-linux-androideabi',
    (OS.android, Architecture.x64) => 'x86_64-linux-android',
    (OS.iOS, Architecture.arm64) => 'aarch64-apple-ios',
    (OS.windows, Architecture.x64) => 'x86_64-pc-windows-msvc',
    (OS.linux, Architecture.arm64) => 'aarch64-unknown-linux-gnu',
    (OS.linux, Architecture.x64) => 'x86_64-unknown-linux-gnu',
    (OS.macOS, Architecture.arm64) => 'aarch64-apple-darwin',
    (OS.macOS, Architecture.x64) => 'x86_64-apple-darwin',
    _ => throw UnsupportedError(
        'Unsupported target: ${config.targetOS} / ${config.targetArchitecture}'),
  };
}

LinkMode _linkMode(CodeConfig config) {
  return switch (config.linkModePreference) {
    LinkModePreference.dynamic ||
    LinkModePreference.preferDynamic =>
      DynamicLoadingBundled(),
    LinkModePreference.static ||
    LinkModePreference.preferStatic =>
      StaticLinking(),
    _ => throw UnsupportedError(
        'Unsupported LinkModePreference: ${config.linkModePreference}'),
  };
}
