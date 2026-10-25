//! Nesting diagrams: one page per stock sheet with its parts and offcuts to scale, then a summary
//! of sheets, offcuts and anything that did not fit.

use std::fmt::Write;

use cabinet_core::nesting::{Nesting, Settings, Sheet};

use crate::pdf::{svg_to_ops, write_pdf};
use crate::{PAGE_H, PAGE_W, esc};

const MARGIN: f64 = 15.0;
/// Room above the drawing for the title, below it for the legend.
const TOP: f64 = 30.0;
const BOTTOM: f64 = 16.0;
/// Summary lines per page.
const SUMMARY_ROWS: usize = 34;

fn parts(n: usize) -> String {
    if n == 1 {
        "1 part".into()
    } else {
        format!("{n} parts")
    }
}

fn page_start(svg: &mut String) {
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">"##
    );
}

/// Sheet `n` of `of` to scale: the stock outline, its trimmed edges, each part labelled with its
/// name and cut size, and the offcuts shaded with their sizes.
pub fn sheet_svg(sheet: &Sheet, n: usize, of: usize, job: &str, settings: &Settings) -> String {
    let (l, w) = (sheet.stock.length, sheet.stock.width);
    let scale = ((PAGE_W - 2.0 * MARGIN) / l).min((PAGE_H - TOP - BOTTOM) / w);
    let (ox, oy) = (MARGIN, TOP);
    let at = |x: f64, y: f64| (ox + x * scale, oy + y * scale);
    let mut svg = String::new();
    page_start(&mut svg);
    let _ = writeln!(
        svg,
        r##"<text x="{MARGIN:.2}" y="16.00" font-size="6">Sheet {n} of {of}</text>"##
    );
    let _ = writeln!(
        svg,
        r##"<text x="{MARGIN:.2}" y="23.00" font-size="3.4" fill="#555">{}  ·  {:.0} mm  ·  {:.0} × {:.0}{}  ·  {}  ·  {:.0} % used</text>"##,
        esc(&sheet.board),
        sheet.thickness,
        l,
        w,
        if sheet.stock.grained {
            "  ·  grain along the length"
        } else {
            ""
        },
        parts(sheet.placements.len()),
        sheet.yield_ratio() * 100.0
    );
    let _ = writeln!(
        svg,
        r##"<text x="{:.2}" y="16.00" font-size="3" fill="#555" text-anchor="end">{}</text>"##,
        PAGE_W - MARGIN,
        esc(job)
    );
    // The whole sheet in the trim colour, the trimmed blank over it.
    let (x0, y0) = at(0.0, 0.0);
    let _ = writeln!(
        svg,
        r##"<rect x="{x0:.2}" y="{y0:.2}" width="{:.2}" height="{:.2}" fill="#d9d4c7" stroke="#222" stroke-width="0.3"/>"##,
        l * scale,
        w * scale
    );
    let (tx, ty) = at(settings.trim, settings.trim);
    let _ = writeln!(
        svg,
        r##"<rect x="{tx:.2}" y="{ty:.2}" width="{:.2}" height="{:.2}" fill="#ffffff"/>"##,
        (l - 2.0 * settings.trim) * scale,
        (w - 2.0 * settings.trim) * scale
    );
    for o in &sheet.offcuts {
        let (x, y) = at(o.x, o.y);
        let (rw, rh) = (o.length * scale, o.width * scale);
        let _ = writeln!(
            svg,
            r##"<rect x="{x:.2}" y="{y:.2}" width="{rw:.2}" height="{rh:.2}" fill="#e8efe4" stroke="#7a9a6a" stroke-width="0.2"/>"##
        );
        if rw > 18.0 && rh > 5.0 {
            let _ = writeln!(
                svg,
                r##"<text x="{:.2}" y="{:.2}" font-size="2.4" fill="#5b7a4c" text-anchor="middle">offcut {:.0} × {:.0}</text>"##,
                x + rw / 2.0,
                y + rh / 2.0 + 0.8,
                o.length,
                o.width
            );
        }
    }
    for p in &sheet.placements {
        let r = p.rect;
        let (x, y) = at(r.x, r.y);
        let (rw, rh) = (r.length * scale, r.width * scale);
        let _ = writeln!(
            svg,
            r##"<rect x="{x:.2}" y="{y:.2}" width="{rw:.2}" height="{rh:.2}" fill="#f6f3ec" stroke="#222" stroke-width="0.35"/>"##
        );
        // Name and size where they fit; a turned part says so.
        let size = format!(
            "{:.0} × {:.0}{}",
            r.length,
            r.width,
            if p.turned { " turned" } else { "" }
        );
        let font = (rh * 0.3).clamp(1.6, 3.0);
        let fits = |s: &str| s.chars().count() as f64 * font * 0.55 < rw - 1.0;
        if rh > 2.0 * font + 1.0 && fits(&p.part) {
            let cx = x + rw / 2.0;
            let cy = y + rh / 2.0;
            let _ = writeln!(
                svg,
                r##"<text x="{cx:.2}" y="{:.2}" font-size="{font:.2}" text-anchor="middle">{}</text>"##,
                cy - 0.2,
                esc(&p.part)
            );
            if fits(&size) {
                let _ = writeln!(
                    svg,
                    r##"<text x="{cx:.2}" y="{:.2}" font-size="{:.2}" fill="#555" text-anchor="middle">{}</text>"##,
                    cy + font,
                    font * 0.85,
                    esc(&size)
                );
            }
        }
    }
    let _ = writeln!(
        svg,
        r##"<text x="{MARGIN:.2}" y="{:.2}" font-size="2.6" fill="#555">Cut sizes in mm, x along the sheet's length from the left, y across it from the top. Kerf {} mm, trim {} mm each edge. Green: offcuts to keep.</text>"##,
        PAGE_H - 8.0,
        settings.kerf,
        settings.trim
    );
    svg.push_str("</svg>\n");
    svg
}

/// Summary lines: each sheet, then the offcuts, then what did not fit.
fn summary_lines(nesting: &Nesting) -> Vec<String> {
    let mut lines = vec!["Sheets".to_string()];
    for (i, s) in nesting.sheets.iter().enumerate() {
        lines.push(format!(
            "  {}. {}, {:.0} mm, {:.0} × {:.0}: {}, {:.0} % used",
            i + 1,
            s.board,
            s.thickness,
            s.stock.length,
            s.stock.width,
            parts(s.placements.len()),
            s.yield_ratio() * 100.0
        ));
    }
    let offcuts: Vec<String> = nesting
        .sheets
        .iter()
        .enumerate()
        .flat_map(|(i, s)| {
            s.offcuts.iter().map(move |o| {
                format!(
                    "  Sheet {}: {:.0} × {:.0} ({}, {:.0} mm)",
                    i + 1,
                    o.length,
                    o.width,
                    s.board,
                    s.thickness
                )
            })
        })
        .collect();
    if !offcuts.is_empty() {
        lines.push(String::new());
        lines.push("Offcuts".to_string());
        lines.extend(offcuts);
    }
    if !nesting.unplaced.is_empty() {
        lines.push(String::new());
        lines.push("Not nested".to_string());
        lines.extend(nesting.unplaced.iter().map(|u| format!("  {u}")));
    }
    lines
}

fn summary_pages(nesting: &Nesting, job: &str) -> Vec<String> {
    let lines = summary_lines(nesting);
    let chunks: Vec<&[String]> = lines.chunks(SUMMARY_ROWS).collect();
    let n = chunks.len();
    chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let mut svg = String::new();
            page_start(&mut svg);
            let _ = writeln!(
                svg,
                r##"<text x="{MARGIN:.2}" y="20.00" font-size="6">Nesting summary{}</text>"##,
                if n > 1 {
                    format!(" {} of {n}", i + 1)
                } else {
                    String::new()
                }
            );
            let _ = writeln!(
                svg,
                r##"<text x="{:.2}" y="20.00" font-size="3" fill="#555" text-anchor="end">{}</text>"##,
                PAGE_W - MARGIN,
                esc(job)
            );
            for (j, line) in chunk.iter().enumerate() {
                let head = !line.starts_with(' ') && !line.is_empty();
                let _ = writeln!(
                    svg,
                    r##"<text x="{MARGIN:.2}" y="{:.2}" font-size="{}"{}>{}</text>"##,
                    32.0 + j as f64 * 4.8,
                    if head { "3.6" } else { "3" },
                    if head { "" } else { r##" fill="#333""## },
                    esc(line)
                );
            }
            svg.push_str("</svg>\n");
            svg
        })
        .collect()
}

/// The nesting as one PDF: the summary, then a page per sheet.
pub fn nesting_pdf(nesting: &Nesting, job: &str, settings: &Settings) -> Vec<u8> {
    let of = nesting.sheets.len();
    let mut pages = summary_pages(nesting, job);
    pages.extend(
        nesting
            .sheets
            .iter()
            .enumerate()
            .map(|(i, s)| sheet_svg(s, i + 1, of, job, settings)),
    );
    write_pdf(&pages.iter().map(|svg| svg_to_ops(svg)).collect::<Vec<_>>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::nesting::{Stock, nest};
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    fn sample() -> Nesting {
        let lists = vec![
            build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap(),
            build_cabinet(&Cabinet::new("T01", CabinetKind::Tall)).unwrap(),
        ];
        let cut = cabinet_core::cutlist::cut_list(&lists, "Generic", 1.0);
        nest(
            &cut.rows,
            |_, _| {
                Some(Stock {
                    length: 2400.0,
                    width: 1200.0,
                    grained: false,
                })
            },
            &Settings::default(),
        )
    }

    #[test]
    fn a_sheet_page_draws_every_part_and_offcut() {
        let n = sample();
        let s = &n.sheets[0];
        let svg = sheet_svg(s, 1, n.sheets.len(), "Job", &Settings::default());
        // Stock, blank, then one rect per offcut and per part.
        assert_eq!(
            svg.matches("<rect").count(),
            2 + s.offcuts.len() + s.placements.len()
        );
        assert!(svg.contains("Sheet 1 of"));
    }

    #[test]
    fn the_pdf_has_a_summary_and_a_page_per_sheet() {
        let n = sample();
        let pdf = nesting_pdf(&n, "Job", &Settings::default());
        // For looking at: NESTING_PDF_OUT=target/nesting.pdf cargo test -p cabinet-sheets.
        if let Ok(out) = std::env::var("NESTING_PDF_OUT") {
            std::fs::write(out, &pdf).unwrap();
        }
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF"));
        assert_eq!(text.matches("/Type /Page ").count(), n.sheets.len() + 1);
    }
}
