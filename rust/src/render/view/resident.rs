//! The rigs a level keeps ready from its load to the next (no stalls mid-fight).
//!
//! A foe's rig used to be meshed when its view was made (in the middle of the update that brought
//! it) and put on the GPU the first frame it was drawn: a new kind coming out of a door, a pack of
//! a kind not yet seen waking, the hero raising his shield or blinking after a blow cost from 5 to
//! 26 ms in that frame. Now, as a level is loaded (behind the loading screen, RP03), every kind it
//! can deal (`Game::level_kinds`) and the hero's own rigs are meshed at every level of detail, and
//! the GPU makes their buffers before the level is shown and keeps them while the level lives
//! (`Gpu::sync_static`, `Gpu::level_ready`). Anything missed is still meshed and sent up when it is
//! first drawn, and counted (`FrameCounts::rig_misses`).

use std::sync::Arc;

use super::animation::rig_for;
use super::*;

/// Numbers each preparation, across every view (a new game's view must not pass for the last one's).
static NEXT_GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// The meshes the level keeps on the GPU, and which preparation they are (a new one per level,
/// 0 before the first).
#[derive(Default)]
pub struct Resident {
    pub gen: u64,
    pub meshes: Vec<Arc<RigMesh>>,
    /// The time the last preparation took on the CPU (meshing), in ms.
    pub ms: f64,
}

/// A rig to prepare: its solid meshes at every level of detail, and its see-through ones too
/// where it can be drawn faded (the hero blinking after a blow, his afterimages and spirits).
struct Want<'a> {
    rig: &'a Rig,
    whole: bool,
}

/// Meshes every level of detail of `rig` (and its see-through meshes if `whole`).
fn mesh_all(w: &Want) -> Vec<Arc<RigMesh>> {
    let mut out = Vec::new();
    for level in 0..LOD_LEVELS {
        let r = w.rig.lod(level);
        out.push(r.mesh());
        if w.whole {
            out.push(r.mesh_whole());
        }
    }
    out
}

impl WorldView {
    /// Makes ready, behind the loading screen, every rig the level can show: the foes of `kinds`
    /// (and those they bring), the hero as he is equipped (also with his shield raised, see-through,
    /// as his shield's breaking would leave him, his thrown weapon and his spirits), the archangels'
    /// shrines, the Hall of Mirrors' statues, and in a boss's meadow the Cyclops and his boulders.
    /// Their meshes are built now (on every core natively; in order on the web, which has no
    /// threads), and the GPU keeps them until the next level is prepared.
    pub fn prepare_rigs(&mut self, kinds: &[EnemyKind], hero_kind: HeroKind, hero: &Hero) {
        let t = web_time::Instant::now();
        self.refit_hero(hero_kind, hero);
        self.make_spare_hero();
        let mut ids: Vec<RigId> = kinds.iter().map(|&k| rig_for(k).0).collect();
        ids.sort_by_key(|&id| id as u8);
        ids.dedup();
        let m = &self.models;
        let mut wants: Vec<Want> = ids.iter().map(|&id| Want { rig: m.rig(id), whole: false }).collect();
        wants.push(Want { rig: &m.hero, whole: true });
        if let Some((bare, shield)) = &m.raised {
            wants.push(Want { rig: bare, whole: true });
            wants.push(Want { rig: shield, whole: true });
        }
        if let Some((_, spare, jointed)) = &m.spare {
            wants.push(Want { rig: spare, whole: true });
            if let Some(j) = jointed {
                wants.push(Want { rig: j, whole: true });
            }
        }
        // His jointed rig for his clips, and the clips themselves (read once, now rather than mid-fight).
        if let Some(j) = &m.jointed {
            wants.push(Want { rig: j, whole: true });
            crate::render::anim::library::clips(hero_kind);
        }
        wants.push(Want { rig: &m.thrown, whole: false });
        match hero_kind {
            HeroKind::Mehmed => wants.push(Want { rig: &m.spirits[0], whole: true }),
            HeroKind::Miyamoto => wants.push(Want { rig: &m.spirits[1], whole: true }),
            HeroKind::Ragnar => {}
        }
        // The archangels' shrines, where the level has them (small: a statue each).
        for shrine in &m.angel_shrines {
            wants.push(Want { rig: shrine, whole: false });
        }
        // The Hall of Mirrors' statues of him.
        if let Some(realms::RunView::Pride(v)) = &self.run_view {
            wants.push(Want { rig: v.statue(), whole: false });
        }
        if self.meadow {
            wants.push(Want { rig: &m.cyclops, whole: false });
            wants.push(Want { rig: &m.boulder, whole: false });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let built: Vec<Vec<Arc<RigMesh>>> = crate::render::faces::par_map(&wants, |_, w| mesh_all(w));
        #[cfg(target_arch = "wasm32")]
        let built: Vec<Vec<Arc<RigMesh>>> = wants.iter().map(mesh_all).collect();
        let mut meshes: Vec<Arc<RigMesh>> = built.into_iter().flatten().collect();
        meshes.sort_by_key(|m| m.id);
        meshes.dedup_by_key(|m| m.id);
        self.resident.meshes = meshes;
        self.resident.gen = NEXT_GEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.resident.ms = t.elapsed().as_secs_f64() * 1e3;
    }

    /// The rigs the level keeps ready (`prepare_rigs`).
    pub fn resident(&self) -> &Resident {
        &self.resident
    }

    /// Whether a foe of `kind` was made ready with the level: every level of its rig meshed and
    /// kept for the GPU.
    pub fn prepared(&self, kind: EnemyKind) -> bool {
        let rig = self.models.rig(rig_for(kind).0);
        (0..LOD_LEVELS).all(|l| {
            let id = rig.lod(l).mesh().id;
            self.resident.meshes.iter().any(|m| m.id == id)
        })
    }
}
