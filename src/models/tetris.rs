//! Core logic for Tetris: tetrominoes, the 7-bag randomizer, SRS wall-kicks,
//! grid, line clearing, hold slot, ghost piece and score bookkeeping.

/// Playfield width in cells.
pub const FIELD_COLS: usize = 10;
/// Playfield height in cells.
pub const FIELD_ROWS: usize = 20;
/// Number of distinct tetromino kinds (O, I, S, Z, T, J, L).
pub const PIECE_COUNT: usize = 7;
/// Number of rotation states per kind.
pub const ROTATIONS: usize = 4;
/// Spawn column of the piece origin (centers a 4-wide bounding box on 10 cols).
pub const SPAWN_X: i32 = 3;
/// Grid row where freshly spawned pieces appear (just below the top edge).
pub const SPAWN_ROW: i32 = 1;
/// Lines needed to raise the level by one.
pub const LINES_PER_LEVEL: u64 = 10;
/// Gravity interval at level 0, seconds per step.
pub const BASE_GRAVITY: f32 = 0.8;
/// Per-level gravity acceleration factor (classic guideline curve).
pub const GRAVITY_FACTOR: f32 = 0.82;
/// Lock delay: a resting piece tolerates this many failed down-steps.
pub const LOCK_DELAY: u8 = 6;

type Cell = (i32, i32);

/// All rotation states of the seven tetrominoes as cell offsets inside their
/// bounding box (square 2x2 for O, 4x4 for I, 3x3 for the rest - SRS boxes).
/// Order: O, I, S, Z, T, J, L; states order: spawn, clockwise, 180, CCW.
pub const SHAPES: [[[Cell; 4]; ROTATIONS]; PIECE_COUNT] = [
    // O (no rotation)
    [[(0, 0), (1, 0), (0, 1), (1, 1)]; ROTATIONS],
    // I
    [
        [(0, 1), (1, 1), (2, 1), (3, 1)], // spawn
        [(2, 0), (2, 1), (2, 2), (2, 3)], // clockwise
        [(0, 2), (1, 2), (2, 2), (3, 2)], // 180
        [(1, 0), (1, 1), (1, 2), (1, 3)], // counter-clockwise
    ],
    // S
    [
        [(0, 1), (1, 1), (1, 0), (2, 0)],
        [(1, 0), (1, 1), (2, 1), (2, 2)],
        [(0, 2), (1, 2), (1, 1), (2, 1)],
        [(0, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // Z
    [
        [(0, 0), (1, 0), (1, 1), (2, 1)],
        [(2, 0), (2, 1), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (1, 2), (2, 2)],
        [(1, 0), (1, 1), (0, 1), (0, 2)],
    ],
    // T
    [
        [(1, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (2, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (1, 2)],
        [(1, 0), (0, 1), (1, 1), (1, 2)],
    ],
    // J
    [
        [(0, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (2, 0), (1, 1), (1, 2)],
        [(0, 1), (1, 1), (2, 1), (2, 2)],
        [(1, 0), (1, 1), (0, 2), (1, 2)],
    ],
    // L
    [
        [(2, 0), (0, 1), (1, 1), (2, 1)],
        [(1, 0), (1, 1), (1, 2), (2, 2)],
        [(0, 1), (1, 1), (2, 1), (0, 2)],
        [(0, 0), (1, 0), (1, 1), (1, 2)],
    ],
];

/// SRS wall-kick offsets. Indexed `[direction][from_state]` where direction
/// 0 = clockwise (R), 1 = counter-clockwise (L); each entry lists the five
/// test offsets `(dx, dy)` with y pointing up (negated when applied).
/// The O-piece never kicks.
const KICKS_R: [[[(i32, i32); 5]; 4]; 2] = [
    // JLSTZ
    [
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)], // 0 -> R
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],     // 1 -> R
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],    // 2 -> R
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],  // 3 -> R
    ],
    // I
    [
        [(0, 0), (-2, 0), (1, 0), (-2, -1), (1, 2)], // 0 -> R
        [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)], // 1 -> R
        [(0, 0), (2, 0), (-1, 0), (2, 1), (-1, -2)], // 2 -> R
        [(0, 0), (1, 0), (-2, 0), (1, -2), (-2, 1)], // 3 -> R
    ],
];
const KICKS_L: [[[(i32, i32); 5]; 4]; 2] = [
    // JLSTZ
    [
        [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],   // 0 -> L
        [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)], // 1 -> L
        [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)], // 2 -> L
        [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],    // 3 -> L
    ],
    // I
    [
        [(0, 0), (-1, 0), (2, 0), (-1, 2), (2, -1)], // 0 -> L
        [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)], // 1 -> L
        [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)], // 2 -> L
        [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)], // 3 -> L
    ],
];

/// Classic line-clear scores, indexed by number of simultaneous clears.
const LINE_SCORES: [i32; 5] = [0, 100, 300, 500, 800];
/// Extra combo bonus for multi-line clears, scaled by the current level.
const LINE_BONUSES: [i32; 5] = [0, 0, 50, 200, 400];

/// Score & level bookkeeping.
#[derive(Default)]
pub struct Score {
    pub current: i32,
    pub best: i32,
    pub lines: u64,
}

impl Score {
    pub fn reset(&mut self) {
        self.current = 0;
        self.lines = 0;
    }
    /// Award points for clearing `n` lines at once.
    pub fn add_lines(&mut self, n: usize) {
        let n = n.min(4);
        self.current += LINE_SCORES[n] + LINE_BONUSES[n] * self.level() as i32;
        self.lines += n as u64;
        if self.current > self.best {
            self.best = self.current;
        }
    }
    pub fn level(&self) -> u64 {
        self.lines / LINES_PER_LEVEL
    }
    /// Seconds per gravity step at the current level.
    pub fn gravity(&self) -> f32 {
        BASE_GRAVITY * GRAVITY_FACTOR.powf(self.level() as f32)
    }
}

/// A falling tetromino: kind, rotation state and origin on the grid.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: usize,
    pub rot: usize,
    pub x: i32,
    pub y: i32,
}

impl Default for Piece {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Piece {
    /// Spawns near the top of the field, horizontally centered.
    pub fn new(kind: usize) -> Self {
        Self { kind, rot: 0, x: SPAWN_X, y: SPAWN_ROW }
    }
    /// Absolute grid cells occupied by this piece.
    pub fn cells(&self) -> [Cell; 4] {
        SHAPES[self.kind][self.rot].map(|(i, j)| (self.x + i, self.y + j))
    }
    pub fn moved(&self, dx: i32, dy: i32) -> Self {
        Self { x: self.x + dx, y: self.y + dy, ..*self }
    }
    /// Rotation candidates (current first, then SRS kicks) for turning
    /// clockwise (`cw = true`) or counter-clockwise from the current state.
    pub fn rotations(&self, cw: bool) -> Vec<Self> {
        if self.kind == 0 {
            return vec![*self]; // O-piece has no meaningful rotation
        }
        let table = if cw { KICKS_R } else { KICKS_L }[if self.kind == 1 { 1 } else { 0 }][self.rot];
        let next = if cw { (self.rot + 1) % ROTATIONS } else { (self.rot + ROTATIONS - 1) % ROTATIONS };
        table
            .iter()
            .map(|&(dx, dy)| Self { rot: next, ..self.moved(dx, -dy) })
            .collect()
    }
}

/// The official 7-bag randomizer: every bag contains each tetromino once.
pub struct Bag {
    queue: Vec<usize>,
}

impl Default for Bag {
    fn default() -> Self {
        Self { queue: Vec::with_capacity(PIECE_COUNT) }
    }
}

impl Bag {
    pub fn next(&mut self) -> usize {
        if self.queue.is_empty() {
            self.fill();
        }
        self.queue.remove(0)
    }
    fn fill(&mut self) {
        self.queue.extend(0..PIECE_COUNT);
        self.queue.shuffle(&mut rand::rng());
    }
}

use rand::seq::SliceRandom;

/// Locked cells of the playfield plus collision and render helpers.
pub struct Grid(Vec<[Option<usize>; FIELD_COLS]>);

impl Default for Grid {
    fn default() -> Self {
        Self(vec![[None; FIELD_COLS]; FIELD_ROWS])
    }
}

impl Grid {
    /// True when every cell of the piece lies inside the walls/floor and does
    /// not overlap locked cells.
    pub fn fits(&self, piece: &Piece) -> bool {
        piece.cells().iter().all(|&(x, y)| {
            x >= 0
                && (x as usize) < FIELD_COLS
                && y >= 0
                && (y as usize) < FIELD_ROWS
                && self.0[y as usize][x as usize].is_none()
        })
    }
    /// Merge the piece into the locked cells (out-of-bounds cells are skipped).
    pub fn lock(&mut self, piece: &Piece) {
        for &(x, y) in &piece.cells() {
            if x >= 0 && (x as usize) < FIELD_COLS && y >= 0 && (y as usize) < FIELD_ROWS {
                self.0[y as usize][x as usize] = Some(piece.kind);
            }
        }
    }
    /// Remove full rows, shifting the remaining stack towards the bottom,
    /// and return the number of cleared lines.
    pub fn clear_lines(&mut self) -> usize {
        let kept: Vec<[Option<usize>; FIELD_COLS]> =
            self.0.iter().filter(|row| !row.contains(&None)).cloned().collect();
        if kept.is_empty() {
            return 0;
        }
        let mut rows: Vec<[Option<usize>; FIELD_COLS]> = self
            .0
            .iter()
            .filter(|row| row.contains(&None))
            .cloned()
            .collect();
        rows.extend(std::iter::repeat_n([None; FIELD_COLS], kept.len()));
        self.0 = rows;
        kept.len()
    }
    /// Render snapshot: locked cells overlaid by the active piece.
    pub fn draw(&self, curr: &Piece) -> Vec<[Option<usize>; FIELD_COLS]> {
        let mut field = self.0.clone();
        for (x, y) in curr.cells() {
            if y >= 0 && (y as usize) < FIELD_ROWS {
                field[y as usize][x as usize] = Some(curr.kind);
            }
        }
        field
    }
    /// Ghost projection of the piece straight onto the stack.
    pub fn ghost(&self, piece: &Piece) -> Piece {
        let mut ghost = *piece;
        while self.fits(&ghost.moved(0, 1)) {
            ghost = ghost.moved(0, 1);
        }
        ghost
    }
}

/// Game rules: two-piece pipeline (next + hold), input buffering, DAS-free
/// discrete moves, soft/hard drop, lock delay and game-over detection.
/// Coordinates are grid cells; the layer that renders knows the pixel size.
pub struct Tetris {
    bag: Bag,
    pub grid: Grid,
    pub curr: Piece,
    pub next_kind: usize,
    pub hold_kind: Option<usize>,
    pub hold_used: bool,
    buffered_rot: Option<bool>,
    buffered_left: bool,
    buffered_right: bool,
    lock_timer: u8,
    pub dropped: bool,
}

impl Default for Tetris {
    fn default() -> Self {
        let mut tetris = Self {
            bag: Bag::default(),
            grid: Grid::default(),
            curr: Piece::new(0),
            next_kind: 0,
            hold_kind: None,
            hold_used: false,
            buffered_rot: None,
            buffered_left: false,
            buffered_right: false,
            lock_timer: 0,
            dropped: false,
        };
        tetris.spawn();
        tetris
    }
}

impl Tetris {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    /// Pull the queued piece in; game over if it spawns into the stack.
    fn spawn(&mut self) {
        self.curr = Piece::new(self.next_kind);
        self.next_kind = self.bag.next();
        self.hold_used = false;
        self.lock_timer = 0;
        self.dropped = false;
    }
    pub fn rotate(&mut self, cw: bool) {
        self.buffered_rot = Some(cw);
    }
    pub fn move_x(&mut self, dir: i32) {
        match dir {
            -1 => self.buffered_left = true,
            1 => self.buffered_right = true,
            _ => {}
        }
    }
    /// Swap the active piece with the hold slot (once per piece).
    pub fn hold(&mut self) {
        if self.hold_used {
            return;
        }
        self.hold_used = true;
        let taken = std::mem::replace(&mut self.hold_kind, Some(self.curr.kind));
        match taken {
            Some(kind) => self.curr = Piece::new(kind),
            // First hold: stash the active piece and pull the queued one in.
            None => {
                self.curr = Piece::new(self.next_kind);
                self.next_kind = self.bag.next();
            }
        }
    }
    pub fn soft_drop(&mut self) {
        self.step(true);
    }
    /// Slam the piece down to its ghost position and lock immediately.
    pub fn hard_drop(&mut self) {
        let ghost = self.grid.ghost(&self.curr);
        self.dropped = true;
        self.curr = ghost;
        self.finish();
    }
    /// One gravity tick: apply buffered inputs, then try to fall one row.
    /// Returns false when the game is over (a new piece spawns into the stack).
    pub fn update(&mut self) -> bool {
        self.step(false)
    }
    fn step(&mut self, soft: bool) -> bool {
        self.apply_input();
        if self.grid.fits(&self.curr.moved(0, 1)) {
            self.curr = self.curr.moved(0, 1);
            if soft {
                self.lock_timer = 0; // moving resets the lock grace
            }
            true
        } else if self.lock_timer < LOCK_DELAY {
            self.lock_timer += 1; // grace period: allow last-moment slides
            true
        } else {
            self.finish()
        }
    }
    fn apply_input(&mut self) {
        if let Some(cw) = self.buffered_rot.take() {
            if let Some(rotated) = self.curr.rotations(cw).into_iter().find(|p| self.grid.fits(p)) {
                self.curr = rotated;
                self.lock_timer = 0;
            }
        }
        for dir in [self.buffered_left.then_some(-1), self.buffered_right.then_some(1)]
            .into_iter()
            .flatten()
        {
            self.buffered_left = false;
            self.buffered_right = false;
            let shifted = self.curr.moved(dir, 0);
            if self.grid.fits(&shifted) {
                self.curr = shifted;
                self.lock_timer = 0;
            }
        }
    }
    /// Lock the piece, clear lines, spawn the next one. False on game over.
    fn finish(&mut self) -> bool {
        self.grid.lock(&self.curr);
        self.spawn();
        if !self.grid.fits(&self.curr) {
            return false;
        }
        true
    }
    pub fn ghost(&self) -> Piece {
        self.grid.ghost(&self.curr)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bag_yields_each_kind_once_per_seven() {
        let mut bag = Bag::default();
        let mut seen = [0u8; PIECE_COUNT];
        for _ in 0..PIECE_COUNT {
            seen[bag.next()] += 1;
        }
        assert!(seen.iter().all(|&c| c == 1));
    }

    #[test]
    fn shapes_have_four_cells_in_bounds() {
        for kind in 0..PIECE_COUNT {
            for rot in SHAPES[kind] {
                for (x, y) in rot {
                    assert!((0..4).contains(&x) && (0..4).contains(&y));
                }
            }
        }
    }

    #[test]
    fn ghost_falls_to_the_floor() {
        let grid = Grid::default();
        let piece = Piece::new(1); // I horizontal
        let ghost = grid.ghost(&piece);
        assert_eq!(ghost.cells().iter().map(|c| c.1).max(), Some(FIELD_ROWS as i32 - 1));
    }

    #[test]
    fn line_clear_shifts_stack() {
        let mut grid = Grid::default();
        // Vertical I at column 8 occupies rows 16..=19.
        grid.lock(&Piece { kind: 1, rot: 1, x: 6, y: FIELD_ROWS as i32 - 4 });
        assert_eq!(grid.clear_lines(), 0);
        // Fill row 19 completely except column 8 (already taken by the I).
        for x in 0..FIELD_COLS as i32 {
            if x != 8 {
                grid.lock(&Piece { kind: 5, rot: 0, x, y: FIELD_ROWS as i32 - 1 });
            }
        }
        assert_eq!(grid.clear_lines(), 1);
        // The bottom row is gone; everything above it shifted down one cell:
        // the I now ends at row 18 and its former bottom cell moved there too.
        assert_eq!(grid.0[FIELD_ROWS - 2][8], Some(1));
        assert_eq!(grid.0[FIELD_ROWS - 1][8], None);
        assert!(grid.0[FIELD_ROWS - 1].iter().all(|c| c.is_none()));
    }

    #[test]
    fn game_over_when_spawn_blocked() {
        let mut tetris = Tetris::default();
        // Fill the top rows so a new piece cannot spawn.
        for y in 0..3 {
            for x in 0..FIELD_COLS {
                tetris.grid.0[y][x] = Some(0);
            }
        }
        tetris.hard_drop();
        // After locking, spawning into the wall must end the game eventually.
        let mut alive = true;
        for _ in 0..LOCK_DELAY + 5 {
            alive = tetris.update();
            if !alive {
                break;
            }
        }
        assert!(!alive);
    }

    #[test]
    fn score_levels_and_best() {
        let mut score = Score::default();
        score.add_lines(4);
        assert_eq!(score.current, 800);
        assert_eq!(score.best, 800);
        for _ in 0..9 {
            score.add_lines(1);
        }
        assert_eq!(score.level(), 1);
        assert!(score.gravity() < BASE_GRAVITY);
    }

    #[test]
    fn hold_swaps_once_per_piece() {
        let mut tetris = Tetris::default();
        let first = tetris.curr.kind;
        tetris.hold();
        assert_eq!(tetris.hold_kind, Some(first));
        let second = tetris.curr.kind;
        tetris.hold(); // blocked: already used this piece
        assert_eq!(tetris.curr.kind, second);
    }
}
