//! LLVM compiler output type.

use std::io::Write;

use handl_plugin_compile::{Artifact, CompileError, CompilerOutput};

/// The successful result of compiling Handl to LLVM.
#[derive(Debug, Clone, PartialEq)]
pub struct LlvmOutput {
    target: String,
    triple: String,
    artifacts: Vec<Artifact>,
    object: Vec<u8>,
}

/// Artifact name for the compiled object of `triple`
/// (`x86_64-unknown-unknown` → `x86_64_unknown_unknown.o`) — shared by
/// [`LlvmOutput::from_object`] and the kernel `Emitter` path.
pub(crate) fn object_artifact_name(triple: &str) -> String {
    format!("{}.o", triple.replace('-', "_"))
}

impl LlvmOutput {
    /// Create an output from a compiled object byte buffer.
    pub fn from_object(triple: impl Into<String>, object: Vec<u8>) -> Self {
        let triple = triple.into();
        let name = object_artifact_name(&triple);
        Self {
            target: "llvm".to_string(),
            triple,
            artifacts: vec![Artifact::new(name, object.clone())],
            object,
        }
    }

    /// The target triple this output was built for.
    pub fn triple(&self) -> &str {
        &self.triple
    }

    /// The emitted object bytes.
    pub fn object(&self) -> &[u8] {
        &self.object
    }
}

impl CompilerOutput for LlvmOutput {
    fn target(&self) -> &str {
        &self.target
    }

    fn artifacts(&self) -> &[Artifact] {
        &self.artifacts
    }

    fn stream(&self, writer: &mut dyn Write) -> Result<(), CompileError> {
        writer
            .write_all(&self.object)
            .map_err(|e| CompileError::Io(e.to_string()))
    }
}
