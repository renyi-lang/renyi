//! `major.minor.patch` (decisions G1 and AC1): a requirement names a version
//! and means the same major and at least that version.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl Version {
    pub fn parse(text: &str) -> Result<Version, String> {
        let wrong = || format!("`{text}` is not a version; write `major.minor.patch`");
        let parts: Vec<&str> = text.split('.').collect();
        if parts.len() != 3 {
            return Err(wrong());
        }
        let number = |part: &str| -> Result<u64, String> {
            let digits = !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
            if !digits || (part.len() > 1 && part.starts_with('0')) {
                return Err(wrong());
            }
            part.parse::<u64>().map_err(|_| wrong())
        };
        Ok(Version {
            major: number(parts[0])?,
            minor: number(parts[1])?,
            patch: number(parts[2])?,
        })
    }

    /// Whether this version satisfies a requirement: the same major, and
    /// at least the version required.
    pub fn satisfies(self, requirement: Version) -> bool {
        self.major == requirement.major && self >= requirement
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_parse_order_and_satisfy() {
        let v = |text: &str| Version::parse(text).expect("a version");
        assert_eq!(v("1.2.0").to_string(), "1.2.0");
        assert!(v("1.2.0") < v("1.10.0"));
        assert!(v("1.10.0") < v("2.0.0"));
        assert!(v("1.3.1").satisfies(v("1.2.0")));
        assert!(v("1.2.0").satisfies(v("1.2.0")));
        assert!(!v("1.1.9").satisfies(v("1.2.0")));
        assert!(!v("2.0.0").satisfies(v("1.2.0")));
        for wrong in ["1.2", "1.2.3.4", "01.2.3", "1.a.0", "", "1..0"] {
            assert!(Version::parse(wrong).is_err(), "{wrong}");
        }
    }
}
