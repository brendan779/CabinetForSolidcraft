//! Cut sheets as one PDF: a cover, an index and a page per part.
//!
//! The pages are the same drawings as the SVG sheets: each SVG (the small set of elements the
//! sheets use: rect, line, circle and text) is turned into PDF drawing operators. Text is set in
//! the standard Helvetica font, which every PDF reader has, so nothing is embedded.

use std::fmt::Write;

use cabinet_core::part::{Part, PartList};

use crate::{PAGE_H, PAGE_W, esc, part_sheet};

/// Index rows per column and columns per index page.
const INDEX_ROWS: usize = 42;
const INDEX_COLUMNS: usize = 3;

/// Every part of every list in one PDF: cover, index (one page per 126 parts), then the sheets in
/// list order. `board` names a part's board for its title block.
pub fn job_pdf(lists: &[PartList], board: impl Fn(&Part) -> String, job: &str) -> Vec<u8> {
    let parts: Vec<(&PartList, &Part)> = lists
        .iter()
        .flat_map(|l| l.parts.iter().map(move |p| (l, p)))
        .collect();
    let index_pages = parts.len().div_ceil(INDEX_ROWS * INDEX_COLUMNS).max(1);
    let first_sheet = 2 + index_pages;
    let mut pages = vec![cover(lists, parts.len(), job)];
    for i in 0..index_pages {
        let per_page = INDEX_ROWS * INDEX_COLUMNS;
        let chunk = &parts[(i * per_page).min(parts.len())..((i + 1) * per_page).min(parts.len())];
        pages.push(index(
            chunk,
            first_sheet + i * per_page,
            job,
            i + 1,
            index_pages,
        ));
    }
    for (l, p) in &parts {
        pages.push(part_sheet(l, p, &board(p), job));
    }
    write_pdf(&pages.iter().map(|svg| svg_to_ops(svg)).collect::<Vec<_>>())
}

fn page_start(svg: &mut String) {
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{PAGE_W}mm" height="{PAGE_H}mm" viewBox="0 0 {PAGE_W} {PAGE_H}" font-family="Helvetica, Arial, sans-serif">"##
    );
}

fn cover(lists: &[PartList], parts: usize, job: &str) -> String {
    let mut svg = String::new();
    page_start(&mut svg);
    let _ = writeln!(
        svg,
        r##"<text x="20.00" y="40.00" font-size="10">{}</text>"##,
        esc(job)
    );
    let _ = writeln!(
        svg,
        r##"<text x="20.00" y="50.00" font-size="4" fill="#555">Cut sheets: {} cabinets and pieces, {parts} parts</text>"##,
        lists.len()
    );
    let _ = writeln!(
        svg,
        r##"<line x1="20.00" y1="56.00" x2="277.00" y2="56.00" stroke="#222" stroke-width="0.3"/>"##
    );
    for (i, l) in lists.iter().enumerate().take(30) {
        let _ = writeln!(
            svg,
            r##"<text x="{:.2}" y="{:.2}" font-size="3.4">{}  ·  {} parts</text>"##,
            20.0 + (i / 15) as f64 * 120.0,
            66.0 + (i % 15) as f64 * 6.0,
            esc(&l.cabinet),
            l.parts.len()
        );
    }
    let _ = writeln!(
        svg,
        r##"<text x="20.00" y="196.00" font-size="2.6" fill="#555">Dimensions in millimetres from each part's reference corner (red dot). Red edges are taped.</text>"##
    );
    svg.push_str("</svg>\n");
    svg
}

fn index(
    parts: &[(&PartList, &Part)],
    first_page: usize,
    job: &str,
    n: usize,
    of: usize,
) -> String {
    let mut svg = String::new();
    page_start(&mut svg);
    let title = if of > 1 {
        format!("Index {n} of {of}")
    } else {
        "Index".to_string()
    };
    let _ = writeln!(
        svg,
        r##"<text x="15.00" y="20.00" font-size="6">{title}</text>"##
    );
    let _ = writeln!(
        svg,
        r##"<text x="282.00" y="20.00" font-size="3" fill="#555" text-anchor="end">{}</text>"##,
        esc(job)
    );
    for (i, (_, p)) in parts.iter().enumerate() {
        let (col, row) = (i / INDEX_ROWS, i % INDEX_ROWS);
        let x = 15.0 + col as f64 * 92.0;
        let y = 32.0 + row as f64 * 4.0;
        let (l, w) = p.finished_size();
        let _ = writeln!(
            svg,
            r##"<text x="{x:.2}" y="{y:.2}" font-size="2.6">{}  {} × {} × {}</text>"##,
            esc(&p.name),
            crate::fmt(l),
            crate::fmt(w),
            crate::fmt(p.thickness())
        );
        let _ = writeln!(
            svg,
            r##"<text x="{:.2}" y="{y:.2}" font-size="2.6" fill="#555" text-anchor="end">{}</text>"##,
            x + 86.0,
            first_page + i
        );
    }
    svg.push_str("</svg>\n");
    svg
}

/// Points per millimetre.
const PT: f64 = 72.0 / 25.4;

/// The SVG's drawing as a PDF content stream, in the SVG's own millimetre coordinates (the stream
/// starts by scaling to millimetres and flipping y to point down).
pub fn svg_to_ops(svg: &str) -> String {
    let mut ops = format!("{PT:.6} 0 0 {:.6} 0 {:.3} cm\n", -PT, PAGE_H * PT);
    for line in svg.lines() {
        let line = line.trim();
        let Some(tag) = line
            .strip_prefix('<')
            .and_then(|l| l.split([' ', '>']).next())
        else {
            continue;
        };
        let a = Attrs(attributes(line));
        let style = a.class_style();
        match tag {
            "rect" => {
                let (x, y, w, h) = (a.num("x"), a.num("y"), a.num("width"), a.num("height"));
                if paint(&mut ops, &a, style) {
                    let _ = writeln!(ops, "{x:.3} {y:.3} {w:.3} {h:.3} re {}", a.paint_op(style));
                }
                ops.push_str("Q\n");
            }
            "line" => {
                let (x1, y1, x2, y2) = (a.num("x1"), a.num("y1"), a.num("x2"), a.num("y2"));
                paint(&mut ops, &a, style);
                let _ = writeln!(ops, "{x1:.3} {y1:.3} m {x2:.3} {y2:.3} l S");
                ops.push_str("Q\n");
            }
            "circle" => {
                let (cx, cy, r) = (a.num("cx"), a.num("cy"), a.num("r"));
                if paint(&mut ops, &a, style) {
                    circle(&mut ops, cx, cy, r);
                    let _ = writeln!(ops, "{}", a.paint_op(style));
                }
                ops.push_str("Q\n");
            }
            "text" => {
                let content = line
                    .split_once('>')
                    .and_then(|(_, rest)| rest.rsplit_once("</text>"))
                    .map(|(t, _)| unescape(t))
                    .unwrap_or_default();
                text(&mut ops, &a, style, &content);
            }
            _ => {}
        }
    }
    ops
}

/// The sheets' CSS classes, as the SVG's `<style>` sets them.
#[derive(Clone, Copy, Default)]
struct Style {
    fill: Option<&'static str>,
    stroke: Option<&'static str>,
    width: f64,
    font: f64,
}

struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn num(&self, k: &str) -> f64 {
        self.get(k).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    }

    fn class_style(&self) -> Style {
        match self.get("class") {
            Some("o") => Style {
                fill: Some("#f6f3ec"),
                stroke: Some("#222"),
                width: 0.35,
                ..Style::default()
            },
            Some("t") => Style {
                fill: Some("#222"),
                font: 3.0,
                ..Style::default()
            },
            Some("s") => Style {
                fill: Some("#555"),
                font: 2.4,
                ..Style::default()
            },
            Some("d") => Style {
                stroke: Some("#4a6fa5"),
                width: 0.2,
                ..Style::default()
            },
            Some("e") => Style {
                stroke: Some("#c0392b"),
                width: 1.1,
                ..Style::default()
            },
            _ => Style::default(),
        }
    }

    fn fill(&self, s: Style) -> Option<&str> {
        match self.get("fill") {
            Some("none") => None,
            Some(c) => Some(c),
            None => s.fill,
        }
    }

    fn stroke(&self, s: Style) -> Option<&str> {
        match self.get("stroke") {
            Some("none") => None,
            Some(c) => Some(c),
            None => s.stroke,
        }
    }

    /// Fill, stroke or both, for a closed shape. SVG's default is a black fill.
    fn paint_op(&self, s: Style) -> &'static str {
        let fill =
            self.get("fill") != Some("none") && (self.fill(s).is_some() || s.stroke.is_none());
        match (fill, self.stroke(s).is_some()) {
            (true, true) => "B",
            (true, false) => "f",
            _ => "S",
        }
    }
}

/// Starts a graphics state with the element's colours and line width; false when it draws nothing.
fn paint(ops: &mut String, a: &Attrs, s: Style) -> bool {
    ops.push_str("q\n");
    let fill = a
        .fill(s)
        .or(if s.stroke.is_none() && a.stroke(s).is_none() {
            Some("#000")
        } else {
            None
        });
    if let Some(c) = fill {
        let (r, g, b) = rgb(c);
        let _ = writeln!(ops, "{r:.3} {g:.3} {b:.3} rg");
    }
    if let Some(c) = a.stroke(s) {
        let (r, g, b) = rgb(c);
        let w = a
            .get("stroke-width")
            .and_then(|v| v.parse().ok())
            .unwrap_or(if s.width > 0.0 { s.width } else { 1.0 });
        let _ = writeln!(ops, "{r:.3} {g:.3} {b:.3} RG {w:.3} w");
    }
    fill.is_some() || a.stroke(s).is_some()
}

/// A circle as four Bézier quarter arcs.
fn circle(ops: &mut String, cx: f64, cy: f64, r: f64) {
    let k = 0.552_284_75 * r;
    let _ = writeln!(
        ops,
        "{:.3} {cy:.3} m {:.3} {:.3} {:.3} {:.3} {cx:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {cy:.3} c {:.3} {:.3} {:.3} {:.3} {cx:.3} {:.3} c {:.3} {:.3} {:.3} {:.3} {:.3} {cy:.3} c h",
        cx + r,
        cx + r,
        cy + k,
        cx + k,
        cy + r,
        cy + r,
        cx - k,
        cy + r,
        cx - r,
        cy + k,
        cx - r,
        cx - r,
        cy - k,
        cx - k,
        cy - r,
        cy - r,
        cx + k,
        cy - r,
        cx + r,
        cy - k,
        cx + r
    );
}

fn text(ops: &mut String, a: &Attrs, s: Style, content: &str) {
    let size = a
        .get("font-size")
        .and_then(|v| v.trim_end_matches("px").parse().ok())
        .unwrap_or(if s.font > 0.0 { s.font } else { 3.0 });
    let (x, y) = (a.num("x"), a.num("y"));
    let bytes = win_ansi(content);
    let width = bytes.iter().map(|&b| helvetica_width(b)).sum::<f64>() * size / 1000.0;
    let shift = match a.get("text-anchor") {
        Some("middle") => -width / 2.0,
        Some("end") => -width,
        _ => 0.0,
    };
    // The y flip makes glyphs upside down; the text matrix flips them back. `rotate(-90 x y)` turns
    // the baseline to run up the page.
    let rotated = a
        .get("transform")
        .is_some_and(|t| t.starts_with("rotate(-90"));
    let tm = if rotated { "0 -1 -1 0" } else { "1 0 0 -1" };
    let (r, g, b) = rgb(a.fill(s).unwrap_or("#000"));
    let _ = writeln!(
        ops,
        "BT {r:.3} {g:.3} {b:.3} rg /F1 {size:.2} Tf {tm} {x:.3} {y:.3} Tm {shift:.3} 0 Td ({}) Tj ET",
        pdf_string(&bytes)
    );
}

fn attributes(tag: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let head = tag.split_once('>').map_or(tag, |(h, _)| h);
    let mut rest = head;
    while let Some(eq) = rest.find("=\"") {
        let name = rest[..eq].rsplit(' ').next().unwrap_or("").to_string();
        let after = &rest[eq + 2..];
        let Some(end) = after.find('"') else { break };
        out.push((name, after[..end].to_string()));
        rest = &after[end + 1..];
    }
    out
}

fn rgb(c: &str) -> (f64, f64, f64) {
    let h = c.trim_start_matches('#');
    let h: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.to_string()
    };
    let v = u32::from_str_radix(&h, 16).unwrap_or(0);
    (
        ((v >> 16) & 0xff) as f64 / 255.0,
        ((v >> 8) & 0xff) as f64 / 255.0,
        (v & 0xff) as f64 / 255.0,
    )
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// Text in the Windows code page Helvetica's WinAnsiEncoding uses. Characters it lacks become
/// their nearest look-alike (the sheets' ● and ▬ keys) or a question mark.
fn win_ansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| match c {
            ' '..='~' => c as u8,
            '●' | '•' => 0x95,
            '▬' | '—' => 0x97,
            '–' => 0x96,
            '″' => b'"',
            '′' => b'\'',
            c if (c as u32) >= 0xa0 && (c as u32) <= 0xff => c as u32 as u8,
            _ => b'?',
        })
        .collect()
}

fn pdf_string(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(b as char);
            }
            32..=126 => s.push(b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s
}

/// Helvetica advance widths (1/1000 em) from its Adobe font metrics, for centring text.
fn helvetica_width(b: u8) -> f64 {
    const ASCII: [u16; 95] = [
        278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
        278, // space-/
        556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0-9
        278, 278, 584, 584, 584, 556, 1015, // :-@
        667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722,
        667, 611, 722, 667, 944, 667, 667, 611, // A-Z
        278, 278, 278, 469, 556, 333, // [-`
        556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333,
        500, 278, 556, 500, 722, 500, 500, 500, // a-z
        334, 260, 334, 584, // {-~
    ];
    match b {
        32..=126 => ASCII[(b - 32) as usize] as f64,
        0x95 => 350.0,
        0x96 => 556.0,
        0x97 => 1000.0,
        0xd7 => 584.0,
        0xd8 => 778.0,
        0xb0 => 400.0,
        0xb7 => 278.0,
        _ => 556.0,
    }
}

/// A PDF file of A4 landscape pages, one content stream each, with Helvetica as `/F1`.
fn write_pdf(pages: &[String]) -> Vec<u8> {
    let n = pages.len();
    // Objects: 1 catalog, 2 page tree, 3 font, then per page its page object and its content.
    let mut objects: Vec<String> = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".into(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {n} >>",
            (0..n)
                .map(|i| format!("{} 0 R", 4 + 2 * i))
                .collect::<Vec<_>>()
                .join(" ")
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".into(),
    ];
    for (i, content) in pages.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            PAGE_W * PT,
            PAGE_H * PT,
            5 + 2 * i
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ));
    }
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{o}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    let mut tail = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(tail, "{o:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cabinet_core::{Cabinet, CabinetKind, build_cabinet};

    fn pages(pdf: &[u8]) -> usize {
        String::from_utf8_lossy(pdf).matches("/Type /Page ").count()
    }

    #[test]
    fn a_job_pdf_has_a_cover_an_index_and_a_page_per_part() {
        let lists = vec![
            build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap(),
            build_cabinet(&Cabinet::new("D01", CabinetKind::DrawerBank)).unwrap(),
        ];
        let parts: usize = lists.iter().map(|l| l.parts.len()).sum();
        let pdf = job_pdf(&lists, |_| "Generic".into(), "Kitchen");
        assert_eq!(pages(&pdf), parts + 2);
        assert!(pdf.starts_with(b"%PDF-1.4"));
    }

    #[test]
    fn the_cross_reference_table_points_at_every_object() {
        let lists = vec![build_cabinet(&Cabinet::new("B01", CabinetKind::Base)).unwrap()];
        let pdf = job_pdf(&lists, |_| "Generic".into(), "Kitchen");
        // Byte offsets: the header's binary marker is not text.
        let at = |needle: &[u8]| {
            pdf.windows(needle.len())
                .rposition(|w| w == needle)
                .unwrap()
        };
        let tail = String::from_utf8(pdf[at(b"startxref")..].to_vec()).unwrap();
        let start: usize = tail.lines().nth(1).unwrap().parse().unwrap();
        assert!(pdf[start..].starts_with(b"xref"));
        let table = String::from_utf8(pdf[start..].to_vec()).unwrap();
        let entries: Vec<&str> = table
            .lines()
            .skip(3)
            .take_while(|l| l.ends_with(" n "))
            .collect();
        assert_eq!(entries.len(), 3 + 2 * (lists[0].parts.len() + 2));
        for (i, line) in entries.iter().enumerate() {
            let offset: usize = line[..10].parse().unwrap();
            assert!(
                pdf[offset..].starts_with(format!("{} 0 obj", i + 1).as_bytes()),
                "object {} at {offset}",
                i + 1
            );
        }
    }

    #[test]
    fn a_long_job_gets_more_index_pages() {
        let lists: Vec<PartList> = (1..=12)
            .map(|i| build_cabinet(&Cabinet::new(format!("T{i:02}"), CabinetKind::Tall)).unwrap())
            .collect();
        let parts: usize = lists.iter().map(|l| l.parts.len()).sum();
        assert!(parts > INDEX_ROWS * INDEX_COLUMNS);
        let pdf = job_pdf(&lists, |_| "Generic".into(), "Kitchen");
        let index = parts.div_ceil(INDEX_ROWS * INDEX_COLUMNS);
        assert_eq!(pages(&pdf), parts + 1 + index);
    }

    #[test]
    fn centred_text_is_shifted_back_by_half_its_helvetica_width() {
        let ops =
            svg_to_ops(r#"<text class="t" x="100.00" y="50.00" text-anchor="middle">720</text>"#);
        // "720" is three digits of 556/1000 em at 3 mm.
        assert!(ops.contains("-2.502 0 Td (720) Tj"), "{ops}");
    }

    #[test]
    fn text_outside_ascii_is_set_in_win_ansi() {
        assert_eq!(win_ansi("Ø5 × 13"), b"\xd85 \xd7 13");
        assert_eq!(pdf_string(&win_ansi("(Ø)")), r"\(\330\)");
    }
}
