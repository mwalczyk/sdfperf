use cgmath::{self, Vector2, Vector3, Vector4, Zero};
use uuid::Uuid;

use bounds::Rect;
use color::Color;
use graph::{Connected, Graph};
use interaction::{InteractionState, MouseInfo, Panel};
use operator::{Connectivity, Op, OpType};
use preview::Preview;
use renderer::{DrawParams, LineMode, Renderer};
use texture::Texture;

use std::cmp::max;
use std::collections::HashMap;
use std::io;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::ffi::OsStr;

/// Palette:
///
/// Background:  0x2B2B2B (dark gray)
/// Accent:      0x373737 (light gray)
/// Generator:   0x8F719D (purple)
/// Combiner:    0xA8B6C5 (blue)
/// Render:      0xC77832 (orange)
/// Selection:   0x76B264 (green)
/// Error:       0xA0502B (dark orange)
/// Other:       0xFEC56D (yellow)
///

pub struct Grid {
    size: Vector2<f32>,
    spacing: Vector2<usize>,
    pub points_vertical: Vec<f32>,
    pub points_horizontal: Vec<f32>
}

impl Grid {
    pub fn new(size: Vector2<f32>, spacing: Vector2<usize>) -> Grid {
        let mut points_vertical = Vec::new();
        let mut points_horizontal = Vec::new();

        let lines_x = size.x as usize / spacing.x;
        let lines_y = size.y as usize / spacing.y;
        let spacing_x = size.x / lines_x as f32;
        let spacing_y = size.y / lines_y as f32;

        let offset = size * 0.5;

        // Draw vertical lines.
        for i in 0..lines_x {
            // Push back the first point.
            points_vertical.push(i as f32 * spacing_x - offset.x);
            points_vertical.push(-offset.y);
            points_vertical.push(0.0);
            points_vertical.push(0.0);

            // Push back the second point.
            points_vertical.push(i as f32 * spacing_x - offset.x);
            points_vertical.push(offset.y);
            points_vertical.push(1.0);
            points_vertical.push(1.0);
        }

        // Draw vertical lines.
        for i in 0..lines_y {
            // Push back the first point.
            points_horizontal.push(-offset.x);
            points_horizontal.push(i as f32 * spacing_y - offset.y);
            points_horizontal.push(0.0);
            points_horizontal.push(0.0);

            // Push back the second point.
            points_horizontal.push(offset.x);
            points_horizontal.push(i as f32 * spacing_y - offset.y);
            points_horizontal.push(1.0);
            points_horizontal.push(1.0);
        }

        Grid {
            size,
            spacing,
            points_vertical,
            points_horizontal
        }
    }
}

type Connection = usize;

pub struct Network {
    /// An adjacency list representation of ops
    pub graph: Graph<Op, Connection>,

    /// The sprite renderer that will be used to draw all nodes and
    /// edges of the graph
    renderer: Renderer,

    /// The preview of the shader that is represented by the
    /// current network
    pub preview: Preview,

    pub grid: Grid,

    /// The index of the currently selected op (if there is one)
    selection: Option<usize>,

    /// The index of the root op (if there is one)
    pub root: Option<usize>,

    /// A flag that controls whether or not the shader graph
    /// needs to be rebuilt
    dirty: bool,

    /// A flag that controls whether or not the preview will
    /// be drawn
    show_preview: bool,

    /// A flag that controls whether or not ops will be snapped
    /// to a grid when dragged
    snapping: bool,

    /// A map of asset names to textures, used to render various
    /// UI elements
    assets: HashMap<String, Texture>,
}

enum Pair<T> {
    Both(T, T),
    One(T),
    None,
}

/// Get mutable references at index `a` and `b`.
fn index_twice<T>(slc: &mut [T], a: usize, b: usize) -> Pair<&mut T> {
    if max(a, b) >= slc.len() {
        Pair::None
    } else if a == b {
        Pair::One(&mut slc[max(a, b)])
    } else {
        unsafe {
            let ar = &mut *(slc.get_unchecked_mut(a) as *mut _);
            let br = &mut *(slc.get_unchecked_mut(b) as *mut _);
            Pair::Both(ar, br)
        }
    }
}

impl Network {
    /// Constructs a new, empty network.
    pub fn new(size: Vector2<f32>) -> Network {
        let mut network = Network {
            graph: Graph::new(),
            renderer: Renderer::new(size),
            preview: Preview::new(),
            grid: Grid::new(size, Vector2::new(20, 20)),
            selection: None,
            root: None,
            dirty: false,
            show_preview: true,
            snapping: true,
            assets: HashMap::new(),
        };
        network.load_assets();
        network
    }

    /// Returns `true` if the shader graph needs to be rebuilt and
    /// `false` otherwise.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Sets the `dirty` flag to `false`.
    pub fn clean(&mut self) {
        self.dirty = false;
    }

    /// Toggles drawing of the preview window.
    pub fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
    }

    /// Scales the distance field represented by the currently
    /// selected op (if one exists).
    pub fn scale_selected(&mut self, val: f32) {
        if let Some(selected) = self.selection {
            let node = self.graph.nodes.get_mut(selected).unwrap();
            node.data.transform.scale(val);
        }
    }

    /// Translates the distance field represented by the currently
    /// selected op (if one exists).
    pub fn translate_selected(&mut self, val: &Vector3<f32>) {
        if let Some(selected) = self.selection {
            let node = self.graph.nodes.get_mut(selected).unwrap();
            node.data.transform.translate(val);
        }
    }

    /// Deletes the currently selected op (if the selection is not empty).
    pub fn delete_selected(&mut self) {
        if let Some(selected) = self.selection {
            // Before removing this vertex from the graph,
            // check to see if it is connected to the root
            // (if one exists). If so, then the shader
            // graph needs to be rebuilt.
            if let Some(root) = self.root {
                for edge in self.graph.edges[selected].outputs.iter() {
                    if *edge == root {
                        self.dirty = true;
                        self.root = None;
                        break;
                    }
                }
            }
            self.graph.remove_node(selected);
            self.selection = None;
        }
    }

    /// Adds a new op of type `family` to the network at coordinates
    /// `position` and dimensions `size`.
    pub fn add_op(&mut self, family: OpType, position: Vector2<f32>, size: Vector2<f32>) {
        let op = Op::new(family, position, size);
        self.graph.add_node(op, 0);
    }

    /// Adds a new connection between two ops.
    pub fn add_connection(&mut self, a: usize, b: usize) {
        self.graph.add_edge(a, b);

        if let Pair::Both(node_a, node_b) = index_twice(&mut self.graph.nodes, a, b) {
            // If we previously connected to a render op, then we
            // know that the graph must be rebuilt.
            if let Some(_) = self.root {
                self.dirty = true;
                println!("Active render node in-line: re-building graph");
            }
            // If we are connecting to a render op, then the shader
            // must be rebuilt.
            if node_b.data.family == OpType::Render {
                self.root = Some(b);
                self.dirty = true;
                println!("Connected to render node: building graph");
            }

            // Deselect both ops.
            node_a.data.state = InteractionState::Deselected;
            node_b.data.state = InteractionState::Deselected;
        } else {
            println!("Attempting to connect two ops with the same index - something is wrong here")
        }
    }

    /// Handles all mouse events.
    pub fn handle_interaction(&mut self, mouse: &MouseInfo) {
        let mut connecting = false;
        let mut src: Option<usize> = None;
        let mut dst: Option<usize> = None;

        for (index, node) in self.graph.nodes.iter_mut().enumerate() {
            if let InteractionState::ConnectSource = node.data.state {
                if mouse.ldown {
                    // If this operator is currently being connected to another:
                    // 1) Set the `connecting` flag to `true`, as the user is
                    //    performing a potential op connection
                    // 2) Store its graph index as a potential connect source
                    // 3) Skip the rest of this loop iteration
                    connecting = true;
                    src = Some(index);
                    continue;
                } else {
                    // Otherwise, deselect this op
                    node.data.state = InteractionState::Deselected;
                }
            }

            // Is the mouse inside of this op's bounding box?
            if node.data.bounds_body.inside(&mouse.curr) {
                // Is there an op currently selected?
                if let Some(selected) = self.selection {
                    // Is this op the selected op?
                    if selected == index {
                        // Is the mouse down?
                        if mouse.ldown {
                            // TODO: let mut velocity = ..;
                            if self.snapping {
                                // TODO
                            }
                            node.data.translate(&mouse.velocity());
                        }
                        continue;
                    }
                }

                // This op is not the selected op, but we are inside of it's
                // bounding box. Is the mouse down?
                if mouse.ldown {
                    // Are we inside the bounds of this op's output slot?
                    if node.data
                        .bounds_output
                        .inside_with_padding(&mouse.curr, 12.0)
                    {
                        // This op is now a potential connection source.
                        node.data.state = InteractionState::ConnectSource;

                        // Store the connection source index.
                        src = Some(index);
                    } else {
                        // This op has been selected.
                        node.data.state = InteractionState::Selected;

                        // Store the selected UUID.
                        self.selection = Some(index);
                    }
                } else {
                    // Otherwise, the mouse is still inside the bounds of this op,
                    // so we must be hovering over it.
                    node.data.state = InteractionState::Hover;
                }

            // The mouse is not inside of this op's bounding box.
            } else {
                // Is there an op currently selected?
                if let Some(selected) = self.selection {
                    // Is this op the selected op?
                    if selected == index {
                        // Is the mouse down?
                        if mouse.ldown {
                            // The user has clicked somewhere else in the
                            // network, so reset the selection.
                            self.selection = None;
                        } else {
                            // Keep this op selected.
                            node.data.state = InteractionState::Selected;
                        }
                    } else {
                        // Deselect the op.
                        node.data.state = InteractionState::Deselected;
                    }
                } else {
                    // Deselect the op.
                    node.data.state = InteractionState::Deselected;
                }
            }
        }

        // If the mouse is dragging from the output slot of one operator,
        // check if a potential connection has happened (i.e. the mouse
        // is now over an input slot of a different operator).
        if connecting {
            for (index, node) in self.graph.nodes.iter_mut().enumerate() {
                // Is the mouse now inside of a different op's input slot region?
                if node.data
                    .bounds_input
                    .inside_with_padding(&mouse.curr, 12.0)
                {
                    node.data.state = InteractionState::ConnectDestination;
                    if let Some(src) = src {
                        dst = Some(index);
                    }
                }
            }
        }

        if let (Some(src), Some(dst)) = (src, dst) {
            self.add_connection(src, dst);
        }

        self.preview.handle_interaction(&mouse);
    }

    /// Draws all of the operators and edges that make
    /// up this graph.
    pub fn draw(&mut self) {
        self.draw_grid();
        self.draw_all_edges();
        self.draw_all_nodes();

        if self.show_preview {
            self.gather_transforms();

            self.preview.prepare(self.renderer.get_projection());
            self.renderer.draw_rect_inner();
        }
    }

    /// Pick a draw color based on the current interaction state of this
    /// operator and the op type.
    fn color_for_op(&self, op: &Op) -> Color {
        let mut color = match op.family {
            OpType::Sphere | OpType::Box | OpType::Plane => Color::from_hex(0x8F719D, 1.0),
            OpType::Union | OpType::Subtraction | OpType::Intersection | OpType::SmoothMinimum => {
                Color::from_hex(0xA8B6C5, 1.0)
            }
            OpType::Render => Color::from_hex(0xC77832, 1.0),
        };

        // Add a contribution based on the op's current interaction state.
        if let InteractionState::Hover = op.state {
            color += Color::mono(0.05, 0.0);
        }
        color
    }

    /// Draws all ops in the network.
    fn draw_all_nodes(&mut self) {
        for node in self.graph.get_nodes().iter() {
            let op = &node.data;
            //self.draw_op(&node.data);
            // Draw the op and other components:
            // - If the op is selected, draw a selection box behind it
            // - If the op is being used as a connection source or
            //   destination, draw the appropriate connection slot
            let slot_color = Color::from_hex(0x373737, 1.0);
            match op.state {
                InteractionState::Selected => {
                    let bounds_select = Rect::expanded_from(&op.bounds_body, &Vector2::new(6.0, 6.0));
                    self.renderer.draw(
                        DrawParams::Rectangle(&bounds_select),
                        &Color::from_hex(0x76B264, 1.0),
                        None,
                        None,
                    );
                }
                InteractionState::ConnectSource => self.renderer.draw(
                    DrawParams::Rectangle(&op.bounds_output),
                    &slot_color,
                    None,
                    None,
                ),
                InteractionState::ConnectDestination => self.renderer.draw(
                    DrawParams::Rectangle(&op.bounds_input),
                    &slot_color,
                    None,
                    None,
                ),
                _ => (),
            }

            // Draw the body of the op.
            let draw_color = self.color_for_op(op);
            let alpha_key = match op.family.get_connectivity() {
                Connectivity::InputOutput => "alpha_input_output".to_string(),
                Connectivity::Input => "alpha_input".to_string(),
                Connectivity::Output => "alpha_output".to_string(),
            };
            let alpha_map = self.assets.get(&alpha_key).unwrap();
            self.renderer.draw(
                DrawParams::Rectangle(&op.bounds_body),
                &draw_color,
                None,
                Some(alpha_map),
            );

            // Draw the icon on top of the op (if one exists).
            let color_map = self.assets.get(op.family.to_string()).unwrap();
            self.renderer.draw(
                DrawParams::Rectangle(&op.bounds_icon),
                &draw_color,
                Some(color_map),
                None,
            );
        }
    }

    /// Draws all edges between ops in the network.
    fn draw_all_edges(&mut self) {
        let mut points = Vec::new();

        for (src, edges) in self.graph.edges.iter().enumerate() {
            for dst in edges.outputs.iter() {
                let src_node = self.graph.get_node(src).unwrap();
                let dst_node = self.graph.get_node(*dst).unwrap();

                // How many inputs does the destination node
                // currently have?
                //let dst_inputs_count = self.graph.edges[*dst].inputs.len();
                let src_centroid = src_node.data.bounds_output.centroid();
                let dst_centroid = dst_node.data.bounds_input.centroid();

                // Push back the first point.
                points.push(src_centroid.x);
                points.push(src_centroid.y);
                points.push(0.0);
                points.push(0.0);

                // Push back the second point.
                points.push(dst_centroid.x);
                points.push(dst_centroid.y);
                points.push(1.0);
                points.push(1.0);
            }
        }

        self.renderer.draw(
            DrawParams::Line(&points, LineMode::Dashed),
            &Color::white(),
            None,
            None,
        );
    }

    /// Draws a grid in the network editor.
    fn draw_grid(&mut self) {
        let draw_color = Color::from_hex(0x373737, 0.25);
        self.renderer.draw(
            DrawParams::Line(&self.grid.points_vertical, LineMode::Solid),
            &draw_color,
            None,
            None,
        );
        self.renderer.draw(
            DrawParams::Line(&self.grid.points_horizontal, LineMode::Solid),
            &draw_color,
            None,
            None,
        );
    }

    fn gather_transforms(&self) {
        let mut transforms = Vec::new();
        for node in self.graph.nodes.iter() {
            transforms.push(node.data.transform.data);
        }

        self.preview.update_transforms(transforms);
    }

    /// Loads all texture assets.
    fn load_assets(&mut self) {
        for entry in fs::read_dir("assets").unwrap() {
            let path = entry.unwrap().path();
            let file = path.file_stem().unwrap();
            let ext = path.extension();

            if ext == Some(OsStr::new("png")) {
                self.assets
                    .insert(file.to_str().unwrap().to_string(), Texture::new(&path));
            }
        }
    }
}
