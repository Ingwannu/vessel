//! How the slider moves: the reference's tween to each stop, the thumb's
//! spring, and the wheel's steps. Times are seconds on the kit's clock.
#![expect(
    clippy::float_arithmetic,
    reason = "easing curves and a spring are float arithmetic; written as formulas they stay checkable against their sources"
)]

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

/// The reference's slide: `cubic-bezier(.23, 1, .32, 1)`.
pub const SLIDE: Bezier = Bezier::new(0.23, 1.0, 0.32, 1.0);
/// The top stop's burst: `cubic-bezier(.25, 1, .5, 1)`.
pub const BURST: Bezier = Bezier::new(0.25, 1.0, 0.5, 1.0);
/// CSS `ease`, the reference's field reveal.
pub const EASE: Bezier = Bezier::new(0.25, 0.1, 0.25, 1.0);
/// CSS `ease-in-out`.
pub const EASE_IN_OUT: Bezier = Bezier::new(0.42, 0.0, 0.58, 1.0);
/// CSS `ease-out`.
pub const EASE_OUT: Bezier = Bezier::new(0.0, 0.0, 0.58, 1.0);

impl Bezier {
    /// The curve through (0, 0), the two control points and (1, 1).
    #[must_use]
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// One coordinate of the curve at parameter `s`, for control values
    /// `a` and `b`.
    fn coordinate(a: f32, b: f32, s: f32) -> f32 {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    }

    fn slope(a: f32, b: f32, s: f32) -> f32 {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    }

    /// The eased progress at time fraction `t`, 0 to 1: the curve's `y`
    /// where its `x` is `t` (Newton's steps, bisection if they stall).
    #[must_use]
    pub fn at(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let mut s = t;
        for _ in 0..8 {
            let error = Self::coordinate(self.x1, self.x2, s) - t;
            if error.abs() < 1e-6 {
                return Self::coordinate(self.y1, self.y2, s);
            }
            let slope = Self::slope(self.x1, self.x2, s);
            if slope.abs() < 1e-6 {
                break;
            }
            s = (s - error / slope).clamp(0.0, 1.0);
        }
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        s = t;
        for _ in 0..30 {
            let x = Self::coordinate(self.x1, self.x2, s);
            if (x - t).abs() < 1e-6 {
                break;
            }
            if x < t {
                low = s;
            } else {
                high = s;
            }
            s = low.midpoint(high);
        }
        Self::coordinate(self.y1, self.y2, s)
    }
}

/// The glide to a stop while dragging, seconds.
pub const DRAG_GLIDE: f32 = 0.15;
/// The glide to a stop on a click, a key or the wheel, seconds.
pub const GLIDE: f32 = 0.3;

/// A value gliding from where it was to a target along [`SLIDE`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tween {
    from: f32,
    to: f32,
    start: f32,
    length: f32,
}

impl Tween {
    /// Resting at `value`.
    #[must_use]
    pub const fn rest(value: f32) -> Self {
        Self {
            from: value,
            to: value,
            start: 0.0,
            length: 0.0,
        }
    }

    /// The value at `now`.
    #[must_use]
    pub fn at(&self, now: f32) -> f32 {
        if self.length <= 0.0 || now >= self.start + self.length {
            return self.to;
        }
        let t = ((now - self.start) / self.length).max(0.0);
        self.from + (self.to - self.from) * SLIDE.at(t)
    }

    /// Where it is going.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.to
    }

    /// Whether it still moves at `now`.
    #[must_use]
    pub fn moving(&self, now: f32) -> bool {
        self.length > 0.0 && now < self.start + self.length
    }

    /// Glide from where it is at `now` to `to` over `length` seconds (0:
    /// jump, as with reduced motion). A glide already going to `to` goes
    /// on undisturbed.
    pub fn glide(&mut self, to: f32, now: f32, length: f32) {
        if (to - self.to).abs() < f32::EPSILON {
            return;
        }
        let from = self.at(now);
        *self = Self {
            from,
            to,
            start: now,
            length,
        };
    }
}

/// A spring's constants, as the reference gives them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpringParams {
    /// Stiffness.
    pub stiffness: f32,
    /// Damping.
    pub damping: f32,
    /// Mass.
    pub mass: f32,
}

/// The thumb growing (hovered or pressed).
pub const GROW: SpringParams = SpringParams {
    stiffness: 420.0,
    damping: 38.0,
    mass: 1.0,
};
/// The thumb returning to rest.
pub const SETTLE: SpringParams = SpringParams {
    stiffness: 220.0,
    damping: 26.0,
    mass: 1.0,
};

/// The longest step the spring is integrated in, seconds.
const SPRING_STEP: f32 = 1.0 / 240.0;
/// The longest gap between two frames the spring honours, seconds (a
/// window that was hidden does not fling it).
const LONGEST_FRAME: f32 = 0.1;

/// A damped spring's value and velocity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    /// Its value.
    pub value: f32,
    /// Its velocity, per second.
    pub velocity: f32,
}

impl Spring {
    /// At rest at `value`.
    #[must_use]
    pub const fn rest(value: f32) -> Self {
        Self {
            value,
            velocity: 0.0,
        }
    }

    /// Move it `elapsed` seconds toward `target` (semi-implicit Euler in
    /// steps of at most 1/240 s).
    pub fn step(&mut self, target: f32, params: SpringParams, elapsed: f32) {
        let mut left = elapsed.clamp(0.0, LONGEST_FRAME);
        while left > 0.0 {
            let dt = left.min(SPRING_STEP);
            let force = -params.stiffness * (self.value - target) - params.damping * self.velocity;
            self.velocity += force / params.mass * dt;
            self.value += self.velocity * dt;
            left -= dt;
        }
    }

    /// Whether it has come to rest at `target`.
    #[must_use]
    pub fn settled(&self, target: f32) -> bool {
        (self.value - target).abs() < 1e-3 && self.velocity.abs() < 1e-2
    }
}

/// One wheel step's units (a line counts as 30).
pub const WHEEL_STEP: f32 = 30.0;
/// The pause after which the wheel's count starts over, seconds.
pub const WHEEL_PAUSE: f32 = 0.16;

/// The wheel's running count: a stop per 30 units in one direction.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Wheel {
    sum: f32,
    last: Option<f32>,
}

impl Wheel {
    /// Add `delta` units at `now`; the stops to move (negative: back).
    pub fn add(&mut self, delta: f32, now: f32) -> i32 {
        let paused = self.last.is_none_or(|last| now - last > WHEEL_PAUSE);
        let turned = self.sum * delta < 0.0;
        if paused || turned {
            self.sum = 0.0;
        }
        self.last = Some(now);
        self.sum += delta;
        let mut steps = 0;
        while self.sum >= WHEEL_STEP {
            self.sum -= WHEEL_STEP;
            steps = i32::saturating_add(steps, 1);
        }
        while self.sum <= -WHEEL_STEP {
            self.sum += WHEEL_STEP;
            steps = i32::saturating_sub(steps, 1);
        }
        steps
    }
}
