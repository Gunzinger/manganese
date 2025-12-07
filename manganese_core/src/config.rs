use crate::InstructionSet;
use crate::tests::{avx2_definitions, avx512_definitions, TestDefinition, TestKind};

pub struct TestConfigEntry {
    pub kind: TestKind,
    pub loops: Option<usize>,
}

pub fn build_tests_from_config(
    entries: &[TestConfigEntry],
    isa: InstructionSet,
) -> Vec<TestDefinition> {
    let defs = match isa {
        InstructionSet::AVX2 => avx2_definitions(),
        InstructionSet::AVX512 => avx512_definitions(),
        _ => std::collections::HashMap::new(),
    };

    // if no entries are given (empty/non-existant config; use defaults)
    if entries.is_empty() {
        let mut defaults: Vec<_> = defs.values().cloned().collect();
        defaults.sort_by_key(|d| d.name);
        return defaults;
    }

    let mut result = Vec::new();

    for entry in entries {
        if let Some(def) = defs.get(&entry.kind) {
            result.push(TestDefinition {
                name:   def.name,
                passes: def.passes,
                iters:  def.iters,
                run:    def.run,
                loops:  entry.loops.unwrap_or(def.loops),
            });
        }
    }

    result
}

pub fn load_custom_config(path: &str) -> Result<Vec<TestConfigEntry>, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(path)?;
    let mut list = Vec::new();

    for (line_no, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();

        // skip blank lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // split into tokens
        let mut parts = line.split_whitespace();

        // first token = test kind
        let test_name = parts
            .next()
            .ok_or_else(|| format!("Invalid line {}: {}", line_no + 1, raw_line))?;

        let kind = TestKind::parse(test_name)
            .ok_or_else(|| format!("Unknown test '{}' on line {}", test_name, line_no + 1))?;

        let mut loops = None;

        // parse passes= and iters=
        for token in parts {
            if let Some(val) = token.strip_prefix("loops=") {
                loops = Some(val.parse::<usize>()
                    .map_err(|_| format!("Invalid loops value '{}' on line {}", val, line_no + 1))?);
            } else {
                return Err(format!("Unknown token '{}' on line {}", token, line_no + 1).into());
            }
        }

        list.push(TestConfigEntry { kind, loops });
    }

    Ok(list)
}
