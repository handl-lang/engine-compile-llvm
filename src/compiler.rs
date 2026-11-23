//! The `LlvmCompiler` implementation.

use handl_plugin_compile::{CompileError, Compiler, CompilerConfig, CompilerInput, CompilerOutput};
use hnl_engine::Term;
use hnl_tool_codegen::{
    ArtifactRef, Change, CodegenError, EmitContext, Emitter as CodegenEmitter,
};

use crate::config::{AArch64, LlvmConfig, LlvmTarget, X86_64};
use crate::emit::{DefaultObjectWriter, ObjectWriter};
use crate::output::{LlvmOutput, object_artifact_name};

/// LLVM compiler for the Handl arrow calculus.
#[derive(Debug, Default)]
pub struct LlvmCompiler<W = DefaultObjectWriter>
where
    W: ObjectWriter,
{
    writer: W,
}

impl LlvmCompiler<DefaultObjectWriter> {
    /// Create a compiler using the default object writer.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<W: ObjectWriter> LlvmCompiler<W> {
    /// Create a compiler with the given object writer.
    pub fn with_writer(writer: W) -> Self {
        Self { writer }
    }
}

impl<W> Compiler for LlvmCompiler<W>
where
    W: ObjectWriter + Send + Sync + 'static,
{
    fn compile(
        &self,
        input: &CompilerInput,
        config: &dyn CompilerConfig,
    ) -> Result<Box<dyn CompilerOutput>, CompileError> {
        let cfg = config
            .as_any()
            .downcast_ref::<LlvmConfig>()
            .ok_or_else(|| CompileError::InvalidInput("expected LlvmConfig".to_string()))?;

        let (triple, object) = self.object_for(&input.term, cfg)?;
        Ok(Box::new(LlvmOutput::from_object(triple, object)))
    }

    fn target(&self) -> &str {
        "llvm"
    }
}

impl<W: ObjectWriter> LlvmCompiler<W> {
    /// Lower `term` and emit the native object bytes for `cfg`'s
    /// target — shared by the legacy `Compiler` impl and the kernel
    /// `Emitter` impl so both produce byte-identical desired state.
    /// Returns `(target triple, object bytes)`.
    fn object_for(&self, term: &Term, cfg: &LlvmConfig) -> Result<(String, Vec<u8>), CompileError> {
        #[cfg(feature = "inkwell")]
        {
            let triple = cfg.target.triple().to_string();
            use inkwell::context::Context;

            use crate::lower::LlvmContext;

            let context = Context::create();
            let module = context.create_module("handl");
            let builder = context.create_builder();
            let mut ctx = LlvmContext::new(&context, &module, builder, triple.as_str());
            crate::lower::lower_root(&mut ctx, term)?;
            let ir = crate::emit::emit_ir(&ctx)?;
            let object = self.writer.write(&triple, &ir, cfg.passes())?;
            Ok((triple, object))
        }

        #[cfg(not(feature = "inkwell"))]
        {
            let _ = (term, &self.writer, cfg);
            Err(CompileError::Other(
                "handl-plugin-compile-llvm was built without the `inkwell` feature; \
                 rebuild with `--features inkwell` (requires llvm-config / LLVM)"
                    .to_string(),
            ))
        }
    }
}

/// `Emitter<Term>` over the desired-state codegen kernel (PL-004
/// T-092): the same render as the legacy `Compiler::compile`, expressed as
/// a desired `Change` set (the `<triple>.o` object artifact) with no
/// side effects. The legacy `Compiler` impl stays until
/// `handl-plugin-compile` migrates to the kernel pipeline.
///
/// Configuration arrives through `EmitContext.config` (the kernel's
/// string-map seam — `LlvmConfig`'s boxed `LlvmTarget` cannot cross
/// it):
///
/// - `target` — `"x86_64"` (default) or `"aarch64"`;
/// - `passes` — comma-separated LLVM pass names (`instcombine`,
///   `reassociate`, `gvn`, `simplifycfg`, `mem2reg`);
/// - `debug` — `"true"` sets `LlvmConfig::debug` (currently advisory:
///   no DWARF emission is wired yet).
///
/// `LlvmConfig::emit_object` is intentionally *not* read here — it is
/// dead config in the legacy path too (the backend always emits a
/// native object); see the field docs in [`crate::config`].
impl<W: ObjectWriter> CodegenEmitter<Term> for LlvmCompiler<W> {
    fn materialize(&self, ir: &Term, req: &EmitContext) -> Result<Vec<Change>, CodegenError> {
        let cfg = llvm_config_from_ctx(req)?;
        let (triple, object) = self
            .object_for(ir, &cfg)
            .map_err(|e| CodegenError::Emit(e.to_string()))?;
        Ok(vec![Change::binary(
            ArtifactRef::new(object_artifact_name(&triple)),
            object,
        )])
    }
}

/// Rebuild a [`LlvmConfig`] from the kernel's `EmitContext.config`
/// string map — see the `Emitter` impl above for the key contract.
fn llvm_config_from_ctx(req: &EmitContext) -> Result<LlvmConfig, CodegenError> {
    let target: Box<dyn LlvmTarget> = match req.config.get("target").map(String::as_str) {
        None | Some("x86_64") | Some("x86-64") => Box::new(X86_64),
        Some("aarch64") | Some("arm64") => Box::new(AArch64),
        Some(other) => {
            return Err(CodegenError::Unsupported(format!(
                "llvm target {other:?} (supported: \"x86_64\", \"aarch64\")"
            )));
        }
    };
    let mut cfg = LlvmConfig::new(target);
    if let Some(passes) = req.config.get("passes") {
        for name in passes.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            cfg.with_pass(name);
        }
    }
    if let Some(debug) = req.config.get("debug") {
        cfg.debug = debug.eq_ignore_ascii_case("true") || debug == "1";
    }
    Ok(cfg)
}
