use std::path::Path;

use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct IgnoreFilter {
    patterns: Vec<(glob::Pattern, bool)>,
}

impl IgnoreFilter {
    pub const ENV_NAME: &str = "H2KV_IGNORE";
    pub const ENV_DESCRIPTION: &str = r#"
    Used with --sync-dir option to filter which files are synchronized.
    Format:
    String of glob patterns separated by spaces or newline characters.
    Comments allowed between '#' and end of line.
    Patterns starting with '!' are treated as exceptions (whitelist).
    Pattern syntax: https://docs.rs/glob/latest/glob/struct.Pattern.html
    NOTE: Syntax is similar to .gitignore but not identical.
    Example: "**/* !/*.html !/static/**/*"
    "#;

    pub fn try_from_env() -> Result<Self> {
        match std::env::var(Self::ENV_NAME) {
            Ok(globs) => Self::try_from_str(&globs),
            Err(std::env::VarError::NotPresent) => Ok(Self { patterns: vec![] }),
            Err(e) => Err(anyhow!(
                "unparsed environment variable {}: {e}",
                Self::ENV_NAME
            )),
        }
    }

    pub fn try_from_str(globs: &str) -> Result<Self> {
        let mut globs = extract_globs(globs);
        globs.sort_by(|a, b| a.trim_start_matches('!').cmp(b.trim_start_matches('!')));
        globs.reverse();

        let mut patterns = vec![];
        for glob in globs {
            let pattern = match glob.strip_prefix('!') {
                None => (glob::Pattern::new(glob)?, false),
                Some(glob) => (glob::Pattern::new(glob)?, true),
            };
            patterns.push(pattern);
        }
        Ok(Self { patterns })
    }

    pub fn matches<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        debug_assert!(path.is_absolute());
        if !self.is_active() {
            return false;
        }

        let options = glob::MatchOptions {
            case_sensitive: true,
            require_literal_separator: true,
            require_literal_leading_dot: false,
        };

        for (pattern, inverted) in self.patterns.iter() {
            if pattern.matches_path_with(path, options) {
                return !inverted;
            }
        }
        false
    }

    pub fn is_active(&self) -> bool {
        !self.patterns.is_empty()
    }
}

impl std::fmt::Display for IgnoreFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[ ")?;
        for (pat, inv) in &self.patterns {
            write!(f, "\"{}{pat}\" ", if *inv { "!" } else { "" })?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

// split lines, remove comments and whitespace
fn extract_globs(input: &str) -> Vec<&str> {
    let mut lines = vec![];
    for line in input.lines() {
        lines.append(&mut line.split("\\n").collect());
    }
    lines = lines
        .into_iter()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with('#') {
                None
            } else {
                let line = line.split('#').next().unwrap().trim();
                Some(line)
            }
        })
        .collect::<Vec<_>>();

    let mut output = vec![];
    for line in lines {
        output.append(&mut line.split(' ').filter(|s| !s.is_empty()).collect());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches() {
        let filter = IgnoreFilter::try_from_str("**/* !/*.html !/assets/*").unwrap();
        assert!(filter.matches("/index.js"));
        assert!(filter.matches("/target/index.html"));
        assert!(!filter.matches("/index.html"));
        assert!(!filter.matches("/assets/index.css"));
    }

    #[test]
    fn test_extract_globs() {
        let input = r#"
            # c1
            one
            # c2
            two  three # c3
            four
        "#;
        assert_eq!(extract_globs(input), vec!["one", "two", "three", "four"]);
    }
}
