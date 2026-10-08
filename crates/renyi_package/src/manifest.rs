//! The project manifest `renyi.json`, the lockfile `renyi.lock.json`, the
//! registry's `package.json` and `versions.json` (decision AC1), read
//! strictly (a field the format does not have is an error, so that every
//! toolchain refuses the same files) and rendered the same way every time
//! (two-space indentation, the keys in one order), so that a package's
//! content hash is the hash of its `package.json` text.

use renyi_json::{read_json, write_json, Json};
use renyi_syntax::{ForeignModule, PythonModule};

use crate::version::Version;

pub const MANIFEST_FILE: &str = "renyi.json";
pub const LOCK_FILE: &str = "renyi.lock.json";
pub const PACKAGE_FILE: &str = "package.json";
pub const VERSIONS_FILE: &str = "versions.json";

/// The thresholds of `renyi index --budgets` (decision R7), each optional.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Budgets {
    pub public_per_module: Option<usize>,
    pub effect_paths_per_module: Option<usize>,
    pub fan_out_per_definition: Option<usize>,
}

/// `renyi.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub version: Version,
    pub purpose: Option<String>,
    /// Package name to the version required, sorted by name.
    pub dependencies: Vec<(String, Version)>,
    /// A directory (absolute, or relative to the project root) or an
    /// `http://` or `https://` base.
    pub registry: Option<String>,
    pub budgets: Option<Budgets>,
    /// The foreign modules of the project (decision AF1), by module name.
    pub foreign: Vec<(String, ForeignModule)>,
    /// The Python modules of the project and their interpreter (decision
    /// AL1).
    pub python: PythonSection,
}

/// The `python` section of `renyi.json` (decision AL1): the interpreter
/// the project names, if any, and its Python modules by module name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PythonSection {
    pub interpreter: Option<String>,
    pub modules: Vec<(String, PythonModule)>,
}

/// One entry of the lockfile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Locked {
    pub name: String,
    pub version: Version,
    /// `sha256:` and the hex digest of the package's `package.json`.
    pub hash: String,
}

/// `renyi.lock.json`: every package the program reaches, sorted by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lock {
    pub packages: Vec<Locked>,
}

/// One public function's transitive effects and failure types.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    /// `module.function`, the module named from the package's root.
    pub function: String,
    pub needs: Vec<String>,
    pub fails: Vec<String>,
}

/// The registry's `package.json` of one version of a package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFile {
    pub name: String,
    pub version: Version,
    pub purpose: Option<String>,
    pub dependencies: Vec<(String, Version)>,
    /// `renyi 0.0.1`, the toolchain that published it.
    pub toolchain: String,
    /// Every source file's path from the package's root, with its
    /// `sha256:` hash, sorted by path.
    pub files: Vec<(String, String)>,
    /// Sorted by function.
    pub effects: Vec<Effect>,
}

/// The registry's `versions.json` of a package, ascending.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Versions {
    pub versions: Vec<Version>,
}

/// A package name: a module name segment, lower case.
pub fn is_package_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

// ----------------------------------------------------------------- reading

type Fields = Vec<(String, Json)>;

fn document(text: &str, what: &str) -> Result<Fields, String> {
    let json = read_json(text)
        .map_err(|(detail, line)| format!("{what} is not JSON: {detail} (line {line})"))?;
    match json {
        Json::Object(fields) => Ok(fields),
        _ => Err(format!("{what} is not a JSON object")),
    }
}

/// Every key is one the format has, and none is repeated.
fn only(fields: &Fields, allowed: &[&str], what: &str) -> Result<(), String> {
    for (index, (key, _)) in fields.iter().enumerate() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("`{key}` is not a field of {what}"));
        }
        if fields[..index].iter().any(|(other, _)| other == key) {
            return Err(format!("`{key}` is given twice in {what}"));
        }
    }
    Ok(())
}

fn field<'a>(fields: &'a Fields, name: &str) -> Option<&'a Json> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn text(fields: &Fields, name: &str, what: &str) -> Result<String, String> {
    match field(fields, name) {
        Some(Json::Text(value)) => Ok(value.clone()),
        Some(_) => Err(format!("`{name}` of {what} is not a string")),
        None => Err(format!("{what} has no `{name}`")),
    }
}

fn optional_text(fields: &Fields, name: &str, what: &str) -> Result<Option<String>, String> {
    match field(fields, name) {
        Some(Json::Text(value)) => Ok(Some(value.clone())),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(format!("`{name}` of {what} is not a string")),
    }
}

fn optional_number(fields: &Fields, name: &str, what: &str) -> Result<Option<usize>, String> {
    match field(fields, name) {
        Some(Json::Number(digits)) => digits
            .parse::<usize>()
            .map(Some)
            .map_err(|_| format!("`{name}` of {what} is not a whole number")),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(format!("`{name}` of {what} is not a number")),
    }
}

fn optional_object<'a>(
    fields: &'a Fields,
    name: &str,
    what: &str,
) -> Result<Option<&'a Fields>, String> {
    match field(fields, name) {
        Some(Json::Object(inner)) => Ok(Some(inner)),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(format!("`{name}` of {what} is not an object")),
    }
}

fn array<'a>(fields: &'a Fields, name: &str, what: &str) -> Result<&'a [Json], String> {
    match field(fields, name) {
        Some(Json::Array(items)) => Ok(items),
        Some(_) => Err(format!("`{name}` of {what} is not a list")),
        None => Err(format!("{what} has no `{name}`")),
    }
}

fn texts(json: &Json, what: &str) -> Result<Vec<String>, String> {
    let Json::Array(items) = json else {
        return Err(format!("{what} is not a list"));
    };
    items
        .iter()
        .map(|item| match item {
            Json::Text(value) => Ok(value.clone()),
            _ => Err(format!("{what} holds something that is not a string")),
        })
        .collect()
}

fn version_of(fields: &Fields, name: &str, what: &str) -> Result<Version, String> {
    Version::parse(&text(fields, name, what)?).map_err(|detail| format!("{what}: {detail}"))
}

fn name_of(fields: &Fields, what: &str) -> Result<String, String> {
    let name = text(fields, "name", what)?;
    if !is_package_name(&name) {
        return Err(format!(
            "`{name}` is not a package name; write lower-case letters, digits and `_`, starting with a letter"
        ));
    }
    Ok(name)
}

/// `{"name": "1.2.0", ...}`, sorted by name.
fn dependencies_of(fields: &Fields, what: &str) -> Result<Vec<(String, Version)>, String> {
    let Some(object) = optional_object(fields, "dependencies", what)? else {
        return Ok(Vec::new());
    };
    let mut dependencies: Vec<(String, Version)> = Vec::new();
    for (name, requirement) in object {
        if !is_package_name(name) {
            return Err(format!(
                "`{name}` in the dependencies of {what} is not a package name"
            ));
        }
        if dependencies.iter().any(|(other, _)| other == name) {
            return Err(format!(
                "`{name}` is given twice in the dependencies of {what}"
            ));
        }
        let Json::Text(spelled) = requirement else {
            return Err(format!("the version of `{name}` in {what} is not a string"));
        };
        let version = Version::parse(spelled).map_err(|detail| format!("{what}: {detail}"))?;
        dependencies.push((name.clone(), version));
    }
    dependencies.sort();
    Ok(dependencies)
}

/// The `foreign` section (decision AF1): module name to the libraries the
/// module's symbols are looked up in, tried in order, and the functions
/// whose C symbol differs from their Renyi name; sorted by module name.
fn foreign_of(fields: &Fields, what: &str) -> Result<Vec<(String, ForeignModule)>, String> {
    let Some(object) = optional_object(fields, "foreign", what)? else {
        return Ok(Vec::new());
    };
    let mut modules: Vec<(String, ForeignModule)> = Vec::new();
    for (name, json) in object {
        if !name.split('.').all(is_package_name) {
            return Err(format!(
                "`{name}` in the foreign modules of {what} is not a module name"
            ));
        }
        if modules.iter().any(|(other, _)| other == name) {
            return Err(format!(
                "`{name}` is given twice in the foreign modules of {what}"
            ));
        }
        let entry_what = format!("the foreign module `{name}` of {what}");
        let Json::Object(entry) = json else {
            return Err(format!("{entry_what} is not an object"));
        };
        only(entry, &["library", "symbols"], &entry_what)?;
        let libraries = match field(entry, "library") {
            Some(json) => texts(json, &format!("`library` of {entry_what}"))?,
            None => return Err(format!("{entry_what} has no `library`")),
        };
        if libraries.is_empty() {
            return Err(format!("`library` of {entry_what} is empty"));
        }
        let symbols = symbols_of(entry, &entry_what)?;
        modules.push((name.clone(), ForeignModule { libraries, symbols }));
    }
    modules.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(modules)
}

fn foreign_json(modules: &[(String, ForeignModule)]) -> Json {
    Json::Object(
        modules
            .iter()
            .map(|(name, module)| {
                let mut fields = vec![(
                    "library".to_string(),
                    Json::Array(
                        module
                            .libraries
                            .iter()
                            .map(|library| Json::Text(library.clone()))
                            .collect(),
                    ),
                )];
                if !module.symbols.is_empty() {
                    fields.push(("symbols".to_string(), symbols_json(&module.symbols)));
                }
                (name.clone(), Json::Object(fields))
            })
            .collect(),
    )
}

/// The `symbols` object of a bound module: the Renyi name to the name on
/// the other side, sorted by the Renyi name.
fn symbols_of(entry: &Fields, what: &str) -> Result<Vec<(String, String)>, String> {
    let mut symbols: Vec<(String, String)> = Vec::new();
    if let Some(object) = optional_object(entry, "symbols", what)? {
        for (renyi, json) in object {
            let Json::Text(symbol) = json else {
                return Err(format!(
                    "`{renyi}` in the symbols of {what} is not a string"
                ));
            };
            symbols.push((renyi.clone(), symbol.clone()));
        }
        symbols.sort();
    }
    Ok(symbols)
}

fn symbols_json(symbols: &[(String, String)]) -> Json {
    Json::Object(
        symbols
            .iter()
            .map(|(renyi, symbol)| (renyi.clone(), Json::Text(symbol.clone())))
            .collect(),
    )
}

/// The `python` section (decision AL1): the interpreter, when the project
/// names one, and each Python module of the project by module name, with
/// the name the Python side imports it by (`package`, the module's own
/// name when left out) and the functions whose Python name differs from
/// their Renyi name; sorted by module name.
fn python_of(fields: &Fields, what: &str) -> Result<PythonSection, String> {
    let Some(section) = optional_object(fields, "python", what)? else {
        return Ok(PythonSection::default());
    };
    let section_what = format!("`python` of {what}");
    only(section, &["interpreter", "modules"], &section_what)?;
    let interpreter = optional_text(section, "interpreter", &section_what)?;
    let mut modules: Vec<(String, PythonModule)> = Vec::new();
    if let Some(object) = optional_object(section, "modules", &section_what)? {
        for (name, json) in object {
            if !name.split('.').all(is_package_name) {
                return Err(format!(
                    "`{name}` in the Python modules of {what} is not a module name"
                ));
            }
            if modules.iter().any(|(other, _)| other == name) {
                return Err(format!(
                    "`{name}` is given twice in the Python modules of {what}"
                ));
            }
            let entry_what = format!("the Python module `{name}` of {what}");
            let Json::Object(entry) = json else {
                return Err(format!("{entry_what} is not an object"));
            };
            only(entry, &["package", "symbols"], &entry_what)?;
            let package =
                optional_text(entry, "package", &entry_what)?.unwrap_or_else(|| name.clone());
            if package.is_empty() {
                return Err(format!("`package` of {entry_what} is empty"));
            }
            let symbols = symbols_of(entry, &entry_what)?;
            modules.push((name.clone(), PythonModule { package, symbols }));
        }
        modules.sort_by(|a, b| a.0.cmp(&b.0));
    }
    Ok(PythonSection {
        interpreter,
        modules,
    })
}

fn python_json(section: &PythonSection) -> Json {
    let mut fields = Vec::new();
    if let Some(interpreter) = &section.interpreter {
        fields.push(("interpreter".to_string(), Json::Text(interpreter.clone())));
    }
    if !section.modules.is_empty() {
        fields.push((
            "modules".to_string(),
            Json::Object(
                section
                    .modules
                    .iter()
                    .map(|(name, module)| {
                        let mut entry = Vec::new();
                        if module.package != *name {
                            entry.push(("package".to_string(), Json::Text(module.package.clone())));
                        }
                        if !module.symbols.is_empty() {
                            entry.push(("symbols".to_string(), symbols_json(&module.symbols)));
                        }
                        (name.clone(), Json::Object(entry))
                    })
                    .collect(),
            ),
        ));
    }
    Json::Object(fields)
}

impl Manifest {
    pub fn read(source: &str) -> Result<Manifest, String> {
        let what = "the manifest";
        let fields = document(source, what)?;
        only(
            &fields,
            &[
                "name",
                "version",
                "purpose",
                "dependencies",
                "registry",
                "budgets",
                "foreign",
                "python",
            ],
            what,
        )?;
        let budgets = match optional_object(&fields, "budgets", what)? {
            Some(inner) => {
                let what = "`budgets`";
                only(
                    inner,
                    &[
                        "public_per_module",
                        "effect_paths_per_module",
                        "fan_out_per_definition",
                    ],
                    what,
                )?;
                Some(Budgets {
                    public_per_module: optional_number(inner, "public_per_module", what)?,
                    effect_paths_per_module: optional_number(
                        inner,
                        "effect_paths_per_module",
                        what,
                    )?,
                    fan_out_per_definition: optional_number(inner, "fan_out_per_definition", what)?,
                })
            }
            None => None,
        };
        Ok(Manifest {
            name: name_of(&fields, what)?,
            version: version_of(&fields, "version", what)?,
            purpose: optional_text(&fields, "purpose", what)?,
            dependencies: dependencies_of(&fields, what)?,
            registry: optional_text(&fields, "registry", what)?,
            budgets,
            foreign: foreign_of(&fields, what)?,
            python: python_of(&fields, what)?,
        })
    }

    pub fn render(&self) -> String {
        let mut fields = vec![
            ("name", Json::Text(self.name.clone())),
            ("version", Json::Text(self.version.to_string())),
        ];
        if let Some(purpose) = &self.purpose {
            fields.push(("purpose", Json::Text(purpose.clone())));
        }
        fields.push(("dependencies", dependencies_json(&self.dependencies)));
        if let Some(registry) = &self.registry {
            fields.push(("registry", Json::Text(registry.clone())));
        }
        if let Some(budgets) = &self.budgets {
            let mut inner = Vec::new();
            for (name, value) in [
                ("public_per_module", budgets.public_per_module),
                ("effect_paths_per_module", budgets.effect_paths_per_module),
                ("fan_out_per_definition", budgets.fan_out_per_definition),
            ] {
                if let Some(value) = value {
                    inner.push((name.to_string(), Json::Number(value.to_string())));
                }
            }
            fields.push(("budgets", Json::Object(inner)));
        }
        if !self.foreign.is_empty() {
            fields.push(("foreign", foreign_json(&self.foreign)));
        }
        if self.python.interpreter.is_some() || !self.python.modules.is_empty() {
            fields.push(("python", python_json(&self.python)));
        }
        render(fields)
    }
}

impl Lock {
    pub fn read(source: &str) -> Result<Lock, String> {
        let what = "the lockfile";
        let fields = document(source, what)?;
        only(&fields, &["packages"], what)?;
        let mut packages = Vec::new();
        for item in array(&fields, "packages", what)? {
            let Json::Object(entry) = item else {
                return Err(format!(
                    "`packages` of {what} holds something that is not an object"
                ));
            };
            let what = "a package of the lockfile";
            only(entry, &["name", "version", "hash"], what)?;
            let name = name_of(entry, what)?;
            if packages.iter().any(|locked: &Locked| locked.name == name) {
                return Err(format!("`{name}` is locked twice"));
            }
            packages.push(Locked {
                name,
                version: version_of(entry, "version", what)?,
                hash: text(entry, "hash", what)?,
            });
        }
        packages.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Lock { packages })
    }

    pub fn render(&self) -> String {
        let packages = self
            .packages
            .iter()
            .map(|locked| {
                Json::Object(vec![
                    ("name".to_string(), Json::Text(locked.name.clone())),
                    (
                        "version".to_string(),
                        Json::Text(locked.version.to_string()),
                    ),
                    ("hash".to_string(), Json::Text(locked.hash.clone())),
                ])
            })
            .collect();
        render(vec![("packages", Json::Array(packages))])
    }

    pub fn get(&self, name: &str) -> Option<&Locked> {
        self.packages.iter().find(|locked| locked.name == name)
    }
}

impl PackageFile {
    pub fn read(source: &str) -> Result<PackageFile, String> {
        let what = "the package file";
        let fields = document(source, what)?;
        only(
            &fields,
            &[
                "name",
                "version",
                "purpose",
                "dependencies",
                "toolchain",
                "files",
                "effects",
            ],
            what,
        )?;
        let mut files = Vec::new();
        for item in array(&fields, "files", what)? {
            let Json::Object(entry) = item else {
                return Err(format!(
                    "`files` of {what} holds something that is not an object"
                ));
            };
            let what = "a file of the package file";
            only(entry, &["path", "hash"], what)?;
            files.push((text(entry, "path", what)?, text(entry, "hash", what)?));
        }
        let mut effects = Vec::new();
        for item in array(&fields, "effects", what)? {
            let Json::Object(entry) = item else {
                return Err(format!(
                    "`effects` of {what} holds something that is not an object"
                ));
            };
            let what = "an effect of the package file";
            only(entry, &["function", "needs", "fails"], what)?;
            let list = |name: &str| -> Result<Vec<String>, String> {
                match field(entry, name) {
                    Some(json) => texts(json, &format!("`{name}` of {what}")),
                    None => Err(format!("{what} has no `{name}`")),
                }
            };
            effects.push(Effect {
                function: text(entry, "function", what)?,
                needs: list("needs")?,
                fails: list("fails")?,
            });
        }
        Ok(PackageFile {
            name: name_of(&fields, what)?,
            version: version_of(&fields, "version", what)?,
            purpose: optional_text(&fields, "purpose", what)?,
            dependencies: dependencies_of(&fields, what)?,
            toolchain: text(&fields, "toolchain", what)?,
            files,
            effects,
        })
    }

    pub fn render(&self) -> String {
        let mut fields = vec![
            ("name", Json::Text(self.name.clone())),
            ("version", Json::Text(self.version.to_string())),
        ];
        if let Some(purpose) = &self.purpose {
            fields.push(("purpose", Json::Text(purpose.clone())));
        }
        fields.push(("dependencies", dependencies_json(&self.dependencies)));
        fields.push(("toolchain", Json::Text(self.toolchain.clone())));
        fields.push((
            "files",
            Json::Array(
                self.files
                    .iter()
                    .map(|(path, hash)| {
                        Json::Object(vec![
                            ("path".to_string(), Json::Text(path.clone())),
                            ("hash".to_string(), Json::Text(hash.clone())),
                        ])
                    })
                    .collect(),
            ),
        ));
        fields.push((
            "effects",
            Json::Array(
                self.effects
                    .iter()
                    .map(|effect| {
                        let list = |items: &[String]| {
                            Json::Array(items.iter().cloned().map(Json::Text).collect())
                        };
                        Json::Object(vec![
                            ("function".to_string(), Json::Text(effect.function.clone())),
                            ("needs".to_string(), list(&effect.needs)),
                            ("fails".to_string(), list(&effect.fails)),
                        ])
                    })
                    .collect(),
            ),
        ));
        render(fields)
    }
}

impl Versions {
    pub fn read(source: &str) -> Result<Versions, String> {
        let what = "the versions file";
        let fields = document(source, what)?;
        only(&fields, &["versions"], what)?;
        let mut versions = Vec::new();
        for item in array(&fields, "versions", what)? {
            let Json::Text(spelled) = item else {
                return Err(format!(
                    "`versions` of {what} holds something that is not a string"
                ));
            };
            versions.push(Version::parse(spelled).map_err(|detail| format!("{what}: {detail}"))?);
        }
        versions.sort();
        versions.dedup();
        Ok(Versions { versions })
    }

    pub fn render(&self) -> String {
        render(vec![(
            "versions",
            Json::Array(
                self.versions
                    .iter()
                    .map(|version| Json::Text(version.to_string()))
                    .collect(),
            ),
        )])
    }
}

fn dependencies_json(dependencies: &[(String, Version)]) -> Json {
    Json::Object(
        dependencies
            .iter()
            .map(|(name, version)| (name.clone(), Json::Text(version.to_string())))
            .collect(),
    )
}

/// An object rendered with two-space indentation and a final line break.
fn render(fields: Vec<(&str, Json)>) -> String {
    let json = Json::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    );
    let mut out = String::new();
    write_json(&json, &mut out, Some(2), 0);
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_reads_and_renders_the_same() {
        let text = "{\n  \"name\": \"invoices\",\n  \"version\": \"1.2.0\",\n  \"purpose\": \"Bill the customers.\",\n  \"dependencies\": {\n    \"greeting\": \"1.0.0\"\n  },\n  \"registry\": \"../registry\",\n  \"budgets\": {\n    \"public_per_module\": 12\n  }\n}\n";
        let manifest = Manifest::read(text).expect("a manifest");
        assert_eq!(manifest.name, "invoices");
        assert_eq!(
            manifest.dependencies,
            vec![("greeting".to_string(), Version::parse("1.0.0").unwrap())]
        );
        assert_eq!(
            manifest.budgets.as_ref().unwrap().public_per_module,
            Some(12)
        );
        assert_eq!(manifest.render(), text);
    }

    #[test]
    fn a_python_section_reads_and_renders_the_same() {
        // decision AL1: the interpreter and the modules, `package` left out
        // when it is the module's own name
        let text = "{\n  \"name\": \"bridge\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": {},\n  \"python\": {\n    \"interpreter\": \"python3\",\n    \"modules\": {\n      \"analysis\": {\n        \"symbols\": {\n          \"mean\": \"average\"\n        }\n      },\n      \"stats\": {\n        \"package\": \"scipy.stats\"\n      }\n    }\n  }\n}\n";
        let manifest = Manifest::read(text).expect("a manifest");
        assert_eq!(manifest.python.interpreter.as_deref(), Some("python3"));
        assert_eq!(manifest.python.modules.len(), 2);
        assert_eq!(manifest.python.modules[0].1.package, "analysis");
        assert_eq!(
            manifest.python.modules[0].1.symbols,
            vec![("mean".to_string(), "average".to_string())]
        );
        assert_eq!(manifest.python.modules[1].1.package, "scipy.stats");
        assert_eq!(manifest.render(), text);
        let error = Manifest::read(
            "{\"name\": \"a\", \"version\": \"1.0.0\", \"python\": {\"modules\": {\"Bad\": {}}}}",
        )
        .expect_err("refused");
        assert_eq!(
            error,
            "`Bad` in the Python modules of the manifest is not a module name"
        );
        let error = Manifest::read(
            "{\"name\": \"a\", \"version\": \"1.0.0\", \"python\": {\"worker\": \"x\"}}",
        )
        .expect_err("refused");
        assert_eq!(error, "`worker` is not a field of `python` of the manifest");
    }

    #[test]
    fn a_field_the_format_lacks_is_refused() {
        let error = Manifest::read("{\"name\": \"a\", \"version\": \"1.0.0\", \"author\": \"x\"}")
            .expect_err("refused");
        assert_eq!(error, "`author` is not a field of the manifest");
        let error =
            Manifest::read("{\"name\": \"A\", \"version\": \"1.0.0\"}").expect_err("refused");
        assert!(error.starts_with("`A` is not a package name"), "{error}");
        let error =
            Lock::read("{\"packages\": [{\"name\": \"a\", \"version\": \"1\", \"hash\": \"x\"}]}")
                .expect_err("refused");
        assert!(error.contains("not a version"), "{error}");
        assert!(Manifest::read("[]").is_err());
        assert!(Manifest::read("{").is_err());
    }

    #[test]
    fn a_lock_and_a_package_file_round_trip() {
        let lock = Lock {
            packages: vec![Locked {
                name: "greeting".to_string(),
                version: Version::parse("1.0.0").unwrap(),
                hash: "sha256:00".to_string(),
            }],
        };
        assert_eq!(Lock::read(&lock.render()).expect("a lock"), lock);
        let package = PackageFile {
            name: "greeting".to_string(),
            version: Version::parse("1.0.0").unwrap(),
            purpose: Some("Greet.".to_string()),
            dependencies: Vec::new(),
            toolchain: "renyi 0.0.1".to_string(),
            files: vec![("greeting.ry".to_string(), "sha256:11".to_string())],
            effects: vec![Effect {
                function: "greeting.announce".to_string(),
                needs: vec!["console".to_string()],
                fails: Vec::new(),
            }],
        };
        assert_eq!(
            PackageFile::read(&package.render()).expect("a package file"),
            package
        );
        let versions = Versions {
            versions: vec![
                Version::parse("1.0.0").unwrap(),
                Version::parse("1.1.0").unwrap(),
            ],
        };
        assert_eq!(
            Versions::read(&versions.render()).expect("versions"),
            versions
        );
    }
}
