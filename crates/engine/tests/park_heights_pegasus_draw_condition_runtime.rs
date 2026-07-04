//! Regression test for a dropped effect-gate on **Park Heights Pegasus**:
//!
//! > Whenever this creature deals combat damage to a player, draw a card if
//! > you had two or more creatures enter the battlefield under your control
//! > this turn.
//!
//! The "draw a card **if you had two or more creatures enter the battlefield
//! under your control this turn**" clause is an effect-level conditional
//! (CR 608.2c) gated on a CR 400.7 this-turn ETB threshold. Before the fix the
//! condition combinator's "you had" branch hardcoded a count of 1 and required
//! an article ("a"/"an"/"another"), so the counted "two or more" surface failed
//! to parse and the whole trailing `if` was DROPPED — the card drew a card on
//! every combat-damage trigger, unconditionally.
//!
//! After the fix, `parse_entered_this_turn` recognizes the counted
//! "you had N or more [type] enter ... this turn" form and emits
//! `QuantityComparison { EnteredThisTurn{creature/You} >= N }`, which the effect
//! pipeline bridges onto the draw as `AbilityCondition::QuantityCheck`.
//!
//! Both tests parse the real effect clause through the production
//! `parse_effect_chain` path and resolve it under P0. `turn_number` is 2 (set by
//! `at_phase`), so `with_summoning_sickness()` creatures (ETB turn 2) count as
//! "entered this turn" while `add_creature` pre-existing creatures (ETB turn 1)
//! do not.
//!
//! CR 400.7: an object entered the battlefield this turn.
//! CR 608.2c: a conditional ("if") clause on an effect.
//! CR 109.5 / CR 205: "under your control" scopes the count to the controller.

use engine::game::ability_utils::build_resolved_from_def;
use engine::game::effects::resolve_ability_chain;
use engine::game::scenario::{GameScenario, P0};
use engine::types::ability::AbilityKind;
use engine::types::game_state::GameState;
use engine::types::phase::Phase;
use engine::types::player::PlayerId;

const DRAW_CLAUSE: &str =
    "Draw a card if you had two or more creatures enter the battlefield under your control this turn.";

fn hand_size(state: &GameState, player: PlayerId) -> usize {
    state
        .players
        .iter()
        .find(|p| p.id == player)
        .map(|p| p.hand.len())
        .unwrap_or(0)
}

/// DISCRIMINATOR (revert-probe): only ONE creature entered under P0's control
/// this turn, so the `>= 2` gate is FALSE and no card is drawn. With the fix
/// reverted the dropped condition makes the draw unconditional, P0 draws, and
/// this assertion fails.
#[test]
fn pegasus_does_not_draw_when_fewer_than_two_creatures_entered() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PostCombatMain);

    // The ability source (entered a prior turn — does NOT count toward the tally).
    let pegasus = scenario.add_creature(P0, "Park Heights Pegasus", 2, 2).id();

    // Exactly ONE creature entered under P0's control this turn.
    scenario
        .add_creature(P0, "Fresh Arrival", 1, 1)
        .with_summoning_sickness();

    // Give P0 a card to draw so that, if the bug is present, the unconditional
    // draw succeeds and the hand grows — making the revert-probe meaningful.
    scenario.with_library_top(P0, &["Reward Card"]);

    let mut runner = scenario.build();

    let before = hand_size(runner.state(), P0);

    let def = engine::parser::oracle_effect::parse_effect_chain(DRAW_CLAUSE, AbilityKind::Spell);
    let ability = build_resolved_from_def(&def, pegasus, P0);

    let mut events = Vec::new();
    resolve_ability_chain(runner.state_mut(), &ability, &mut events, 0)
        .expect("draw effect must resolve");

    let after = hand_size(runner.state(), P0);
    assert_eq!(
        after,
        before,
        "P0 drew {} card(s) but only one creature entered this turn; the >= 2 \
         gate must suppress the draw (dropped condition would draw unconditionally)",
        after as i64 - before as i64
    );
}

/// POSITIVE case: TWO creatures entered under P0's control this turn, so the
/// `>= 2` gate is satisfied and exactly one card is drawn.
#[test]
fn pegasus_draws_when_two_creatures_entered() {
    let mut scenario = GameScenario::new();
    scenario.at_phase(Phase::PostCombatMain);

    let pegasus = scenario.add_creature(P0, "Park Heights Pegasus", 2, 2).id();

    // TWO creatures entered under P0's control this turn.
    scenario
        .add_creature(P0, "Fresh Arrival A", 1, 1)
        .with_summoning_sickness();
    scenario
        .add_creature(P0, "Fresh Arrival B", 1, 1)
        .with_summoning_sickness();

    scenario.with_library_top(P0, &["Reward Card"]);

    let mut runner = scenario.build();

    let before = hand_size(runner.state(), P0);

    let def = engine::parser::oracle_effect::parse_effect_chain(DRAW_CLAUSE, AbilityKind::Spell);
    let ability = build_resolved_from_def(&def, pegasus, P0);

    let mut events = Vec::new();
    resolve_ability_chain(runner.state_mut(), &ability, &mut events, 0)
        .expect("draw effect must resolve");

    let after = hand_size(runner.state(), P0);
    assert_eq!(
        after,
        before + 1,
        "P0 should draw exactly one card when two creatures entered this turn"
    );
}
