//! Unit tests for the LLVM compiler.

use handl_language::{Ast, AstNode, Language};
#[cfg(feature = "inkwell")]
use handl_plugin_compile::CompileError;
use handl_plugin_compile::{Compiler, CompilerInput, CompilerOutput};
use hnl_engine::{ArrSource, RewriteWitness, Term};
use hnl_tool_typecheck::TypecheckReport;
use serde_json::Value;

use crate::{
    AArch64, DefaultObjectWriter, LlvmCompiler, LlvmConfig, LlvmOutput, LlvmTarget, ObjectWriter,
    X86_64,
};

fn input(term: Term) -> CompilerInput {
    let source = "test".to_string();
    let mut ast = Ast::new();
    let root = ast.push(AstNode::new("module"));
    let language = Language::new();
    let report = TypecheckReport::default();
    CompilerInput::new(source, ast, root, language, term, report)
}

#[cfg(feature = "inkwell")]
#[derive(Debug)]
struct MockWriter;

#[cfg(feature = "inkwell")]
impl ObjectWriter for MockWriter {
    fn write(&self, _triple: &str, _ir: &str, _passes: &[String]) -> Result<Vec<u8>, CompileError> {
        Ok(b"mock object".to_vec())
    }
}

#[test]
fn compiler_target_is_llvm() {
    assert_eq!(LlvmCompiler::new().target(), "llvm");
}

#[test]
fn x86_64_triple_is_correct() {
    assert_eq!(X86_64.name(), "x86_64");
    assert_eq!(X86_64.triple(), "x86_64-unknown-unknown");
}

#[test]
fn aarch64_triple_is_correct() {
    assert_eq!(AArch64.name(), "aarch64");
    assert_eq!(AArch64.triple(), "aarch64-unknown-unknown");
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn compile_without_inkwell_feature_fails() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    let err = LlvmCompiler::new()
        .compile(&input(Term::constant(Value::Null)), &cfg)
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("inkwell"), "{msg}");
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn dispatcher_branches_for_phase9_terms() {
    let terms = phase9_terms();
    for term in &terms {
        let mut ctx = crate::lower::LlvmContext::new(term, X86_64.triple());
        let result = crate::lower::lower_term(&mut ctx, term, ());
        assert!(result.is_ok(), "lower_term failed for {term:?}");
    }
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn compile_without_inkwell_feature_fails_for_phase9_terms() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    for term in phase9_terms() {
        let err = LlvmCompiler::new().compile(&input(term), &cfg).unwrap_err();
        assert!(format!("{err}").contains("inkwell"), "{err}");
    }
}

#[cfg(feature = "inkwell")]
#[test]
fn compiler_uses_supplied_object_writer() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    let compiler = LlvmCompiler::with_writer(MockWriter);
    let output = compiler
        .compile(&input(Term::constant(Value::Null)), &cfg)
        .unwrap();
    assert_eq!(output.primary_output().unwrap(), &b"mock object"[..]);
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn default_object_writer_fails_without_inkwell() {
    let writer = DefaultObjectWriter;
    let err = writer
        .write("x86_64-unknown-unknown", "; ir", &[])
        .unwrap_err();
    assert!(format!("{err}").contains("inkwell"), "{err}");
}

#[cfg(feature = "inkwell")]
#[test]
fn default_object_writer_emits_object() {
    let ir = "define i32 @main() nounwind {\n  ret i32 0\n}\n";
    let bytes = DefaultObjectWriter
        .write("x86_64-unknown-unknown", ir, &[])
        .unwrap();
    assert!(!bytes.is_empty());
}

#[cfg(feature = "inkwell")]
#[test]
fn llvm_const_compiles_to_object() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    let output = LlvmCompiler::new()
        .compile(&input(Term::constant(Value::Null)), &cfg)
        .unwrap();
    assert_eq!(output.target(), "llvm");
    assert_eq!(output.artifacts().len(), 1);
    assert!(output.artifacts()[0].name.ends_with(".o"));
    assert!(!output.primary_output().unwrap().is_empty());
}

#[cfg(feature = "inkwell")]
#[test]
fn phase9_terms_compile_to_object() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    for term in phase9_terms() {
        let output = LlvmCompiler::new().compile(&input(term), &cfg).unwrap();
        assert_eq!(output.artifacts().len(), 1);
        assert!(
            output.artifacts()[0].name.ends_with(".o"),
            "term {term:?} did not emit .o"
        );
        assert!(
            !output.primary_output().unwrap().is_empty(),
            "term {term:?} emitted empty object"
        );
    }
}

#[test]
fn llvm_config_default_optimized_has_passes() {
    let cfg = LlvmConfig::default_optimized();
    assert_eq!(cfg.passes(), &["instcombine", "reassociate", "gvn"]);
}

#[test]
fn llvm_config_passes_round_trip() {
    let mut cfg = LlvmConfig::new(Box::new(X86_64));
    assert!(cfg.passes().is_empty());
    cfg.with_pass("simplifycfg");
    assert_eq!(cfg.passes(), &["simplifycfg"]);
    cfg.with_pass("gvn");
    assert_eq!(cfg.passes(), &["simplifycfg", "gvn"]);
}

#[test]
fn pass_list_round_trip() {
    let mut cfg = LlvmConfig::default_optimized();
    cfg.with_pass("simplifycfg");
    assert_eq!(
        cfg.passes(),
        &["instcombine", "reassociate", "gvn", "simplifycfg"]
    );
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn default_object_writer_fails_before_pass_validation() {
    let writer = DefaultObjectWriter;
    let err = writer
        .write("x86_64-unknown-unknown", "; ir", &["unknown".to_string()])
        .unwrap_err();
    assert!(format!("{err}").contains("inkwell"), "{err}");
}

#[cfg(feature = "inkwell")]
#[test]
fn default_object_writer_rejects_unknown_pass() {
    let writer = DefaultObjectWriter;
    let err = writer
        .write("x86_64-unknown-unknown", "; ir", &["unknown".to_string()])
        .unwrap_err();
    assert!(format!("{err}").contains("unknown pass"), "{err}");
}

#[test]
fn llvm_output_stream_writes_object() {
    let output = LlvmOutput::from_object("x86_64-unknown-unknown", b"obj".to_vec());
    let mut buf = Vec::new();
    output.stream(&mut buf).unwrap();
    assert_eq!(buf, b"obj");
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn compile_error_names_missing_inkwell_feature() {
    let cfg = LlvmConfig::new(Box::new(X86_64));
    let err = LlvmCompiler::new()
        .compile(&input(Term::constant(Value::Null)), &cfg)
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("inkwell"), "{msg}");
    assert!(msg.contains("--features inkwell"), "{msg}");
}

#[cfg(feature = "inkwell")]
#[test]
fn llvm_compiler_helpful_error_for_missing_target() {
    let err = DefaultObjectWriter
        .write("not-a-real-triple", "; ir", &[])
        .unwrap_err();
    assert!(matches!(err, CompileError::UnsupportedTarget(_)), "{err}");
    assert!(format!("{err}").contains("not-a-real-triple"), "{err}");
}

fn phase9_terms() -> Vec<Term> {
    vec![
        Term::first(Term::constant(Value::Null)),
        Term::choice(Term::constant(Value::Null), Term::constant(Value::Null)),
        Term::loop_feedback(Term::constant(Value::Null)),
        Term::Iter {
            fuel: Box::new(Term::constant(Value::Null)),
            predicate: Box::new(Term::constant(Value::Null)),
            step: Box::new(Term::constant(Value::Null)),
            initial: Box::new(Term::constant(Value::Null)),
        },
        Term::app(
            Term::constant(Value::Null),
            vec![Term::constant(Value::Null)],
        ),
        Term::Child {
            role: "body".to_string(),
        },
        Term::Handle {
            body: Box::new(Term::constant(Value::Null)),
            target: "io".to_string(),
            impl_: Box::new(Term::constant(Value::Null)),
            acc: None,
        },
        Term::Rewrite {
            source: Box::new(Term::constant(Value::Null)),
            target: Box::new(Term::constant(Value::Null)),
            witness: RewriteWitness::Compute,
        },
        Term::Arr {
            dim: Box::new(Term::constant(Value::from(1))),
            value: Value::Null,
        },
        Term::Op {
            dim: Box::new(Term::constant(Value::from(1))),
            name: "id".to_string(),
            args: vec![],
            source: ArrSource::Native,
            lit: None,
            attrs: None,
        },
    ]
}

// --- PL-004 T-092: kernel `Emitter<Term>` path -------------------------

#[test]
fn materialize_rejects_unknown_target() {
    use hnl_tool_codegen::{CodegenError, EmitContext, Emitter as _};

    let req = EmitContext::new().with_config("target", "mips");
    let err = LlvmCompiler::new()
        .materialize(&Term::constant(Value::Null), &req)
        .unwrap_err();
    assert!(
        matches!(err, CodegenError::Unsupported(_)),
        "expected Unsupported, got {err}"
    );
}

#[cfg(not(feature = "inkwell"))]
#[test]
fn materialize_without_inkwell_feature_fails() {
    use hnl_tool_codegen::{EmitContext, Emitter as _};

    let err = LlvmCompiler::new()
        .materialize(&Term::constant(Value::Null), &EmitContext::new())
        .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("inkwell"), "{msg}");
}

#[cfg(feature = "inkwell")]
#[test]
fn materialize_matches_compile_artifact() {
    use hnl_tool_codegen::{EmitContext, Emitter as _};

    let term = Term::constant(Value::Null);
    let compiler = LlvmCompiler::with_writer(MockWriter);
    let cfg = LlvmConfig::new(Box::new(X86_64));
    let expected = compiler.compile(&input(term.clone()), &cfg).unwrap();

    let changes = compiler
        .materialize(&term, &EmitContext::new())
        .expect("materialize");
    assert_eq!(changes.len(), 1);
    let artifact = expected.artifacts().first().unwrap();
    assert_eq!(artifact.name, "x86_64_unknown_unknown.o");
    assert_eq!(changes[0].artifact().as_str(), artifact.name);
    assert_eq!(changes[0].content().unwrap(), &b"mock object"[..]);
}

#[cfg(feature = "inkwell")]
#[test]
fn materialize_honours_ctx_target_and_passes() {
    use hnl_tool_codegen::{EmitContext, Emitter as _};

    /// Writer that records the triple/passes it was asked to emit.
    #[derive(Debug)]
    struct RecordingWriter {
        seen: std::sync::Arc<std::sync::Mutex<Vec<(String, Vec<String>)>>>,
    }

    impl ObjectWriter for RecordingWriter {
        fn write(
            &self,
            triple: &str,
            _ir: &str,
            passes: &[String],
        ) -> Result<Vec<u8>, CompileError> {
            self.seen
                .lock()
                .unwrap()
                .push((triple.to_string(), passes.to_vec()));
            Ok(b"obj".to_vec())
        }
    }

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let compiler = LlvmCompiler::with_writer(RecordingWriter { seen: seen.clone() });
    let req = EmitContext::new()
        .with_config("target", "aarch64")
        .with_config("passes", "instcombine, gvn");
    let changes = compiler
        .materialize(&Term::constant(Value::Null), &req)
        .expect("materialize");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].artifact().as_str(), "aarch64_unknown_unknown.o");
    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.as_slice(),
        &[(
            "aarch64-unknown-unknown".to_string(),
            vec!["instcombine".to_string(), "gvn".to_string()]
        )]
    );
}
