//! Snapshot + restore using df_displmgr (replaces DEVMODE/ChangeDisplaySettingsExW usage)

use log::{info, warn, error};
use df_displmgr::NativeTopology;
use df_displmgr::types::{Point2D, Extent2D};
use df_displmgr::DisplayId as DfDisplayId;

#[derive(Clone, Debug)]
struct SavedOutput {
    id: String,
    enabled: bool,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

/// Restores a Snapshot of the Display Config using df_displmgr API.
pub struct DisplayRestorer {
    pub snapshot: Vec<SavedOutput>,
}

impl DisplayRestorer {
    /// Take a snapshot of the current topology via df_displmgr
    pub fn take_snapshot() -> Self {
        let mut saved = Vec::new();
        if let Ok(topo) = NativeTopology::acquire() {
            for o in topo.get_outputs() {
                saved.push(SavedOutput {
                    id: o.identity.id.0.clone(),
                    enabled: o.enabled,
                    x: o.geometry.origin.x,
                    y: o.geometry.origin.y,
                    width: o.geometry.size.width,
                    height: o.geometry.size.height,
                });
            }
        }
        DisplayRestorer { snapshot: saved }
    }

    /// Restore the saved topology using df_displmgr (edits + commit)
    pub fn restore(&self) -> Result<(), df_displmgr::DisplayError> {
        let mut topo = NativeTopology::acquire()?;
        for s in &self.snapshot {
            let did = DfDisplayId(s.id.clone());
            if let Ok(mut editor) = topo.edit_output(&did) {
                let _ = editor.set_enabled(s.enabled);
                let _ = editor.set_position(Point2D { x: s.x, y: s.y });
                let _ = editor.set_resolution(Extent2D { width: s.width, height: s.height });
            }
        }
        topo.set_persistence(true);
        // validate + commit synchronously
        futures::executor::block_on(async {
            topo.validate().await?;
            topo.commit().await.map_err(|e| df_displmgr::DisplayError::DisplayError(format!("commit failed: {e:?}")))
        })?;
        Ok(())
    }
}

impl Drop for DisplayRestorer {
    fn drop(&mut self) {
        if let Err(e) = self.restore() {
            error!("Restore failed in Drop: {:?}", e);
        }
    }
}

/// Match display by query against a DisplayRow (unchanged behavior)
pub fn match_display(row: &crate::scraper::DisplayRow, query: &str) -> bool {
    let q = query.to_uppercase();
    row.persistent_id.to_string() == q 
        || row.name_id.to_uppercase() == q 
        || row.serial.to_uppercase() == q
        || row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
}
