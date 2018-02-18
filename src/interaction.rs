use cgmath::Vector2;

pub struct MouseInfo {
    /// The current position of the mouse
    pub curr: Vector2<f32>,

    /// The last position of the mouse
    pub last: Vector2<f32>,

    /// The last position that the user clicked
    pub clicked: Vector2<f32>,

    /// A flag denoting whether or not the mouse is
    /// currently pressed
    pub down: bool,
}

pub enum InteractionState {
    Deselected,
    Selected,
    Hover,
    ConnectSource,
    ConnectDestination,
    // TODO: change these to `DragFrom` and `DragTo` or `Drag` and `Drop`
}

/// A trait that represents a view or region that the
/// user can interact with.
trait Panel {
    fn mouse_pressed(&self);
    fn mouse_release(&self);
    fn mouse_entered(&self);
    fn mouse_exited(&self);
    fn handle_interaction(&mut self, info: &MouseInfo);
}
