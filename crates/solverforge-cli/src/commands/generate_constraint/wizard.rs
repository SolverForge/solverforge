use dialoguer::{theme::ColorfulTheme, Select};
use owo_colors::OwoColorize;

use super::domain::DomainModel;
use super::skeleton::Pattern;

pub(crate) fn resolve_pattern_and_hardness(
    soft: bool,
    unary: bool,
    pair: bool,
    join: bool,
    balance: bool,
    reward: bool,
    domain: &Option<DomainModel>,
) -> Result<(Pattern, bool), String> {
    let explicit_pattern: Option<Pattern> = match (unary, pair, join, balance, reward) {
        (true, _, _, _, _) => Some(Pattern::Unary),
        (_, true, _, _, _) => Some(Pattern::Pair),
        (_, _, true, _, _) => Some(Pattern::Join),
        (_, _, _, true, _) => Some(Pattern::Balance),
        (_, _, _, _, true) => Some(Pattern::Reward),
        _ => None,
    };

    // balance and reward imply soft
    let pattern_implies_soft = matches!(
        explicit_pattern,
        Some(Pattern::Balance) | Some(Pattern::Reward)
    );
    let is_soft_explicit = soft || pattern_implies_soft;

    match explicit_pattern {
        Some(p) => Ok((p, is_soft_explicit)),
        None => {
            // Interactive wizard
            run_wizard(soft, domain)
        }
    }
}

fn run_wizard(soft_flag: bool, domain: &Option<DomainModel>) -> Result<(Pattern, bool), String> {
    // Print domain summary
    if let Some(d) = domain {
        println!("{} Scanning domain model...", "▸".bright_green());
        println!();
        println!("  Found: {}", d.solution_type.bright_white().bold());
        for e in &d.entities {
            let var_info = if e.planning_vars.is_empty() {
                String::new()
            } else {
                format!("  — planning variable: {}", e.planning_vars.join(", "))
            };
            println!(
                "    Entities:  {} ({}){}",
                e.field_name.bright_cyan(),
                e.item_type,
                var_info
            );
        }
        for f in &d.facts {
            println!(
                "    Facts:     {} ({})",
                f.field_name.bright_cyan(),
                f.item_type
            );
        }
        println!();
    }

    // Pattern selection
    let has_join = domain
        .as_ref()
        .map(|d| !d.facts.is_empty())
        .unwrap_or(false);

    let mut pattern_options: Vec<(&str, Pattern)> = vec![
        (
            "Penalize matching entities        (e.g. unassigned, invalid state)",
            Pattern::Unary,
        ),
        (
            "Penalize conflicting pairs        (e.g. overlapping shifts, double-booking)",
            Pattern::Pair,
        ),
    ];
    if has_join {
        pattern_options.push((
            "Penalize entity-fact mismatch     (e.g. missing skill, wrong location)",
            Pattern::Join,
        ));
    }
    pattern_options.push((
        "Balance assignments                (e.g. fair workload distribution)",
        Pattern::Balance,
    ));
    pattern_options.push((
        "Reward matching entities           (e.g. desired day, preferred shift)",
        Pattern::Reward,
    ));

    let labels: Vec<&str> = pattern_options.iter().map(|(l, _)| *l).collect();

    let pattern_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Constraint type")
        .items(&labels)
        .default(0)
        .interact()
        .map_err(|e| format!("prompt error: {}", e))?;

    let pattern = pattern_options[pattern_idx].1;

    // Hard/soft selection (skip if pattern implies soft)
    let is_soft = if matches!(pattern, Pattern::Balance | Pattern::Reward) || soft_flag {
        true
    } else {
        let hardness_idx = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Hard or soft")
            .items(&[
                "Hard  (must be satisfied — correctness)",
                "Soft  (should be optimized — quality)",
            ])
            .default(0)
            .interact()
            .map_err(|e| format!("prompt error: {}", e))?;
        hardness_idx == 1
    };

    Ok((pattern, is_soft))
}
