//! The clips the game carries: every `assets/anim/<hero>/<clip>.glb` with its `<clip>.json`
//! beside it, embedded in the build (the web build has no file system; `build.rs` lists them), and
//! read once per hero, the first time his clips are wanted (with his level, never mid-fight).

use std::sync::OnceLock;

use super::clip::{Clip, Skeleton};
use crate::logic::heroes::HeroKind;

// `EMBEDDED: &[(hero, clip, glb bytes, markers json)]`, written by build.rs.
include!(concat!(env!("OUT_DIR"), "/anim_clips.rs"));

/// How the hero is animated: by his clips (the default), by the code as before (`VH_ANIM=code`,
/// for comparisons), or by his clips with every voxel painted its bone's colour (`VH_ANIM=bones`,
/// to check the cut).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Clips,
    Code,
    Bones,
}

impl Mode {
    /// The mode `VH_ANIM` asks for (read once).
    pub fn from_env() -> Mode {
        static MODE: OnceLock<Mode> = OnceLock::new();
        *MODE.get_or_init(|| match std::env::var("VH_ANIM").ok().as_deref().map(str::trim) {
            Some("code") => Mode::Code,
            Some("bones") => Mode::Bones,
            _ => Mode::Clips,
        })
    }
}

/// A hero's clips, read and resampled, and the skeleton they share.
pub struct HeroClips {
    pub skeleton: Skeleton,
    pub clips: Vec<Clip>,
    /// How long reading them took, in milliseconds.
    pub load_ms: f64,
    /// The clips that would not read, and why (each falls back to the code's animation).
    pub errors: Vec<String>,
}

impl HeroClips {
    pub fn get(&self, name: &str) -> Option<&Clip> {
        self.clips.iter().find(|c| c.name == name)
    }

    /// Reads every clip of `hero` in `files` (hero, clip, GLB, markers), or none if none reads.
    pub fn read(hero: &str, files: &[(&str, &str, &[u8], &str)]) -> Option<HeroClips> {
        let t = web_time::Instant::now();
        let mut clips = Vec::new();
        let mut errors = Vec::new();
        for (h, name, glb, json) in files.iter().filter(|f| f.0 == hero) {
            match Clip::from_glb(name, glb, json) {
                Ok(c) => clips.push(c),
                Err(e) => errors.push(format!("{h}/{e}")),
            }
        }
        let skeleton = clips.first()?.skeleton.clone();
        Some(HeroClips { skeleton, clips, load_ms: t.elapsed().as_secs_f64() * 1e3, errors })
    }
}

fn hero_dir(hero: HeroKind) -> &'static str {
    match hero {
        HeroKind::Ragnar => "ragnar",
        HeroKind::Mehmed => "mehmed",
        HeroKind::Miyamoto => "miyamoto",
    }
}

/// `hero`'s clips, read the first time they are asked for; none if he has none.
pub fn clips(hero: HeroKind) -> Option<&'static HeroClips> {
    static HEROES: [OnceLock<Option<HeroClips>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = match hero {
        HeroKind::Ragnar => 0,
        HeroKind::Mehmed => 1,
        HeroKind::Miyamoto => 2,
    };
    HEROES[slot]
        .get_or_init(|| {
            let read = HeroClips::read(hero_dir(hero), EMBEDDED);
            if let Some(c) = &read {
                for e in &c.errors {
                    crate::platform::warn(&format!("animation: {e}"));
                }
            }
            read
        })
        .as_ref()
}

/// The names of the clips embedded for `hero`.
pub fn embedded(hero: HeroKind) -> Vec<&'static str> {
    EMBEDDED.iter().filter(|f| f.0 == hero_dir(hero)).map(|f| f.1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::anim::skeleton::BONE_COUNT;

    const HEROES: [HeroKind; 3] = [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto];

    #[test]
    fn every_heros_clips_read_and_carry_their_markers() {
        for hero in HEROES {
            let c = clips(hero).unwrap_or_else(|| panic!("{hero:?}'s clips are embedded"));
            assert!(c.errors.is_empty(), "{hero:?}: {:?}", c.errors);
            for name in
                ["idle", "walk", "run", "slash_fore", "slash_back", "slash_heavy", "block", "block_idle", "jump", "roll", "hit", "parry"]
            {
                let clip = c.get(name).unwrap_or_else(|| panic!("{hero:?} has no {name}"));
                assert!(clip.frame_count() > 10, "{hero:?} {name}");
            }
            for cycle in ["idle", "walk", "run", "block_idle"] {
                assert!(c.get(cycle).unwrap().markers.looping, "{hero:?}: {cycle} loops");
            }
            assert!(c.get("run").unwrap().markers.stride.is_some());
            for name in ["slash_fore", "slash_back", "slash_heavy"] {
                let k = c.get(name).unwrap();
                let m = |n: &str| k.mark(n).unwrap_or_else(|| panic!("{hero:?} {name} has no {n}"));
                assert!(
                    0.0 < m("windup_end") && m("windup_end") < m("active_end") && m("active_end") <= m("recover_end"),
                    "{hero:?} {name}"
                );
                assert!((m("recover_end") - k.duration()).abs() < 1e-4, "{hero:?} {name} ends at its last frame");
            }
            for leap in ["jump", "roll"] {
                let j = c.get(leap).unwrap();
                assert!(j.mark("takeoff").unwrap() < j.mark("land").unwrap() && j.mark("land").unwrap() < j.mark("recover_end").unwrap());
            }
            assert!(c.get("block").unwrap().mark("raised").is_some());
        }
    }

    #[test]
    fn each_hero_plays_his_own_clips() {
        // The three sets differ: no two heroes share a strike or an idle (Mehmed's picks are not
        // Ragnar's, Miyamoto's are from the great sword's pack).
        let firsts: Vec<Vec<f32>> = HEROES
            .iter()
            .map(|&h| {
                let c = clips(h).unwrap();
                ["idle", "slash_fore", "slash_back", "slash_heavy"].iter().map(|n| c.get(n).unwrap().duration()).collect()
            })
            .collect();
        for a in 0..3 {
            for b in a + 1..3 {
                for k in 0..4 {
                    assert!((firsts[a][k] - firsts[b][k]).abs() > 1e-3, "{:?} and {:?} share clip {k}", HEROES[a], HEROES[b]);
                }
            }
        }
        assert_eq!(embedded(HeroKind::Mehmed).len(), embedded(HeroKind::Ragnar).len());
    }

    #[test]
    fn the_clips_rest_on_the_joints_the_game_cut_him_at() {
        for hero in HEROES {
            // Out to Blender and back (export_clips.py): every joint is where the rig put it.
            let c = clips(hero).unwrap();
            let rig = crate::render::view::jointed_hero(hero, 1, Some(1)).unwrap();
            for clip in &c.clips {
                for b in 0..BONE_COUNT {
                    let at = clip.skeleton.rest_inv[b].inverse().w_axis.truncate();
                    assert!(
                        at.abs_diff_eq(rig.parts[b].rest, 1e-4),
                        "{hero:?} {}: {} at {at} not {}",
                        clip.name,
                        rig.parts[b].name,
                        rig.parts[b].rest
                    );
                }
            }
            // And sampled at its start, the idle is a pose of him, not of another body: his feet on the floor.
            let joints: Vec<glam::Vec3> = rig.parts.iter().map(|p| p.rest).collect();
            let m = c.skeleton.matrices(&c.get("idle").unwrap().sample(0.0), &joints, &[glam::Quat::IDENTITY; BONE_COUNT]);
            for foot in [13, 16] {
                let y = m[foot].w_axis.y;
                assert!((y - rig.parts[foot].rest.y).abs() < 0.03, "{hero:?}: {} at {y}", rig.parts[foot].name);
            }
        }
    }
}
