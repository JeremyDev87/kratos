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
        self.is_ignored_from_root(relative_path, is_dir, None)
    }

    pub fn is_ignored_from_root(
        &self,
        relative_path: &str,
        is_dir: bool,
        traversal_root: Option<&str>,
    ) -> bool {
        let normalized = normalize_relative_path(relative_path);
        let normalized_root = traversal_root.map(normalize_relative_path);
        let mut ignored = false;

        for rule in &self.rules {
            if rule.matches(&normalized, is_dir, normalized_root.as_deref()) {
                ignored = !rule.negated;
            }
        }

        ignored
    }

    pub fn should_traverse_dir(&self, relative_dir: &str) -> bool {
        self.should_traverse_dir_from_root(relative_dir, None)
    }

    pub fn should_traverse_dir_from_root(
        &self,
        relative_dir: &str,
        traversal_root: Option<&str>,
    ) -> bool {
        let normalized = normalize_relative_path(relative_dir);
        let ignored_by_directory_name_rule =
            self.is_ignored_by_directory_name_rule(&normalized, traversal_root);
        if !self.is_ignored_from_root(&normalized, true, traversal_root)
            && !self.ignores_descendants(&normalized, traversal_root)
        {
            return true;
        }

        self.rules.iter().any(|rule| {
            if !rule.negated || !rule.may_match_descendant(&normalized) {
                return false;
            }
            if ignored_by_directory_name_rule && !rule.has_explicit_path_prefix() {
                return false;
            }

            let Some((sample_path, is_dir)) = rule.sample_path_at_or_below(&normalized) else {
                return false;
            };

            !self.is_ignored_from_root(&sample_path, is_dir, traversal_root)
        })
    }

    fn ignores_descendants(&self, relative_dir: &str, traversal_root: Option<&str>) -> bool {
        let probe = descendant_probe_path(relative_dir);
        !probe.is_empty() && self.is_ignored_from_root(&probe, false, traversal_root)
    }

    fn is_ignored_by_directory_name_rule(
        &self,
        relative_dir: &str,
        traversal_root: Option<&str>,
    ) -> bool {
        let normalized_root = traversal_root.map(normalize_relative_path);
        self.rules.iter().any(|rule| {
            !rule.negated
                && rule.directory_only
                && rule.scoped_to_traversal_root
                && rule.matches(relative_dir, true, normalized_root.as_deref())
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IgnoreRule {
    pattern: String,
    negated: bool,
    directory_only: bool,
    scoped_to_traversal_root: bool,
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
            scoped_to_traversal_root: true,
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
            scoped_to_traversal_root: false,
            anchored,
            has_slash,
            literal_prefix,
        })
    }

    fn matches(&self, path: &str, is_dir: bool, traversal_root: Option<&str>) -> bool {
        let scoped_path = self.scoped_match_path(path, traversal_root);
        if scoped_path.is_empty() {
            return false;
        }

        if self.directory_only {
            if is_dir {
                return self.matches_path(scoped_path);
            }

            if self.negated {
                return false;
            }

            for ancestor in ancestor_directories(scoped_path) {
                if self.matches_path(ancestor) {
                    return true;
                }
            }

            return false;
        }

        if !is_dir && !self.negated {
            for ancestor in ancestor_directories(scoped_path) {
                if self.matches_path(ancestor) {
                    return true;
                }
            }
        }

        self.matches_path(scoped_path)
    }

    fn scoped_match_path<'a>(&self, path: &'a str, traversal_root: Option<&str>) -> &'a str {
        if !self.directory_only || !self.scoped_to_traversal_root {
            return path;
        }

        strip_traversal_root(path, traversal_root)
    }

    fn matches_path(&self, path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        if self.anchored || self.has_slash {
            return glob_match(&self.pattern, path);
        }

        path.split('/').any(|segment| glob_match(&self.pattern, segment))
    }

    fn may_match_descendant(&self, relative_dir: &str) -> bool {
        if relative_dir.is_empty() {
            return true;
        }

        if self.matches(relative_dir, true, None) {
            return true;
        }

        if !self.has_slash || self.literal_prefix.is_empty() {
            return true;
        }

        let prefix = self.literal_prefix.as_str();
        let dir = trim_slashes(relative_dir);
        let dir_prefix = format!("{dir}/");
        let prefix_with_slash = format!("{prefix}/");

        prefix == dir || prefix.starts_with(&dir_prefix) || dir.starts_with(&prefix_with_slash)
    }

    fn has_explicit_path_prefix(&self) -> bool {
        self.has_slash && !self.literal_prefix.is_empty()
    }

    fn sample_path_at_or_below(&self, relative_dir: &str) -> Option<(String, bool)> {
        let normalized_dir = normalize_relative_path(relative_dir);

        if self.directory_only
            && !normalized_dir.is_empty()
            && self.matches(&normalized_dir, true, None)
        {
            return Some((normalized_dir, true));
        }

        if self.anchored || self.has_slash {
            return self.sample_slash_pattern_path_at_or_below(&normalized_dir);
        }

        let candidate = join_relative_path(&normalized_dir, &materialize_segment_pattern(&self.pattern));
        let is_dir = self.directory_only;

        self.matches(&candidate, is_dir, None)
            .then_some((candidate, is_dir))
    }

    fn sample_slash_pattern_path_at_or_below(&self, relative_dir: &str) -> Option<(String, bool)> {
        let pattern_segments = split_path_segments(&self.pattern);
        let dir_segments = split_path_segments(relative_dir);
        let mut suffix =
            synthesize_suffix_after_dir(&pattern_segments, &dir_segments, !self.directory_only)?;
        let mut candidate_segments = dir_segments
            .iter()
            .map(|segment| (*segment).to_string())
            .collect::<Vec<_>>();
        candidate_segments.append(&mut suffix);

        let candidate = candidate_segments.join("/");
        if candidate.is_empty() {
            return None;
        }

        let is_dir = self.directory_only;
        self.matches(&candidate, is_dir, None)
            .then_some((candidate, is_dir))
    }
}

fn ancestor_directories(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/').map(|(index, _)| &path[..index])
}

fn split_path_segments(path: &str) -> Vec<&str> {
    if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').collect()
    }
}

fn materialize_segment_pattern(pattern: &str) -> String {
    let mut materialized = String::new();

    for character in pattern.chars() {
        match character {
            '*' => materialized.push_str("sample"),
            '?' => materialized.push('x'),
            _ => materialized.push(character),
        }
    }

    if materialized.is_empty() {
        "sample".to_string()
    } else {
        materialized
    }
}

fn join_relative_path(base: &str, suffix: &str) -> String {
    match (trim_slashes(base), trim_slashes(suffix)) {
        ("", "") => String::new(),
        ("", suffix) => suffix.to_string(),
        (base, "") => base.to_string(),
        (base, suffix) => format!("{base}/{suffix}"),
    }
}

fn synthesize_suffix_after_dir(
    pattern_segments: &[&str],
    dir_segments: &[&str],
    need_descendant: bool,
) -> Option<Vec<String>> {
    synthesize_suffix_after_dir_from(
        pattern_segments,
        dir_segments,
        0,
        0,
        need_descendant,
    )
}

fn synthesize_suffix_after_dir_from(
    pattern_segments: &[&str],
    dir_segments: &[&str],
    pattern_index: usize,
    dir_index: usize,
    need_descendant: bool,
) -> Option<Vec<String>> {
    if dir_index == dir_segments.len() {
        return synthesize_pattern_suffix(pattern_segments, pattern_index, need_descendant);
    }

    let segment = *pattern_segments.get(pattern_index)?;
    if segment == "**" {
        if let Some(suffix) = synthesize_suffix_after_dir_from(
            pattern_segments,
            dir_segments,
            pattern_index + 1,
            dir_index,
            need_descendant,
        ) {
            return Some(suffix);
        }

        return synthesize_suffix_after_dir_from(
            pattern_segments,
            dir_segments,
            pattern_index,
            dir_index + 1,
            need_descendant,
        );
    }

    if !glob_match(segment, dir_segments[dir_index]) {
        return None;
    }

    synthesize_suffix_after_dir_from(
        pattern_segments,
        dir_segments,
        pattern_index + 1,
        dir_index + 1,
        need_descendant,
    )
}

fn synthesize_pattern_suffix(
    pattern_segments: &[&str],
    pattern_index: usize,
    need_descendant: bool,
) -> Option<Vec<String>> {
    let Some(segment) = pattern_segments.get(pattern_index) else {
        return (!need_descendant).then(|| Vec::new());
    };

    if *segment == "**" {
        if let Some(suffix) =
            synthesize_pattern_suffix(pattern_segments, pattern_index + 1, need_descendant)
        {
            return Some(suffix);
        }

        let mut suffix = vec!["sample".to_string()];
        suffix.extend(synthesize_pattern_suffix(
            pattern_segments,
            pattern_index + 1,
            false,
        )?);
        return Some(suffix);
    }

    let mut suffix = vec![materialize_segment_pattern(segment)];
    suffix.extend(synthesize_pattern_suffix(
        pattern_segments,
        pattern_index + 1,
        false,
    )?);
    Some(suffix)
}

fn normalize_pattern(value: &str) -> String {
    trim_slashes(&value.replace('\\', "/")).to_string()
}

fn normalize_relative_path(value: &str) -> String {
    trim_slashes(&value.replace('\\', "/")).to_string()
}

fn strip_traversal_root<'a>(path: &'a str, traversal_root: Option<&str>) -> &'a str {
    let Some(root) = traversal_root else {
        return path;
    };
    let root = trim_slashes(root);

    if root.is_empty() || path.is_empty() {
        return path;
    }

    if path == root {
        return "";
    }

    path.strip_prefix(root)
        .and_then(|suffix| suffix.strip_prefix('/'))
        .unwrap_or(path)
}

fn descendant_probe_path(relative_dir: &str) -> String {
    if relative_dir.is_empty() {
        "__kratos_probe__".to_string()
    } else {
        format!("{relative_dir}/__kratos_probe__")
    }
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

    #[test]
    fn descendant_only_patterns_can_prune_nested_directories() {
        let matcher = IgnoreMatcher::new(&[], &["src/generated/**".to_string()]);

        assert!(!matcher.is_ignored("src/generated", true));
        assert!(matcher.should_traverse_dir("src"));
        assert!(!matcher.should_traverse_dir("src/generated"));
    }

    #[test]
    fn descendant_only_patterns_keep_traversal_open_for_negated_descendants() {
        let matcher = IgnoreMatcher::new(
            &[],
            &[
                "src/generated/**".to_string(),
                "!src/generated/keep.ts".to_string(),
            ],
        );

        assert!(matcher.should_traverse_dir("src/generated"));
        assert!(!matcher.is_ignored("src/generated/keep.ts", false));
        assert!(matcher.is_ignored("src/generated/drop.ts", false));
    }

    #[test]
    fn explicit_roots_only_bypass_directory_name_ignore_rules() {
        let directory_matcher = IgnoreMatcher::new(&["tests".to_string()], &[]);
        let pattern_matcher = IgnoreMatcher::new(&[], &["tests/".to_string()]);

        assert!(!directory_matcher.is_ignored_from_root("tests/a.ts", false, Some("tests")));
        assert!(directory_matcher.is_ignored("tests/a.ts", false));
        assert!(pattern_matcher.is_ignored_from_root("tests/a.ts", false, Some("tests")));
    }

    #[test]
    fn anchored_negated_double_star_patterns_keep_anchor_directories_traversable() {
        let matcher = IgnoreMatcher::new(
            &[],
            &["src/**".to_string(), "!/src/**/keep.ts".to_string()],
        );

        assert!(matcher.should_traverse_dir("src"));
        assert!(matcher.should_traverse_dir("src/foo"));
        assert!(!matcher.is_ignored("src/foo/keep.ts", false));
        assert!(matcher.is_ignored("src/foo/drop.ts", false));
    }

    #[test]
    fn slash_patterns_stay_project_relative_without_leading_slash() {
        let matcher = IgnoreMatcher::new(&[], &["src/generated/keep.ts".to_string()]);

        assert!(matcher.is_ignored("src/generated/keep.ts", false));
        assert!(!matcher.is_ignored("node_modules/pkg/src/generated/keep.ts", false));
    }

    #[test]
    fn unrelated_negated_slash_patterns_do_not_reopen_other_ignored_trees() {
        let matcher = IgnoreMatcher::new(
            &[],
            &["node_modules/**".to_string(), "!src/generated/keep.ts".to_string()],
        );

        assert!(!matcher.should_traverse_dir("node_modules"));
        assert!(matcher.should_traverse_dir("src"));
        assert!(matcher.should_traverse_dir("src/generated"));
    }

    #[test]
    fn later_positive_patterns_override_earlier_negated_descendants_for_traversal() {
        let matcher = IgnoreMatcher::new(
            &[],
            &[
                "!src/generated/keep.ts".to_string(),
                "src/generated/**".to_string(),
            ],
        );

        assert!(!matcher.should_traverse_dir("src/generated"));
        assert!(matcher.is_ignored("src/generated/keep.ts", false));
    }

    #[test]
    fn wildcard_segment_negations_still_reopen_deeper_descendants() {
        let matcher = IgnoreMatcher::new(
            &[],
            &["src/*/drop.ts".to_string(), "!src/*/keep.ts".to_string()],
        );

        assert!(matcher.should_traverse_dir("src/foo"));
        assert!(!matcher.is_ignored("src/foo/keep.ts", false));
        assert!(matcher.is_ignored("src/foo/drop.ts", false));
    }

    #[test]
    fn broad_wildcard_negations_do_not_reopen_default_ignored_directories() {
        let matcher = IgnoreMatcher::new(
            &["node_modules".to_string()],
            &["!**/*.ts".to_string(), "!*.tsx".to_string()],
        );

        assert!(!matcher.should_traverse_dir("node_modules"));
        assert!(!matcher.should_traverse_dir("node_modules/@demo"));
    }

    #[test]
    fn negated_directory_only_patterns_reopen_traversal_without_reopening_all_descendants() {
        let matcher = IgnoreMatcher::new(
            &[],
            &[
                "src/generated/**".to_string(),
                "!src/generated/".to_string(),
                "!src/generated/keep.ts".to_string(),
            ],
        );

        assert!(matcher.should_traverse_dir("src/generated"));
        assert!(!matcher.is_ignored("src/generated/keep.ts", false));
        assert!(matcher.is_ignored("src/generated/drop.ts", false));
    }

    #[test]
    fn basename_negations_can_reopen_raw_ignored_descendants() {
        let matcher = IgnoreMatcher::new(
            &[],
            &["src/generated/**".to_string(), "!keep.ts".to_string()],
        );

        assert!(matcher.should_traverse_dir("src/generated"));
        assert!(!matcher.is_ignored("src/generated/keep.ts", false));
        assert!(matcher.is_ignored("src/generated/drop.ts", false));
    }

    #[test]
    fn slash_patterns_that_match_directories_still_ignore_descendant_files() {
        let matcher = IgnoreMatcher::new(
            &[],
            &[
                "src/generated".to_string(),
                "!src/generated/keep.ts".to_string(),
            ],
        );

        assert!(matcher.should_traverse_dir("src/generated"));
        assert!(!matcher.is_ignored("src/generated/keep.ts", false));
        assert!(matcher.is_ignored("src/generated/drop.ts", false));
    }
}
