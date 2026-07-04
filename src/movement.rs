#[derive(PartialEq, Eq, Debug)]
pub enum Direction {
    Forward,
    Backward,
}

pub enum Movement {
    Move,
    Extend,
}
