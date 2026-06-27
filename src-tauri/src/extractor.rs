#[allow(clippy::type_complexity)]
pub mod archive;
#[allow(clippy::type_complexity)]
pub mod bandizip;
pub mod detector;
pub mod flow;
#[allow(clippy::type_complexity)]
pub mod seven_zip;

pub use archive::{extract_single_archive, find_archives_in_dir};
pub use detector::resolve_exe_path;
pub use flow::run_extraction_flow;
