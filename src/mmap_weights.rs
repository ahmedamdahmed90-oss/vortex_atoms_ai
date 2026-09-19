use crate::{Result, VortexAtomsError};
use candle_core::{Device, Tensor};
use memmap2::{Mmap, MmapOptions};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Supported on-disk weight encodings.
///
/// The initial kernel supports little-endian f32 values. Additional dtypes can be
/// added without changing the mmap safety model.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WeightDType {
    F32Le,
}

impl WeightDType {
    pub const fn bytes_per_element(self) -> usize {
        match self {
            Self::F32Le => std::mem::size_of::<f32>(),
        }
    }
}

/// Declarative description of a tensor inside a memory-mapped weight file.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TensorSpec {
    pub name: String,
    pub offset_bytes: u64,
    pub dims: Vec<usize>,
    pub dtype: WeightDType,
}

impl TensorSpec {
    /// Number of elements represented by `dims`.
    pub fn element_count(&self) -> Result<usize> {
        if self.dims.is_empty() {
            return Err(VortexAtomsError::InvalidTensorSpec(format!(
                "tensor '{}' must include at least one dimension",
                self.name
            )));
        }

        self.dims.iter().try_fold(1usize, |acc, dim| {
            if *dim == 0 {
                return Err(VortexAtomsError::InvalidTensorSpec(format!(
                    "tensor '{}' contains a zero-sized dimension",
                    self.name
                )));
            }

            acc.checked_mul(*dim)
                .ok_or(VortexAtomsError::Overflow("tensor element count"))
        })
    }

    /// Total number of bytes occupied by this tensor in the mapped file.
    pub fn byte_len(&self) -> Result<usize> {
        self.element_count()?
            .checked_mul(self.dtype.bytes_per_element())
            .ok_or(VortexAtomsError::Overflow("tensor byte length"))
    }
}

/// Read-only memory-mapped model weights.
///
/// `MmapWeights` keeps model data backed by the operating system page cache,
/// avoiding an eager heap allocation for the whole model. Callers request a
/// bounded range or tensor, and the kernel validates offsets before access.
#[derive(Clone, Debug)]
pub struct MmapWeights {
    path: PathBuf,
    len: usize,
    mmap: Arc<Mmap>,
}

impl MmapWeights {
    /// Opens and read-only maps a model weight file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let file = File::open(&path)?;
        let metadata = file.metadata()?;

        if metadata.len() == 0 {
            return Err(VortexAtomsError::EmptyWeightFile(path));
        }

        let len = usize::try_from(metadata.len())
            .map_err(|_| VortexAtomsError::Overflow("weight file length"))?;

        // SAFETY:
        // - The file descriptor is opened read-only.
        // - We create an immutable `Mmap`, never `MmapMut`.
        // - All public accessors validate bounds before slicing.
        // - We do not transmute bytes into typed references, so unaligned model
        //   offsets cannot cause undefined behavior.
        //
        // External mutation of a mapped file cannot be fully prevented by this
        // process; production deployments should publish immutable weight files
        // atomically and avoid modifying files in place while mapped.
        let mmap = unsafe { MmapOptions::new().map(&file)? };

        Ok(Self {
            path,
            len,
            mmap: Arc::new(mmap),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns a checked byte slice from the mapped file.
    pub fn bytes(&self, offset: u64, len: usize) -> Result<&[u8]> {
        let start = usize::try_from(offset)
            .map_err(|_| VortexAtomsError::Overflow("weight byte offset"))?;
        let end = start
            .checked_add(len)
            .ok_or(VortexAtomsError::Overflow("weight byte range end"))?;

        self.mmap
            .get(start..end)
            .ok_or(VortexAtomsError::InvalidRange {
                offset,
                len,
                file_len: self.len,
            })
    }

    /// Reads little-endian f32 values from a checked range.
    ///
    /// This intentionally returns an owned `Vec<f32>` for the requested tensor
    /// only. The whole model file remains memory mapped and is not copied into
    /// process heap memory.
    pub fn f32_le_vec(&self, offset: u64, elements: usize) -> Result<Vec<f32>> {
        let byte_len = elements
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or(VortexAtomsError::Overflow("f32 tensor byte length"))?;
        let bytes = self.bytes(offset, byte_len)?;

        let mut values = Vec::with_capacity(elements);
        for chunk in bytes.chunks_exact(std::mem::size_of::<f32>()) {
            values.push(f32::from_le_bytes(
                chunk
                    .try_into()
                    .expect("chunks_exact(4) always yields 4-byte chunks"),
            ));
        }

        Ok(values)
    }

    /// Materializes one tensor described by `spec` onto the requested Candle
    /// device. Only the specified tensor range is copied out of the mapping.
    pub fn tensor_f32(&self, spec: &TensorSpec, device: &Device) -> Result<Tensor> {
        if spec.dtype != WeightDType::F32Le {
            return Err(VortexAtomsError::InvalidTensorSpec(format!(
                "tensor '{}' is not f32_le",
                spec.name
            )));
        }

        let values = self.f32_le_vec(spec.offset_bytes, spec.element_count()?)?;
        Ok(Tensor::from_vec(values, spec.dims.clone(), device)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn maps_and_reads_f32_values() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "vortex_atoms_ai_mmap_test_{}.bin",
            std::process::id()
        ));

        let mut file = File::create(&path)?;
        for value in [1.0f32, 2.5, -3.25, 4.0] {
            file.write_all(&value.to_le_bytes())?;
        }
        drop(file);

        let mapped = MmapWeights::open(&path)?;
        let spec = TensorSpec {
            name: "test_tensor".to_string(),
            offset_bytes: 0,
            dims: vec![2, 2],
            dtype: WeightDType::F32Le,
        };

        assert_eq!(
            mapped.f32_le_vec(spec.offset_bytes, spec.element_count()?)?,
            vec![1.0, 2.5, -3.25, 4.0]
        );

        std::fs::remove_file(path)?;
        Ok(())
    }
}
