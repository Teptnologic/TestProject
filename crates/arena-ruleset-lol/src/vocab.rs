//! Every name the ruleset registers, resolved to ids once.

use arena_kernel::*;

pub struct Vocab {
    // Stats.
    pub max_hp: StatId,
    pub hp_regen: StatId,
    pub max_mana: StatId,
    pub mana_regen: StatId,
    pub ad: StatId,
    pub ap: StatId,
    pub armor: StatId,
    pub mr: StatId,
    /// Bonus attack speed as a fraction (0.35 is +35%).
    pub attack_speed: StatId,
    pub crit_chance: StatId,
    pub crit_damage: StatId,
    pub move_speed: StatId,
    pub attack_range: StatId,
    pub lethality: StatId,
    pub armor_pen: StatId,
    pub magic_pen: StatId,
    pub magic_pen_pct: StatId,
    pub life_steal: StatId,
    pub omnivamp: StatId,
    pub ability_haste: StatId,
    pub tenacity: StatId,
    pub damage_amp: StatId,
    pub damage_reduction: StatId,
    pub heal_shield_power: StatId,
    // Resources.
    pub hp: ResourceId,
    pub mana: ResourceId,
    // Flags.
    pub stunned: FlagId,
    pub rooted: FlagId,
    pub silenced: FlagId,
    pub disarmed: FlagId,
    pub untargetable: FlagId,
    pub invulnerable: FlagId,
    pub grievous: FlagId,
    // Tags.
    pub basic_attack: TagId,
    pub ability: TagId,
    pub ultimate: TagId,
    pub physical: TagId,
    pub magic: TagId,
    pub true_damage: TagId,
    pub proc_: TagId,
    pub shield: TagId,
    pub slow: TagId,
    // Events.
    pub ev_damage: EventKind,
    pub ev_on_hit: EventKind,
    pub ev_ability_cast: EventKind,
    pub ev_healed: EventKind,
    pub ev_cc: EventKind,
    // Custom effects.
    pub fx_damage: CustomId,
    pub fx_heal: CustomId,
    pub fx_shield: CustomId,
    pub fx_stun: CustomId,
    pub fx_root: CustomId,
    pub fx_silence: CustomId,
    pub fx_slow: CustomId,
    pub fx_grievous: CustomId,
    // Built-in buffs.
    pub b_stun: BuffId,
    pub b_root: BuffId,
    pub b_silence: BuffId,
    pub b_slow: BuffId,
    pub b_shield: BuffId,
    pub b_grievous: BuffId,
}

impl Vocab {
    pub fn register(defs: &mut Defs) -> Vocab {
        let r = &mut defs.reg;
        let max_hp = r.stat("max_hp");
        let hp_regen = r.stat("hp_regen");
        let max_mana = r.stat("max_mana");
        let mana_regen = r.stat("mana_regen");
        // Regeneration stats are quoted per 5 seconds.
        let pool = |name: &str, max_stat, regen| ResourceDef {
            name: name.to_string(),
            max_stat,
            regen_stat: Some(regen),
            regen_per_sec: 0.2,
        };
        let hp = r.resource(pool("hp", max_hp, hp_regen));
        let mana = r.resource(pool("mana", max_mana, mana_regen));
        let (stunned, rooted, silenced) = (r.flag("stunned"), r.flag("rooted"), r.flag("silenced"));
        let grievous = r.flag("grievous_wounds");
        let (shield, slow) = (r.tag("shield"), r.tag("slow"));
        for tag in [
            "spell",
            "aoe",
            "dot",
            "on_hit",
            "melee",
            "ranged",
            "structure",
            "summoner",
        ] {
            r.tag(tag);
        }

        let mut buff = |name: &str, flags: &[FlagId], tags: &[TagId], policy| {
            let mut b = BuffDef::new(name);
            b.flags = flags.to_vec();
            b.tags = tags.to_vec();
            b.policy = policy;
            defs.add_buff(b)
        };
        // Independent instances: the longest one decides when the effect ends.
        let b_stun = buff("lol_stun", &[stunned], &[], StackPolicy::Independent);
        let b_root = buff("lol_root", &[rooted], &[], StackPolicy::Independent);
        let b_silence = buff("lol_silence", &[silenced], &[], StackPolicy::Independent);
        let b_slow = buff("lol_slow", &[], &[slow], StackPolicy::Independent);
        let b_shield = buff("lol_shield", &[], &[shield], StackPolicy::Independent);
        let b_grievous = buff("lol_grievous", &[grievous], &[], StackPolicy::Refresh);

        let r = &mut defs.reg;
        Vocab {
            max_hp,
            hp_regen,
            max_mana,
            mana_regen,
            ad: r.stat("ad"),
            ap: r.stat("ap"),
            armor: r.stat("armor"),
            mr: r.stat("mr"),
            attack_speed: r.stat("attack_speed"),
            crit_chance: r.stat("crit_chance"),
            crit_damage: r.stat("crit_damage"),
            move_speed: r.stat("move_speed"),
            attack_range: r.stat("attack_range"),
            lethality: r.stat("lethality"),
            armor_pen: r.stat("armor_pen"),
            magic_pen: r.stat("magic_pen"),
            magic_pen_pct: r.stat("magic_pen_pct"),
            life_steal: r.stat("life_steal"),
            omnivamp: r.stat("omnivamp"),
            ability_haste: r.stat("ability_haste"),
            tenacity: r.stat("tenacity"),
            damage_amp: r.stat("damage_amp"),
            damage_reduction: r.stat("damage_reduction"),
            heal_shield_power: r.stat("heal_shield_power"),
            hp,
            mana,
            stunned,
            rooted,
            silenced,
            disarmed: r.flag("disarmed"),
            untargetable: r.flag("untargetable"),
            invulnerable: r.flag("invulnerable"),
            grievous,
            basic_attack: r.tag("basic_attack"),
            ability: r.tag("ability"),
            ultimate: r.tag("ultimate"),
            physical: r.tag("physical"),
            magic: r.tag("magic"),
            true_damage: r.tag("true"),
            proc_: r.tag("proc"),
            shield,
            slow,
            ev_damage: r.event("damage_dealt"),
            ev_on_hit: r.event("on_hit"),
            ev_ability_cast: r.event("ability_cast"),
            ev_healed: r.event("healed"),
            ev_cc: r.event("cc_applied"),
            fx_damage: r.custom("damage", &["amount", "crit"]),
            fx_heal: r.custom("heal", &["amount"]),
            fx_shield: r.custom("shield", &["amount", "duration"]),
            fx_stun: r.custom("stun", &["duration"]),
            fx_root: r.custom("root", &["duration"]),
            fx_silence: r.custom("silence", &["duration"]),
            fx_slow: r.custom("slow", &["amount", "duration"]),
            fx_grievous: r.custom("grievous", &["duration"]),
            b_stun,
            b_root,
            b_silence,
            b_slow,
            b_shield,
            b_grievous,
        }
    }
}
