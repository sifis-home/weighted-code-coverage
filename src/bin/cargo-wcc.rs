use std::path::PathBuf;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use weighted_code_coverage::error::*;
use weighted_code_coverage::files::*;
use weighted_code_coverage::functions::*;
use weighted_code_coverage::output::*;
use weighted_code_coverage::utility::{Complexity, JsonFormat, Mode, Sort};

const fn thresholds_long_help() -> &'static str {
    "Set four  thresholds in this order: -t WCC_PLAIN, WCC_QUANTIZED, CRAP, SKUNK\n
    All the values must be floats\n
    All Thresholds has 0 as minimum value, thus no threshold at all.\n
    WCC PLAIN has a max threshold of COMP*SLOC/PLOC\n
    WCC QUANTIZED has a max threshold of 2*SLOC/PLOC\n
    CRAP has a max threshold of COMP^2 +COMP\n
    SKUNK has a max threshold of COMP/25\n"
}

#[derive(Debug, Clone, PartialEq)]
struct Thresholds(Vec<f64>);

impl std::str::FromStr for Thresholds {
    type Err = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Thresholds(
            s.split(',')
                .map(|x| x.trim().parse::<f64>().unwrap())
                .collect::<Vec<f64>>(),
        ))
    }
}

fn run_functions(args: &Args) -> Result<()> {
    let metric_to_use = args.complexity;
    let thresholds = &args.thresholds.0;
    let sort_by = args.sort;
    let (metrics, files_ignored, complex_files, project_coverage) = match args.json_format {
        JsonFormat::Covdir => get_functions_metrics_concurrent_covdir(
            &args.path_file,
            &args.path_json,
            metric_to_use,
            args.n_threads.max(1),
            thresholds,
            sort_by,
        )?,
        JsonFormat::Coveralls => get_functions_metrics_concurrent(
            &args.path_file,
            &args.path_json,
            metric_to_use,
            args.n_threads.max(1),
            thresholds,
            sort_by,
        )?,
    };
    if let Some(csv) = &args.path_csv {
        print_metrics_to_csv_function(&metrics, &files_ignored, csv, project_coverage, sort_by)?;
    }
    if let Some(json) = &args.json_output {
        print_metrics_to_json_function(
            &metrics,
            &files_ignored,
            &json,
            &&args.path_file,
            project_coverage,
            sort_by,
        )?;
    };
    if let Some(html) = &args.path_html {
        print_metrics_to_html_function(
            &metrics,
            &files_ignored,
            &html,
            &&args.path_file,
            project_coverage,
            sort_by,
        )?;
    };
    get_metrics_output_function(&metrics, &files_ignored, &complex_files);
    Ok(())
}

fn run_files(args: &Args) -> Result<()> {
    let metric_to_use = args.complexity;
    let thresholds = &args.thresholds.0;
    let sort_by = args.sort;
    let (metrics, files_ignored, complex_files, project_coverage) = match args.json_format {
        JsonFormat::Covdir => get_metrics_concurrent_covdir(
            &args.path_file,
            &args.path_json,
            metric_to_use,
            args.n_threads.max(1),
            thresholds,
            sort_by,
        )?,
        JsonFormat::Coveralls => get_metrics_concurrent(
            &args.path_file,
            &args.path_json,
            metric_to_use,
            args.n_threads.max(1),
            thresholds,
            sort_by,
        )?,
    };
    if let Some(csv) = &args.path_csv {
        print_metrics_to_csv(&metrics, &files_ignored, csv, project_coverage, sort_by)?;
    }
    if let Some(json) = &args.json_output {
        print_metrics_to_json(
            &metrics,
            &files_ignored,
            &json,
            &&args.path_file,
            project_coverage,
            sort_by,
        )?;
    };
    if let Some(html) = &args.path_html {
        print_metrics_to_html(
            &metrics,
            &files_ignored,
            &html,
            &&args.path_file,
            project_coverage,
            sort_by,
        )?;
    };
    get_metrics_output(&metrics, &files_ignored, &complex_files);
    Ok(())
}

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to the project folder
    #[clap(short, value_hint = clap::ValueHint::DirPath)]
    path_file: PathBuf,

    /// Path to the grcov json in coveralls/covdir format
    #[clap(short = 'j', long = "path_json", value_hint = clap::ValueHint::DirPath)]
    path_json: PathBuf,
    /// Path where to save the output of the csv file
    #[clap(long = "csv", value_hint = clap::ValueHint::DirPath)]
    path_csv: Option<PathBuf>,
    /// Path where to save the output of the HTML file
    #[clap(long = "html", value_hint = clap::ValueHint::DirPath)]
    path_html: Option<PathBuf>,
    /// Path where to save the output of the json file
    #[clap(long = "json", value_hint = clap::ValueHint::DirPath)]
    json_output: Option<PathBuf>,
    /// Choose complexity metric to use
    #[clap(long, short, default_value = Complexity::default(), value_parser = PossibleValuesParser::new(Complexity::all())
        .map(|s| s.parse::<Complexity>().unwrap()))]
    complexity: Complexity,

    /// Number of threads to use for concurrency
    #[clap(default_value_t = 2)]
    n_threads: usize,
    /// Specify the type of format used between coveralls and covdir
    #[clap(long, short = 'f', default_value= JsonFormat::default(), value_parser = PossibleValuesParser::new(JsonFormat::all())
        .map(|s| s.parse::<JsonFormat>().unwrap()))]
    json_format: JsonFormat,
    #[clap(long, short, long_help = thresholds_long_help(), default_value = "35.0,1.5,35.0,30.0")]
    thresholds: Thresholds,
    /// Output the generated paths as they are produced
    #[clap(short, long, global = true)]
    verbose: bool,
    /// Choose mode to use for analysis
    #[clap(long, short = 'm', default_value= Mode::default(), value_parser = PossibleValuesParser::new(Mode::all())
        .map(|s| s.parse::<Mode>().unwrap()))]
    mode: Mode,
    /// Sort complex value with the chosen metric
    #[clap(long, short, default_value = Sort::default(), value_parser = PossibleValuesParser::new(Sort::all())
        .map(|s| s.parse::<Sort>().unwrap()))]
    sort: Sort,
}

#[derive(Subcommand)]
enum Cmd {
    /// Weighted Code Coverage cargo subcommand
    #[clap(name = "wcc")]
    Wcc(Args),
}

/// Weighted Code Coverage cargo applet
#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    args: Cmd,
}

fn main() -> Result<()> {
    let Cli {
        args: Cmd::Wcc(args),
    } = Cli::parse();
    //let args = Args::parse();
    /*let mut cmd = cargo_metadata::MetadataCommand::new();
    if let Some(ref manifest_path) = args.manifest_path {
        cmd.manifest_path(manifest_path);
    }

    let metadata = cmd.exec()?;
    let source_path = metadata.workspace_packages()[0]
        .manifest_path
        .parent()
        .unwrap()
        .join("src")
        .into_std_path_buf();*/
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| {
            if args.verbose {
                EnvFilter::try_new("debug")
            } else {
                EnvFilter::try_new("info")
            }
        })
        .unwrap();

    tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(filter_layer)
        .with_writer(std::io::stderr)
        .init();
    match args.mode {
        Mode::Functions => run_functions(&args),
        Mode::Files => run_files(&args),
    }
}
