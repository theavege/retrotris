use crate::*;

/// Fixed timestep for the simulation (kept for API symmetry with other games;
/// Minesweeper itself is event-driven, there is no per-frame physics).
pub const TICK: f32 = 1.0 / 60.0;

/// A single board cell and its cached neighbor-mine count.
#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub mine: bool,
    pub open: bool,
    pub flag: bool,
    pub adj: u8,
}

/// Classic Minesweeper board: `rows x cols` cells with `mines` hidden bombs.
/// The first opened cell is always safe (and its neighbors too), matching the
/// friendly "first click never hits a mine" rule of modern implementations.
#[derive(Default)]
pub struct Board {
    rows: usize,
    cols: usize,
    mines: usize,
    cells: Vec<Cell>,
    placed: bool,
    pub lost: bool,
    pub won: bool,
}

impl Board {
    pub fn new(rows: usize, cols: usize, mines: usize) -> Self {
        Self {
            rows,
            cols,
            mines,
            cells: vec![Cell::default(); rows * cols],
            placed: false,
            lost: false,
            won: false,
        }
    }
    pub fn cell(&self, r: usize, c: usize) -> &Cell {
        &self.cells[r * self.cols + c]
    }
    /// Hidden mines that are neither opened nor flagged — the HUD counter.
    pub fn remaining(&self) -> usize {
        self.cells
            .iter()
            .filter(|c| c.mine && !c.open && !c.flag)
            .count()
    }
    /// Iterator over every in-bounds cell coordinate, row-major.
    pub fn coords(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.cells.len()).map(|i| (i / self.cols, i % self.cols))
    }
    fn neighbors(&self, r: usize, c: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::with_capacity(8);
        for dr in -1isize..=1 {
            for dc in -1isize..=1 {
                if dr == 0 && dc == 0 {
                    continue;
                }
                let nr = r as isize + dr;
                let nc = c as isize + dc;
                if nr >= 0 && nc >= 0 && (nr as usize) < self.rows && (nc as usize) < self.cols {
                    out.push((nr as usize, nc as usize));
                }
            }
        }
        out
    }
    /// Scatter mines avoiding the safe zone around the first click.
    fn place(&mut self, sr: usize, sc: usize, rng: &mut impl RngExt) {
        let safe: Vec<usize> = std::iter::once(sr * self.cols + sc)
            .chain(self.neighbors(sr, sc).into_iter().map(|(r, c)| r * self.cols + c))
            .collect();
        // Collect every cell outside the safe zone, shuffle it Fisher-Yates
        // style and take the first `mines` entries as bomb positions.
        let mut pool: Vec<usize> = (0..self.cells.len())
            .filter(|i| !safe.contains(i))
            .collect();
        for i in 1..pool.len() {
            let j = rng.random_range(0..=i);
            pool.swap(i, j);
        }
        for &idx in pool.iter().take(self.mines) {
            self.cells[idx].mine = true;
        }
        for i in 0..self.cells.len() {
            let (r, c) = (i / self.cols, i % self.cols);
            let adj = self
                .neighbors(r, c)
                .iter()
                .filter(|&&(nr, nc)| self.cells[nr * self.cols + nc].mine)
                .count();
            self.cells[i].adj = adj as u8;
        }
        self.placed = true;
    }
    /// Reveal a cell: opens it (flood-filling empty regions), ends the game on
    /// a mine. Returns false when nothing changed (cell already open/flagged).
    pub fn open(&mut self, r: usize, c: usize, rng: &mut impl RngExt) -> bool {
        if self.lost || self.won {
            return false;
        }
        let cell = &self.cells[r * self.cols + c];
        if cell.open || cell.flag {
            return false;
        }
        if !self.placed {
            self.place(r, c, rng);
        }
        if self.cells[r * self.cols + c].mine {
            self.cells[r * self.cols + c].open = true;
            self.lost = true;
            return true;
        }
        // Flood fill from the clicked cell through zero-count regions.
        let mut stack = vec![(r, c)];
        while let Some((cr, cc)) = stack.pop() {
            let i = cr * self.cols + cc;
            if self.cells[i].open || self.cells[i].flag || self.cells[i].mine {
                continue;
            }
            self.cells[i].open = true;
            if self.cells[i].adj == 0 {
                stack.extend(self.neighbors(cr, cc));
            }
        }
        self.check_win();
        true
    }
    /// Toggle a flag on a closed cell.
    pub fn toggle_flag(&mut self, r: usize, c: usize) -> bool {
        if self.lost || self.won {
            return false;
        }
        let cell = &mut self.cells[r * self.cols + c];
        if cell.open {
            return false;
        }
        cell.flag = !cell.flag;
        true
    }
    /// Chord (middle click): when the number on an open cell equals the count
    /// of adjacent flags, open all remaining closed neighbors at once.
    pub fn chord(&mut self, r: usize, c: usize, rng: &mut impl RngExt) -> bool {
        if self.lost || self.won || !self.placed {
            return false;
        }
        if !self.cells[r * self.cols + c].open || self.cells[r * self.cols + c].adj == 0 {
            return false;
        }
        let n = self.cells[r * self.cols + c].adj as usize;
        let flags = self
            .neighbors(r, c)
            .iter()
            .filter(|&&(nr, nc)| self.cells[nr * self.cols + nc].flag)
            .count();
        if flags != n {
            return false;
        }
        let closed: Vec<(usize, usize)> = self
            .neighbors(r, c)
            .into_iter()
            .filter(|&(nr, nc)| !self.cells[nr * self.cols + nc].open)
            .collect();
        let mut changed = false;
        for (nr, nc) in closed {
            if self.open(nr, nc, rng) {
                changed = true;
            }
            if self.lost {
                break;
            }
        }
        changed
    }
    /// Win when every non-mine cell is revealed.
    fn check_win(&mut self) {
        if self.cells.iter().all(|c| c.mine || c.open) {
            self.won = true;
        }
    }
}

/// Game score: elapsed seconds, remaining flags and persisted best time
/// (lower is better — the opposite of most other retrotris games).
#[derive(Default)]
pub struct Score {
    pub time: f32,
    pub best: i32,
}

impl Score {
    pub fn reset(&mut self) {
        self.time = 0.0;
    }
    pub fn tick(&mut self, dt: f32) {
        self.time += dt;
    }
    /// Record a win; returns true when it is a new board-best time.
    pub fn finish(&mut self) -> bool {
        let secs = self.time.round() as i32;
        if self.best == 0 || secs < self.best {
            self.best = secs;
            return true;
        }
        false
    }
}
