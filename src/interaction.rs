use cgmath::Vector2;

pub struct MouseInfo {
    pub curr: Vector2<f32>,
    pub last: Vector2<f32>,
    pub clicked: Vector2<f32>,
    pub down: bool
}

pub enum InteractionState {
    Deselected,
    Selected,
    Hover,
    ConnectSource,
    ConnectDestination
}

trait InterfaceElement {
    //fn get_bounding_rect() -> BoundingRect;
}