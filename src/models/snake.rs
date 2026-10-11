use rand::RngExt;

#[derive(Default)]
pub struct Score {
    pub current: i32,
    pub best: i32,
    level: u64,
}

impl Score {
    pub fn inc(&mut self) {
        self.current += 1;
        if self.current > self.best {
            self.best = self.current;
        }
    }
    pub fn level(&self) -> u64 {
        self.level
    }
    pub fn reset(&mut self) {
        self.current = 0;
        self.level = 0;
    }
    pub fn update(&mut self) {
        // NOTE: exclusive ranges on purpose - with inclusive `..=` the snake would
        // never reach level 3 (current == 5, 15, 30 fell into the previous arm).
        self.level = match self.current {
            0..5 => 0,
            5..15 => 1,
            15..30 => 2,
            _ => 3,
        }
    }
}

#[derive(Default)]
pub struct Apple(pub i32, pub i32);

impl Apple {
    pub fn new(field: (i32, i32)) -> Self {
        Self(
            rand::rng().random_range(0..field.0),
            rand::rng().random_range(0..field.1),
        )
    }
}

#[derive(Default)]
pub struct Snake {
    pub body: Vec<(i32, i32)>,
    pub direction: (i32, i32),
    buffered: Option<(i32, i32)>,
}

impl Snake {
    pub fn set(&mut self, field: (i32, i32)) {
        *self = Self {
            body: vec![
                (field.0 / 2, field.1 / 2),
                (field.0 / 2 - 1, field.1 / 2),
                (field.0 / 2 - 2, field.1 / 2),
            ],
            direction: (1, 0),
            buffered: None,
        }
    }
    pub fn update(&mut self, field: (i32, i32)) -> Option<(i32, i32)> {
        if let Some(dir) = self.buffered.take() {
            self.direction = dir;
        }
        let (mut x, mut y) = self.body[0];
        x = check_limit(x + self.direction.0, field.0);
        y = check_limit(y + self.direction.1, field.1);
        if self.body.contains(&(x, y)) {
            None
        } else {
            self.body.insert(0, (x, y));
            Some((x, y))
        }
    }
    pub fn direction(&mut self, x: i32, y: i32) {
        let current = self.buffered.unwrap_or(self.direction);
        if current != (x, y) && current != (-x, -y) {
            self.buffered = Some((x, y));
        }
    }
}

fn check_limit(coord: i32, limit: i32) -> i32 {
    if coord < 0 {
        limit - 1
    } else if coord >= limit {
        0
    } else {
        coord
    }
}
