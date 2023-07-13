use super::{
    known_state::{KnownState, TurnResult},
    other_types::{FinalMainPhaseChoice, SabotagePhaseChoice},
    types::{Battlefield, Creature, Edict, Player, PlayerStatusEffect, PlayerStatusEffects},
};
use crate::game::types::EdictSet;
use std::{debug_assert_eq, ops::Not};

// {{{ BattleResult
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum BattleResult {
    Lost,
    Tied,
    Won,
}

impl Not for BattleResult {
    type Output = Self;
    fn not(self) -> Self::Output {
        match self {
            BattleResult::Lost => BattleResult::Won,
            BattleResult::Tied => BattleResult::Tied,
            BattleResult::Won => BattleResult::Lost,
        }
    }
}
// }}}

// Context required resolving a battle
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct BattleContext {
    pub main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
    pub sabotage_choices: (SabotagePhaseChoice, SabotagePhaseChoice),
    pub state: KnownState,
}

impl BattleContext {
    #[inline]
    pub fn new(
        main_choices: (FinalMainPhaseChoice, FinalMainPhaseChoice),
        sabotage_choices: (SabotagePhaseChoice, SabotagePhaseChoice),
        state: KnownState,
    ) -> Self {
        Self {
            main_choices,
            sabotage_choices,
            state,
        }
    }

    #[inline]
    fn main_choice(&self, player: Player) -> FinalMainPhaseChoice {
        player.select(self.main_choices)
    }

    /// Returns the edict played by the current player.
    #[inline]
    fn edict(&self, player: Player) -> Edict {
        self.main_choice(player).edict
    }

    /// Returns the creature played by the current player.
    #[inline]
    fn creature(&self, player: Player) -> Creature {
        self.main_choice(player).creature
    }

    /// Returns the player effects active on some player.
    #[inline]
    fn player_effects(&self, player: Player) -> PlayerStatusEffects {
        player.select(self.state.player_states).effects
    }

    /// Returns the current battlefield
    #[inline]
    fn battlefield(&self) -> Battlefield {
        self.state.battlefields.current()
    }

    /// Checks if the creature a player has played is negated.
    #[inline]
    fn creature_is_negated(&self, player: Player) -> bool {
        // [[[WITCH EFFECT 1]]]
        let witch = self.creature(!player) == Creature::Witch;
        // [[[ROGUE EFFECT 1]]]
        let rogue =
            self.creature(player) == Creature::Seer && self.creature(!player) == Creature::Rogue;

        witch || rogue
    }

    /// Returns true if the given creature is the one a given player
    /// has played, and if it's effect has not been negated
    #[inline]
    fn is_active_creature(&self, player: Player, creature: Creature) -> bool {
        creature == self.creature(player) && !self.creature_is_negated(player)
    }

    /// Calculates the edict multiplier for some player.
    /// This multiplier is influenced by:
    /// - the urban battlefield
    /// - the steward creature
    fn edict_multiplier(&self, player: Player) -> i8 {
        let mut result = 1;

        // [[[URBAN EFFECT 1]]]
        if self.battlefield() == Battlefield::Urban {
            result += 1;
        }

        // [[[STEWARD EFFECT 1]]]
        if self.is_active_creature(player, Creature::Steward) {
            result += 1;
        }

        result
    }

    /// Returns true if the creature a player has played
    /// is affected by the battlefield bonus.
    #[inline]
    fn battlefield_bonus(&self, player: Player) -> bool {
        self.battlefield().bonus(self.creature(player))
    }

    /// Calculates the strength modifier for the creature the current player has played
    fn strength_modifier(&self, player: Player) -> i8 {
        let effects = self.player_effects(player);
        let mut result: i8 = 0;

        if self.battlefield_bonus(player) {
            result += 2;
        }

        // Creature strength bonuses:
        if !self.creature_is_negated(player) {
            match self.creature(player) {
                // [[[RANGER EFFECT 1]]]
                Creature::Ranger
                    if self.battlefield_bonus(player) && !(self.battlefield_bonus(!player)) =>
                {
                    result += 2;
                }
                // [[[BARBARIAN EFFECT 1]]]
                Creature::Barbarian if effects.has(PlayerStatusEffect::Barbarian) => {
                    result += 2;
                }
                _ => {}
            }
        }

        // Edict strength bonuses:
        // (the witch cannot get strength bonuses from edicts)
        // [[[WITCH EFFECT 2]]]
        if self.creature(player) != Creature::Witch {
            result += self.edict_multiplier(player) as i8
                * match self.edict(player) {
                    // [[[SABOTAGE EFFECT 1]]]
                    Edict::Sabotage
                        if Some(self.creature(!player)) == player.select(self.sabotage_choices) =>
                    {
                        3
                    }
                    // [[[AMBUSH EFFECT 1]]]
                    Edict::Ambush if self.battlefield_bonus(player) => 1,
                    // [[[GAMBIT EFFECT 1]]]
                    Edict::Gambit => 1,
                    _ => 0,
                }
        }

        // Lingering effects which modify strength:
        // Effects caused by the previously played creature
        // [[[BARD EFFECT 1]]]
        if effects.has(PlayerStatusEffect::Bard) {
            result += 1;
        // [[[MERCENARY EFFECT 1]]]
        } else if effects.has(PlayerStatusEffect::Mercenary) {
            result -= 1;
        }

        // Effects caused by previous battlefields
        // [[[MOUNTAIN EFFECT 1]]]
        if effects.has(PlayerStatusEffect::Mountain) {
            result += 1;
        }

        result
    }

    /// Calculate strength modifiers for a player and it's opponent.
    fn strength_modifiers(&self, player: Player) -> (i8, i8) {
        (
            self.strength_modifier(player),
            self.strength_modifier(!player),
        )
    }

    /// Check if some player wins because of an effect
    fn wins_by_effect(&self, player: Player) -> bool {
        if self.creature_is_negated(player) {
            return false;
        }

        // The wall gets negated by the witch and rogue characters
        // [[[ROGUE EFFECT 2]]]
        // [[[WITCH EFFECT 3]]]
        if self.creature(!player) == Creature::Wall
            && (self.creature(player) == Creature::Witch
                || self.creature(player) == Creature::Rogue)
        {
            return true;
        }

        // The rogue wins against the monarch
        // [[[ROGUE EFFECT 2]]]
        if self.creature(player) == Creature::Rogue && self.creature(!player) == Creature::Monarch {
            return true;
        }

        // The diplomat wins against any creature
        // if the two edicts are identical
        // [[[DIPLOMAT EFFECT 1]]]
        if self.creature(player) == Creature::Diplomat && self.edict(player) == self.edict(!player)
        {
            return true;
        }

        return false;
    }

    /// Resolves the gambit effects on a tie, relative to a given player.
    /// [[[GAMBIT EFFECT 2]]]
    fn resolve_gambits(&self, player: Player) -> BattleResult {
        // If both players played gambits, nothing happens
        if self.edict(player) == self.edict(!player) {
            return BattleResult::Tied;
        }

        // if we played a gambit, we lose on ties
        if self.edict(player) == Edict::Gambit {
            return BattleResult::Lost;
        }

        // if the opponent has played a gambit, they lose on ties
        if self.edict(!player) == Edict::Gambit {
            return BattleResult::Won;
        }

        // Otherwise it's still a tie
        BattleResult::Tied
    }

    /// Resolves a battle relative to some player
    fn battle_result(&self, player: Player) -> BattleResult {
        if self.wins_by_effect(player) {
            return BattleResult::Won;
        } else if self.wins_by_effect(!player) {
            return BattleResult::Lost;
        }
        // The wall can force ties.
        // We don't have to check for the wall being negated here,
        // as that would trigger a win by effect.
        // [[[WALL EFFECT 1]]]
        else if self.creature(player) == Creature::Wall
            || self.creature(!player) == Creature::Wall
        {
            return self.resolve_gambits(player);
        }

        let base_strengths = (
            self.creature(player).strength() as i8,
            self.creature(!player).strength() as i8,
        );

        let strength_modifiers = self.strength_modifiers(player);
        let strengths = (
            base_strengths.0 + strength_modifiers.0,
            base_strengths.1 + strength_modifiers.1,
        );

        if strengths.0 < strengths.1 {
            BattleResult::Lost
        } else if strengths.0 > strengths.1 {
            BattleResult::Won
        } else {
            self.resolve_gambits(player)
        }
    }

    /// Calculate the amount of victory points
    /// the value of the current battle changed by
    /// because of the cards played by a player.
    fn edict_reward(&self, player: Player) -> i8 {
        self.edict_multiplier(player) as i8
            * match self.edict(player) {
                // [[[RILETHEPUBLIC EFFECT 1]]]
                Edict::RileThePublic => 1,
                // [[[DIVERTATTENTION EFFECT 1]]]
                // [[[RILETHEPUBLIC EFFECT 2]]]
                Edict::DivertAttention if self.edict(!player) != Edict::RileThePublic => -1,
                _ => 0,
            }
    }

    /// Calculates the amount of victory points
    /// earned by winning this partidcular battle
    /// as a given player.
    fn battle_reward(&self, player: Player) -> u8 {
        let effects = self.player_effects(player);
        let mut total = self.battlefield().reward();

        // Lingering effects:
        // [[[NIGHT EFFECT 1]]]
        if effects.has(PlayerStatusEffect::Night) {
            total += 1;
        // [[[GLADE EFFECT 1]]]
        } else if effects.has(PlayerStatusEffect::Glade) {
            total += 2;
        }

        // [[[BARD EFFECT 2]]]
        if effects.has(PlayerStatusEffect::Bard) {
            total += 1;
        }

        // Apply the "rile the public" and "divert attention" edicts.
        // This is the only place where the total can decrease,
        // which is why we must be careful for it not to become negative.
        total = i8::max(
            0,
            total as i8 + self.edict_reward(player) + self.edict_reward(!player),
        ) as u8;

        total
    }

    /// The reward for a player killing the monarch
    /// [[[MONARCH EFFECT 1]]]
    fn monarch_reward(&self, player: Player, result: BattleResult) -> u8 {
        match result {
            BattleResult::Won | BattleResult::Tied
                if self.is_active_creature(!player, Creature::Monarch) =>
            {
                2
            }
            _ => 0,
        }
    }

    /// Calculates the delta we need to change the score by.
    /// - positive values mean we've earned points
    /// - negative values mean the opponent has gained points
    fn battle_score_delta(&self, result: BattleResult, player: Player) -> i8 {
        let mut delta = match result {
            BattleResult::Tied => 0,
            BattleResult::Won => self.battle_reward(player) as i8,
            BattleResult::Lost => -(self.battle_reward(player) as i8),
        };

        // Trigger monarch's effect
        delta += self.monarch_reward(player, result) as i8;
        delta -= self.monarch_reward(!player, !result) as i8;

        delta
    }

    pub fn advance_known_state(&self) -> (BattleResult, TurnResult<KnownState>) {
        let player = Player::Me;
        let battle_result = self.battle_result(player);

        debug_assert_eq!(battle_result, !self.battle_result(!player));

        let score_delta = self.battle_score_delta(battle_result, player);
        let score = self.state.score + score_delta;

        debug_assert_eq!(
            score_delta,
            -self.battle_score_delta(!battle_result, !player)
        );

        let turn_result = match self.state.battlefields.next() {
            // Continue game
            Some(battlefields) => {
                let mut new_state = KnownState {
                    battlefields,
                    score,
                    ..self.state
                };

                let (p1, p2) = &mut new_state.player_states;

                // Discard used edicts
                p1.edicts.remove(self.edict(player));
                p2.edicts.remove(self.edict(!player));

                // Clear status effects
                p1.effects.clear();
                p2.effects.clear();

                // Resolve the Steward effect
                // [[[STEWARD EFFECT 2]]]
                if self.is_active_creature(player, Creature::Steward) {
                    p1.edicts = EdictSet::all();
                } else if self.is_active_creature(!player, Creature::Steward) {
                    p2.edicts = EdictSet::all();
                }

                // Set up global lingering effects
                if self.battlefield() == Battlefield::Night {
                    // [[[NIGHT SETUP]]]
                    p1.effects.add(PlayerStatusEffect::Night);
                    p2.effects.add(PlayerStatusEffect::Night);
                }

                // first is winner, second is loser
                let player_by_status = match battle_result {
                    BattleResult::Won => Some((p1, p2)),
                    BattleResult::Lost => Some((p2, p1)),
                    BattleResult::Tied => None,
                };

                if let Some((winner, loser)) = player_by_status {
                    match self.battlefield() {
                        // [[[GLADE SETUP]]]
                        Battlefield::Glade => {
                            winner.effects.add(PlayerStatusEffect::Glade);
                        }
                        // [[[MOUNTAIN SETUP]]]
                        Battlefield::Mountain => {
                            winner.effects.add(PlayerStatusEffect::Mountain);
                        }
                        _ => {}
                    }

                    // if this card has already been played there's no point
                    // in adding the status effect anymore
                    // [[[BARBARIAN SETUP]]]
                    if !new_state.graveyard.has(Creature::Barbarian) {
                        loser.effects.add(PlayerStatusEffect::Barbarian)
                    }
                }

                for player in Player::PLAYERS {
                    let effects = &mut player.select(new_state.player_states).effects;
                    match self.creature(player) {
                        // [[[MERCENARY SETUP]]]
                        Creature::Mercenary => effects.add(PlayerStatusEffect::Mercenary),
                        // [[[SEER SETUP]]]
                        Creature::Seer => effects.add(PlayerStatusEffect::Seer),
                        // [[[BARD SETUP]]]
                        Creature::Bard => effects.add(PlayerStatusEffect::Bard),
                        _ => {}
                    }
                }

                TurnResult::Unfinished(new_state)
            }

            // Report final results
            None => TurnResult::Finished(score),
        };

        (battle_result, turn_result)
    }
}

// {{{ Test helpers
impl BattleContext {
    /// Sets the creature played by a player.
    #[inline]
    fn set_creature(&mut self, player: Player, creature: Creature) {
        let choice = player.select_mut(&mut self.main_choices);
        choice.creature = creature;
    }

    /// Sets the edict played by a player.
    #[inline]
    fn set_edict(&mut self, player: Player, edict: Edict) {
        let choice = player.select_mut(&mut self.main_choices);
        choice.edict = edict;
    }

    /// Returns a mut ref to the player effects active on some player.
    #[inline]
    fn player_effects_mut(&mut self, player: Player) -> &mut PlayerStatusEffects {
        &mut player.select_mut(&mut self.state.player_states).effects
    }

    /// Returns a mut ref to the player effects active on some player.
    #[inline]
    fn add_effect(&mut self, player: Player, effect: PlayerStatusEffect) {
        player
            .select_mut(&mut self.state.player_states)
            .effects
            .add(effect)
    }

    /// Sets the main creature played by a player.
    #[inline]
    fn set_battlefield(&mut self, battlefield: Battlefield) {
        self.state.battlefields.all[self.state.battlefields.current] = battlefield;
    }
}
// }}}
// {{{ Tests
#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;
    use crate::game::{
        known_state::{Battlefields, Score},
        types::CreatureSet,
    };
    use once_cell::sync::Lazy;

    // {{{ Common setup
    const BASIC_STATE: Lazy<KnownState> = Lazy::new(|| KnownState {
        battlefields: Battlefields::new([Battlefield::Plains; 4]),
        graveyard: CreatureSet::default(),
        score: Score::default(),
        player_states: Default::default(),
    });

    const BASIC_BATTLE_CONTEXT: Lazy<BattleContext> = Lazy::new(|| {
        let p1_choice = FinalMainPhaseChoice::new(Creature::Mercenary, Edict::Gambit);
        let p2_choice = FinalMainPhaseChoice::new(Creature::Seer, Edict::Gambit);

        BattleContext::new((p1_choice, p2_choice), (None, None), *BASIC_STATE)
    });
    // }}}
    // {{{ Battlefields
    #[test]
    fn mountain_glade_setup() {
        let setups = [
            (Battlefield::Glade, PlayerStatusEffect::Glade),
            (Battlefield::Mountain, PlayerStatusEffect::Mountain),
        ];

        let mut ctx = *BASIC_BATTLE_CONTEXT;

        for (battlefield, effect) in setups {
            ctx.set_battlefield(battlefield);

            let has_effect = ctx
                .advance_known_state()
                .1
                .get_unfinished()
                .unwrap()
                .player_states
                .0
                .effects
                .has(effect);

            assert!(has_effect, "{:?} setup does not work", battlefield,);
        }
    }

    #[test]
    fn glade_effect() {
        let winner = Player::Me;
        for player in Player::PLAYERS {
            let mut ctx = *BASIC_BATTLE_CONTEXT;
            ctx.add_effect(player, PlayerStatusEffect::Glade);

            let extra_reward = ctx.battle_reward(winner) - ctx.battlefield().reward();

            if player == winner {
                assert_eq!(extra_reward, 2, "Glade effect does not work");
            } else {
                assert_eq!(extra_reward, 0, "Glade should only affect one player");
            }
        }
    }

    #[test]
    fn mountain_effect() {
        for player in Player::PLAYERS {
            let mut ctx = *BASIC_BATTLE_CONTEXT;
            ctx.add_effect(player, PlayerStatusEffect::Mountain);

            // We don't want the edicts to influence the strength values.
            ctx.set_edict(player, Edict::RileThePublic);
            ctx.set_edict(!player, Edict::RileThePublic);

            assert_eq!(ctx.strength_modifier(player), 1, "Mountain does not work");
            assert_eq!(
                ctx.strength_modifier(!player),
                0,
                "Mountain should only affect one player"
            );
        }
    }

    #[test]
    fn night_setup() {
        let mut ctx = *BASIC_BATTLE_CONTEXT;
        ctx.set_battlefield(Battlefield::Night);

        let effect = PlayerStatusEffect::Night;

        let unfinished = ctx.advance_known_state().1.get_unfinished().unwrap();
        let has_effects = unfinished.player_states.0.effects.has(effect)
            && unfinished.player_states.1.effects.has(effect);

        assert!(has_effects);
    }

    #[test]
    fn night_effect() {
        let mut ctx = *BASIC_BATTLE_CONTEXT;

        // Give the status effect to both players
        ctx.add_effect(Player::Me, PlayerStatusEffect::Night);
        ctx.add_effect(Player::You, PlayerStatusEffect::Night);

        for player in Player::PLAYERS {
            assert_eq!(
                ctx.battle_reward(player) - ctx.battlefield().reward(),
                1,
                "Night effect does not work"
            );
        }
    }

    #[test]
    fn urban_effect() {
        let mut ctx = *BASIC_BATTLE_CONTEXT;
        ctx.set_battlefield(Battlefield::Urban);

        for player in Player::PLAYERS {
            assert_eq!(ctx.edict_multiplier(player), 2);
        }
    }
    // }}}
    // {{{ Creatures
    #[test]
    fn steward_effect_1() {
        for player in Player::PLAYERS {
            let mut ctx = *BASIC_BATTLE_CONTEXT;
            ctx.set_creature(player, Creature::Steward);

            assert_eq!(ctx.edict_multiplier(player), 2);
            assert_eq!(ctx.edict_multiplier(!player), 1);
        }
    }

    #[test]
    fn steward_effect_2() {
        for player in Player::PLAYERS {
            let mut ctx = *BASIC_BATTLE_CONTEXT;
            ctx.set_creature(player, Creature::Steward);

            let player_states = ctx
                .advance_known_state()
                .1
                .get_unfinished()
                .unwrap()
                .player_states;

            let player_edicts = player.select(player_states).edicts;
            let opponent_edicts = (!player).select(player_states).edicts;

            assert_eq!(
                player_edicts,
                EdictSet::default(),
                "Steward player must have all the edicts"
            );

            // In contrast, the opponent has fewer edicts
            assert_eq!(opponent_edicts.len(), 4, "Opponent must have 4 edicts");
        }
    }
    // }}}
    // {{{ Rules
    #[test]
    fn edict_multiplier_additive() {
        for player in Player::PLAYERS {
            let mut ctx = *BASIC_BATTLE_CONTEXT;
            ctx.set_battlefield(Battlefield::Urban);
            ctx.set_creature(player, Creature::Steward);

            assert_eq!(ctx.edict_multiplier(player), 3);
            assert_eq!(ctx.edict_multiplier(!player), 2);
        }
    }
    // }}}
}
// }}}
