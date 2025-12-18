#[derive(Clone, Copy, Debug)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectI {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    margin: i32,
    group_h: i32,
    group_w_left: i32,
    gap: i32,
    group_w_right: i32,
}

impl Layout {
    pub const fn new() -> Self {
        Self {
            margin: 12,
            group_h: 170,
            group_w_left: 240,
            gap: 12,
            group_w_right: 260,
        }
    }

    pub const fn left_x(self) -> i32 {
        self.margin
    }

    pub const fn top_y(self) -> i32 {
        self.margin
    }

    pub const fn right_x(self) -> i32 {
        self.margin + self.group_w_left + self.gap
    }

    pub const fn group_h(self) -> i32 {
        self.group_h
    }

    pub const fn group_w_left(self) -> i32 {
        self.group_w_left
    }

    pub const fn group_w_right(self) -> i32 {
        self.group_w_right
    }
}
