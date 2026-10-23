//! Part labels for a 62 mm label printer: one label per part, cut from continuous 62 mm tape,
//! with a QR code of the part id to scan at the saw or the edgebander.

use std::fmt::Write;

use cabinet_core::part::{Part, PartList};
use qrcode::{EcLevel, QrCode};

use crate::{esc, fmt};

/// Label size in mm: the tape's width, and the length cut for each label.
pub const LABEL_W: f64 = 62.0;
pub const LABEL_H: f64 = 40.0;
const PAD: f64 = 2.5;
/// The QR code's side, quiet zone included.
const QR: f64 = 22.0;

/// One label per part of every list, in order, as a PDF of 62 × 40 mm pages. `board` names a
/// part's board.
pub fn labels_pdf(lists: &[PartList], board: impl Fn(&Part) -> String, job: &str) -> Vec<u8> {
    let pages: Vec<String> = lists
        .iter()
        .flat_map(|l| l.parts.iter())
        .map(|p| crate::pdf::page_ops(&label_svg(p, &board(p), job), LABEL_H))
        .collect();
    crate::pdf::write_pdf_sized(&pages, LABEL_W, LABEL_H)
}

/// One part's label: QR code top left with the name, role and size beside it; board, taped edges
/// and job across the bottom.
pub fn label_svg(part: &Part, board: &str, job: &str) -> String {
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{LABEL_W}mm" height="{LABEL_H}mm" viewBox="0 0 {LABEL_W} {LABEL_H}" font-family="Helvetica, Arial, sans-serif">"##
    );
    qr(&mut svg, &part.name, PAD, PAD, QR);
    let x = PAD + QR + 1.5;
    let (l, w) = part.finished_size();
    let tape = {
        let e = part.edges;
        let n = [e.front, e.back, e.top, e.bottom, e.left, e.right]
            .iter()
            .filter(|b| **b)
            .count();
        match n {
            0 => "no edge tape".to_string(),
            1 => "1 taped edge".to_string(),
            n => format!("{n} taped edges"),
        }
    };
    let beside: [(f64, &str, String); 3] = [
        (4.2, "#000", part.name.clone()),
        (2.6, "#333", part.role.label().to_string()),
        (
            3.6,
            "#000",
            format!("{} × {} × {}", fmt(l), fmt(w), fmt(part.thickness())),
        ),
    ];
    let below: [(f64, &str, String); 3] = [
        (2.4, "#000", board.to_string()),
        (2.2, "#333", tape),
        (2.0, "#666", job.to_string()),
    ];
    let mut y = PAD + 5.5;
    for (size, colour, text) in beside {
        line(&mut svg, x, y, size, colour, &text);
        y += size + 2.6;
    }
    let mut y = PAD + QR + 2.0;
    for (size, colour, text) in below {
        line(&mut svg, PAD + 1.0, y, size, colour, &text);
        y += size + 1.6;
    }
    svg.push_str("</svg>\n");
    svg
}

fn line(svg: &mut String, x: f64, y: f64, size: f64, colour: &str, text: &str) {
    let _ = writeln!(
        svg,
        r##"<text x="{x:.2}" y="{y:.2}" font-size="{size}" fill="{colour}">{}</text>"##,
        esc(&fit(text, size, LABEL_W - x - PAD))
    );
}

/// Cuts `text` with an ellipsis to fit `width` mm at `size`, by Helvetica's average width.
fn fit(text: &str, size: f64, width: f64) -> String {
    let max = (width / (size * 0.52)).floor() as usize;
    if text.chars().count() <= max {
        text.to_string()
    } else {
        let mut t: String = text.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

/// The QR code of `data` as black rects (one per horizontal run of dark modules), `side` mm
/// square with its quiet zone, at (`x`, `y`).
fn qr(svg: &mut String, data: &str, x: f64, y: f64, side: f64) {
    let Ok(code) = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::M) else {
        return;
    };
    let n = code.width();
    let colours = code.to_colors();
    // Four modules of quiet zone each side.
    let m = side / (n + 8) as f64;
    let dark = |r: usize, c: usize| colours[r * n + c] == qrcode::Color::Dark;
    for r in 0..n {
        let mut c = 0;
        while c < n {
            if dark(r, c) {
                let start = c;
                while c < n && dark(r, c) {
                    c += 1;
                }
                let _ = writeln!(
                    svg,
                    r##"<rect x="{:.3}" y="{:.3}" width="{:.3}" height="{:.3}" fill="#000"/>"##,
                    x + (start + 4) as f64 * m,
                    y + (r + 4) as f64 * m,
                    (c - start) as f64 * m,
                    // A hair taller so rows do not show seams.
                    m + 0.01
                );
            } else {
                c += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    #[test]
    fn every_part_gets_a_62_mm_label() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let pdf = labels_pdf(std::slice::from_ref(&list), |_| "Board".into(), "Job");
        let text = String::from_utf8_lossy(&pdf);
        assert_eq!(text.matches("/Type /Page ").count(), list.parts.len());
        assert!(text.contains("/MediaBox [0 0 175.75 113.39]"), "62 × 40 mm");
    }

    #[test]
    fn the_label_carries_the_part_name_and_a_qr_code_of_it() {
        let list = build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap();
        let side = list.part("B01.LeftSide").unwrap();
        let svg = label_svg(side, "Board", "Job");
        assert!(svg.contains(">B01.LeftSide<"));
        assert!(svg.contains(">720 × 580 × 16<"));
        let code = QrCode::with_error_correction_level(b"B01.LeftSide", EcLevel::M).unwrap();
        let dark = code
            .to_colors()
            .iter()
            .filter(|c| **c == qrcode::Color::Dark)
            .count();
        // The runs drawn add up to every dark module.
        let modules: f64 = svg
            .lines()
            .filter(|l| l.starts_with("<rect"))
            .map(|l| {
                let w: f64 = l
                    .split("width=\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                w
            })
            .sum::<f64>()
            / (QR / (code.width() + 8) as f64);
        assert!((modules - dark as f64).abs() < 0.5, "{modules} vs {dark}");
    }

    #[test]
    fn long_text_is_cut_to_the_label() {
        assert_eq!(fit("short", 2.2, 30.0), "short");
        let cut = fit(&"x".repeat(80), 2.2, 30.0);
        assert!(cut.ends_with('…') && cut.chars().count() < 80);
    }
}
