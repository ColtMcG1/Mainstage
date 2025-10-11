/// A CLI tool for Mainstage
/// Author: Colton McGraw
/// Version: 0.1.0
/// License: TBD
/// Description: A command-line interface for Mainstage operations
/// Usage: mainstage [ACTION] path/to/script <args>
/// Actions:
///   -b, --build       Build the script
///   -r, --run         Run the script
/// Example: mainstage -b path/to/script --dump --verbose
use clap::{Arg, ArgMatches, Command, error};
use console::{Style, strip_ansi_codes, measure_text_width};


const WIDTH: usize = 80;

// content width between the two single-space paddings
fn inner_content_width() -> usize {
    // line format: '│' ' ' <content> ' ' '│' => 4 extra chars
    WIDTH.saturating_sub(4)
}

fn print_top_border() {
    // ┌────────────────────────────────────────────────────────────────────┐
    println!("┌{}┐", "─".repeat(WIDTH.saturating_sub(2)));
}

fn print_bottom_border() {
    // └────────────────────────────────────────────────────────────────────┘
    println!("└{}┘", "─".repeat(WIDTH.saturating_sub(2)));
}

fn print_empty_line() {
    println!("│ {} │", " ".repeat(inner_content_width()));
}

fn build_output(message: &str, style: &Style) {
    let styled = style.apply_to(message).to_string();
    let visible = strip_ansi_codes(&styled);
    let visible_width = measure_text_width(&visible);

    let inner_w = inner_content_width();
    let padding = inner_w.saturating_sub(visible_width);

    println!("│ {}{} │", styled, " ".repeat(padding));
}

fn build_stage_output(stage: &str, message: &str, style: &Style) {
    // visible width reserved for stage label
    let stage_field: usize = 30;

    let styled_stage = style.apply_to(stage).to_string();
    let visible_stage = strip_ansi_codes(&styled_stage);
    let stage_width = measure_text_width(&visible_stage);

    // pad visible stage label to stage_field
    let stage_pad = if stage_field > stage_width { stage_field - stage_width } else { 1 };

    // build label: styled (may contain ANSI) + visible padding spaces + ": "
    let mut label = String::new();
    label.push_str(&styled_stage);
    label.push_str(&" ".repeat(stage_pad));
    label.push_str(": ");

    // compute visible width of label + message (strip all ANSI for measurement)
    let visible_label = strip_ansi_codes(&label);
    let visible_inner = format!("{}{}", visible_label, message);
    let inner_vis_width = measure_text_width(&visible_inner);

    let inner_w = inner_content_width();
    let padding = inner_w.saturating_sub(inner_vis_width);

    // print styled label (contains ANSI), then raw message, then padding and closing
    print!("│ {}{}", label, message);
    print!("{}", " ".repeat(padding));
    println!(" │");
}

fn build_cli() -> Command {
    Command::new("mainstage")
        .version("0.1.0")
        .author("Colton McGraw")
        .about("A CLI tool for Mainstage")
        .subcommand(
            Command::new("build").about("Build the script").arg(
                Arg::new("path")
                    .index(1)
                    .required(true)
                    .help("Path to the script to build"),
            ),
        )
        .subcommand(
            Command::new("run")
                .about("Run the script")
                .arg(
                    Arg::new("path")
                        .index(1)
                        .required(true)
                        .help("Path to the script to run"),
                )
                .arg(
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Perform a dry run without executing the script"),
                ),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Activate verbose mode")
                .num_args(0)
                .global(true),
        )
        .arg(
            Arg::new("dump")
                .short('d')
                .long("dump")
                .help("Dump the output of a stage")
                .value_name("stage")
                .num_args(1)
                .global(true),
        )
}

fn handle_build(matches: &ArgMatches) {
    let header_style = Style::new().green().bold();
    let message_style = Style::new().cyan();
    let error_style = Style::new().red().bold();

    print_top_border();
    print_empty_line();
    build_output("Mainstage - Starting Build Process", &header_style);
    print_empty_line();

    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        build_stage_output("Build", &format!("Building the script at: {}", path), &message_style);
        // Add build logic here
        print_empty_line();
        build_output("Mainstage - Build Process Complete", &header_style);
    } else {
        build_output("No path provided for build.", &error_style);
    }

    print_empty_line();
    print_bottom_border();
}

fn handle_run(matches: &ArgMatches) {
    if let Some(path) = matches.get_one::<String>("path").map(|s| s.as_str()) {
        println!("Running the script at: {}", path);
        // Add run logic here
    } else {
        println!("No path provided for run.");
    }
}

fn main() {
    let matches = build_cli().try_get_matches().unwrap_or_else(|e| e.exit());

    match matches.subcommand() {
        Some(("build", sub_m)) => handle_build(sub_m),
        Some(("run", sub_m)) => handle_run(sub_m),
        _ => println!("No valid command provided. Use --help for more information."),
    }
}
