use cgmath::{Vector2, Zero};

use bounds::Rect;

pub struct MouseInfo {
    /// The current position of the mouse
    pub curr: Vector2<f32>,

    /// The last position of the mouse
    pub last: Vector2<f32>,

    /// The last position that the user clicked
    pub clicked: Vector2<f32>,

    /// A flag denoting whether or not the left
    /// mouse button is currently pressed
    pub ldown: bool,

    /// A flag denoting whether or not the right
    /// mouse button is currently pressed
    pub rdown: bool,

    /// A flag denoting whether or not the middle
    /// mouse button is currently pressed
    pub mdown: bool,

    /// The scroll status of the mouse
    pub scroll: f32,
}

impl MouseInfo {
    pub fn new() -> MouseInfo {
        MouseInfo {
            curr: Vector2::zero(),
            last: Vector2::zero(),
            clicked: Vector2::zero(),
            ldown: false,
            rdown: false,
            mdown: false,
            scroll: 1.0,
        }
    }

    pub fn velocity(&self) -> Vector2<f32> {
        self.curr - self.last
    }
}
pub enum InteractionState {
    Deselected,
    Selected,
    Hover,
    ConnectSource,
    ConnectDestination,
    // TODO: change these to `DragFrom` and `DragTo` or `Drag` and `Drop`
}

/// A trait that represents a rectangular region of the
/// display window that the user can interact with.
pub trait Panel {
    /// Returns the bounding rectangular defined by this panel.
    fn get_bounds(&self) -> &Rect;

    /// Returns the current interaction state of the panel.
    fn get_state(&self) -> InteractionState;

    /// Handles any mouse events.
    fn handle_interaction(&mut self, info: &MouseInfo);
}

pub struct Button {
    bounds: Rect,
    state: InteractionState,
}
