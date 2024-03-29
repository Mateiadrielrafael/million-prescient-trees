use super::battlefield::{Battlefield, Battlefields};
use super::creature::{Creature, CreatureSet};
use super::edict::{Edict, EdictSet};
use super::known_state_summary::KnownStateEssentials;
use super::status_effect::{StatusEffect, StatusEffectSet};
use super::types::{Player, Score};
use crate::helpers::bitfield::Bitfield;
use crate::helpers::pair::{are_equal, Pair};

/// State of a player known by both players.
#[derive(PartialEq, Eq, Clone, Copy, Debug, Default)]
pub struct KnownPlayerState {
    pub edicts: EdictSet,
    pub effects: StatusEffectSet,
}

/// State known by both players at some point in time.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct KnownState {
    pub player_states: Pair<KnownPlayerState>,
    pub battlefields: Battlefields,
    pub graveyard: CreatureSet,
    pub score: Score,
}

impl KnownStateEssentials for KnownState {
    #[inline(always)]
    fn graveyard(&self) -> CreatureSet {
        self.graveyard
    }

    fn seer_player(&self) -> Option<Player> {
        if self.player_states[0]
            .effects
            .has(super::status_effect::StatusEffect::Seer)
        {
            Some(Player::Me)
        } else if self.player_states[1]
            .effects
            .has(super::status_effect::StatusEffect::Seer)
        {
            Some(Player::You)
        } else {
            None
        }
    }

    #[inline(always)]
    fn edict_sets(&self) -> Pair<EdictSet> {
        self.player_states.map(|s| s.edicts)
    }
}

impl KnownState {
    pub fn new_starting(battlefields: [Battlefield; 4]) -> Self {
        Self {
            player_states: Default::default(),
            graveyard: Default::default(),
            score: Default::default(),
            battlefields: Battlefields::new(battlefields),
        }
    }

    /// Returns whether the current known game state is symmetrical.
    /// A game state is symmetrical if whenever (A, B) is a possible
    /// combination of hidden information the two players might know,
    /// (B, A) is also such a possibility.
    ///
    /// The first turn is usually the only symmetrical game state.
    pub fn is_symmetrical(&self) -> bool {
        self.battlefields.current == 0
            && are_equal(self.player_states)
            && self.score == Score::default()
    }

    /// Returns the score from a given player's perspective
    #[inline(always)]
    pub fn score(&self, player: Player) -> Score {
        match player {
            Player::Me => self.score,
            Player::You => -self.score,
        }
    }

    /// Computes whether a given player is guaranteed to win,
    /// no matter what the opponent can pull off.
    // TODO: add stalling with wall?
    pub fn guaranteed_win(&self, player: Player) -> bool {
        // {{{ Rile the public spam
        let has_rtp = self.player_edicts(!player).has(Edict::RileThePublic);
        let has_steward = !self.graveyard.has(Creature::Steward);
        let has_urban = self.battlefields.will_be_active(Battlefield::Urban);

        let turns_left = 4 - self.battlefields.current;
        let mut rtp_usages = 0;

        if has_rtp {
            rtp_usages += 1; // base usage
        };

        if has_urban {
            rtp_usages += 1; // edict multiplier
        };

        if has_steward {
            rtp_usages += 1; // edict multiplier

            if turns_left > 1 {
                rtp_usages += 1; // steward return edicts to hand effect
            }
        };
        // }}}

        let mut max_opponent_gain = self
            .battlefields
            .active()
            .into_iter()
            .map(|battlefield| battlefield.reward())
            .sum::<u8>() as i8
            + rtp_usages;

        // {{{ Battlefield vp bonuses
        let effects = (!player).select(self.player_states).effects;

        if effects.has(StatusEffect::Glade) || self.battlefields.will_be_active(Battlefield::Glade)
        {
            max_opponent_gain += 2;
        }

        if effects.has(StatusEffect::Night) || self.battlefields.will_be_active(Battlefield::Night)
        {
            max_opponent_gain += 1;
        }
        // }}}

        self.score(player) > Score(max_opponent_gain)
    }
}
