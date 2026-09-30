//! Every model the view draws, built once: the heroes, the foes (and coarse copies for far
//! away), props, keys, the portal doors and the story's cast.

use super::*;

/// Past this distance from the camera a foe is drawn from its coarse copy: its fine voxels
/// would be a pixel or two across there, and a dungeon can hold a dozen foes.
const FOE_DETAIL_DISTANCE: f32 = 6.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RigId {
    Skeleton,
    Bat,
    Vampire,
    Slime,
    Witch,
    Minion,
    Helmeted,
    Werewolf,
    Blemmyes,
    LargeBat,
}

pub(super) struct Models {
    pub(super) ragnar: Rig,
    pub(super) skeleton: Rig,
    pub(super) bat: Rig,
    pub(super) vampire: Rig,
    pub(super) slime: Rig,
    /// The same four at half the resolution, for foes far from the camera.
    pub(super) witch: Rig,
    pub(super) minion: Rig,
    pub(super) helmeted: Rig,
    pub(super) werewolf: Rig,
    pub(super) blemmyes: Rig,
    pub(super) large_bat: Rig,
    /// Every foe's coarse copy, in `RigId` order.
    pub(super) foes_far: Vec<Rig>,
    pub(super) medkit: Mesh,
    pub(super) bomb: Mesh,
    pub(super) arrow: Mesh,
    pub(super) boomerang: Mesh,
    pub(super) fireball: Mesh,
    pub(super) crescent: Mesh,
    pub(super) diamond: Mesh,
    /// The vaults' gold: a stack of coins, a heap of them, a chest (body and lid).
    pub(super) coin: Mesh,
    pub(super) pile: Mesh,
    pub(super) chest: (Mesh, Mesh),
    pub(super) tree: Mesh,
    pub(super) key: Mesh,
    /// The Cyclops' red diamond key.
    /// The diamond keys the bosses drop: the Cyclops' red one, then one in each sin's colour.
    pub(super) keys: Vec<Mesh>,
    /// The meadow's hedge ring, its bushes, and the forest's darker bushes: four shapes of each.
    pub(super) hedges: Vec<Mesh>,
    pub(super) bushes: Vec<Mesh>,
    pub(super) forest_bushes: Vec<Mesh>,
    /// The seven portal doors of the Portals of Hell, each a pair of leaves carrying its sin in relief,
    /// and the sin's name in letters for the lintel.
    pub(super) portal_doors: Vec<[Mesh; 2]>,
    pub(super) portal_names: Vec<Mesh>,
    /// Shrine pedestals: weapon (blue runes) and item (gold runes).
    pub(super) pedestals: [(Mesh, Mesh); 2],
    pub(super) bottles: HashMap<u32, Mesh>,
    pub(super) cyclops: Rig,
    pub(super) boulder: Rig,
    /// The hero as he is equipped now (weapon in hand, shield on his back), what that is,
    /// and his weapon on its own for when he throws it. Rebuilt when his gear changes.
    pub(super) hero: Rig,
    pub(super) hero_key: (HeroKind, u8, Option<u8>),
    pub(super) thrown: Rig,
    /// The other two heroes as they are shown on the title and character screens, as
    /// souls in Scene I and as ancestral spirits (Ragnar is `ragnar`).
    pub(super) mehmed: Rig,
    pub(super) miyamoto: Rig,
    /// Mehmed's and Miyamoto's ancestral spirits: the same at half the resolution, see-through.
    pub(super) spirits: [Rig; 2],
    /// An archangel's shrine as it honours each of the four (plinth and statue).
    pub(super) angel_shrines: [Rig; 4],
    /// Scene I's cast: the four friends, the old gatekeeper, his dog, and Krampus as a shadow.
    pub(super) kids: [Rig; 4],
    pub(super) old_man: Rig,
    pub(super) dog: Rig,
    pub(super) krampus: Rig,
}

impl Models {
    pub(super) fn new() -> Self {
        let mut bottles = HashMap::new();
        for hero in ROSTER {
            for kind in ALL_POTIONS {
                let hex = gear::potion(hero, kind).color;
                bottles.entry(hex).or_insert_with(|| crate::render::props::bottle(hex));
            }
        }
        let ragnar = HeroKind::Ragnar;
        use crate::render::foes::{bog_slime, draugr, raven, DRAUGR_RIG, RAVEN_RIG};
        let skeleton = build_rig(&draugr(), DRAUGR_RIG, 0.0125);
        let bat = build_rig(&raven(false), RAVEN_RIG, 0.0105);
        let vampire = build_rig(&raven(true), RAVEN_RIG, 0.0125);
        let slime = build_rig(&bog_slime(), &[], 0.01875);
        use crate::render::foes2::{blemmyes, helmeted_skeleton, large_bat, loup_garou, minion, witch, BAT_RIG, BLEMMYES_RIG, HELMET_RIG, WITCH_RIG, WOLF_RIG};
        let witch = build_rig(&witch(), WITCH_RIG, 0.0125);
        let minion = build_rig(&minion(), &[], 0.014);
        let helmeted = build_rig(&helmeted_skeleton(), HELMET_RIG, 0.0125);
        let werewolf = build_rig(&loup_garou(), WOLF_RIG, 0.0135);
        let blemmyes = build_rig(&blemmyes(), BLEMMYES_RIG, 0.0145);
        let large_bat = build_rig(&large_bat(), BAT_RIG, 0.011);
        let foes_far = vec![
            skeleton.coarse(),
            bat.coarse(),
            vampire.coarse(),
            slime.coarse(),
            witch.coarse(),
            minion.coarse(),
            helmeted.coarse(),
            werewolf.coarse(),
            blemmyes.coarse(),
            large_bat.coarse(),
        ];
        Self {
            ragnar: forge::hero_model(ragnar, 1, Some(1)),
            hero: forge::hero_model(ragnar, 1, Some(1)),
            hero_key: (ragnar, 1, Some(1)),
            thrown: forge::thrown_model(ragnar, 1),
            skeleton,
            bat,
            vampire,
            slime,
            witch,
            minion,
            helmeted,
            werewolf,
            blemmyes,
            large_bat,
            foes_far,
            medkit: crate::render::props::medkit(),
            bomb: crate::render::props::bomb(),
            arrow: crate::render::props::arrow(),
            boomerang: crate::render::props::boomerang(),
            fireball: crate::render::props::fireball(),
            crescent: crate::render::props::crescent(),
            diamond: crate::render::props::gem(),
            coin: vault::coin(),
            pile: vault::pile(),
            chest: vault::chest(),
            tree: crate::render::props::pine(),
            key: gate::golden_key(),
            keys: std::iter::once(gate::RED_GEM).chain(SINS.iter().map(|s| s.color())).map(gate::diamond_key).collect(),
            hedges: (0..4).map(foliage::hedge_block).collect(),
            bushes: (0..4).map(|v| foliage::bush(v, MEADOW)).collect(),
            forest_bushes: (0..4).map(|v| foliage::small_bush(v, FOREST)).collect(),
            portal_doors: SINS.iter().map(|&s| hell::portal_doors(s)).collect(),
            portal_names: SINS.iter().map(|&s| hell::carved_name(s.title())).collect(),
            pedestals: [crate::render::props::pedestal(0x6ab8ff), crate::render::props::pedestal(0xffc84a)],
            bottles,
            cyclops: {
                use crate::render::foes::{cyclops, cyclops_iris, cyclops_lid, CYCLOPS_EYE, CYCLOPS_RIG, CYCLOPS_VOXEL};
                build_layered_rig(
                    &cyclops(),
                    CYCLOPS_RIG,
                    &[("iris", &cyclops_iris(), CYCLOPS_EYE), ("lid", &cyclops_lid(), CYCLOPS_EYE)],
                    CYCLOPS_VOXEL,
                )
            },
            boulder: build_rig(&meadow::boulder(), &[], 0.09),
            mehmed: forge::hero_model(HeroKind::Mehmed, 1, Some(1)),
            miyamoto: forge::hero_model(HeroKind::Miyamoto, 1, Some(1)),
            spirits: [HeroKind::Mehmed, HeroKind::Miyamoto].map(|h| forge::hero_model(h, 1, Some(1)).coarse()),
            angel_shrines: ARCHANGELS.map(|a| build_rig(&angels::angel_shrine(a), &[], angels::SHRINE_VOXEL)),
            kids: [0, 1, 2, 3].map(|i| build_rig(&crate::render::cast::kid(i), crate::render::cast::KID_RIG, story::KID_VOXEL)),
            old_man: build_rig(&crate::render::cast::old_man(), crate::render::cast::OLD_MAN_RIG, story::OLD_MAN_VOXEL),
            dog: build_rig(&crate::render::cast::dog(), crate::render::cast::DOG_RIG, story::DOG_VOXEL),
            krampus: build_rig(&crate::render::cast::krampus_shade(), crate::render::cast::KRAMPUS_RIG, story::KRAMPUS_VOXEL),
        }
    }

    pub(super) fn rig(&self, id: RigId) -> &Rig {
        match id {
            RigId::Skeleton => &self.skeleton,
            RigId::Bat => &self.bat,
            RigId::Vampire => &self.vampire,
            RigId::Slime => &self.slime,
            RigId::Witch => &self.witch,
            RigId::Minion => &self.minion,
            RigId::Helmeted => &self.helmeted,
            RigId::Werewolf => &self.werewolf,
            RigId::Blemmyes => &self.blemmyes,
            RigId::LargeBat => &self.large_bat,
        }
    }

    /// A foe as it is drawn from `distance` away: in full detail near the camera, coarser far off.
    pub(super) fn foe_at(&self, id: RigId, distance: f32) -> &Rig {
        if distance < FOE_DETAIL_DISTANCE {
            return self.rig(id);
        }
        &self.foes_far[id as usize]
    }
}
