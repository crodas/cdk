// Generates a Cargo.lock for a standalone binding crate that matches
// the workspace's dependency versions by pinning drifted transitive
// dependencies as exact-version constraints in Cargo.toml.
//
// Compile: rustc misc/scripts/fix-lockfile.rs -o /tmp/fix-lockfile
// Usage:   /tmp/fix-lockfile <target-dir> <reference-lockfile>

use std::collections::BTreeMap;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <target-dir> <reference-lockfile>", args[0]);
        process::exit(1);
    }
    let target_dir = &args[1];
    let reference_path = &args[2];

    let ref_contents = fs::read_to_string(reference_path).unwrap_or_else(|e| {
        eprintln!("cannot read {reference_path}: {e}");
        process::exit(1);
    });
    let ref_map = parse_versions(&ref_contents);

    // Step 1: generate fresh lock file
    eprintln!("generating lockfile...");
    run_cargo(target_dir, &["generate-lockfile"]);

    // Step 2: find drifted versions
    let target_lock = format!("{target_dir}/Cargo.lock");
    let target_contents = fs::read_to_string(&target_lock).unwrap();
    let target_map = parse_versions(&target_contents);
    let drifts = find_drifts(&target_map, &ref_map);

    if drifts.is_empty() {
        eprintln!("ok: all versions match reference");
        return;
    }

    // Step 3: pin drifted versions in Cargo.toml
    eprintln!("pinning {} drifted versions in Cargo.toml...", drifts.len());
    let cargo_toml_path = format!("{target_dir}/Cargo.toml");
    let mut cargo_toml = fs::read_to_string(&cargo_toml_path).unwrap_or_else(|e| {
        eprintln!("cannot read {cargo_toml_path}: {e}");
        process::exit(1);
    });

    // Insert pins, skipping deps already declared in Cargo.toml
    let pin_lines: String = drifts
        .iter()
        .filter(|(name, _, _)| {
            // Skip if already a direct dependency (e.g., cdk-ffi)
            let pat1 = format!("{name} = ");
            let pat2 = format!("{name} = {{");
            !cargo_toml.lines().any(|l| {
                let t = l.trim();
                t.starts_with(&pat1) || t.starts_with(&pat2)
            })
        })
        .map(|(name, _current, ref_ver)| {
            eprintln!("  {name} -> ={ref_ver}");
            format!("{name} = \"={ref_ver}\"\n")
        })
        .collect();

    // Find the end of [dependencies] section (next section header or EOF)
    if let Some(deps_pos) = cargo_toml.find("\n[dependencies]") {
        let after_header = deps_pos + "\n[dependencies]\n".len();
        cargo_toml.insert_str(after_header, &pin_lines);
    } else {
        cargo_toml.push_str(&format!("\n[dependencies]\n{pin_lines}"));
    }

    fs::write(&cargo_toml_path, &cargo_toml).unwrap_or_else(|e| {
        eprintln!("cannot write {cargo_toml_path}: {e}");
        process::exit(1);
    });

    // Step 4: regenerate lock file, iteratively removing yanked pins
    eprintln!("regenerating lockfile...");
    let mut yanked_pins: Vec<String> = Vec::new();

    loop {
        let gen_result = process::Command::new("cargo")
            .args(["generate-lockfile"])
            .current_dir(target_dir)
            .output()
            .unwrap();

        if gen_result.status.success() {
            break;
        }

        let stderr = String::from_utf8_lossy(&gen_result.stderr);
        if !stderr.contains("is yanked") {
            eprintln!("cargo generate-lockfile failed:\n{stderr}");
            process::exit(1);
        }

        // Extract yanked crate name: "requirement `name = ..."
        let mut found_yanked = false;
        for line in stderr.lines() {
            if line.contains("requirement `") {
                if let Some(spec) = line.split('`').nth(1) {
                    if let Some(name) = spec.split(' ').next() {
                        let name = name.to_string();
                        if !yanked_pins.contains(&name) {
                            eprintln!("  {name} is yanked, removing pin");
                            yanked_pins.push(name.clone());

                            let mut toml = fs::read_to_string(&cargo_toml_path).unwrap();
                            let pin_prefix = format!("{name} = \"=");
                            toml = toml.lines()
                                .filter(|l| !l.starts_with(&pin_prefix))
                                .collect::<Vec<_>>()
                                .join("\n") + "\n";
                            fs::write(&cargo_toml_path, &toml).unwrap();
                            found_yanked = true;
                            break; // retry after removing one
                        }
                    }
                }
            }
        }
        if !found_yanked {
            eprintln!("cargo generate-lockfile failed:\n{stderr}");
            process::exit(1);
        }
    }

    // Pin yanked crates via --precise (which allows yanked versions)
    if !yanked_pins.is_empty() {
        eprintln!("pinning {} yanked crates via --precise...", yanked_pins.len());
        // Re-read the lock to get current versions
        let mid_contents = fs::read_to_string(&target_lock).unwrap();
        let mid_map = parse_versions(&mid_contents);
        for name in &yanked_pins {
            let ref_ver = drifts.iter().find(|(n, _, _)| n == name).map(|(_, _, r)| r.as_str());
            let cur_ver = mid_map.keys().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
            if let (Some(ref_ver), Some(cur_ver)) = (ref_ver, cur_ver) {
                eprintln!("  {name} {cur_ver} -> {ref_ver} (yanked, --precise)");
                let _ = process::Command::new("cargo")
                    .args(["update", "-p", &format!("{name}@{cur_ver}"), "--precise", ref_ver])
                    .current_dir(target_dir)
                    .stdout(process::Stdio::null())
                    .stderr(process::Stdio::null())
                    .status();
            }
        }
    }

    // Step 5: verify
    let final_contents = fs::read_to_string(&target_lock).unwrap();
    let final_map = parse_versions(&final_contents);
    let remaining = find_drifts(&final_map, &ref_map);

    if remaining.is_empty() {
        eprintln!("ok: all {} versions pinned", drifts.len());
    } else {
        // Try --precise for remaining drifts
        for (name, current, ref_ver) in &remaining {
            let _ = process::Command::new("cargo")
                .args(["update", "-p", &format!("{name}@{current}"), "--precise", ref_ver])
                .current_dir(target_dir)
                .stdout(process::Stdio::null())
                .stderr(process::Stdio::null())
                .status();
        }

        let final2 = fs::read_to_string(&target_lock).unwrap();
        let final2_map = parse_versions(&final2);
        let remaining2 = find_drifts(&final2_map, &ref_map);
        if remaining2.is_empty() {
            eprintln!("ok: all {} versions pinned", drifts.len());
        } else {
            eprintln!("{} versions still differ:", remaining2.len());
            for (name, current, ref_ver) in &remaining2 {
                eprintln!("  {name} {current} (wanted {ref_ver})");
            }
            process::exit(1);
        }
    }
}

fn run_cargo(dir: &str, args: &[&str]) {
    let status = process::Command::new("cargo")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("cargo {} failed: {e}", args.join(" "));
            process::exit(1);
        });
    if !status.success() {
        eprintln!("cargo {} exited with {status}", args.join(" "));
        process::exit(1);
    }
}

fn find_drifts(
    target: &BTreeMap<(String, String), bool>,
    reference: &BTreeMap<(String, String), bool>,
) -> Vec<(String, String, String)> {
    let mut ref_by_name: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, ver) in reference.keys() {
        ref_by_name.entry(name).or_default().push(ver);
    }

    let mut drifts = Vec::new();
    for ((name, version), &has_source) in target {
        if !has_source { continue; }
        let ref_versions = match ref_by_name.get(name.as_str()) {
            Some(v) => v,
            None => continue,
        };
        let matched = if ref_versions.len() == 1 {
            Some(ref_versions[0])
        } else {
            ref_versions.iter().find(|v| same_major_minor(v, version)).copied()
        };
        if let Some(ref_ver) = matched {
            if ref_ver != version {
                drifts.push((name.clone(), version.clone(), ref_ver.to_string()));
            }
        }
    }
    drifts
}

fn parse_versions(contents: &str) -> BTreeMap<(String, String), bool> {
    let mut result = BTreeMap::new();
    let mut name = String::new();
    let mut version = String::new();
    let mut has_source = false;
    let mut in_pkg = false;

    for line in contents.lines() {
        if line == "[[package]]" {
            if in_pkg && !name.is_empty() {
                result.insert((name.clone(), version.clone()), has_source);
            }
            in_pkg = true;
            name.clear();
            version.clear();
            has_source = false;
            continue;
        }
        if !in_pkg { continue; }
        if line.starts_with("name = \"") { name = extract_quoted(line); }
        else if line.starts_with("version = \"") { version = extract_quoted(line); }
        else if line.starts_with("source = \"") { has_source = true; }
    }
    if in_pkg && !name.is_empty() {
        result.insert((name, version), has_source);
    }
    result
}

fn extract_quoted(line: &str) -> String {
    let start = line.find('"').unwrap() + 1;
    let end = line[start..].find('"').unwrap() + start;
    line[start..end].to_string()
}

fn same_major_minor(a: &str, b: &str) -> bool {
    let a: Vec<&str> = a.splitn(3, '.').collect();
    let b: Vec<&str> = b.splitn(3, '.').collect();
    a.len() >= 2 && b.len() >= 2 && a[0] == b[0] && a[1] == b[1]
}
