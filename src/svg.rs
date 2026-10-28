use std::{error::Error, fmt, fmt::Write};

use num_bigint::BigUint;
use num_traits::ToPrimitive;

use crate::{Decomposition, Rational, VerificationError, verify};

const WIDTH: u64 = 1_000;
const BAR_X: u64 = 52;
const BAR_WIDTH: u64 = 896;
const BAR_HEIGHT: u64 = 32;
const NANO_PIXELS: u64 = 1_000_000_000;
const COLORS: [&str; 8] = [
    "#3266d6", "#18a08c", "#e8aa35", "#a36ac7", "#dc6d64", "#50a5c9", "#809640", "#bd739a",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgRow {
    /// Include algorithm names and completion status here when comparing runs.
    pub label: String,
    pub decomposition: Decomposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SvgError {
    EmptyRows,
    InvalidInput,
    InvalidRow {
        index: usize,
        source: VerificationError,
    },
}

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRows => f.write_str("SVG comparison requires at least one decomposition"),
            Self::InvalidInput => f.write_str("SVG input must be positive and proper"),
            Self::InvalidRow { index, source } => {
                write!(f, "SVG row {} failed verification: {source}", index + 1)
            }
        }
    }
}

impl Error for SvgError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRow { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Render a standalone, accessible comparison of complete verified solutions.
///
/// Every bar has the same width and represents the entire input fraction. A unit
/// 1/d occupies b/(a*d) of that width for input a/b. Geometry is computed using
/// integer billionths of a pixel before a bounded conversion to floating point;
/// no BigUint is converted directly to f64. Sub-nanopixel terms may have zero
/// visible width, and their complete exact labels remain in the external key.
/// Verification, including strict denominator ordering, precedes rendering.
pub fn render_comparison(
    input: &Rational,
    rows: &[SvgRow],
    subtitle: &str,
) -> Result<String, SvgError> {
    if rows.is_empty() {
        return Err(SvgError::EmptyRows);
    }
    if !input.is_proper() {
        return Err(SvgError::InvalidInput);
    }
    for (index, row) in rows.iter().enumerate() {
        verify(input, &row.decomposition)
            .map_err(|source| SvgError::InvalidRow { index, source })?;
    }

    let mut body = String::new();
    writeln!(
        body,
        "<text class=\"heading\" x=\"52\" y=\"54\">Egyptian fraction comparison</text>"
    )
    .unwrap();
    let input_lines = wrap_text(&format!("Target: {input}"), 108);
    text_lines(&mut body, "mono", BAR_X, 84, &input_lines, 18);
    let mut y = 84 + input_lines.len() as u64 * 18 + 8;
    if !subtitle.trim().is_empty() {
        let lines = wrap_text(subtitle, 112);
        text_lines(&mut body, "muted", BAR_X, y, &lines, 18);
        y += lines.len() as u64 * 18 + 10;
    }
    let explanation = wrap_text(
        "Each full bar represents the target fraction. Colored segments show the proportional shares of its distinct unit fractions.",
        112,
    );
    text_lines(&mut body, "muted", BAR_X, y, &explanation, 18);
    y += explanation.len() as u64 * 18 + 18;

    for (index, row) in rows.iter().enumerate() {
        y = render_row(&mut body, input, row, index, y) + 16;
    }
    text_lines(
        &mut body,
        "muted",
        BAR_X,
        y + 4,
        &wrap_text(
            "Verified with exact integer arithmetic. Very small segments may be invisible; every unit fraction is listed in full below its bar.",
            112,
        ),
        18,
    );
    let height = y + 64;
    let description = format!(
        "Comparison of {} exact decompositions of {input}. Each aligned bar represents the entire target fraction. Segment widths are proportional; full unit labels, term counts, and largest denominators are shown. {subtitle}",
        rows.len()
    );
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{height}\" viewBox=\"0 0 {WIDTH} {height}\" role=\"img\" aria-labelledby=\"chart-title chart-desc\">\n<title id=\"chart-title\">Egyptian fraction comparison for {}</title>\n<desc id=\"chart-desc\">{}</desc>\n<defs><style>text{{font-family:Arial,Helvetica,sans-serif;fill:#24324a}}.heading{{font-size:26px;font-weight:700}}.label{{font-size:16px;font-weight:700}}.muted{{font-size:13px;fill:#63718a}}.mono{{font-family:Consolas,monospace;font-size:13px}}.key{{font-family:Consolas,monospace;font-size:12.5px}}</style></defs>\n<rect width=\"{WIDTH}\" height=\"{height}\" fill=\"#f3f5fa\"/>\n{body}</svg>\n",
        escape_xml(&input.to_string()),
        escape_xml(&description)
    ))
}

fn render_row(body: &mut String, input: &Rational, row: &SvgRow, index: usize, top: u64) -> u64 {
    let mut content = String::new();
    let labels = wrap_text(&row.label, 92);
    text_lines(&mut content, "label", BAR_X, top + 30, &labels, 21);
    let metric_y = top + 30 + labels.len() as u64 * 21 + 2;
    let metrics = wrap_text(
        &format!(
            "Terms: {}    Largest denominator: {}",
            row.decomposition.term_count(),
            row.decomposition
                .largest_denominator()
                .expect("verified decompositions are nonempty")
        ),
        108,
    );
    text_lines(&mut content, "mono", BAR_X, metric_y, &metrics, 18);
    let bar_y = metric_y + metrics.len() as u64 * 18 + 12;
    writeln!(
        content,
        "<defs><clipPath id=\"bar-{index}\"><rect x=\"{BAR_X}\" y=\"{bar_y}\" width=\"{BAR_WIDTH}\" height=\"{BAR_HEIGHT}\" rx=\"7\"/></clipPath></defs>\n<rect class=\"track\" x=\"{BAR_X}\" y=\"{bar_y}\" width=\"{BAR_WIDTH}\" height=\"{BAR_HEIGHT}\" rx=\"7\" fill=\"#e6eaf2\"/>\n<g clip-path=\"url(#bar-{index})\">"
    )
    .unwrap();

    let mut offset = 0_u64;
    let scaled_numerator = input.denominator() * BigUint::from(BAR_WIDTH * NANO_PIXELS);
    let mut unit_labels = Vec::with_capacity(row.decomposition.term_count());
    for (term, denominator) in row.decomposition.denominators.iter().enumerate() {
        // Independent verification implies each ratio is in [0,1], so the
        // quotient is bounded by BAR_WIDTH*NANO_PIXELS and fits exactly in u64.
        let scaled_width = (&scaled_numerator / (input.numerator() * denominator))
            .to_u64()
            .expect("verified unit fractions fit inside the normalized bar");
        let x = BAR_X as f64 + offset as f64 / NANO_PIXELS as f64;
        let width = scaled_width as f64 / NANO_PIXELS as f64;
        let label = format!("1/{denominator}");
        writeln!(
            content,
            "<rect class=\"segment\" x=\"{x:.9}\" y=\"{bar_y}\" width=\"{width:.9}\" height=\"{BAR_HEIGHT}\" fill=\"{}\" data-denominator=\"{denominator}\"><title>{}</title></rect>",
            COLORS[term % COLORS.len()],
            escape_xml(&label)
        )
        .unwrap();
        offset += scaled_width;
        unit_labels.push(label);
    }
    content.push_str("</g>\n");

    // Short labels use a compact grid. Any long denominator gets the complete
    // row width, with hard wrapping of long digit strings and no abbreviation.
    let columns = if unit_labels.iter().all(|label| label.len() <= 22) {
        4
    } else {
        1
    };
    let mut legend_y = bar_y + BAR_HEIGHT + 26;
    for (chunk_index, chunk) in unit_labels.chunks(columns).enumerate() {
        let mut line_count = 1;
        for (column, label) in chunk.iter().enumerate() {
            let term = chunk_index * columns + column;
            let x = BAR_X + column as u64 * (BAR_WIDTH / columns as u64);
            let lines = wrap_text(label, if columns == 1 { 112 } else { 25 });
            line_count = line_count.max(lines.len());
            writeln!(
                content,
                "<rect x=\"{x}\" y=\"{}\" width=\"10\" height=\"10\" rx=\"2\" fill=\"{}\"/>",
                legend_y - 10,
                COLORS[term % COLORS.len()]
            )
            .unwrap();
            text_lines(&mut content, "key", x + 18, legend_y, &lines, 18);
        }
        legend_y += line_count as u64 * 18 + 6;
    }
    let bottom = legend_y + 12;
    writeln!(
        body,
        "<rect class=\"panel\" x=\"24\" y=\"{top}\" width=\"952\" height=\"{}\" rx=\"14\" fill=\"#ffffff\" stroke=\"#dce2ee\"/>\n{content}",
        bottom - top
    )
    .unwrap();
    bottom
}

fn text_lines(output: &mut String, class: &str, x: u64, y: u64, lines: &[String], spacing: u64) {
    write!(output, "<text class=\"{class}\" x=\"{x}\" y=\"{y}\">").unwrap();
    for (index, line) in lines.iter().enumerate() {
        write!(
            output,
            "<tspan x=\"{x}\" dy=\"{}\">{}</tspan>",
            if index == 0 { 0 } else { spacing },
            escape_xml(line)
        )
        .unwrap();
    }
    output.push_str("</text>\n");
}

fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    let mut remaining = text.trim();
    let mut lines = Vec::new();
    while let Some((boundary, _)) = remaining.char_indices().nth(max_chars) {
        let split = remaining[..boundary]
            .char_indices()
            .rev()
            .find(|(index, character)| *index > 0 && character.is_whitespace())
            .map_or(boundary, |(index, _)| index);
        lines.push(remaining[..split].trim_end().to_owned());
        remaining = remaining[split..].trim_start();
    }
    if !remaining.is_empty() || lines.is_empty() {
        lines.push(remaining.to_owned());
    }
    lines
}

fn escape_xml(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\u{0}'..='\u{8}'
            | '\u{b}'
            | '\u{c}'
            | '\u{e}'..='\u{1f}'
            | '\u{fffe}'
            | '\u{ffff}' => {
                escaped.push('\u{fffd}');
            }
            _ => escaped.push(character),
        }
    }
    escaped
}
