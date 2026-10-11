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
    /// Random cell that is not occupied by the snake and lies inside the field.
    pub fn new(snake: &Snake, field: (i32, i32)) -> Self {
        let mut rng = rand::rng();
        loop {
            let x = rng.random_range(0..field.0.max(1));
            let y = rng.random_range(0..field.1.max(1));
            if !snake.body.contains(&(x, y)) {
                return Self(x, y);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_levels_use_exclusive_ranges() {
        let mut s = Score::default();
        for _ in 0..5 {
            s.inc();
        }
        s.update();
        assert_eq!(s.level(), 1);
        for _ in 0..25 {
            s.inc();
        }
        s.update();
        assert_eq!(s.level(), 3);
        assert_eq!(s.best, 30);
    }

    #[test]
    fn snake_starts_centered_moving_right() {
        let mut snake = Snake::default();
        snake.set((20, 15));
        assert_eq!(snake.body[0], (10, 7));
        assert_eq!(snake.body.len(), 3);
        assert_eq!(snake.direction, (1, 0));
    }

    #[test]
    fn wrap_at_edges_inclusive() {
        let mut snake = Snake::default();
        snake.set((4, 4));
        // walk right past the edge
        for _ in 0..6 {
            snake.update((4, 4));
        }
        for (x, y) in &snake.body {
            assert!((0..4).contains(x) && (0..4).contains(y), "out of bounds");
        }
    }

    #[test]
    fn buffered_turn_applies_once_and_blocks_reverse() {
        let mut snake = Snake::default();
        snake.set((10, 10)); // head (5,5), body extends left, moving right
        snake.direction(0, -1); // buffer a turn up
        let head = snake.update((10, 10)).unwrap();
        assert_eq!(head, (5, 4)); // moved up, not right
                                  // the buffered turn was consumed: keep going up
        let head2 = snake.update((10, 10)).unwrap();
        assert_eq!(head2, (5, 3));
        // reversing into the neck is ignored while moving up
        snake.direction(0, 1); // down == reverse of up -> must be dropped
        let head3 = snake.update((10, 10)).unwrap();
        assert_eq!(head3, (5, 2)); // still moving up
    }

    #[test]
    fn apple_never_spawns_on_snake() {
        let mut snake = Snake::default();
        snake.set((8, 8));
        for _ in 0..200 {
            let Apple(x, y) = Apple::new(&snake, (8, 8));
            assert!(!snake.body.contains(&(x, y)));
            assert!((0..8).contains(&x) && (0..8).contains(&y));
        }
    }
}
