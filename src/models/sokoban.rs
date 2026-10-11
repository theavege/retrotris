use rand::RngExt;

/// Fixed timestep for the simulation (kept for API symmetry with other games;
/// Sokoban is turn-based — every push happens on a discrete input event).
pub const TICK: f32 = 1.0 / 60.0;

/// One classic Microban-style level as text rows:
/// `#` wall, `@` player, `$` crate, `.` goal, `*` crate on goal, `+` player
/// on goal, anything else is floor. Levels may differ in size and have ragged
/// rows; the parser pads them to the widest line.
pub type LevelSpec = &'static [&'static str];

/// A hand-picked set of small beginner levels (public-domain style layouts).
pub const LEVELS: &[LevelSpec] = &[
    &[
        "########",
        "#      #",
        "#.$$$ @#",
        "#  ....#",
        "#      #",
        "########",
    ],
    &[
        "##########",
        "#   #    #",
        "# $ # .  #",
        "#  @$  . #",
        "## ## ## #",
        "#        #",
        "##########",
    ],
    &[
        "######  ####",
        "#    #  #  #",
        "# $      $ #",
        "#  # ## #  #",
        "@  .  .  # #",
        "#### ###   #",
        "   #     ###",
        "   #######  ",
    ],
];

/// Board state: static walls/goals plus dynamic player/box positions, stored
/// as flat row-major grids so bounds checks stay simple and single-sourced.
#[derive(Default)]
pub struct Sokoban {
    rows: usize,
    cols: usize,
    walls: Vec<bool>,
    goals: Vec<bool>,
    boxes: Vec<bool>,
    player: usize,
    moves: u32,
    pushes: u32,
}

impl Sokoban {
    /// Parse one of the `LEVELS` text maps into flat board arrays.
    pub fn load(spec: &[&str]) -> Self {
        let rows = spec.len();
        let cols = spec.iter().map(|r| r.chars().count()).max().unwrap_or(0);
        let mut board = Self {
            rows,
            cols,
            walls: vec![false; rows * cols],
            goals: vec![false; rows * cols],
            boxes: vec![false; rows * cols],
            ..Default::default()
        };
        for (r, line) in spec.iter().enumerate() {
            for (c, ch) in line.chars().enumerate() {
                let i = r * cols + c;
                match ch {
                    '#' => board.walls[i] = true,
                    '.' => board.goals[i] = true,
                    '$' => board.boxes[i] = true,
                    '*' => {
                        board.boxes[i] = true;
                        board.goals[i] = true;
                    }
                    '@' => board.player = i,
                    '+' => {
                        board.player = i;
                        board.goals[i] = true;
                    }
                    _ => {}
                }
            }
        }
        board
    }
    pub fn level(index: usize) -> Self {
        Self::load(LEVELS[index % LEVELS.len()])
    }
    pub fn dims(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
    fn inside(&self, i: usize) -> bool {
        i < self.rows * self.cols
    }
    /// Flat index of the cell at (row, col), if it exists.
    fn at(&self, r: usize, c: usize) -> Option<usize> {
        if r < self.rows && c < self.cols {
            Some(r * self.cols + c)
        } else {
            None
        }
    }
    pub fn wall(&self, r: usize, c: usize) -> bool {
        self.at(r, c).is_some_and(|i| self.walls[i])
    }
    pub fn goal(&self, r: usize, c: usize) -> bool {
        self.at(r, c).is_some_and(|i| self.goals[i])
    }
    pub fn crate_at(&self, r: usize, c: usize) -> bool {
        self.at(r, c).is_some_and(|i| self.boxes[i])
    }
    pub fn player(&self) -> (usize, usize) {
        (self.player / self.cols.max(1), self.player % self.cols.max(1))
    }
    pub fn stats(&self) -> (u32, u32) {
        (self.moves, self.pushes)
    }
    /// Move the player by `(dr, dc)`; drags one box along when it is directly
    /// in front and the space behind it is free. Returns true when something
    /// actually moved. Horizontal steps must not wrap across grid rows.
    pub fn push(&mut self, dr: isize, dc: isize) -> bool {
        let cols = self.cols as isize;
        let base = self.player as isize;
        // Compute the target index arithmetically; any overflow or wrap lands
        // outside the grid and is rejected by `inside`.
        let raw = base + dr * cols + dc;
        let wrapped = dr == 0 && (raw / cols) != (base / cols);
        let Ok(tidx) = usize::try_from(raw) else {
            return false;
        };
        if wrapped || !self.inside(tidx) || self.walls[tidx] {
            return false;
        }
        if self.boxes[tidx] {
            let braw = raw + dr * cols + dc;
            let bwrapped = dr == 0 && (braw / cols) != (raw / cols);
            let Ok(bidx) = usize::try_from(braw) else {
                return false;
            };
            if bwrapped || !self.inside(bidx) || self.walls[bidx] || self.boxes[bidx] {
                return false;
            }
            self.boxes[tidx] = false;
            self.boxes[bidx] = true;
            self.pushes += 1;
        }
        self.player = tidx;
        self.moves += 1;
        true
    }
    /// All boxes standing on goals means the room is solved.
    pub fn solved(&self) -> bool {
        self.boxes
            .iter()
            .zip(self.goals.iter())
            .all(|(&b, &g)| !b || g)
            && self.boxes.iter().any(|&b| b)
    }
}

/// Game score: current level index and the total number of rooms ever solved
/// (persisted between sessions as the player's "best").
#[derive(Default)]
pub struct Score {
    pub level: usize,
    pub best: i32,
}

impl Score {
    pub fn reset(&mut self) {
        self.level = 0;
    }
    /// Advance to the next level after a solve; returns the new index.
    pub fn solved(&mut self, level_count: usize) -> usize {
        self.best += 1;
        self.level = (self.level + 1) % level_count;
        self.level
    }
}
