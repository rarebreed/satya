//! # Thin Edge Walls, Doors & Windows for the 2D Battlemap (`map_2d_art_walls`)
//!
//! Renders [`MapWallEdge`]s as architectural floorplan strokes:
//! - **Walls**: collinear runs are merged into long strokes with square caps for clean joints. Exterior
//!   masonry (brick/concrete) is drawn heavier than drywall partitions, with an ink outline around a
//!   solid poché core.
//! - **Doors**: jamb ticks plus either a closed leaf lying in the opening, or an open leaf swung 90° into
//!   the owning cell with a dashed swing arc.
//! - **Windows**: a double-line frame with a pale glass core.

use crate::wasm_client::map_2d_art_props::BATTLEMAP_INK;
use satya_schema::{EdgeSide, MapWallEdge, MaterialClass, WallEdgeKind};
use std::collections::HashMap;
use std::fmt::Write as _;

const fn is_heavy(m: MaterialClass) -> bool {
    matches!(
        m,
        MaterialClass::Brick
            | MaterialClass::ConcreteBlock
            | MaterialClass::ReinforcedConcrete
            | MaterialClass::ArmorSteel
    )
}

/// Appends edge walls/doors/windows for a `w × h` board at `cell` SVG units per cell.
pub fn push_edge_walls_svg(out: &mut String, edges: &[MapWallEdge], w: i32, h: i32, wall_fill: &str, door_fill: &str, cell: f32) {
    if edges.is_empty() {
        return;
    }
    let by_addr: HashMap<(i32, i32, bool), &MapWallEdge> =
        edges.iter().map(|e| ((e.x, e.y, e.side == EdgeSide::North), e)).collect();
    let wall_class = |x: i32, y: i32, north: bool| -> Option<bool> {
        by_addr
            .get(&(x, y, north))
            .filter(|e| e.kind == WallEdgeKind::Wall)
            .map(|e| is_heavy(e.material))
    };

    // Merged wall runs per weight class
    for heavy in [true, false] {
        let mut d = String::new();
        for y in 0..=h {
            let mut run: Option<i32> = None;
            for x in 0..=w {
                let on = x < w && wall_class(x, y, true) == Some(heavy);
                match (on, run) {
                    (true, None) => run = Some(x),
                    (false, Some(x0)) => {
                        let _ = write!(d, "M{:.0} {:.0}H{:.0}", x0 as f32 * cell, y as f32 * cell, x as f32 * cell);
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        for x in 0..=w {
            let mut run: Option<i32> = None;
            for y in 0..=h {
                let on = y < h && wall_class(x, y, false) == Some(heavy);
                match (on, run) {
                    (true, None) => run = Some(y),
                    (false, Some(y0)) => {
                        let _ = write!(d, "M{:.0} {:.0}V{:.0}", x as f32 * cell, y0 as f32 * cell, y as f32 * cell);
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        if d.is_empty() {
            continue;
        }
        let (outer, inner) = if heavy { (15.0, 10.0) } else { (10.5, 6.0) };
        let _ = write!(
            out,
            r#"<g filter="url(#bm-ink)" stroke-linecap="square" fill="none"><path d="{d}" stroke="{BATTLEMAP_INK}" stroke-width="{outer}"/><path d="{d}" stroke="{wall_fill}" stroke-width="{inner}"/></g>"#
        );
    }

    // Doors and windows
    out.push_str(r#"<g filter="url(#bm-prop)">"#);
    for e in edges {
        let (ax, ay) = (e.x as f32 * cell, e.y as f32 * cell);
        let north = e.side == EdgeSide::North;
        let (bx, by) = if north { (ax + cell, ay) } else { (ax, ay + cell) };
        match e.kind {
            WallEdgeKind::Wall => {}
            WallEdgeKind::Window => {
                let _ = write!(
                    out,
                    r##"<path d="M{ax:.0} {ay:.0}L{bx:.0} {by:.0}" stroke="{BATTLEMAP_INK}" stroke-width="10"/><path d="M{ax:.0} {ay:.0}L{bx:.0} {by:.0}" stroke="#d6ebf5" stroke-width="6"/><path d="M{ax:.0} {ay:.0}L{bx:.0} {by:.0}" stroke="{BATTLEMAP_INK}" stroke-width="1"/>"##
                );
            }
            WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen => {
                let leaf = cell - 6.0;
                // Jamb ticks perpendicular to the opening at both ends.
                let (tx, ty) = if north { (0.0, 6.0) } else { (6.0, 0.0) };
                let _ = write!(
                    out,
                    r#"<path d="M{:.1} {:.1}L{:.1} {:.1}M{:.1} {:.1}L{:.1} {:.1}" stroke="{BATTLEMAP_INK}" stroke-width="3"/>"#,
                    ax - tx, ay - ty, ax + tx, ay + ty, bx - tx, by - ty, bx + tx, by + ty
                );
                if e.kind == WallEdgeKind::DoorClosed {
                    let (rx, ry, rw, rh) = if north { (ax + 3.0, ay - 4.0, leaf, 8.0) } else { (ax - 4.0, ay + 3.0, 8.0, leaf) };
                    let _ = write!(
                        out,
                        r#"<rect x="{rx:.1}" y="{ry:.1}" width="{rw:.1}" height="{rh:.1}" fill="{door_fill}" stroke="{BATTLEMAP_INK}" stroke-width="1.6"/>"#
                    );
                } else {
                    // Hinge at the edge start; the leaf swings into the owning cell (+y for North, +x for West).
                    let (lx, ly, lw, lh, arc) = if north {
                        (ax - 2.5, ay, 5.0, leaf, format!("M{:.1} {ay:.1}A{leaf} {leaf} 0 0 1 {ax:.1} {:.1}", ax + leaf, ay + leaf))
                    } else {
                        (ax, ay - 2.5, leaf, 5.0, format!("M{ax:.1} {:.1}A{leaf} {leaf} 0 0 0 {:.1} {ay:.1}", ay + leaf, ax + leaf))
                    };
                    let _ = write!(
                        out,
                        r#"<path d="{arc}" fill="none" stroke="{BATTLEMAP_INK}" stroke-width="1.3" stroke-dasharray="4 3" stroke-opacity=".7"/><rect x="{lx:.1}" y="{ly:.1}" width="{lw:.1}" height="{lh:.1}" fill="{door_fill}" stroke="{BATTLEMAP_INK}" stroke-width="1.6"/>"#
                    );
                }
            }
        }
    }
    out.push_str("</g>");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_runs_merge_and_doors_render() {
        let mk = |x, y, side, kind, material| MapWallEdge { x, y, side, kind, material };
        let edges = vec![
            mk(0, 0, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Brick),
            mk(1, 0, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Brick),
            mk(2, 0, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Brick),
            mk(3, 1, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::Drywall),
            mk(3, 2, EdgeSide::West, WallEdgeKind::Window, MaterialClass::Glass),
        ];
        let mut s = String::new();
        push_edge_walls_svg(&mut s, &edges, 6, 4, "#3b3835", "#9a6b43", 64.0);
        assert!(s.contains("M0 0H192"), "three brick edges merge into one run: {s}");
        assert!(s.contains("stroke-dasharray=\"4 3\""), "open door swing arc");
        assert!(s.contains("#d6ebf5"), "window glass");
        assert_eq!(s.matches("<g").count(), s.matches("</g>").count());
    }
}
