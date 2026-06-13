/**
 * Item overrides for stats and passives that Data Dragon's `stats` field
 * does not include (lethality, flat/% magic pen, % armor pen, ability haste,
 * and key damage-relevant passives).
 *
 * Values are for patch 16.x / Season 2026 and need MANUAL updates when
 * Riot ships balance changes.  Cross-check with the LoL Wiki:
 *   https://wiki.leagueoflegends.com/en-us/List_of_items'_stats
 */

export const ITEM_OVERRIDES = {
  // ─────────────────────────────────────────────────────────
  // Lethality items
  // ─────────────────────────────────────────────────────────

  3142: { // Youmuu's Ghostblade
    lethality: 18,
    abilityHaste: 15,
  },
  3814: { // Edge of Night
    lethality: 15,
  },
  6697: { // Hubris
    lethality: 18,
    abilityHaste: 15,
  },
  6701: { // Opportunity
    lethality: 18,
  },
  6676: { // The Collector
    lethality: 18,
  },
  6698: { // Profane Hydra
    lethality: 18,
    abilityHaste: 20,
  },
  6699: { // Voltaic Cyclosword
    lethality: 10,
    abilityHaste: 10,
    passive: {
      type: 'voltaicCyclosword',
      // When fully Energized: +15 lethality for 4 s (melee) / +12 (ranged),
      // and on-hit 9% / 7% current HP physical damage (cap 200 vs non-champs).
      energizedLethalityMelee: 15,
      energizedLethalityRanged: 12,
      energizedCurrentHpMelee: 0.09,
      energizedCurrentHpRanged: 0.07,
    },
  },
  6696: { // Axiom Arc
    lethality: 10,
    abilityHaste: 25,
  },
  6695: { // Serpent's Fang
    lethality: 15,
  },
  3179: { // Umbral Glaive
    lethality: 18,
    abilityHaste: 15,
  },

  // ─────────────────────────────────────────────────────────
  // Magic penetration items
  // ─────────────────────────────────────────────────────────

  3020: { // Sorcerer's Shoes
    flatMagicPen: 20,
  },
  4645: { // Shadowflame
    flatMagicPen: 15,
    passive: {
      type: 'shadowflame',
      // Cinderbloom: 10-20 bonus flat magic pen based on target's current HP
      // (max pen at <= 1000 HP, min pen at >= 2500 HP).
      bonusFlatMagicPenMin: 10,
      bonusFlatMagicPenMax: 20,
      hpFloor: 1000,
      hpCeiling: 2500,
    },
  },
  3135: { // Void Staff
    pctMagicPen: 0.40,
  },
  4005: { // Cryptbloom
    pctMagicPen: 0.30,
    abilityHaste: 20,
  },
  4646: { // Stormsurge
    flatMagicPen: 10,
    passive: {
      type: 'stormsurge',
      // Squall: 100-200 (scales with level) + 50% AP after dealing 25% max HP.
      flatDamageMin: 100,
      flatDamageMax: 200,
      apRatio: 0.50,
      damageType: 'magic',
    },
  },
  6655: { // Luden's Echo
    abilityHaste: 10,
    passive: {
      type: 'ludens',
      // Echo: 6 bolts on first ability hit. Primary = 100 + 10% AP,
      // remaining 5 at 20% each on single target. Total ≈ 200 + 20% AP.
      flatDamage: 200,
      apRatio: 0.20,
      damageType: 'magic',
    },
  },
  2503: { // Blackfire Torch
    abilityHaste: 20,
    passive: {
      type: 'blackfireTorch',
      // Baleful Blaze: 30 + 4% AP per second for 3 seconds.
      flatDamageTotal: 90,
      apRatio: 0.12,
      damageType: 'magic',
    },
  },

  // ─────────────────────────────────────────────────────────
  // Armor penetration items
  // ─────────────────────────────────────────────────────────

  3036: { // Lord Dominik's Regards
    pctArmorPen: 0.35,
  },
  3033: { // Mortal Reminder
    pctArmorPen: 0.30,
  },
  6694: { // Serylda's Grudge
    pctArmorPen: 0.35,
    abilityHaste: 15,
  },
  3071: { // Black Cleaver
    abilityHaste: 20,
    passive: {
      type: 'blackCleaver',
      // Carve: physical damage applies stacks (max 5), each = 6% armor reduction.
      maxStacks: 5,
      armorReductionPerStack: 0.06,
    },
  },

  // ─────────────────────────────────────────────────────────
  // Key passives for damage calculation
  // ─────────────────────────────────────────────────────────

  6653: { // Liandry's Torment
    passive: {
      type: 'liandrys',
      // Torment: 2% target max HP magic damage/s for 3s = 6% total.
      maxHpPerSecond: 0.02,
      duration: 3,
      damageType: 'magic',
    },
  },
  3118: { // Malignance
    abilityHaste: 15,
    passive: {
      type: 'malignance',
      // Hatefog: ult ground burn 60 + 12% AP/s for 3s. Only procs on R.
      flatDamagePerSecond: 60,
      apRatioPerSecond: 0.12,
      duration: 3,
      damageType: 'magic',
      ultOnly: true,
    },
  },
  3089: { // Rabadon's Deathcap
    passive: { type: 'rabadons', apMultiplier: 0.35 },
  },
  3031: { // Infinity Edge
    passive: {
      type: 'infinityEdge',
      critDamageBonus: 0.35,
      critThreshold: 0.40, // requires >= 40% crit chance
    },
  },
  2510: { // Dusk and Dawn
    abilityHaste: 20,
    passive: {
      type: 'spellblade',
      subtype: 'duskAndDawn',
      baseAdRatio: 0.75,
      apRatio: 0.45,
      damageType: 'magic',
    },
  },
  3100: { // Lich Bane
    abilityHaste: 10,
    passive: {
      type: 'spellblade',
      subtype: 'lichBane',
      baseAdRatio: 0.75,
      apRatio: 0.45,
      damageType: 'magic',
    },
  },
  3078: { // Trinity Force
    abilityHaste: 15,
    passive: {
      type: 'spellblade',
      subtype: 'trinityForce',
      baseAdRatio: 2.0,
      damageType: 'physical',
    },
  },
  3057: { // Sheen
    passive: {
      type: 'spellblade',
      subtype: 'sheen',
      baseAdRatio: 1.0,
      damageType: 'physical',
    },
  },
  3115: { // Nashor's Tooth
    abilityHaste: 15,
    passive: {
      type: 'nashors',
      flatOnHit: 15,
      apRatio: 0.15,
      damageType: 'magic',
    },
  },
  3153: { // Blade of the Ruined King
    passive: {
      type: 'botrk',
      // Mist's Edge: on-hit 9% current HP (melee) / 6% (ranged), physical.
      currentHpRatioMelee: 0.09,
      currentHpRatioRanged: 0.06,
      damageType: 'physical',
    },
  },
  3091: { // Wit's End
    passive: {
      type: 'witsEnd',
      // On-hit 15-80 magic damage, scales linearly with champion level 1-18.
      flatOnHitMin: 15,
      flatOnHitMax: 80,
      damageType: 'magic',
    },
  },

  // ─────────────────────────────────────────────────────────
  // Additional ability haste items
  // (AH is missing from Data Dragon for all of these)
  // ─────────────────────────────────────────────────────────

  3158: { // Ionian Boots of Lucidity
    abilityHaste: 15,
  },
  3508: { // Essence Reaver
    abilityHaste: 25,
    passive: {
      type: 'spellblade',
      subtype: 'essenceReaver',
      baseAdRatio: 1.0,
      damageType: 'physical',
    },
  },
  4629: { // Cosmic Drive
    abilityHaste: 30,
  },
  6672: { // Kraken Slayer
    passive: {
      type: 'kraken',
      procEvery: 3,
      flatDamageMin: 35,
      flatDamageMax: 75,
      bonusAdRatio: 0.65,
      damageType: 'true',
    },
  },
  3152: { // Hextech Rocketbelt
    abilityHaste: 15,
    active: {
      name: 'Rocketbelt',
      flatDamage: 125,
      apRatio: 0.15,
      damageType: 'magic',
    },
  },
  3165: { // Morellonomicon
    abilityHaste: 15,
  },
  3157: { // Zhonya's Hourglass
    abilityHaste: 10,
  },
  3102: { // Banshee's Veil
    abilityHaste: 10,
  },
  3003: { // Archangel's Staff / Seraph's Embrace
    abilityHaste: 25,
  },
  3004: { // Manamune / Muramana
    abilityHaste: 15,
  },
  3161: { // Spear of Shojin
    abilityHaste: 20,
  },
  3074: { // Ravenous Hydra
    abilityHaste: 20,
  },
  6630: { // Sundered Sky
    abilityHaste: 15,
  },
  6333: { // Death's Dance
    abilityHaste: 15,
  },
  3156: { // Maw of Malmortius
    abilityHaste: 15,
  },
  4633: { // Riftmaker
    abilityHaste: 15,
  },
  3146: { // Hextech Gunblade
    active: {
      name: 'Gunblade',
      flatDamageMin: 175,
      flatDamageMax: 250,
      apRatio: 0.30,
      damageType: 'magic',
    },
  },
  3110: { // Frozen Heart
    abilityHaste: 20,
  },
  6662: { // Iceborn Gauntlet
    abilityHaste: 15,
  },
  3065: { // Spirit Visage
    abilityHaste: 10,
  },
  6665: { // Jak'Sho, The Protean
    abilityHaste: 10,
  },
  3001: { // Abyssal Mask
    abilityHaste: 10,
  },
  2065: { // Shurelya's Battlesong
    abilityHaste: 20,
  },
  6617: { // Moonstone Renewer
    abilityHaste: 20,
  },
  3190: { // Locket of the Iron Solari
    abilityHaste: 15,
  },
  3504: { // Ardent Censer
    abilityHaste: 15,
  },
  3222: { // Mikael's Blessing
    abilityHaste: 15,
  },
  6616: { // Staff of Flowing Water
    abilityHaste: 15,
  },
  3050: { // Zeke's Convergence
    abilityHaste: 15,
  },
  6657: { // Rod of Ages
    abilityHaste: 15,
  },
  6664: { // Hollow Radiance
    abilityHaste: 10,
  },
};
