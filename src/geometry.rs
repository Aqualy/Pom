//! Tomato artwork, read from the SVG at compile time, and the draining wedge cut from it.
//!
//! The SVG holds two `<path>`s: the body first, then the crown (optionally `translate`d).
//! Only absolute `M`, `L`, `C` and `Z` commands are supported, which is what the art uses.

use std::f32::consts::TAU;
use std::fmt::Write;

const SVG: &str = include_str!("../tomato.svg");

/// Angular resolution of the wedge outline.
const SAMPLES: usize = 720;
/// Line segments per cubic curve when flattening.
const CURVE_STEPS: usize = 16;

type Point = (f32, f32);

pub struct Tomato {
    pub body: String,
    pub crown: String,
    /// Square viewbox around the art: (x, y, size).
    pub viewbox: (f32, f32, f32),
    center: Point,
    outline: Vec<Point>,
    /// Distance from `center` to the outline at each sample angle.
    radii: Vec<f32>,
}

impl Tomato {
    pub fn load() -> Self {
        let paths = svg_paths(SVG);
        assert!(paths.len() >= 2, "tomato SVG needs a body and a crown path");
        let (body, _) = &paths[0];
        let (crown, (dx, dy)) = &paths[1];

        let outline = flatten(body);
        let crown_outline = flatten(crown);
        let (min, max) = bounds(&outline);
        let center = ((min.0 + max.0) / 2.0, (min.1 + max.1) / 2.0);

        let (cmin, cmax) = bounds(&crown_outline);
        let (x0, y0) = (min.0.min(cmin.0 + dx), min.1.min(cmin.1 + dy));
        let (x1, y1) = (max.0.max(cmax.0 + dx), max.1.max(cmax.1 + dy));
        let size = (x1 - x0).max(y1 - y0) * 1.04;
        let viewbox = ((x0 + x1 - size) / 2.0, (y0 + y1 - size) / 2.0, size);

        let radii = (0..SAMPLES)
            .map(|i| ray_hit(center, &outline, i as f32 / SAMPLES as f32))
            .collect();

        Self {
            body: body.clone(),
            crown: translate(crown, *dx, *dy),
            viewbox,
            center,
            outline,
            radii,
        }
    }

    /// Point at distance `r` from the center, `turn` measured clockwise from 12 o'clock.
    fn point_at(&self, turn: f32, r: f32) -> Point {
        let a = turn * TAU;
        (self.center.0 + r * a.sin(), self.center.1 - r * a.cos())
    }

    /// The red part still left. The elapsed share is eaten clockwise from 12 o'clock,
    /// like a kitchen timer.
    pub fn wedge(&self, fraction_left: f32) -> String {
        let f = fraction_left.clamp(0.0, 1.0);
        if f >= 0.9995 {
            return self.body.clone();
        }
        if f <= 0.0005 {
            return String::new();
        }
        let start = 1.0 - f;
        let (cx, cy) = self.center;
        let mut d = format!("M {cx:.1} {cy:.1}");
        let mut line = |p: Point| {
            let _ = write!(d, " L {:.1} {:.1}", p.0, p.1);
        };
        line(self.point_at(start, ray_hit(self.center, &self.outline, start)));
        let first = (start * SAMPLES as f32).floor() as usize + 1;
        for i in first..SAMPLES {
            line(self.point_at(i as f32 / SAMPLES as f32, self.radii[i]));
        }
        line(self.point_at(0.0, self.radii[0]));
        d.push_str(" Z");
        d
    }
}

/// `(d, translate)` of every `<path>` element, in document order.
fn svg_paths(svg: &str) -> Vec<(String, Point)> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let key = format!(" {name}=\"");
        let start = tag.find(&key)? + key.len();
        let len = tag[start..].find('"')?;
        Some(tag[start..start + len].to_string())
    };
    svg.match_indices("<path")
        .filter_map(|(i, _)| {
            let tag = &svg[i..i + svg[i..].find('>')?];
            let d = attr(tag, "d")?;
            let offset = attr(tag, "transform")
                .and_then(|t| {
                    let inner = t.strip_prefix("translate(")?.strip_suffix(')')?;
                    let mut n = inner.split([' ', ',']).filter(|s| !s.is_empty()).map(str::parse::<f32>);
                    let x = n.next()?.ok()?;
                    let y = n.next().and_then(Result::ok).unwrap_or(0.0);
                    Some((x, y))
                })
                .unwrap_or((0.0, 0.0));
            Some((d, offset))
        })
        .collect()
}

enum Token {
    Cmd(char),
    Num(f32),
}

fn tokens(d: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut num = String::new();
    let flush = |num: &mut String, out: &mut Vec<Token>| {
        if let Ok(v) = num.parse() {
            out.push(Token::Num(v));
        }
        num.clear();
    };
    for c in d.chars() {
        match c {
            c if c.is_ascii_alphabetic() && c != 'e' && c != 'E' => {
                flush(&mut num, &mut out);
                out.push(Token::Cmd(c));
            }
            '-' if !num.is_empty() && !num.ends_with(['e', 'E']) => {
                flush(&mut num, &mut out);
                num.push(c);
            }
            c if c.is_whitespace() || c == ',' => flush(&mut num, &mut out),
            c => num.push(c),
        }
    }
    flush(&mut num, &mut out);
    out
}

/// The path as a closed polygon.
fn flatten(d: &str) -> Vec<Point> {
    let mut pts: Vec<Point> = Vec::new();
    let mut cmd = 'M';
    let mut nums: Vec<f32> = Vec::new();
    let mut cur = (0.0, 0.0);

    let apply = |cmd: char, n: &[f32], cur: &mut Point, pts: &mut Vec<Point>| match cmd {
        'M' | 'L' => {
            *cur = (n[0], n[1]);
            pts.push(*cur);
        }
        'C' => {
            let (p0, p1, p2, p3) = (*cur, (n[0], n[1]), (n[2], n[3]), (n[4], n[5]));
            for s in 1..=CURVE_STEPS {
                let t = s as f32 / CURVE_STEPS as f32;
                let u = 1.0 - t;
                let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                pts.push((
                    a * p0.0 + b * p1.0 + c * p2.0 + e * p3.0,
                    a * p0.1 + b * p1.1 + c * p2.1 + e * p3.1,
                ));
            }
            *cur = p3;
        }
        _ => {}
    };
    let arity = |c: char| match c {
        'M' | 'L' => 2,
        'C' => 6,
        _ => 0,
    };

    for tok in tokens(d) {
        match tok {
            Token::Cmd(c) => {
                assert!(
                    matches!(c, 'M' | 'L' | 'C' | 'Z'),
                    "unsupported SVG path command '{c}'"
                );
                cmd = c;
                nums.clear();
            }
            Token::Num(v) => {
                nums.push(v);
                if nums.len() == arity(cmd) {
                    apply(cmd, &nums, &mut cur, &mut pts);
                    nums.clear();
                    // Extra coordinates after M are implicit L.
                    if cmd == 'M' {
                        cmd = 'L';
                    }
                }
            }
        }
    }
    pts
}

/// Re-emits an absolute path with every coordinate pair shifted.
fn translate(d: &str, dx: f32, dy: f32) -> String {
    if dx == 0.0 && dy == 0.0 {
        return d.to_string();
    }
    let mut out = String::new();
    let mut is_x = true;
    for tok in tokens(d) {
        match tok {
            Token::Cmd(c) => {
                let _ = write!(out, "{c} ");
                is_x = true;
            }
            Token::Num(v) => {
                let _ = write!(out, "{} ", if is_x { v + dx } else { v + dy });
                is_x = !is_x;
            }
        }
    }
    out.trim_end().to_string()
}

fn bounds(pts: &[Point]) -> (Point, Point) {
    pts.iter().fold(
        ((f32::MAX, f32::MAX), (f32::MIN, f32::MIN)),
        |(lo, hi), &(x, y)| ((lo.0.min(x), lo.1.min(y)), (hi.0.max(x), hi.1.max(y))),
    )
}

/// Distance from `c` to the farthest outline crossing in direction `turn`.
fn ray_hit(c: Point, poly: &[Point], turn: f32) -> f32 {
    let a = turn * TAU;
    let dir = (a.sin(), -a.cos());
    let mut best = 0.0f32;
    for i in 0..poly.len() {
        let p = poly[i];
        let q = poly[(i + 1) % poly.len()];
        let e = (q.0 - p.0, q.1 - p.1);
        let denom = dir.0 * e.1 - dir.1 * e.0;
        if denom.abs() < 1e-9 {
            continue;
        }
        let w = (p.0 - c.0, p.1 - c.1);
        let t = (w.0 * e.1 - w.1 * e.0) / denom;
        let s = (w.0 * dir.1 - w.1 * dir.0) / denom;
        if t > 0.0 && (0.0..=1.0).contains(&s) {
            best = best.max(t);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_body_and_crown() {
        let t = Tomato::load();
        assert!(t.body.starts_with('M') && t.body.ends_with('Z'));
        assert!(t.crown.starts_with('M'));
        assert!(t.radii.iter().all(|&r| r > 100.0), "every direction hits the body");
        let (x, y, size) = t.viewbox;
        assert!(size > 0.0 && x < t.center.0 && y < t.center.1);
    }

    #[test]
    fn reads_translate() {
        let svg = r#"<svg><path d="M 0 0 Z"/><path d="M 1 2 C 3 4 5 6 7 8 Z" transform="translate(0 6)"/></svg>"#;
        let p = svg_paths(svg);
        assert_eq!(p.len(), 2);
        assert_eq!(p[1].1, (0.0, 6.0));
        assert_eq!(translate(&p[1].0, 0.0, 6.0), "M 1 8 C 3 10 5 12 7 14 Z");
    }

    #[test]
    fn flattens_curves() {
        let pts = flatten("M 0 0 L 10 0 C 10 5 10 5 10 10 Z");
        assert_eq!(pts.len(), 2 + CURVE_STEPS);
        assert_eq!(*pts.last().unwrap(), (10.0, 10.0));
    }

    #[test]
    fn ray_hits_square_edge() {
        let square = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        assert!((ray_hit((0.0, 0.0), &square, 0.0) - 1.0).abs() < 1e-5);
        assert!((ray_hit((0.0, 0.0), &square, 0.125) - 2f32.sqrt()).abs() < 1e-4);
    }

    #[test]
    fn wedge_extremes_and_size() {
        let t = Tomato::load();
        assert_eq!(t.wedge(1.0), t.body);
        assert_eq!(t.wedge(0.0), "");
        let count = |s: &str| s.matches(" L ").count();
        // Half left: about half the samples, plus the two cut points.
        assert_eq!(count(&t.wedge(0.5)), SAMPLES / 2 + 1);
        assert!(count(&t.wedge(0.25)) < count(&t.wedge(0.75)));
    }
}
