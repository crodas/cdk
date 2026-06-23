// Self-contained Cargo.lock transformer for standalone binding crates.
//
// Converts a workspace Cargo.lock into one valid for a detached binding crate
// by resolving path dependencies to registry dependencies (fetching checksums
// from the crates.io sparse index) and pruning unreachable workspace members.
//
// Compile: rustc misc/scripts/fix-lockfile.rs -o /tmp/fix-lockfile
// Usage:   /tmp/fix-lockfile <lockfile> <root-crate>

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::{env, fs, process};

const REGISTRY: &str = "registry+https://github.com/rust-lang/crates.io-index";
const INDEX_BASE: &str = "https://index.crates.io";

#[derive(Clone)]
struct Package {
    name: String,
    version: String,
    source: Option<String>,
    checksum: Option<String>,
    deps: Vec<String>, // "name" or "name version"
}

/// (name, version) key
type PkgKey = (String, String);

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <lockfile> <root-crate>", args[0]);
        process::exit(1);
    }
    let lockfile_path = &args[1];
    let root_crate = &args[2];

    let contents = fs::read_to_string(lockfile_path).unwrap_or_else(|e| {
        eprintln!("cannot read {lockfile_path}: {e}");
        process::exit(1);
    });

    let mut packages = parse_lockfile(&contents);

    // Find the root crate
    let root_key = packages
        .keys()
        .find(|(n, _)| n == root_crate)
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("root crate '{root_crate}' not found in lockfile");
            process::exit(1);
        });

    // BFS to find all reachable packages
    let reachable = find_reachable(&packages, &root_key);

    // Convert reachable path deps to registry deps
    let path_deps_to_convert: Vec<PkgKey> = reachable
        .iter()
        .filter(|k| {
            let pkg = &packages[*k];
            pkg.source.is_none() && **k != root_key
        })
        .cloned()
        .collect();

    for key in &path_deps_to_convert {
        let pkg = &packages[key];
        let cksum = fetch_checksum(&pkg.name, &pkg.version);
        let pkg = packages.get_mut(key).unwrap();
        pkg.source = Some(REGISTRY.to_string());
        pkg.checksum = Some(cksum);
    }

    // Prune unreachable
    packages.retain(|k, _| reachable.contains(k));

    // Write
    let out = serialize_lockfile(&packages);
    fs::write(lockfile_path, out.as_bytes()).unwrap_or_else(|e| {
        eprintln!("cannot write {lockfile_path}: {e}");
        process::exit(1);
    });

    eprintln!(
        "ok: {} packages kept, {} path deps converted",
        packages.len(),
        path_deps_to_convert.len()
    );
}

fn parse_lockfile(contents: &str) -> BTreeMap<PkgKey, Package> {
    let mut packages = BTreeMap::new();

    // Split into blocks separated by blank lines; each [[package]] block
    // starts with "[[package]]" and ends at the next blank line (or EOF).
    let mut in_package = false;
    let mut name = String::new();
    let mut version = String::new();
    let mut source: Option<String> = None;
    let mut checksum: Option<String> = None;
    let mut deps: Vec<String> = Vec::new();
    let mut in_deps = false;

    for line in contents.lines() {
        if line == "[[package]]" {
            if in_package {
                let key = (name.clone(), version.clone());
                packages.insert(
                    key,
                    Package { name: name.clone(), version: version.clone(), source: source.take(), checksum: checksum.take(), deps: std::mem::take(&mut deps) },
                );
            }
            in_package = true;
            name.clear();
            version.clear();
            source = None;
            checksum = None;
            deps.clear();
            in_deps = false;
            continue;
        }

        if !in_package {
            continue;
        }

        if in_deps {
            let trimmed = line.trim();
            if trimmed == "]" {
                in_deps = false;
                continue;
            }
            // Parse dep entry: " \"name\"," or " \"name version\","
            if let Some(s) = trimmed.strip_prefix('"') {
                if let Some(s) = s.strip_suffix(',').or(Some(s)) {
                    if let Some(s) = s.strip_suffix('"') {
                        deps.push(s.to_string());
                    }
                }
            }
            continue;
        }

        if line.starts_with("name = \"") {
            name = extract_quoted(line);
        } else if line.starts_with("version = \"") {
            version = extract_quoted(line);
        } else if line.starts_with("source = \"") {
            source = Some(extract_quoted(line));
        } else if line.starts_with("checksum = \"") {
            checksum = Some(extract_quoted(line));
        } else if line.starts_with("dependencies = [") {
            in_deps = true;
        }
    }

    // Flush last package
    if in_package && !name.is_empty() {
        let key = (name.clone(), version.clone());
        packages.insert(
            key,
            Package { name, version, source, checksum, deps },
        );
    }

    packages
}

fn extract_quoted(line: &str) -> String {
    let start = line.find('"').unwrap() + 1;
    let end = line[start..].find('"').unwrap() + start;
    line[start..end].to_string()
}

fn find_reachable(packages: &BTreeMap<PkgKey, Package>, root: &PkgKey) -> BTreeSet<PkgKey> {
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(root.clone());
    visited.insert(root.clone());

    // Build a name→keys index for resolving unversioned dep refs
    let mut by_name: BTreeMap<&str, Vec<&PkgKey>> = BTreeMap::new();
    for key in packages.keys() {
        by_name.entry(&key.0).or_default().push(key);
    }

    while let Some(key) = queue.pop_front() {
        let pkg = match packages.get(&key) {
            Some(p) => p,
            None => continue,
        };
        for dep in &pkg.deps {
            let resolved = resolve_dep(dep, &by_name);
            for r in resolved {
                if visited.insert(r.clone()) {
                    queue.push_back(r);
                }
            }
        }
    }

    visited
}

fn resolve_dep<'a>(dep: &str, by_name: &BTreeMap<&str, Vec<&'a PkgKey>>) -> Vec<PkgKey> {
    let parts: Vec<&str> = dep.splitn(2, ' ').collect();
    let name = parts[0];
    let version = parts.get(1).copied();

    match by_name.get(name) {
        Some(keys) => {
            if let Some(ver) = version {
                keys.iter()
                    .filter(|k| k.1 == ver)
                    .map(|k| (*k).clone())
                    .collect()
            } else {
                keys.iter().map(|k| (*k).clone()).collect()
            }
        }
        None => Vec::new(),
    }
}

fn fetch_checksum(name: &str, version: &str) -> String {
    let prefix = sparse_index_prefix(name);
    let url = format!("{INDEX_BASE}/{prefix}/{name}");

    let output = process::Command::new("curl")
        .args(["-sfL", &url])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run curl for {name}: {e}");
            process::exit(1);
        });

    if !output.status.success() {
        eprintln!(
            "curl failed for {name} v{version} ({}): {}",
            url,
            String::from_utf8_lossy(&output.stderr)
        );
        process::exit(1);
    }

    let body = String::from_utf8_lossy(&output.stdout);

    // Each line is a JSON object. Find the one with matching "vers".
    let vers_needle = format!("\"vers\":\"{version}\"");
    for line in body.lines() {
        // Normalize whitespace for matching
        let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if compact.contains(&vers_needle) {
            // Extract "cksum":"<hex>"
            if let Some(pos) = compact.find("\"cksum\":\"") {
                let start = pos + "\"cksum\":\"".len();
                let end = compact[start..].find('"').unwrap() + start;
                return compact[start..end].to_string();
            }
        }
    }

    eprintln!("checksum not found for {name} v{version} in sparse index");
    process::exit(1);
}

fn sparse_index_prefix(name: &str) -> String {
    match name.len() {
        1 => format!("1"),
        2 => format!("2"),
        3 => format!("3/{}", &name[..1]),
        _ => format!("{}/{}", &name[..2], &name[2..4]),
    }
}

fn serialize_lockfile(packages: &BTreeMap<PkgKey, Package>) -> String {
    let mut out = String::new();
    out.push_str("# This file is automatically @generated by Cargo.\n");
    out.push_str("# It is not intended for manual editing.\n");
    out.push_str("version = 4\n");

    for pkg in packages.values() {
        out.push_str("\n[[package]]\n");
        out.push_str(&format!("name = \"{}\"\n", pkg.name));
        out.push_str(&format!("version = \"{}\"\n", pkg.version));
        if let Some(src) = &pkg.source {
            out.push_str(&format!("source = \"{src}\"\n"));
        }
        if let Some(ck) = &pkg.checksum {
            out.push_str(&format!("checksum = \"{ck}\"\n"));
        }
        if !pkg.deps.is_empty() {
            out.push_str("dependencies = [\n");
            for dep in &pkg.deps {
                out.push_str(&format!(" \"{dep}\",\n"));
            }
            out.push_str("]\n");
        }
    }

    out
}
