//! Windows forbids directory renames with open descendants. Move exact held
//! files into fresh pinned containers; never release content guards to rename.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactBoundary {
    FileMoved(usize),
    BeforeCleanup,
}

impl LockedFiles {
    pub(super) fn move_artifacts(
        &mut self,
        name: &str,
        from: &Path,
        to: &Path,
        hook: &mut impl FnMut(ArtifactBoundary) -> AppResult<()>,
    ) -> AppResult<()> {
        if name != "owner-artifacts" {
            return Err(error("only the managed artifact tree is moved by files"));
        }
        let source_parent = from
            .parent()
            .ok_or_else(|| error("artifact source parent is absent"))?;
        let target_parent = to
            .parent()
            .ok_or_else(|| error("artifact destination parent is absent"))?;
        let _source_pins = files::pin_chain(source_parent)?;
        let _target_pins = files::pin_chain(target_parent)?;
        let prefix = format!("{name}/");
        let directories = self
            .directories
            .keys()
            .filter(|key| key.as_str() == name || key.starts_with(&prefix))
            .cloned()
            .collect::<Vec<_>>();
        let mut destination_guards = BTreeMap::new();
        // Sorted relative names put each parent before its children. Existing
        // destination names are never adopted or merged into this generation.
        for key in &directories {
            let suffix = suffix(key, name)?;
            let source = join_suffix(from, suffix);
            let target = join_suffix(to, suffix);
            io(
                &source,
                "validate held artifact source directory",
                native::validate_location(&self.directories[key], &source),
            )?;
            io(
                &target,
                "create fresh artifact directory",
                std::fs::create_dir(&target),
            )?;
            let pin = io(
                &target,
                "pin artifact destination",
                native::pin_directory(&target),
            )?;
            destination_guards.insert(key.clone(), pin);
        }
        for (index, (key, guard)) in self
            .files
            .iter()
            .filter(|(key, _)| key.starts_with(&prefix))
            .enumerate()
        {
            let expected = self
                .fingerprint
                .get(key)
                .ok_or_else(|| error("held artifact lacks a validated content digest"))?;
            if &files::hash(guard)? != expected {
                return Err(error("held artifact changed before promotion"));
            }
            let suffix = suffix(key, name)?;
            let source = join_suffix(from, suffix);
            let target = join_suffix(to, suffix);
            io(
                &source,
                "move exact held artifact",
                native::rename_held(guard, &source, &target),
            )?;
            hook(ArtifactBoundary::FileMoved(index))?;
        }
        hook(ArtifactBoundary::BeforeCleanup)?;
        // Keep fresh destination pins until complete-generation verification.
        // Source removal is nonrecursive and can remove only empty containers.
        // Any unexpected child fails closed with the Prepared journal intact.
        for key in directories.into_iter().rev() {
            let source = join_suffix(from, suffix(&key, name)?);
            drop(self.directories.remove(&key));
            io(
                &source,
                "remove empty artifact source directory",
                std::fs::remove_dir(&source),
            )?;
            self.directories.insert(
                key.clone(),
                destination_guards
                    .remove(&key)
                    .ok_or_else(|| error("artifact destination pin disappeared"))?,
            );
        }
        Ok(())
    }
}

fn join_suffix(root: &Path, relative: &Path) -> PathBuf {
    // Joining an empty path adds a trailing separator on Windows. Keep the
    // root spelling identical to the held handle's final directory path.
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn suffix<'a>(key: &'a str, root: &str) -> AppResult<&'a Path> {
    Path::new(key)
        .strip_prefix(root)
        .map_err(|_| error("artifact path is outside its managed root"))
}
