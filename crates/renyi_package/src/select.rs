//! The choice of versions (decision AC1): a requirement names a version and
//! means the same major and at least that version; a program holds one
//! version of a package, the highest one every requirement allows, and
//! requirements that disagree on the major are an error. The chosen
//! packages' own dependencies are requirements too, so the choice is
//! repeated until it is stable.

use std::collections::BTreeMap;

use crate::version::Version;

/// One party's requirement of a package, for the messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub name: String,
    pub version: Version,
    /// Who requires it: `renyi.json`, or `<package> <version>`.
    pub by: String,
}

/// The registry's versions of a package, ascending; none for a package it
/// does not have.
pub type Available<'a> = dyn FnMut(&str) -> Result<Vec<Version>, String> + 'a;

/// What one version of a package requires.
pub type DependenciesOf<'a> =
    dyn FnMut(&str, Version) -> Result<Vec<(String, Version)>, String> + 'a;

/// One version per package, in name order. Either reader may fail with a
/// message, which ends the choice.
pub fn select(
    requirements: &[Requirement],
    available: &mut Available<'_>,
    dependencies_of: &mut DependenciesOf<'_>,
) -> Result<Vec<(String, Version)>, String> {
    let mut chosen: BTreeMap<String, Version> = BTreeMap::new();
    // the set of names only grows (a version never changes once chosen,
    // since the choice within a major is the highest the registry has), so
    // the rounds end; the bound guards a registry that lies
    for _ in 0..64 {
        let mut all: Vec<Requirement> = requirements.to_vec();
        for (name, version) in &chosen {
            for (required, at_least) in dependencies_of(name, *version)? {
                all.push(Requirement {
                    name: required,
                    version: at_least,
                    by: format!("{name} {version}"),
                });
            }
        }
        let mut names: Vec<&str> = all.iter().map(|r| r.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        let mut next: BTreeMap<String, Version> = BTreeMap::new();
        for name in names {
            let wanted: Vec<&Requirement> = all.iter().filter(|r| r.name == name).collect();
            let first = wanted[0];
            if let Some(other) = wanted
                .iter()
                .find(|r| r.version.major != first.version.major)
            {
                return Err(format!(
                    "`{name}` is required at {} by {} and at {} by {}, which disagree on the major version",
                    first.version, first.by, other.version, other.by
                ));
            }
            let floor = wanted
                .iter()
                .map(|r| r.version)
                .max()
                .expect("at least one requirement");
            let versions = available(name)?;
            let Some(best) = versions
                .iter()
                .copied()
                .filter(|version| version.satisfies(floor))
                .max()
            else {
                let have = if versions.is_empty() {
                    "no version of it".to_string()
                } else {
                    versions
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                return Err(format!(
                    "no version of `{name}` satisfies {floor} (the same major, at least that version): the registry has {have}"
                ));
            };
            next.insert(name.to_string(), best);
        }
        if next == chosen {
            return Ok(chosen.into_iter().collect());
        }
        chosen = next;
    }
    Err("the versions do not settle: a package's dependencies keep changing the choice".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).expect("a version")
    }

    fn requirement(name: &str, version: &str) -> Requirement {
        Requirement {
            name: name.to_string(),
            version: v(version),
            by: "renyi.json".to_string(),
        }
    }

    /// A registry: `a` 1.0.0 and 1.2.0 (1.2.0 needs `b` 1.1.0); `b` 1.0.0,
    /// 1.1.0 and 2.0.0; `c` 1.0.0 (needs `b` 1.0.0).
    fn available(name: &str) -> Result<Vec<Version>, String> {
        Ok(match name {
            "a" => vec![v("1.0.0"), v("1.2.0")],
            "b" => vec![v("1.0.0"), v("1.1.0"), v("2.0.0")],
            "c" => vec![v("1.0.0")],
            "broken" => return Err("the registry cannot be read".to_string()),
            _ => Vec::new(),
        })
    }

    fn dependencies(name: &str, version: Version) -> Result<Vec<(String, Version)>, String> {
        Ok(match (name, version.to_string().as_str()) {
            ("a", "1.2.0") => vec![("b".to_string(), v("1.1.0"))],
            ("c", "1.0.0") => vec![("b".to_string(), v("1.0.0"))],
            _ => Vec::new(),
        })
    }

    fn choose(requirements: &[Requirement]) -> Result<Vec<(String, Version)>, String> {
        select(requirements, &mut available, &mut dependencies)
    }

    fn chosen(pairs: &[(&str, &str)]) -> Vec<(String, Version)> {
        pairs
            .iter()
            .map(|(name, version)| (name.to_string(), v(version)))
            .collect()
    }

    #[test]
    fn the_highest_allowed_version_is_chosen_with_its_dependencies() {
        assert_eq!(
            choose(&[requirement("a", "1.0.0")]).expect("a choice"),
            chosen(&[("a", "1.2.0"), ("b", "1.1.0")])
        );
        // a dependency's requirement and the manifest's agree on the major
        assert_eq!(
            choose(&[requirement("c", "1.0.0"), requirement("b", "1.1.0")]).expect("a choice"),
            chosen(&[("b", "1.1.0"), ("c", "1.0.0")])
        );
        assert_eq!(
            choose(&[requirement("b", "2.0.0")]).expect("a choice"),
            chosen(&[("b", "2.0.0")])
        );
        assert_eq!(choose(&[]).expect("a choice"), Vec::new());
    }

    #[test]
    fn requirements_that_disagree_on_the_major_are_refused() {
        // the manifest wants b 2.x, a 1.2.0 wants b 1.1.0
        let error =
            choose(&[requirement("a", "1.0.0"), requirement("b", "2.0.0")]).expect_err("refused");
        assert_eq!(
            error,
            "`b` is required at 2.0.0 by renyi.json and at 1.1.0 by a 1.2.0, which disagree on the major version"
        );
        let error = choose(&[requirement("a", "1.3.0")]).expect_err("refused");
        assert_eq!(
            error,
            "no version of `a` satisfies 1.3.0 (the same major, at least that version): the registry has 1.0.0, 1.2.0"
        );
        let error = choose(&[requirement("d", "1.0.0")]).expect_err("refused");
        assert!(
            error.ends_with("the registry has no version of it"),
            "{error}"
        );
        let error = choose(&[requirement("broken", "1.0.0")]).expect_err("refused");
        assert_eq!(error, "the registry cannot be read");
    }
}
