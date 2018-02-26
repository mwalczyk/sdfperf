use cgmath::{self, Vector2, Vector3, Vector4, Zero};
use uuid::Uuid;

use color::Color;
use graph::{Connected, Graph};
use interaction::{InteractionState, MouseInfo};
use operator::{Op, OpType};
use preview::Preview;
use renderer::Renderer;
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

type Connection = usize;

pub struct Network {
    /// An adjacency list representation of ops
    pub graph: Graph<Op, Connection>,

    /// The preview of the shader that is represented by the
    /// current network
    pub preview: Preview,

    /// The index of the currently selected op (if there is one)
    pub selection: Option<usize>,

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

    icons: HashMap<String, Texture>,
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
    pub fn new() -> Network {
        let mut network = Network {
            graph: Graph::new(),
            preview: Preview::new(),
            selection: None,
            root: None,
            dirty: false,
            show_preview: true,
            snapping: true,
            icons: HashMap::new(),
        };

        // Load all assets.
        for entry in fs::read_dir("assets").unwrap() {
            let path = entry.unwrap().path();
            let file = path.file_stem().unwrap();
            let ext = path.extension();

            if ext == Some(OsStr::new("png")) {
                network
                    .icons
                    .insert(file.to_str().unwrap().to_string(), Texture::new(&path));
            }
        }

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

    pub fn gather_transforms(&self) {
        let mut transforms = Vec::new();
        for node in self.graph.nodes.iter() {
            transforms.push(node.data.transform);
        }

        self.preview.update_transforms(transforms);
    }

    pub fn scale_selected(&mut self, val: f32) {
        if let Some(selected) = self.selection {
            let node = self.graph.nodes.get_mut(selected).unwrap();
            node.data.transform.w += val;
        }
    }

    pub fn translate_selected(&mut self, val: &Vector3<f32>) {
        if let Some(selected) = self.selection {
            let node = self.graph.nodes.get_mut(selected).unwrap();
            node.data.transform.x += val.x;
            node.data.transform.y += val.y;
            node.data.transform.z += val.z;
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

    /// Draws a single op in the network.
    fn draw_op(&self, op: &Op, renderer: &mut Renderer) {
        // Draw the op and other components:
        // - If the op is selected, draw a selection box behind it
        // - If the op is being used as a connection source or
        //   destination, draw the appropriate connection slot
        let slot_color = Color::from_hex(0x373737, 1.0);

        match op.state {
            InteractionState::Selected => {
                let aabb_select = op.aabb_op.expand_from_center(&Vector2::new(6.0, 6.0));
                renderer.draw_rect(&aabb_select, &Color::from_hex(0x76B264, 1.0), None);
            }
            InteractionState::ConnectSource => {
                renderer.draw_rect(&op.aabb_slot_output, &slot_color, None)
            }
            InteractionState::ConnectDestination => {
                renderer.draw_rect(&op.aabb_slot_input, &slot_color, None)
            }
            _ => (),
        }

        // Draw the body of the op.
        renderer.draw_rect(&op.aabb_op, &self.color_for_op(op), None);

        // Draw the icon (if one exists).
        if let Some(tex) = self.icons.get(op.family.to_string()) {
            renderer.draw_rect(&op.aabb_icon, &self.color_for_op(op), Some(tex));
        }
    }

    /// Draws all ops in the network.
    fn draw_all_ops(&self, renderer: &mut Renderer) {
        for node in self.graph.get_nodes().iter() {
            self.draw_op(&node.data, renderer);
        }
    }

    /// Draws all edges between ops in the network.
    fn draw_all_edges(&self, renderer: &mut Renderer) {
        let mut points = Vec::new();

        for (src, edges) in self.graph.edges.iter().enumerate() {
            for dst in edges.outputs.iter() {
                let src_node = self.graph.get_node(src).unwrap();
                let dst_node = self.graph.get_node(*dst).unwrap();

                // How many inputs does the destination node
                // currently have?
                //let dst_inputs_count = self.graph.edges[*dst].inputs.len();
                let src_centroid = src_node.data.aabb_slot_output.centroid();
                let dst_centroid = dst_node.data.aabb_slot_input.centroid();

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

        let draw_color = Color::white();

        renderer.draw_lines(&points, &draw_color, true);
    }

    /// Draws a grid in the network editor.
    pub fn draw_grid(&self, renderer: &mut Renderer) {
        let mut points_v = Vec::new();
        let mut points_h = Vec::new();

        let lines_x = renderer.get_resolution().x as u32 / 20;
        let lines_y = renderer.get_resolution().y as u32 / 20;
        let spacing_x = renderer.get_resolution().x / lines_x as f32;
        let spacing_y = renderer.get_resolution().y / lines_y as f32;
        let offset = renderer.get_resolution() * 0.5;

        // Draw vertical lines.
        for i in 0..lines_x {
            // Push back the first point.
            points_v.push(i as f32 * spacing_x - offset.x);
            points_v.push(-offset.y);
            points_v.push(0.0);
            points_v.push(0.0);

            // Push back the second point.
            points_v.push(i as f32 * spacing_x - offset.x);
            points_v.push(offset.y);
            points_v.push(1.0);
            points_v.push(1.0);
        }

        // Draw vertical lines.
        for i in 0..lines_y {
            // Push back the first point.
            points_h.push(-offset.x);
            points_h.push(i as f32 * spacing_y - offset.y);
            points_h.push(0.0);
            points_h.push(0.0);

            // Push back the second point.
            points_h.push(offset.x);
            points_h.push(i as f32 * spacing_y - offset.y);
            points_h.push(1.0);
            points_h.push(1.0);
        }

        let mut draw_color = Color::from_hex(0x373737, 0.25);
        renderer.draw_lines(&points_v, &draw_color, false);
        renderer.draw_lines(&points_h, &draw_color, false);
    }

    /// Draws all of the operators and edges that make
    /// up this graph.
    pub fn draw(&self, renderer: &mut Renderer) {
        self.draw_grid(renderer);
        self.draw_all_edges(renderer);
        self.draw_all_ops(renderer);

        if self.show_preview {
            self.gather_transforms();
            self.preview.draw(renderer);
        }
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
            if node.data.aabb_op.inside(&mouse.curr) {
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
                        .aabb_slot_output
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
                    .aabb_slot_input
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
}
