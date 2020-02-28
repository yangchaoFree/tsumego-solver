#![allow(dead_code)]

mod benson;
mod bit_board;
pub use bit_board::{BitBoard, BoardPosition};

use std::fmt;
use std::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BoardCell {
    Empty,
    // OutOfBounds,
    Occupied(GoPlayer),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum GoPlayer {
    Black,
    White,
}

impl GoPlayer {
    pub fn flip(self) -> GoPlayer {
        match self {
            GoPlayer::Black => GoPlayer::White,
            GoPlayer::White => GoPlayer::Black,
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub struct GoBoard {
    white: BitBoard,
    black: BitBoard,
}

impl Debug for GoBoard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for j in 0..BitBoard::height() {
            for i in 0..BitBoard::width() {
                f.write_str(match self.get_cell(BoardPosition::new(i, j)) {
                    BoardCell::Empty => ".",
                    BoardCell::Occupied(GoPlayer::White) => "w",
                    BoardCell::Occupied(GoPlayer::Black) => "b",
                })?;
            }
            f.write_str("\n")?;
        }

        Ok(())
    }
}

impl GoBoard {
    fn empty() -> GoBoard {
        GoBoard {
            white: BitBoard::empty(),
            black: BitBoard::empty(),
        }
    }

    fn empty_cells(&self) -> BitBoard {
        !(self.white | self.black)
    }

    fn set_cell(&mut self, position: BoardPosition, cell: BoardCell) {
        let mask = BitBoard::singleton(position);

        match cell {
            BoardCell::Empty => {
                self.white = self.white & !mask;
                self.black = self.black & !mask;
            }
            BoardCell::Occupied(GoPlayer::Black) => {
                self.black = self.black | mask;
            }
            BoardCell::Occupied(GoPlayer::White) => {
                self.white = self.white | mask;
            }
        }
    }

    fn get_cell(&self, position: BoardPosition) -> BoardCell {
        let mask = BitBoard::singleton(position);

        if !((mask & self.white).is_empty()) {
            return BoardCell::Occupied(GoPlayer::White);
        }

        if !((mask & self.black).is_empty()) {
            return BoardCell::Occupied(GoPlayer::Black);
        }

        BoardCell::Empty
    }

    fn get_bitboard_for_player(&self, player: GoPlayer) -> BitBoard {
        match player {
            GoPlayer::Black => self.black,
            GoPlayer::White => self.white,
        }
    }

    fn get_bitboard_at_position(&self, position: BoardPosition) -> BitBoard {
        let mask = BitBoard::singleton(position);

        if !((mask & self.white).is_empty()) {
            return self.white;
        }

        if !((mask & self.black).is_empty()) {
            return self.black;
        }

        panic!("No board at this position")
    }

    fn group_has_liberties(&self, position: BoardPosition) -> bool {
        let mask = self.get_bitboard_at_position(position);

        let group = BitBoard::singleton(position).flood_fill(mask);

        !(group.expand_one() & self.empty_cells()).is_empty()
    }

    fn remove_dead_groups_for_player(&mut self, player: GoPlayer) {
        let opponents_bitboard = self.get_bitboard_for_player(player);
        let stones_with_liberties =
            (self.empty_cells().expand_one() & opponents_bitboard).flood_fill(opponents_bitboard);

        match player {
            GoPlayer::White => self.white = stones_with_liberties,
            GoPlayer::Black => self.black = stones_with_liberties,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct GoGame {
    ko_violations: BitBoard,
    board: GoBoard,
    out_of_bounds: BitBoard,
    pub current_player: GoPlayer,
    pub last_move_pass: bool,
}

#[derive(Debug, PartialEq)]
pub enum MoveError {
    Occupied,
    OutOfBounds,
    OutOfTurn,
    Suicidal,
    Ko,
}

impl GoGame {
    pub fn empty() -> GoGame {
        GoGame {
            board: GoBoard::empty(),
            ko_violations: BitBoard::empty(),
            current_player: GoPlayer::Black,
            out_of_bounds: BitBoard::empty(),
            last_move_pass: false,
        }
    }

    pub fn from_board(board: GoBoard, out_of_bounds: BitBoard) -> GoGame {
        GoGame {
            board,
            ko_violations: BitBoard::empty(),
            current_player: GoPlayer::Black,
            out_of_bounds,
            last_move_pass: false,
        }
    }

    pub fn get_board(&self) -> GoBoard {
        self.board
    }

    fn get_cell(&self, position: BoardPosition) -> BoardCell {
        self.get_board().get_cell(position)
    }

    fn is_out_of_bounds(&self, position: BoardPosition) -> bool {
        !(BitBoard::singleton(position) & self.out_of_bounds).is_empty()
    }

    fn empty_cells(&self) -> BitBoard {
        self.get_board().empty_cells() & !self.out_of_bounds
    }

    pub fn play_move_for_player(
        &self,
        position: BoardPosition,
        player: GoPlayer,
    ) -> Result<GoGame, MoveError> {
        if self.current_player != player {
            return Err(MoveError::OutOfTurn);
        }

        self.play_move(position)
    }

    pub fn play_move(&self, position: BoardPosition) -> Result<GoGame, MoveError> {
        if self.get_cell(position) != BoardCell::Empty {
            return Err(MoveError::Occupied);
        }

        if self.is_out_of_bounds(position) {
            return Err(MoveError::OutOfBounds);
        }

        let next_player = self.current_player.flip();

        let mut new_board = self.get_board().clone();
        new_board.set_cell(position, BoardCell::Occupied(self.current_player));

        // Remove dead groups owned by other player
        new_board.remove_dead_groups_for_player(next_player);

        // Evaluate suicide
        if !new_board.group_has_liberties(position) {
            return Err(MoveError::Suicidal);
        }

        // Evaluate ko
        if self.ko_violations.is_set(position) {
            return Err(MoveError::Ko);
        }

        let ko_violations = (self.board.get_bitboard_for_player(next_player)
            & !new_board.get_bitboard_for_player(next_player))
        .singletons();

        Ok(GoGame {
            ko_violations,
            board: new_board,
            current_player: next_player,
            out_of_bounds: self.out_of_bounds,
            last_move_pass: false,
        })
    }

    pub fn pass(&self) -> GoGame {
        GoGame {
            board: self.board,
            ko_violations: BitBoard::empty(),
            current_player: self.current_player.flip(),
            out_of_bounds: self.out_of_bounds,
            last_move_pass: true,
        }
    }

    pub fn generate_moves(&self) -> Vec<GoGame> {
        let mut games = Vec::new();

        for position in self.empty_cells().positions() {
            if let Ok(game) = self.play_move(position) {
                games.push(game);
            }
        }

        games
    }

    // pub fn plys(&self) -> usize {
    //     self.boards.len() - 1
    // }
}

impl GoGame {
    pub fn from_sgf(sgf_string: &str) -> GoGame {
        use sgf_parser::{parse, Action, Color, SgfToken};

        let sgf = parse(sgf_string).unwrap();

        assert_eq!(sgf.count_variations(), 0);

        let mut nodes = sgf.iter();

        let first_node = nodes.next().unwrap();

        let mut board = GoBoard::empty();
        let mut triangle_location = None;

        for token in first_node.tokens.iter() {
            match token {
                SgfToken::Add {
                    color,
                    coordinate: (i, j),
                } => board.set_cell(
                    BoardPosition::new(i - 1, j - 1),
                    BoardCell::Occupied(match color {
                        Color::Black => GoPlayer::Black,
                        Color::White => GoPlayer::White,
                    }),
                ),
                SgfToken::Triangle { coordinate: (i, j) } => {
                    triangle_location = Some(BoardPosition::new(i - 1, j - 1))
                }
                SgfToken::Move { .. } => panic!("Cannot move at this time!"),
                _ => {}
            }
        }

        let out_of_bounds = match triangle_location {
            None => BitBoard::empty(),
            Some(position) => BitBoard::singleton(position).flood_fill(board.empty_cells()),
        };

        let mut game = GoGame::from_board(board, out_of_bounds);

        for node in nodes {
            for token in node.tokens.iter() {
                match token {
                    SgfToken::Move {
                        color,
                        action: Action::Move(i, j),
                    } => {
                        game = game
                            .play_move_for_player(
                                BoardPosition::new(i - 1, j - 1),
                                match color {
                                    Color::Black => GoPlayer::Black,
                                    Color::White => GoPlayer::White,
                                },
                            )
                            .unwrap()
                    }
                    SgfToken::Add { .. } => panic!("Cannot add stones at this time!"),
                    _ => {}
                }
            }
        }

        game
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_add_stone() {
        let game = GoGame::empty();
        let game = game
            .play_move_for_player(BoardPosition::new(0, 0), GoPlayer::Black)
            .unwrap();

        assert_eq!(
            game.get_cell(BoardPosition::new(0, 0)),
            BoardCell::Occupied(GoPlayer::Black)
        );
    }

    #[test]
    fn previous_board_is_not_mutated() {
        let old_game = GoGame::empty();
        let new_game = old_game
            .play_move_for_player(BoardPosition::new(0, 0), GoPlayer::Black)
            .unwrap();

        assert_ne!(
            old_game.get_cell(BoardPosition::new(0, 0)),
            new_game.get_cell(BoardPosition::new(0, 0))
        );
    }

    #[test]
    fn current_player_starts_as_black() {
        let game = GoGame::empty();

        assert_eq!(game.current_player, GoPlayer::Black);
    }

    #[test]
    fn player_advances_when_playing_move() {
        let game = GoGame::empty().play_move(BoardPosition::new(0, 0)).unwrap();

        assert_eq!(game.current_player, GoPlayer::White);
    }

    #[test]
    fn cannot_play_move_out_of_turn() {
        let result =
            GoGame::empty().play_move_for_player(BoardPosition::new(0, 0), GoPlayer::White);

        assert_eq!(result, Err(MoveError::OutOfTurn));
    }

    #[test]
    fn cannot_play_in_occupied_space() {
        let game = GoGame::empty().play_move(BoardPosition::new(0, 0)).unwrap();
        let result = game.play_move(BoardPosition::new(0, 0));

        assert_eq!(result, Err(MoveError::Occupied));
    }

    #[test]
    fn single_groups_are_captured() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/single_groups_are_captured.sgf"));

        assert_eq!(game.get_cell(BoardPosition::new(0, 0)), BoardCell::Empty);
    }

    #[test]
    fn complex_groups_are_captured() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/complex_capture.sgf"));
        let game = game.play_move(BoardPosition::new(11, 6)).unwrap();

        assert_eq!(
            format!("{:?}", game.get_board()),
            "................\n\
             .b.b.bbww.w.....\n\
             .......bww.w.w..\n\
             ...b.bb.w..wb...\n\
             ....b...ww.w....\n\
             ...b..w.w...w...\n\
             .........www....\n\
             ................\n"
        );
    }

    #[test]
    fn capturing_has_precedence_over_suicide() {
        let game = GoGame::from_sgf(include_str!(
            "test_sgfs/capturing_has_precedence_over_suicide.sgf"
        ));

        assert_eq!(game.get_cell(BoardPosition::new(1, 0)), BoardCell::Empty);
    }

    #[test]
    fn cannot_commit_suicide() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/cannot_commit_suicide.sgf"));
        let result = game.play_move(BoardPosition::new(0, 0));

        assert_eq!(result, Err(MoveError::Suicidal));
    }

    #[test]
    fn ko_rule_simple() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/ko_rule_simple.sgf"));
        let result = game.play_move(BoardPosition::new(2, 2));

        assert_eq!(result, Err(MoveError::Ko));
    }

    #[test]
    fn capture_two_recapture_one_not_ko_violation() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/capture_two_recapture_one.sgf"));

        game.play_move(BoardPosition::new(3, 2)).unwrap();
    }

    #[test]
    fn out_of_bounds_moves_are_not_generated() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/puzzles/true_simple1.sgf"));
        let moves = game.generate_moves();

        assert_eq!(moves.len(), 5);
    }

    #[test]
    fn pass_sets_last_move_pass() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/ko_rule_simple.sgf"));
        let new_game = game.pass();

        assert!(new_game.last_move_pass);
    }

    #[test]
    fn move_clears_last_move_pass() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/ko_rule_simple.sgf"));
        let game = game.pass();
        let game = game.play_move(BoardPosition::new(13, 7)).unwrap();

        assert!(!game.last_move_pass);
    }

    #[test]
    fn pass_advances_player() {
        let game = GoGame::from_sgf(include_str!("test_sgfs/ko_rule_simple.sgf"));
        let new_game = game.pass();

        assert_ne!(game.current_player, new_game.current_player);
    }
}
