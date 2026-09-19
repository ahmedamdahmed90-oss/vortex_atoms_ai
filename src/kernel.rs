use crate::{MmapWeights, Result, TensorSpec};
use candle_core::{Device, Tensor};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Runtime device preference for Vortex Atoms AI.
///
/// The initial foundation defaults to CPU because it is universally available.
/// GPU backends can be added later behind Candle feature flags without changing
/// the mmap loading layer.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DevicePreference {
    #[default]
    Cpu,
}

/// Kernel configuration for low-latency inference services.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct KernelConfig {
    pub identity: String,
    pub device: DevicePreference,
    pub qdrant_url: Option<String>,
    pub max_inflight_requests: usize,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            identity: "vortex_atoms_ai".to_string(),
            device: DevicePreference::Cpu,
            qdrant_url: None,
            max_inflight_requests: 64,
        }
    }
}

/// Foundational inference kernel for Vortex Atoms AI.
pub struct VortexAtomsKernel {
    config: KernelConfig,
    weights: MmapWeights,
    device: Device,
}

impl VortexAtomsKernel {
    /// Creates a kernel with memory-mapped weights.
    pub fn new(config: KernelConfig, weights_path: impl AsRef<Path>) -> Result<Self> {
        let device = match config.device {
            DevicePreference::Cpu => Device::Cpu,
        };

        let weights = MmapWeights::open(weights_path)?;

        Ok(Self {
            config,
            weights,
            device,
        })
    }

    pub fn config(&self) -> &KernelConfig {
        &self.config
    }

    pub fn weights(&self) -> &MmapWeights {
        &self.weights
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Loads a single tensor from the mapped weights into Candle.
    ///
    /// This is deliberately granular: the mmap remains the source of truth and
    /// only the requested tensor is materialized for compute.
    pub fn load_tensor(&self, spec: &TensorSpec) -> Result<Tensor> {
        self.weights.tensor_f32(spec, &self.device)
    }

    /// Lightweight readiness hook for service startup.
    pub fn readiness(&self) -> KernelReadiness {
        KernelReadiness {
            identity: self.config.identity.clone(),
            mapped_weight_path: self.weights.path().display().to_string(),
            mapped_weight_bytes: self.weights.len(),
            device: match self.config.device {
                DevicePreference::Cpu => "cpu".to_string(),
            },
        }
    }
}

/// Serializable readiness payload suitable for health checks.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct KernelReadiness {
    pub identity: String,
    pub mapped_weight_path: String,
    pub mapped_weight_bytes: usize,
    pub device: String,
}
