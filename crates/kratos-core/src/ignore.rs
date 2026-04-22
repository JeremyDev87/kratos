use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IgnoreMatcher {
    rules: Vec<IgnoreRule>,
}

impl IgnoreMatcher {
    pub fn new(ignored_directories: &[String], ignore_patterns: &[String]) -> Self {
        let mut rules = Vec::new();

        for directory in ignored_directories {
            if let Some(rule) = IgnoreRule::from_directory_name(directory) {
                rules.push(rule);
            }
        }

        for raw_pattern in ignore_patterns {
            if let Some(rule) = IgnoreRule::from_raw(raw_pattern) {
                rules.push(rule);
            }
        }

        Self { rules }
    }

    pub fn is_ignored(&self, relative_path: &str, is_dir: bool) -> bool {
        let normalized = normalize_relative_path(relative_path);
        let mut ignored = false;

        for rule in &self.rules {
            if rule.matches(&normalized, is_dir) {
                ignored = !rule.negated;
            }
        }

        ignored
    }

    pub fn should_traverse_dir(&self, relative_dir: &str) -> bool {
        let normalized = normalize_relative_path(relative_dir);
        if !self.is_ignored(&normalized, true) {
            return true;
        }

        self.rules
            .iter()
            .any(|rule| rule.negated && rule.may_match_descendant(&normalized))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IgnoreRule {
    pattern: String,
    negated: bool,
    directory_only: bool,
    anchored: bool,
    has_slash: bool,
    literal_prefix: String,
}

impl IgnoreRule {
    fn from_directory_name(name: &str) -> Option<Self> {
        let pattern = normalize_pattern(name);
        if pattern.is_empty() {
            return None;
        }

        Some(Self {
            literal_prefix: pattern.clone(),
            pattern,
            negated: false,
            directory_only: true,
            anchored: false,
            has_slash: false,
        })
    }

    fn from_raw(raw: &str) -> Option<Self> {
        let mut pattern = raw.trim();
        if pattern.is_empty() {
            return None;
        }

        let negated = pattern.starts_with('!');
        if negated {
            pattern = pattern[1..].trim();
            if pattern.is_empty() {
                return None;
            }
        }

        let anchored = pattern.starts_with('/');
        if anchored {
            pattern = pattern.trim_start_matches('/');
        }

        let directory_only = pattern.ends_with('/');
        if directory_only {
            pattern = pattern.trim_end_matches('/');
        }

        let normalized = normalize_pattern(pattern);
        if normalized.is_empty() {
            return None;
        }

        let has_slash = normalized.contains('/');
        let literal_prefix = trim_slashes(&extract_literal_prefix(&normalized)).to_string();

        Some(Self {
            pattern: normalized,
            negated,
            directory_only,
            anchored,
            has_slash,
            literal_prefix,
        })
    }

    fn matches(&self, path: &str, is_dir: bool) -> bool {
        if self.directory_only {
            if is_dir {
                return self.matches_path(path);
            }

            for ancestor in ancestor_directories(path) {
                if self.matches_path(ancestor) {
                    return true;
                }
            }

            return false;
        }

        self.matches_path(path)
    }

    fn matches_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if self.anchored {
            return glob_match(&self.pattern, path);
        }

        if self.has_slash {
            for candidate in path_suffixes(path) {
                if glob_match(&self.pattern, candidate) {
                    return true;
                }
            }

            return false;
        }

        path.split('/').any(|segment| glob_match(&self.pattern, segment))
    }

    fn may_match_descendant(&self, relative_dir: &str) -> bool {
        if relative_dir.is_empty() {
            return true;
        }

        if self.matches(relative_dir, true) {
            return true;
        }

        if !self.anchored || !self.has_slash || self.literal_prefix.is_empty() {
            return true;
        }

        let prefix = self.literal_prefix.as_str();
        let dir = trim_slashes(relative_dir);
        let dir_prefix = format!("{dir}/");
        let prefix_with_slash = format!("{prefix}/");

        prefix.starts_with(&dir_prefix) || dir.starts_with(&prefix_with_slash)
    }
}

fn ancestor_directories(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/').map(|(index, _)| &path[..index])
}

fn path_suffixes(path: &str) -> impl Iterator<Item = &str> {
    std::iter::once(path).chain(path.match_indices('/').map(|(index, _)| &path[index + 1..]))
}

fn normalize_pattern(value: &str) -> String {
    trim_slashes(&value.replace('\\', "/")).to_string()
}

fn normalize_relative_path(value: &str) -> String {
    trim_slashes(&value.replace('\\', "/")).to_string()
}

fn trim_slashes(value: &str) -> &str {
    value.trim_matches('/')
}

fn extract_literal_prefix(pattern: &str) -> String {
    let mut prefix = String::new();

    for character in pattern.chars() {
        if matches!(character, '*' | '?') {
            break;
        }

        prefix.push(character);
    }

    prefix
}

fn glob_match(pattern: &str, candidate: &str) -> bool {
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    let candidate_chars = candidate.chars().collect::<Vec<_>>();
    let mut memo = HashMap::new();

    glob_match_recursive(&pattern_chars, &candidate_chars, 0, 0, &mut memo)
}

fn glob_match_recursive(
    pattern: &[char],
    candidate: &[char],
    pattern_index: usize,
    candidate_index: usize,
    memo: &mut HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(result) = memo.get(&(pattern_index, candidate_index)) {
        return *result;
    }

    let result = if pattern_index == pattern.len() {
        candidate_index == candidate.len()
    } else {
        match pattern[pattern_index] {
            '*' => {
                let mut next_index = pattern_index + 1;
                let is_double_star =
                    next_index < pattern.len() && pattern.get(next_index) == Some(&'*');

                if is_double_star {
                    while next_index < pattern.len() && pattern[next_index] == '*' {
                        next_index += 1;
                    }

                    if next_index == pattern.len() {
                        true
                    } else if pattern.get(next_index) == Some(&'/') {
                        glob_match_recursive(pattern, candidate, next_index + 1, candidate_index, memo)
                            || candidate
                                .iter()
                                .enumerate()
                                .skip(candidate_index)
                                .filter_map(|(index, character)| (*character == '/').then_some(index))
                                .any(|slash_index| {
                                    glob_match_recursive(
                                        pattern,
                                        candidate,
                                        next_index + 1,
                                        slash_index + 1,
                                        memo,
                                    )
                                })
                    } else {
                        (candidate_index..=candidate.len()).any(|next_candidate| {
                            glob_match_recursive(
                                pattern,
                                candidate,
                                next_index,
                                next_candidate,
                                memo,
                            )
                        })
                    }
                } else {
                    let mut next_candidates = vec![candidate_index];
                    let mut index = candidate_index;

                    while index < candidate.len() && candidate[index] != '/' {
                        index += 1;
                        next_candidates.push(index);
                    }

                    next_candidates.into_iter().any(|next_candidate| {
                        glob_match_recursive(
                            pattern,
                            candidate,
                            next_index,
                            next_candidate,
                            memo,
                        )
                    })
                }
            }
            '?' => {
                candidate.get(candidate_index).is_some_and(|character| *character != '/')
                    && glob_match_recursive(
                        pattern,
                        candidate,
                        pattern_index + 1,
                        candidate_index + 1,
                        memo,
                    )
            }
            literal => {
                candidate.get(candidate_index) == Some(&literal)
                    && glob_match_recursive(
                        pattern,
                        candidate,
                        pattern_index + 1,
                        candidate_index + 1,
                        memo,
                    )
            }
        }
    };

    memo.insert((pattern_index, candidate_index), result);
    result
}

#[cfg(test)]
mod tests {
    use super::IgnoreMatcher;

    #[test]
    fn directory_name_rules_still_ignore_nested_directories() {
        let matcher = IgnoreMatcher::new(&["dist".to_string()], &[]);

        assert!(matcher.is_ignored("src/dist", true));
        assert!(matcher.is_ignored("src/dist/output.ts", false));
        assert!(!matcher.is_ignored("src/distribute.ts", false));
    }

    #[test]
    fn negated_patterns_can_reopen_default_ignored_directories() {
        let matcher = IgnoreMatcher::new(
            &["node_modules".to_string()],
            &["!node_modules/@demo/keep.ts".to_string()],
        );

        assert!(matcher.should_traverse_dir("node_modules"));
        assert!(matcher.should_traverse_dir("node_modules/@demo"));
        assert!(!matcher.is_ignored("node_modules/@demo/keep.ts", false));
        assert!(matcher.is_ignored("node_modules/@demo/drop.ts", false));
    }

    #[test]
    fn anchored_patterns_match_project_relative_paths() {
        let matcher = IgnoreMatcher::new(
            &[],
            &[
                "/src/generated/**".to_string(),
                "!/src/generated/keep.ts".to_string(),
            ],
        );

        assert!(matcher.is_ignored("src/generated/drop.ts", false));
        assert!(!matcher.is_ignored("src/generated/keep.ts", false));
        assert!(!matcher.is_ignored("generated/drop.ts", false));
    }

    #[test]
    fn double_star_prefix_patterns_match_root_and_nested_files() {
        let matcher = IgnoreMatcher::new(&[], &["**/*.ts".to_string()]);

        assert!(matcher.is_ignored("foo.ts", false));
        assert!(matcher.is_ignored("src/nested.ts", false));
        assert!(!matcher.is_ignored("foo.js", false));
    }

    #[test]
    fn anchored_plain_patterns_only_match_project_root_entries() {
        let matcher = IgnoreMatcher::new(
            &[],
            &["/dist".to_string(), "/keep.ts".to_string(), "!/keep.ts".to_string()],
        );

        assert!(matcher.is_ignored("dist", true));
        assert!(!matcher.is_ignored("src/dist", true));
        assert!(!matcher.is_ignored("nested/keep.ts", false));
        assert!(!matcher.is_ignored("keep.ts", false));
    }
}
