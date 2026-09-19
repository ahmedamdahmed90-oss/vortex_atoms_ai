#![cfg(feature = "dashboard")]

fn main() -> eframe::Result<()> {
    vortex_atoms_ai::dashboard::run_vortex_dashboard()
}
