//! Runtime (engine-path) regression for Coalition Victory.
//!
//! Oracle: "You win the game if you control a land of each basic land type and a
//! creature of each color." The trailing "if …" condition was dropped, so the
//! spell parsed to a bare `Effect::WinTheGame` and casting it won the game
//! immediately regardless of the board (CR 104.2b — an effect may state a player
//! wins the game; CR 608.2h — the game-state check is made once, as the spell
//! resolves). This drives the real cast pipeline:
//! casting with an incomplete board must NOT win; casting while controlling a
//! land of each basic type and a creature of each color must win.

use engine::game::scenario::{GameScenario, P0};
use engine::types::game_state::WaitingFor;
use engine::types::mana::{ManaColor, ManaCost};
use engine::types::phase::Phase;

const ORACLE: &str =
    "You win the game if you control a land of each basic land type and a creature of each color.";

fn caster_won(runner: &engine::game::scenario::GameRunner) -> bool {
    matches!(runner.state().waiting_for, WaitingFor::GameOver { winner: Some(w) } if w == P0)
}

/// Condition FALSE (empty board): casting Coalition Victory must NOT win.
/// Revert-probe: on the old bare-`WinTheGame` parse this wins unconditionally.
#[test]
fn coalition_victory_does_not_win_without_the_board() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cv = scenario
        .add_spell_to_hand_from_oracle(P0, "Coalition Victory", false, ORACLE)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    let mut runner = scenario.build();

    runner.cast(cv).resolve();

    assert!(
        !caster_won(&runner),
        "Coalition Victory must NOT win with no lands/creatures (its condition is unmet); \
         the old bare WinTheGame parse won unconditionally"
    );
}

/// Condition TRUE: controlling a land of each basic land type and a creature of
/// each color, casting Coalition Victory wins the game.
#[test]
fn coalition_victory_wins_with_full_domain_and_all_colors() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PreCombatMain);
    let cv = scenario
        .add_spell_to_hand_from_oracle(P0, "Coalition Victory", false, ORACLE)
        .with_mana_cost(ManaCost::generic(0))
        .id();
    // A land of each basic land type (full domain).
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        scenario.add_basic_land(P0, color);
    }
    // A creature of each color.
    let mut creature_ids = Vec::new();
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        let id = scenario.add_creature(P0, "Coalition Soldier", 1, 1).id();
        creature_ids.push((id, color));
    }
    let mut runner = scenario.build();
    // Paint each creature its color (DistinctColorsAmongPermanents reads obj.color).
    for (id, color) in creature_ids {
        let obj = runner.state_mut().objects.get_mut(&id).unwrap();
        obj.color = vec![color];
        obj.base_color = vec![color];
    }

    runner.cast(cv).resolve();

    assert!(
        caster_won(&runner),
        "Coalition Victory must win the game when its controller has a land of each basic \
         land type and a creature of each color; waiting_for = {:?}",
        runner.state().waiting_for
    );
}
