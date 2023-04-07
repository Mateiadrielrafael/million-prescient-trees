#![feature(allocator_api)]
#![feature(iter_collect_into)]
use std::mem::size_of;

use echo::InfoSet;
use interactive::{get_initial_infoset, play_game};
use rand::thread_rng;
use smallvec::SmallVec;

use crate::{cfr::decision::DecisionNode, game::types::Battlefield};

mod cfr;
mod echo;
mod game;
mod helpers;
mod interactive;
mod montecarlo;
mod train;

fn main() {
    println!("Size of node {}", size_of::<cfr::decision::Node>());
    println!("Size of decision node {}", size_of::<DecisionNode>());
    println!("Size of infoset{}", size_of::<InfoSet>());

    println!(
        "Battlefields size {}",
        size_of::<SmallVec<[Battlefield; 4]>>()
    );
    println!("battlefields size 2 {}", size_of::<[Battlefield; 4]>());

    let rng = &mut thread_rng();
    // let start = Instant::now();main
    // let utility = estimate_utility(&GameState::new(), rng, 100000);
    // let duration = start.elapsed();
    //
    // get_initial_infoset();
    //
    // println!("Time elapsed: {:?}", duration);
    // println!(
    //     "You have a {}% chance of winning by playing randomly",
    //     utility_to_percentage(utility)
    // );
    //
    // let start = Instant::now();
    // let options = TrainingOptions {
    //     pruning_threshold: Some(0.01),
    //     board_evaluation: BoardEvaluation::MonteCarlo {
    //         iterations: 100,
    //         max_depth: 1,
    //     },
    //     starting_hand: get_initial_infoset()
    // };
    // let (utility, mut context) = train(options, 100, rng);
    // let duration = start.elapsed();
    //
    // println!("Time elapsed: {:?}", duration);
    // println!(
    //     "You have a {}% chance of winning against an optimal player",
    //     &utility_to_percentage(utility)
    // );
    //
    // let start = Instant::now();
    // let utility = check_against_randomness(&mut context, 100000);
    // let duration = start.elapsed();
    // println!("Time elapsed: {:?}", duration);
    // println!(
    //     "You have a {}% chance of winning against a random player",
    //     utility_to_percentage(utility)
    // );
    //
    let initial = get_initial_infoset();
    play_game(&initial, true, rng);
}
