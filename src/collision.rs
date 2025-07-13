use glam::Vec3;
use log::info;

use crate::geometry::Transform;

#[derive(Debug, Clone)]
pub struct Collision {
    // Vertices of space A that are inside space B
    pub my_vertices_inside: VertexCollision,

    // Vertices of space B that are inside space A
    pub other_vertices_inside: VertexCollision,

    // Edges of space A that intersect with edges of space B
    pub my_edge_intersections: EdgeCollision,

    // Whether space B is completely contained within space A
    pub other_space_inside_me: bool,

    // Whether space A is completely contained within space B
    pub i_am_inside_other: bool,

    // Intersection points for physics calculations
    pub intersection_points: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub struct VertexCollision {
    pub top_left: bool,
    pub bottom_left: bool,
    pub bottom_right: bool,
    pub top_right: bool,
}

#[derive(Debug, Clone)]
pub struct EdgeCollision {
    pub top_edge: Vec<Vec3>,    // Intersection points on top edge
    pub left_edge: Vec<Vec3>,   // Intersection points on left edge
    pub bottom_edge: Vec<Vec3>, // Intersection points on bottom edge
    pub right_edge: Vec<Vec3>,  // Intersection points on right edge
}

impl VertexCollision {
    pub fn new() -> Self {
        Self {
            top_left: false,
            bottom_left: false,
            bottom_right: false,
            top_right: false,
        }
    }

    pub fn any(&self) -> bool {
        self.top_left || self.bottom_left || self.bottom_right || self.top_right
    }

    pub fn count(&self) -> usize {
        [
            self.top_left,
            self.bottom_left,
            self.bottom_right,
            self.top_right,
        ]
        .iter()
        .filter(|&&b| b)
        .count()
    }
}

impl EdgeCollision {
    pub fn new() -> Self {
        Self {
            top_edge: Vec::new(),
            left_edge: Vec::new(),
            bottom_edge: Vec::new(),
            right_edge: Vec::new(),
        }
    }

    pub fn any(&self) -> bool {
        !self.top_edge.is_empty()
            || !self.left_edge.is_empty()
            || !self.bottom_edge.is_empty()
            || !self.right_edge.is_empty()
    }

    pub fn total_intersections(&self) -> usize {
        self.top_edge.len() + self.left_edge.len() + self.bottom_edge.len() + self.right_edge.len()
    }
}

impl Collision {
    pub fn new() -> Self {
        Self {
            my_vertices_inside: VertexCollision::new(),
            other_vertices_inside: VertexCollision::new(),
            my_edge_intersections: EdgeCollision::new(),
            other_space_inside_me: false,
            i_am_inside_other: false,
            intersection_points: Vec::new(),
        }
    }

    pub fn has_collision(&self) -> bool {
        self.my_vertices_inside.any()
            || self.other_vertices_inside.any()
            || self.my_edge_intersections.any()
            || self.other_space_inside_me
            || self.i_am_inside_other
    }

    pub fn do_spaces_collide(a: &Transform, b: &Transform) -> Option<Collision> {
        let mut collision_info = Collision::new();

        // Check vertices of A inside B
        collision_info.my_vertices_inside = Self::check_vertices_in_space(a, b);

        // Check vertices of B inside A
        collision_info.other_vertices_inside = Self::check_vertices_in_space(b, a);

        // Check edge intersections
        collision_info.my_edge_intersections = Self::check_edge_intersections(a, b);

        // Check complete containment
        collision_info.other_space_inside_me = Self::is_space_inside_other(b, a);
        collision_info.i_am_inside_other = Self::is_space_inside_other(a, b);

        // Collect all intersection points
        collision_info.intersection_points =
            Self::collect_intersection_points(&collision_info.my_edge_intersections);

        if collision_info.has_collision() {
            Some(collision_info)
        } else {
            None
        }
    }

    fn check_vertices_in_space(from: &Transform, to: &Transform) -> VertexCollision {
        let transform = from.map_towards(to);

        let corners = [
            Vec3::new(0.0, 0.0, 0.0), // top_left
            Vec3::new(0.0, 1.0, 0.0), // bottom_left
            Vec3::new(1.0, 1.0, 0.0), // bottom_right
            Vec3::new(1.0, 0.0, 0.0), // top_right
        ];

        let projected: Vec<Vec3> = corners
            .iter()
            .map(|&corner| transform.project(corner))
            .collect();

        let in_bounds: Vec<bool> = projected
            .iter()
            .map(|corner| corner.x >= 0.0 && corner.x <= 1.0 && corner.y >= 0.0 && corner.y <= 1.0)
            .collect();

        VertexCollision {
            top_left: in_bounds[0],
            bottom_left: in_bounds[1],
            bottom_right: in_bounds[2],
            top_right: in_bounds[3],
        }
    }

    fn check_edge_intersections(a: &Transform, b: &Transform) -> EdgeCollision {
        let mut edge_collision = EdgeCollision::new();

        // Get world coordinates of both spaces
        let a_corners = Self::get_world_corners(a);
        let b_corners = Self::get_world_corners(b);

        // Define edges of space A
        let a_edges = [
            (a_corners[0], a_corners[3]), // top edge (top_left to top_right)
            (a_corners[0], a_corners[1]), // left edge (top_left to bottom_left)
            (a_corners[1], a_corners[2]), // bottom edge (bottom_left to bottom_right)
            (a_corners[3], a_corners[2]), // right edge (top_right to bottom_right)
        ];

        // Define edges of space B
        let b_edges = [
            (b_corners[0], b_corners[3]), // top edge
            (b_corners[0], b_corners[1]), // left edge
            (b_corners[1], b_corners[2]), // bottom edge
            (b_corners[3], b_corners[2]), // right edge
        ];

        // Check intersections between A's edges and B's edges
        for (a_edge_idx, a_edge) in a_edges.iter().enumerate() {
            for b_edge in b_edges.iter() {
                if let Some(intersection) = Self::line_segments_intersect(*a_edge, *b_edge) {
                    match a_edge_idx {
                        0 => edge_collision.top_edge.push(intersection),
                        1 => edge_collision.left_edge.push(intersection),
                        2 => edge_collision.bottom_edge.push(intersection),
                        3 => edge_collision.right_edge.push(intersection),
                        _ => unreachable!(),
                    }
                }
            }
        }

        edge_collision
    }

    fn get_world_corners(transform: &Transform) -> [Vec3; 4] {
        let corners = [
            Vec3::new(0.0, 0.0, 0.0), // top_left
            Vec3::new(0.0, 1.0, 0.0), // bottom_left
            Vec3::new(1.0, 1.0, 0.0), // bottom_right
            Vec3::new(1.0, 0.0, 0.0), // top_right
        ];

        corners.map(|corner| transform.project(corner))
    }

    fn line_segments_intersect(line1: (Vec3, Vec3), line2: (Vec3, Vec3)) -> Option<Vec3> {
        let (p1, p2) = line1;
        let (p3, p4) = line2;

        let denom = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);
        if denom.abs() < f32::EPSILON {
            return None; // Lines are parallel
        }

        let t = ((p1.x - p3.x) * (p3.y - p4.y) - (p1.y - p3.y) * (p3.x - p4.x)) / denom;
        let u = -((p1.x - p2.x) * (p1.y - p3.y) - (p1.y - p2.y) * (p1.x - p3.x)) / denom;

        if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
            // Calculate intersection point
            let intersection_x = p1.x + t * (p2.x - p1.x);
            let intersection_y = p1.y + t * (p2.y - p1.y);
            Some(Vec3::new(intersection_x, intersection_y, 0.0))
        } else {
            None
        }
    }

    fn is_space_inside_other(inner: &Transform, outer: &Transform) -> bool {
        let vertices = Self::check_vertices_in_space(inner, outer);
        vertices.top_left && vertices.bottom_left && vertices.bottom_right && vertices.top_right
    }

    fn collect_intersection_points(edge_collision: &EdgeCollision) -> Vec<Vec3> {
        let mut points = Vec::new();
        points.extend_from_slice(&edge_collision.top_edge);
        points.extend_from_slice(&edge_collision.left_edge);
        points.extend_from_slice(&edge_collision.bottom_edge);
        points.extend_from_slice(&edge_collision.right_edge);
        points
    }
}

// Usage example:
/*
if let Some(collision) = CollisionInfo::do_spaces_collide(&player_transform, &wall_transform) {
    // Handle collision based on detailed information

    if collision.my_vertices_inside.any() {
        println!("Player vertices inside wall: {:?}", collision.my_vertices_inside);
    }

    if collision.other_vertices_inside.any() {
        println!("Wall vertices inside player: {:?}", collision.other_vertices_inside);
    }

    if collision.my_edge_intersections.any() {
        println!("Edge intersections found: {} total", collision.my_edge_intersections.total_intersections());
        println!("Intersection points: {:?}", collision.intersection_points);
    }

    if collision.other_space_inside_me {
        println!("Wall is completely inside player");
    }

    if collision.i_am_inside_other {
        println!("Player is completely inside wall");
    }
}
*/
