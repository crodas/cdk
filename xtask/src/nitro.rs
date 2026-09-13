//! The React Native / Nitro binding pipeline.

use std::fs;
use std::path::{Path, PathBuf};

use crate::shell::{self, Result, TaskError};

const FFI_CRATE: &str = "cashu-ffi";
const LIB_STEM: &str = "cashu_ffi";
const IOS_TARGETS: [&str; 3] = [
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "x86_64-apple-ios",
];
const ANDROID_TARGETS: [(&str, &str); 4] = [
    ("aarch64-linux-android", "arm64-v8a"),
    ("armv7-linux-androideabi", "armeabi-v7a"),
    ("x86_64-linux-android", "x86_64"),
    ("i686-linux-android", "x86"),
];

/// Paths and tools the pipeline needs.
#[derive(Debug)]
pub struct NitroTask {
    root: PathBuf,
    package: PathBuf,
}

impl NitroTask {
    /// Locate the workspace root and the React Native package.
    pub fn new() -> Result<Self> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or_else(|| TaskError::Missing {
                what: "workspace root".to_string(),
                path: env!("CARGO_MANIFEST_DIR").to_string(),
            })?
            .to_path_buf();
        let package = root.join("bindings/react-native");
        Ok(Self { root, package })
    }

    /// Build the cdylib, generate everything, and run nitrogen in between.
    pub fn bindings(&self, spec_only: bool) -> Result<()> {
        println!("==> building {FFI_CRATE} for the host");
        shell::run("cargo", ["build", "-p", FFI_CRATE], &self.root)?;
        let library = self.host_library()?;

        println!("==> generating the Nitro spec and the UniFFI bridge");
        let ts_out = self.package.join("src/generated");
        let cpp_out = self.package.join("cpp/generated");
        let node_out = self.package.join("test/generated");
        self.clean(&ts_out)?;
        self.clean(&cpp_out)?;
        self.clean(&node_out)?;
        shell::run(
            "cargo",
            [
                "run",
                "--quiet",
                "-p",
                "uniffi-bindgen-nitro",
                "--",
                "spec",
                "--library",
                library.to_string_lossy().as_ref(),
                "--crate",
                LIB_STEM,
                "--config",
                self.root
                    .join("crates/cashu-ffi/uniffi.toml")
                    .to_string_lossy()
                    .as_ref(),
                "--ts-out",
                ts_out.to_string_lossy().as_ref(),
                "--cpp-out",
                cpp_out.to_string_lossy().as_ref(),
                "--node-out",
                self.package.join("test/generated").to_string_lossy().as_ref(),
            ],
            &self.root,
        )?;

        if spec_only {
            println!("==> stopping before nitrogen (--spec-only)");
            return Ok(());
        }

        println!("==> running nitrogen");
        let node = self.node()?;
        let nitrogen = self.package.join("node_modules/nitrogen/lib/index.js");
        if !nitrogen.exists() {
            return Err(TaskError::Missing {
                what: "nitrogen".to_string(),
                path: nitrogen.to_string_lossy().into_owned(),
            });
        }
        shell::run(
            node.to_string_lossy().as_ref(),
            [nitrogen.to_string_lossy().as_ref()],
            &self.package,
        )?;

        println!("==> generating the Nitro adapters");
        shell::run(
            "cargo",
            [
                "run",
                "--quiet",
                "-p",
                "uniffi-bindgen-nitro",
                "--",
                "hybrids",
                "--library",
                library.to_string_lossy().as_ref(),
                "--crate",
                LIB_STEM,
                "--config",
                self.root
                    .join("crates/cashu-ffi/uniffi.toml")
                    .to_string_lossy()
                    .as_ref(),
                "--nitrogen",
                self.package.join("nitrogen").to_string_lossy().as_ref(),
                "--cpp-out",
                cpp_out.to_string_lossy().as_ref(),
            ],
            &self.root,
        )?;

        println!("==> done");
        Ok(())
    }

    /// Build the static library for every iOS target and assemble an XCFramework.
    pub fn ios(&self, release: bool) -> Result<()> {
        let profile = if release { "release" } else { "debug" };
        for target in IOS_TARGETS {
            let mut args = vec!["build", "-p", FFI_CRATE, "--target", target];
            if release {
                args.push("--release");
            }
            shell::run("cargo", args, &self.root)?;
        }

        let staging = self.package.join("ios/generated");
        self.clean(&staging)?;

        let device = self
            .root
            .join(format!("target/aarch64-apple-ios/{profile}/lib{LIB_STEM}.a"));
        let simulator = staging.join("simulator/lib").join(format!("lib{LIB_STEM}.a"));
        fs::create_dir_all(simulator.parent().unwrap_or(&staging)).map_err(|err| TaskError::Io {
            action: "create".to_string(),
            path: staging.to_string_lossy().into_owned(),
            reason: err.to_string(),
        })?;

        shell::run(
            "lipo",
            [
                "-create",
                self.root
                    .join(format!(
                        "target/aarch64-apple-ios-sim/{profile}/lib{LIB_STEM}.a"
                    ))
                    .to_string_lossy()
                    .as_ref(),
                self.root
                    .join(format!("target/x86_64-apple-ios/{profile}/lib{LIB_STEM}.a"))
                    .to_string_lossy()
                    .as_ref(),
                "-output",
                simulator.to_string_lossy().as_ref(),
            ],
            &self.root,
        )?;

        let xcframework = staging.join(format!("{LIB_STEM}.xcframework"));
        shell::run(
            "xcodebuild",
            [
                "-create-xcframework",
                "-library",
                device.to_string_lossy().as_ref(),
                "-library",
                simulator.to_string_lossy().as_ref(),
                "-output",
                xcframework.to_string_lossy().as_ref(),
            ],
            &self.root,
        )?;

        println!("==> {}", xcframework.to_string_lossy());
        Ok(())
    }

    /// Build the shared library for every Android ABI into `android/src/main/jniLibs`.
    pub fn android(&self, release: bool) -> Result<()> {
        if shell::capture("cargo", ["ndk", "--version"], &self.root).is_err() {
            return Err(TaskError::MissingTool {
                tool: "cargo-ndk".to_string(),
                hint: "install it with `cargo install cargo-ndk` and set ANDROID_NDK_HOME"
                    .to_string(),
            });
        }

        let jni_libs = self.package.join("android/src/main/jniLibs");
        self.clean(&jni_libs)?;

        let mut args: Vec<String> = vec!["ndk".to_string(), "-o".to_string()];
        args.push(jni_libs.to_string_lossy().into_owned());
        for (target, _) in ANDROID_TARGETS {
            args.push("-t".to_string());
            args.push(target.to_string());
        }
        args.push("build".to_string());
        args.push("-p".to_string());
        args.push(FFI_CRATE.to_string());
        if release {
            args.push("--release".to_string());
        }
        shell::run("cargo", args, &self.root)?;

        println!("==> {}", jni_libs.to_string_lossy());
        Ok(())
    }

    /// Compile and run the C++ harness against the generated bridge.
    pub fn test_cpp(&self) -> Result<()> {
        shell::run("cargo", ["build", "-p", FFI_CRATE], &self.root)?;
        let build = self.root.join("target/nitro-cpp-tests");
        fs::create_dir_all(&build).map_err(|err| TaskError::Io {
            action: "create".to_string(),
            path: build.to_string_lossy().into_owned(),
            reason: err.to_string(),
        })?;

        shell::run(
            "cmake",
            [
                "-S",
                self.package.join("cpp/test").to_string_lossy().as_ref(),
                "-B",
                build.to_string_lossy().as_ref(),
                format!("-DCDK_TARGET_DIR={}", self.root.join("target/debug").display()).as_str(),
                "-DCMAKE_BUILD_TYPE=Debug",
            ],
            &self.root,
        )?;
        shell::run("cmake", ["--build", build.to_string_lossy().as_ref()], &self.root)?;
        shell::run(
            build.join("bridge_tests").to_string_lossy().as_ref(),
            Vec::<String>::new(),
            &self.root,
        )
    }

    /// Type-check the generated Nitro adapters without a device toolchain.
    ///
    /// Nitro's headers are included as `<NitroModules/...>`, a layout only the
    /// CocoaPods and prefab builds produce, so a directory of links is staged
    /// to give clang the same view.
    pub fn check_cpp(&self) -> Result<()> {
        let nitro = self
            .package
            .join("node_modules/react-native-nitro-modules/cpp");
        let jsi = self.package.join("node_modules/react-native/ReactCommon/jsi");
        for path in [&nitro, &jsi] {
            if !path.exists() {
                return Err(TaskError::Missing {
                    what: "the React Native dependencies".to_string(),
                    path: path.to_string_lossy().into_owned(),
                });
            }
        }

        let staging = self.root.join("target/nitro-headers/NitroModules");
        self.clean(&staging)?;
        stage_headers(&nitro, &staging)?;

        let generated = self.package.join("cpp/generated");
        let mut sources: Vec<PathBuf> = Vec::new();
        let entries = fs::read_dir(&generated).map_err(|err| TaskError::Io {
            action: "read".to_string(),
            path: generated.to_string_lossy().into_owned(),
            reason: err.to_string(),
        })?;
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "cpp") {
                sources.push(entry.path());
            }
        }
        sources.sort();

        for source in sources {
            shell::run(
                "clang++",
                [
                    "-std=c++20",
                    "-fsyntax-only",
                    "-I",
                    self.root.join("target/nitro-headers").to_string_lossy().as_ref(),
                    "-I",
                    jsi.to_string_lossy().as_ref(),
                    "-I",
                    self.package
                        .join("nitrogen/generated/shared/c++")
                        .to_string_lossy()
                        .as_ref(),
                    "-I",
                    generated.to_string_lossy().as_ref(),
                    source.to_string_lossy().as_ref(),
                ],
                &self.root,
            )?;
        }
        println!("==> the generated adapters type-check against Nitro and JSI");
        Ok(())
    }

    /// Run the Node harness tests.
    pub fn test_node(&self) -> Result<()> {
        shell::run("cargo", ["build", "-p", FFI_CRATE], &self.root)?;
        let node = self.node()?;
        shell::run(
            node.to_string_lossy().as_ref(),
            ["--test", "test/harness.test.mjs", "test/parity.test.mjs"],
            &self.package,
        )
    }

    /// Run the TypeScript versus native benchmark.
    pub fn bench(&self) -> Result<()> {
        shell::run("cargo", ["build", "-p", FFI_CRATE, "--release"], &self.root)?;
        let node = self.node()?;
        shell::run(
            node.to_string_lossy().as_ref(),
            ["test/benchmark.mjs"],
            &self.package,
        )
    }

    fn host_library(&self) -> Result<PathBuf> {
        for extension in ["dylib", "so", "dll"] {
            let candidate = self
                .root
                .join(format!("target/debug/lib{LIB_STEM}.{extension}"));
            if candidate.exists() {
                return Ok(candidate);
            }
        }
        Err(TaskError::Missing {
            what: format!("the {FFI_CRATE} cdylib"),
            path: self.root.join("target/debug").to_string_lossy().into_owned(),
        })
    }

    /// Nitrogen needs Node 22 or newer; fall back to an nvm install if the
    /// default `node` on PATH is older.
    fn node(&self) -> Result<PathBuf> {
        if let Ok(version) = shell::capture("node", ["--version"], &self.root) {
            if major(&version) >= 22 {
                return Ok(PathBuf::from("node"));
            }
        }

        let home = std::env::var("HOME").unwrap_or_default();
        let versions = Path::new(&home).join(".nvm/versions/node");
        let mut best: Option<(u32, PathBuf)> = None;
        if let Ok(entries) = fs::read_dir(&versions) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                let major = major(&name);
                if major >= 22 && best.as_ref().is_none_or(|(seen, _)| major > *seen) {
                    best = Some((major, entry.path().join("bin/node")));
                }
            }
        }

        best.map(|(_, path)| path).ok_or(TaskError::MissingTool {
            tool: "Node 22 or newer".to_string(),
            hint: "nitrogen needs it; install with `nvm install 22`".to_string(),
        })
    }

    fn clean(&self, directory: &Path) -> Result<()> {
        if directory.exists() {
            fs::remove_dir_all(directory).map_err(|err| TaskError::Io {
                action: "remove".to_string(),
                path: directory.to_string_lossy().into_owned(),
                reason: err.to_string(),
            })?;
        }
        fs::create_dir_all(directory).map_err(|err| TaskError::Io {
            action: "create".to_string(),
            path: directory.to_string_lossy().into_owned(),
            reason: err.to_string(),
        })
    }
}

/// Link every Nitro header into one directory so `<NitroModules/X.hpp>` resolves.
fn stage_headers(source: &Path, staging: &Path) -> Result<()> {
    let entries = fs::read_dir(source).map_err(|err| TaskError::Io {
        action: "read".to_string(),
        path: source.to_string_lossy().into_owned(),
        reason: err.to_string(),
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            stage_headers(&path, staging)?;
        } else if path.extension().is_some_and(|ext| ext == "hpp" || ext == "h") {
            let Some(name) = path.file_name() else { continue };
            let target = staging.join(name);
            if target.exists() {
                continue;
            }
            std::os::unix::fs::symlink(&path, &target).map_err(|err| TaskError::Io {
                action: "link".to_string(),
                path: target.to_string_lossy().into_owned(),
                reason: err.to_string(),
            })?;
        }
    }
    Ok(())
}

fn major(version: &str) -> u32 {
    version
        .trim_start_matches('v')
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}
