//! Every model the view draws, built once: the heroes, the foes, props, keys, the portal doors
//! and the story's cast. (Coarser copies for far away come from `Rig::lod`, as the camera calls for them.)

use super::*;

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
    Tengu,
    Rasetsu,
    DireWolf,
    Jorogumo,
    Cobra,
    Kappa,
    Rokurokubi,
    Kitsune,
    Oni,
    Tulukus,
    Albasti,
    Cin,
    Arcura,
    Karakoncolos,
    FrostGiant,
    Surtr,
    Nidhogg,
    Hel,
    Tepegoz,
    Gashadokuro,
    Ejder,
    Goryo,
    /// The realms' own (the Peerless is drawn as the hero).
    Ganglot,
    Gaki,
    Ellepige,
    /// The courts' own: Mammon and his gold golems.
    Mammon,
    GoldGolem,
    // Pride's court
    Lucifer,
    Reflection,
    // Envy's court
    Leviathan,
    Serpentling,
    // Wrath's court
    Amon,
    Raven,
    // Sloth's court
    Belphegor,
    Nightmare,
    // Gluttony's court
    Beelzebub,
    FlySwarm,
    // Lust's court
    Asmodeus,
}

pub(super) struct Models {
    pub(super) ragnar: Rig,
    pub(super) skeleton: Rig,
    pub(super) bat: Rig,
    pub(super) vampire: Rig,
    pub(super) slime: Rig,
    pub(super) witch: Rig,
    pub(super) minion: Rig,
    pub(super) helmeted: Rig,
    pub(super) werewolf: Rig,
    pub(super) blemmyes: Rig,
    pub(super) large_bat: Rig,
    /// The third stage's five.
    pub(super) tengu: Rig,
    pub(super) rasetsu: Rig,
    pub(super) dire_wolf: Rig,
    pub(super) jorogumo: Rig,
    pub(super) cobra: Rig,
    /// The fourth stage's four.
    pub(super) kappa: Rig,
    pub(super) rokurokubi: Rig,
    pub(super) kitsune: Rig,
    pub(super) oni: Rig,
    /// The fifth stage's five, and the seventh's four.
    pub(super) tulukus: Rig,
    pub(super) albasti: Rig,
    pub(super) cin: Rig,
    pub(super) arcura: Rig,
    pub(super) karakoncolos: Rig,
    pub(super) frost_giant: Rig,
    pub(super) surtr: Rig,
    pub(super) nidhogg: Rig,
    pub(super) hel: Rig,
    /// The sixth stage's four.
    pub(super) tepegoz: Rig,
    pub(super) gashadokuro: Rig,
    pub(super) ejder: Rig,
    pub(super) goryo: Rig,
    /// The realms' own: Hel's servants, the gaki, the elf-maids.
    pub(super) ganglot: Rig,
    pub(super) gaki: Rig,
    pub(super) ellepige: Rig,
    /// The courts' own: Mammon and his gold golems.
    pub(super) mammon: Rig,
    pub(super) gold_golem: Rig,
    /// The six other princes and what they raise (`foes_court`, a module per sin).
    pub(super) lucifer: Rig,
    pub(super) reflection: Rig,
    pub(super) leviathan: Rig,
    pub(super) serpentling: Rig,
    pub(super) amon: Rig,
    pub(super) raven: Rig,
    pub(super) belphegor: Rig,
    pub(super) nightmare: Rig,
    pub(super) beelzebub: Rig,
    pub(super) fly_swarm: Rig,
    pub(super) asmodeus: Rig,
    pub(super) medkit: Mesh,
    pub(super) bomb: Mesh,
    pub(super) arrow: Mesh,
    pub(super) boomerang: Mesh,
    pub(super) fireball: Mesh,
    pub(super) crescent: Mesh,
    pub(super) diamond: Mesh,
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
    pub(super) hero_key: (HeroKind, forge::Kit),
    /// The hero without the shield on his back, and the shield itself: drawn while he holds it up.
    pub(super) raised: Option<(Rig, Rig)>,
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
        use crate::render::foes2::{
            blemmyes, helmeted_skeleton, large_bat, loup_garou, minion, witch, BAT_RIG, BLEMMYES_RIG, HELMET_RIG, WITCH_RIG, WOLF_RIG,
        };
        let witch = build_rig(&witch(), WITCH_RIG, 0.0125);
        let minion = build_rig(&minion(), &[], 0.014);
        let helmeted = build_rig(&helmeted_skeleton(), HELMET_RIG, 0.0125);
        let werewolf = build_rig(&loup_garou(), WOLF_RIG, 0.0135);
        let blemmyes = build_rig(&blemmyes(), BLEMMYES_RIG, 0.0145);
        let large_bat = build_rig(&large_bat(), BAT_RIG, 0.011);
        use crate::render::foes3::{cobra, dire_wolf, jorogumo, rasetsu, tengu, COBRA_RIG, DIRE_RIG, JORO_RIG, RASETSU_RIG, TENGU_RIG};
        let tengu = build_rig(&tengu(), TENGU_RIG, 0.0125);
        let rasetsu = build_rig(&rasetsu(), RASETSU_RIG, 0.0125);
        let dire_wolf = build_rig(&dire_wolf(), DIRE_RIG, 0.0125);
        let jorogumo = build_rig(&jorogumo(), JORO_RIG, 0.0125);
        let cobra = build_rig(&cobra(), COBRA_RIG, 0.0125);
        use crate::render::foes4::{kappa, kitsune, oni, rokurokubi, KAPPA_RIG, KITSUNE_RIG, ONI_RIG, ROKURO_RIG};
        let kappa = build_rig(&kappa(), KAPPA_RIG, 0.0125);
        let rokurokubi = build_rig(&rokurokubi(), ROKURO_RIG, 0.0125);
        let kitsune = build_rig(&kitsune(), KITSUNE_RIG, 0.0125);
        let oni = build_rig(&oni(), ONI_RIG, 0.0135);
        use crate::render::foes5::{
            albasti, arcura, cin, karakoncolos, tulukus, ALBASTI_RIG, ARCURA_RIG, CIN_RIG, KARAKONCOLOS_RIG, TULUKUS_RIG,
        };
        let tulukus = build_rig(&tulukus(), TULUKUS_RIG, 0.0125);
        let albasti = build_rig(&albasti(), ALBASTI_RIG, 0.0125);
        let cin = build_rig(&cin(), CIN_RIG, 0.0125);
        let arcura = build_rig(&arcura(), ARCURA_RIG, 0.0125);
        let karakoncolos = build_rig(&karakoncolos(), KARAKONCOLOS_RIG, 0.013);
        use crate::render::foes7::{frost_giant, hel, nidhogg, surtr, FROST_GIANT_RIG, HEL_RIG, NIDHOGG_RIG, SURTR_RIG};
        let frost_giant = build_rig(&frost_giant(), FROST_GIANT_RIG, 0.02);
        let surtr = build_rig(&surtr(), SURTR_RIG, 0.02);
        let nidhogg = build_rig(&nidhogg(), NIDHOGG_RIG, 0.0175);
        let hel = build_rig(&hel(), HEL_RIG, 0.0135);
        use crate::render::foes6::{ejder, gashadokuro, goryo, tepegoz, EJDER_RIG, GASHADOKURO_RIG, GORYO_RIG, TEPEGOZ_RIG};
        let tepegoz = build_rig(&tepegoz(), TEPEGOZ_RIG, 0.02);
        let gashadokuro = build_rig(&gashadokuro(), GASHADOKURO_RIG, 0.025);
        let ejder = build_rig(&ejder(), EJDER_RIG, 0.018);
        let goryo = build_rig(&goryo(), GORYO_RIG, 0.0125);
        let ganglot = {
            use crate::render::foes_mire::{ganglot, GANGLOT_RIG, GANGLOT_VOXEL};
            build_rig(&ganglot(), GANGLOT_RIG, GANGLOT_VOXEL)
        };
        let gaki = {
            use crate::render::foes_feast::{gaki, GAKI_RIG, GAKI_VOXEL};
            build_rig(&gaki(), GAKI_RIG, GAKI_VOXEL)
        };
        let ellepige = {
            use crate::render::foes_thorns::{ellepige, ELLEPIGE_RIG, ELLEPIGE_VOXEL};
            build_rig(&ellepige(), ELLEPIGE_RIG, ELLEPIGE_VOXEL)
        };
        let (mammon, gold_golem) = {
            use crate::render::foes_court::{gold_golem, mammon, GOLEM_RIG, GOLEM_VOXEL, MAMMON_RIG, MAMMON_VOXEL};
            (build_rig(&mammon(), MAMMON_RIG, MAMMON_VOXEL), build_rig(&gold_golem(), GOLEM_RIG, GOLEM_VOXEL))
        };
        // Pride's court
        let (lucifer, reflection) = {
            use crate::render::foes_court::pride::*;
            (build_rig(&lucifer(), LUCIFER_RIG, LUCIFER_VOXEL), build_rig(&reflection(), REFLECTION_RIG, REFLECTION_VOXEL))
        };
        // Envy's court
        let (leviathan, serpentling) = {
            use crate::render::foes_court::envy::*;
            (build_rig(&leviathan(), LEVIATHAN_RIG, LEVIATHAN_VOXEL), build_rig(&serpentling(), SERPENTLING_RIG, SERPENTLING_VOXEL))
        };
        // Wrath's court
        let (amon, raven) = {
            use crate::render::foes_court::wrath::*;
            (build_rig(&amon(), AMON_RIG, AMON_VOXEL), build_rig(&raven(), RAVEN_RIG, RAVEN_VOXEL))
        };
        // Sloth's court
        let (belphegor, nightmare) = {
            use crate::render::foes_court::sloth::*;
            (build_rig(&belphegor(), BELPHEGOR_RIG, BELPHEGOR_VOXEL), build_rig(&nightmare(), NIGHTMARE_RIG, NIGHTMARE_VOXEL))
        };
        // Gluttony's court
        let (beelzebub, fly_swarm) = {
            use crate::render::foes_court::gluttony::*;
            (build_rig(&beelzebub(), BEELZEBUB_RIG, BEELZEBUB_VOXEL), build_rig(&fly_swarm(), FLY_SWARM_RIG, FLY_SWARM_VOXEL))
        };
        // Lust's court
        let asmodeus = {
            use crate::render::foes_court::lust::*;
            build_rig(&asmodeus(), ASMODEUS_RIG, ASMODEUS_VOXEL)
        };
        // The played hero and his shield raised, built together (the raised pair is only remade
        // when his kit changes, so it must be made here too, or he blocks with a bare fist).
        let (hero, raised) = forge::hero_rigs(ragnar, forge::Kit::plain(1, Some(1)));
        Self {
            ragnar: forge::hero_model(ragnar, 1, Some(1)),
            hero,
            hero_key: (ragnar, forge::Kit::plain(1, Some(1))),
            raised,
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
            tengu,
            rasetsu,
            dire_wolf,
            jorogumo,
            cobra,
            kappa,
            rokurokubi,
            kitsune,
            oni,
            tulukus,
            albasti,
            cin,
            arcura,
            karakoncolos,
            frost_giant,
            surtr,
            nidhogg,
            hel,
            tepegoz,
            gashadokuro,
            ejder,
            goryo,
            ganglot,
            gaki,
            ellepige,
            mammon,
            gold_golem,
            lucifer,
            reflection,
            leviathan,
            serpentling,
            amon,
            raven,
            belphegor,
            nightmare,
            beelzebub,
            fly_swarm,
            asmodeus,
            medkit: crate::render::props::medkit(),
            bomb: crate::render::props::bomb(),
            arrow: crate::render::props::arrow(),
            boomerang: crate::render::props::boomerang(),
            fireball: crate::render::props::fireball(),
            crescent: crate::render::props::crescent(),
            diamond: crate::render::props::gem(),
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
            RigId::Tengu => &self.tengu,
            RigId::Rasetsu => &self.rasetsu,
            RigId::DireWolf => &self.dire_wolf,
            RigId::Jorogumo => &self.jorogumo,
            RigId::Cobra => &self.cobra,
            RigId::Kappa => &self.kappa,
            RigId::Rokurokubi => &self.rokurokubi,
            RigId::Kitsune => &self.kitsune,
            RigId::Oni => &self.oni,
            RigId::Tulukus => &self.tulukus,
            RigId::Albasti => &self.albasti,
            RigId::Cin => &self.cin,
            RigId::Arcura => &self.arcura,
            RigId::Karakoncolos => &self.karakoncolos,
            RigId::FrostGiant => &self.frost_giant,
            RigId::Surtr => &self.surtr,
            RigId::Nidhogg => &self.nidhogg,
            RigId::Hel => &self.hel,
            RigId::Tepegoz => &self.tepegoz,
            RigId::Gashadokuro => &self.gashadokuro,
            RigId::Ejder => &self.ejder,
            RigId::Goryo => &self.goryo,
            RigId::Ganglot => &self.ganglot,
            RigId::Gaki => &self.gaki,
            RigId::Ellepige => &self.ellepige,
            RigId::Mammon => &self.mammon,
            RigId::GoldGolem => &self.gold_golem,
            RigId::Lucifer => &self.lucifer,
            RigId::Reflection => &self.reflection,
            RigId::Leviathan => &self.leviathan,
            RigId::Serpentling => &self.serpentling,
            RigId::Amon => &self.amon,
            RigId::Raven => &self.raven,
            RigId::Belphegor => &self.belphegor,
            RigId::Nightmare => &self.nightmare,
            RigId::Beelzebub => &self.beelzebub,
            RigId::FlySwarm => &self.fly_swarm,
            RigId::Asmodeus => &self.asmodeus,
        }
    }
}
