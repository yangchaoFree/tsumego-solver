use crate::go::{GoBoard, GoGame, GoPlayer};
use crate::puzzle::Puzzle;
use std::time::Duration;

pub fn validate_candidate(candidate: GoBoard, timeout: Duration) -> bool {
    if candidate.has_dead_groups() {
        return false;
    }

    GoPlayer::both().all(|first_player| {
        let mut puzzle = Puzzle::new(GoGame::from_board(candidate, *first_player));

        if !puzzle.solve_with_timeout(timeout) {
            return false;
        }

        puzzle.is_proved()
    })
}