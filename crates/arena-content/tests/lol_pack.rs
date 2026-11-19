//! Loads the real LoL pack and checks that broken content is rejected with a useful message.

use arena_content::load_pack;
use arena_kernel::Defs;
use std::fs;
use std::path::{Path, PathBuf};

fn pack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/lol")
}

#[test]
fn the_lol_pack_loads() {
    let mut defs = Defs::default();
    let ruleset = arena_ruleset_lol::build(&mut defs);
    let pack =
        load_pack(&pack_dir(), &mut defs, ruleset.as_ref()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(pack.id, "lol");
    for unit in ["garen", "ashe", "annie", "nexus"] {
        assert!(defs.unit_id(unit).is_some(), "missing unit {unit}");
    }
    // Numbers come from the generated layer, behaviour from the authored one.
    let garen = defs.unit(defs.unit_id("garen").unwrap());
    let max_hp = defs.reg.find_stat("max_hp").unwrap();
    assert!(garen.stats.iter().any(|(s, v)| *s == max_hp && *v > 0.0));
    assert_eq!(
        garen
            .slots
            .iter()
            .map(|s| s.key.as_str())
            .collect::<Vec<_>>(),
        ["A", "Q", "W", "E", "R"]
    );
    assert!(defs.perk(defs.perk_id("infinity_edge").unwrap()).cost > 0.0);
    // At level 6: one point in the ultimate, three in the first priority, one each in the rest.
    assert_eq!(ruleset.default_ranks(&defs, garen, 6), vec![1, 1, 1, 3, 1]);
}

/// Copies the pack's metadata and writes one authored unit file.
fn broken_pack(name: &str, unit_yaml: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("arena-content-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("authored/units")).unwrap();
    fs::copy(pack_dir().join("pack.yaml"), dir.join("pack.yaml")).unwrap();
    fs::write(dir.join("authored/units/broken.yaml"), unit_yaml).unwrap();
    dir
}

fn load_error(name: &str, unit_yaml: &str) -> String {
    let dir = broken_pack(name, unit_yaml);
    let mut defs = Defs::default();
    let ruleset = arena_ruleset_lol::build(&mut defs);
    let err = load_pack(&dir, &mut defs, ruleset.as_ref())
        .unwrap_err()
        .to_string();
    let _ = fs::remove_dir_all(&dir);
    err
}

#[test]
fn broken_content_fails_at_load_with_file_and_place() {
    let unit = |action_body: &str| {
        format!("id: broken\nradius: 50\nactions:\n  - id: broken_hit\n    slot: A\n{action_body}")
    };
    let err = load_error("formula", &unit("    range: \"attack_rnage\"\n"));
    assert!(
        err.contains("broken.yaml") && err.contains("action broken_hit"),
        "{err}"
    );
    assert!(err.contains("unknown variable `attack_rnage`"), "{err}");

    let err = load_error(
        "effect",
        &unit("    on_execute:\n      - explode: { amount: 5 }\n"),
    );
    assert!(err.contains("unknown effect `explode`"), "{err}");

    let err = load_error(
        "field",
        &unit("    on_execute:\n      - damage: { amuont: 5 }\n"),
    );
    assert!(err.contains("unknown field `amuont`"), "{err}");

    let err = load_error(
        "buff",
        &unit("    on_execute:\n      - apply_buff: { buff: nope }\n"),
    );
    assert!(err.contains("unknown buff `nope`"), "{err}");

    let err = load_error("key", &unit("    cooldwon: 3\n"));
    assert!(err.contains("cooldwon"), "{err}");
}

#[test]
fn a_pack_for_another_ruleset_version_is_rejected() {
    let dir = broken_pack("version", "id: broken\nradius: 50\n");
    fs::write(
        dir.join("pack.yaml"),
        "id: lol\nruleset: lol\nruleset_version: \"9.9\"\n",
    )
    .unwrap();
    let mut defs = Defs::default();
    let ruleset = arena_ruleset_lol::build(&mut defs);
    let err = load_pack(&dir, &mut defs, ruleset.as_ref())
        .unwrap_err()
        .to_string();
    let _ = fs::remove_dir_all(&dir);
    assert!(err.contains("targets ruleset"), "{err}");
}
