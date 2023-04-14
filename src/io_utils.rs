use anyhow::{bail, Context, Result};
use std::{
    fs::{create_dir_all, hard_link},
    path::PathBuf,
};

pub fn chk_canon_dir_exists(path: &PathBuf) -> Result<PathBuf> {
    create_dir_all(path)
        .with_context(|| format!("Cannot create directory at path: {}", path.display()))?;
    path.canonicalize()
        .with_context(|| format!("Invalid path: {}", path.display()))
}

pub fn link_file(src_base_dir: &PathBuf, src_file: &PathBuf, dst_base_dir: &PathBuf) -> Result<()> {
    let relative_src_file = src_file.strip_prefix(&src_base_dir).with_context(|| {
        format!(
            "Fail to strip path {} from path {}",
            src_base_dir.display(),
            src_file.display()
        )
    })?;
    let dst_file = &dst_base_dir.join(relative_src_file);

    let dst_parent_dir = &mut dst_file.clone();
    if !dst_parent_dir.pop() {
        bail!("Invalid parent path for file path: {}", dst_file.display());
    }

    create_dir_all(&dst_parent_dir).with_context(|| {
        format!(
            "Cannot create directory at path: {}",
            dst_parent_dir.display()
        )
    })?;

    hard_link(src_file, dst_file).with_context(|| {
        format!(
            "Failed to hardlink from {} to {}",
            src_file.display(),
            dst_file.display()
        )
    })
}
