use egyptian_fraction_search::svg::{SvgError, SvgRow, render_comparison};
use egyptian_fraction_search::{Decomposition, Metrics, Rational, VerificationError};
use num_bigint::BigUint;

fn row(label: &str, denominators: &[u64]) -> SvgRow {
    SvgRow {
        label: label.to_owned(),
        decomposition: Decomposition {
            denominators: denominators.iter().copied().map(BigUint::from).collect(),
        },
    }
}

fn huge_denominator() -> BigUint {
    BigUint::from(10_u32).pow(400) + BigUint::from(7_u32)
}

#[test]
fn comparison_has_aligned_proportional_bars_metrics_and_accessibility() {
    let svg = render_comparison(
        &"2/7".parse().unwrap(),
        &[
            row("Greedy", &[4, 28]),
            row("Search completed", &[6, 14, 21]),
        ],
        "D <= 35; K <= 4",
    )
    .unwrap();
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1000\""));
    assert!(svg.contains("role=\"img\" aria-labelledby=\"chart-title chart-desc\""));
    assert!(svg.contains("<title id=\"chart-title\">"));
    assert!(svg.contains("<desc id=\"chart-desc\">"));
    assert_eq!(svg.matches("class=\"panel\"").count(), 2);
    assert_eq!(svg.matches("class=\"track\"").count(), 2);
    assert_eq!(svg.matches("class=\"segment\"").count(), 5);
    assert!(svg.contains("width=\"784.000000000\""));
    assert!(svg.contains("width=\"112.000000000\""));
    assert!(svg.contains("Terms: 2    Largest denominator: 28"));
    assert!(svg.contains("Terms: 3    Largest denominator: 21"));
    assert!(svg.contains("Each full bar represents the target fraction."));
    assert_finite_geometry(&svg);
}

#[test]
fn every_row_must_be_a_complete_verified_ordered_decomposition() {
    let input = "2/7".parse().unwrap();
    let invalid = render_comparison(&input, &[row("Partial prefix", &[4])], "").unwrap_err();
    assert_eq!(
        invalid,
        SvgError::InvalidRow {
            index: 0,
            source: VerificationError::IncorrectSum
        }
    );
    let invalid = render_comparison(
        &input,
        &[row("Valid", &[4, 28]), row("Reversed", &[28, 4])],
        "",
    )
    .unwrap_err();
    assert_eq!(
        invalid,
        SvgError::InvalidRow {
            index: 1,
            source: VerificationError::NotIncreasing { index: 1 }
        }
    );
    assert!(render_comparison(&"1/2".parse().unwrap(), &[row("Duplicates", &[4, 4])], "").is_err());
    assert!(render_comparison(&input, &[row("Empty", &[])], "").is_err());
}

#[test]
fn empty_rows_and_zero_input_are_rejected() {
    let input: Rational = "1/2".parse().unwrap();
    assert_eq!(
        render_comparison(&input, &[], "").unwrap_err(),
        SvgError::EmptyRows
    );
    let zero = input
        .subtract_unit(&BigUint::from(2_u32), &mut Metrics::default())
        .unwrap();
    assert_eq!(
        render_comparison(&zero, &[row("Zero", &[2])], "").unwrap_err(),
        SvgError::InvalidInput
    );
}

#[test]
fn arbitrary_labels_and_subtitles_are_escaped_as_xml_text() {
    let svg = render_comparison(
        &"1/2".parse().unwrap(),
        &[row("A < B & \"quoted\" 'label'", &[2])],
        "Status <complete> & 'verified' \"exact\"",
    )
    .unwrap();
    assert!(svg.contains("A &lt; B &amp; &quot;quoted&quot; &apos;label&apos;"));
    assert!(svg.contains("Status &lt;complete&gt; &amp; &apos;verified&apos; &quot;exact&quot;"));
    assert!(!svg.contains("<complete>"));
    let svg =
        render_comparison(&"1/2".parse().unwrap(), &[row("bad\u{0}label", &[2])], "").unwrap();
    assert!(!svg.contains('\u{0}'));
    assert!(svg.contains("bad\u{fffd}label"));
}

#[test]
fn huge_unit_denominators_wrap_in_full_without_float_overflow() {
    let denominator = huge_denominator();
    let input = Rational::new(BigUint::from(1_u32), denominator.clone()).unwrap();
    let svg = render_comparison(
        &input,
        &[SvgRow {
            label: "Huge exact unit fraction".to_owned(),
            decomposition: Decomposition {
                denominators: vec![denominator.clone()],
            },
        }],
        "",
    )
    .unwrap();
    assert!(svg.contains(&format!("<title>1/{denominator}</title>")));
    assert!(svg.contains("width=\"896.000000000\""));
    assert!(svg.matches("<tspan").count() >= 12);
    assert_finite_geometry(&svg);
}

#[test]
fn tiny_terms_keep_zero_width_and_exact_labels() {
    let denominator = huge_denominator();
    let input = Rational::new(
        &denominator + BigUint::from(2_u32),
        &denominator * BigUint::from(2_u32),
    )
    .unwrap();
    let svg = render_comparison(
        &input,
        &[SvgRow {
            label: "One visible term and one tiny term".to_owned(),
            decomposition: Decomposition {
                denominators: vec![BigUint::from(2_u32), denominator.clone()],
            },
        }],
        "",
    )
    .unwrap();
    let tiny = svg
        .lines()
        .find(|line| {
            line.contains("class=\"segment\"")
                && line.contains(&format!("data-denominator=\"{denominator}\""))
        })
        .unwrap();
    assert!(tiny.contains("width=\"0.000000000\""));
    assert!(tiny.contains(&format!("<title>1/{denominator}</title>")));
    assert!(svg.contains("Very small segments may be invisible;"));
    assert_finite_geometry(&svg);
}

#[test]
fn long_labels_and_many_terms_expand_the_svg_height() {
    let input = "1/2".parse().unwrap();
    let short = render_comparison(&input, &[row("Short", &[2])], "").unwrap();
    let long = render_comparison(
        &input,
        &[row(&"Long label ".repeat(100), &[2])],
        &"long subtitle ".repeat(100),
    )
    .unwrap();
    let height = |svg: &str| {
        svg.split(" height=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };
    assert!(height(&long) > height(&short));
    let many = render_comparison(
        &input,
        &[row("Five terms", &[3, 7, 43, 1807, 3_263_442])],
        "",
    )
    .unwrap();
    assert!(height(&many) > height(&short));
    assert_eq!(many.matches("class=\"segment\"").count(), 5);
    assert_finite_geometry(&many);
    assert_finite_geometry(&long);
}

fn assert_finite_geometry(svg: &str) {
    for attribute in [
        " x=\"",
        " y=\"",
        " width=\"",
        " height=\"",
        " rx=\"",
        " dy=\"",
    ] {
        for value in svg.split(attribute).skip(1) {
            let number = value.split('"').next().unwrap().parse::<f64>().unwrap();
            assert!(
                number.is_finite() && number >= 0.0,
                "invalid {attribute}: {number}"
            );
        }
    }
    let view_box = svg
        .split("viewBox=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert_eq!(view_box.split_whitespace().count(), 4);
    assert!(
        view_box
            .split_whitespace()
            .all(|number| number.parse::<f64>().unwrap().is_finite())
    );
}
