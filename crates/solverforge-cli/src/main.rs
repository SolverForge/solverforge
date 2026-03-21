use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

mod commands;
mod error;
mod output;
mod rc;
mod template;

use error::CliResult;

const EXAMPLES: &str = "\x1b[1mExamples:\x1b[0m
  solverforge new my-scheduler --basic=employee-scheduling
  solverforge new my-router --list=vehicle-routing
  solverforge generate entity shift --planning-variable employee_idx
  solverforge generate constraint no_overlap --pair --hard
  solverforge generate scaffold shift employee_idx:usize --entity --constraint no_overlap --pair
  solverforge server
  solverforge info
  solverforge check
  solverforge test
  solverforge routes
  solverforge config show";

#[derive(Parser)]
#[command(
    name = "solverforge",
    about = "CLI for SolverForge — a zero-erasure constraint solver in Rust",
    version,
    infer_subcommands = true,
    after_help = EXAMPLES,
)]
struct Cli {
    /// Suppress all output except errors
    #[arg(long, short, global = true)]
    quiet: bool,

    /// Show extra diagnostic output
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Disable colored output (also respects NO_COLOR env var)
    #[arg(long, global = true)]
    no_color: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold a new SolverForge project
    ///
    /// Variable class (required, mutually exclusive):
    ///
    ///   --basic     Standard variable — each entity holds one assigned value
    ///   --list=...  List variable     — each entity owns an ordered sequence
    ///
    /// Specializations (append after the flag with =):
    ///
    ///   --basic=employee-scheduling
    ///   --list=vehicle-routing
    #[command(
        after_help = "Examples:\n  solverforge new my-scheduler --basic=employee-scheduling\n  solverforge new my-router --list=vehicle-routing\n  solverforge new my-planner --basic"
    )]
    New {
        /// Project name (directory that will be created)
        name: String,

        /// Scaffold a standard-variable project (optionally: --basic=employee-scheduling)
        #[arg(long = "basic", value_name = "SPECIALIZATION", num_args = 0..=1, require_equals = true)]
        basic: Option<Option<String>>,

        /// Scaffold a list-variable project specialization (currently: --list=vehicle-routing)
        #[arg(
            long = "list",
            value_name = "SPECIALIZATION",
            num_args = 0..=1,
            require_equals = true
        )]
        list: Option<Option<String>>,

        /// Skip running `git init` and initial commit
        #[arg(long)]
        skip_git: bool,

        /// Skip generating README.md
        #[arg(long)]
        skip_readme: bool,
    },
    /// Generate a new resource for the current project
    #[command(
        after_help = "Examples:\n  solverforge generate entity shift --planning-variable employee_idx\n  solverforge generate fact employee\n  solverforge generate constraint no_overlap --pair --hard\n  solverforge generate solution schedule --score HardSoftScore"
    )]
    Generate {
        #[command(subcommand)]
        resource: GenerateResource,
    },
    /// Remove a resource from the current project
    #[command(
        after_help = "Examples:\n  solverforge destroy entity shift\n  solverforge destroy constraint no_overlap"
    )]
    Destroy {
        /// Skip confirmation prompt
        #[arg(long, short)]
        yes: bool,

        #[command(subcommand)]
        resource: DestroyResource,
    },
    /// Start the development server
    #[command(
        after_help = "Examples:\n  solverforge server\n  solverforge server --port 8080\n  solverforge server --debug"
    )]
    Server {
        /// Port to bind the server to
        #[arg(long, short, default_value = "7860")]
        port: u16,

        /// Run in debug mode (faster compilation, slower runtime)
        #[arg(long)]
        debug: bool,
    },
    /// Show project summary: entities, facts, constraints, score type
    #[command(after_help = "Examples:\n  solverforge info")]
    Info,
    /// Validate project structure and configuration
    #[command(after_help = "Examples:\n  solverforge check")]
    Check,
    /// Run `cargo test` with optional passthrough arguments
    #[command(
        after_help = "Examples:\n  solverforge test\n  solverforge test -- --nocapture\n  solverforge test integration"
    )]
    Test {
        /// Extra arguments passed directly to `cargo test`
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extra_args: Vec<String>,
    },
    /// List HTTP routes defined in src/api/
    #[command(after_help = "Examples:\n  solverforge routes")]
    Routes,
    /// Manage solver configuration (solver.toml)
    #[command(
        after_help = "Examples:\n  solverforge config show\n  solverforge config set termination.time_spent_seconds 60"
    )]
    Config {
        #[command(subcommand)]
        subcommand: ConfigSubcommand,
    },
    /// Interactive REPL console (deprecated compatibility alias)
    #[command(hide = true)]
    Console,
    /// Generate shell completions
    #[command(
        after_help = "Examples:\n  solverforge completions bash >> ~/.bashrc\n  solverforge completions zsh >> ~/.zshrc\n  solverforge completions fish > ~/.config/fish/completions/solverforge.fish"
    )]
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

#[derive(Subcommand)]
enum ConfigSubcommand {
    /// Print the contents of solver.toml
    Show,
    /// Set a key in solver.toml (e.g. termination.time_spent_seconds = 60)
    Set {
        /// Dotted key path (e.g. termination.time_spent_seconds)
        key: String,
        /// New value
        value: String,
    },
}

#[derive(Subcommand)]
enum GenerateResource {
    /// Add a new constraint skeleton to src/constraints/
    #[command(
        after_help = "Examples:\n  solverforge generate constraint max_hours --unary --hard\n  solverforge generate constraint no_overlap --pair\n  solverforge generate constraint required_skill --join --hard"
    )]
    Constraint {
        /// Constraint module name in snake_case (e.g. max_hours)
        name: String,

        /// Hard constraint — must be satisfied (default)
        #[arg(long, conflicts_with = "soft")]
        hard: bool,

        /// Soft constraint — should be optimized
        #[arg(long, conflicts_with = "hard")]
        soft: bool,

        /// Penalize matching entities (for_each + filter + penalize)
        #[arg(long, conflicts_with_all = ["pair", "join", "balance", "reward"])]
        unary: bool,

        /// Penalize conflicting pairs (for_each_unique_pair)
        #[arg(long, conflicts_with_all = ["unary", "join", "balance", "reward"])]
        pair: bool,

        /// Penalize entity-fact mismatch (for_each + join)
        #[arg(long, conflicts_with_all = ["unary", "pair", "balance", "reward"])]
        join: bool,

        /// Balance assignments across entities
        #[arg(long, conflicts_with_all = ["unary", "pair", "join", "reward"])]
        balance: bool,

        /// Reward matching entities (for_each + filter + reward)
        #[arg(long, conflicts_with_all = ["unary", "pair", "join", "balance"])]
        reward: bool,

        /// Overwrite if constraint already exists
        #[arg(long, short)]
        force: bool,

        /// Preview changes without writing files
        #[arg(long)]
        pretend: bool,
    },
    /// Scaffold a planning entity struct in src/domain/
    #[command(
        after_help = "Examples:\n  solverforge generate entity shift --planning-variable employee_idx\n  solverforge generate entity task"
    )]
    Entity {
        /// Entity name in snake_case (e.g. shift)
        name: String,

        /// Planning variable field name (e.g. employee_idx)
        #[arg(long = "planning-variable", value_name = "FIELD")]
        planning_variable: Option<String>,

        /// Additional fields in "name:Type" format (repeatable, e.g. --field "start:String")
        #[arg(long = "field", value_name = "NAME:TYPE")]
        fields: Vec<String>,

        /// Overwrite if entity already exists
        #[arg(long, short)]
        force: bool,

        /// Preview changes without writing files
        #[arg(long)]
        pretend: bool,
    },
    /// Scaffold a problem fact struct in src/domain/
    #[command(
        after_help = "Examples:\n  solverforge generate fact employee\n  solverforge generate fact location --field \"lat:f64\" --field \"lng:f64\""
    )]
    Fact {
        /// Fact name in snake_case (e.g. employee)
        name: String,

        /// Additional fields in "name:Type" format (repeatable, e.g. --field "skill:String")
        #[arg(long = "field", value_name = "NAME:TYPE")]
        fields: Vec<String>,

        /// Overwrite if fact already exists
        #[arg(long, short)]
        force: bool,

        /// Preview changes without writing files
        #[arg(long)]
        pretend: bool,
    },
    /// Scaffold a planning solution struct in src/domain/
    #[command(
        after_help = "Examples:\n  solverforge generate solution schedule --score HardSoftScore"
    )]
    Solution {
        /// Solution name in snake_case (e.g. schedule)
        name: String,

        /// Score type (e.g. HardSoftScore, HardSoftDecimalScore)
        #[arg(long, value_name = "SCORE_TYPE", default_value = "HardSoftScore")]
        score: String,
    },
    /// Add a planning variable field to an existing entity
    #[command(
        after_help = "Examples:\n  solverforge generate variable employee_idx --entity Shift"
    )]
    Variable {
        /// Field name in snake_case (e.g. preferred_shift)
        field: String,

        /// Entity struct name (e.g. Shift)
        #[arg(long, value_name = "ENTITY_TYPE")]
        entity: String,
    },
    /// Change the score type in the existing planning solution
    #[command(after_help = "Examples:\n  solverforge generate score HardSoftDecimalScore")]
    Score {
        /// Score type (e.g. HardSoftScore, HardSoftDecimalScore, HardMediumSoftScore, SimpleScore)
        score_type: String,
    },
    /// Compound generator: entity + optional constraint + optional twin entity in one go
    #[command(
        after_help = "Examples:\n  solverforge generate scaffold shift employee_idx:usize --entity --constraint no_overlap --pair\n  solverforge generate scaffold task resource_idx:usize --entity"
    )]
    Scaffold {
        /// Entity name in snake_case (e.g. shift)
        name: String,

        /// Fields in \"name:Type\" format. The first field becomes the planning variable.
        fields: Vec<String>,

        /// Also generate a planning entity for this name
        #[arg(long)]
        entity: bool,

        /// Also generate a constraint with this name
        #[arg(long, value_name = "CONSTRAINT_NAME")]
        constraint: Option<String>,

        /// Also generate a paired twin entity named `<name>_pair`
        #[arg(long)]
        pair: bool,

        /// Overwrite if resources already exist
        #[arg(long, short)]
        force: bool,

        /// Preview changes without writing files
        #[arg(long)]
        pretend: bool,
    },
}

#[derive(Subcommand)]
enum DestroyResource {
    /// Remove the planning solution struct
    Solution,
    /// Remove a planning entity struct
    Entity {
        /// Entity name to remove
        name: String,
    },
    /// Remove a problem fact struct
    Fact {
        /// Fact name to remove
        name: String,
    },
    /// Remove a constraint
    Constraint {
        /// Constraint name to remove
        name: String,
    },
}

fn main() {
    // Load rc config first so CLI flags can override it.
    let rc = rc::load_rc().unwrap_or_default();

    let cli = Cli::parse();

    // Configure output — CLI flags override rc config.
    if cli.quiet || rc.quiet {
        output::set_verbosity(0);
    } else if cli.verbose {
        output::set_verbosity(2);
    }
    if cli.no_color || rc.no_color || std::env::var("NO_COLOR").is_ok() {
        output::set_no_color(true);
    }

    let result: CliResult = match cli.command {
        Command::New {
            name,
            basic,
            list,
            skip_git,
            skip_readme,
        } => {
            let is_basic = basic.is_some();
            let is_list = list.is_some();
            let specialization: Option<String> = basic.flatten().or_else(|| list.flatten());

            match commands::new::Template::parse(is_basic, is_list, specialization.as_deref()) {
                Ok(template) => {
                    commands::new::run(&name, template, skip_git, skip_readme, cli.quiet)
                }
                Err(e) => Err(e),
            }
        }
        Command::Generate {
            resource:
                GenerateResource::Constraint {
                    name,
                    hard: _,
                    soft,
                    unary,
                    pair,
                    join,
                    balance,
                    reward,
                    force,
                    pretend,
                },
        } => commands::generate_constraint::run(
            &name, soft, unary, pair, join, balance, reward, force, pretend,
        ),
        Command::Generate {
            resource:
                GenerateResource::Entity {
                    name,
                    planning_variable,
                    fields,
                    force,
                    pretend,
                },
        } => commands::generate_domain::run_entity(
            &name,
            planning_variable.as_deref(),
            &fields,
            force,
            pretend,
        ),
        Command::Generate {
            resource:
                GenerateResource::Fact {
                    name,
                    fields,
                    force,
                    pretend,
                },
        } => commands::generate_domain::run_fact(&name, &fields, force, pretend),
        Command::Generate {
            resource: GenerateResource::Solution { name, score },
        } => commands::generate_domain::run_solution(&name, &score),
        Command::Generate {
            resource: GenerateResource::Variable { field, entity },
        } => commands::generate_domain::run_variable(&field, &entity),
        Command::Generate {
            resource: GenerateResource::Score { score_type },
        } => commands::generate_domain::run_score(&score_type),
        Command::Destroy {
            yes,
            resource: DestroyResource::Solution,
        } => commands::destroy::run_solution(yes),
        Command::Destroy {
            yes,
            resource: DestroyResource::Entity { name },
        } => commands::destroy::run_entity(&name, yes),
        Command::Destroy {
            yes,
            resource: DestroyResource::Fact { name },
        } => commands::destroy::run_fact(&name, yes),
        Command::Destroy {
            yes,
            resource: DestroyResource::Constraint { name },
        } => commands::destroy::run_constraint(&name, yes),
        Command::Server { port, debug } => commands::server::run(port, debug),
        Command::Info => commands::info::run(),
        Command::Check => commands::check::run(),
        Command::Test { extra_args } => commands::test::run(&extra_args),
        Command::Routes => commands::routes::run(),
        Command::Config {
            subcommand: ConfigSubcommand::Show,
        } => commands::config::run_show(),
        Command::Config {
            subcommand: ConfigSubcommand::Set { key, value },
        } => commands::config::run_set(&key, &value),
        Command::Console => commands::console::run(),
        Command::Generate {
            resource:
                GenerateResource::Scaffold {
                    name,
                    fields,
                    entity,
                    constraint,
                    pair,
                    force,
                    pretend,
                },
        } => commands::generate_scaffold::run(
            &name,
            &fields,
            entity,
            constraint.as_deref(),
            pair,
            force,
            pretend,
        ),
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "solverforge", &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(e) = result {
        output::print_error(&e.to_string());
        std::process::exit(1);
    }
}
