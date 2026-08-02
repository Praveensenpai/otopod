use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::ui::{Attributes, Color, RenderConfig, Styled, StyleSheet};
use std::time::Duration;

pub fn print_banner() {
    println!();
    println!("{}", style("┌───────────────────────────────────────────────────────────────┐").magenta());
    println!(
        "│  {}  │",
        style("    🎧  O T O P O D  ──  Anime Immersion Audio Condenser     ").cyan().bold()
    );
    println!("{}", style("└───────────────────────────────────────────────────────────────┘").magenta());
    println!();
}

pub fn custom_render_config() -> RenderConfig<'static> {
    let mut config = RenderConfig::default();
    config.prompt_prefix = Styled::new("✦ ").with_fg(Color::LightCyan).with_attr(Attributes::BOLD);
    config.highlighted_option_prefix = Styled::new("❯ ").with_fg(Color::LightMagenta).with_attr(Attributes::BOLD);
    config.scroll_up_prefix = Styled::new("▲").with_fg(Color::DarkGrey);
    config.scroll_down_prefix = Styled::new("▼").with_fg(Color::DarkGrey);
    config.selected_option = Some(StyleSheet::new().with_fg(Color::LightMagenta).with_attr(Attributes::BOLD));
    config
}

pub fn create_spinner(msg: &'static str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.magenta} {msg:.cyan}")
            .expect("Valid template"),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn create_spinner_owned(msg: String) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.magenta} {msg:.cyan}")
            .expect("Valid template"),
    );
    pb.set_message(msg);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn print_step(step: usize, total: usize, title: &str) {
    println!();
    println!(
        "{} {}",
        style(format!("── Step [{}/{}] ──", step, total)).magenta().bold(),
        style(title).cyan().bold()
    );
}

pub fn print_success(msg: &str) {
    println!(" {} {}", style("✔").green().bold(), style(msg).bold());
}

pub fn print_info(msg: &str) {
    println!(" {} {}", style("ℹ").cyan().bold(), style(msg).dim());
}

pub fn print_warning(msg: &str) {
    println!(" {} {}", style("⚠").yellow().bold(), style(msg));
}

pub fn print_error(msg: &str) {
    println!(" {} {}", style("✖").red().bold(), style(msg).red());
}
