use cgmath::Vector2;

// Main window
pub const WINDOW_RESOLUTION: Vector2<f32> = Vector2 { x: 1400.0, y: 700.0 };
pub const WINDOW_MULTISAMPLES: u16 = 8;
pub const WINDOW_TITLE: &str = "signed-distance fields";

// Preview region
pub const PREVIEW_RESOLUTION: Vector2<f32> = Vector2{ x: 300.0, y: 300.0 };
pub const PREVIEW_ROTATION_SENSITIVITY: f32 = 0.25;
pub const PREVIEW_TRANSLATION_SENSITIVITY: f32 = 0.01;

// Interface controls
pub const ZOOM_INCREMENT: f32 = 0.05;

// Network
pub const NETWORK_BACKGROUND_COLOR: u32 = 0x2B2B2B;
pub const NETWORK_BACKGROUND_ALPHA: f32 = 1.0;

// Operators
pub const OPERATOR_SIZE: Vector2<f32> = Vector2 { x: 100.0, y: 50.0 };

// Parameters
pub const PARAMETER_CAPACITY: usize = 4;
pub const PARAMETER_SSBO_CAPACITY: usize = 256;



