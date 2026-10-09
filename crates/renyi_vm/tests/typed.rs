//! The types the emitters write beside every op (decision AU1, stage iii)
//! against the VM's own inference: wherever the analysis of the baseline
//! JIT types what an op pushes as one of the three scalars, the type the
//! checker noted for the op's expression must be that scalar's. Over the
//! corpus, the conformance programs and the compiler's own sources.

use std::path::{Path, PathBuf};

use renyi_check::types::Ty;
use renyi_syntax::SourceFile;
use renyi_vm::native::infer::{abs_of_type, analyse, Abs};
use renyi_vm::{compile_project, Program};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

fn sources(directory: &Path) -> Vec<SourceFile> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .expect("the directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "ry"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(&path).expect("the source");
            SourceFile::new(path.display().to_string(), &text)
        })
        .collect()
}

/// The program of the files when every one of them checks cleanly.
fn compiled(files: &[SourceFile]) -> Option<Program> {
    let checked = renyi_check::check_project(files);
    let clean = checked
        .modules
        .iter()
        .all(|module| module.diagnostics.iter().all(|d| !d.is_error()));
    clean.then(|| compile_project(&checked, files))
}

/// Whether a value of the noted type may be what the analysis says the op
/// pushes: the type's own representation, or, for a `maybe`, its inner
/// type's (a `maybe Integer` slot the analysis proved always holds an
/// Integer).
fn compatible(program: &Program, noted: &Ty, abs: Abs) -> bool {
    if abs_of_type(program, noted) == abs {
        return true;
    }
    match noted {
        Ty::Maybe(inner) => abs_of_type(program, inner) == abs,
        _ => false,
    }
}

/// Every op whose noted type disagrees with what the analysis says it
/// pushes, named.
fn disagreements(program: &Program) -> Vec<String> {
    let mut found = Vec::new();
    for code in &program.codes {
        let Ok(analysis) = analyse(program, code) else {
            continue;
        };
        assert_eq!(code.types.len(), code.ops.len(), "{}", code.name);
        for (pc, op) in code.ops.iter().enumerate() {
            if !op.pushes_its_expression() {
                continue;
            }
            let Some(index) = code.types[pc] else {
                continue;
            };
            let noted_ty = &program.result_types[index as usize];
            let noted = abs_of_type(program, noted_ty);
            let next = pc + 1;
            if next >= code.ops.len() || analysis.block_starts[next] {
                continue;
            }
            let Some(Some(state)) = analysis.entry.get(next) else {
                continue;
            };
            let Some(top) = state.last() else {
                continue;
            };
            if matches!(top, Abs::Int | Abs::Bool | Abs::Float)
                && !compatible(program, noted_ty, *top)
            {
                found.push(format!(
                    "{}: op {pc} ({op:?}) pushes {top:?} by the analysis, {noted:?} by the checker",
                    code.name
                ));
            }
        }
    }
    found
}

#[test]
fn the_noted_types_agree_with_the_inference_over_the_corpus_and_the_compiler() {
    let root = workspace();
    let mut checked = 0;
    for file in sources(&root.join("examples")) {
        if let Some(program) = compiled(std::slice::from_ref(&file)) {
            assert_eq!(
                disagreements(&program),
                Vec::<String>::new(),
                "{}",
                file.name
            );
            checked += 1;
        }
    }
    for file in sources(&root.join("tests/conformance/programs")) {
        if let Some(program) = compiled(std::slice::from_ref(&file)) {
            assert_eq!(
                disagreements(&program),
                Vec::<String>::new(),
                "{}",
                file.name
            );
            checked += 1;
        }
    }
    let compiler = sources(&root.join("compiler"));
    let program = compiled(&compiler).expect("the compiler checks cleanly");
    assert_eq!(disagreements(&program), Vec::<String>::new(), "compiler/");
    // the compiler's own ops are typed for the most part
    let (typed, total) = program.codes.iter().fold((0, 0), |(typed, total), code| {
        (
            typed + code.types.iter().filter(|t| t.is_some()).count(),
            total + code.ops.len(),
        )
    });
    assert!(typed * 2 > total, "{typed} of {total} ops typed");
    assert!(checked >= 30, "{checked} programs checked");
}
