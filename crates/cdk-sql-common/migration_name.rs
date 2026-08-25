use std::cmp::Ordering;
use std::path::Path;

const INVALID_PATH_CHARACTER: &str =
    "control, bidirectional formatting, and double-quote characters are not allowed";

pub(crate) fn validate_migration_path(path: &str) -> Result<(), &'static str> {
    if path
        .chars()
        .any(|ch| ch.is_control() || is_bidi_control(ch) || ch == '"')
    {
        Err(INVALID_PATH_CHARACTER)
    } else {
        Ok(())
    }
}

pub(crate) fn rust_string_literal(value: &str) -> String {
    format!("{value:?}")
}

/// Orders migrations by backend directory, then by the leading numeric version,
/// then by full file name. The final tie-break is what keeps the generated
/// `MIGRATIONS` array independent of `fs::read_dir` order when two migrations
/// share a version prefix.
pub(crate) fn compare_migration_paths(path_a: &Path, path_b: &Path, skip_name: usize) -> Ordering {
    let parts = |path: &Path| {
        path.to_str().unwrap().replace("\\", "/")[skip_name + 1..]
            .split('/')
            .map(|part| part.to_owned())
            .collect::<Vec<_>>()
    };
    let backend = |parts: &[String]| {
        if parts.len() == 2 {
            parts.first().cloned().unwrap_or_default()
        } else {
            String::new()
        }
    };

    let backend_cmp = backend(&parts(path_a)).cmp(&backend(&parts(path_b)));
    if backend_cmp != Ordering::Equal {
        return backend_cmp;
    }

    let name_a = path_a.file_name().unwrap().to_str().unwrap();
    let name_b = path_b.file_name().unwrap().to_str().unwrap();

    let version = |name: &str| {
        name.split('_')
            .next()
            .and_then(|prefix| prefix.parse::<usize>().ok())
            .unwrap_or_default()
    };
    let (version_a, version_b) = (version(name_a), version(name_b));

    if version_a != 0 && version_b != 0 {
        version_a.cmp(&version_b).then_with(|| name_a.cmp(name_b))
    } else {
        name_a.cmp(name_b)
    }
}

const fn is_bidi_control(ch: char) -> bool {
    matches!(
        ch,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_migration_paths() {
        assert_eq!(
            validate_migration_path("wallet/migrations/001_initialize.sql"),
            Ok(())
        );
    }

    #[test]
    fn rejects_source_and_output_injection_characters() {
        for path in [
            "wallet/migrations/001_newline\n.sql",
            "wallet/migrations/001_carriage\rreturn.sql",
            "wallet/migrations/001_escape\u{1b}.sql",
            "wallet/migrations/001_bell\u{07}.sql",
            "wallet/migrations/001_\"#], malicious.sql",
            "wallet/migrations/001_bidi\u{202e}.sql",
        ] {
            assert_eq!(
                validate_migration_path(path),
                Err(INVALID_PATH_CHARACTER),
                "path should be rejected: {path:?}"
            );
        }
    }

    #[test]
    fn migrations_sharing_a_version_prefix_order_by_name() {
        let skip = "src/wallet/migrations".len();
        let a = Path::new("src/wallet/migrations/sqlite/20260810000000_derivation_counter.sql");
        let b = Path::new(
            "src/wallet/migrations/sqlite/20260810000000_drop_mint_quote_created_time.sql",
        );

        assert_eq!(compare_migration_paths(a, b, skip), Ordering::Less);
        assert_eq!(compare_migration_paths(b, a, skip), Ordering::Greater);
    }

    #[test]
    fn migrations_order_by_backend_then_numeric_version() {
        let skip = "src/wallet/migrations".len();
        let postgres = Path::new("src/wallet/migrations/postgres/99999999999999_last.sql");
        let sqlite = Path::new("src/wallet/migrations/sqlite/00000000000001_first.sql");
        assert_eq!(
            compare_migration_paths(postgres, sqlite, skip),
            Ordering::Less
        );

        let early = Path::new("src/wallet/migrations/sqlite/2_second.sql");
        let late = Path::new("src/wallet/migrations/sqlite/10_tenth.sql");
        assert_eq!(compare_migration_paths(early, late, skip), Ordering::Less);
    }

    #[test]
    fn source_literals_are_escaped_defensively() {
        assert_eq!(rust_string_literal("a\"b\n.sql"), "\"a\\\"b\\n.sql\"");
    }
}
