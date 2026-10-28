use std::error::Error;

use clap::{Args, Parser, Subcommand};
use egyptian_fraction_search::{
    Decomposition, GreedyLimits, GreedyReport, GreedyStatus, Metrics, Objective, Rational,
    SearchConfig, SearchResult, SearchStatus, bounded_search, greedy, verify, verify_bounded,
    verify_prefix,
};
use num_bigint::BigUint;

#[derive(Parser)]
#[command(
    version,
    about = "Exact Egyptian fractions and bounded searches for better decompositions"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Construct the greedy decomposition using exact integers.
    Greedy(GreedyArgs),
    /// Search small fractions with increasing denominators and explicit bounds.
    Search(SearchArgs),
    /// Compare greedy with a bounded search, including separate work metrics.
    Compare(CompareArgs),
    /// Check an increasing list of unit denominators by exact addition.
    Verify {
        fraction: Rational,
        #[arg(required = true, num_args = 1.., value_delimiter = ',')]
        denominators: Vec<BigUint>,
    },
}

#[derive(Args)]
struct GreedyOptions {
    /// Stop after this many greedy terms and report the exact remainder.
    #[arg(long, default_value_t = 128)]
    max_steps: usize,
    /// Maximum bit length of greedy denominators and reduced remainders.
    #[arg(long, default_value_t = 16_384)]
    max_bits: u64,
}

impl GreedyOptions {
    fn limits(&self) -> GreedyLimits {
        GreedyLimits {
            max_steps: self.max_steps,
            max_bits: self.max_bits,
        }
    }
}

#[derive(Args)]
struct GreedyArgs {
    fraction: Rational,
    #[command(flatten)]
    options: GreedyOptions,
}

#[derive(Args)]
struct SearchArgs {
    /// Positive proper input; reduced numerator <= 32 and denominator <= 200.
    fraction: Rational,
    /// Inclusive denominator ceiling (2..=10000).
    #[arg(long, default_value_t = 200)]
    max_denominator: u64,
    /// Inclusive term ceiling (1..=8).
    #[arg(long, default_value_t = 5)]
    max_terms: usize,
    /// Maximum entered DFS states; zero returns an incomplete search immediately.
    #[arg(long, default_value_t = 1_000_000)]
    max_nodes: u64,
    /// Ranking: terms, denominator, or the nondominated Pareto frontier.
    #[arg(long, default_value = "pareto", value_name = "OBJECTIVE")]
    objective: Objective,
}

impl SearchArgs {
    fn config(&self) -> SearchConfig {
        SearchConfig {
            max_denominator: self.max_denominator,
            max_terms: self.max_terms,
            max_nodes: self.max_nodes,
            objective: self.objective,
        }
    }
}

#[derive(Args)]
struct CompareArgs {
    #[command(flatten)]
    search: SearchArgs,
    #[command(flatten)]
    greedy: GreedyOptions,
}

pub fn run() -> Result<u8, Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Greedy(args) => {
            let report = greedy(&args.fraction, args.options.limits())?;
            print_greedy(&args.fraction, &report)?;
            Ok(if report.status == GreedyStatus::Complete {
                0
            } else {
                3
            })
        }
        Command::Search(args) => {
            let result = bounded_search(&args.fraction, &args.config())?;
            print_search(&args, &result)?;
            Ok(if result.status == SearchStatus::BudgetExhausted {
                3
            } else {
                0
            })
        }
        Command::Compare(args) => {
            let result = bounded_search(&args.search.fraction, &args.search.config())?;
            let report = greedy(&args.search.fraction, args.greedy.limits())?;
            print_greedy(&args.search.fraction, &report)?;
            if report.status == GreedyStatus::Complete {
                let within_bounds = verify_bounded(
                    &args.search.fraction,
                    &report.decomposition,
                    args.search.max_denominator,
                    args.search.max_terms,
                )
                .is_ok();
                println!(
                    "Greedy within search bounds: {}",
                    if within_bounds { "yes" } else { "no" }
                );
            }
            println!();
            print_search(&args.search, &result)?;
            Ok(
                if report.status != GreedyStatus::Complete
                    || result.status == SearchStatus::BudgetExhausted
                {
                    3
                } else {
                    0
                },
            )
        }
        Command::Verify {
            fraction,
            denominators,
        } => {
            let decomposition = Decomposition { denominators };
            verify(&fraction, &decomposition)?;
            println!("Verified: {fraction} = {decomposition}");
            print_quality(&decomposition);
            Ok(0)
        }
    }
}

fn print_greedy(input: &Rational, report: &GreedyReport) -> Result<(), Box<dyn Error>> {
    match report.status {
        GreedyStatus::Complete => {
            verify(input, &report.decomposition)?;
            println!("Greedy status: complete");
            println!("Greedy: {input} = {}", report.decomposition);
            println!("Verified: exact sum and strictly increasing denominators");
        }
        GreedyStatus::StepLimit | GreedyStatus::BitLimit => {
            verify_prefix(input, &report.decomposition, &report.remainder)?;
            let reason = if report.status == GreedyStatus::StepLimit {
                "step limit"
            } else {
                "bit limit"
            };
            println!("Greedy status: incomplete ({reason})");
            println!("Greedy prefix: {}", report.decomposition);
            println!("Remainder: {}", report.remainder);
            println!("Verified: prefix + remainder = {input}");
        }
    }
    print_quality(&report.decomposition);
    print_metrics("Greedy", &report.metrics);
    Ok(())
}

fn print_search(args: &SearchArgs, result: &SearchResult) -> Result<(), Box<dyn Error>> {
    println!("Input: {}", args.fraction);
    println!(
        "Bounds: denominator <= {}, terms <= {}, nodes <= {}",
        args.max_denominator, args.max_terms, args.max_nodes
    );
    println!("Objective: {}", args.objective);
    println!(
        "Search status: {}",
        match result.status {
            SearchStatus::Completed => "optimal within bounds",
            SearchStatus::NoSolutionWithinBounds => "no solution within bounds",
            SearchStatus::BudgetExhausted => "budget exhausted; search incomplete",
        }
    );
    if result.solutions.is_empty() && result.status == SearchStatus::BudgetExhausted {
        println!("No complete decomposition found before the budget was exhausted");
    }
    for (index, solution) in result.solutions.iter().enumerate() {
        verify_bounded(
            &args.fraction,
            solution,
            args.max_denominator,
            args.max_terms,
        )?;
        let label = if result.status == SearchStatus::BudgetExhausted {
            "Best found"
        } else {
            "Solution"
        };
        println!("{label} {}: {} = {solution}", index + 1, args.fraction);
        print_quality(solution);
        println!("Verified: exact sum, increasing denominators, and search bounds");
    }
    print_metrics("Search", &result.metrics);
    Ok(())
}

fn print_quality(decomposition: &Decomposition) {
    let largest = decomposition
        .largest_denominator()
        .map_or_else(|| "none".into(), ToString::to_string);
    println!(
        "Terms: {}; largest denominator: {largest}",
        decomposition.term_count()
    );
}

fn print_metrics(label: &str, metrics: &Metrics) {
    println!(
        "{label} work: denominators examined: {}; search nodes: {}; search branches: {}; arithmetic operations: {}; GCD steps: {}; runtime: {:.3} ms",
        metrics.denominators_examined,
        metrics.search_nodes,
        metrics.search_branches,
        metrics.arithmetic_operations,
        metrics.gcd_steps,
        metrics.elapsed.as_secs_f64() * 1000.0
    );
}
