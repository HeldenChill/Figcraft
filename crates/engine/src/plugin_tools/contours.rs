use super::quantize::LabelImage;
use figcraft_geom::BezPath;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ContourBudget {
    pub vertices_left: usize,
    pub loops_left: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
enum Dir {
    Right = 0,
    Down = 1,
    Left = 2,
    Up = 3,
}

impl Dir {
    fn from_u8(v: u8) -> Self {
        match v % 4 {
            0 => Dir::Right,
            1 => Dir::Down,
            2 => Dir::Left,
            _ => Dir::Up,
        }
    }

    fn right_turn(self) -> Self {
        Self::from_u8(self as u8 + 1)
    }

    fn straight(self) -> Self {
        self
    }

    fn left_turn(self) -> Self {
        Self::from_u8(self as u8 + 3)
    }

    fn reverse(self) -> Self {
        Self::from_u8(self as u8 + 2)
    }
}

fn bad(msg: impl Into<String>) -> crate::EngineError {
    crate::bad("image.trace", msg)
}

type OutgoingMap = BTreeMap<(u32, u32), Vec<(Dir, (u32, u32))>>;

pub(crate) fn trace_label(image: &LabelImage, label: u8, budget: &mut ContourBudget) -> crate::Result<BezPath> {
    let w = image.width;
    let h = image.height;
    if w == 0 || h == 0 {
        return Ok(BezPath::new());
    }

    let total_pixels = (w as usize).checked_mul(h as usize).ok_or_else(|| bad("image dimensions overflow"))?;

    if image.labels.len() != total_pixels {
        return Err(bad("labels length does not match image dimensions"));
    }

    let max_edges = 4 * 512 * 512;

    // Outgoing edges indexed by start vertex (vx, vy): (Dir, (next_vx, next_vy))
    // Multi-map since a saddle point can have at most 2 outgoing edges from a vertex.
    let mut outgoing: OutgoingMap = BTreeMap::new();
    let mut edge_count: usize = 0;

    let get_label = |x: i64, y: i64| -> Option<u8> {
        if x < 0 || y < 0 || x >= w as i64 || y >= h as i64 {
            None
        } else {
            let idx = (y as usize) * (w as usize) + (x as usize);
            image.labels.get(idx).copied().flatten()
        }
    };

    for y in 0..h {
        for x in 0..w {
            let idx = (y as usize) * (w as usize) + (x as usize);
            if image.labels.get(idx).copied().flatten() != Some(label) {
                continue;
            }

            let xi = x as i64;
            let yi = y as i64;

            // 1. Top boundary: (x, y - 1) != label
            if get_label(xi, yi - 1) != Some(label) {
                edge_count += 1;
                if edge_count > max_edges {
                    return Err(bad("edge storage ceiling exceeded"));
                }
                outgoing.entry((x, y)).or_default().push((Dir::Right, (x + 1, y)));
            }

            // 2. Right boundary: (x + 1, y) != label
            if get_label(xi + 1, yi) != Some(label) {
                edge_count += 1;
                if edge_count > max_edges {
                    return Err(bad("edge storage ceiling exceeded"));
                }
                outgoing.entry((x + 1, y)).or_default().push((Dir::Down, (x + 1, y + 1)));
            }

            // 3. Bottom boundary: (x, y + 1) != label
            if get_label(xi, yi + 1) != Some(label) {
                edge_count += 1;
                if edge_count > max_edges {
                    return Err(bad("edge storage ceiling exceeded"));
                }
                outgoing.entry((x + 1, y + 1)).or_default().push((Dir::Left, (x, y + 1)));
            }

            // 4. Left boundary: (x - 1, y) != label
            if get_label(xi - 1, yi) != Some(label) {
                edge_count += 1;
                if edge_count > max_edges {
                    return Err(bad("edge storage ceiling exceeded"));
                }
                outgoing.entry((x, y + 1)).or_default().push((Dir::Up, (x, y)));
            }
        }
    }

    if edge_count == 0 {
        return Ok(BezPath::new());
    }

    // Stable sort outgoing edges at each vertex by Dir
    for edges in outgoing.values_mut() {
        edges.sort_by_key(|(d, _)| *d);
    }

    let mut path = BezPath::new();

    // Walk unused edges in stable row-major / start-direction order.
    while let Some(&start_vertex) = outgoing.keys().next() {
        let (first_dir, next_vertex) = {
            let edges = match outgoing.get_mut(&start_vertex) {
                Some(e) if !e.is_empty() => e,
                _ => {
                    outgoing.remove(&start_vertex);
                    continue;
                }
            };
            edges.remove(0)
        };
        if outgoing.get(&start_vertex).is_some_and(Vec::is_empty) {
            outgoing.remove(&start_vertex);
        }

        budget.loops_left = budget.loops_left.checked_sub(1).ok_or_else(|| bad("contour loop budget exceeded"))?;

        let mut loop_verts = vec![start_vertex, next_vertex];
        let mut loop_dirs = vec![first_dir];
        let mut curr_vertex = next_vertex;
        let mut curr_dir = first_dir;

        while curr_vertex != start_vertex {
            let edges = match outgoing.get_mut(&curr_vertex) {
                Some(e) if !e.is_empty() => e,
                _ => {
                    return Err(bad("open contour boundary walk cannot be closed"));
                }
            };

            // Saddle choice preference: Right turn, Straight, Left turn, Reverse
            let pref = [curr_dir.right_turn(), curr_dir.straight(), curr_dir.left_turn(), curr_dir.reverse()];

            let mut chosen_edge_idx = None;
            for p_dir in pref {
                if let Some(pos) = edges.iter().position(|(d, _)| *d == p_dir) {
                    chosen_edge_idx = Some(pos);
                    break;
                }
            }

            let edge_idx = chosen_edge_idx.ok_or_else(|| bad("unreachable edge in contour walk"))?;

            let (next_dir, next_v) = edges.remove(edge_idx);
            if edges.is_empty() {
                outgoing.remove(&curr_vertex);
            }

            curr_vertex = next_v;
            curr_dir = next_dir;
            loop_verts.push(curr_vertex);
            loop_dirs.push(curr_dir);

            if loop_verts.len() > edge_count + 2 {
                return Err(bad("infinite loop detected in contour walk"));
            }
        }

        // Simplify collinear vertices along the closed loop.
        // loop_verts has N+1 vertices where loop_verts[0] == loop_verts[N], and loop_dirs has N edges.
        let n = loop_dirs.len();
        if n < 3 {
            continue;
        }

        let mut simplified_verts: Vec<(u32, u32)> = Vec::new();
        for i in 0..n {
            let prev_dir =
                if i == 0 { loop_dirs.get(n - 1).copied().unwrap_or(Dir::Right) } else { loop_dirs.get(i - 1).copied().unwrap_or(Dir::Right) };
            let curr_dir = loop_dirs.get(i).copied().unwrap_or(Dir::Right);

            // If direction changes, keep vertex i
            if prev_dir != curr_dir {
                simplified_verts.push(loop_verts[i]);
            }
        }

        if simplified_verts.len() < 3 {
            continue;
        }

        for (i, v) in simplified_verts.iter().enumerate() {
            budget.vertices_left = budget.vertices_left.checked_sub(1).ok_or_else(|| bad("contour vertex budget exceeded"))?;

            if i == 0 {
                path.move_to((f64::from(v.0), f64::from(v.1)));
            } else {
                path.line_to((f64::from(v.0), f64::from(v.1)));
            }
        }
        path.close_path();
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_rectangle_is_four_vertices() {
        // Solid 3x2 image -> 4 corners
        let labels = vec![Some(0); 6];
        let img = LabelImage { width: 3, height: 2, labels, palette: vec![[0, 0, 0, 255]] };
        let mut budget = ContourBudget { vertices_left: 100, loops_left: 10 };
        let path = trace_label(&img, 0, &mut budget).unwrap();
        let elements = path.elements();
        // MoveTo, LineTo, LineTo, LineTo, ClosePath (4 vertices)
        assert_eq!(elements.len(), 5);
        assert_eq!(budget.vertices_left, 96);
        assert_eq!(budget.loops_left, 9);
    }

    #[test]
    fn contour_donut_keeps_hole() {
        // 3x3 filled cells with empty center
        let mut labels = vec![Some(0); 9];
        labels[4] = None; // center empty
        let img = LabelImage { width: 3, height: 3, labels, palette: vec![[0, 0, 0, 255]] };
        let mut budget = ContourBudget { vertices_left: 100, loops_left: 10 };
        let path = trace_label(&img, 0, &mut budget).unwrap();
        // 2 closed loops (outer 4 corners + inner hole 4 corners)
        assert_eq!(budget.loops_left, 8); // 2 loops consumed
        assert_eq!(budget.vertices_left, 92); // 8 vertices consumed
        let elements = path.elements();
        assert_eq!(elements.len(), 10); // 2 * (1 MoveTo + 3 LineTo + 1 ClosePath)
    }

    #[test]
    fn contour_diagonal_islands_stay_separate() {
        // 2x2 diagonal foreground: (0,0) and (1,1)
        let labels = vec![Some(0), None, None, Some(0)];
        let img = LabelImage { width: 2, height: 2, labels, palette: vec![[0, 0, 0, 255]] };
        let mut budget = ContourBudget { vertices_left: 100, loops_left: 10 };
        let path = trace_label(&img, 0, &mut budget).unwrap();
        // 2 distinct loops, 4 vertices each
        assert_eq!(budget.loops_left, 8);
        assert_eq!(budget.vertices_left, 92);
        assert_eq!(path.elements().len(), 10);
    }

    #[test]
    fn contour_two_colors_partition() {
        // 2x1: left cell 0, right cell 1
        let labels = vec![Some(0), Some(1)];
        let img = LabelImage { width: 2, height: 1, labels, palette: vec![[255, 0, 0, 255], [0, 0, 255, 255]] };
        let mut b0 = ContourBudget { vertices_left: 10, loops_left: 5 };
        let path0 = trace_label(&img, 0, &mut b0).unwrap();
        let mut b1 = ContourBudget { vertices_left: 10, loops_left: 5 };
        let path1 = trace_label(&img, 1, &mut b1).unwrap();

        assert_eq!(path0.elements().len(), 5);
        assert_eq!(path1.elements().len(), 5);
    }

    #[test]
    fn contour_rejects_budget_exhaustion() {
        let labels = vec![Some(0); 4];
        let img = LabelImage { width: 2, height: 2, labels, palette: vec![[0, 0, 0, 255]] };
        let mut b_no_loops = ContourBudget { vertices_left: 10, loops_left: 0 };
        assert!(trace_label(&img, 0, &mut b_no_loops).is_err());

        let mut b_no_verts = ContourBudget { vertices_left: 2, loops_left: 1 };
        assert!(trace_label(&img, 0, &mut b_no_verts).is_err());
    }
}
