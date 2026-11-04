//! The chess Carol needs and no more: a board from FEN, the drills from `drills.json`, and whether a
//! move's squares are the line's. There is no move generation: every position is given.

use serde::Deserialize;

/// A square, `rank * 8 + file` (a1 = 0, h8 = 63).
pub type Sq = u8;

#[derive(Clone, Debug, PartialEq)]
pub struct Board {
    squares: [Option<char>; 64],
    pub white_to_move: bool,
}

impl Board {
    pub fn empty() -> Board {
        Board { squares: [None; 64], white_to_move: true }
    }

    pub fn from_fen(fen: &str) -> anyhow::Result<Board> {
        let mut parts = fen.split_whitespace();
        let rows = parts.next().ok_or_else(|| anyhow::anyhow!("empty FEN"))?;
        let mut b = Board::empty();
        let mut rank = 7i32;
        for row in rows.split('/') {
            let mut file = 0i32;
            for ch in row.chars() {
                if let Some(n) = ch.to_digit(10) {
                    file += n as i32;
                } else {
                    anyhow::ensure!((0..8).contains(&file) && (0..8).contains(&rank), "bad FEN {fen}");
                    b.squares[(rank * 8 + file) as usize] = Some(ch);
                    file += 1;
                }
            }
            anyhow::ensure!(file == 8, "bad FEN row {row}");
            rank -= 1;
        }
        anyhow::ensure!(rank == -1, "bad FEN {fen}");
        b.white_to_move = parts.next() != Some("b");
        Ok(b)
    }

    pub fn at(&self, sq: Sq) -> Option<char> {
        self.squares.get(sq as usize).copied().flatten()
    }
}

pub fn parse_sq(s: &str) -> anyhow::Result<Sq> {
    let b = s.as_bytes();
    anyhow::ensure!(b.len() == 2 && (b'a'..=b'h').contains(&b[0]) && (b'1'..=b'8').contains(&b[1]), "bad square {s}");
    Ok((b[1] - b'1') * 8 + (b[0] - b'a'))
}

pub fn name(sq: Sq) -> String {
    format!("{}{}", (b'a' + sq % 8) as char, (b'1' + sq / 8) as char)
}

/// The atlas cell of a piece (tools/toys/carol/pieces.py: king queen rook bishop knight pawn).
pub fn cell(p: char) -> u32 {
    match p.to_ascii_lowercase() {
        'k' => 0,
        'q' => 1,
        'r' => 2,
        'b' => 3,
        'n' => 4,
        _ => 5,
    }
}

#[derive(Clone, Debug)]
pub struct Ply {
    pub from: Sq,
    pub to: Sq,
    pub san: String,
    pub after: Board,
    pub capture: bool,
}

impl Ply {
    pub fn squares(&self) -> (Sq, Sq) {
        (self.from, self.to)
    }

    /// Whether a move from `from` to `to` on `board` is this ply: the same squares, or, when this ply
    /// castles, the king onto the rook it castles with.
    pub fn accepts(&self, board: &Board, from: Sq, to: Sq) -> bool {
        if from != self.from {
            return false;
        }
        if to == self.to {
            return true;
        }
        let king = board.at(from).is_some_and(|p| p.eq_ignore_ascii_case(&'k'));
        let castles = king && (self.from as i32 - self.to as i32).abs() == 2;
        if castles {
            let rank = self.from / 8;
            let rook = if self.to % 8 == 6 { rank * 8 + 7 } else { rank * 8 };
            return to == rook;
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct Drill {
    pub group: String,
    /// The side the reader plays (and the board is drawn from).
    pub white: bool,
    pub start: Board,
    /// Plies played at once when the drill opens.
    pub auto: usize,
    pub plies: Vec<Ply>,
}

impl Drill {
    /// Where a square is drawn: column and row from the screen's top left.
    pub fn screen(&self, sq: Sq) -> (u8, u8) {
        let (f, r) = (sq % 8, sq / 8);
        if self.white { (f, 7 - r) } else { (7 - f, r) }
    }

    pub fn from_screen(&self, c: u8, r: u8) -> Sq {
        if self.white { (7 - r) * 8 + c } else { r * 8 + (7 - c) }
    }
}

#[derive(Deserialize)]
struct RawPly {
    uci: String,
    san: String,
    after: String,
}

#[derive(Deserialize)]
struct RawDrill {
    group: String,
    orient: String,
    fen: String,
    auto: usize,
    plies: Vec<RawPly>,
}

pub fn load(bytes: &[u8]) -> anyhow::Result<Vec<Drill>> {
    let raw: Vec<RawDrill> = serde_json::from_slice(bytes)?;
    let mut out = Vec::new();
    for d in raw {
        let start = Board::from_fen(&d.fen)?;
        let mut plies = Vec::new();
        let mut before = start.clone();
        for p in d.plies {
            let after = Board::from_fen(&p.after)?;
            let from = parse_sq(&p.uci[0..2])?;
            let to = parse_sq(&p.uci[2..4])?;
            let capture = before.at(to).is_some() || p.san.contains('x');
            before = after.clone();
            plies.push(Ply { from, to, san: p.san, after, capture });
        }
        out.push(Drill { group: d.group, white: d.orient == "w", start, auto: d.auto, plies });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drills() -> Vec<Drill> {
        load(include_bytes!("../data/drills.json")).unwrap()
    }

    #[test]
    fn every_ply_moves_its_piece() {
        for (i, d) in drills().iter().enumerate() {
            let mut b = d.start.clone();
            for p in &d.plies {
                let piece = b.at(p.from).unwrap_or_else(|| panic!("drill {i} {}: nothing on {}", p.san, name(p.from)));
                assert_eq!(piece.is_ascii_uppercase(), b.white_to_move, "drill {i} {}: wrong side", p.san);
                assert!(p.after.at(p.from).is_none(), "drill {i} {}", p.san);
                assert!(p.after.at(p.to).is_some(), "drill {i} {}", p.san);
                assert_eq!(p.after.white_to_move, !b.white_to_move);
                b = p.after.clone();
            }
            assert!(d.auto < d.plies.len());
        }
    }

    #[test]
    fn the_reader_has_a_move_in_every_drill() {
        for d in drills() {
            let b = if d.auto > 0 { d.plies[d.auto - 1].after.clone() } else { d.start.clone() };
            let mut side = b.white_to_move;
            let mut yours = 0;
            for _ in d.auto..d.plies.len() {
                if side == d.white {
                    yours += 1;
                }
                side = !side;
            }
            assert!(yours > 0);
        }
    }

    #[test]
    fn castling_by_the_king_onto_its_rook() {
        let d = &drills()[0];
        let k = d.plies.iter().position(|p| p.san == "O-O").unwrap();
        let before = &d.plies[k - 1].after;
        let p = &d.plies[k];
        assert!(p.accepts(before, parse_sq("e8").unwrap(), parse_sq("g8").unwrap()));
        assert!(p.accepts(before, parse_sq("e8").unwrap(), parse_sq("h8").unwrap()));
        assert!(!p.accepts(before, parse_sq("e8").unwrap(), parse_sq("f8").unwrap()));
    }

    #[test]
    fn screen_and_back() {
        for d in drills().iter().take(8) {
            for sq in 0..64 {
                let (c, r) = d.screen(sq);
                assert_eq!(d.from_screen(c, r), sq);
            }
        }
    }
}
