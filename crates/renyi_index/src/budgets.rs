//! Module-level budgets (design document 05 section 5, decisions O3 and R7):
//! values the map reports as over budget, never compile errors.

use crate::Index;

/// The thresholds. The defaults are the first setting (decision R7), each
/// one just above the corpus maximum measured in section 5 of the design
/// document; the `budgets` of `renyi.json` override them (decision AC1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budgets {
    pub public_per_module: usize,
    pub effect_paths_per_module: usize,
    pub fan_out_per_definition: usize,
}

impl Default for Budgets {
    fn default() -> Self {
        Budgets {
            public_per_module: 10,
            effect_paths_per_module: 5,
            fan_out_per_definition: 7,
        }
    }
}

/// Every value over its budget, one line each, modules first and then
/// definitions, in map order: `<what>: <value> (budget <threshold>)`.
pub fn over_budget(index: &Index, budgets: &Budgets) -> Vec<String> {
    let mut lines = Vec::new();
    for module in &index.modules {
        if module.public > budgets.public_per_module {
            lines.push(format!(
                "module {}: {} public definitions (budget {})",
                module.name, module.public, budgets.public_per_module
            ));
        }
        if module.effects.len() > budgets.effect_paths_per_module {
            lines.push(format!(
                "module {}: {} transitive effect paths (budget {}): {}",
                module.name,
                module.effects.len(),
                budgets.effect_paths_per_module,
                module.effects.join(", ")
            ));
        }
    }
    for definition in &index.definitions {
        if definition.metrics.fan_out > budgets.fan_out_per_definition {
            lines.push(format!(
                "{} {}: fan-out {} (budget {})",
                definition.kind.name(),
                definition.qualified(),
                definition.metrics.fan_out,
                budgets.fan_out_per_definition
            ));
        }
    }
    lines
}
