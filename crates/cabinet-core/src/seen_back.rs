//! Plan a seen rear face across a run before making its flat panel blanks. This layout is
//! independent of the finish: later lining/overlay generators can follow the same spans.
use crate::part::{Part, PartList};
use crate::run::{Run, RunItem};
use crate::{Error, Result};

struct Span {
    owner: usize,
    members: Vec<usize>,
    template: Part,
    end: f64,
    maximum: f64,
    joints: Vec<f64>,
}

pub(crate) fn continuous(run: &Run, out: &mut [PartList]) -> Result<()> {
    let mut spans = Vec::new();
    let mut active: [Option<Span>; 2] = [None, None];
    for (i, item) in run.items.iter().enumerate() {
        let RunItem::Cabinet(c) = item else {
            match item {
                RunItem::Gap(g) => {
                    spans.extend(active[usize::from(g.line == crate::run::GapLine::Upper)].take())
                }
                RunItem::Walls(_) => {
                    for open in &mut active {
                        spans.extend(open.take());
                    }
                }
                _ => {}
            }
            continue;
        };
        let level = usize::from(matches!(
            c.kind,
            crate::CabinetKind::Wall
                | crate::CabinetKind::CornerWall
                | crate::CabinetKind::Rangehood
        ));
        let open = &mut active[level];
        let Some(p) = out[i].part(&format!("{}.SeenBack", c.id)).cloned() else {
            spans.extend(open.take());
            continue;
        };
        let maximum = c.cladding.back_max_width;
        if !maximum.is_finite() || maximum <= 0.0 || c.cladding.back_grain == crate::geom::Axis::Y {
            return Err(Error::Invalid(format!(
                "{}: seen-back maximum width must be positive, with grain across or upright",
                c.id
            )));
        }
        let same = open.as_ref().is_some_and(|s| {
            let q = &s.template;
            p.board == q.board
                && p.grain_axis == q.grain_axis
                && p.bounds.min.x >= s.end - 1e-6
                && (maximum - s.maximum).abs() < 1e-6
                && (p.bounds.min.y - q.bounds.min.y).abs() < 1e-6
                && (p.bounds.min.z - q.bounds.min.z).abs() < 1e-6
                && (p.bounds.size.y - q.bounds.size.y).abs() < 1e-6
                && (p.bounds.size.z - q.bounds.size.z).abs() < 1e-6
        });
        if !same {
            spans.extend(open.take());
        }
        let s = open.get_or_insert_with(|| Span {
            owner: i,
            members: Vec::new(),
            end: p.bounds.min.x,
            template: p.clone(),
            maximum,
            joints: Vec::new(),
        });
        s.members.push(i);
        s.end = p.bounds.max().x;
        s.joints.push(s.end);
    }
    spans.extend(active.into_iter().flatten());
    for s in spans {
        for &i in &s.members {
            let name = format!("{}.SeenBack", out[i].cabinet);
            out[i].parts.retain(|p| p.name != name);
        }
        let mut from = s.template.bounds.min.x;
        let mut count = 0;
        while from < s.end - 1e-6 {
            count += 1;
            let to = if s.end - from <= s.maximum + 1e-6 {
                s.end
            } else {
                s.joints.iter().copied().filter(|&x|x>from+1e-6 && x<=from+s.maximum+1e-6).fold(None,|best,x|Some(best.map_or(x,|b:f64|b.max(x))))
                    .unwrap_or_else(|| {
                        out[s.owner].warnings.push(format!("{}: seen-back joint falls within a cabinet; provide backing at the join",s.template.name));
                        from+s.maximum
                    })
            };
            let mut panel = s.template.clone();
            panel.bounds.min.x = from;
            panel.bounds.size.x = to - from;
            if count > 1 {
                panel.name = format!("{}{count}", s.template.name);
            }
            out[s.owner].parts.push(panel);
            from = to;
        }
    }
    Ok(())
}
